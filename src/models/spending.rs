use serde::{Deserialize, Serialize};
use chrono::{Utc, NaiveDateTime, DateTime};
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
    pub is_active: i32
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpendingV2 {
    #[serde(skip_deserializing)]
    pub spending_id : Uuid, 
    pub total_amount: f64,
    pub description: String,
    pub spending_category_id: Uuid,
    pub spending_category: String,
    pub source_id: Uuid,
    pub source: String,
    #[serde(skip_deserializing)]
    pub created_date: NaiveDateTime,
    #[serde(skip_deserializing)]
    pub created_by: String,
    #[serde(skip_deserializing)]
    pub is_active: i32
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpendingParam {
    pub description: Option<String>,
    pub spending_category_id: Option<Uuid>,
    pub spending_category: Option<String>,
    pub source_id: Option<Uuid>,
    pub source: Option<String>,
    pub month: Option<i32>,
    pub year: Option<i32>,
    pub day: Option<i32>,
    pub spending_id: Option<Uuid>
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
    pub is_active: i32
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpendingCategoryV2 {
    #[serde(skip_deserializing)]
    pub spending_category_id: Uuid,
    pub spending_category: String,
    #[serde(skip_deserializing)]
    pub created_date: NaiveDateTime,
    #[serde(skip_deserializing)]
    pub created_by: String,
    #[serde(skip_deserializing)]
    pub is_active: i32
}