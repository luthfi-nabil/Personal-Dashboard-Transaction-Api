use actix_web::{web, HttpResponse};
use chrono::Utc;
use uuid::Uuid;

use crate::models::earning::{Earning, EarningCategory};
use crate::models::responses::{Response};
use crate::repository::earning_repository::{
    select_earnings, 
    select_all_earning_categories,
    select_earning_category, 
    insert_earning, 
    insert_earning_category, 
    delete_earning, 
    delete_earning_category
};
use crate::repository::source_repository::{select_source};
use crate::helper::connection::{establish_connection};

pub async fn get_all_earnings_api() -> HttpResponse {
    let connection = establish_connection().expect("Failed to connect to database");
    match select_earnings(&connection) {
        Ok(earnings) => {
            let response = Response {
                status: "Success".to_string(),
                code: crate::helper::response_code::RESPONSE_CODE_DATA_RETRIEVAL_SUCCESS,
                message: "Success get earning categories".to_string(),
                description: "".to_string(),
                data: Some(serde_json::to_value(earnings).unwrap()),
            };
            HttpResponse::Ok().json(response)
        },
        Err(err) => {
            let response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_RETRIEVAL_FAILED,
                message: "Failed to retrieve earning categories".to_string(),
                description: err.sqlite_error().map(|e| format!("{:?}", e)) // or e.to_string() if available
                .unwrap_or_else(|| err.to_string()),
                data: None,
            };
            HttpResponse::InternalServerError().json(response)
        },
    }
}

pub async fn get_all_earning_categories_api() -> HttpResponse {
    let connection = establish_connection().expect("Failed to connect to database");
    match select_all_earning_categories(&connection) {
        Ok(categories) => {
            let response = Response {
                status: "Success".to_string(),
                code: crate::helper::response_code::RESPONSE_CODE_DATA_RETRIEVAL_SUCCESS,
                message: "Success get earning categories".to_string(),
                description: "".to_string(),
                data: Some(serde_json::to_value(categories).unwrap()),
            };
            HttpResponse::Ok().json(response)
        }
        Err(err) => {
            let response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_RETRIEVAL_FAILED,
                message: "Failed to retrieve earning categories".to_string(),
                description: err.sqlite_error().map(|e| format!("{:?}", e)) // or e.to_string() if available
                .unwrap_or_else(|| err.to_string()),
                data: None,
            };
            HttpResponse::InternalServerError().json(response)
        },
    }
}

pub async fn post_earning_api(earning: web::Json<Earning>) -> HttpResponse {
    let connection = establish_connection().expect("Failed to connect to database");
    let new_earning = Earning {
        earning_id: Uuid::new_v4(),
        total_amount: earning.total_amount,
        description: earning.description.clone(),
        earning_category_id: earning.earning_category_id,
        earning_category: earning.earning_category.clone(),
        source_id: earning.source_id,
        source: earning.source.clone(),
        created_date: Utc::now(),
        created_by: earning.created_by.clone(),
        is_active: 1
    };
    let mut response = Response {
        status: "Success".to_string(),
        message: "Earning created successfully".to_string(),
        code: crate::helper::response_code::RESPONSE_CODE_DATA_INSERTION_SUCCESS,
        description: "".to_string(),
        data: None,
    };
    let _check_source = select_source(&connection, &new_earning.source_id.to_string());
    let _check_category = select_earning_category(&connection, &new_earning.earning_category_id.to_string());
    if _check_source.is_ok() && _check_category.is_ok() && _check_category.as_ref().unwrap().len() > 0 && _check_source.as_ref().unwrap().len() > 0 {
        let _result  = insert_earning(&connection, &new_earning);
        
        if _result.is_err() {
            response = Response {
                status: "Error".to_string(),
                message: "Failed to create earning ".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                description: _result.err().unwrap().to_string(),
                data: None,
            };
        }else{
            response.data = Some(serde_json::to_value(new_earning).unwrap());
        }
    }else{
        if _check_category.is_err() {
            response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                message: "Failed to create earning [Category]".to_string(),
                description: _check_category.err().unwrap().to_string(),
                data: None,
            };
        }else if _check_category.as_ref().unwrap().len() == 0 {
            response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                message: "Earning category not found".to_string(),
                description: "Please create the earning category first.".to_string(),
                data: None,
            };
        }else if _check_source.is_err() {
            response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                message: "Failed to create earning [Source]".to_string(),
                description: _check_source.err().unwrap().to_string(),
                data: None,
            };
        }else if _check_source.as_ref().unwrap().len() == 0 {
            response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                message: "Source not found".to_string(),
                description: "Please create the source first.".to_string(),
                data: None,
            };
        }
    }
    if response.code == crate::helper::response_code::RESPONSE_CODE_DATA_INSERTION_SUCCESS {
        HttpResponse::Created().json(response)
    }else{
        HttpResponse::BadRequest().json(response)
    }
    
}

