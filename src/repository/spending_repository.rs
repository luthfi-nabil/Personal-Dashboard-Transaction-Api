use rusqlite::{Connection, Result};
use crate::models::source::{Source};
use crate::models::spending::{Spending, SpendingCategory};
use uuid::Uuid;
use chrono::{Utc, DateTime};

pub fn create_spending_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS spending (
            spending_id TEXT PRIMARY KEY,
            total_amount REAL NOT NULL,
            description TEXT,
            spending_category_id TEXT NOT NULL,
            spending_category TEXT NOT NULL,
            source_id TEXT NOT NULL,
            source TEXT NOT NULL,
            created_date TEXT NOT NULL,
            created_by TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1
        )",
        [],
    )?;
    Ok(())
}

pub fn create_spending_category_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS spending_category (
            spending_category_id TEXT PRIMARY KEY,
            spending_category TEXT NOT NULL UNIQUE,
            created_date TEXT NOT NULL,
            created_by TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1
        )",
        [],
    )?;
    Ok(())
}

pub fn select_spendings(conn: &Connection) -> Result<Vec<Source>> {
    let mut stmt = conn.prepare("SELECT spending_id, total_amount, description, spending_category_id, spending_category, source_id, source, created_date, created_by, is_active FROM spending where is_active = 1")?;
    let spending_iter = stmt.query_map([], |row| {
        Ok(Source {
            source_id: row
            .get::<_, Option<String>>(0)?
                .and_then(|s| if s.is_empty() { None } else { s.parse::<Uuid>().ok() })
                .unwrap_or_else(Uuid::nil),
            source: row.get(1)?,
            created_date: row
            .get::<_, Option<String>>(2)?
                .and_then(|s| if s.is_empty() { None } else { s.parse::<DateTime<Utc>>().ok() })
                .unwrap_or_else(DateTime::default),
            created_by: row.get(3)?,
            is_active:row.get(4)?
        })
    })?;

    Ok(spending_iter.filter_map(Result::ok).collect())
}

pub fn select_spending_category(conn: &Connection, category_id: &String) -> Result<Vec<Source>> {
    let mut stmt = conn.prepare("SELECT spending_category_id, spending_category, created_date, created_by, is_active FROM spending_category where spending_category_id = ?1 and is_active = 1")?;
    let category_iter = stmt.query_map([category_id], |row| {
        let result_category = Source {
            source_id: row
            .get::<_, Option<String>>(0)?
                .and_then(|s| if s.is_empty() { None } else { s.parse::<Uuid>().ok() })
                .unwrap_or_else(Uuid::nil),
            source: row.get(1)?,
            created_date: row
            .get::<_, Option<String>>(2)?
                .and_then(|s| if s.is_empty() { None } else { s.parse::<DateTime<Utc>>().ok() })
                .unwrap_or_else(DateTime::default),
            created_by: row.get(3)?,
            is_active:row.get(4)?
        };
        Ok(result_category)
    })?;

    Ok(category_iter.filter_map(Result::ok).collect())
}

pub fn select_all_spending_categories(conn: &Connection) -> Result<Vec<Source>> {
    let mut stmt = conn.prepare("SELECT spending_category_id, spending_category, created_date, created_by, is_active FROM spending_category where is_active = 1")?;
    let category_iter = stmt.query_map([], |row| {
        Ok(Source {
            source_id: row
            .get::<_, Option<String>>(0)?
                .and_then(|s| if s.is_empty() { None } else { s.parse::<Uuid>().ok() })
                .unwrap_or_else(Uuid::nil),
            source: row.get(1)?,
            created_date: row
            .get::<_, Option<String>>(2)?
                .and_then(|s| if s.is_empty() { None } else { s.parse::<DateTime<Utc>>().ok() })
                .unwrap_or_else(DateTime::default),
            created_by: row.get(3)?,
            is_active:row.get(4)?
        })
    })?;

    Ok(category_iter.filter_map(Result::ok).collect())
}

pub fn insert_spending_category(conn: &Connection, category: &SpendingCategory) -> Result<()> {
    conn.execute(
        "INSERT INTO spending_category (spending_category_id, spending_category, created_date, created_by, is_active) VALUES (?1, ?2, ?3, ?4, ?5)",
        [
            category.spending_category_id.to_string(),
            category.spending_category.to_string(),
            category.created_date.to_rfc3339(),
            category.created_by.clone(),
            category.is_active.to_string(),
        ],
    )?;
    Ok(())
}

pub fn insert_spending(conn: &Connection, spending: &Spending) -> Result<()> {
    conn.execute(
        "INSERT INTO spending (spending_id, total_amount, description, spending_category_id, spending_category, source_id, source, created_date, created_by, is_active)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        [
            spending.spending_id.to_string(),
            spending.total_amount.to_string(),
            spending.description.to_string(),
            spending.spending_category_id.to_string(),
            spending.spending_category.to_string(),
            spending.source_id.to_string(),
            spending.source.to_string(),
            spending.created_date.to_rfc3339(),
            spending.created_by.to_string(),
            spending.is_active.to_string(),
        ],
    )?;
    Ok(())
}

pub fn delete_spending_category(conn: &Connection, category: &String) -> Result<()> {
    conn.execute(
        "UPDATE spending_category SET is_active = 0 WHERE spending_category = ?1",
        [category],
    )?;
    Ok(())
}

pub fn delete_spending(conn: &Connection, spending_id: &String) -> Result<()> {
    conn.execute(
        "UPDATE spending SET is_active = 0 WHERE spending_id = ?1",
        [spending_id],
    )?;
    Ok(())
}