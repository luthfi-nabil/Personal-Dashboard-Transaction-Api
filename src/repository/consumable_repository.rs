use chrono::{Local, NaiveDateTime};
use mysql::prelude::*;
use mysql::*;
use std::error::Error;
use uuid::Uuid;

use crate::models::consumable::Consumable;

pub fn create_consumable_table(conn: &mut PooledConn) -> Result<()> {
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS consumable (
            consumable_id CHAR(36) PRIMARY KEY,
            item_name VARCHAR(255) NOT NULL,
            notes TEXT NULL,
            unit_index INTEGER NOT NULL DEFAULT 1,
            unit_total INTEGER NOT NULL DEFAULT 1,
            price DOUBLE NOT NULL DEFAULT 0,
            in_date DATETIME NOT NULL,
            out_date DATETIME NULL,
            spending_id CHAR(36) NULL,
            spending_detail_id CHAR(36) NULL,
            created_date DATETIME NOT NULL,
            updated_date DATETIME NOT NULL,
            created_by VARCHAR(255) NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            INDEX idx_consumable_created_by (created_by),
            INDEX idx_consumable_spending (spending_id)
        )",
    )?;
    Ok(())
}

/// Everything the user still owns, units in use first (an empty `out_date`
/// sorts ahead), then most recently acquired.
pub fn select_consumables(
    conn: &mut PooledConn,
    created_by: &str,
) -> Result<Vec<Consumable>, Box<dyn Error>> {
    // `created_by` and `is_active` are left out of the projection on purpose:
    // both are pinned by the WHERE clause, and `FromRow` is only implemented
    // for tuples up to 12 columns.
    let rows = conn.exec_map(
        "SELECT consumable_id, item_name, COALESCE(notes, '') AS notes,
            unit_index, unit_total, price, in_date, out_date,
            spending_id, spending_detail_id, created_date, updated_date
         FROM consumable
         WHERE created_by = :created_by AND is_active = 1
         ORDER BY (out_date IS NOT NULL), in_date DESC, item_name ASC, unit_index ASC",
        params! { "created_by" => created_by },
        |(
            consumable_id,
            item_name,
            notes,
            unit_index,
            unit_total,
            price,
            in_date,
            out_date,
            spending_id,
            spending_detail_id,
            created_date,
            updated_date,
        ): (
            String,
            String,
            String,
            i32,
            i32,
            f64,
            NaiveDateTime,
            Option<NaiveDateTime>,
            Option<String>,
            Option<String>,
            NaiveDateTime,
            NaiveDateTime,
        )| Consumable {
            consumable_id: Uuid::parse_str(&consumable_id).unwrap_or_else(|_| Uuid::nil()),
            item_name,
            notes,
            unit_index,
            unit_total,
            price,
            in_date,
            out_date,
            spending_id: spending_id.and_then(|id| Uuid::parse_str(&id).ok()),
            spending_detail_id: spending_detail_id.and_then(|id| Uuid::parse_str(&id).ok()),
            created_date,
            updated_date,
            created_by: created_by.to_string(),
            is_active: 1,
        },
    )?;
    Ok(rows)
}

/// Upsert, so a client retrying a queued offline write with the same id does
/// not end up with two units.
pub fn upsert_consumable(conn: &mut PooledConn, item: &Consumable) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "INSERT INTO consumable (
            consumable_id, item_name, notes, unit_index, unit_total, price,
            in_date, out_date, spending_id, spending_detail_id,
            created_date, updated_date, created_by, is_active
        ) VALUES (
            :id, :name, :notes, :unit_index, :unit_total, :price,
            :in_date, :out_date, :spending_id, :spending_detail_id,
            :created, :updated, :created_by, :active
        )
        ON DUPLICATE KEY UPDATE
            item_name = VALUES(item_name),
            notes = VALUES(notes),
            unit_index = VALUES(unit_index),
            unit_total = VALUES(unit_total),
            price = VALUES(price),
            in_date = VALUES(in_date),
            out_date = VALUES(out_date),
            spending_id = VALUES(spending_id),
            spending_detail_id = VALUES(spending_detail_id),
            updated_date = VALUES(updated_date),
            is_active = VALUES(is_active)",
        params! {
            "id" => item.consumable_id.to_string(),
            "name" => &item.item_name,
            "notes" => &item.notes,
            "unit_index" => item.unit_index,
            "unit_total" => item.unit_total,
            "price" => item.price,
            "in_date" => item.in_date.to_string(),
            "out_date" => item.out_date.map(|d| d.to_string()),
            "spending_id" => item.spending_id.map(|id| id.to_string()),
            "spending_detail_id" => item.spending_detail_id.map(|id| id.to_string()),
            "created" => item.created_date.to_string(),
            "updated" => item.updated_date.to_string(),
            "created_by" => &item.created_by,
            "active" => item.is_active,
        },
    )?;
    Ok(())
}

/// Marks a unit as used up, or puts it back in use when `out_date` is `None`.
/// Returns the number of rows touched so the caller can report "not found".
pub fn update_consumable_out_date(
    conn: &mut PooledConn,
    consumable_id: &str,
    created_by: &str,
    out_date: Option<NaiveDateTime>,
) -> Result<u64, Box<dyn Error>> {
    conn.exec_drop(
        "UPDATE consumable SET out_date = :out_date, updated_date = :now
         WHERE consumable_id = :id AND created_by = :created_by AND is_active = 1",
        params! {
            "id" => consumable_id,
            "created_by" => created_by,
            "out_date" => out_date.map(|d| d.to_string()),
            "now" => Local::now().naive_local().to_string(),
        },
    )?;
    Ok(conn.affected_rows())
}

pub fn remove_consumable(
    conn: &mut PooledConn,
    consumable_id: &str,
    created_by: &str,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "UPDATE consumable SET is_active = 0, updated_date = :now
         WHERE consumable_id = :id AND created_by = :created_by",
        params! {
            "id" => consumable_id,
            "created_by" => created_by,
            "now" => Local::now().naive_local().to_string(),
        },
    )?;
    Ok(())
}
