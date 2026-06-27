use chrono::Local;
use mysql::prelude::*;
use mysql::*;
use std::error::Error;
use uuid::Uuid;

use crate::models::routine::{RoutinePayment, RoutineTransaction};

pub fn create_routine_table(conn: &mut PooledConn) -> Result<()> {
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS routine_transaction (
            routine_id CHAR(36) PRIMARY KEY,
            item_name VARCHAR(255) NOT NULL,
            price DOUBLE NOT NULL,
            reminder VARCHAR(64) NOT NULL,
            spending_category_id CHAR(36) NOT NULL,
            spending_category VARCHAR(255) NOT NULL,
            status VARCHAR(32) NOT NULL DEFAULT 'active',
            last_bought_at DATETIME NULL,
            created_date DATETIME NOT NULL,
            updated_date DATETIME NOT NULL,
            created_by VARCHAR(255) NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1
        )",
    )?;
    Ok(())
}

pub fn create_routine_payment_table(conn: &mut PooledConn) -> Result<()> {
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS routine_payment (
            routine_payment_id CHAR(36) PRIMARY KEY,
            routine_id CHAR(36) NOT NULL,
            item_name VARCHAR(255) NOT NULL,
            price DOUBLE NOT NULL,
            spending_category_id CHAR(36) NOT NULL,
            spending_category VARCHAR(255) NOT NULL,
            source_id CHAR(36) NOT NULL,
            source VARCHAR(255) NOT NULL,
            bought_at DATETIME NOT NULL,
            created_by VARCHAR(255) NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1
        )",
    )?;
    Ok(())
}

pub fn select_routines(
    conn: &mut PooledConn,
    created_by: &str,
) -> Result<Vec<RoutineTransaction>, Box<dyn Error>> {
    let rows = conn.exec_map(
        "SELECT routine_id, item_name, price, reminder, spending_category_id,
            spending_category, status, last_bought_at, created_date, updated_date,
            created_by, is_active
         FROM routine_transaction
         WHERE created_by = :created_by AND is_active = 1
         ORDER BY updated_date DESC",
        params! { "created_by" => created_by },
        |(
            routine_id,
            item_name,
            price,
            reminder,
            spending_category_id,
            spending_category,
            status,
            last_bought_at,
            created_date,
            updated_date,
            created_by,
            is_active,
        ): (
            String,
            String,
            f64,
            String,
            String,
            String,
            String,
            Option<chrono::NaiveDateTime>,
            chrono::NaiveDateTime,
            chrono::NaiveDateTime,
            String,
            i32,
        )| RoutineTransaction {
            routine_id: Uuid::parse_str(&routine_id).unwrap_or_else(|_| Uuid::nil()),
            item_name,
            price,
            reminder,
            spending_category_id: Uuid::parse_str(&spending_category_id)
                .unwrap_or_else(|_| Uuid::nil()),
            spending_category,
            status,
            last_bought_at,
            created_date,
            updated_date,
            created_by,
            is_active,
        },
    )?;
    Ok(rows)
}

pub fn select_routine_payments(
    conn: &mut PooledConn,
    created_by: &str,
) -> Result<Vec<RoutinePayment>, Box<dyn Error>> {
    let rows = conn.exec_map(
        "SELECT routine_payment_id, routine_id, item_name, price,
            spending_category_id, spending_category, source_id, source,
            bought_at, created_by, is_active
         FROM routine_payment
         WHERE created_by = :created_by AND is_active = 1
         ORDER BY bought_at DESC",
        params! { "created_by" => created_by },
        |(
            routine_payment_id,
            routine_id,
            item_name,
            price,
            spending_category_id,
            spending_category,
            source_id,
            source,
            bought_at,
            created_by,
            is_active,
        ): (
            String,
            String,
            String,
            f64,
            String,
            String,
            String,
            String,
            chrono::NaiveDateTime,
            String,
            i32,
        )| RoutinePayment {
            routine_payment_id: Uuid::parse_str(&routine_payment_id)
                .unwrap_or_else(|_| Uuid::nil()),
            routine_id: Uuid::parse_str(&routine_id).unwrap_or_else(|_| Uuid::nil()),
            item_name,
            price,
            spending_category_id: Uuid::parse_str(&spending_category_id)
                .unwrap_or_else(|_| Uuid::nil()),
            spending_category,
            source_id: Uuid::parse_str(&source_id).unwrap_or_else(|_| Uuid::nil()),
            source,
            bought_at,
            created_by,
            is_active,
        },
    )?;
    Ok(rows)
}

