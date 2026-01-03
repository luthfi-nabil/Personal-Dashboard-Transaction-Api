use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct AppSettings {
    pub app_setting_id : Uuid, 
    pub app_setting_key: String,
    pub app_setting_value: String,
    pub is_active: i32
}
