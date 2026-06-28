use chrono::Local;
use mysql::prelude::*;
use mysql::*;
use std::error::Error;
use uuid::Uuid;

use crate::models::activity::ActivityCategory;

pub fn create_activity_category_table(conn: &mut PooledConn) -> Result<()> {
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS activity_category (
            activity_category_id CHAR(36) PRIMARY KEY,
            activity_category VARCHAR(255) NOT NULL,
            created_date DATETIME NOT NULL,
            created_by VARCHAR(255) NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            UNIQUE KEY activity_category_user (activity_category, created_by)
        )",
    )?;
    Ok(())
}

pub fn select_activity_categories(
    conn: &mut PooledConn,
    created_by: &str,
) -> Result<Vec<ActivityCategory>, Box<dyn Error>> {
    let rows = conn.exec_map(
        "SELECT activity_category_id, activity_category,
            created_date, created_by, is_active
         FROM activity_category
         WHERE created_by = :created_by AND is_active = 1
         ORDER BY activity_category ASC",
        params! { "created_by" => created_by },
        |(activity_category_id, activity_category, created_date, created_by, is_active): (
            String,
            String,
            chrono::NaiveDateTime,
            String,
            i32,
        )| ActivityCategory {
            activity_category_id: Uuid::parse_str(&activity_category_id)
                .unwrap_or_else(|_| Uuid::nil()),
            activity_category,
            created_date,
            created_by,
            is_active,
        },
    )?;
    Ok(rows)
}

pub fn select_activity_category_by_name(
    conn: &mut PooledConn,
    name: &str,
    created_by: &str,
) -> Result<Option<ActivityCategory>, Box<dyn Error>> {
    let row: Option<Row> = conn.exec_first(
        "SELECT activity_category_id, activity_category,
            created_date, created_by, is_active
         FROM activity_category
         WHERE activity_category = :name AND created_by = :created_by AND is_active = 1
         LIMIT 1",
        params! {
            "name" => name,
            "created_by" => created_by,
        },
    )?;
    Ok(row.map(|row| {
        let id = row
            .get::<Option<String>, _>("activity_category_id")
            .flatten()
            .unwrap_or_default();
        ActivityCategory {
            activity_category_id: Uuid::parse_str(&id).unwrap_or_else(|_| Uuid::nil()),
            activity_category: row
                .get::<Option<String>, _>("activity_category")
                .flatten()
                .unwrap_or_default(),
            created_date: row
                .get("created_date")
                .unwrap_or_else(|| Local::now().naive_local()),
            created_by: row
                .get::<Option<String>, _>("created_by")
                .flatten()
                .unwrap_or_default(),
            is_active: row
                .get::<Option<i32>, _>("is_active")
                .flatten()
                .unwrap_or(1),
        }
    }))
}

pub fn upsert_activity_category(
    conn: &mut PooledConn,
    category: &ActivityCategory,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "INSERT INTO activity_category (
            activity_category_id, activity_category,
            created_date, created_by, is_active
        ) VALUES (
            :id, :category, :created_date, :created_by, :is_active
        )
        ON DUPLICATE KEY UPDATE
            is_active = VALUES(is_active)",
        params! {
            "id" => category.activity_category_id.to_string(),
            "category" => &category.activity_category,
            "created_date" => category.created_date.to_string(),
            "created_by" => &category.created_by,
            "is_active" => category.is_active,
        },
    )?;
    Ok(())
}

pub fn delete_activity_category(
    conn: &mut PooledConn,
    category_id: &str,
    created_by: &str,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "UPDATE activity_category
         SET is_active = 0
         WHERE activity_category_id = :id AND created_by = :created_by",
        params! {
            "id" => category_id,
            "created_by" => created_by,
        },
    )?;
    Ok(())
}
