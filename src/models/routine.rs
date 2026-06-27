use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct RoutineTransaction {
    pub routine_id: Uuid,
    pub item_name: String,
    pub price: f64,
    pub reminder: String,
    pub spending_category_id: Uuid,
    pub spending_category: String,
    pub status: String,
    pub last_bought_at: Option<NaiveDateTime>,
    pub created_date: NaiveDateTime,
    pub updated_date: NaiveDateTime,
    pub created_by: String,
    pub is_active: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoutineInput {
    pub routine_id: Option<Uuid>,
    pub item_name: String,
    pub price: f64,
    pub reminder: String,
    pub spending_category_id: Uuid,
    pub spending_category: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoutinePayment {
    pub routine_payment_id: Uuid,
    pub routine_id: Uuid,
    pub item_name: String,
    pub price: f64,
    pub spending_category_id: Uuid,
    pub spending_category: String,
    pub source_id: Uuid,
    pub source: String,
    pub bought_at: NaiveDateTime,
    pub created_by: String,
    pub is_active: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoutinePaymentInput {
    pub routine_payment_id: Option<Uuid>,
    pub price: f64,
    pub source_id: Uuid,
    pub source: String,
}
