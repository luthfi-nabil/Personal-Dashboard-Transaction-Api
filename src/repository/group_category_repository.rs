use chrono::NaiveDateTime;
use mysql::prelude::*;
use mysql::*;
use std::error::Error;
use uuid::Uuid;

use crate::models::group_category::GroupCategory;

fn parse_uuid(value: &str) -> Uuid {
    Uuid::parse_str(value).unwrap_or_else(|_| Uuid::nil())
}

pub fn create_group_category_table(conn: &mut PooledConn) -> Result<()> {
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS group_category (
            category_id CHAR(36) PRIMARY KEY,
            group_id CHAR(36) NOT NULL,
            category_name VARCHAR(255) NOT NULL,
            kind VARCHAR(16) NOT NULL DEFAULT 'spending',
            created_by VARCHAR(255) NOT NULL,
            created_date DATETIME NOT NULL,
            is_active TINYINT(1) NOT NULL DEFAULT 1,
            INDEX idx_group_category_group (group_id)
        )",
    )?;
    Ok(())
}

type CategoryRow = (String, String, String, String, String, NaiveDateTime);

fn category_from_row(
    (category_id, group_id, category_name, kind, created_by, created_date): CategoryRow,
) -> GroupCategory {
    GroupCategory {
        category_id: parse_uuid(&category_id),
        group_id: parse_uuid(&group_id),
        category_name,
        kind,
        created_by,
        created_date,
    }
}

/// Active categories of every group `username` belongs to.
pub fn select_group_categories_for_user(
    conn: &mut PooledConn,
    username: &str,
) -> Result<Vec<GroupCategory>, Box<dyn Error>> {
    let rows = conn.exec_map(
        "SELECT c.category_id, c.group_id, c.category_name, c.kind, c.created_by, c.created_date
         FROM group_category c
         WHERE c.is_active = 1
           AND c.group_id IN (SELECT group_id FROM spending_group_member WHERE username = :username)
         ORDER BY c.category_name ASC",
        params! { "username" => username },
        category_from_row,
    )?;
    Ok(rows)
}

/// The active `kind` category `category_id` of `group_id`, if there is one.
pub fn select_group_category(
    conn: &mut PooledConn,
    group_id: Uuid,
    category_id: Uuid,
    kind: &str,
) -> Result<Option<GroupCategory>, Box<dyn Error>> {
    let row: Option<CategoryRow> = conn.exec_first(
        "SELECT category_id, group_id, category_name, kind, created_by, created_date
         FROM group_category
         WHERE group_id = :group_id AND category_id = :id AND kind = :kind AND is_active = 1",
        params! {
            "group_id" => group_id.to_string(),
            "id" => category_id.to_string(),
            "kind" => kind,
        },
    )?;
    Ok(row.map(category_from_row))
}

/// Group of `category_id`, active or not - used to refuse reusing an id that
/// already belongs to another group.
pub fn select_group_category_owner(
    conn: &mut PooledConn,
    category_id: Uuid,
) -> Result<Option<String>, Box<dyn Error>> {
    let group_id: Option<String> = conn.exec_first(
        "SELECT group_id FROM group_category WHERE category_id = :id",
        params! { "id" => category_id.to_string() },
    )?;
    Ok(group_id)
}

/// Whether `group_id` already has an active `kind` category called `name`
/// (any case), other than `except_id`.
pub fn group_category_name_taken(
    conn: &mut PooledConn,
    group_id: Uuid,
    name: &str,
    kind: &str,
    except_id: Uuid,
) -> Result<bool, Box<dyn Error>> {
    let found: Option<u8> = conn.exec_first(
        "SELECT 1 FROM group_category
         WHERE group_id = :group_id AND kind = :kind AND is_active = 1
           AND LOWER(category_name) = LOWER(:name) AND category_id <> :id",
        params! {
            "group_id" => group_id.to_string(),
            "kind" => kind,
            "name" => name,
            "id" => except_id.to_string(),
        },
    )?;
    Ok(found.is_some())
}

pub fn upsert_group_category(
    conn: &mut PooledConn,
    category: &GroupCategory,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "INSERT INTO group_category
            (category_id, group_id, category_name, kind, created_by, created_date, is_active)
         VALUES (:id, :group_id, :name, :kind, :created_by, :created_date, 1)
         ON DUPLICATE KEY UPDATE category_name = VALUES(category_name), is_active = 1",
        params! {
            "id" => category.category_id.to_string(),
            "group_id" => category.group_id.to_string(),
            "name" => &category.category_name,
            "kind" => &category.kind,
            "created_by" => &category.created_by,
            "created_date" => category.created_date.to_string(),
        },
    )?;
    Ok(())
}

/// Soft-deletes the category. Transactions already filed under it keep the
/// name they were saved with. Returns `false` when there was nothing to
/// remove.
pub fn deactivate_group_category(
    conn: &mut PooledConn,
    group_id: Uuid,
    category_id: Uuid,
) -> Result<bool, Box<dyn Error>> {
    conn.exec_drop(
        "UPDATE group_category SET is_active = 0
         WHERE group_id = :group_id AND category_id = :id AND is_active = 1",
        params! {
            "group_id" => group_id.to_string(),
            "id" => category_id.to_string(),
        },
    )?;
    Ok(conn.affected_rows() > 0)
}

/// The name of the group category a new group transaction is filed under.
///
/// `Some` only when the transaction is tagged into `group_id`, `username`
/// belongs to that group and `category_id` is one of its active `kind`
/// categories - then the member's own category check is skipped, because a
/// group transaction deliberately uses the group's categories instead.
pub fn resolve_group_category_name(
    conn: &mut PooledConn,
    group_id: Option<Uuid>,
    category_id: Uuid,
    kind: &str,
    username: &str,
) -> Option<String> {
    let group_id = group_id?;
    match crate::repository::group_repository::is_group_member(conn, group_id, username) {
        Ok(true) => {}
        _ => return None,
    }
    select_group_category(conn, group_id, category_id, kind)
        .ok()
        .flatten()
        .map(|category| category.category_name)
}
