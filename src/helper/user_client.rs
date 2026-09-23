//! Account lookup against login-api, which owns the `users` table.
//!
//! Used before adding someone to a spending group so a typo cannot create a
//! member nobody can ever log in as. The caller's bearer token is forwarded,
//! since login-api's lookup route sits behind its JWT middleware.

use serde::Deserialize;
use std::sync::OnceLock;
use std::time::Duration;

use crate::helper::settings_client::login_api_base;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

static HTTP: OnceLock<reqwest::Client> = OnceLock::new();

fn http() -> &'static reqwest::Client {
    HTTP.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

#[derive(Debug, Deserialize)]
struct UserData {
    username: String,
}

#[derive(Debug, Deserialize)]
struct UserEnvelope {
    #[serde(default)]
    data: Option<UserData>,
}

#[derive(Debug)]
pub enum UserLookup {
    /// The account exists; holds the username exactly as login-api stores it.
    Found(String),
    NotFound,
    /// login-api could not be asked. The caller should not guess either way.
    Unavailable(String),
}

/// Looks `username` up in login-api on behalf of the request carrying
/// `authorization` (the raw `Authorization` header value).
pub async fn lookup_user(username: &str, authorization: &str) -> UserLookup {
    let url = format!(
        "{}/api/user/users/{}",
        login_api_base(),
        urlencoding_path(username)
    );
    let response = match http()
        .get(&url)
        .header("Authorization", authorization)
        .send()
        .await
    {
        Ok(r) => r,
        Err(err) => return UserLookup::Unavailable(format!("GET {url} failed: {err}")),
    };

    let status = response.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        return UserLookup::NotFound;
    }
    if !status.is_success() {
        return UserLookup::Unavailable(format!("GET {url} returned HTTP {status}"));
    }
    match response.json::<UserEnvelope>().await {
        Ok(UserEnvelope {
            data: Some(UserData { username }),
        }) => UserLookup::Found(username),
        Ok(_) => UserLookup::NotFound,
        Err(err) => UserLookup::Unavailable(format!("GET {url} returned an unreadable body: {err}")),
    }
}

/// Percent-encodes everything outside the unreserved set so a username can
/// be dropped into a path segment safely.
fn urlencoding_path(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{:02X}", byte)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_reserved_characters_in_usernames() {
        assert_eq!(urlencoding_path("alice"), "alice");
        assert_eq!(urlencoding_path("a b/c"), "a%20b%2Fc");
    }
}
