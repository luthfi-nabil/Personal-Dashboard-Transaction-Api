use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Money one group member asks another for (or sends them) for a purpose -
/// the `tag`, a spending category name such as "Transportation".
///
/// A `request` starts `requested`: the recipient (`requester`) asked the
/// `payer`, who either sends it (`sent`, a member transfer between their
/// sources) or rejects it; the requester can cancel it while it waits. A
/// `send` is the payer sending directly, so it starts `sent`.
///
/// When `tracked`, the money is earmarked: the recipient's later spendings in
/// the tag's category use it up, and the remainder is their "tag balance"
/// (e.g. Transportation balance). The group leader can `waive` a sent,
/// tracked one: the unspent remainder is dropped from the tag balance (back
/// to what it was before), the spendings stay recorded, and the request is
/// marked `waived`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupFundRequest {
    pub request_id: Uuid,
    pub group_id: Uuid,
    /// `request` or `send`.
    pub kind: String,
    /// Who receives the money.
    pub requester: String,
    /// Who gives it.
    pub payer: String,
    pub amount: f64,
    pub tag: String,
    pub note: String,
    pub tracked: bool,
    /// `requested`, `rejected`, `canceled`, `sent` or `waived`.
    pub status: String,
    pub to_source_id: Option<Uuid>,
    pub to_source: Option<String>,
    pub from_source_id: Option<Uuid>,
    pub from_source: Option<String>,
    /// The member transfer that moved the money, once sent.
    pub transfer_id: Option<Uuid>,
    pub created_by: String,
    pub created_date: NaiveDateTime,
    pub responded_at: Option<NaiveDateTime>,
    /// When the money moved; spendings from then on count against it.
    pub sent_at: Option<NaiveDateTime>,
    pub waived_by: Option<String>,
    pub waived_at: Option<NaiveDateTime>,
    pub waive_note: Option<String>,
    /// Tracked and sent/waived only: how much of it the recipient has spent
    /// in the tag's category, what is left, and - once waived - how much was
    /// dropped from the tag balance.
    #[serde(default)]
    pub spent: f64,
    #[serde(default)]
    pub remaining: f64,
    #[serde(default)]
    pub waived_amount: f64,
    /// The spendings that used it up (oldest first).
    #[serde(default)]
    pub usage: Vec<FundUsage>,
}

/// Part of one of the recipient's spendings that used up a tracked fund.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FundUsage {
    pub spending_id: Uuid,
    pub description: String,
    pub spending_amount: f64,
    /// How much of this spending was taken from the fund.
    pub amount: f64,
    pub created_date: NaiveDateTime,
}

/// A member's balance for one tag, summed over their tracked funds.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TagBalance {
    pub group_id: Uuid,
    pub username: String,
    pub tag: String,
    pub received: f64,
    pub spent: f64,
    pub waived: f64,
    /// `received - spent - waived`: what is still earmarked for the tag.
    pub balance: f64,
}

/// `GET /api/user/fund-requests`.
#[derive(Debug, Serialize, Deserialize)]
pub struct FundRequestsView {
    pub requests: Vec<GroupFundRequest>,
    pub balances: Vec<TagBalance>,
}

/// `POST /api/user/groups/{group_id}/fund-requests`.
///
/// `kind: "request"` - the caller asks `username` for money, into one of
/// the caller's own sources (`to_source_id`, optional until it is sent).
/// `kind: "send"` - the caller sends `username` money now, from one of the
/// caller's sources (`from_source_id`) into one of theirs (`to_source_id`).
#[derive(Debug, Serialize, Deserialize)]
pub struct FundRequestInput {
    pub request_id: Option<Uuid>,
    #[serde(default = "default_kind")]
    pub kind: String,
    pub username: String,
    pub amount: f64,
    pub tag: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub tracked: bool,
    #[serde(default)]
    pub to_source_id: Option<Uuid>,
    #[serde(default)]
    pub from_source_id: Option<Uuid>,
}

fn default_kind() -> String {
    "request".to_string()
}

/// `PUT .../fund-requests/{id}/fulfill` - the payer sends it from
/// `from_source_id`. `to_source_id` is only needed when the requester did not
/// pick one.
#[derive(Debug, Serialize, Deserialize)]
pub struct FundFulfilInput {
    pub from_source_id: Uuid,
    #[serde(default)]
    pub to_source_id: Option<Uuid>,
}

/// `PUT .../fund-requests/{id}/waive` (group leader only).
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct FundWaiveInput {
    #[serde(default)]
    pub note: String,
}
