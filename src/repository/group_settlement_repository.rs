use chrono::NaiveDateTime;
use mysql::prelude::*;
use mysql::*;
use std::error::Error;
use uuid::Uuid;

use crate::models::group_balance::GroupBalanceEntry;
use crate::models::group_settlement::{GroupReimbursement, GroupSplitPayment, GroupSplitShare};
use crate::repository::group_balance_repository::{insert_entry, spend_group_balance};
use crate::repository::member_transfer_repository::TransferCategory;

fn parse_uuid(value: &str) -> Uuid {
    Uuid::parse_str(value).unwrap_or_else(|_| Uuid::nil())
}

/// Leaves room for float rounding when a spending is settled down to zero.
const EPSILON: f64 = 0.000_001;

pub fn create_group_settlement_tables(conn: &mut PooledConn) -> Result<()> {
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS group_reimbursement (
            reimbursement_id CHAR(36) PRIMARY KEY,
            group_id CHAR(36) NOT NULL,
            transaction_id CHAR(36) NOT NULL,
            owner VARCHAR(255) NOT NULL,
            paid_by VARCHAR(255) NOT NULL,
            from_source_id CHAR(36) NULL,
            from_source VARCHAR(255) NOT NULL,
            personal_amount DOUBLE NOT NULL,
            to_source_id CHAR(36) NULL,
            to_source VARCHAR(255) NULL,
            group_amount DOUBLE NOT NULL,
            amount DOUBLE NOT NULL,
            description TEXT,
            spending_id CHAR(36) NULL,
            earning_id CHAR(36) NULL,
            from_entry_id CHAR(36) NULL,
            to_entry_id CHAR(36) NULL,
            created_date DATETIME NOT NULL,
            INDEX idx_group_reimbursement_group (group_id),
            INDEX idx_group_reimbursement_txn (transaction_id)
        )",
    )?;
    let has_return_balance: Option<i64> = conn.exec_first(
        "SELECT COUNT(*) FROM information_schema.COLUMNS
         WHERE TABLE_SCHEMA = DATABASE()
           AND TABLE_NAME = 'group_reimbursement'
           AND COLUMN_NAME = 'return_balance'",
        (),
    )?;
    if has_return_balance.unwrap_or(0) == 0 {
        conn.query_drop(
            "ALTER TABLE group_reimbursement
             ADD COLUMN return_balance TINYINT(1) NOT NULL DEFAULT 0",
        )?;
    }
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS group_split_share (
            share_id CHAR(36) PRIMARY KEY,
            group_id CHAR(36) NOT NULL,
            transaction_id CHAR(36) NOT NULL,
            owner VARCHAR(255) NOT NULL,
            username VARCHAR(255) NULL,
            name VARCHAR(255) NOT NULL,
            amount DOUBLE NOT NULL,
            is_active TINYINT(1) NOT NULL DEFAULT 1,
            created_date DATETIME NOT NULL,
            INDEX idx_group_split_share_group (group_id),
            INDEX idx_group_split_share_txn (transaction_id)
        )",
    )?;
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS group_split_payment (
            payment_id CHAR(36) PRIMARY KEY,
            share_id CHAR(36) NOT NULL,
            group_id CHAR(36) NOT NULL,
            transaction_id CHAR(36) NOT NULL,
            owner VARCHAR(255) NOT NULL,
            paid_by VARCHAR(255) NULL,
            payer_name VARCHAR(255) NOT NULL,
            amount DOUBLE NOT NULL,
            status VARCHAR(16) NOT NULL,
            from_source_id CHAR(36) NULL,
            from_source VARCHAR(255) NULL,
            to_source_id CHAR(36) NULL,
            to_source VARCHAR(255) NULL,
            note TEXT,
            requested_at DATETIME NOT NULL,
            reviewed_at DATETIME NULL,
            spending_id CHAR(36) NULL,
            earning_id CHAR(36) NULL,
            from_entry_id CHAR(36) NULL,
            to_entry_id CHAR(36) NULL,
            INDEX idx_group_split_payment_share (share_id),
            INDEX idx_group_split_payment_group (group_id)
        )",
    )?;
    // People outside the app, remembered per group so a name typed once can
    // be picked again in the next split bill.
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS group_contact_name (
            group_id CHAR(36) NOT NULL,
            name VARCHAR(255) NOT NULL,
            created_by VARCHAR(255) NOT NULL,
            created_date DATETIME NOT NULL,
            PRIMARY KEY (group_id, name)
        )",
    )?;
    Ok(())
}

// ── The settled spending ─────────────────────────────────────────────────────

