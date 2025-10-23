use rusqlite::{Connection, Result, Error as RusqliteError};
use chrono::{Utc, DateTime};
use crate::models::earning::{self, Earning, EarningCategory};
use uuid::Uuid;

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
            created_by TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1
        )",
        [],
    )?;
    Ok(())
}

pub fn create_earning_category_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS earning_category (
            earning_category_id TEXT PRIMARY KEY,
            earning_category TEXT NOT NULL UNIQUE,
            created_date TEXT NOT NULL,
            created_by TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1
        )",
        [],
    )?;
    Ok(())
}

pub fn select_earnings(conn: &Connection) -> Result<Vec<Earning>> {
    let mut stmt = conn.prepare("SELECT earning_id, total_amount, description, earning_category_id, earning_category, source_id, source, created_date, created_by FROM earning where is_active = 1")?;
    let earning_iter = stmt.query_map([], |row| {
        Ok(Earning {
            earning_id: row
            .get::<_, Option<String>>(0)?
                .and_then(|s| if s.is_empty() { None } else { s.parse::<Uuid>().ok() })
                .unwrap_or_else(Uuid::nil),
            total_amount: row.get(1)?,
            description: row.get(2)?,
            earning_category_id: row
            .get::<_, Option<String>>(3)?
                .and_then(|s| if s.is_empty() { None } else { s.parse::<Uuid>().ok() })
                .unwrap_or_else(Uuid::nil),
            earning_category: row.get(4)?,
            source_id: row
            .get::<_, Option<String>>(5)?
                .and_then(|s| if s.is_empty() { None } else { s.parse::<Uuid>().ok() })
                .unwrap_or_else(Uuid::nil),
            source: row.get(6)?,
            created_date: row.get::<_, Option<String>>(2)?
                .and_then(|s| if s.is_empty() { None } else { s.parse::<DateTime<Utc>>().ok() })
                .unwrap_or_else(DateTime::default),
            created_by: row.get(8)?,
            is_active: row.get(9)?
        })
    })?;

    let mut earnings = Vec::new();
    for earning in earning_iter {
        earnings.push(earning?);
    }
    Ok(earnings)
}

pub fn select_earning_category(conn: &Connection, earning_category_id: &String) -> Result<Vec<EarningCategory>> {
    let mut stmt = conn.prepare("SELECT earning_category_id, earning_category, created_date, created_by, is_active FROM earning_category where earning_category_id = ?1 and is_active = 1")?;
    let category_iter = stmt.query_map([earning_category_id], |row| {
        let result_category = EarningCategory {
            earning_category_id: row
                .get::<_, Option<String>>(0)?
                .and_then(|s| if s.is_empty() { None } else { s.parse::<Uuid>().ok() })
                .unwrap_or_else(Uuid::nil),
            earning_category: row.get(1)?,
            created_date: row.get::<_, Option<String>>(2)?
                .and_then(|s| if s.is_empty() { None } else { s.parse::<DateTime<Utc>>().ok() })
                .unwrap_or_else(DateTime::default),
            created_by: row.get(3)?,
            is_active: row.get(4)?
        };
        return Ok(result_category);
    })?;
    
    Ok(category_iter.filter_map(Result::ok).collect())
}

pub fn select_all_earning_categories(conn: &Connection) -> Result<Vec<EarningCategory>> {
    let mut stmt = conn.prepare("SELECT earning_category_id, earning_category, created_date, created_by, is_active FROM earning_category where is_active = 1")?;
    let category_iter = stmt.query_map([], |row| {
        
        Ok(EarningCategory {
            earning_category_id: row
            .get::<_, Option<String>>(0)?
                .and_then(|s| if s.is_empty() { None } else { s.parse::<Uuid>().ok() })
                .unwrap_or_else(Uuid::nil),
            earning_category: row.get(1)?,
            created_date: row.get(2)?,
            created_by: row.get(3)?,
            is_active: row.get(4)?
        })
    })?;

    let mut categories = Vec::new();
    for category in category_iter {
        categories.push(category?);
    }
    Ok(categories)
}

pub fn insert_earning(conn: &Connection, earning: &Earning) -> Result<()> {
    conn.execute(
        "INSERT INTO earning (earning_id, total_amount, description, earning_category_id, earning_category, source_id, source, created_date, created_by, is_active)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
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
            earning.is_active.to_string(),
        ],
    )?;
    Ok(())
}

pub fn insert_earning_category(conn: &Connection, category: &EarningCategory) -> Result<()> {
    conn.execute(
        "INSERT INTO earning_category (earning_category_id, earning_category, created_date, created_by)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        [
            category.earning_category_id.to_string(),
            category.earning_category.to_string(),
            category.created_date.to_rfc3339(),
            category.created_by.to_string(),
            category.is_active.to_string(),
        ],
    )?;
    Ok(())
}

pub fn delete_earning(conn: &Connection, earning_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM earning WHERE earning_id = ?1",
        [earning_id],
    )?;
    Ok(())
}

pub fn delete_earning_category(conn: &Connection, category_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM earning_category WHERE earning_category_id = ?1",
        [category_id],
    )?;
    Ok(())
}