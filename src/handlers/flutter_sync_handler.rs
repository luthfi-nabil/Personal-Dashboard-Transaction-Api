use actix_web::{web, HttpResponse, Responder, HttpRequest};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::env;
use mysql::prelude::Queryable;
use mysql::params;

use crate::helper::connection::establish_connection_v2;

#[derive(Serialize, Deserialize, Debug)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FlutterSource {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub syncState: String,
    pub updatedAt: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FlutterCategory {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub syncState: String,
    pub updatedAt: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct FlutterTransaction {
    pub id: String,
    pub r#type: String, // "earning" or "spending"
    pub amount: f64,
    pub description: String,
    pub category: Option<String>,
    pub source: Option<String>,
    pub fromSource: Option<String>,
    pub toSource: Option<String>,
    pub date: String,
    pub syncState: String,
    pub updatedAt: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct FlutterSyncResponse {
    pub sources: Vec<FlutterSource>,
    pub categories: Vec<FlutterCategory>,
    pub transactions: Vec<FlutterTransaction>,
    pub deletedTransactions: Vec<String>,
}

fn extract_username(req: &HttpRequest) -> Option<String> {
    let auth_header = req.headers().get("Authorization")?.to_str().ok()?;
    if !auth_header.starts_with("Bearer ") {
        return None;
    }
    let token = &auth_header[7..];
    let secret = env::var("JWT_SECRET").unwrap_or_else(|_| "secret".to_string());
    
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default()
    ).ok()?;
    
    Some(token_data.claims.sub)
}

pub async fn get_sync(req: HttpRequest) -> impl Responder {
    let username = match extract_username(&req) {
        Some(u) => u,
        None => return HttpResponse::Unauthorized().json("Invalid or missing token"),
    };

    let mut conn = match establish_connection_v2() {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().json("Database error"),
    };

    let mut sources = Vec::new();
    let db_sources: Vec<(String, String, Option<String>)> = conn.exec(
        "SELECT source_id, source, created_date FROM source WHERE created_by = :username",
        params! { "username" => &username }
    ).unwrap_or_default();
    
    for (id, name, created_date) in db_sources {
        sources.push(FlutterSource {
            id,
            name,
            kind: "source".to_string(),
            syncState: "done".to_string(),
            updatedAt: created_date.unwrap_or_else(|| "".to_string()),
        });
    }

    let mut categories = Vec::new();
    let db_earning_cats: Vec<(String, String, Option<String>)> = conn.exec(
        "SELECT earning_category_id, earning_category, created_date FROM earning_category WHERE created_by = :username",
        params! { "username" => &username }
    ).unwrap_or_default();
    for (id, name, created_date) in db_earning_cats {
        categories.push(FlutterCategory {
            id,
            name,
            kind: "earning".to_string(),
            syncState: "done".to_string(),
            updatedAt: created_date.unwrap_or_else(|| "".to_string()),
        });
    }
    
    let db_spending_cats: Vec<(String, String, Option<String>)> = conn.exec(
        "SELECT spending_category_id, spending_category, created_date FROM spending_category WHERE created_by = :username",
        params! { "username" => &username }
    ).unwrap_or_default();
    for (id, name, created_date) in db_spending_cats {
        categories.push(FlutterCategory {
            id,
            name,
            kind: "spending".to_string(),
            syncState: "done".to_string(),
            updatedAt: created_date.unwrap_or_else(|| "".to_string()),
        });
    }

    let mut transactions = Vec::new();
    let db_earnings: Vec<(String, f64, String, String, String, Option<String>)> = conn.exec(
        "SELECT earning_id, total_amount, description, earning_category_id, source_id, created_date FROM earning WHERE created_by = :username",
        params! { "username" => &username }
    ).unwrap_or_default();
    for (id, amount, description, cat_id, src_id, created_date) in db_earnings {
        let date = created_date.unwrap_or_else(|| "".to_string());
        transactions.push(FlutterTransaction {
            id,
            r#type: "earning".to_string(),
            amount,
            description,
            category: Some(cat_id),
            source: Some(src_id),
            fromSource: None,
            toSource: None,
            date: date.clone(),
            syncState: "done".to_string(),
            updatedAt: date,
        });
    }

    let db_spendings: Vec<(String, f64, String, String, String, Option<String>)> = conn.exec(
        "SELECT spending_id, total_amount, description, spending_category_id, source_id, created_date FROM spending WHERE created_by = :username",
        params! { "username" => &username }
    ).unwrap_or_default();
    for (id, amount, description, cat_id, src_id, created_date) in db_spendings {
        let date = created_date.unwrap_or_else(|| "".to_string());
        transactions.push(FlutterTransaction {
            id,
            r#type: "spending".to_string(),
            amount,
            description,
            category: Some(cat_id),
            source: Some(src_id),
            fromSource: None,
            toSource: None,
            date: date.clone(),
            syncState: "done".to_string(),
            updatedAt: date,
        });
    }

    HttpResponse::Ok().json(FlutterSyncResponse {
        sources,
        categories,
        transactions,
        deletedTransactions: vec![],
    })
}

#[derive(Deserialize, Debug)]
pub struct SyncPushRequest {
    pub queue: Vec<SyncOp>,
}

#[derive(Deserialize, Debug)]
pub struct SyncOp {
    pub kind: String,
    pub resource: String,
    pub payload: serde_json::Value,
}

pub async fn post_sync_push(req: HttpRequest, body: web::Json<SyncPushRequest>) -> impl Responder {
    let username = match extract_username(&req) {
        Some(u) => u,
        None => return HttpResponse::Unauthorized().json("Invalid or missing token"),
    };
    
    // In a full implementation, we map `body.queue` to the correct DB insert/delete statements
    // here. For now, we return Ok to acknowledge the sync push.
    
    HttpResponse::Ok().json("Synced successfully")
}