/// A group spending that can be reimbursed or split.
pub struct GroupSpending {
    pub owner: String,
    pub total_amount: f64,
    pub description: String,
}

/// The group spending `transaction_id` of `group_id`: a tagged personal
/// spending or a spending paid from a group balance. Earnings are not
/// settled, so they are never found here.
pub fn select_group_spending<Q: Queryable>(
    conn: &mut Q,
    group_id: Uuid,
    transaction_id: Uuid,
) -> Result<Option<GroupSpending>, Box<dyn Error>> {
    let row: Option<(String, f64, String)> = conn.exec_first(
        "SELECT s.created_by, s.total_amount, COALESCE(s.description, '')
         FROM spending_group_transaction l
         JOIN spending s ON s.spending_id = l.transaction_id AND s.is_active = 1
         WHERE l.transaction_type = 'spending' AND l.group_id = :g1 AND l.transaction_id = :t1
         UNION ALL
         SELECT b.username, -b.amount, COALESCE(b.description, '')
         FROM group_balance_entry b
         WHERE b.entry_type = 'spending' AND b.group_id = :g2 AND b.entry_id = :t2
         LIMIT 1",
        params! {
            "g1" => group_id.to_string(),
            "t1" => transaction_id.to_string(),
            "g2" => group_id.to_string(),
            "t2" => transaction_id.to_string(),
        },
    )?;
    Ok(row.map(|(owner, total_amount, description)| GroupSpending {
        owner,
        total_amount,
        description,
    }))
}

/// How much of the spending is already claimed: reimbursed plus every
/// active split share (paid or not).
fn committed_amount<Q: Queryable>(
    conn: &mut Q,
    transaction_id: Uuid,
) -> Result<f64, Box<dyn Error>> {
    let amount: Option<f64> = conn.exec_first(
        "SELECT
            COALESCE((SELECT SUM(amount) FROM group_reimbursement WHERE transaction_id = :t1), 0)
          + COALESCE((SELECT SUM(amount) FROM group_split_share
                      WHERE transaction_id = :t2 AND is_active = 1), 0)",
        params! {
            "t1" => transaction_id.to_string(),
            "t2" => transaction_id.to_string(),
        },
    )?;
    Ok(amount.unwrap_or(0.0))
}

/// Serialises every settlement of one group, so two members racing each
/// other cannot both claim the same unsettled part of a spending.
fn lock_group<Q: Queryable>(conn: &mut Q, group_id: Uuid) -> Result<(), Box<dyn Error>> {
    let _lock: Option<u8> = conn.exec_first(
        "SELECT 1 FROM spending_group WHERE group_id = :id FOR UPDATE",
        params! { "id" => group_id.to_string() },
    )?;
    Ok(())
}

// ── Money legs ───────────────────────────────────────────────────────────────

/// A personal source: `(source_id, source name)`.
pub type Source = (Uuid, String);

#[allow(clippy::too_many_arguments)]
/// A spending in `username`'s own records under the Transfer category, so the
/// source goes down without counting as an expense.
fn debit_source<Q: Queryable>(
    conn: &mut Q,
    spending_id: Uuid,
    username: &str,
    source: &Source,
    amount: f64,
    description: &str,
    category: &TransferCategory,
    at: NaiveDateTime,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "INSERT INTO spending
            (spending_id, total_amount, description, spending_category_id, spending_category,
             source_id, source, created_date, created_by, is_active)
         VALUES (:id, :total, :description, :cat_id, :cat, :src_id, :src, :created, :by, 1)",
        params! {
            "id" => spending_id.to_string(),
            "total" => amount,
            "description" => description,
            "cat_id" => category.id.to_string(),
            "cat" => category.name,
            "src_id" => source.0.to_string(),
            "src" => &source.1,
            "created" => at.to_string(),
            "by" => username,
        },
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
/// An earning in `username`'s own records under the Transfer category.
fn credit_source<Q: Queryable>(
    conn: &mut Q,
    earning_id: Uuid,
    username: &str,
    source: &Source,
    amount: f64,
    description: &str,
    category: &TransferCategory,
    at: NaiveDateTime,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "INSERT INTO earning
            (earning_id, total_amount, description, earning_category_id, earning_category,
             source_id, source, created_date, created_by, is_active)
         VALUES (:id, :total, :description, :cat_id, :cat, :src_id, :src, :created, :by, 1)",
        params! {
            "id" => earning_id.to_string(),
            "total" => amount,
            "description" => description,
            "cat_id" => category.id.to_string(),
            "cat" => category.name,
            "src_id" => source.0.to_string(),
            "src" => &source.1,
            "created" => at.to_string(),
            "by" => username,
        },
    )?;
    Ok(())
}

