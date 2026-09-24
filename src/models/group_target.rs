use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// How much one member means to spend on a group each month. The client
/// compares it with the member's group spendings of the month; nothing here
/// moves money.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupMemberTarget {
    pub group_id: Uuid,
    pub username: String,
    pub amount: f64,
    pub updated_date: NaiveDateTime,
}

/// Response of `GET /api/user/groups/{group_id}/targets`. The leader sees
/// every member's target; anyone else only their own. `enabled` is the
/// leader's switch - when it is off the clients hide target spendings.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupTargets {
    pub group_id: Uuid,
    pub is_leader: bool,
    pub enabled: bool,
    pub targets: Vec<GroupMemberTarget>,
}

/// Request body for `PUT /api/user/groups/{group_id}/target`. There is no
/// username on purpose: a member only ever sets their own target. Zero
/// clears it.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupTargetInput {
    pub amount: f64,
}

/// Request body for `PUT /api/user/groups/{group_id}/target-setting`.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupTargetSettingInput {
    pub enabled: bool,
}
