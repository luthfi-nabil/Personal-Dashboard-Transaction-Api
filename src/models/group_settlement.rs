use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A member paying another member back for one of their group spendings.
///
/// `paid_by` pays out of one of their own sources (`from_source_id`) or, when
/// that is empty, out of their balance in this group. The owner of the
/// spending receives `personal_amount` in one of their sources
/// (`to_source_id`). With `return_balance` the reimbursed amount is also
/// given back to the owner's group balance, undoing that much of the
/// deduction the spending made; without it the group balance stays as it is.
///
/// Older clients split the payment into `personal_amount` and a
/// `group_amount` paid into the owner's group balance instead.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupReimbursement {
    pub reimbursement_id: Uuid,
    pub group_id: Uuid,
    /// The reimbursed group transaction (always a spending).
    pub transaction_id: Uuid,
    /// Who recorded the spending and gets paid back.
    pub owner: String,
    pub paid_by: String,
    /// `None` = paid from `paid_by`'s group balance.
    pub from_source_id: Option<Uuid>,
    pub from_source: String,
    pub personal_amount: f64,
    pub to_source_id: Option<Uuid>,
    pub to_source: Option<String>,
    pub group_amount: f64,
    /// `personal_amount + group_amount`.
    pub amount: f64,
    /// The owner's group balance got `amount` back.
    #[serde(default)]
    pub return_balance: bool,
    pub description: String,
    pub created_date: NaiveDateTime,
}

/// One person's share of a group spending under a split bill.
///
/// The person is either a registered group member (`username` set) who pays
/// through the app, or just a name (`username` empty) whose payments the
/// owner records by hand.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupSplitShare {
    pub share_id: Uuid,
    pub group_id: Uuid,
    pub transaction_id: Uuid,
    pub owner: String,
    pub username: Option<String>,
    /// The username, or the free-text name.
    pub name: String,
    pub amount: f64,
    /// Sum of approved payments.
    pub paid_amount: f64,
    /// Sum of payments still waiting for the owner.
    pub pending_amount: f64,
    pub created_date: NaiveDateTime,
}

/// A payment towards a split-bill share.
///
/// A registered member's payment starts `pending`; no money moves until the
/// owner approves it (`approved`) - or `rejected`, or `cancelled` by the
/// payer. A payment for a name-only share is recorded by the owner and is
/// `approved` straight away.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupSplitPayment {
    pub payment_id: Uuid,
    pub share_id: Uuid,
    pub group_id: Uuid,
    pub transaction_id: Uuid,
    pub owner: String,
    /// The paying member, `None` for a name-only share.
    pub paid_by: Option<String>,
    pub payer_name: String,
    pub amount: f64,
    pub status: String,
    /// `None` = from the payer's group balance (or nothing, for a name).
    pub from_source_id: Option<Uuid>,
    pub from_source: Option<String>,
    /// Where the owner took the money; `None` = the owner's group balance.
    pub to_source_id: Option<Uuid>,
    pub to_source: Option<String>,
    pub note: String,
    pub requested_at: NaiveDateTime,
    pub reviewed_at: Option<NaiveDateTime>,
}

/// Response of `GET /api/user/groups/{group_id}/settlements`: every
/// reimbursement and split bill in the group, plus the group's saved names.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupSettlements {
    pub group_id: Uuid,
    pub reimbursements: Vec<GroupReimbursement>,
    pub shares: Vec<GroupSplitShare>,
    pub payments: Vec<GroupSplitPayment>,
    /// Names of people outside the app used in this group's split bills.
    pub names: Vec<String>,
}

/// Request body for `POST /api/user/groups/{group_id}/reimbursements`.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupReimbursementInput {
    pub reimbursement_id: Option<Uuid>,
    pub transaction_id: Uuid,
    #[serde(default)]
    pub from_source_id: Option<Uuid>,
    #[serde(default)]
    pub from_group_balance: bool,
    #[serde(default)]
    pub personal_amount: f64,
    #[serde(default)]
    pub to_source_id: Option<Uuid>,
    #[serde(default)]
    pub group_amount: f64,
    /// Set by current clients: the whole amount goes to the owner's source
    /// (`personal_amount`), and `true` also returns it to the owner's group
    /// balance. `group_amount` is ignored when this is present.
    #[serde(default)]
    pub return_balance: Option<bool>,
    #[serde(default)]
    pub description: String,
}

/// Request body for `POST /api/user/groups/{group_id}/split-shares`. Either
/// `username` (a group member) or `name` (anyone) is set.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupSplitShareInput {
    pub share_id: Option<Uuid>,
    pub transaction_id: Uuid,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    pub amount: f64,
}

/// Request body for `POST .../split-shares/{share_id}/payments` - the
/// member who owes pays part or all of their share.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupSplitPaymentInput {
    pub payment_id: Option<Uuid>,
    pub amount: f64,
    #[serde(default)]
    pub from_source_id: Option<Uuid>,
    #[serde(default)]
    pub from_group_balance: bool,
    #[serde(default)]
    pub note: String,
}

/// Request body for `PUT .../split-payments/{payment_id}/review` (owner).
/// On approve the money lands in `to_source_id`, or the owner's group balance
/// with `to_group_balance`.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupSplitReviewInput {
    pub approve: bool,
    #[serde(default)]
    pub to_source_id: Option<Uuid>,
    #[serde(default)]
    pub to_group_balance: bool,
}

/// Request body for `POST .../split-shares/{share_id}/manual-payments` -
/// the owner records money received from a name-only person.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupSplitManualPaymentInput {
    pub payment_id: Option<Uuid>,
    pub amount: f64,
    #[serde(default)]
    pub to_source_id: Option<Uuid>,
    #[serde(default)]
    pub to_group_balance: bool,
    #[serde(default)]
    pub note: String,
}
