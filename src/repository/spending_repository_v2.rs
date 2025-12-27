use mysql::*;
use mysql::prelude::*;
use chrono::{NaiveDateTime};
use crate::models::spending::{SpendingV2, SpendingCategoryV2,SpendingParam};
use uuid::Uuid;
use std::error::Error;
pub fn create_spending_category_table(conn: &mut PooledConn) -> Result<()> {
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS spending_category (
            spending_category_id CHAR(36) PRIMARY KEY,
            spending_category VARCHAR(255) NOT NULL UNIQUE,
            created_date DATETIME NOT NULL,
            created_by VARCHAR(255) NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1
        )"
    )?;
    Ok(())
}

pub fn create_spending_table(conn: &mut PooledConn) -> Result<()> {
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS spending (
            spending_id CHAR(36) PRIMARY KEY,
            total_amount double NOT NULL,
            description TEXT,
            spending_category_id CHAR(36) NOT NULL,
            spending_category VARCHAR(255) NOT NULL,
            source_id CHAR(255) NOT NULL,
            source VARCHAR(255) NOT NULL,
            created_date DATETIME NOT NULL,
            created_by TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1
        )",
    )?;
    Ok(())
}

pub fn select_spendings(conn: &mut PooledConn, param: &SpendingParam) -> Result<Vec<SpendingV2>, Box<dyn Error>> {
    let mut query = String::from("SELECT spending_id, total_amount, description, spending_category_id, spending_category, source_id, source, created_date, created_by, is_active FROM spending");
    query.push_str(" where is_active = 1");
    match &param.description {
        Some(val)=>query.push_str(&format!(" and description like '%{}%'", val)),
        None => {}
    }

    match &param.spending_category {
        Some(val)=>query.push_str(&format!(" and upper(spending_category) = upper('{}')", val)),
        None => {}
    }

    match &param.source {
        Some(val)=>query.push_str(&format!(" and upper(source) = upper('{}')", val)),
        None => {}
    }

    match &param.spending_category_id {
        Some(val)=>query.push_str(&format!(" and spending_category_id = '{}'", val)),
        None => {}
    }

    match &param.source_id {
        Some(val)=>query.push_str(&format!(" and source_id = '{}'", val)),
        None => {}
    }
    
    match &param.month {
        Some(val)=>query.push_str(&format!(" and MONTH(created_date) = {}", val)),
        None => {}
    }

    let results: Vec<SpendingV2> = conn.query_map(query, |(spending_id, total_amount, description, spending_category_id, spending_category, source_id, source, created_date, created_by, is_active): (String, String, String, String, String,String, String, String, String, String)|{
        SpendingV2 {
            spending_id: Uuid::parse_str(&spending_id)
                .unwrap_or_else(|_| Uuid::nil()),
            total_amount: total_amount.parse::<f64>().unwrap_or(0.0),
            description: description,
            spending_category_id: Uuid::parse_str(&spending_category_id)
                .unwrap_or_else(|_| Uuid::nil()),
            spending_category,
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

/// ✅ Select one spending category by ID
pub fn select_spending_category(conn: &mut PooledConn, spending_category_id: &str) -> Result<Vec<SpendingCategoryV2>, Box<dyn Error>> {
    let query = r#"
        SELECT spending_category_id, spending_category, created_date, created_by, is_active
        FROM spending_category
        WHERE spending_category_id = :id AND is_active = 1
    "#;

    let result: Vec<SpendingCategoryV2> = conn.exec_map(
        query,
        params! { "id" => spending_category_id },
        |(spending_category_id, spending_category, created_date, created_by, is_active): (String, String, NaiveDateTime, String, i32)| {
            SpendingCategoryV2 {
                spending_category_id: Uuid::parse_str(&spending_category_id)
                .unwrap_or_else(|_| Uuid::nil()),
                spending_category: spending_category,
                created_date: created_date,
                created_by: created_by,
                is_active: is_active,
            }
        },
    )?;

    Ok(result)
}

/// ✅ Select one spending category by ID
pub fn select_all_spending_categories(conn: &mut PooledConn) -> Result<Vec<SpendingCategoryV2>, Box<dyn Error>> {
    let query = r#"
        SELECT spending_category_id, spending_category, created_date, created_by, is_active
        FROM spending_category
    "#;

    let result: Vec<SpendingCategoryV2> = conn.query_map(
        query,
        |(spending_category_id, spending_category, created_date, created_by, is_active): (String, String, String, String, String)| {
            SpendingCategoryV2 {
                spending_category_id: Uuid::parse_str(&spending_category_id)
                .unwrap_or_else(|_| Uuid::nil()),
                spending_category: spending_category,
                created_date: NaiveDateTime::parse_from_str(&created_date, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_else(|_| NaiveDateTime::from_timestamp_opt(0, 0).unwrap()),
                created_by: created_by,
                is_active: is_active.parse::<i32>().unwrap_or(0),
            }
        },
    )?;

    Ok(result)
}


/// ✅ Insert a new spending
pub fn insert_spending(conn: &mut PooledConn, spending: &SpendingV2) -> Result<(), Box<dyn Error>> {
    let query = r#"
        INSERT INTO spending 
        (spending_id, total_amount, description, spending_category_id, spending_category,
         source_id, source, created_date, created_by, is_active)
        VALUES 
        (:id, :total, :desc, :cat_id, :cat, :src_id, :src, :created, :by, :active)
    "#;

    conn.exec_drop(
        query,
        params! {
            "id" => spending.spending_id.to_string(),
            "total" => spending.total_amount,
            "desc" => &spending.description,
            "cat_id" => spending.spending_category_id.to_string(),
            "cat" => &spending.spending_category,
            "src_id" => spending.source_id.to_string(),
            "src" => &spending.source,
            "created" => spending.created_date.to_string(),
            "by" => &spending.created_by,
            "active" => spending.is_active,
        },
    )?;

    Ok(())
}

/// ✅ Insert a new spending category
pub fn insert_spending_category(conn: &mut PooledConn, category: &SpendingCategoryV2) -> Result<(), Box<dyn Error>> {
    let query = r#"
        INSERT INTO spending_category 
        (spending_category_id, spending_category, created_date, created_by, is_active)
        VALUES 
        (:id, :cat, :created, :by, :active)
    "#;

    conn.exec_drop(
        query,
        params! {
            "id" => category.spending_category_id.to_string(),
            "cat" => &category.spending_category,
            "created" => category.created_date.to_string(),
            "by" => &category.created_by,
            "active" => category.is_active,
        },
    )?;

    Ok(())
}

/// ✅ Delete an spending permanently
pub fn delete_spending(conn: &mut PooledConn, spending_id: &str) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "DELETE FROM spending WHERE spending_id = :id",
        params! { "id" => spending_id },
    )?;
    Ok(())
}



/// ✅ Soft delete (deactivate) an spending category
pub fn delete_spending_category(conn: &mut PooledConn, category: &str) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "UPDATE spending_category SET is_active = 0 WHERE spending_category = :cat",
        params! { "cat" => category },
    )?;
    Ok(())
}
