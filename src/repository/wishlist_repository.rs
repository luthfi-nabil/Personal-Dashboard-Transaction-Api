use chrono::Local;
use mysql::prelude::*;
use mysql::*;
use std::error::Error;
use uuid::Uuid;

use crate::models::wishlist::{PlannedExpenseCategory, PlannedExpenseItem};

pub fn create_wishlist_table(conn: &mut PooledConn) -> Result<()> {
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS wishlist (
            wishlist_id CHAR(36) PRIMARY KEY,
            item_name VARCHAR(255) NOT NULL,
            price DOUBLE NOT NULL,
            transaction_type VARCHAR(32) NOT NULL DEFAULT 'spending',
            category_id CHAR(36) NULL,
            category VARCHAR(255) NULL,
            notes TEXT NULL,
            priority VARCHAR(32) NOT NULL,
            status VARCHAR(32) NOT NULL DEFAULT 'active',
            fulfilled_price DOUBLE NULL,
            fulfilled_at DATETIME NULL,
            canceled_at DATETIME NULL,
            created_date DATETIME NOT NULL,
            updated_date DATETIME NOT NULL,
            created_by VARCHAR(255) NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1
        )",
    )?;
    add_column_if_missing(
        conn,
        "wishlist",
        "transaction_type",
        "ALTER TABLE wishlist ADD COLUMN transaction_type VARCHAR(32) NOT NULL DEFAULT 'spending' AFTER price",
    )?;
    add_column_if_missing(
        conn,
        "wishlist",
        "category_id",
        "ALTER TABLE wishlist ADD COLUMN category_id CHAR(36) NULL AFTER transaction_type",
    )?;
    add_column_if_missing(
        conn,
        "wishlist",
        "category",
        "ALTER TABLE wishlist ADD COLUMN category VARCHAR(255) NULL AFTER category_id",
    )?;
    Ok(())
}

pub fn create_planned_expense_category_table(conn: &mut PooledConn) -> Result<()> {
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS planned_expense_category (
            planned_expense_category_id CHAR(36) PRIMARY KEY,
            planned_expense_category VARCHAR(255) NOT NULL,
            created_date DATETIME NOT NULL,
            created_by VARCHAR(255) NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            UNIQUE KEY planned_expense_category_user (planned_expense_category, created_by)
        )",
    )?;
    Ok(())
}

fn add_column_if_missing(
    conn: &mut PooledConn,
    table: &str,
    column: &str,
    alter_sql: &str,
) -> Result<()> {
    let exists: Option<u8> = conn.exec_first(
        "SELECT 1
         FROM INFORMATION_SCHEMA.COLUMNS
         WHERE TABLE_SCHEMA = DATABASE()
           AND TABLE_NAME = :table
           AND COLUMN_NAME = :column
         LIMIT 1",
        params! {
            "table" => table,
            "column" => column,
        },
    )?;
    if exists.is_none() {
        conn.query_drop(alter_sql)?;
    }
    Ok(())
}

pub fn select_wishlist(
    conn: &mut PooledConn,
    created_by: &str,
) -> Result<Vec<PlannedExpenseItem>, Box<dyn Error>> {
    let rows: Vec<Row> = conn.exec(
        "SELECT wishlist_id, item_name, price, transaction_type, category_id, category,
            notes, priority, status,
            fulfilled_price, fulfilled_at, canceled_at, created_date, updated_date,
            created_by
         FROM wishlist
         WHERE created_by = :created_by AND is_active = 1
         ORDER BY
           CASE status WHEN 'active' THEN 0 ELSE 1 END,
           CASE priority WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END,
            updated_date DESC",
        params! { "created_by" => created_by },
    )?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let wishlist_id = row
                .get::<Option<String>, _>("wishlist_id")
                .flatten()
                .unwrap_or_default();
            let category_id = row.get::<Option<String>, _>("category_id").flatten();
            PlannedExpenseItem {
                planned_expense_id: Uuid::parse_str(&wishlist_id).unwrap_or_else(|_| Uuid::nil()),
                item_name: row
                    .get::<Option<String>, _>("item_name")
                    .flatten()
                    .unwrap_or_default(),
                price: row
                    .get::<Option<f64>, _>("price")
                    .flatten()
                    .unwrap_or_default(),
                transaction_type: row
                    .get::<Option<String>, _>("transaction_type")
                    .flatten()
                    .unwrap_or_else(|| "spending".to_string()),
                category_id: category_id
                    .as_deref()
                    .and_then(|id| Uuid::parse_str(id).ok()),
                category: row.get::<Option<String>, _>("category").flatten(),
                notes: row.get::<Option<String>, _>("notes").flatten(),
                priority: row
                    .get::<Option<String>, _>("priority")
                    .flatten()
                    .unwrap_or_else(|| "medium".to_string()),
                status: row
                    .get::<Option<String>, _>("status")
                    .flatten()
                    .unwrap_or_else(|| "active".to_string()),
                fulfilled_price: row.get::<Option<f64>, _>("fulfilled_price").flatten(),
                fulfilled_at: row
                    .get::<Option<chrono::NaiveDateTime>, _>("fulfilled_at")
                    .flatten(),
                canceled_at: row
                    .get::<Option<chrono::NaiveDateTime>, _>("canceled_at")
                    .flatten(),
                created_date: row.get("created_date").unwrap(),
                updated_date: row.get("updated_date").unwrap(),
                created_by: row
                    .get::<Option<String>, _>("created_by")
                    .flatten()
                    .unwrap_or_default(),
                is_active: 1,
            }
        })
        .collect())
}