fn balance_entry(
    entry_id: Uuid,
    group_id: Uuid,
    username: &str,
    amount: f64,
    entry_type: &str,
    description: &str,
    at: NaiveDateTime,
) -> GroupBalanceEntry {
    GroupBalanceEntry {
        entry_id,
        group_id,
        username: username.to_string(),
        amount,
        entry_type: entry_type.to_string(),
        description: description.to_string(),
        spending_category_id: None,
        spending_category: None,
        created_date: at,
    }
}

/// Descriptions written on each side's records.
pub struct LegTexts {
    pub payer: String,
    pub receiver: String,
}

/// Why a settlement wrote nothing.
#[derive(Debug, PartialEq)]
pub enum SettleOutcome {
    Done,
    /// The spending no longer exists in the group.
    SpendingGone,
    /// More than what is left unsettled; carries what is left.
    OverLimit(f64),
    /// The payer's group balance does not cover the amount.
    NotEnoughBalance,
    /// The payment is no longer pending (already reviewed or cancelled).
    NotPending,
}

// ── Reimbursements ───────────────────────────────────────────────────────────

/// Pays the owner back, all or nothing: the payer's spending (or group
/// balance entry), the owner's earning and/or group balance entry, and the
/// reimbursement record.
pub fn insert_reimbursement(
    conn: &mut PooledConn,
    r: &GroupReimbursement,
    category: Option<&TransferCategory>,
    texts: &LegTexts,
) -> Result<SettleOutcome, Box<dyn Error>> {
    let personal_leg = r.from_source_id.is_some() || r.personal_amount > 0.0;
    let category = match (personal_leg, category) {
        (true, None) => return Err("Transfer category is not configured".into()),
        (_, category) => category,
    };
    let mut tx = conn.start_transaction(TxOpts::default())?;
    lock_group(&mut tx, r.group_id)?;
    let Some(spending) = select_group_spending(&mut tx, r.group_id, r.transaction_id)? else {
        tx.rollback()?;
        return Ok(SettleOutcome::SpendingGone);
    };
    let left = spending.total_amount - committed_amount(&mut tx, r.transaction_id)?;
    if r.amount > left + EPSILON {
        tx.rollback()?;
        return Ok(SettleOutcome::OverLimit(left.max(0.0)));
    }

    let at = r.created_date;
    let (mut spending_id, mut from_entry_id) = (None, None);
    match r.from_source_id {
        Some(source_id) => {
            let id = Uuid::new_v4();
            let source = (source_id, r.from_source.clone());
            let category = category.ok_or("Transfer category is not configured")?;
            debit_source(&mut tx, id, &r.paid_by, &source, r.amount, &texts.payer, category, at)?;
            spending_id = Some(id);
        }
        None => {
            let id = Uuid::new_v4();
            let entry = balance_entry(
                id,
                r.group_id,
                &r.paid_by,
                -r.amount,
                "reimburse_out",
                &texts.payer,
                at,
            );
            if !spend_group_balance(&mut tx, &entry)? {
                tx.rollback()?;
                return Ok(SettleOutcome::NotEnoughBalance);
            }
            from_entry_id = Some(id);
        }
    }

    let (mut earning_id, mut to_entry_id) = (None, None);
    if r.personal_amount > 0.0 {
        if let (Some(source_id), Some(name), Some(category)) =
            (r.to_source_id, &r.to_source, category)
        {
            let id = Uuid::new_v4();
            let source = (source_id, name.clone());
            credit_source(
                &mut tx,
                id,
                &r.owner,
                &source,
                r.personal_amount,
                &texts.receiver,
                category,
                at,
            )?;
            earning_id = Some(id);
        }
    }
    if r.group_amount > 0.0 {
        let id = Uuid::new_v4();
        let entry = balance_entry(
            id,
            r.group_id,
            &r.owner,
            r.group_amount,
            "reimburse_in",
            &texts.receiver,
            at,
        );
        insert_entry(&mut tx, &entry)?;
        to_entry_id = Some(id);
    }
    if r.return_balance {
        // Not money changing hands: the owner's group balance simply gets
        // back what this part of the spending had taken from it.
        let id = Uuid::new_v4();
        let entry = balance_entry(
            id,
            r.group_id,
            &r.owner,
            r.amount,
            "balance_return",
            &texts.receiver,
            at,
        );
        insert_entry(&mut tx, &entry)?;
        to_entry_id = Some(id);
    }

    tx.exec_drop(
        "INSERT INTO group_reimbursement
            (reimbursement_id, group_id, transaction_id, owner, paid_by, from_source_id,
             from_source, personal_amount, to_source_id, to_source, group_amount, amount,
             return_balance, description, spending_id, earning_id, from_entry_id, to_entry_id,
             created_date)
         VALUES (:id, :group_id, :txn_id, :owner, :paid_by, :from_source_id,
                 :from_source, :personal_amount, :to_source_id, :to_source, :group_amount, :amount,
                 :return_balance, :description, :spending_id, :earning_id, :from_entry_id,
                 :to_entry_id, :created)",
        params! {
            "id" => r.reimbursement_id.to_string(),
            "group_id" => r.group_id.to_string(),
            "txn_id" => r.transaction_id.to_string(),
            "owner" => &r.owner,
            "paid_by" => &r.paid_by,
            "from_source_id" => r.from_source_id.map(|id| id.to_string()),
            "from_source" => &r.from_source,
            "personal_amount" => r.personal_amount,
            "to_source_id" => r.to_source_id.map(|id| id.to_string()),
            "to_source" => &r.to_source,
            "group_amount" => r.group_amount,
            "amount" => r.amount,
            "return_balance" => i32::from(r.return_balance),
            "description" => &r.description,
            "spending_id" => spending_id.map(|id| id.to_string()),
            "earning_id" => earning_id.map(|id| id.to_string()),
            "from_entry_id" => from_entry_id.map(|id| id.to_string()),
            "to_entry_id" => to_entry_id.map(|id| id.to_string()),
            "created" => at.to_string(),
        },
    )?;
    tx.commit()?;
    Ok(SettleOutcome::Done)
}

