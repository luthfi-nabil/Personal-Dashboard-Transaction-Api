use serde::{Deserialize, Serialize};
use chrono::{Utc, DateTime};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Spending {
    #[serde(skip_deserializing)]
    pub spending_id : Uuid, 
    pub total_amount: f64,
    pub description: String,
    pub spending_category_id: Uuid,
    pub spending_category: String,
    pub source_id: Uuid,
    pub source: String,
    #[serde(skip_deserializing)]
    pub created_date: DateTime<Utc>,
    #[serde(skip_deserializing)]
    pub created_by: String,
    #[serde(skip_deserializing)]
    pub is_active: bool
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpendingCategory {
    #[serde(skip_deserializing)]
    pub spending_category_id: Uuid,
    pub spending_category: String,
    #[serde(skip_deserializing)]
    pub created_date: DateTime<Utc>,
    #[serde(skip_deserializing)]
    pub created_by: String,
    #[serde(skip_deserializing)]
    pub is_active: bool
}