pub fn upsert_wishlist(
    conn: &mut PooledConn,
    item: &PlannedExpenseItem,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "INSERT INTO wishlist (
            wishlist_id, item_name, price, transaction_type, category_id, category,
            notes, priority, status,
            fulfilled_price, fulfilled_at, canceled_at, created_date, updated_date,
            created_by, is_active
        ) VALUES (
            :id, :name, :price, :transaction_type, :category_id, :category,
            :notes, :priority, :status,
            :fulfilled_price, :fulfilled_at, :canceled_at, :created, :updated,
            :created_by, :active
        )
        ON DUPLICATE KEY UPDATE
            item_name = VALUES(item_name),
            price = VALUES(price),
            transaction_type = VALUES(transaction_type),
            category_id = VALUES(category_id),
            category = VALUES(category),
            notes = VALUES(notes),
            priority = VALUES(priority),
            status = VALUES(status),
            fulfilled_price = VALUES(fulfilled_price),
            fulfilled_at = VALUES(fulfilled_at),
            canceled_at = VALUES(canceled_at),
            updated_date = VALUES(updated_date),
            is_active = VALUES(is_active)",
        params! {
            "id" => item.planned_expense_id.to_string(),
            "name" => &item.item_name,
            "price" => item.price,
            "transaction_type" => &item.transaction_type,
            "category_id" => item.category_id.map(|id| id.to_string()),
            "category" => &item.category,
            "notes" => &item.notes,
            "priority" => &item.priority,
            "status" => &item.status,
            "fulfilled_price" => item.fulfilled_price,
            "fulfilled_at" => item.fulfilled_at.map(|d| d.to_string()),
            "canceled_at" => item.canceled_at.map(|d| d.to_string()),
            "created" => item.created_date.to_string(),
            "updated" => item.updated_date.to_string(),
            "created_by" => &item.created_by,
            "active" => item.is_active,
        },
    )?;
    Ok(())
}

pub fn update_wishlist_status(
    conn: &mut PooledConn,
    wishlist_id: &str,
    created_by: &str,
    status: &str,
    fulfilled_price: Option<f64>,
) -> Result<(), Box<dyn Error>> {
    let now = Local::now().naive_local();
    conn.exec_drop(
        "UPDATE wishlist
         SET status = :status,
             fulfilled_price = CASE WHEN :status = 'fulfilled' THEN :fulfilled_price ELSE fulfilled_price END,
             fulfilled_at = CASE WHEN :status = 'fulfilled' THEN :now ELSE fulfilled_at END,
             canceled_at = CASE WHEN :status = 'canceled' THEN :now ELSE canceled_at END,
             updated_date = :now
         WHERE wishlist_id = :id AND created_by = :created_by",
        params! {
            "id" => wishlist_id,
            "created_by" => created_by,
            "status" => status,
            "fulfilled_price" => fulfilled_price,
            "now" => now.to_string(),
        },
    )?;
    Ok(())
}

pub fn remove_wishlist(
    conn: &mut PooledConn,
    wishlist_id: &str,
    created_by: &str,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "UPDATE wishlist SET is_active = 0, updated_date = :now
         WHERE wishlist_id = :id AND created_by = :created_by",
        params! {
            "id" => wishlist_id,
            "created_by" => created_by,
            "now" => Local::now().naive_local().to_string(),
        },
    )?;
    Ok(())
}

pub fn select_planned_expense_categories(
    conn: &mut PooledConn,
    created_by: &str,
) -> Result<Vec<PlannedExpenseCategory>, Box<dyn Error>> {
    let rows = conn.exec_map(
        "SELECT planned_expense_category_id, planned_expense_category,
            created_date, created_by, is_active
         FROM planned_expense_category
         WHERE created_by = :created_by AND is_active = 1
         ORDER BY planned_expense_category ASC",
        params! { "created_by" => created_by },
        |(
            planned_expense_category_id,
            planned_expense_category,
            created_date,
            created_by,
            is_active,
        ): (String, String, chrono::NaiveDateTime, String, i32)| PlannedExpenseCategory {
            planned_expense_category_id: Uuid::parse_str(&planned_expense_category_id)
                .unwrap_or_else(|_| Uuid::nil()),
            planned_expense_category,
            created_date,
            created_by,
            is_active,
        },
    )?;
    Ok(rows)
}

pub fn insert_planned_expense_category(
    conn: &mut PooledConn,
    category: &PlannedExpenseCategory,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "INSERT INTO planned_expense_category (
            planned_expense_category_id, planned_expense_category,
            created_date, created_by, is_active
        ) VALUES (
            :id, :category, :created_date, :created_by, :is_active
        )",
        params! {
            "id" => category.planned_expense_category_id.to_string(),
            "category" => &category.planned_expense_category,
            "created_date" => category.created_date.to_string(),
            "created_by" => &category.created_by,
            "is_active" => category.is_active,
        },
    )?;
    Ok(())
}

pub fn delete_planned_expense_category(
    conn: &mut PooledConn,
    category_id: &str,
    created_by: &str,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "UPDATE planned_expense_category
         SET is_active = 0
         WHERE planned_expense_category_id = :id AND created_by = :created_by",
        params! {
            "id" => category_id,
            "created_by" => created_by,
        },
    )?;
    Ok(())
}