const SELECT_REIMBURSEMENT: &str = "SELECT reimbursement_id, group_id, transaction_id, owner,
    paid_by, from_source_id, from_source, personal_amount, to_source_id, to_source,
    group_amount, amount, COALESCE(description, ''), created_date, return_balance
    FROM group_reimbursement";

fn opt_uuid(row: &Row, i: usize) -> Option<Uuid> {
    row.get::<Option<String>, _>(i)
        .flatten()
        .map(|id| parse_uuid(&id))
}

fn opt_text(row: &Row, i: usize) -> Option<String> {
    row.get::<Option<String>, _>(i).flatten()
}

fn reimbursement_from_row(row: Row) -> GroupReimbursement {
    let text = |i: usize| -> String { row.get::<String, _>(i).unwrap_or_default() };
    GroupReimbursement {
        reimbursement_id: parse_uuid(&text(0)),
        group_id: parse_uuid(&text(1)),
        transaction_id: parse_uuid(&text(2)),
        owner: text(3),
        paid_by: text(4),
        from_source_id: opt_uuid(&row, 5),
        from_source: text(6),
        personal_amount: row.get(7).unwrap_or(0.0),
        to_source_id: opt_uuid(&row, 8),
        to_source: opt_text(&row, 9),
        group_amount: row.get(10).unwrap_or(0.0),
        amount: row.get(11).unwrap_or(0.0),
        description: text(12),
        created_date: row.get(13).unwrap(),
        return_balance: row.get::<i32, _>(14).unwrap_or(0) != 0,
    }
}

pub fn select_reimbursement(
    conn: &mut PooledConn,
    reimbursement_id: Uuid,
) -> Result<Option<GroupReimbursement>, Box<dyn Error>> {
    let row: Option<Row> = conn.exec_first(
        format!("{SELECT_REIMBURSEMENT} WHERE reimbursement_id = :id"),
        params! { "id" => reimbursement_id.to_string() },
    )?;
    Ok(row.map(reimbursement_from_row))
}

pub fn select_group_reimbursements(
    conn: &mut PooledConn,
    group_id: Uuid,
) -> Result<Vec<GroupReimbursement>, Box<dyn Error>> {
    let rows = conn.exec_map(
        format!("{SELECT_REIMBURSEMENT} WHERE group_id = :id ORDER BY created_date DESC"),
        params! { "id" => group_id.to_string() },
        reimbursement_from_row,
    )?;
    Ok(rows)
}

// ── Split bill shares ────────────────────────────────────────────────────────

const SELECT_SHARE: &str = "SELECT s.share_id, s.group_id, s.transaction_id, s.owner,
    s.username, s.name, s.amount, s.created_date,
    COALESCE((SELECT SUM(p.amount) FROM group_split_payment p
              WHERE p.share_id = s.share_id AND p.status = 'approved'), 0),
    COALESCE((SELECT SUM(p.amount) FROM group_split_payment p
              WHERE p.share_id = s.share_id AND p.status = 'pending'), 0)
    FROM group_split_share s";

