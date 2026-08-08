//! Read-only client for login-api's `app_settings`.
//!
//! `app_settings` used to live in transaction_db. It now lives in login_db and
//! is owned by login-api, so the transfer/recount/debt category wiring is
//! fetched over HTTP instead of queried locally. Only the global rows are
//! needed here, which is why the token-less `GET /api/settings` route is used.
//!
//! Responses are cached in-process for [`CACHE_TTL`] so a burst of writes does
//! not turn into a burst of HTTP calls. If login-api is unreachable the last
//! good response is served, however stale, and only a cold cache yields an
//! error.

use crate::models::app_setting::AppSettings;
use serde::Deserialize;
use std::env;
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};
use uuid::Uuid;

/// How long a fetched settings list is reused before login-api is asked again.
const CACHE_TTL: Duration = Duration::from_secs(60);

/// How long to wait for login-api before giving up and using the cache.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Deserialize)]
struct SettingsEnvelope {
    #[serde(default)]
    data: Option<Vec<AppSettings>>,
}

struct CacheEntry {
    settings: Vec<AppSettings>,
    fetched_at: Instant,
}

static CACHE: OnceLock<RwLock<Option<CacheEntry>>> = OnceLock::new();
static HTTP: OnceLock<reqwest::Client> = OnceLock::new();

fn cache() -> &'static RwLock<Option<CacheEntry>> {
    CACHE.get_or_init(|| RwLock::new(None))
}

fn http() -> &'static reqwest::Client {
    HTTP.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

/// Base URL of login-api, e.g. `http://127.0.0.1:3002`. Trailing slashes are
/// trimmed so the path can be appended directly.
pub fn login_api_base() -> String {
    let raw = env::var("LOGIN_API_BASE").unwrap_or_else(|_| "http://127.0.0.1:3002".to_string());
    raw.trim().trim_end_matches('/').to_string()
}

/// The server-owned category wiring, resolved from the global settings rows.
/// Every field defaults to nil/empty, which is what the handlers see when
/// login-api is unreachable on a cold cache — the same behaviour as before,
/// when a failed local query simply skipped the category rewrite.
#[derive(Debug, Clone, Default)]
pub struct CategoryWiring {
    pub transfer_category_id: Uuid,
    pub transfer_category_name: String,
    pub recount_category_id: Uuid,
    pub recount_category_name: String,
    pub debt_category_id: Uuid,
    pub debt_category_name: String,
}

impl CategoryWiring {
    fn from_settings(settings: &[AppSettings]) -> Self {
        let mut wiring = CategoryWiring::default();
        for setting in settings {
            let value = setting.app_setting_value.as_str();
            match setting.app_setting_key.as_str() {
                "TRANSFER_CATEGORY_ID" => {
                    wiring.transfer_category_id = parse_uuid(value);
                }
                "TRANSFER_CATEGORY_NAME" => {
                    wiring.transfer_category_name = value.to_string();
                }
                "RECOUNT_CATEGORY_ID" => {
                    wiring.recount_category_id = parse_uuid(value);
                }
                "RECOUNT_CATEGORY_NAME" => {
                    wiring.recount_category_name = value.to_string();
                }
                "DEBT_CATEGORY_ID" => {
                    wiring.debt_category_id = parse_uuid(value);
                }
                "DEBT_CATEGORY_NAME" => {
                    wiring.debt_category_name = value.to_string();
                }
                _ => {}
            }
        }
        wiring
    }

    /// Display name configured for `category_id` when it is one of the wired
    /// categories, else `None`. A `None` means the caller must fall back to
    /// validating the category against its own table.
    pub fn resolve_name(&self, category_id: Uuid) -> Option<String> {
        if category_id == Uuid::nil() {
            return None;
        }
        if category_id == self.transfer_category_id {
            return Some(self.transfer_category_name.clone());
        }
        if category_id == self.recount_category_id {
            return Some(self.recount_category_name.clone());
        }
        if category_id == self.debt_category_id {
            return Some(self.debt_category_name.clone());
        }
        None
    }
}

