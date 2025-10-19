use rusqlite::{Connection, Result};
use crate::models::earning::{Earning, EarningCategory};

pub fn create_earning_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS earning (
            earning_id TEXT PRIMARY KEY,
            total_amount REAL NOT NULL,
            description TEXT,
            earning_category_id TEXT NOT NULL,
            earning_category TEXT NOT NULL,
            source_id TEXT NOT NULL,
            source TEXT NOT NULL,
            created_date TEXT NOT NULL,
            created_by TEXT NOT NULL
        )",
        [],
    )?;
    Ok(())
}

pub fn create_earning_category_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS earning_category (
            earning_category_id TEXT PRIMARY KEY,
            earning_category TEXT NOT NULL,
            created_date TEXT NOT NULL,
            created_by TEXT NOT NULL
        )",
        [],
    )?;
    Ok(())
}

pub fn insert_earning(conn: &Connection, earning: &Earning) -> Result<()> {
    conn.execute(
        "INSERT INTO earning (earning_id, total_amount, description, earning_category_id, earning_category, source_id, source, created_date, created_by)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        [
            earning.earning_id.to_string(),
            earning.total_amount.to_string(),
            earning.description.to_string(),
            earning.earning_category_id.to_string(),
            earning.earning_category.to_string(),
            earning.source_id.to_string(),
            earning.source.to_string(),
            earning.created_date.to_rfc3339(),
            earning.created_by.to_string(),
        ],
    )?;
    Ok(())
}

pub fn insert_earning_category(conn: &Connection, category: &EarningCategory) -> Result<()> {
    conn.execute(
        "INSERT INTO earning_category (earning_category_id, earning_category, created_date, created_by)
         VALUES (?1, ?2, ?3, ?4)",
        [
            category.earning_category_id.to_string(),
            category.earning_category.to_string(),
            category.created_date.to_rfc3339(),
            category.created_by.to_string(),
        ],
    )?;
    Ok(())
}