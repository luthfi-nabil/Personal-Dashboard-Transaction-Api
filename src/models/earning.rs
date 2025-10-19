use serde::{Deserialize, Serialize};
use chrono::{Utc, DateTime};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Earning {
    pub earning_id : Uuid, 
    pub total_amount: f64,
    pub description: String,
    pub earning_category_id: Uuid,
    pub earning_category: String,
    pub source_id: Uuid,
    pub source: String,
    pub created_date: DateTime<Utc>,
    pub created_by: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EarningCategory {
    pub earning_category_id: Uuid,
    pub earning_category: String,
    pub created_date: DateTime<Utc>,
    pub created_by: String
}