use actix_web::{web, HttpResponse, HttpRequest, HttpMessage};
use chrono::{DateTime, Utc, Local};
use uuid::Uuid;
use crate::models::app_setting;
use crate::models::source::{SourceV2};
use crate::models::earning::{EarningV2, EarningParam, EarningCategoryV2};
use crate::models::responses::{Response, DatabaseResult};
use crate::helper::connection::{establish_connection_v2};
use crate::repository::source_repository_v2::{select_source};
use crate::repository::earning_repository_v2::{select_all_earning_categories, insert_earning, select_earnings, select_earning_category, delete_earning_category, insert_earning_category};
use crate::repository::app_setting_repository::{select_all_settings};
use crate::route_middleware::get_user::CreatedBy;

pub async fn get_all_setting_api() -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let app_setting: app_setting::AppSettings = app_setting::AppSettings {
        app_setting_id: Uuid::nil(),
        app_setting_key: "".to_string(),
        app_setting_value: "".to_string(),
        is_active: 1
    };
    let _result = select_all_settings(&mut conn, &app_setting);

    match _result {
        Ok(sources) => {
            let response = Response {
                status: "Success".to_string(),
                code: crate::helper::response_code::RESPONSE_CODE_DATA_RETRIEVAL_SUCCESS,
                message: "Success get settings".to_string(),
                description: "".to_string(),
                data: Some(serde_json::to_value(sources).unwrap()),
                success: true
            };
            HttpResponse::Ok().json(response)
        },
        Err(err) => {
            let response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_RETRIEVAL_FAILED,
                message: "Failed to retrieve settings".to_string(),
                description: err.to_string(),
                data: None,
                success: false
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
}