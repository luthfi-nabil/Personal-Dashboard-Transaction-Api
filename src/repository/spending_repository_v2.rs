use crate::models::spending::{
    SpendingCategoryV2, SpendingDetailV2, SpendingParam, SpendingV2,
};
use crate::repository::add_column_if_missing;
use chrono::NaiveDateTime;
use mysql::prelude::*;
use mysql::*;
use std::error::Error;
use uuid::Uuid;
pub fn create_spending_category_table(conn: &mut PooledConn) -> Result<()> {
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS spending_category (
            spending_category_id CHAR(36) PRIMARY KEY,
            spending_category VARCHAR(255) NOT NULL UNIQUE,
            created_date DATETIME NOT NULL,
            created_by VARCHAR(255) NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1
        )",
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

pub fn create_spending_detail_table(conn: &mut PooledConn) -> Result<()> {
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS spending_detail (
            spending_detail_id CHAR(36) PRIMARY KEY,
            spending_id CHAR(36) NOT NULL,
            item_name VARCHAR(255) NOT NULL,
            quantity double NOT NULL DEFAULT 1,
            unit_price double NOT NULL DEFAULT 0,
            amount double NOT NULL DEFAULT 0,
            note TEXT,
            is_checked TINYINT(1) NOT NULL DEFAULT 1,
            created_date DATETIME NOT NULL,
            created_by VARCHAR(255) NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            INDEX idx_spending_detail_spending (spending_id),
            INDEX idx_spending_detail_created_by (created_by)
        )",
    )?;
    add_column_if_missing(
        conn,
        "spending_detail",
        "is_checked",
        "ALTER TABLE spending_detail ADD COLUMN is_checked TINYINT(1) NOT NULL DEFAULT 1 AFTER note",
    )?;
    Ok(())
}

pub fn select_spendings(
    conn: &mut PooledConn,
    param: &SpendingParam,
    created_by: Option<String>,
) -> Result<Vec<SpendingV2>, Box<dyn Error>> {
    let mut query = String::from(
        "SELECT spending_id, total_amount, description, spending_category_id, spending_category, source_id, source, created_date, created_by, is_active FROM spending",
    );
    query.push_str(" where is_active = 1");
    let mut params: Vec<mysql::Value> = Vec::new();
    println!("Select Spending with params: {:?}", param);
    match &param.description {
        Some(val) => {
            query.push_str(" and description like ?");
            params.push(("%".to_string() + val + "%").into());
        }
        None => {}
    }

    match &param.spending_category {
        Some(val) => {
            query.push_str(" and upper(spending_category) = ?");
            params.push(("upper('".to_string() + val + "')").into());
        }
        None => {}
    }

    match &param.spending_id {
        Some(val) => {
            query.push_str(" and spending_id = ?");
            params.push(val.to_string().into());
        }
        None => {}
    }

    match &param.source {
        Some(val) => {
            query.push_str(" and upper(source) = ?");
            params.push(("upper('".to_string() + val + "')").into());
        }
        None => {}
    }

    match &param.spending_category_id {
        Some(val) => {
            query.push_str(" and spending_category_id = ?");
            params.push(val.into());
        }
        None => {}
    }

    match &param.source_id {
        Some(val) => {
            query.push_str(" and source_id = ?");
            params.push(val.into());
        }
        None => {}
    }

    match &param.month {
        Some(val) => {
            query.push_str(" and MONTH(created_date) = ?");
            params.push(val.into());
        }
        None => {}
    }

    match &created_by {
        Some(val) => {
            query.push_str(" and created_by = ?");
            params.push(val.into());
        }
        None => {}
    }
    println!("Final Params: {:?}", params);
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
            is_active,
        ): (
            String,        // spending_id (BINARY)
            f64,           // total_amount
            String,        // description (nullable safe)
            String,        // spending_category_id
            String,        // spending_category
            String,        // source_id
            String,        // source
            NaiveDateTime, // created_date
            String,        // created_by
            i32,           // is_active
        )| {
            SpendingV2 {
                spending_id: Uuid::parse_str(&spending_id).unwrap_or_else(|_| Uuid::nil()),
                total_amount,
                description,
                spending_category_id: Uuid::parse_str(&spending_category_id)
                    .unwrap_or_else(|_| Uuid::nil()),
                spending_category,
                source_id: Uuid::parse_str(&source_id).unwrap_or_else(|_| Uuid::nil()),
                source,
                created_date,
                created_by,
                is_active,
            }
        },
    )?;
    Ok(results)
}