fn share_from_row(row: Row) -> GroupSplitShare {
    let text = |i: usize| -> String { row.get::<String, _>(i).unwrap_or_default() };
    GroupSplitShare {
        share_id: parse_uuid(&text(0)),
        group_id: parse_uuid(&text(1)),
        transaction_id: parse_uuid(&text(2)),
        owner: text(3),
        username: opt_text(&row, 4),
        name: text(5),
        amount: row.get(6).unwrap_or(0.0),
        created_date: row.get(7).unwrap(),
        paid_amount: row.get(8).unwrap_or(0.0),
        pending_amount: row.get(9).unwrap_or(0.0),
    }
}

/// An active share by id.
pub fn select_split_share<Q: Queryable>(
    conn: &mut Q,
    share_id: Uuid,
) -> Result<Option<GroupSplitShare>, Box<dyn Error>> {
    let row: Option<Row> = conn.exec_first(
        format!("{SELECT_SHARE} WHERE s.share_id = :id AND s.is_active = 1"),
        params! { "id" => share_id.to_string() },
    )?;
    Ok(row.map(share_from_row))
}

pub fn select_group_split_shares(
    conn: &mut PooledConn,
    group_id: Uuid,
) -> Result<Vec<GroupSplitShare>, Box<dyn Error>> {
    let rows = conn.exec_map(
        format!(
            "{SELECT_SHARE} WHERE s.group_id = :id AND s.is_active = 1
             ORDER BY s.created_date ASC"
        ),
        params! { "id" => group_id.to_string() },
        share_from_row,
    )?;
    Ok(rows)
}

/// Adds a share when it still fits in the unsettled part of the spending.
/// A name-only share also saves the name in the group's name list.
pub fn insert_split_share(
    conn: &mut PooledConn,
    share: &GroupSplitShare,
) -> Result<SettleOutcome, Box<dyn Error>> {
    let mut tx = conn.start_transaction(TxOpts::default())?;
    lock_group(&mut tx, share.group_id)?;
    let Some(spending) = select_group_spending(&mut tx, share.group_id, share.transaction_id)?
    else {
        tx.rollback()?;
        return Ok(SettleOutcome::SpendingGone);
    };
    let left = spending.total_amount - committed_amount(&mut tx, share.transaction_id)?;
    if share.amount > left + EPSILON {
        tx.rollback()?;
        return Ok(SettleOutcome::OverLimit(left.max(0.0)));
    }
    tx.exec_drop(
        "INSERT INTO group_split_share
            (share_id, group_id, transaction_id, owner, username, name, amount, is_active,
             created_date)
         VALUES (:id, :group_id, :txn_id, :owner, :username, :name, :amount, 1, :created)",
        params! {
            "id" => share.share_id.to_string(),
            "group_id" => share.group_id.to_string(),
            "txn_id" => share.transaction_id.to_string(),
            "owner" => &share.owner,
            "username" => &share.username,
            "name" => &share.name,
            "amount" => share.amount,
            "created" => share.created_date.to_string(),
        },
    )?;
    if share.username.is_none() {
        tx.exec_drop(
            "INSERT IGNORE INTO group_contact_name (group_id, name, created_by, created_date)
             VALUES (:group_id, :name, :by, :created)",
            params! {
                "group_id" => share.group_id.to_string(),
                "name" => &share.name,
                "by" => &share.owner,
                "created" => share.created_date.to_string(),
            },
        )?;
    }
    tx.commit()?;
    Ok(SettleOutcome::Done)
}

/// Whether the share id is taken at all (also by a removed share).
pub fn split_share_id_owner(
    conn: &mut PooledConn,
    share_id: Uuid,
) -> Result<Option<String>, Box<dyn Error>> {
    let owner: Option<String> = conn.exec_first(
        "SELECT owner FROM group_split_share WHERE share_id = :id",
        params! { "id" => share_id.to_string() },
    )?;
    Ok(owner)
}

