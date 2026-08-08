use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One physical unit of something that gets used up - a bottle of shampoo, a
/// tube of toothpaste, a pack of razor blades.
///
/// Units are tracked separately rather than as a stock count: buying a
/// three-pack creates three rows, each with its own in/out dates, so how long
/// one unit lasts can be read straight off the pair. A row with no `out_date`
/// is still in use.
#[derive(Debug, Serialize, Deserialize)]
pub struct Consumable {
    pub consumable_id: Uuid,
    pub item_name: String,
    pub notes: String,
    /// Position within the batch it was bought in ("2 of 3"). Both default to
    /// 1 for a single unit added by hand.
    pub unit_index: i32,
    pub unit_total: i32,
    pub price: f64,
    pub in_date: NaiveDateTime,
    pub out_date: Option<NaiveDateTime>,
    /// Set when the unit was created from a transaction's line item, so the
    /// client can link back to what it was bought on.
    pub spending_id: Option<Uuid>,
    pub spending_detail_id: Option<Uuid>,
    pub created_date: NaiveDateTime,
    pub updated_date: NaiveDateTime,
    pub created_by: String,
    pub is_active: i32,
}

fn default_unit() -> i32 {
    1
}

/// Request body for `POST /api/user/consumables`.
///
/// Dates arrive as free-form strings because they come from several places -
/// a picker, `DateTime.toIso8601String()`, or a transaction date the API
/// itself printed as `YYYY-MM-DD HH:MM:SS`. See [parse_date].
#[derive(Debug, Serialize, Deserialize)]
pub struct ConsumableInput {
    pub consumable_id: Option<Uuid>,
    pub item_name: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default = "default_unit")]
    pub unit_index: i32,
    #[serde(default = "default_unit")]
    pub unit_total: i32,
    #[serde(default)]
    pub price: f64,
    pub in_date: Option<String>,
    pub out_date: Option<String>,
    pub spending_id: Option<Uuid>,
    pub spending_detail_id: Option<Uuid>,
}

/// Request body for `PUT /api/user/consumables/{id}/out`. A null `out_date`
/// puts the unit back in use.
#[derive(Debug, Serialize, Deserialize)]
pub struct ConsumableOutInput {
    pub out_date: Option<String>,
}

/// Reads the date formats this API is handed in practice, newest-style first.
/// Returns `None` for an absent or unparseable value, letting the caller fall
/// back to "now".
pub fn parse_date(raw: &Option<String>) -> Option<NaiveDateTime> {
    let value = raw.as_ref()?.trim().to_string();
    if value.is_empty() {
        return None;
    }
    // `2026-08-08T10:11:12.345`, with or without the fractional part.
    if let Ok(parsed) = NaiveDateTime::parse_from_str(&value, "%Y-%m-%dT%H:%M:%S%.f") {
        return Some(parsed);
    }
    // What this API prints back: `2026-08-08 10:11:12`.
    if let Ok(parsed) = NaiveDateTime::parse_from_str(&value, "%Y-%m-%d %H:%M:%S%.f") {
        return Some(parsed);
    }
    // A bare date from a picker.
    if let Ok(parsed) = chrono::NaiveDate::parse_from_str(&value, "%Y-%m-%d") {
        return Some(parsed.and_hms_opt(0, 0, 0).unwrap_or_default());
    }
    // A trailing zone offset (`...+07:00`, `...Z`) - keep the local wall clock,
    // which is what every other date in this schema stores.
    if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(&value) {
        return Some(parsed.naive_local());
    }
    None
}