fn parse_uuid(value: &str) -> Uuid {
    Uuid::parse_str(value).unwrap_or_else(|_| Uuid::nil())
}

fn cached_settings(allow_stale: bool) -> Option<Vec<AppSettings>> {
    let guard = cache().read().ok()?;
    let entry = guard.as_ref()?;
    if allow_stale || entry.fetched_at.elapsed() < CACHE_TTL {
        Some(entry.settings.clone())
    } else {
        None
    }
}

fn store_settings(settings: &[AppSettings]) {
    if let Ok(mut guard) = cache().write() {
        *guard = Some(CacheEntry {
            settings: settings.to_vec(),
            fetched_at: Instant::now(),
        });
    }
}

/// One uncached `GET {LOGIN_API_BASE}/api/settings`.
async fn fetch_from_login_api(url: &str) -> Result<Vec<AppSettings>, String> {
    let response = http()
        .get(url)
        .send()
        .await
        .map_err(|err| format!("GET {url} failed: {err}"))?;
    if !response.status().is_success() {
        return Err(format!("GET {url} returned HTTP {}", response.status()));
    }
    let envelope: SettingsEnvelope = response
        .json()
        .await
        .map_err(|err| format!("GET {url} returned an unreadable body: {err}"))?;
    Ok(envelope.data.unwrap_or_default())
}

/// Fetches the global settings rows from login-api, honouring the cache.
pub async fn fetch_global_settings() -> Result<Vec<AppSettings>, String> {
    if let Some(settings) = cached_settings(false) {
        return Ok(settings);
    }

    let url = format!("{}/api/settings", login_api_base());
    match fetch_from_login_api(&url).await {
        Ok(settings) => {
            store_settings(&settings);
            Ok(settings)
        }
        Err(err) => match cached_settings(true) {
            Some(stale) => {
                tracing::warn!("{}; serving stale app settings from cache", err);
                Ok(stale)
            }
            None => Err(err),
        },
    }
}

/// Category wiring for the current request. Never fails: an unreachable
/// login-api with a cold cache yields the default (all-nil) wiring, so a
/// spending/earning write is validated against the local category table only.
pub async fn global_category_wiring() -> CategoryWiring {
    match fetch_global_settings().await {
        Ok(settings) => CategoryWiring::from_settings(&settings),
        Err(err) => {
            tracing::warn!("Falling back to empty category wiring: {}", err);
            CategoryWiring::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setting(key: &str, value: &str) -> AppSettings {
        AppSettings {
            app_setting_id: Uuid::nil(),
            app_setting_key: key.to_string(),
            app_setting_value: value.to_string(),
            created_by: String::new(),
            is_active: 1,
        }
    }

    #[test]
    fn resolves_wired_category_names() {
        let transfer = Uuid::new_v4();
        let debt = Uuid::new_v4();
        let wiring = CategoryWiring::from_settings(&[
            setting("TRANSFER_CATEGORY_ID", &transfer.to_string()),
            setting("TRANSFER_CATEGORY_NAME", "Transfer"),
            setting("DEBT_CATEGORY_ID", &debt.to_string()),
            setting("DEBT_CATEGORY_NAME", "Debt"),
            setting("FEATURE_HEALTH", "false"),
        ]);

        assert_eq!(wiring.resolve_name(transfer).as_deref(), Some("Transfer"));
        assert_eq!(wiring.resolve_name(debt).as_deref(), Some("Debt"));
        assert_eq!(wiring.resolve_name(Uuid::new_v4()), None);
    }

    #[test]
    fn nil_category_never_matches_unset_wiring() {
        let wiring = CategoryWiring::default();
        assert_eq!(wiring.resolve_name(Uuid::nil()), None);
    }

    #[test]
    fn trims_trailing_slashes_from_base_url() {
        unsafe { env::set_var("LOGIN_API_BASE", "http://localhost:3002/") };
        assert_eq!(login_api_base(), "http://localhost:3002");
        unsafe { env::remove_var("LOGIN_API_BASE") };
    }
}
