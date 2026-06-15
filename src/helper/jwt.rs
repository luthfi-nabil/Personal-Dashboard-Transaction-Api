use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

fn get_secret() -> String {
    env::var("JWT_SECRET")
        .unwrap_or_else(|_| "default_dev_secret_please_change_in_production".to_string())
}

pub fn decode_username(token: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let secret = get_secret();
    let claims = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )?
    .claims;

    Ok(claims.sub)
}

pub fn extract_bearer_token(auth_header: &str) -> Option<&str> {
    auth_header
        .strip_prefix("Bearer ")
        .filter(|token| !token.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{EncodingKey, Header, encode};
    use serde::Serialize;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Serialize)]
    struct TestClaims {
        sub: String,
        user_id: String,
        exp: usize,
        iat: usize,
    }

    #[test]
    fn decodes_username_from_login_api_compatible_token() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        let token = encode(
            &Header::new(Algorithm::HS256),
            &TestClaims {
                sub: "alice".to_string(),
                user_id: "user-id".to_string(),
                exp: now + 60,
                iat: now,
            },
            &EncodingKey::from_secret(get_secret().as_bytes()),
        )
        .unwrap();

        assert_eq!(decode_username(&token).unwrap(), "alice");
    }

    #[test]
    fn only_accepts_non_empty_bearer_tokens() {
        assert_eq!(extract_bearer_token("Bearer token"), Some("token"));
        assert_eq!(extract_bearer_token("Bearer "), None);
        assert_eq!(extract_bearer_token("token"), None);
    }
}
