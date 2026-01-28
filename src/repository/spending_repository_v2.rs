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

pub fn select_spendings(conn: &mut PooledConn, param: &SpendingParam, created_by: Option<String>) -> Result<Vec<SpendingV2>, Box<dyn Error>> {
    let mut query = String::from("SELECT spending_id, total_amount, description, spending_category_id, spending_category, source_id, source, created_date, created_by, is_active FROM spending");
    query.push_str(" where is_active = 1");
    let mut params: Vec<mysql::Value> = Vec::new();
    match &param.description {
        Some(val)=>{
            query.push_str(" and description like ?");
            params.push(("%".to_string() + val + "%").into());
        },
        None => {}
    }

     match &param.spending_category {
        Some(val)=>{
            query.push_str(" and upper(spending_category) = ?");
            params.push(("upper('".to_string() + val + "')").into());
        },
        None => {}
    }

    match &param.spending_id {
        Some(val)=>{
            query.push_str(" and spending_id = ?");
            params.push(val.into());
        },
        None => {}
    }

    match &param.source {
        Some(val)=>{
            query.push_str(" and upper(source) = ?");
            params.push(("upper('".to_string() + val + "')").into());
        },
        None => {}
    }

    match &param.spending_category_id {
        Some(val)=>{
            query.push_str(" and spending_category_id = ?");
            params.push(val.into());
        },
        None => {}
    }

    match &param.source_id {
        Some(val)=>{
            query.push_str(" and source_id = ?");
            params.push(val.into());
        },
        None => {}
    }
    
    match &param.month {
        Some(val)=>{
            query.push_str(" and MONTH(created_date) = ?");
            params.push(val.into());
        },
        None => {}
    }

    match &created_by {
        Some(val)=>{
            query.push_str(" and created_by = ?");
            params.push(val.into());
        },
        None => {}
    }

    let results: Vec<SpendingV2> = conn.exec_map(
    query,
    params,
    |(
        spending_id,
        total_amount,
        description,
        spending_category_id,
        spending_category,
        source_id,
        source,
        created_date,
        created_by,
        is_active
    ): (
        String,          // spending_id (BINARY)
        f64,              // total_amount
        String,           // description (nullable safe)
        String,          // spending_category_id
        String,           // spending_category
        String,           // source_id
        String,           // source
        NaiveDateTime,    // created_date
        String,           // created_by
        i32               // is_active
    )| {
        SpendingV2 {
            spending_id: Uuid::parse_str(&spending_id)
                .unwrap_or_else(|_| Uuid::nil()),
            total_amount,
            description,
            spending_category_id: Uuid::parse_str(&spending_category_id)
                .unwrap_or_else(|_| Uuid::nil()),
            spending_category,
            source_id: Uuid::parse_str(&source_id)
                .unwrap_or_else(|_| Uuid::nil()),
            source,
            created_date,
            created_by,
            is_active,
        }
    })?;
    Ok(results)
}

/// ✅ Select one spending category by ID
pub fn select_spending_category(conn: &mut PooledConn, spending_category: &SpendingCategoryV2) -> Result<Vec<SpendingCategoryV2>, Box<dyn Error>> {
    let mut query = String::from(r#"
        SELECT spending_category_id, spending_category, created_date, created_by, is_active
        FROM spending_category
        WHERE is_active = 1
    "#);
    let mut params: Vec<mysql::Value> = Vec::new();
    if spending_category.spending_category_id != Uuid::nil() {
        query.push_str(" AND spending_category_id = ?");
        params.push(spending_category.spending_category_id.to_string().into());
    }

    if spending_category.created_by != "" {
        query.push_str(" AND created_by = ?");
        params.push(spending_category.created_by.to_string().into());
    }
    let result: Vec<SpendingCategoryV2> = conn.exec_map(
        query,
        params,
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
pub fn select_all_spending_categories(conn: &mut PooledConn, spending_category: &SpendingCategoryV2) -> Result<Vec<SpendingCategoryV2>, Box<dyn Error>> {
    let mut query = String::from(r#"
        SELECT spending_category_id, spending_category, created_date, created_by, is_active
        FROM spending_category WHERE is_active = 1
    "#);

    let mut params: Vec<mysql::Value> = Vec::new();
    if spending_category.spending_category_id != Uuid::nil() {
        query.push_str(" AND spending_category_id = ?");
        params.push(spending_category.spending_category_id.to_string().into());
    }

    if spending_category.created_by != "" {
        query.push_str(" AND created_by = ?");
        params.push(spending_category.created_by.to_string().into());
    }

    let result: Vec<SpendingCategoryV2> = conn.exec_map(
        query,
        params,
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
pub fn delete_spending_category(conn: &mut PooledConn, category: &SpendingCategoryV2) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "UPDATE spending_category SET is_active = 0 WHERE spending_category_id = :cat AND created_by = :created_by",
        params! { "cat" => category.spending_category_id, "created_by" => category.created_by.to_string() },
    )?;
    Ok(())
}
