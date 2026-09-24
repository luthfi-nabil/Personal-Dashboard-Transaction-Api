use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One movement of a member's group balance.
///
/// Every member has their own balance inside each group, separate from their
/// personal sources and usable only for that group's spending. It is the sum
/// of that member's entries minus their tagged group spendings: `top_up`
/// entries are positive; `spending` entries (paid from the balance) and
/// `transaction` rows (a personal spending added to the group - listed in the
/// history, not stored as an entry) are negative. It may go negative.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupBalanceEntry {
    pub entry_id: Uuid,
    pub group_id: Uuid,
    /// Whose balance moved. Always the member who made the entry: nobody can
    /// top up, or spend, someone else's balance.
    pub username: String,
    /// Signed: positive adds to the balance, negative spends from it.
    pub amount: f64,
    /// `top_up`, `spending`, `transaction`, `reimburse_in`/`reimburse_out`,
    /// `split_in`/`split_out` or `balance_return`.
    pub entry_type: String,
    pub description: String,
    pub spending_category_id: Option<Uuid>,
    pub spending_category: Option<String>,
    pub created_date: NaiveDateTime,
}

/// One member's balance in a group.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupMemberBalance {
    pub group_id: Uuid,
    pub username: String,
    pub balance: f64,
    pub total_top_up: f64,
    pub total_spent: f64,
}

/// Response of `GET /api/user/groups/{group_id}/balances`. The leader sees
/// every member; anyone else only themselves.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupBalances {
    pub group_id: Uuid,
    pub is_leader: bool,
    pub balances: Vec<GroupMemberBalance>,
    pub entries: Vec<GroupBalanceEntry>,
}

/// Request body for `POST /api/user/groups/{group_id}/balance/top-ups`.
/// There is deliberately no username: a top-up always goes to the caller.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupTopUpInput {
    pub entry_id: Option<Uuid>,
    pub amount: f64,
    #[serde(default)]
    pub description: String,
}

/// Request body for `POST /api/user/groups/{group_id}/balance/spendings` - a
/// group transaction paid from the caller's group balance.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupBalanceSpendingInput {
    pub entry_id: Option<Uuid>,
    pub amount: f64,
    #[serde(default)]
    pub description: String,
    pub spending_category_id: Uuid,
    pub spending_category: String,
    #[serde(default)]
    pub created_date: Option<NaiveDateTime>,
}
