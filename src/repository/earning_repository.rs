use rusqlite::{Connection, Result, Error as RusqliteError};
use crate::models::earning::{Earning, EarningCategory};
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

pub fn select_earnings(conn: &Connection) -> Result<Vec<Earning>> {
    let mut stmt = conn.prepare("SELECT earning_id, total_amount, description, earning_category_id, earning_category, source_id, source, created_date, created_by FROM earning")?;
    let earning_iter = stmt.query_map([], |row| {
        let earning_id_str: String = row.get(0)?; 
        let earning_id_val = Uuid::parse_str(&earning_id_str)
        .map_err(|e| RusqliteError::ToSqlConversionFailure(Box::new(e)))?;
        Ok(Earning {
            earning_id: earning_id_val,
            total_amount: row.get(1)?,
            description: row.get(2)?,
            earning_category_id: row.get(3)?,
            earning_category: row.get(4)?,
            source_id: row.get(5)?,
            source: row.get(6)?,
            created_date: row.get(7)?,
            created_by: row.get(8)?,
        })
    })?;

    let mut earnings = Vec::new();
    for earning in earning_iter {
        earnings.push(earning?);
    }
    Ok(earnings)
}

pub fn select_earning_categories(conn: &Connection) -> Result<Vec<EarningCategory>> {
    let mut stmt = conn.prepare("SELECT earning_category_id, earning_category, created_date, created_by FROM earning_category")?;
    let category_iter = stmt.query_map([], |row| {
        let earning_category_id_str: String = row.get(0)?; 
        let earning_category_id_val = Uuid::parse_str(&earning_category_id_str)
        .map_err(|e| RusqliteError::ToSqlConversionFailure(Box::new(e)))?;
        Ok(EarningCategory {
            earning_category_id: earning_category_id_val,
            earning_category: row.get(1)?,
            created_date: row.get(2)?,
            created_by: row.get(3)?,
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