use mysql::{Result, PooledConn};
use mysql::prelude::*;
use uuid::Uuid;
use std::error::Error;
use crate::models::app_setting::AppSettings;
pub fn init_setting_table(conn: &mut PooledConn) -> Result<()> {
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS app_settings (
            app_setting_id CHAR(36) PRIMARY KEY,
            app_setting_key VARCHAR(255) NOT NULL UNIQUE,
            app_setting_value VARCHAR(255) NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            UNIQUE KEY `app_setting_unique` (`app_setting_key`)
        )"
    )?;

    Ok(())
}

pub fn select_all_settings(conn: &mut PooledConn, app_setting: &AppSettings) -> Result<Vec<AppSettings>, Box<dyn Error>> {
    let mut query = String::from(r#"
        SELECT app_setting_id, app_setting_key, app_setting_value, is_active
        FROM app_settings
        WHERE is_active = 1
    "#);
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
        |(app_setting_id, app_setting_key, app_setting_value, is_active): (String, String, String, i32)| {
            AppSettings {
                app_setting_id: Uuid::parse_str(&app_setting_id)
                .unwrap_or_else(|_| Uuid::nil()),
                app_setting_key: app_setting_key,
                app_setting_value: app_setting_value,
                is_active: is_active,
            }
        },
    )?;

    Ok(result)
}