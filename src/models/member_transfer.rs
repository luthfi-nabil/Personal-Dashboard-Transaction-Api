use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Money moved from one member's source to another member's source.
///
/// Always initiated by the sender: there is no way to pull money out of
/// someone else's source. It is stored as a spending in the sender's records
/// and an earning in the recipient's, both under the server's Transfer
/// category, so each side's source balance moves the right way.
#[derive(Debug, Serialize, Deserialize)]
pub struct MemberTransfer {
    pub transfer_id: Uuid,
    pub from_user: String,
    pub from_source_id: Uuid,
    pub from_source: String,
    pub to_user: String,
    pub to_source_id: Uuid,
    pub to_source: String,
    pub amount: f64,
    pub description: String,
    pub spending_id: Uuid,
    pub earning_id: Uuid,
    pub created_date: NaiveDateTime,
}

/// One of another member's sources, as offered when choosing where a
/// transfer lands. Names only - never balances.
#[derive(Debug, Serialize, Deserialize)]
pub struct MemberSource {
    pub source_id: Uuid,
    pub source: String,
}

/// Request body for `POST /api/user/member-transfers`. `transfer_id` is
/// client-generated so a retried request transfers once.
#[derive(Debug, Serialize, Deserialize)]
pub struct MemberTransferInput {
    pub transfer_id: Option<Uuid>,
    pub to_username: String,
    pub from_source_id: Uuid,
    pub to_source_id: Uuid,
    pub amount: f64,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub created_date: Option<NaiveDateTime>,
}
