use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub status: String,
    pub code: i32,
    pub message: String,
    pub description: String,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug)]
pub enum DatabaseResult {
    Inserted,
    Duplicate,
}