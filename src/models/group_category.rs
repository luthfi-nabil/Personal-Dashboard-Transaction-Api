use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A category that belongs to one spending group rather than to a user.
///
/// Group transactions are filed under these instead of the member's own
/// categories, so every member reads the recap with the same labels. Only the
/// group leader (admin) can add, rename or remove them.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupCategory {
    pub category_id: Uuid,
    pub group_id: Uuid,
    pub category_name: String,
    /// `spending` or `earning`.
    pub kind: String,
    pub created_by: String,
    pub created_date: NaiveDateTime,
}

/// Request body for `POST /api/user/groups/{group_id}/categories`. Saving an
/// existing id renames it, so a retry never duplicates a category.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupCategoryInput {
    pub category_id: Option<Uuid>,
    pub category_name: String,
    #[serde(default)]
    pub kind: Option<String>,
}
