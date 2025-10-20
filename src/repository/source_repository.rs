use rusqlite::{Connection, Result, Error as RusqliteError};
use crate::models::source::{Source};
use uuid::Uuid;
use chrono::{Utc, DateTime};

pub fn create_source_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS source (
            source_id TEXT PRIMARY KEY,
            source TEXT NOT NULL,
            created_date TEXT NOT NULL,
            created_by TEXT NOT NULL
        )",
        [],
    )?;
    Ok(())
}

pub fn select_source(conn: &Connection, source_id: &String) -> Result<Vec<Source>> {
    let mut stmt = conn.prepare("SELECT source_id, source, created_date, created_by FROM source where source_id = ?1")?;
    let source_iter = stmt.query_map([source_id], |row| {
        let source_id_str: String = row.get(0)?; 
        let source_id_val = Uuid::parse_str(&source_id_str)
        .map_err(|e| RusqliteError::ToSqlConversionFailure(Box::new(e)))?;
        let created_date_str: String = row.get(2)?; 
        let created_date_val: DateTime<Utc> = created_date_str.parse().unwrap();
        Ok(Source {
            source_id: source_id_val,
            source: row.get(1)?,
            created_date: created_date_val,
            created_by: row.get(3)?,
        })
    })?;

    let mut sources = Vec::new();
    for source in source_iter {
        sources.push(source?);
    }
    Ok(sources)
}

pub fn select_all_sources(conn: &Connection) -> Result<Vec<Source>> {
    let mut stmt = conn.prepare("SELECT source_id, source, created_date, created_by FROM source")?;
    let source_iter = stmt.query_map([], |row| {
        let source_id_str: String = row.get(0)?; 
        let source_id_val = Uuid::parse_str(&source_id_str)
        .map_err(|e| RusqliteError::ToSqlConversionFailure(Box::new(e)))?;
        Ok(Source {
            source_id: source_id_val,
            source: row.get(1)?,
            created_date: row.get(2)?,
            created_by: row.get(3)?,
        })
    })?;

    let mut sources = Vec::new();
    for source in source_iter {
        sources.push(source?);
    }
    Ok(sources)
}

pub fn insert_source(conn: &Connection, source: &Source) -> Result<()> {
    conn.execute(
        "INSERT INTO source (source_id, source, created_date, created_by) VALUES (?1, ?2, ?3, ?4)",
        [
            source.source_id.to_string(),
            source.source.to_string(),
            source.created_date.to_rfc3339(),
            source.created_by.clone(),
        ],
    )?;
    Ok(())
}