use rusqlite::{Connection, Result};
use crate::models::source::{Source};
use uuid::Uuid;
use chrono::{Utc, DateTime};

pub fn create_source_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS source (
            source_id TEXT PRIMARY KEY,
            source TEXT NOT NULL UNIQUE,
            created_date TEXT NOT NULL,
            created_by TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1
        )",
        [],
    )?;
    Ok(())
}

pub fn select_source(conn: &Connection, source_id: &String) -> Result<Vec<Source>> {
    let mut stmt = conn.prepare("SELECT source_id, source, created_date, created_by FROM source where source_id = ?1 and is_active = 1")?;
    let source_iter = stmt.query_map([source_id], |row| {
        let result_source = Source {
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
        Ok(result_source)
    })?;

    Ok(source_iter.filter_map(Result::ok).collect())
}

pub fn select_all_sources(conn: &Connection) -> Result<Vec<Source>> {
    let mut stmt = conn.prepare("SELECT source_id, source, created_date, created_by FROM source where is_active = 1")?;
    let source_iter = stmt.query_map([], |row| {
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

    Ok(source_iter.filter_map(Result::ok).collect())
}

pub fn insert_source(conn: &Connection, source: &Source) -> Result<()> {
    conn.execute(
        "INSERT INTO source (source_id, source, created_date, created_by, is_active) VALUES (?1, ?2, ?3, ?4, ?5)",
        [
            source.source_id.to_string(),
            source.source.to_string(),
            source.created_date.to_rfc3339(),
            source.created_by.clone(),
            source.is_active.to_string()
        ],
    )?;
    Ok(())
}

pub fn delete_source(conn: &Connection, source: &String) -> Result<()> {
    conn.execute(
        "UPDATE source SET is_active = 0 WHERE source = ?1",
        [source],
    )?;
    Ok(())
}