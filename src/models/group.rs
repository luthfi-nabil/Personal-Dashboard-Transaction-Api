use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A shared ledger several users tag their own spendings/earnings into.
///
/// The transactions themselves stay personal - a tag only links an existing
/// `spending` / `earning` row to the group, so every member can read the
/// group's recap without being able to touch anyone's own records.
#[derive(Debug, Serialize, Deserialize)]
pub struct SpendingGroup {
    pub group_id: Uuid,
    pub group_name: String,
    /// The creator. Only the leader can rename the group or switch it on/off.
    pub leader: String,
    /// Current on/off state, i.e. the latest entry of [`Self::status_log`].
    pub is_active: bool,
    pub created_date: NaiveDateTime,
    pub updated_date: NaiveDateTime,
    pub members: Vec<GroupMember>,
    /// Every on/off switch, oldest first. Kept as history rather than a single
    /// flag so a transaction synced late can still be judged against the state
    /// the group was in when the transaction happened.
    pub status_log: Vec<GroupStatusChange>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroupMember {
    pub group_id: Uuid,
    pub username: String,
    pub added_by: String,
    pub added_date: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroupStatusChange {
    pub status_id: Uuid,
    pub group_id: Uuid,
    pub is_active: bool,
    pub changed_by: String,
    pub changed_at: NaiveDateTime,
}

/// One member's spending/earning as seen from the group recap.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupTransaction {
    pub group_id: Uuid,
    /// `spending` or `earning`.
    pub transaction_type: String,
    pub transaction_id: Uuid,
    pub total_amount: f64,
    pub description: String,
    pub category: String,
    pub created_date: NaiveDateTime,
    pub created_by: String,
    /// The spender's source it was paid from (or received into), or
    /// "Group balance" for one paid from a member's group balance.
    pub source: String,
    /// The group was switched off when this transaction happened, so the recap
    /// counts it separately from the regular totals.
    pub after_turned_off: bool,
}

/// Request body for `POST /api/user/groups`. The id is client-generated so a
/// group created offline can be retried without duplicating it.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupInput {
    pub group_id: Option<Uuid>,
    pub group_name: String,
    #[serde(default)]
    pub created_date: Option<NaiveDateTime>,
}

/// Request body for `POST /api/user/groups/{group_id}/members`.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupMemberInput {
    pub username: String,
    #[serde(default)]
    pub added_date: Option<NaiveDateTime>,
}

/// Request body for `PUT /api/user/groups/{group_id}/status`. `status_id` is
/// client-generated and `changed_at` is when the leader flipped the switch on
/// the device, so a switch made offline lands at the right point in history.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupStatusInput {
    pub status_id: Option<Uuid>,
    pub is_active: bool,
    #[serde(default)]
    pub changed_at: Option<NaiveDateTime>,
}
