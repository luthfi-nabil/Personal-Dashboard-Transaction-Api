use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub status: String,
    pub code: u16,
    pub message: String,
    pub description: String,
    pub data: Option<serde_json::Value>,
    pub success: bool,
}

#[derive(Debug)]
pub enum DatabaseResult {
    Inserted,
    Duplicate,
}

#[derive(Debug)]
pub enum AppError {
    Unauthorized,

    Forbidden,

    NotFound,

    InternalError,
}
