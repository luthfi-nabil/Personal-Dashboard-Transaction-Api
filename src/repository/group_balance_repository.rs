use chrono::NaiveDateTime;
use mysql::prelude::*;
use mysql::*;
use std::error::Error;
use uuid::Uuid;

use crate::models::group_balance::{GroupBalanceEntry, GroupMemberBalance};

fn parse_uuid(value: &str) -> Uuid {
    Uuid::parse_str(value).unwrap_or_else(|_| Uuid::nil())
}

pub fn create_group_balance_table(conn: &mut PooledConn) -> Result<()> {
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS group_balance_entry (
            entry_id CHAR(36) PRIMARY KEY,
            group_id CHAR(36) NOT NULL,
            username VARCHAR(255) NOT NULL,
            amount DOUBLE NOT NULL,
            entry_type VARCHAR(16) NOT NULL,
            description TEXT,
            spending_category_id CHAR(36) NULL,
            spending_category VARCHAR(255) NULL,
            created_date DATETIME NOT NULL,
            INDEX idx_group_balance_entry_member (group_id, username),
            INDEX idx_group_balance_entry_date (group_id, created_date)
        )",
    )?;
    Ok(())
}

pub fn group_balance_entry_exists(
    conn: &mut PooledConn,
    entry_id: Uuid,
) -> Result<Option<GroupBalanceEntry>, Box<dyn Error>> {
    let row: Option<Row> = conn.exec_first(
        format!("{SELECT_ENTRY} WHERE entry_id = :id"),
        params! { "id" => entry_id.to_string() },
    )?;
    Ok(row.map(entry_from_row))
}

const SELECT_ENTRY: &str = "SELECT entry_id, group_id, username, amount, entry_type,
    COALESCE(description, ''), spending_category_id, spending_category, created_date
    FROM group_balance_entry";

fn entry_from_row(row: Row) -> GroupBalanceEntry {
    let text = |i: usize| -> String { row.get::<String, _>(i).unwrap_or_default() };
    GroupBalanceEntry {
        entry_id: parse_uuid(&text(0)),
        group_id: parse_uuid(&text(1)),
        username: text(2),
        amount: row.get(3).unwrap_or(0.0),
        entry_type: text(4),
        description: text(5),
        spending_category_id: row
            .get::<Option<String>, _>(6)
            .flatten()
            .map(|id| parse_uuid(&id)),
        spending_category: row.get::<Option<String>, _>(7).flatten(),
        created_date: row.get(8).unwrap(),
    }
}

fn insert_entry<Q: Queryable>(conn: &mut Q, entry: &GroupBalanceEntry) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "INSERT INTO group_balance_entry
            (entry_id, group_id, username, amount, entry_type, description,
             spending_category_id, spending_category, created_date)
         VALUES (:id, :group_id, :username, :amount, :entry_type, :description,
                 :cat_id, :cat, :created)",
        params! {
            "id" => entry.entry_id.to_string(),
            "group_id" => entry.group_id.to_string(),
            "username" => &entry.username,
            "amount" => entry.amount,
            "entry_type" => &entry.entry_type,
            "description" => &entry.description,
            "cat_id" => entry.spending_category_id.map(|id| id.to_string()),
            "cat" => &entry.spending_category,
            "created" => entry.created_date.to_string(),
        },
    )?;
    Ok(())
}

pub fn insert_group_top_up(
    conn: &mut PooledConn,
    entry: &GroupBalanceEntry,
) -> Result<(), Box<dyn Error>> {
    insert_entry(conn, entry)
}

