use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A named bundle built up from real purchases - e.g. "Camping trip" - so
/// items tagged from several transactions over time can be seen and totaled
/// together. Unlike a [crate::models::planned_expense::PlannedExpenseItem]
/// this has no status/fulfillment: it is just a name and a running list.
#[derive(Debug, Serialize, Deserialize)]
pub struct PlannedTransaction {
    pub planned_transaction_id: Uuid,
    pub name: String,
    pub created_date: NaiveDateTime,
    pub updated_date: NaiveDateTime,
    #[serde(skip_deserializing)]
    pub created_by: String,
    #[serde(skip_deserializing)]
    pub is_active: i32,
}

/// Request body for `POST /api/user/planned-transactions`. Posting an id that
/// already exists updates it (name only), so a write queued offline can be
/// retried safely.
#[derive(Debug, Serialize, Deserialize)]
pub struct PlannedTransactionInput {
    pub planned_transaction_id: Option<Uuid>,
    pub name: String,
    #[serde(default)]
    pub created_date: Option<NaiveDateTime>,
}

fn default_quantity() -> f64 {
    1.0
}

/// One item tagged into a [PlannedTransaction]. `spending_id` /
/// `spending_detail_id` point back at the real transaction line item it came
/// from, so the client can link to the purchase.
#[derive(Debug, Serialize, Deserialize)]
pub struct PlannedTransactionDetail {
    pub planned_transaction_detail_id: Uuid,
    pub planned_transaction_id: Uuid,
    pub item_name: String,
    pub quantity: f64,
    pub unit_price: f64,
    pub amount: f64,
    pub note: String,
    pub spending_id: Option<Uuid>,
    pub spending_detail_id: Option<Uuid>,
    pub created_date: NaiveDateTime,
    #[serde(skip_deserializing)]
    pub created_by: String,
    #[serde(skip_deserializing)]
    pub is_active: i32,
}

/// Request body for `POST /api/user/planned-transactions/{id}/details`.
#[derive(Debug, Serialize, Deserialize)]
pub struct PlannedTransactionDetailInput {
    pub planned_transaction_detail_id: Option<Uuid>,
    pub item_name: String,
    #[serde(default = "default_quantity")]
    pub quantity: f64,
    #[serde(default)]
    pub unit_price: f64,
    #[serde(default)]
    pub amount: f64,
    #[serde(default)]
    pub note: String,
    pub spending_id: Option<Uuid>,
    pub spending_detail_id: Option<Uuid>,
    #[serde(default)]
    pub created_date: Option<NaiveDateTime>,
}

/// Query string for `GET /api/user/planned-transaction-details`.
#[derive(Debug, Serialize, Deserialize)]
pub struct PlannedTransactionDetailQuery {
    pub planned_transaction_id: Option<Uuid>,
}