/// ✅ Select one spending category by ID
pub fn select_spending_category(
    conn: &mut PooledConn,
    spending_category: &SpendingCategoryV2,
) -> Result<Vec<SpendingCategoryV2>, Box<dyn Error>> {
    let mut query = String::from(
        r#"
        SELECT spending_category_id, spending_category, created_date, created_by, is_active
        FROM spending_category
        WHERE is_active = 1
    "#,
    );
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
        |(spending_category_id, spending_category, created_date, created_by, is_active): (
            String,
            String,
            NaiveDateTime,
            String,
            i32,
        )| {
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
pub fn select_all_spending_categories(
    conn: &mut PooledConn,
    spending_category: &SpendingCategoryV2,
) -> Result<Vec<SpendingCategoryV2>, Box<dyn Error>> {
    let mut query = String::from(
        r#"
        SELECT spending_category_id, spending_category, created_date, created_by, is_active
        FROM spending_category WHERE is_active = 1
    "#,
    );

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
        |(spending_category_id, spending_category, created_date, created_by, is_active): (
            String,
            String,
            NaiveDateTime,
            String,
            i32,
        )| {
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

/// ✅ Insert the line items belonging to a spending
pub fn insert_spending_details(
    conn: &mut PooledConn,
    details: &[SpendingDetailV2],
) -> Result<(), Box<dyn Error>> {
    if details.is_empty() {
        return Ok(());
    }

    let query = r#"
        INSERT INTO spending_detail
        (spending_detail_id, spending_id, item_name, quantity, unit_price, amount, note,
         is_checked, created_date, created_by, is_active)
        VALUES
        (:id, :spending_id, :item_name, :quantity, :unit_price, :amount, :note,
         :is_checked, :created, :by, :active)
    "#;

    for detail in details {
        conn.exec_drop(
            query,
            params! {
                "id" => detail.spending_detail_id.to_string(),
                "spending_id" => detail.spending_id.to_string(),
                "item_name" => &detail.item_name,
                "quantity" => detail.quantity,
                "unit_price" => detail.unit_price,
                "amount" => detail.amount,
                "note" => &detail.note,
                "is_checked" => i32::from(detail.is_checked),
                "created" => detail.created_date.to_string(),
                "by" => &detail.created_by,
                "active" => detail.is_active,
            },
        )?;
    }

    Ok(())
}

/// ✅ Select spending line items, optionally scoped to one spending
pub fn select_spending_details(
    conn: &mut PooledConn,
    spending_id: Option<Uuid>,
    created_by: Option<String>,
) -> Result<Vec<SpendingDetailV2>, Box<dyn Error>> {
    let mut query = String::from(
        "SELECT spending_detail_id, spending_id, item_name, quantity, unit_price, amount, \
         COALESCE(note, '') AS note, is_checked, created_date, created_by, is_active \
         FROM spending_detail WHERE is_active = 1",
    );
    let mut params: Vec<mysql::Value> = Vec::new();

    match &spending_id {
        Some(val) => {
            query.push_str(" AND spending_id = ?");
            params.push(val.to_string().into());
        }
        None => {}
    }

    match &created_by {
        Some(val) => {
            query.push_str(" AND created_by = ?");
            params.push(val.into());
        }
        None => {}
    }

    query.push_str(" ORDER BY created_date ASC, item_name ASC");

    let results: Vec<SpendingDetailV2> = conn.exec_map(
        query,
        params,
        |(
            spending_detail_id,
            spending_id,
            item_name,
            quantity,
            unit_price,
            amount,
            note,
            is_checked,
            created_date,
            created_by,
            is_active,
        ): (
            String,
            String,
            String,
            f64,
            f64,
            f64,
            String,
            i32,
            NaiveDateTime,
            String,
            i32,
        )| {
            SpendingDetailV2 {
                spending_detail_id: Uuid::parse_str(&spending_detail_id)
                    .unwrap_or_else(|_| Uuid::nil()),
                spending_id: Uuid::parse_str(&spending_id).unwrap_or_else(|_| Uuid::nil()),
                item_name,
                quantity,
                unit_price,
                amount,
                note,
                is_checked: is_checked != 0,
                created_date,
                created_by,
                is_active,
            }
        },
    )?;

    Ok(results)
}

/// ✅ Tick / untick one line item. Returns the number of rows touched, so the
/// caller can tell "not yours / not found" apart from a successful update.
pub fn update_spending_detail_checked(
    conn: &mut PooledConn,
    spending_detail_id: Uuid,
    created_by: &str,
    checked: bool,
) -> Result<u64, Box<dyn Error>> {
    conn.exec_drop(
        "UPDATE spending_detail SET is_checked = :is_checked
         WHERE spending_detail_id = :id AND created_by = :created_by AND is_active = 1",
        params! {
            "id" => spending_detail_id.to_string(),
            "created_by" => created_by,
            "is_checked" => i32::from(checked),
        },
    )?;
    Ok(conn.affected_rows())
}

/// ✅ Delete every line item of a spending (used when the spending is deleted)
pub fn delete_spending_details(
    conn: &mut PooledConn,
    spending_id: Uuid,
    created_by: &str,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "DELETE FROM spending_detail WHERE spending_id = :id AND created_by = :created_by",
        params! {
            "id" => spending_id.to_string(),
            "created_by" => created_by,
        },
    )?;
    Ok(())
}

/// ✅ Insert a new spending category
pub fn insert_spending_category(
    conn: &mut PooledConn,
    category: &SpendingCategoryV2,
) -> Result<(), Box<dyn Error>> {
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
pub fn delete_spending(conn: &mut PooledConn, spending: &SpendingV2) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "DELETE FROM spending WHERE spending_id = :id AND created_by = :created_by",
        params! {
            "id" => spending.spending_id.to_string(),
            "created_by" => spending.created_by.to_string(),
        },
    )?;
    Ok(())
}

/// ✅ Soft delete (deactivate) an spending category
pub fn delete_spending_category(
    conn: &mut PooledConn,
    category: &SpendingCategoryV2,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "UPDATE spending_category SET is_active = 0 WHERE spending_category_id = :cat AND created_by = :created_by",
        params! { "cat" => category.spending_category_id, "created_by" => category.created_by.to_string() },
    )?;
    Ok(())
}
