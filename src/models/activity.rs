use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct ActivityCategory {
    pub activity_category_id: Uuid,
    pub activity_category: String,
    pub created_date: NaiveDateTime,
    pub created_by: String,
    pub is_active: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActivityCategoryInput {
    pub activity_category: String,
}
