use serde::{Deserialize, Serialize};
use chrono::{Utc, DateTime};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Spending {
    pub spending_id : Uuid, 
    pub total_amount: f64,
    pub description: String,
    pub spending_category_id: Uuid,
    pub spending_category: String,
    pub source_id: Uuid,
    pub source: String,
    pub created_date: DateTime<Utc>,
    pub created_by: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpendingCategory {
    pub spending_category_id: Uuid,
    pub spending_category: String,
    pub created_date: DateTime<Utc>,
    pub created_by: String
}