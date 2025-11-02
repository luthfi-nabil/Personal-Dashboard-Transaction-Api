use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc, Local};
use uuid::Uuid;
use crate::models::source::{SourceV2};
use crate::models::spending::{SpendingV2, SpendingParam, SpendingCategoryV2};
use crate::models::responses::{Response, DatabaseResult};
use crate::helper::connection::{establish_connection_v2};
use crate::repository::source_repository_v2::{select_source};
use crate::repository::spending_repository_v2::{select_all_spending_categories, insert_spending, select_spendings, select_spending_category, delete_spending_category, insert_spending_category};


pub async fn get_all_spendings_api_v2(query: web::Query<SpendingParam>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let _result = select_spendings(&mut conn, &query);

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

pub async fn get_all_spending_categories_api_v2() -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    match select_all_spending_categories(&mut conn) {
        Ok(categories) => {
            let response = Response {
                status: "Success".to_string(),
                code: crate::helper::response_code::RESPONSE_CODE_DATA_RETRIEVAL_SUCCESS,
                message: "Success get spending categories".to_string(),
                description: "".to_string(),
                data: Some(serde_json::to_value(categories).unwrap()),
            };
            HttpResponse::Ok().json(response)
        }
        Err(err) => {
            let response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_RETRIEVAL_FAILED,
                message: "Failed to retrieve spending categories".to_string(),
                description: err.to_string(),
                data: None,
            };
            HttpResponse::InternalServerError().json(response)
        },
    }
}

pub async fn post_spending_api_v2(spending: web::Json<SpendingV2>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let new_spending = SpendingV2 {
        spending_id: Uuid::new_v4(),
        total_amount: spending.total_amount,
        description: spending.description.clone(),
        spending_category_id: spending.spending_category_id,
        spending_category: spending.spending_category.clone(),
        source_id: spending.source_id,
        source: spending.source.clone(),
        created_date: Local::now().naive_local(),
        created_by: spending.created_by.clone(),
        is_active: 1
    };
    
    let _check_source = select_source(&mut conn, &new_spending.source_id.to_string());
    let _check_category = select_spending_category(&mut conn, &new_spending.spending_category_id.to_string());

    let mut response = Response {
        status: "Success".to_string(),
        code: crate::helper::response_code::RESPONSE_CODE_DATA_INSERTION_SUCCESS,
        message: "Spending category created successfully".to_string(),
        description: "".to_string(),
        data: None,
    };
    if _check_source.is_ok() && _check_category.is_ok() && _check_category.as_ref().unwrap().len() > 0 && _check_source.as_ref().unwrap().len() > 0 {
        let _result  = insert_spending(&mut conn, &new_spending);
        
        if _result.is_err() {
            response = Response {
                status: "Error".to_string(),
                message: "Failed to create spending ".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                description: _result.err().unwrap().to_string(),
                data: None,
            };
        }else{
            response.data = Some(serde_json::to_value(new_spending).unwrap());
        }
    }else{
        if _check_category.is_err() {
            response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                message: "Failed to create spending [Category]".to_string(),
                description: _check_category.err().unwrap().to_string(),
                data: None,
            };
        }else if _check_category.as_ref().unwrap().len() == 0 {
            response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                message: "Spending category not found".to_string(),
                description: "Please create the spending category first.".to_string(),
                data: None,
            };
        }else if _check_source.is_err() {
            response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                message: "Failed to create spending [Source]".to_string(),
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

pub async fn post_spending_category_api_v2(category: web::Json<SpendingCategoryV2>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let new_category = SpendingCategoryV2 {
        spending_category_id: Uuid::new_v4(),
        spending_category: category.spending_category.clone(),
        created_date: Local::now().naive_local(),
        created_by: category.created_by.clone(),
        is_active: 1
    };
    
    let _result  = insert_spending_category(&mut conn, &new_category);

    let mut response = Response {
        status: "Success".to_string(),
        code: crate::helper::response_code::RESPONSE_CODE_DATA_INSERTION_SUCCESS,
        message: "Spending category created successfully".to_string(),
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
                message: "Failed to insert spending category".to_string(),
                description: err.to_string(),
                data: None,
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
    
}

pub async fn delete_spending_category_api_v2(path: web::Path<String>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let spending_category = path.into_inner();
    let result = delete_spending_category(&mut conn, &spending_category);

    match result {
        Ok(_) => {
            let response = Response {
                status: "Success".to_string(),
                code: crate::helper::response_code::RESPONSE_CODE_DATA_RETRIEVAL_SUCCESS,
                message: "Success delete spending category".to_string(),
                description: "".to_string(),
                data: None,
            };
            HttpResponse::Ok().json(response)
        },
        Err(err) => {
            let response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_RETRIEVAL_FAILED,
                message: "Failed to delete spending category".to_string(),
                description: err.to_string(),
                data: None,
            };
            HttpResponse::InternalServerError().json(response)
        },
    }
}