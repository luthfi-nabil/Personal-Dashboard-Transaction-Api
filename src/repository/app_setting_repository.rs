use crate::models::app_setting::AppSettings;
use mysql::prelude::*;
use mysql::{PooledConn, Result};
use std::error::Error;
use uuid::Uuid;

const DEFAULT_APP_SETTINGS: [(&str, &str); 6] = [
    (
        "TRANSFER_CATEGORY_ID",
        "00000000-0000-4000-8000-000000000001",
    ),
    ("TRANSFER_CATEGORY_NAME", "Transfer"),
    (
        "RECOUNT_CATEGORY_ID",
        "00000000-0000-4000-8000-000000000002",
    ),
    ("RECOUNT_CATEGORY_NAME", "Recount"),
    ("DEBT_CATEGORY_ID", "00000000-0000-4000-8000-000000000003"),
    ("DEBT_CATEGORY_NAME", "Debt"),
];

pub fn init_setting_table(conn: &mut PooledConn) -> Result<()> {
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS app_settings (
            app_setting_id CHAR(36) PRIMARY KEY,
            app_setting_key VARCHAR(255) NOT NULL UNIQUE,
            app_setting_value VARCHAR(255) NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            UNIQUE KEY `app_setting_unique` (`app_setting_key`)
        )",
    )?;

    Ok(())
}

pub fn ensure_default_settings(conn: &mut PooledConn) -> Result<()> {
    for (key, value) in DEFAULT_APP_SETTINGS {
        conn.exec_drop(
            "INSERT INTO app_settings
            (app_setting_id, app_setting_key, app_setting_value, is_active)
            VALUES (?, ?, ?, 1)
            ON DUPLICATE KEY UPDATE
                app_setting_value = IF(app_setting_value = '', VALUES(app_setting_value), app_setting_value),
                is_active = 1",
            (Uuid::new_v4().to_string(), key, value),
        )?;
    }
    Ok(())
}

pub fn select_all_settings(
    conn: &mut PooledConn,
    app_setting: &AppSettings,
) -> Result<Vec<AppSettings>, Box<dyn Error>> {
    let mut query = String::from(
        r#"
        SELECT app_setting_id, app_setting_key, app_setting_value, is_active
        FROM app_settings
        WHERE is_active = 1
    "#,
    );
    let mut params: Vec<mysql::Value> = Vec::new();
    if app_setting.app_setting_id != Uuid::nil() {
        query.push_str(" AND app_setting_id = ?");
        params.push(app_setting.app_setting_id.to_string().into());
    }

    if app_setting.app_setting_key != "" {
        query.push_str(" AND app_setting_key = ?");
        params.push(app_setting.app_setting_key.to_string().into());
    }

    let result: Vec<AppSettings> = conn.exec_map(
        query,
        params,
        |(app_setting_id, app_setting_key, app_setting_value, is_active): (
            String,
            String,
            String,
            i32,
        )| {
            AppSettings {
                app_setting_id: Uuid::parse_str(&app_setting_id).unwrap_or_else(|_| Uuid::nil()),
                app_setting_key: app_setting_key,
                app_setting_value: app_setting_value,
                is_active: is_active,
            }
        },
    )?;

    Ok(result)
}
