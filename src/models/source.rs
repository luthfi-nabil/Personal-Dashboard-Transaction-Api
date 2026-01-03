use serde::{Deserialize, Serialize};
use chrono::{DateTime, NaiveDateTime, Utc};
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

#[derive(Debug, Serialize, Deserialize)]
pub struct SourceV2 {
    #[serde(skip_deserializing)]
    pub source_id : Uuid, 
    pub source: String,
    #[serde(skip_deserializing)]
    pub created_date: NaiveDateTime,
    #[serde(skip_deserializing)]
    pub created_by: String,
    #[serde(skip_deserializing)]
    pub is_active: i32
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SourceBalance {
    pub source: String,
    pub source_id: Uuid,
    pub total: f64,
}