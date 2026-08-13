use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct PlannedExpenseItem {
    pub planned_expense_id: Uuid,
    pub item_name: String,
    pub price: f64,
    pub transaction_type: String,
    pub category_id: Option<Uuid>,
    pub category: Option<String>,
    pub notes: Option<String>,
    pub priority: String,
    pub status: String,
    pub fulfilled_price: Option<f64>,
    pub fulfilled_at: Option<NaiveDateTime>,
    pub canceled_at: Option<NaiveDateTime>,
    pub created_date: NaiveDateTime,
    pub updated_date: NaiveDateTime,
    pub created_by: String,
    pub is_active: i32,
}

/// Request body for `POST /api/user/planned-expenses`. `created_date` is
/// optional so older clients keep working; the Flutter app sends the moment the
/// item was entered, which for one queued in local mode is earlier than the
/// push.
#[derive(Debug, Serialize, Deserialize)]
pub struct PlannedExpenseInput {
    #[serde(alias = "wishlist_id")]
    pub planned_expense_id: Option<Uuid>,
    pub item_name: String,
    pub price: f64,
    pub transaction_type: Option<String>,
    pub category_id: Option<Uuid>,
    pub category: Option<String>,
    pub notes: Option<String>,
    pub priority: String,
    #[serde(default)]
    pub created_date: Option<NaiveDateTime>,
}

/// Request body for `PUT /api/user/planned-expenses/{id}/status`. `changed_at`
/// is when the item was fulfilled or canceled on the device; see
/// [PlannedExpenseInput] for why it is optional.
#[derive(Debug, Serialize, Deserialize)]
pub struct PlannedExpenseStatusInput {
    pub status: String,
    pub fulfilled_price: Option<f64>,
    #[serde(default)]
    pub changed_at: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlannedExpenseCategory {
    pub planned_expense_category_id: Uuid,
    pub planned_expense_category: String,
    pub created_date: NaiveDateTime,
    pub created_by: String,
    pub is_active: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlannedExpenseCategoryInput {
    pub planned_expense_category: String,
}
