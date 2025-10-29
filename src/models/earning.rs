use serde::{Deserialize, Serialize};
use chrono::{Utc, DateTime};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Earning {
    #[serde(skip_deserializing)]
    pub earning_id : Uuid, 
    pub total_amount: f64,
    pub description: String,
    pub earning_category_id: Uuid,
    pub earning_category: String,
    pub source_id: Uuid,
    pub source: String,
    #[serde(skip_deserializing)]
    pub created_date: DateTime<Utc>,
    #[serde(skip_deserializing)]
    pub created_by: String,
    #[serde(skip_deserializing)]
    pub is_active: i32
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EarningParam {
    pub description: Option<String>,
    pub earning_category_id: Option<Uuid>,
    pub earning_category: Option<String>,
    pub source_id: Option<Uuid>,
    pub source: Option<String>,
    pub month: Option<i32>,
    pub year: Option<i32>,
    pub day: Option<i32>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EarningCategory {
    #[serde(skip_deserializing)]
    pub earning_category_id: Uuid,
    pub earning_category: String,
    #[serde(skip_deserializing)]
    pub created_date: DateTime<Utc>,
    #[serde(skip_deserializing)]
    pub created_by: String,
    #[serde(skip_deserializing)]
    pub is_active: i32
}