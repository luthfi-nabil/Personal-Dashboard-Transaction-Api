use chrono::NaiveDateTime;
use mysql::prelude::*;
use mysql::*;
use std::error::Error;
use uuid::Uuid;

use crate::models::planned_transaction::{PlannedTransaction, PlannedTransactionDetail};

pub fn create_planned_transaction_table(conn: &mut PooledConn) -> Result<()> {
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS planned_transaction (
            planned_transaction_id CHAR(36) PRIMARY KEY,
            name VARCHAR(255) NOT NULL,
            created_date DATETIME NOT NULL,
            updated_date DATETIME NOT NULL,
            created_by VARCHAR(255) NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            INDEX idx_planned_transaction_created_by (created_by)
        )",
    )?;
    Ok(())
}

pub fn create_planned_transaction_detail_table(conn: &mut PooledConn) -> Result<()> {
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS planned_transaction_detail (
            planned_transaction_detail_id CHAR(36) PRIMARY KEY,
            planned_transaction_id CHAR(36) NOT NULL,
            item_name VARCHAR(255) NOT NULL,
            quantity DOUBLE NOT NULL DEFAULT 1,
            unit_price DOUBLE NOT NULL DEFAULT 0,
            amount DOUBLE NOT NULL DEFAULT 0,
            note TEXT NULL,
            spending_id CHAR(36) NULL,
            spending_detail_id CHAR(36) NULL,
            created_date DATETIME NOT NULL,
            created_by VARCHAR(255) NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            INDEX idx_planned_transaction_detail_parent (planned_transaction_id),
            INDEX idx_planned_transaction_detail_created_by (created_by)
        )",
    )?;
    Ok(())
}

/// Bundles for the user, most recently touched first.
pub fn select_planned_transactions(
    conn: &mut PooledConn,
    created_by: &str,
) -> Result<Vec<PlannedTransaction>, Box<dyn Error>> {
    let rows = conn.exec_map(
        "SELECT planned_transaction_id, name, created_date, updated_date
         FROM planned_transaction
         WHERE created_by = :created_by AND is_active = 1
         ORDER BY updated_date DESC",
        params! { "created_by" => created_by },
        |(planned_transaction_id, name, created_date, updated_date): (
            String,
            String,
            NaiveDateTime,
            NaiveDateTime,
        )| PlannedTransaction {
            planned_transaction_id: Uuid::parse_str(&planned_transaction_id)
                .unwrap_or_else(|_| Uuid::nil()),
            name,
            created_date,
            updated_date,
            created_by: created_by.to_string(),
            is_active: 1,
        },
    )?;
    Ok(rows)
}

/// Confirms a bundle exists and belongs to `created_by`, so a detail can't be
/// posted onto someone else's (or a nonexistent) header.
pub fn planned_transaction_exists(
    conn: &mut PooledConn,
    planned_transaction_id: Uuid,
    created_by: &str,
) -> Result<bool, Box<dyn Error>> {
    let found: Option<u8> = conn.exec_first(
        "SELECT 1 FROM planned_transaction
         WHERE planned_transaction_id = :id AND created_by = :created_by AND is_active = 1
         LIMIT 1",
        params! {
            "id" => planned_transaction_id.to_string(),
            "created_by" => created_by,
        },
    )?;
    Ok(found.is_some())
}

/// Upsert, so a client retrying a queued offline write with the same id does
/// not end up with two bundles.
pub fn upsert_planned_transaction(
    conn: &mut PooledConn,
    item: &PlannedTransaction,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "INSERT INTO planned_transaction (
            planned_transaction_id, name, created_date, updated_date, created_by, is_active
        ) VALUES (
            :id, :name, :created, :updated, :created_by, :active
        )
        ON DUPLICATE KEY UPDATE
            name = VALUES(name),
            updated_date = VALUES(updated_date),
            is_active = VALUES(is_active)",
        params! {
            "id" => item.planned_transaction_id.to_string(),
            "name" => &item.name,
            "created" => item.created_date.to_string(),
            "updated" => item.updated_date.to_string(),
            "created_by" => &item.created_by,
            "active" => item.is_active,
        },
    )?;
    Ok(())
}

