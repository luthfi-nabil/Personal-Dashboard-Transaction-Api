use chrono::{Local, NaiveDateTime};
use mysql::prelude::*;
use mysql::*;
use std::error::Error;
use uuid::Uuid;

use crate::models::group::{GroupMember, GroupStatusChange, GroupTransaction, SpendingGroup};

fn parse_uuid(value: &str) -> Uuid {
    Uuid::parse_str(value).unwrap_or_else(|_| Uuid::nil())
}

pub fn create_group_tables(conn: &mut PooledConn) -> Result<()> {
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS spending_group (
            group_id CHAR(36) PRIMARY KEY,
            group_name VARCHAR(255) NOT NULL,
            leader VARCHAR(255) NOT NULL,
            is_active TINYINT(1) NOT NULL DEFAULT 1,
            created_date DATETIME NOT NULL,
            updated_date DATETIME NOT NULL
        )",
    )?;
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS spending_group_member (
            group_id CHAR(36) NOT NULL,
            username VARCHAR(255) NOT NULL,
            added_by VARCHAR(255) NOT NULL,
            added_date DATETIME NOT NULL,
            PRIMARY KEY (group_id, username),
            INDEX idx_spending_group_member_username (username)
        )",
    )?;
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS spending_group_status (
            status_id CHAR(36) PRIMARY KEY,
            group_id CHAR(36) NOT NULL,
            is_active TINYINT(1) NOT NULL,
            changed_by VARCHAR(255) NOT NULL,
            changed_at DATETIME NOT NULL,
            INDEX idx_spending_group_status_group (group_id, changed_at)
        )",
    )?;
    // One group per transaction: the primary key is the transaction itself.
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS spending_group_transaction (
            transaction_type VARCHAR(16) NOT NULL,
            transaction_id CHAR(36) NOT NULL,
            group_id CHAR(36) NOT NULL,
            created_by VARCHAR(255) NOT NULL,
            created_date DATETIME NOT NULL,
            PRIMARY KEY (transaction_type, transaction_id),
            INDEX idx_spending_group_transaction_group (group_id)
        )",
    )?;
    Ok(())
}

/// The leader of `group_id`, or `None` when the group does not exist.
pub fn select_group_leader(
    conn: &mut PooledConn,
    group_id: Uuid,
) -> Result<Option<String>, Box<dyn Error>> {
    let leader: Option<String> = conn.exec_first(
        "SELECT leader FROM spending_group WHERE group_id = :id",
        params! { "id" => group_id.to_string() },
    )?;
    Ok(leader)
}

pub fn is_group_member(
    conn: &mut PooledConn,
    group_id: Uuid,
    username: &str,
) -> Result<bool, Box<dyn Error>> {
    let found: Option<u8> = conn.exec_first(
        // Case-insensitive whatever the column collation is: one account is
        // one member however its name was typed.
        "SELECT 1 FROM spending_group_member
         WHERE group_id = :id AND LOWER(username) = LOWER(:username)",
        params! { "id" => group_id.to_string(), "username" => username },
    )?;
    Ok(found.is_some())
}

