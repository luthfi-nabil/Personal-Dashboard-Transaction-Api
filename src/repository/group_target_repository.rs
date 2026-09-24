use chrono::NaiveDateTime;
use mysql::prelude::*;
use mysql::*;
use std::error::Error;
use uuid::Uuid;

use crate::models::group_target::GroupMemberTarget;

fn parse_uuid(value: &str) -> Uuid {
    Uuid::parse_str(value).unwrap_or_else(|_| Uuid::nil())
}

pub fn create_group_target_tables(conn: &mut PooledConn) -> Result<()> {
    // One row per group once its leader has flipped the switch; a group
    // without a row has target spendings on.
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS group_target_setting (
            group_id CHAR(36) PRIMARY KEY,
            enabled TINYINT NOT NULL DEFAULT 1,
            updated_by VARCHAR(255) NOT NULL,
            updated_date DATETIME NOT NULL
        )",
    )?;
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS group_member_target (
            group_id CHAR(36) NOT NULL,
            username VARCHAR(255) NOT NULL,
            amount DOUBLE NOT NULL,
            updated_date DATETIME NOT NULL,
            PRIMARY KEY (group_id, username)
        )",
    )?;
    Ok(())
}

/// Whether target spendings are on for `group_id` (on unless the leader
/// turned them off).
pub fn select_group_target_enabled(
    conn: &mut PooledConn,
    group_id: Uuid,
) -> Result<bool, Box<dyn Error>> {
    let enabled: Option<i32> = conn.exec_first(
        "SELECT enabled FROM group_target_setting WHERE group_id = :id",
        params! { "id" => group_id.to_string() },
    )?;
    Ok(enabled.map(|v| v != 0).unwrap_or(true))
}

pub fn upsert_group_target_setting(
    conn: &mut PooledConn,
    group_id: Uuid,
    enabled: bool,
    username: &str,
    now: NaiveDateTime,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "INSERT INTO group_target_setting (group_id, enabled, updated_by, updated_date)
         VALUES (:id, :enabled, :username, :now)
         ON DUPLICATE KEY UPDATE enabled = VALUES(enabled),
            updated_by = VALUES(updated_by), updated_date = VALUES(updated_date)",
        params! {
            "id" => group_id.to_string(),
            "enabled" => i32::from(enabled),
            "username" => username,
            "now" => now.to_string(),
        },
    )?;
    Ok(())
}

/// Targets of `group_id`, only `only_user`'s when given.
pub fn select_group_targets(
    conn: &mut PooledConn,
    group_id: Uuid,
    only_user: Option<&str>,
) -> Result<Vec<GroupMemberTarget>, Box<dyn Error>> {
    let rows: Vec<Row> = conn.exec(
        "SELECT group_id, username, amount, updated_date FROM group_member_target
         WHERE group_id = :id
           AND (:only_user IS NULL OR LOWER(username) = LOWER(:only_user2))
         ORDER BY username",
        params! {
            "id" => group_id.to_string(),
            "only_user" => only_user,
            "only_user2" => only_user,
        },
    )?;
    Ok(rows
        .into_iter()
        .map(|row| GroupMemberTarget {
            group_id: parse_uuid(&row.get::<String, _>(0).unwrap_or_default()),
            username: row.get::<String, _>(1).unwrap_or_default(),
            amount: row.get(2).unwrap_or(0.0),
            updated_date: row.get(3).unwrap(),
        })
        .collect())
}

/// Sets `username`'s target in `group_id`; zero or less removes it.
pub fn upsert_group_member_target(
    conn: &mut PooledConn,
    target: &GroupMemberTarget,
) -> Result<(), Box<dyn Error>> {
    if target.amount <= 0.0 {
        conn.exec_drop(
            "DELETE FROM group_member_target
             WHERE group_id = :id AND LOWER(username) = LOWER(:username)",
            params! {
                "id" => target.group_id.to_string(),
                "username" => &target.username,
            },
        )?;
        return Ok(());
    }
    conn.exec_drop(
        "INSERT INTO group_member_target (group_id, username, amount, updated_date)
         VALUES (:id, :username, :amount, :now)
         ON DUPLICATE KEY UPDATE amount = VALUES(amount), updated_date = VALUES(updated_date)",
        params! {
            "id" => target.group_id.to_string(),
            "username" => &target.username,
            "amount" => target.amount,
            "now" => target.updated_date.to_string(),
        },
    )?;
    Ok(())
}
