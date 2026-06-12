use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub user_id: String,
    pub username: String,
    pub password_hash: String,
    pub email: Option<String>,
    pub phone_number: Option<String>,
    pub telegram_username: Option<String>,
    pub created_date: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
    pub phone_number: Option<String>,
    pub telegram_username: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthResponse {
    pub token: String,
    pub username: String,
    pub email: Option<String>,
    pub phone_number: Option<String>,
    pub telegram_username: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Claims {
    pub sub: String, // username
    pub exp: usize,
}
