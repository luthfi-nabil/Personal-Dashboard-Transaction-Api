use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A recurring purchase the whole group shares (rent, groceries, internet).
///
/// Only the group leader can add or remove one. Any member can pay it: the
/// payment becomes a spending in the payer's own records - so it comes out of
/// their balance - and is tagged into the group so it shows up in the group's
/// transaction history too.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupRoutine {
    pub routine_id: Uuid,
    pub group_id: Uuid,
    pub item_name: String,
    pub price: f64,
    /// `daily`, `weekly`, `monthly`, ... - same values as personal routines.
    pub reminder: String,
    pub spending_category_id: Uuid,
    pub spending_category: String,
    pub last_paid_at: Option<NaiveDateTime>,
    pub last_paid_by: Option<String>,
    pub created_by: String,
    pub created_date: NaiveDateTime,
    pub updated_date: NaiveDateTime,
}

/// One payment of a [`GroupRoutine`] by one member.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupRoutinePayment {
    pub payment_id: Uuid,
    pub routine_id: Uuid,
    pub group_id: Uuid,
    /// The spending created in the payer's own records.
    pub spending_id: Uuid,
    pub item_name: String,
    pub price: f64,
    pub source_id: Uuid,
    pub source: String,
    pub paid_by: String,
    pub paid_at: NaiveDateTime,
}

/// A one-off purchase the group intends to make.
///
/// Leader-added items start as `planned`. A regular member can only request
/// one: it starts as `requested` and waits for the leader to approve
/// (`planned`) or reject (`rejected`) it. A `planned` item is fulfilled by
/// whichever member pays for it (`fulfilled`), which - like a routine payment
/// - becomes a spending of the payer tagged into the group.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupPlannedExpense {
    pub planned_expense_id: Uuid,
    pub group_id: Uuid,
    pub item_name: String,
    pub price: f64,
    pub spending_category_id: Uuid,
    pub spending_category: String,
    pub notes: String,
    /// `requested`, `planned`, `rejected`, `fulfilled` or `canceled`.
    pub status: String,
    pub requested_by: String,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<NaiveDateTime>,
    pub fulfilled_by: Option<String>,
    pub fulfilled_price: Option<f64>,
    pub fulfilled_at: Option<NaiveDateTime>,
    pub spending_id: Option<Uuid>,
    pub created_date: NaiveDateTime,
    pub updated_date: NaiveDateTime,
}

/// Request body for `POST /api/user/groups/{group_id}/routines`.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupRoutineInput {
    pub routine_id: Option<Uuid>,
    pub item_name: String,
    pub price: f64,
    pub reminder: String,
    pub spending_category_id: Uuid,
    pub spending_category: String,
}

/// Request body for `POST /api/user/groups/{group_id}/planned-expenses`.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupPlannedExpenseInput {
    pub planned_expense_id: Option<Uuid>,
    pub item_name: String,
    pub price: f64,
    pub spending_category_id: Uuid,
    pub spending_category: String,
    #[serde(default)]
    pub notes: String,
}

/// Request body for `PUT .../planned-expenses/{id}/review` (leader only).
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupPlannedExpenseReviewInput {
    pub approve: bool,
}

/// Request body for paying a routine or fulfilling a planned expense.
/// `payment_id` is client-generated so a retried request pays once.
///
/// The money comes either from one of the payer's own sources (`source_id`)
/// or, with `from_group_balance`, from the payer's balance in this group.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupPaymentInput {
    pub payment_id: Option<Uuid>,
    pub price: f64,
    #[serde(default)]
    pub source_id: Option<Uuid>,
    #[serde(default)]
    pub from_group_balance: bool,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub paid_at: Option<NaiveDateTime>,
}
