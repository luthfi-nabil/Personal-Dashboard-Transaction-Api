use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc, Local};
use uuid::Uuid;
use crate::models::source::{SourceV2};
use crate::models::earning::{EarningV2, EarningParam, EarningCategoryV2};
use crate::models::responses::{Response};
use crate::helper::connection::{establish_connection_v2};
use crate::repository::source_repository_v2::{select_source};
use crate::repository::earning_repository_v2::{select_all_earning_categories, insert_earning, select_earnings, select_earning_category, delete_earning_category, insert_earning_category};


pub async fn get_all_earnings_api_v2(query: web::Query<EarningParam>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let _result = select_earnings(&mut conn, &query);

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
                description: err.to_string(),
                data: None,
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
}

pub async fn get_all_earning_categories_api_v2() -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    match select_all_earning_categories(&mut conn) {
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
                description: err.to_string(),
                data: None,
            };
            HttpResponse::InternalServerError().json(response)
        },
    }
}

pub async fn post_earning_api_v2(earning: web::Json<EarningV2>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let new_earning = EarningV2 {
        earning_id: Uuid::new_v4(),
        total_amount: earning.total_amount,
        description: earning.description.clone(),
        earning_category_id: earning.earning_category_id,
        earning_category: earning.earning_category.clone(),
        source_id: earning.source_id,
        source: earning.source.clone(),
        created_date: Local::now().naive_local(),
        created_by: earning.created_by.clone(),
        is_active: 1
    };
    
    let _check_source = select_source(&mut conn, &new_earning.source_id.to_string());
    let _check_category = select_earning_category(&mut conn, &new_earning.earning_category_id.to_string());

    let mut response = Response {
        status: "Success".to_string(),
        code: crate::helper::response_code::RESPONSE_CODE_DATA_INSERTION_SUCCESS,
        message: "Earning category created successfully".to_string(),
        description: "".to_string(),
        data: None,
    };
    if _check_source.is_ok() && _check_category.is_ok() && _check_category.as_ref().unwrap().len() > 0 && _check_source.as_ref().unwrap().len() > 0 {
        let _result  = insert_earning(&mut conn, &new_earning);
        
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

pub async fn post_earning_category_api_v2(category: web::Json<EarningCategoryV2>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let new_category = EarningCategoryV2 {
        earning_category_id: Uuid::new_v4(),
        earning_category: category.earning_category.clone(),
        created_date: Local::now().naive_local(),
        created_by: category.created_by.clone(),
        is_active: 1
    };
    
    let _result  = insert_earning_category(&mut conn, &new_category);

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
                description: err.to_string(),
                data: None,
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
    
}

pub async fn delete_earning_category_api_v2(path: web::Path<String>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let earning_category = path.into_inner();
    let result = delete_earning_category(&mut conn, &earning_category);

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
                description: err.to_string(),
                data: None,
            };
            HttpResponse::InternalServerError().json(response)
        },
    }
}