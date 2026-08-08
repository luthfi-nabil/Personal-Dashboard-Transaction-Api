use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Spending {
    #[serde(skip_deserializing)]
    pub spending_id: Uuid,
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
    pub is_active: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpendingV2 {
    #[serde(skip_deserializing)]
    pub spending_id: Uuid,
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
    pub is_active: i32,
}

/// A single line item belonging to a spending, as stored/returned by the API.
#[derive(Debug, Serialize, Deserialize)]
pub struct SpendingDetailV2 {
    #[serde(skip_deserializing)]
    pub spending_detail_id: Uuid,
    pub spending_id: Uuid,
    pub item_name: String,
    pub quantity: f64,
    pub unit_price: f64,
    pub amount: f64,
    pub note: String,
    /// Whether the item is ticked off on the client's checklist. Every item is
    /// checked when it is first saved; unticking one records that it was not
    /// actually bought, without removing it from the breakdown.
    pub is_checked: bool,
    #[serde(skip_deserializing)]
    pub created_date: NaiveDateTime,
    #[serde(skip_deserializing)]
    pub created_by: String,
    #[serde(skip_deserializing)]
    pub is_active: i32,
}

fn default_quantity() -> f64 {
    1.0
}

fn default_checked() -> bool {
    true
}

/// A line item as submitted by the client when creating a spending. Sent by
/// the Flutter receipt scanner after the user confirms the recognised prices.
#[derive(Debug, Serialize, Deserialize)]
pub struct SpendingDetailParam {
    pub item_name: String,
    #[serde(default = "default_quantity")]
    pub quantity: f64,
    #[serde(default)]
    pub unit_price: f64,
    #[serde(default)]
    pub amount: f64,
    #[serde(default)]
    pub note: String,
    /// Defaults to ticked so clients that predate the checklist keep working.
    #[serde(default = "default_checked")]
    pub checked: bool,
}

/// Request body for `PUT /api/user/spending-details/{id}/checked`.
#[derive(Debug, Serialize, Deserialize)]
pub struct SpendingDetailCheckedInput {
    pub checked: bool,
}

/// Request body for `POST /api/user/spendings`. `details` is optional, so
/// existing clients that only post the flat spending keep working.
#[derive(Debug, Serialize, Deserialize)]
pub struct SpendingCreateV2 {
    #[serde(default)]
    pub total_amount: f64,
    pub description: String,
    pub spending_category_id: Uuid,
    pub spending_category: String,
    pub source_id: Uuid,
    pub source: String,
    #[serde(default)]
    pub details: Vec<SpendingDetailParam>,
}

/// Query string for `GET /api/user/spending-details`.
#[derive(Debug, Serialize, Deserialize)]
pub struct SpendingDetailParamQuery {
    pub spending_id: Option<Uuid>,
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
    pub spending_id: Option<Uuid>,
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
    pub is_active: i32,
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
    pub is_active: i32,
}
