use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One investment holding, shown on the client's Finance > Investment page.
///
/// Deliberately not a `source`: source balances are liquid cash, these are
/// not, which is why the clients' headline figure is now labelled "Liquid".
///
/// A single shape covers both kinds of holding. `units` and the two unit
/// prices mean whatever [kind] implies:
///
/// | kind          | units       | unit prices    |
/// |---------------|-------------|----------------|
/// | `mutual_fund` | units owned | NAB per unit   |
/// | `gold`        | grams       | price per gram |
/// | `silver`      | grams       | price per gram |
#[derive(Debug, Serialize, Deserialize)]
pub struct Investment {
    pub investment_id: Uuid,
    pub kind: String,
    pub name: String,
    pub provider: String,
    pub units: f64,
    /// Cost basis per unit. Set once when the holding is recorded and never
    /// touched again, so gain/loss stays honest across price refreshes.
    pub buy_unit_price: f64,
    /// Latest known value per unit: typed in by hand for reksa dana (no public
    /// NAB API exists) and refreshed from a price API for gold and silver.
    /// Stored rather than fetched on read so clients still show a value
    /// offline - [price_updated_date] says how stale it is.
    pub last_unit_price: f64,
    pub price_source: String,
    pub price_updated_date: Option<NaiveDateTime>,
    pub notes: String,
    pub acquired_date: NaiveDateTime,
    pub created_date: NaiveDateTime,
    pub updated_date: NaiveDateTime,
    pub created_by: String,
    pub is_active: i32,
}

/// The three holdings the clients know how to render. Anything else is
/// rejected rather than stored, so a typo cannot create a fourth silent
/// category that no screen groups.
pub const INVESTMENT_KINDS: [&str; 3] = ["mutual_fund", "gold", "silver"];

/// Request body for `POST /api/user/investments`.
///
/// `investment_id` is client-generated so a queued offline write can be
/// retried without creating a duplicate. Dates arrive as free-form strings for
/// the same reasons as elsewhere in this API; see
/// [`crate::models::consumable::parse_date`].
#[derive(Debug, Serialize, Deserialize)]
pub struct InvestmentInput {
    pub investment_id: Option<Uuid>,
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub units: f64,
    #[serde(default)]
    pub buy_unit_price: f64,
    /// Left out on a fresh holding, where the buy price is the best value
    /// known; the handler falls back to it.
    pub last_unit_price: Option<f64>,
    #[serde(default)]
    pub price_source: String,
    pub price_updated_date: Option<String>,
    #[serde(default)]
    pub notes: String,
    pub acquired_date: Option<String>,
}

/// Request body for `PUT /api/user/investments/{id}/price`.
///
/// Split from the full upsert because refreshing a price is a different act
/// from editing a holding: the metal price refresh writes many rows at once
/// and must not be able to disturb units or cost basis.
#[derive(Debug, Serialize, Deserialize)]
pub struct InvestmentPriceInput {
    pub last_unit_price: f64,
    #[serde(default)]
    pub price_source: String,
    pub price_updated_date: Option<String>,
}
