use mysql::{Result, PooledConn, params, Error as MysqlError};
use mysql::prelude::*;
use crate::models::source::{self, SourceV2, SourceBalance};
use crate::models::responses::{DatabaseResult};
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
            is_active INTEGER NOT NULL DEFAULT 1,
            UNIQUE KEY `source_unique` (`source`,`created_by`)
        )"
    )?;
    Ok(())
}



pub fn select_all_sources(conn: &mut PooledConn, source: &SourceV2) -> Result<Vec<SourceV2>> {
    let mut query = String::from("SELECT source_id, source, created_date, created_by, is_active FROM source WHERE is_active = 1");
    let mut params: Vec<mysql::Value> = Vec::new();
    if source.source_id != Uuid::nil() {
        query.push_str(" AND source_id = ?");
        params.push(source.source_id.to_string().into());
    }
    if !source.created_by.is_empty() {
        query.push_str(" AND created_by = ?");
        params.push(source.created_by.to_string().into());
    }
    let result = conn.exec_map(
        query,
        params,
        |(source_id, source, created_date, created_by, is_active): (String, String, NaiveDateTime, String, i32)| {
            let source_id = Uuid::parse_str(&source_id)
                .unwrap_or_else(|_| Uuid::nil());


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

pub fn select_source(conn: &mut PooledConn, source: &SourceV2) -> Result<Vec<SourceV2>> {
    let mut query = String::from("SELECT source_id, source, created_date, created_by, is_active FROM source WHERE is_active = 1");
    let mut params: Vec<mysql::Value> = Vec::new();
    if source.source_id != Uuid::nil() {
        query.push_str(" AND source_id = ?");
        params.push(source.source_id.to_string().into());
    }

    if !source.created_by.is_empty() {
        query.push_str(" AND created_by = ?");
        params.push(source.created_by.clone().into());
    }
    let result = conn.exec_map(
        query,
        params,
        |(source_id, source, created_date, created_by, is_active): (String, String, NaiveDateTime, String, i32)| {

        SourceV2 { source_id: Uuid::parse_str(&source_id)
                .unwrap_or_else(|_| Uuid::nil()), source: source, created_date: created_date, created_by: created_by, is_active: is_active}
    })?;
    
    Ok(result)
}

pub fn insert_source(conn: &mut PooledConn, source: &SourceV2) -> Result<DatabaseResult, Box<dyn Error>> {
    let result = conn.exec_drop(
        "INSERT INTO source (source_id, source, created_date, created_by, is_active)
         VALUES (?,?,?,?,?)",
        (
            source.source_id.to_string(),
            source.source.to_string(),
            source.created_date.to_string(),
            source.created_by.clone(),
            source.is_active,
        ),
    );

    match result {
        Ok(_) => Ok(DatabaseResult::Inserted),

        Err(MysqlError::MySqlError(ref e)) => match e.code {
            1062u16 => {
                Ok(DatabaseResult::Duplicate)
            }
            _ => Err(Box::new(MysqlError::MySqlError(e.clone()))),
        },

        Err(e) => Err(Box::new(e)),
    }
}

pub fn delete_source(conn: &mut PooledConn, source: &SourceV2) -> Result<()> {
    conn.exec_drop(
        "UPDATE source SET is_active = 0 WHERE source = :source and created_by = :by",
        params!{"source"=>source.source.to_string(), "by"=>source.created_by.to_string()},
    )?;
    Ok(())
}