pub async fn post_earning_category_api(category: web::Json<EarningCategory>) -> HttpResponse {
    let connection = establish_connection().expect("Failed to connect to database");
    let new_category = EarningCategory {
        earning_category_id: Uuid::new_v4(),
        earning_category: category.earning_category.clone(),
        created_date: Utc::now(),
        created_by: category.created_by.clone(),
        is_active: 1
    };
    
    let _result  = insert_earning_category(&connection, &new_category);

    let mut response = Response {
        status: "Success".to_string(),
        code: crate::helper::response_code::RESPONSE_CODE_DATA_INSERTION_SUCCESS,
        message: "Earning category created successfully".to_string(),
        description: "".to_string(),
        data: None,
    };
    match _result {
        Ok(_) => {
            let response = Response {
                status: "Success".to_string(),
                code: crate::helper::response_code::RESPONSE_CODE_DATA_INSERTION_SUCCESS,
                message: "Spending category inserted successfully".to_string(),
                description: "".to_string(),
                data: Some(serde_json::to_value(new_category).unwrap()),
            };
            HttpResponse::Ok().json(response)
        },
        Err(err) => {
            let mut response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                message: "Failed to insert earning category".to_string(),
                description: err.sqlite_error().map(|e| format!("{:?}", e)) // or e.to_string() if available
                .unwrap_or_else(|| err.to_string()),
                data: None,
            };
            if err.sqlite_error().map_or(false, |e| e.code == rusqlite::ErrorCode::ConstraintViolation) {
                response.description = "Earning category already exists".to_string();
                return HttpResponse::BadRequest().json(response);
            }else{
                return HttpResponse::InternalServerError().json(response)
            }
        }
    }
    
}

pub async fn delete_earning_api(earning_id: web::Path<String>) -> HttpResponse {
    let connection = establish_connection().expect("Failed to connect to database");
    let earning_id_str = earning_id.into_inner();
    let result = delete_earning(&connection, &earning_id_str);

    match result {
        Ok(_) => {
            let response = Response {
                status: "Success".to_string(),
                code: crate::helper::response_code::RESPONSE_CODE_DATA_RETRIEVAL_SUCCESS,
                message: "Success delete earning".to_string(),
                description: "".to_string(),
                data: None,
            };
            HttpResponse::Ok().json(response)
        },
        Err(err) => {
            let response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_RETRIEVAL_FAILED,
                message: "Failed to delete earning".to_string(),
                description: err.sqlite_error().map(|e| format!("{:?}", e)) // or e.to_string() if available
                .unwrap_or_else(|| err.to_string()),
                data: None,
            };
            HttpResponse::InternalServerError().json(response)
        },
    }
}

pub async fn delete_earning_category_api(category_id: web::Path<String>) -> HttpResponse {
    let connection = establish_connection().expect("Failed to connect to database");
    let category_id_str = category_id.into_inner();
    let result = delete_earning_category(&connection, &category_id_str);

    match result {
        Ok(_) => {
            let response = Response {
                status: "Success".to_string(),
                code: crate::helper::response_code::RESPONSE_CODE_DATA_RETRIEVAL_SUCCESS,
                message: "Success delete earning category".to_string(),
                description: "".to_string(),
                data: None,
            };
            HttpResponse::Ok().json(response)
        },
        Err(err) => {
            let response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_RETRIEVAL_FAILED,
                message: "Failed to delete earning category".to_string(),
                description: err.sqlite_error().map(|e| format!("{:?}", e)) // or e.to_string() if available
                .unwrap_or_else(|| err.to_string()),
                data: None,
            };
            HttpResponse::InternalServerError().json(response)
        },
    }
}

pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json("API is up and running!")
}