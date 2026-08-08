use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Read-only view of a row in login-api's `app_settings` table.
///
/// transaction-api no longer owns this data — it is fetched over HTTP by
/// `helper::settings_client` and only used to resolve the transfer / recount /
/// debt category wiring. Writes go to `POST /api/user/settings` on login-api.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub app_setting_id: Uuid,
    pub app_setting_key: String,
    pub app_setting_value: String,
    /// Username that owns this row. Empty means a global/shared default that
    /// every account sees (e.g. `TRANSFER_CATEGORY_NAME`).
    #[serde(default)]
    pub created_by: String,
    pub is_active: i32,
}