/// Line items, optionally scoped to one bundle.
pub fn select_planned_transaction_details(
    conn: &mut PooledConn,
    planned_transaction_id: Option<Uuid>,
    created_by: &str,
) -> Result<Vec<PlannedTransactionDetail>, Box<dyn Error>> {
    let mut query = String::from(
        "SELECT planned_transaction_detail_id, planned_transaction_id, item_name, quantity,
            unit_price, amount, COALESCE(note, '') AS note, spending_id, spending_detail_id,
            created_date
         FROM planned_transaction_detail
         WHERE created_by = ? AND is_active = 1",
    );
    let mut params: Vec<mysql::Value> = vec![created_by.into()];
    if let Some(id) = planned_transaction_id {
        query.push_str(" AND planned_transaction_id = ?");
        params.push(id.to_string().into());
    }
    query.push_str(" ORDER BY created_date ASC, item_name ASC");

    let rows = conn.exec_map(
        query,
        params,
        |(
            planned_transaction_detail_id,
            planned_transaction_id,
            item_name,
            quantity,
            unit_price,
            amount,
            note,
            spending_id,
            spending_detail_id,
            created_date,
        ): (
            String,
            String,
            String,
            f64,
            f64,
            f64,
            String,
            Option<String>,
            Option<String>,
            NaiveDateTime,
        )| PlannedTransactionDetail {
            planned_transaction_detail_id: Uuid::parse_str(&planned_transaction_detail_id)
                .unwrap_or_else(|_| Uuid::nil()),
            planned_transaction_id: Uuid::parse_str(&planned_transaction_id)
                .unwrap_or_else(|_| Uuid::nil()),
            item_name,
            quantity,
            unit_price,
            amount,
            note,
            spending_id: spending_id.and_then(|id| Uuid::parse_str(&id).ok()),
            spending_detail_id: spending_detail_id.and_then(|id| Uuid::parse_str(&id).ok()),
            created_date,
            created_by: created_by.to_string(),
            is_active: 1,
        },
    )?;
    Ok(rows)
}

/// Upsert, mirroring [upsert_planned_transaction].
pub fn upsert_planned_transaction_detail(
    conn: &mut PooledConn,
    item: &PlannedTransactionDetail,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "INSERT INTO planned_transaction_detail (
            planned_transaction_detail_id, planned_transaction_id, item_name, quantity,
            unit_price, amount, note, spending_id, spending_detail_id,
            created_date, created_by, is_active
        ) VALUES (
            :id, :planned_transaction_id, :item_name, :quantity,
            :unit_price, :amount, :note, :spending_id, :spending_detail_id,
            :created, :created_by, :active
        )
        ON DUPLICATE KEY UPDATE
            item_name = VALUES(item_name),
            quantity = VALUES(quantity),
            unit_price = VALUES(unit_price),
            amount = VALUES(amount),
            note = VALUES(note),
            spending_id = VALUES(spending_id),
            spending_detail_id = VALUES(spending_detail_id),
            is_active = VALUES(is_active)",
        params! {
            "id" => item.planned_transaction_detail_id.to_string(),
            "planned_transaction_id" => item.planned_transaction_id.to_string(),
            "item_name" => &item.item_name,
            "quantity" => item.quantity,
            "unit_price" => item.unit_price,
            "amount" => item.amount,
            "note" => &item.note,
            "spending_id" => item.spending_id.map(|id| id.to_string()),
            "spending_detail_id" => item.spending_detail_id.map(|id| id.to_string()),
            "created" => item.created_date.to_string(),
            "created_by" => &item.created_by,
            "active" => item.is_active,
        },
    )?;
    Ok(())
}
