use mysql::*;
use mysql::prelude::*;
use chrono::{Utc, DateTime, NaiveDateTime};
use crate::models::earning::{self,Earning, EarningV2, EarningCategory, EarningParam};
use uuid::Uuid;
use std::error::Error;
pub fn create_earning_category_table(conn: &mut PooledConn) -> Result<()> {
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS earning_category (
            earning_category_id CHAR(36) PRIMARY KEY,
            earning_category VARCHAR(255) NOT NULL UNIQUE,
            created_date DATETIME NOT NULL,
            created_by VARCHAR(255) NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1
        )"
    )?;
    Ok(())
}

pub fn create_earning_table(conn: &mut PooledConn) -> Result<()> {
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS earning (
            earning_id CHAR(36) PRIMARY KEY,
            total_amount double NOT NULL,
            description TEXT,
            earning_category_id CHAR(36) NOT NULL,
            earning_category VARCHAR(255) NOT NULL,
            source_id CHAR(255) NOT NULL,
            source VARCHAR(255) NOT NULL,
            created_date DATETIME NOT NULL,
            created_by TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1
        )",
    )?;
    Ok(())
}

pub fn select_earnings(conn: &mut PooledConn, param: &EarningParam) -> Result<Vec<EarningV2>, Box<dyn Error>> {
    let mut query = String::from("SELECT earning_id, total_amount, description, earning_category_id, earning_category, source_id, source, created_date, created_by, is_active FROM earning");
    query.push_str(" where is_active = 1");
    match &param.description {
        Some(val)=>query.push_str(" and description like '%:description%'"),
        None => {}
    }

    match &param.earning_category {
        Some(val)=>query.push_str(" and upper(earning_category) = upper(:earning_category)"),
        None => {}
    }

    match &param.source {
        Some(val)=>query.push_str(" and upper(source) = upper(:source)"),
        None => {}
    }

    match &param.earning_category_id {
        Some(val)=>query.push_str(" and earning_category_id = :earning_category_id"),
        None => {}
    }

    match &param.source_id {
        Some(val)=>query.push_str(" and source_id = :source_id"),
        None => {}
    }
    
    match &param.month {
        Some(val)=>query.push_str(" and MONTH(created_date) = :month"),
        None => {}
    }
    
    let results: Vec<EarningV2> = conn.exec_map(query, params!{
        "description"=>param.description.clone(),
        "earning_category"=>param.earning_category.clone(),
        "earning_category_id"=>param.earning_category_id,
        "source"=>param.source.clone(),
        "source_id"=>param.source_id,
        "month"=>param.month,
    },|(earning_id, total_amount, description, earning_category_id, earning_category, source_id, source, created_date, created_by, is_active): (String, String, String, String, String,String, String, String, String, String)|{
        EarningV2 {
            earning_id: Uuid::parse_str(&earning_id)
                .unwrap_or_else(|_| Uuid::nil()),
            total_amount: total_amount.parse::<f64>().unwrap_or(0.0),
            description: description,
            earning_category_id: Uuid::parse_str(&earning_category_id)
                .unwrap_or_else(|_| Uuid::nil()),
            earning_category,
            source_id: Uuid::parse_str(&source_id)
                .unwrap_or_else(|_| Uuid::nil()),
            source,
            created_date: NaiveDateTime::parse_from_str(&created_date, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_else(|_| NaiveDateTime::from_timestamp_opt(0, 0).unwrap()),
            created_by: created_by,
            is_active: is_active.parse::<i32>().unwrap_or(0),
        }
    })?;
    Ok(results)
}

/// ✅ Insert a new earning
pub fn insert_earning(conn: &mut PooledConn, earning: &Earning) -> Result<(), Box<dyn Error>> {
    let query = r#"
        INSERT INTO earning 
        (earning_id, total_amount, description, earning_category_id, earning_category,
         source_id, source, created_date, created_by, is_active)
        VALUES 
        (:id, :total, :desc, :cat_id, :cat, :src_id, :src, :created, :by, :active)
    "#;

    conn.exec_drop(
        query,
        params! {
            "id" => earning.earning_id.to_string(),
            "total" => earning.total_amount,
            "desc" => &earning.description,
            "cat_id" => earning.earning_category_id.to_string(),
            "cat" => &earning.earning_category,
            "src_id" => earning.source_id.to_string(),
            "src" => &earning.source,
            "created" => earning.created_date.to_rfc3339(),
            "by" => &earning.created_by,
            "active" => earning.is_active,
        },
    )?;

    Ok(())
}

/// ✅ Insert a new earning category
pub fn insert_earning_category(conn: &mut PooledConn, category: &EarningCategory) -> Result<(), Box<dyn Error>> {
    let query = r#"
        INSERT INTO earning_category 
        (earning_category_id, earning_category, created_date, created_by, is_active)
        VALUES 
        (:id, :cat, :created, :by, :active)
    "#;

    conn.exec_drop(
        query,
        params! {
            "id" => category.earning_category_id.to_string(),
            "cat" => &category.earning_category,
            "created" => category.created_date.to_rfc3339(),
            "by" => &category.created_by,
            "active" => category.is_active,
        },
    )?;

    Ok(())
}

/// ✅ Delete an earning permanently
pub fn delete_earning(conn: &mut PooledConn, earning_id: &str) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "DELETE FROM earning WHERE earning_id = :id",
        params! { "id" => earning_id },
    )?;
    Ok(())
}



/// ✅ Soft delete (deactivate) an earning category
pub fn delete_earning_category(conn: &mut PooledConn, category: &str) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "UPDATE earning_category SET is_active = 0 WHERE earning_category = :cat",
        params! { "cat" => category },
    )?;
    Ok(())
}