/// Removes a share nobody has paid yet; its pending payments are cancelled.
/// `false` when an approved payment already exists.
pub fn remove_split_share(conn: &mut PooledConn, share_id: Uuid) -> Result<bool, Box<dyn Error>> {
    let mut tx = conn.start_transaction(TxOpts::default())?;
    let group: Option<String> = tx.exec_first(
        "SELECT group_id FROM group_split_share WHERE share_id = :id",
        params! { "id" => share_id.to_string() },
    )?;
    if let Some(group_id) = group {
        lock_group(&mut tx, parse_uuid(&group_id))?;
    }
    let approved: Option<u8> = tx.exec_first(
        "SELECT 1 FROM group_split_payment WHERE share_id = :id AND status = 'approved' LIMIT 1",
        params! { "id" => share_id.to_string() },
    )?;
    if approved.is_some() {
        tx.rollback()?;
        return Ok(false);
    }
    tx.exec_drop(
        "UPDATE group_split_payment SET status = 'cancelled'
         WHERE share_id = :id AND status = 'pending'",
        params! { "id" => share_id.to_string() },
    )?;
    tx.exec_drop(
        "UPDATE group_split_share SET is_active = 0 WHERE share_id = :id",
        params! { "id" => share_id.to_string() },
    )?;
    tx.commit()?;
    Ok(true)
}

pub fn select_group_contact_names(
    conn: &mut PooledConn,
    group_id: Uuid,
) -> Result<Vec<String>, Box<dyn Error>> {
    let rows = conn.exec_map(
        "SELECT name FROM group_contact_name WHERE group_id = :id ORDER BY name ASC",
        params! { "id" => group_id.to_string() },
        |name: String| name,
    )?;
    Ok(rows)
}

pub fn delete_group_contact_name(
    conn: &mut PooledConn,
    group_id: Uuid,
    name: &str,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "DELETE FROM group_contact_name WHERE group_id = :id AND name = :name",
        params! { "id" => group_id.to_string(), "name" => name },
    )?;
    Ok(())
}

// ── Split bill payments ──────────────────────────────────────────────────────

const SELECT_PAYMENT: &str = "SELECT payment_id, share_id, group_id, transaction_id, owner,
    paid_by, payer_name, amount, status, from_source_id, from_source, to_source_id, to_source,
    COALESCE(note, ''), requested_at, reviewed_at FROM group_split_payment";

fn payment_from_row(row: Row) -> GroupSplitPayment {
    let text = |i: usize| -> String { row.get::<String, _>(i).unwrap_or_default() };
    GroupSplitPayment {
        payment_id: parse_uuid(&text(0)),
        share_id: parse_uuid(&text(1)),
        group_id: parse_uuid(&text(2)),
        transaction_id: parse_uuid(&text(3)),
        owner: text(4),
        paid_by: opt_text(&row, 5),
        payer_name: text(6),
        amount: row.get(7).unwrap_or(0.0),
        status: text(8),
        from_source_id: opt_uuid(&row, 9),
        from_source: opt_text(&row, 10),
        to_source_id: opt_uuid(&row, 11),
        to_source: opt_text(&row, 12),
        note: text(13),
        requested_at: row.get(14).unwrap(),
        reviewed_at: row.get::<Option<NaiveDateTime>, _>(15).flatten(),
    }
}

pub fn select_split_payment(
    conn: &mut PooledConn,
    payment_id: Uuid,
) -> Result<Option<GroupSplitPayment>, Box<dyn Error>> {
    let row: Option<Row> = conn.exec_first(
        format!("{SELECT_PAYMENT} WHERE payment_id = :id"),
        params! { "id" => payment_id.to_string() },
    )?;
    Ok(row.map(payment_from_row))
}

pub fn select_group_split_payments(
    conn: &mut PooledConn,
    group_id: Uuid,
) -> Result<Vec<GroupSplitPayment>, Box<dyn Error>> {
    let rows = conn.exec_map(
        format!("{SELECT_PAYMENT} WHERE group_id = :id ORDER BY requested_at DESC"),
        params! { "id" => group_id.to_string() },
        payment_from_row,
    )?;
    Ok(rows)
}

fn insert_payment_row<Q: Queryable>(
    conn: &mut Q,
    p: &GroupSplitPayment,
    ids: &LegIds,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "INSERT INTO group_split_payment
            (payment_id, share_id, group_id, transaction_id, owner, paid_by, payer_name, amount,
             status, from_source_id, from_source, to_source_id, to_source, note, requested_at,
             reviewed_at, spending_id, earning_id, from_entry_id, to_entry_id)
         VALUES (:id, :share_id, :group_id, :txn_id, :owner, :paid_by, :payer_name, :amount,
                 :status, :from_source_id, :from_source, :to_source_id, :to_source, :note,
                 :requested_at, :reviewed_at, :spending_id, :earning_id, :from_entry_id,
                 :to_entry_id)",
        params! {
            "id" => p.payment_id.to_string(),
            "share_id" => p.share_id.to_string(),
            "group_id" => p.group_id.to_string(),
            "txn_id" => p.transaction_id.to_string(),
            "owner" => &p.owner,
            "paid_by" => &p.paid_by,
            "payer_name" => &p.payer_name,
            "amount" => p.amount,
            "status" => &p.status,
            "from_source_id" => p.from_source_id.map(|id| id.to_string()),
            "from_source" => &p.from_source,
            "to_source_id" => p.to_source_id.map(|id| id.to_string()),
            "to_source" => &p.to_source,
            "note" => &p.note,
            "requested_at" => p.requested_at.to_string(),
            "reviewed_at" => p.reviewed_at.map(|at| at.to_string()),
            "spending_id" => ids.spending_id.map(|id| id.to_string()),
            "earning_id" => ids.earning_id.map(|id| id.to_string()),
            "from_entry_id" => ids.from_entry_id.map(|id| id.to_string()),
            "to_entry_id" => ids.to_entry_id.map(|id| id.to_string()),
        },
    )?;
    Ok(())
}