pub fn upsert_routine(
    conn: &mut PooledConn,
    item: &RoutineTransaction,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "INSERT INTO routine_transaction (
            routine_id, item_name, price, reminder, spending_category_id,
            spending_category, status, last_bought_at, created_date, updated_date,
            created_by, is_active
        ) VALUES (
            :id, :name, :price, :reminder, :cat_id,
            :cat, :status, :last_bought_at, :created, :updated,
            :created_by, :active
        )
        ON DUPLICATE KEY UPDATE
            item_name = VALUES(item_name),
            price = VALUES(price),
            reminder = VALUES(reminder),
            spending_category_id = VALUES(spending_category_id),
            spending_category = VALUES(spending_category),
            status = VALUES(status),
            last_bought_at = VALUES(last_bought_at),
            updated_date = VALUES(updated_date),
            is_active = VALUES(is_active)",
        params! {
            "id" => item.routine_id.to_string(),
            "name" => &item.item_name,
            "price" => item.price,
            "reminder" => &item.reminder,
            "cat_id" => item.spending_category_id.to_string(),
            "cat" => &item.spending_category,
            "status" => &item.status,
            "last_bought_at" => item.last_bought_at.map(|d| d.to_string()),
            "created" => item.created_date.to_string(),
            "updated" => item.updated_date.to_string(),
            "created_by" => &item.created_by,
            "active" => item.is_active,
        },
    )?;
    Ok(())
}

pub fn insert_routine_payment(
    conn: &mut PooledConn,
    payment: &RoutinePayment,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "INSERT INTO routine_payment (
            routine_payment_id, routine_id, item_name, price,
            spending_category_id, spending_category, source_id, source,
            bought_at, created_by, is_active
        ) VALUES (
            :id, :routine_id, :name, :price,
            :cat_id, :cat, :source_id, :source,
            :bought_at, :created_by, :active
        )",
        params! {
            "id" => payment.routine_payment_id.to_string(),
            "routine_id" => payment.routine_id.to_string(),
            "name" => &payment.item_name,
            "price" => payment.price,
            "cat_id" => payment.spending_category_id.to_string(),
            "cat" => &payment.spending_category,
            "source_id" => payment.source_id.to_string(),
            "source" => &payment.source,
            "bought_at" => payment.bought_at.to_string(),
            "created_by" => &payment.created_by,
            "active" => payment.is_active,
        },
    )?;

    conn.exec_drop(
        "UPDATE routine_transaction
         SET last_bought_at = :bought_at, updated_date = :updated, status = 'active'
         WHERE routine_id = :routine_id AND created_by = :created_by",
        params! {
            "routine_id" => payment.routine_id.to_string(),
            "created_by" => &payment.created_by,
            "bought_at" => payment.bought_at.to_string(),
            "updated" => Local::now().naive_local().to_string(),
        },
    )?;
    Ok(())
}

pub fn remove_routine(
    conn: &mut PooledConn,
    routine_id: &str,
    created_by: &str,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "UPDATE routine_transaction SET is_active = 0, updated_date = :now
         WHERE routine_id = :id AND created_by = :created_by",
        params! {
            "id" => routine_id,
            "created_by" => created_by,
            "now" => Local::now().naive_local().to_string(),
        },
    )?;
    Ok(())
}
