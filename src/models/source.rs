use serde::{Deserialize, Serialize};
use chrono::{Utc, DateTime};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Source {
    #[serde(skip_deserializing)]
    pub source_id : Uuid, 
    pub source: String,
    #[serde(skip_deserializing)]
    pub created_date: DateTime<Utc>,
    #[serde(skip_deserializing)]
    pub created_by: String,
    #[serde(skip_deserializing)]
    pub is_active: i32
}