use serde::{Deserialize, Serialize};
use chrono::{DateTime, NaiveDateTime, Utc};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Debt {
    #[serde(skip_deserializing)]
    pub debt_id : Uuid, 
    pub amount: f64,
    pub description: String,
    pub debt_type: i32,
    #[serde(skip_deserializing)]
    pub debt_earning_id: Option<Uuid>,
    #[serde(skip_deserializing)]
    pub debt_spending_id: Option<Uuid>,
    pub status: i32,
    #[serde(skip_deserializing)]
    pub created_date: NaiveDateTime,
    #[serde(skip_deserializing)]
    pub created_by: String,
    #[serde(skip_deserializing)]
    pub is_active: i32
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DebtParam {
    pub description: Option<String>,
    pub debt_id: Option<String>,
    pub debt_type: Option<i32>,
    pub debt_earning_id: Option<String>,
    pub debt_spending_id: Option<String>,
    pub status: Option<i32>
}