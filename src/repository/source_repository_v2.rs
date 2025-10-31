use mysql::*;
use mysql::prelude::*;
use crate::models::source::{self, SourceV2};
use chrono::{Utc, DateTime, NaiveDateTime};
use uuid::Uuid;
use std::error::Error;
pub fn create_source_table(conn: &mut PooledConn) -> Result<()> {
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS source (
            source_id CHAR(36) PRIMARY KEY,
            SOURCE VARCHAR(255) NOT NULL UNIQUE,
            created_date DATETIME NOT NULL,
            created_by VARCHAR(255) NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1
        )"
    )?;
    Ok(())
}



pub fn select_all_sources(conn: &mut PooledConn) -> Result<Vec<SourceV2>> {
    let result = conn.query_map(
        "SELECT source_id, source, created_date, created_by, is_active FROM source",
        |(source_id, source, created_date, created_by, is_active): (String, String, String, String, String)| {
            let source_id = Uuid::parse_str(&source_id)
                .unwrap_or_else(|_| Uuid::nil());

            let created_date = NaiveDateTime::parse_from_str(&created_date, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_else(|_| NaiveDateTime::from_timestamp_opt(0, 0).unwrap());

            let is_active = is_active.parse::<i32>().unwrap_or(0);

            SourceV2 {
                source_id,
                source,
                created_date,
                created_by,
                is_active,
            }
        },
    )?;
    Ok(result)
}

pub fn select_source(conn: &mut PooledConn, source_id: &String) -> Result<Vec<SourceV2>> {
    let result = conn.exec_map("SELECT source_id, source, created_date, created_by, is_active FROM source where source_id = :source_id and is_active = 1",
    params!{
        "source_id"=>source_id
    },
    |(source_id, source, created_date, created_by, is_active)|{
        SourceV2 { source_id, source, created_date, created_by, is_active}
    })?;
    
    Ok(result)
}

pub fn insert_source(conn: &mut PooledConn, source: &SourceV2) -> Result<()> {
    conn.exec_drop(
        "INSERT INTO source (source_id, source, created_date, created_by, is_active) VALUES (?,?,?,?,?)",
            (source.source_id.to_string(),
            source.source.to_string(),
            source.created_date.to_string(),
            source.created_by.clone(),
            source.is_active.to_string())
        
    )?;
    Ok(())
}

pub fn delete_source(conn: &mut PooledConn, source: &String) -> Result<()> {
    conn.exec_drop(
        "UPDATE source SET is_active = 0 WHERE source = :source",
        params!{"source"=>source},
    )?;
    Ok(())
}