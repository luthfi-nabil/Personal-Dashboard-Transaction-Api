use actix_web::{web, HttpResponse, HttpRequest, HttpMessage};
use chrono::{Local};
use uuid::Uuid;
use crate::models::source::{SourceBalance, SourceV2};
use crate::models::responses::{Response, DatabaseResult};
use crate::helper::connection::{establish_connection_v2};
use crate::repository::source_repository_v2::{select_all_sources, insert_source, delete_source};
use crate::repository::spending_repository_v2::{select_spendings};
use crate::repository::earning_repository_v2::{select_earnings};
use crate::route_middleware::get_user::CreatedBy;

pub async fn get_all_sources_api_v2(req: HttpRequest) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let source_filter = SourceV2 {
        source_id: Uuid::nil(),
        source: "".to_string(),
        created_date: Local::now().naive_local(),
        created_by: created_by,
        is_active: 1
    };
    let _result = select_all_sources(&mut conn, &source_filter);

    match _result {
        Ok(sources) => {
            let response = Response {
                status: "Success".to_string(),
                code: crate::helper::response_code::RESPONSE_CODE_DATA_RETRIEVAL_SUCCESS,
                message: "Success get sources".to_string(),
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
                message: "Failed to retrieve sources".to_string(),
                description: err.to_string(),
                data: None,
                success: false
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
}

pub async fn get_all_source_balance(req: HttpRequest) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let source_filter = SourceV2 {
        source_id: Uuid::nil(),
        source: "".to_string(),
        created_date: Local::now().naive_local(),
        created_by: created_by.clone(),
        is_active: 1
    };
    let _spending_result = select_spendings(&mut conn, &crate::models::spending::SpendingParam {
        description: None,
        spending_category_id: None,
        spending_category: None,
        source_id: None,
        source: None,
        month: None,
        year: None,
        day: None
    }, Some(created_by.clone()));
    let _earning_result = select_earnings(&mut conn, &crate::models::earning::EarningParam {
        description: None,
        earning_category_id: None,
        earning_category: None,
        source_id: None,
        source: None,
        month: None,
        year: None,
        day: None
    }, Some(created_by.clone()));
    let _source_result = select_all_sources(&mut conn, &source_filter);
    let mut _source_balance: Vec<SourceBalance> = Vec::new();
    match &_source_result {
        Ok(vec) => {
            for item in vec.iter() {
                let _earning_item = _earning_result.as_ref().unwrap().iter().filter(|e| e.created_by == created_by.clone() && e.source_id == item.source_id);
                let _spending_item = _spending_result.as_ref().unwrap().iter().filter(|s| s.created_by == created_by.clone() && s.source_id == item.source_id);
                let total_earning: f64 = _earning_item.clone().map(|e| e.total_amount).sum();
                let total_spending: f64 = _spending_item.clone().map(|s| s.total_amount).sum();
                let source_balance = SourceBalance {
                    source_id: item.source_id,
                    source: item.source.clone(),
                    total: total_earning - total_spending
                };
                _source_balance.push(source_balance);
            }
        }
        Err(err) => {
            eprintln!("query failed: {}", err);
        }
    }
    let response = Response {
        status: "Success".to_string(),
        code: crate::helper::response_code::RESPONSE_CODE_DATA_RETRIEVAL_SUCCESS,
        message: "Success get sources balance".to_string(),
        description: "".to_string(),
        data: Some(serde_json::to_value(_source_balance).unwrap()),
        success: true
    };
    HttpResponse::Ok().json(response)
}

pub async fn post_source_api_v2(req: HttpRequest, source: web::Json<SourceV2>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    
    let new_source = SourceV2 {
        source_id: Uuid::new_v4(),
        source: source.source.clone(),
        created_date: Local::now().naive_local(),
        created_by: created_by.clone(),
        is_active: 1
    };

    let _result = insert_source(&mut conn, &new_source);

    match _result {
        Ok(e) => match e{
            DatabaseResult::Inserted => {
                let response = Response {
                    status: "Success".to_string(),
                    code: crate::helper::response_code::RESPONSE_CODE_DATA_INSERTION_SUCCESS,
                    message: "Source inserted successfully".to_string(),
                    description: "".to_string(),
                    data: Some(serde_json::to_value(new_source).unwrap()),
                    success: true
                };
                HttpResponse::Ok().json(response)
            },
            DatabaseResult::Duplicate => {
                let response = Response {
                    status: "Error".to_string(),
                    code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                    message: "Failed to insert source".to_string(),
                    description: "Source already exists".to_string(),
                    data: None,
                    success: false
                };
                HttpResponse::BadRequest().json(response)
            }
            
        },
        Err(err) => {
            let response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                message: "Failed to insert source".to_string(),
                description: err.to_string(),
                data: None,
                success: false
            };
            HttpResponse::InternalServerError().json(response)
            
        }
    }
}

pub async fn delete_source_api_v2(req: HttpRequest,path: web::Path<String>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let source_name = path.into_inner();
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let source = SourceV2 {
        source_id: Uuid::parse_str(&source_name).unwrap_or_else(|_| Uuid::nil()),
        source: "".to_string(),
        created_date: Local::now().naive_local(),
        created_by: created_by.clone(),
        is_active: 1
    };
    let _result = delete_source(&mut conn, &source);

    match _result {
        Ok(_) => {
            let response = Response {
                status: "Success".to_string(),
                code: crate::helper::response_code::RESPONSE_CODE_DATA_DELETION_SUCCESS,
                message: "Source deleted successfully".to_string(),
                description: "".to_string(),
                data: None,
                success: true
            };
            HttpResponse::Ok().json(response)
        },
        Err(err) => {
            let response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_DELETION_FAILED,
                message: "Failed to delete source".to_string(),
                description: err.to_string(),
                data: None,
                success: false
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
}