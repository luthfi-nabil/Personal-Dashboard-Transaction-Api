use actix_web::{web, HttpResponse};
use chrono::Utc;
use uuid::Uuid;

use crate::models::source::{Source};
use crate::models::responses::{Response};
use crate::repository::source_repository::{select_source, select_all_sources, insert_source};
use crate::helper::connection::{establish_connection};

pub async fn get_all_sources_api() -> HttpResponse {
    let conn = establish_connection().expect("Failed to connect to database");
    let _result = select_all_sources(&conn);

    match _result {
        Ok(sources) => {
            let response = Response {
                status: "Success".to_string(),
                code: crate::helper::response_code::RESPONSE_CODE_DATA_RETRIEVAL_SUCCESS,
                message: "Success get sources".to_string(),
                description: "".to_string(),
                data: Some(serde_json::to_value(sources).unwrap()),
            };
            HttpResponse::Ok().json(response)
        },
        Err(err) => {
            let response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_RETRIEVAL_FAILED,
                message: "Failed to retrieve sources".to_string(),
                description: err.sqlite_error().map(|e| format!("{:?}", e)) // or e.to_string() if available
                .unwrap_or_else(|| err.to_string()),
                data: None,
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
}

pub async fn post_source_api(source: web::Json<Source>) -> HttpResponse {
    let conn = establish_connection().expect("Failed to connect to database");

    let new_source = Source {
        source_id: Uuid::new_v4(),
        source: source.source.clone(),
        created_date: Utc::now(),
        created_by: source.created_by.clone(),
    };

    let _result = insert_source(&conn, &new_source);

    match _result {
        Ok(_) => {
            let response = Response {
                status: "Success".to_string(),
                code: crate::helper::response_code::RESPONSE_CODE_DATA_INSERTION_SUCCESS,
                message: "Source inserted successfully".to_string(),
                description: "".to_string(),
                data: Some(serde_json::to_value(new_source).unwrap()),
            };
            HttpResponse::Ok().json(response)
        },
        Err(err) => {
            let response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                message: "Failed to insert source".to_string(),
                description: err.sqlite_error().map(|e| format!("{:?}", e)) // or e.to_string() if available
                .unwrap_or_else(|| err.to_string()),
                data: None,
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
}