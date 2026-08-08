use chrono::{Local, NaiveDateTime};
use mysql::prelude::*;
use mysql::*;
use std::error::Error;
use uuid::Uuid;

use crate::models::investment::Investment;

pub fn create_investment_table(conn: &mut PooledConn) -> Result<()> {
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS investment (
            investment_id CHAR(36) PRIMARY KEY,
            kind VARCHAR(32) NOT NULL,
            name VARCHAR(255) NOT NULL,
            provider VARCHAR(255) NOT NULL DEFAULT '',
            units DOUBLE NOT NULL DEFAULT 0,
            buy_unit_price DOUBLE NOT NULL DEFAULT 0,
            last_unit_price DOUBLE NOT NULL DEFAULT 0,
            price_source VARCHAR(64) NOT NULL DEFAULT '',
            price_updated_date DATETIME NULL,
            notes TEXT NULL,
            acquired_date DATETIME NOT NULL,
            created_date DATETIME NOT NULL,
            updated_date DATETIME NOT NULL,
            created_by VARCHAR(255) NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            INDEX idx_investment_created_by (created_by),
            INDEX idx_investment_kind (kind, created_by)
        )",
    )?;
    Ok(())
}

/// Everything the user still holds, grouped by kind so the client can render
/// its sections without re-sorting, then newest acquisition first.
///
/// Read through `Row` rather than a tuple because `FromRow` only goes up to 12
/// columns and this projection is 13.
pub fn select_investments(
    conn: &mut PooledConn,
    created_by: &str,
) -> Result<Vec<Investment>, Box<dyn Error>> {
    let rows: Vec<Row> = conn.exec(
        "SELECT investment_id, kind, name, provider, units,
            buy_unit_price, last_unit_price, price_source, price_updated_date,
            COALESCE(notes, '') AS notes, acquired_date, created_date, updated_date
         FROM investment
         WHERE created_by = :created_by AND is_active = 1
         ORDER BY
           CASE kind WHEN 'mutual_fund' THEN 0 WHEN 'gold' THEN 1 ELSE 2 END,
           acquired_date DESC,
           name ASC",
        params! { "created_by" => created_by },
    )?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let investment_id = row
                .get::<Option<String>, _>("investment_id")
                .flatten()
                .unwrap_or_default();
            Investment {
                investment_id: Uuid::parse_str(&investment_id).unwrap_or_else(|_| Uuid::nil()),
                kind: row
                    .get::<Option<String>, _>("kind")
                    .flatten()
                    .unwrap_or_else(|| "mutual_fund".to_string()),
                name: row
                    .get::<Option<String>, _>("name")
                    .flatten()
                    .unwrap_or_default(),
                provider: row
                    .get::<Option<String>, _>("provider")
                    .flatten()
                    .unwrap_or_default(),
                units: row
                    .get::<Option<f64>, _>("units")
                    .flatten()
                    .unwrap_or_default(),
                buy_unit_price: row
                    .get::<Option<f64>, _>("buy_unit_price")
                    .flatten()
                    .unwrap_or_default(),
                last_unit_price: row
                    .get::<Option<f64>, _>("last_unit_price")
                    .flatten()
                    .unwrap_or_default(),
                price_source: row
                    .get::<Option<String>, _>("price_source")
                    .flatten()
                    .unwrap_or_default(),
                price_updated_date: row
                    .get::<Option<NaiveDateTime>, _>("price_updated_date")
                    .flatten(),
                notes: row
                    .get::<Option<String>, _>("notes")
                    .flatten()
                    .unwrap_or_default(),
                acquired_date: row.get("acquired_date").unwrap_or_default(),
                created_date: row.get("created_date").unwrap_or_default(),
                updated_date: row.get("updated_date").unwrap_or_default(),
                created_by: created_by.to_string(),
                is_active: 1,
            }
        })
        .collect())
}

/// Upsert, so a client retrying a queued offline write with the same id does
/// not end up holding the same thing twice.
///
/// `created_date` is left alone on update: an edit must not make an old
/// holding look new.
pub fn upsert_investment(conn: &mut PooledConn, item: &Investment) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "INSERT INTO investment (
            investment_id, kind, name, provider, units,
            buy_unit_price, last_unit_price, price_source, price_updated_date,
            notes, acquired_date, created_date, updated_date, created_by, is_active
        ) VALUES (
            :id, :kind, :name, :provider, :units,
            :buy_unit_price, :last_unit_price, :price_source, :price_updated,
            :notes, :acquired, :created, :updated, :created_by, :active
        )
        ON DUPLICATE KEY UPDATE
            kind = VALUES(kind),
            name = VALUES(name),
            provider = VALUES(provider),
            units = VALUES(units),
            buy_unit_price = VALUES(buy_unit_price),
            last_unit_price = VALUES(last_unit_price),
            price_source = VALUES(price_source),
            price_updated_date = VALUES(price_updated_date),
            notes = VALUES(notes),
            acquired_date = VALUES(acquired_date),
            updated_date = VALUES(updated_date),
            is_active = VALUES(is_active)",
        params! {
            "id" => item.investment_id.to_string(),
            "kind" => &item.kind,
            "name" => &item.name,
            "provider" => &item.provider,
            "units" => item.units,
            "buy_unit_price" => item.buy_unit_price,
            "last_unit_price" => item.last_unit_price,
            "price_source" => &item.price_source,
            "price_updated" => item.price_updated_date.map(|d| d.to_string()),
            "notes" => &item.notes,
            "acquired" => item.acquired_date.to_string(),
            "created" => item.created_date.to_string(),
            "updated" => item.updated_date.to_string(),
            "created_by" => &item.created_by,
            "active" => item.is_active,
        },
    )?;
    Ok(())
}

/// Records a fresh valuation without touching units or cost basis. Returns the
/// number of rows changed so the caller can report "not found".
pub fn update_investment_price(
    conn: &mut PooledConn,
    investment_id: &str,
    created_by: &str,
    last_unit_price: f64,
    price_source: &str,
    price_updated_date: NaiveDateTime,
) -> Result<u64, Box<dyn Error>> {
    conn.exec_drop(
        "UPDATE investment
         SET last_unit_price = :price,
             price_source = :source,
             price_updated_date = :price_updated,
             updated_date = :now
         WHERE investment_id = :id AND created_by = :created_by AND is_active = 1",
        params! {
            "id" => investment_id,
            "created_by" => created_by,
            "price" => last_unit_price,
            "source" => price_source,
            "price_updated" => price_updated_date.to_string(),
            "now" => Local::now().naive_local().to_string(),
        },
    )?;
    Ok(conn.affected_rows())
}

pub fn remove_investment(
    conn: &mut PooledConn,
    investment_id: &str,
    created_by: &str,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "UPDATE investment SET is_active = 0, updated_date = :now
         WHERE investment_id = :id AND created_by = :created_by",
        params! {
            "id" => investment_id,
            "created_by" => created_by,
            "now" => Local::now().naive_local().to_string(),
        },
    )?;
    Ok(())
}