/// Spends from a member's group balance inside the caller's DB transaction.
///
/// The member's row in `spending_group_member` is locked first, so two
/// spendings racing each other cannot both pass the balance check. Returns
/// `false` - writing nothing - when the balance does not cover the amount.
pub fn spend_group_balance<Q: Queryable>(
    conn: &mut Q,
    entry: &GroupBalanceEntry,
) -> Result<bool, Box<dyn Error>> {
    let _lock: Option<u8> = conn.exec_first(
        "SELECT 1 FROM spending_group_member
         WHERE group_id = :group_id AND username = :username FOR UPDATE",
        params! {
            "group_id" => entry.group_id.to_string(),
            "username" => &entry.username,
        },
    )?;
    let balance: Option<f64> = conn.exec_first(
        "SELECT COALESCE(SUM(amount), 0) FROM group_balance_entry
         WHERE group_id = :group_id AND username = :username",
        params! {
            "group_id" => entry.group_id.to_string(),
            "username" => &entry.username,
        },
    )?;
    // `entry.amount` is negative for a spending; a tiny epsilon keeps a
    // balance spent down to exactly zero from failing on float rounding.
    if balance.unwrap_or(0.0) + entry.amount < -0.000_001 {
        return Ok(false);
    }
    insert_entry(conn, entry)?;
    Ok(true)
}

/// Standalone group transaction paid from the balance, all or nothing.
pub fn insert_group_balance_spending(
    conn: &mut PooledConn,
    entry: &GroupBalanceEntry,
) -> Result<bool, Box<dyn Error>> {
    let mut tx = conn.start_transaction(TxOpts::default())?;
    if !spend_group_balance(&mut tx, entry)? {
        tx.rollback()?;
        return Ok(false);
    }
    tx.commit()?;
    Ok(true)
}

/// Balances of `group_id`'s members - every member, or only `only_user`.
/// Members with no entries yet are listed with zero.
pub fn select_group_balances(
    conn: &mut PooledConn,
    group_id: Uuid,
    only_user: Option<&str>,
) -> Result<Vec<GroupMemberBalance>, Box<dyn Error>> {
    let rows = conn.exec_map(
        "SELECT m.username,
                COALESCE(SUM(b.amount), 0),
                COALESCE(SUM(CASE WHEN b.amount > 0 THEN b.amount ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN b.amount < 0 THEN -b.amount ELSE 0 END), 0)
         FROM spending_group_member m
         LEFT JOIN group_balance_entry b
                ON b.group_id = m.group_id AND b.username = m.username
         WHERE m.group_id = :group_id
           AND (:only_user IS NULL OR m.username = :only_user2)
         GROUP BY m.username
         ORDER BY m.username ASC",
        params! {
            "group_id" => group_id.to_string(),
            "only_user" => only_user,
            "only_user2" => only_user,
        },
        |(username, balance, total_top_up, total_spent): (String, f64, f64, f64)| {
            GroupMemberBalance {
                group_id,
                username,
                balance,
                total_top_up,
                total_spent,
            }
        },
    )?;
    Ok(rows)
}

/// Balance history of `group_id`, newest first - everyone's, or only
/// `only_user`'s.
pub fn select_group_balance_entries(
    conn: &mut PooledConn,
    group_id: Uuid,
    only_user: Option<&str>,
) -> Result<Vec<GroupBalanceEntry>, Box<dyn Error>> {
    let rows = conn.exec_map(
        format!(
            "{SELECT_ENTRY}
             WHERE group_id = :group_id AND (:only_user IS NULL OR username = :only_user2)
             ORDER BY created_date DESC"
        ),
        params! {
            "group_id" => group_id.to_string(),
            "only_user" => only_user,
            "only_user2" => only_user,
        },
        entry_from_row,
    )?;
    Ok(rows)
}

/// A spending entry for `username`, ready for [`spend_group_balance`].
#[allow(clippy::too_many_arguments)]
pub fn balance_spending(
    entry_id: Uuid,
    group_id: Uuid,
    username: &str,
    amount: f64,
    description: &str,
    category_id: Uuid,
    category: &str,
    created_date: NaiveDateTime,
) -> GroupBalanceEntry {
    GroupBalanceEntry {
        entry_id,
        group_id,
        username: username.to_string(),
        amount: -amount.abs(),
        entry_type: "spending".to_string(),
        description: description.to_string(),
        spending_category_id: Some(category_id),
        spending_category: Some(category.to_string()),
        created_date,
    }
}