/// Every group `username` belongs to, with members and switch history filled in.
pub fn select_groups_for_user(
    conn: &mut PooledConn,
    username: &str,
) -> Result<Vec<SpendingGroup>, Box<dyn Error>> {
    let mut groups: Vec<SpendingGroup> = conn.exec_map(
        "SELECT g.group_id, g.group_name, g.leader, g.is_active, g.created_date, g.updated_date
         FROM spending_group g
         JOIN spending_group_member m ON m.group_id = g.group_id
         WHERE m.username = :username
         ORDER BY g.created_date ASC",
        params! { "username" => username },
        |(group_id, group_name, leader, is_active, created_date, updated_date): (
            String,
            String,
            String,
            i32,
            NaiveDateTime,
            NaiveDateTime,
        )| SpendingGroup {
            group_id: parse_uuid(&group_id),
            group_name,
            leader,
            is_active: is_active != 0,
            created_date,
            updated_date,
            members: Vec::new(),
            status_log: Vec::new(),
        },
    )?;

    let members: Vec<GroupMember> = conn.exec_map(
        "SELECT group_id, username, added_by, added_date
         FROM spending_group_member
         WHERE group_id IN (SELECT group_id FROM spending_group_member WHERE username = :username)
         ORDER BY added_date ASC",
        params! { "username" => username },
        |(group_id, username, added_by, added_date): (String, String, String, NaiveDateTime)| {
            GroupMember {
                group_id: parse_uuid(&group_id),
                username,
                added_by,
                added_date,
            }
        },
    )?;

    let statuses: Vec<GroupStatusChange> = conn.exec_map(
        "SELECT status_id, group_id, is_active, changed_by, changed_at
         FROM spending_group_status
         WHERE group_id IN (SELECT group_id FROM spending_group_member WHERE username = :username)
         ORDER BY changed_at ASC",
        params! { "username" => username },
        |(status_id, group_id, is_active, changed_by, changed_at): (
            String,
            String,
            i32,
            String,
            NaiveDateTime,
        )| GroupStatusChange {
            status_id: parse_uuid(&status_id),
            group_id: parse_uuid(&group_id),
            is_active: is_active != 0,
            changed_by,
            changed_at,
        },
    )?;

    for group in groups.iter_mut() {
        group.members = members
            .iter()
            .filter(|m| m.group_id == group.group_id)
            .map(|m| GroupMember {
                group_id: m.group_id,
                username: m.username.clone(),
                added_by: m.added_by.clone(),
                added_date: m.added_date,
            })
            .collect();
        group.status_log = statuses
            .iter()
            .filter(|s| s.group_id == group.group_id)
            .map(|s| GroupStatusChange {
                status_id: s.status_id,
                group_id: s.group_id,
                is_active: s.is_active,
                changed_by: s.changed_by.clone(),
                changed_at: s.changed_at,
            })
            .collect();
    }
    Ok(groups)
}

/// Creates the group, or renames it when it already exists. The leader is
/// always a member of their own group.
pub fn upsert_group(
    conn: &mut PooledConn,
    group_id: Uuid,
    group_name: &str,
    leader: &str,
    created_date: NaiveDateTime,
) -> Result<(), Box<dyn Error>> {
    let now = Local::now().naive_local().to_string();
    conn.exec_drop(
        "INSERT INTO spending_group (group_id, group_name, leader, is_active, created_date, updated_date)
         VALUES (:id, :name, :leader, 1, :created, :updated)
         ON DUPLICATE KEY UPDATE group_name = VALUES(group_name), updated_date = VALUES(updated_date)",
        params! {
            "id" => group_id.to_string(),
            "name" => group_name,
            "leader" => leader,
            "created" => created_date.to_string(),
            "updated" => &now,
        },
    )?;
    insert_group_member(conn, group_id, leader, leader, created_date)?;
    Ok(())
}

/// Adds `username` to the group. Adding someone who is already a member is a
/// no-op, so a queued add can be retried safely.
pub fn insert_group_member(
    conn: &mut PooledConn,
    group_id: Uuid,
    username: &str,
    added_by: &str,
    added_date: NaiveDateTime,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "INSERT IGNORE INTO spending_group_member (group_id, username, added_by, added_date)
         VALUES (:id, :username, :added_by, :added_date)",
        params! {
            "id" => group_id.to_string(),
            "username" => username,
            "added_by" => added_by,
            "added_date" => added_date.to_string(),
        },
    )?;
    Ok(())
}

/// Records one on/off switch and re-derives the group's current state from
/// the latest switch by time - not by arrival - so an older switch that was
/// queued offline never overrides a newer one.
pub fn insert_group_status(
    conn: &mut PooledConn,
    change: &GroupStatusChange,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "INSERT IGNORE INTO spending_group_status (status_id, group_id, is_active, changed_by, changed_at)
         VALUES (:id, :group_id, :is_active, :changed_by, :changed_at)",
        params! {
            "id" => change.status_id.to_string(),
            "group_id" => change.group_id.to_string(),
            "is_active" => i32::from(change.is_active),
            "changed_by" => &change.changed_by,
            "changed_at" => change.changed_at.to_string(),
        },
    )?;
    conn.exec_drop(
        "UPDATE spending_group
         SET is_active = COALESCE((
                 SELECT st.is_active FROM spending_group_status st
                 WHERE st.group_id = :status_group_id
                 ORDER BY st.changed_at DESC LIMIT 1
             ), 1),
             updated_date = :now
         WHERE group_id = :group_id",
        params! {
            "status_group_id" => change.group_id.to_string(),
            "group_id" => change.group_id.to_string(),
            "now" => Local::now().naive_local().to_string(),
        },
    )?;
    Ok(())
}

