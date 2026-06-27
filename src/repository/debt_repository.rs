use crate::models::debt::{Debt, DebtParam};
use crate::models::earning::{EarningCategoryV2, EarningParam, EarningV2};
use crate::models::responses::DatabaseResult;
use chrono::NaiveDateTime;
use mysql::prelude::*;
use mysql::{Error as MysqlError, PooledConn, Result, params};
use std::error::Error;
use uuid::Uuid;

pub fn create_debt_table(conn: &mut PooledConn) -> Result<()> {
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS debt (
            debt_id CHAR(36) PRIMARY KEY,
            description TEXT NOT NULL,
            debt_type int(11) NOT NULL,
            amount double NOT NULL,
            debt_earning_id CHAR(36) NOT NULL,
            debt_spending_id CHAR(36) NOT NULL,
            status int(11) NOT NULL,
            created_date DATETIME NOT NULL,
            created_by TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1
        )",
    )?;
    Ok(())
}

pub fn select_debt(
    conn: &mut PooledConn,
    param: &DebtParam,
    created_by: Option<String>,
) -> Result<Vec<Debt>, Box<dyn Error>> {
    let mut query = String::from(
        "SELECT description, debt_type, amount, debt_earning_id, debt_spending_id, status, created_date, created_by, is_active FROM debt",
    );
    query.push_str(" where is_active = 1");
    let mut params: Vec<mysql::Value> = Vec::new();

    match &param.debt_earning_id {
        Some(val) => {
            query.push_str(" and upper(debt_earning_id) = ?");
            params.push(("upper('".to_string() + val + "')").into());
        }
        None => {}
    }

    match &param.debt_spending_id {
        Some(val) => {
            query.push_str(" and upper(debt_spending_id) = ?");
            params.push(("upper('".to_string() + val + "')").into());
        }
        None => {}
    }

    match &param.debt_type {
        Some(val) => {
            query.push_str(" and debt_type = ?");
            params.push(val.into());
        }
        None => {}
    }

    match &param.status {
        Some(val) => {
            query.push_str(" and status = ?");
            params.push(val.into());
        }
        None => {}
    }

    match &param.debt_id {
        Some(val) => {
            query.push_str(" and debt_id = ?");
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

    let results: Vec<Debt> = conn.exec_map(
        query,
        params,
        |(
            debt_id,
            amount,
            description,
            debt_type,
            debt_earning_id,
            debt_spending_id,
            status,
            created_date,
            created_by,
            is_active,
        ): (
            String,        // debt_id (BINARY UUID)
            f64,           // amount (DOUBLE)
            String,        // description
            i32,           // debt_type (INT)
            String,        // debt_earning_id (BINARY UUID)
            String,        // debt_spending_id (BINARY UUID)
            i32,           // status (INT)
            NaiveDateTime, // created_date (DATETIME)
            String,        // created_by
            i32,           // is_active (INT)
        )| {
            Debt {
                debt_id: Uuid::parse_str(&debt_id).unwrap_or_else(|_| Uuid::nil()),
                amount,
                description,
                debt_type,
                debt_earning_id: Uuid::parse_str(&debt_earning_id).ok(),
                debt_spending_id: Uuid::parse_str(&debt_spending_id).ok(),
                status,
                created_date,
                created_by,
                is_active,
            }
        },
    )?;
    Ok(results)
}

/// ✅ Insert a new debt
pub fn insert_debt(conn: &mut PooledConn, debt: &Debt) -> Result<(), Box<dyn Error>> {
    let query = r#"
        INSERT INTO debt 
        (debt_id, amount, description, debt_type, debt_earning_id, debt_spending_id,
         status, created_date, created_by, is_active)
        VALUES 
        (:id, :total, :desc, :debt_type, :debt_earning_id, :debt_spending_id, :status, :created, :by, :active)
    "#;

    conn.exec_drop(
        query,
        params! {
            "id" => debt.debt_id.to_string(),
            "total" => debt.amount,
            "desc" => &debt.description,
            "debt_type" => debt.debt_type,
            "debt_earning_id" => debt.debt_earning_id.map(|id| id.to_string()).unwrap_or_else(|| "".into()),
            "debt_spending_id" => debt.debt_spending_id.map(|id| id.to_string()).unwrap_or_else(|| "".into()),
            "status" => &debt.status,
            "created" => debt.created_date.to_string(),
            "by" => &debt.created_by,
            "active" => debt.is_active,
        },
    )?;

    Ok(())
}

/// ✅ Delete an earning permanently
pub fn update_debt(conn: &mut PooledConn, debt: &Debt) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "UPDATE debt set status = :status, debt_earning_id = :debt_earning_id, debt_spending_id = :debt_spending_id WHERE debt_id = :id and created_by = :by",
        params! { "status" => debt.status, "debt_earning_id" => &debt.debt_earning_id, "debt_spending_id" => &debt.debt_spending_id, "id" => debt.debt_id.to_string(), "by" => debt.created_by.to_string() },
    )?;
    Ok(())
}

/// ✅ Soft delete (deactivate) an earning category
pub fn delete_earning_category(
    conn: &mut PooledConn,
    category: &EarningCategoryV2,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "UPDATE earning_category SET is_active = 0 WHERE earning_category_id = :cat_id and created_by = :by",
        params! { "cat_id" => category.earning_category_id.to_string(), "by" => category.created_by.to_string() },
    )?;
    Ok(())
}
