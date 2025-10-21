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
        };
        Ok(result_source)
    })?;

    Ok(source_iter.filter_map(Result::ok).collect())
}

pub fn select_all_sources(conn: &Connection) -> Result<Vec<Source>> {
    let mut stmt = conn.prepare("SELECT source_id, source, created_date, created_by FROM source")?;
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
        })
    })?;

    Ok(source_iter.filter_map(Result::ok).collect())
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