/// Ids of the records a settled payment wrote.
#[derive(Default)]
struct LegIds {
    spending_id: Option<Uuid>,
    earning_id: Option<Uuid>,
    from_entry_id: Option<Uuid>,
    to_entry_id: Option<Uuid>,
}

/// Unpaid part of a share, not counting pending requests.
fn share_left<Q: Queryable>(conn: &mut Q, share_id: Uuid) -> Result<Option<f64>, Box<dyn Error>> {
    Ok(select_split_share(conn, share_id)?.map(|s| s.amount - s.paid_amount))
}

/// A member's request to pay part of their share. Nothing moves yet; it only
/// has to fit in what is unpaid and not already requested.
pub fn insert_split_payment_request(
    conn: &mut PooledConn,
    p: &GroupSplitPayment,
) -> Result<SettleOutcome, Box<dyn Error>> {
    let mut tx = conn.start_transaction(TxOpts::default())?;
    lock_group(&mut tx, p.group_id)?;
    let Some(share) = select_split_share(&mut tx, p.share_id)? else {
        tx.rollback()?;
        return Ok(SettleOutcome::SpendingGone);
    };
    let left = share.amount - share.paid_amount - share.pending_amount;
    if p.amount > left + EPSILON {
        tx.rollback()?;
        return Ok(SettleOutcome::OverLimit(left.max(0.0)));
    }
    insert_payment_row(&mut tx, p, &LegIds::default())?;
    tx.commit()?;
    Ok(SettleOutcome::Done)
}

/// The owner approves a pending payment: the money moves from the payer to
/// `to` (one of the owner's sources, or `None` for their group balance) and
/// the payment becomes `approved`, all in one DB transaction.
pub fn approve_split_payment(
    conn: &mut PooledConn,
    payment_id: Uuid,
    to: Option<&Source>,
    category: Option<&TransferCategory>,
    texts: &LegTexts,
    at: NaiveDateTime,
) -> Result<SettleOutcome, Box<dyn Error>> {
    let mut tx = conn.start_transaction(TxOpts::default())?;
    let group: Option<String> = tx.exec_first(
        "SELECT group_id FROM group_split_payment WHERE payment_id = :id",
        params! { "id" => payment_id.to_string() },
    )?;
    let Some(group_id) = group.map(|g| parse_uuid(&g)) else {
        tx.rollback()?;
        return Ok(SettleOutcome::NotPending);
    };
    lock_group(&mut tx, group_id)?;
    let row: Option<Row> = tx.exec_first(
        format!("{SELECT_PAYMENT} WHERE payment_id = :id"),
        params! { "id" => payment_id.to_string() },
    )?;
    let Some(p) = row.map(payment_from_row) else {
        tx.rollback()?;
        return Ok(SettleOutcome::NotPending);
    };
    if p.status != "pending" {
        tx.rollback()?;
        return Ok(SettleOutcome::NotPending);
    }
    let Some(left) = share_left(&mut tx, p.share_id)? else {
        tx.rollback()?;
        return Ok(SettleOutcome::SpendingGone);
    };
    if p.amount > left + EPSILON {
        tx.rollback()?;
        return Ok(SettleOutcome::OverLimit(left.max(0.0)));
    }
    let payer = p.paid_by.clone().unwrap_or_default();

    let mut ids = LegIds::default();
    match (p.from_source_id, &p.from_source) {
        (Some(source_id), Some(name)) => {
            let Some(category) = category else {
                tx.rollback()?;
                return Err("Transfer category is not configured".into());
            };
            let id = Uuid::new_v4();
            debit_source(
                &mut tx,
                id,
                &payer,
                &(source_id, name.clone()),
                p.amount,
                &texts.payer,
                category,
                at,
            )?;
            ids.spending_id = Some(id);
        }
        _ => {
            let id = Uuid::new_v4();
            let entry =
                balance_entry(id, group_id, &payer, -p.amount, "split_out", &texts.payer, at);
            if !spend_group_balance(&mut tx, &entry)? {
                tx.rollback()?;
                return Ok(SettleOutcome::NotEnoughBalance);
            }
            ids.from_entry_id = Some(id);
        }
    }
    credit_owner(&mut tx, group_id, &p.owner, p.amount, to, category, texts, at, &mut ids)?;

    tx.exec_drop(
        "UPDATE group_split_payment
         SET status = 'approved', reviewed_at = :at, to_source_id = :to_source_id,
             to_source = :to_source, spending_id = :spending_id, earning_id = :earning_id,
             from_entry_id = :from_entry_id, to_entry_id = :to_entry_id
         WHERE payment_id = :id",
        params! {
            "at" => at.to_string(),
            "to_source_id" => to.map(|s| s.0.to_string()),
            "to_source" => to.map(|s| s.1.clone()),
            "spending_id" => ids.spending_id.map(|id| id.to_string()),
            "earning_id" => ids.earning_id.map(|id| id.to_string()),
            "from_entry_id" => ids.from_entry_id.map(|id| id.to_string()),
            "to_entry_id" => ids.to_entry_id.map(|id| id.to_string()),
            "id" => payment_id.to_string(),
        },
    )?;
    tx.commit()?;
    Ok(SettleOutcome::Done)
}