/// Tags a freshly created spending/earning into `group_id`, but only when
/// `username` belongs to that group. A tag that cannot be applied is dropped
/// rather than failing the write: the transaction is personal first, and a
/// write queued offline must not get stuck because membership changed.
pub fn link_transaction_to_group(
    conn: &mut PooledConn,
    group_id: Uuid,
    transaction_type: &str,
    transaction_id: Uuid,
    username: &str,
    created_date: NaiveDateTime,
) -> Result<bool, Box<dyn Error>> {
    if !is_group_member(conn, group_id, username)? {
        return Ok(false);
    }
    conn.exec_drop(
        "INSERT INTO spending_group_transaction
            (transaction_type, transaction_id, group_id, created_by, created_date)
         VALUES (:type, :txn_id, :group_id, :created_by, :created_date)
         ON DUPLICATE KEY UPDATE group_id = VALUES(group_id)",
        params! {
            "type" => transaction_type,
            "txn_id" => transaction_id.to_string(),
            "group_id" => group_id.to_string(),
            "created_by" => username,
            "created_date" => created_date.to_string(),
        },
    )?;
    Ok(true)
}

/// Every member's tagged spendings and earnings across all of `username`'s
/// groups, plus group transactions paid from a member's group balance (those
/// are listed as spendings; their id is the balance entry's id).
/// `after_turned_off` is judged against the switch history at the moment the
/// transaction happened; with no switch yet a group counts as on.
pub fn select_group_transactions(
    conn: &mut PooledConn,
    username: &str,
) -> Result<Vec<GroupTransaction>, Box<dyn Error>> {
    let rows = conn.exec_map(
        "SELECT l.group_id, 'spending' AS transaction_type, s.spending_id AS transaction_id,
                s.total_amount, COALESCE(s.description, '') AS description,
                s.spending_category AS category, s.created_date, s.created_by,
                COALESCE(s.source, '') AS source,
                COALESCE((
                    SELECT st.is_active FROM spending_group_status st
                    WHERE st.group_id = l.group_id AND st.changed_at <= s.created_date
                    ORDER BY st.changed_at DESC LIMIT 1
                ), 1) = 0 AS after_turned_off
         FROM spending_group_transaction l
         JOIN spending s ON s.spending_id = l.transaction_id AND s.is_active = 1
         WHERE l.transaction_type = 'spending'
           AND l.group_id IN (SELECT group_id FROM spending_group_member WHERE username = :u1)
         UNION ALL
         SELECT l.group_id, 'earning' AS transaction_type, e.earning_id AS transaction_id,
                e.total_amount, COALESCE(e.description, '') AS description,
                e.earning_category AS category, e.created_date, e.created_by,
                COALESCE(e.source, '') AS source,
                COALESCE((
                    SELECT st.is_active FROM spending_group_status st
                    WHERE st.group_id = l.group_id AND st.changed_at <= e.created_date
                    ORDER BY st.changed_at DESC LIMIT 1
                ), 1) = 0 AS after_turned_off
         FROM spending_group_transaction l
         JOIN earning e ON e.earning_id = l.transaction_id AND e.is_active = 1
         WHERE l.transaction_type = 'earning'
           AND l.group_id IN (SELECT group_id FROM spending_group_member WHERE username = :u2)
         UNION ALL
         SELECT b.group_id, 'spending' AS transaction_type, b.entry_id AS transaction_id,
                -b.amount AS total_amount, COALESCE(b.description, '') AS description,
                COALESCE(b.spending_category, '') AS category, b.created_date,
                b.username AS created_by,
                'Group balance' AS source,
                COALESCE((
                    SELECT st.is_active FROM spending_group_status st
                    WHERE st.group_id = b.group_id AND st.changed_at <= b.created_date
                    ORDER BY st.changed_at DESC LIMIT 1
                ), 1) = 0 AS after_turned_off
         FROM group_balance_entry b
         WHERE b.entry_type = 'spending'
           AND b.group_id IN (SELECT group_id FROM spending_group_member WHERE username = :u3)
         ORDER BY created_date DESC",
        params! { "u1" => username, "u2" => username, "u3" => username },
        |(
            group_id,
            transaction_type,
            transaction_id,
            total_amount,
            description,
            category,
            created_date,
            created_by,
            source,
            after_turned_off,
        ): (
            String,
            String,
            String,
            f64,
            String,
            String,
            NaiveDateTime,
            String,
            String,
            i64,
        )| GroupTransaction {
            group_id: parse_uuid(&group_id),
            transaction_type,
            transaction_id: parse_uuid(&transaction_id),
            total_amount,
            description,
            category,
            created_date,
            created_by,
            source,
            after_turned_off: after_turned_off != 0,
        },
    )?;
    Ok(rows)
}