/// The owner's side of a split payment: an earning in their source, or a
/// positive entry in their group balance.
#[allow(clippy::too_many_arguments)]
fn credit_owner<Q: Queryable>(
    conn: &mut Q,
    group_id: Uuid,
    owner: &str,
    amount: f64,
    to: Option<&Source>,
    category: Option<&TransferCategory>,
    texts: &LegTexts,
    at: NaiveDateTime,
    ids: &mut LegIds,
) -> Result<(), Box<dyn Error>> {
    match to {
        Some(source) => {
            let category = category.ok_or("Transfer category is not configured")?;
            let id = Uuid::new_v4();
            credit_source(conn, id, owner, source, amount, &texts.receiver, category, at)?;
            ids.earning_id = Some(id);
        }
        None => {
            let id = Uuid::new_v4();
            let entry = balance_entry(id, group_id, owner, amount, "split_in", &texts.receiver, at);
            insert_entry(conn, &entry)?;
            ids.to_entry_id = Some(id);
        }
    }
    Ok(())
}

/// Rejects (owner) or cancels (payer) a payment that is still pending.
pub fn close_split_payment(
    conn: &mut PooledConn,
    payment_id: Uuid,
    status: &str,
    at: NaiveDateTime,
) -> Result<bool, Box<dyn Error>> {
    let result = conn.exec_iter(
        "UPDATE group_split_payment SET status = :status, reviewed_at = :at
         WHERE payment_id = :id AND status = 'pending'",
        params! { "status" => status, "at" => at.to_string(), "id" => payment_id.to_string() },
    )?;
    Ok(result.affected_rows() > 0)
}

/// The owner records money received from a name-only person; it is approved
/// on the spot and credited to the owner.
pub fn insert_manual_split_payment(
    conn: &mut PooledConn,
    p: &GroupSplitPayment,
    to: Option<&Source>,
    category: Option<&TransferCategory>,
    texts: &LegTexts,
) -> Result<SettleOutcome, Box<dyn Error>> {
    let mut tx = conn.start_transaction(TxOpts::default())?;
    lock_group(&mut tx, p.group_id)?;
    let Some(left) = share_left(&mut tx, p.share_id)? else {
        tx.rollback()?;
        return Ok(SettleOutcome::SpendingGone);
    };
    if p.amount > left + EPSILON {
        tx.rollback()?;
        return Ok(SettleOutcome::OverLimit(left.max(0.0)));
    }
    let at = p.requested_at;
    let mut ids = LegIds::default();
    credit_owner(&mut tx, p.group_id, &p.owner, p.amount, to, category, texts, at, &mut ids)?;
    insert_payment_row(&mut tx, p, &ids)?;
    tx.commit()?;
    Ok(SettleOutcome::Done)
}

/// `username` exactly as stored in `group_id`'s member list, if a member.
pub fn select_group_member_name(
    conn: &mut PooledConn,
    group_id: Uuid,
    username: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    let name: Option<String> = conn.exec_first(
        "SELECT username FROM spending_group_member
         WHERE group_id = :id AND LOWER(username) = LOWER(:username)",
        params! { "id" => group_id.to_string(), "username" => username },
    )?;
    Ok(name)
}
