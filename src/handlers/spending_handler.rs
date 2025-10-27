use actix_web::{web, HttpResponse};
use chrono::Utc;
use uuid::Uuid;

use crate::models::spending::{Spending, SpendingCategory};
use crate::models::responses::{Response};
use crate::repository::spending_repository::{
    select_spendings, 
    select_all_spending_categories,
    select_spending_category, 
    insert_spending, 
    insert_spending_category, 
    delete_spending, 
    delete_spending_category
};
use crate::repository::source_repository::{select_source};
use crate::helper::connection::{establish_connection};

pub async fn get_all_spendings_api() -> HttpResponse {
    let conn = establish_connection().expect("Failed to connect to database");
    let _result = select_spendings(&conn);

    match _result {
        Ok(spendings) => {
            let response = Response {
                status: "Success".to_string(),
                code: crate::helper::response_code::RESPONSE_CODE_DATA_RETRIEVAL_SUCCESS,
                message: "Success get spendings".to_string(),
                description: "".to_string(),
                data: Some(serde_json::to_value(spendings).unwrap()),
            };
            HttpResponse::Ok().json(response)
        },
        Err(err) => {
            let response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_RETRIEVAL_FAILED,
                message: "Failed to retrieve spendings".to_string(),
                description: err.sqlite_error().map(|e| format!("{:?}", e)) // or e.to_string() if available
                .unwrap_or_else(|| err.to_string()),
                data: None,
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
}

pub async fn get_all_spending_categories_api() -> HttpResponse {
    let conn = establish_connection().expect("Failed to connect to database");
    let _result: Result<Vec<crate::models::spending::SpendingCategory>, rusqlite::Error> = select_all_spending_categories(&conn);

    match _result {
        Ok(categories) => {
            let response = Response {
                status: "Success".to_string(),
                code: crate::helper::response_code::RESPONSE_CODE_DATA_RETRIEVAL_SUCCESS,
                message: "Success get spending categories".to_string(),
                description: "".to_string(),
                data: Some(serde_json::to_value(categories).unwrap()),
            };
            HttpResponse::Ok().json(response)
        },
        Err(err) => {
            let response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_RETRIEVAL_FAILED,
                message: "Failed to retrieve spending categories".to_string(),
                description: err.sqlite_error().map(|e| format!("{:?}", e)) // or e.to_string() if available
                .unwrap_or_else(|| err.to_string()),
                data: None,
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
}

pub async fn post_spending_api(spending: web::Json<Spending>) -> HttpResponse {
    let connection = establish_connection().expect("Failed to connect to database");

    let new_spending = Spending {
        spending_id: Uuid::new_v4(),
        total_amount: spending.total_amount,
        description: spending.description.clone(),
        spending_category_id: spending.spending_category_id,
        spending_category: spending.spending_category.clone(),
        source_id: spending.source_id,
        source: spending.source.clone(),
        created_date: Utc::now(),
        created_by: spending.created_by.clone(),
        is_active: 1
    };

    
    let mut response = Response {
        status: "Success".to_string(),
        message: "Spending created successfully".to_string(),
        code: crate::helper::response_code::RESPONSE_CODE_DATA_INSERTION_SUCCESS,
        description: "".to_string(),
        data: None,
    };
    let _check_source = select_source(&connection, &new_spending.source_id.to_string());
    let _check_category = select_spending_category(&connection, &new_spending.spending_category_id.to_string());
    if _check_source.is_ok() && _check_category.is_ok() && _check_category.as_ref().unwrap().len() > 0 && _check_source.as_ref().unwrap().len() > 0 {
        let _result  = insert_spending(&connection, &new_spending);
        let mut response = Response {
            status: "Success".to_string(),
            message: "Spending created successfully".to_string(),
            code: crate::helper::response_code::RESPONSE_CODE_DATA_INSERTION_SUCCESS,
            description: "".to_string(),
            data: None,
        };
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

pub async fn post_spending_category_api(category: web::Json<SpendingCategory>) -> HttpResponse {
    let conn = establish_connection().expect("Failed to connect to database");

    let new_category = SpendingCategory {
        spending_category_id: Uuid::new_v4(),
        spending_category: category.spending_category.clone(),
        created_date: Utc::now(),
        created_by: category.created_by.clone(),
        is_active: 1
    };

    let _result = insert_spending_category(&conn, &new_category);

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
                description: err.sqlite_error().map(|e| format!("{:?}", e)) // or e.to_string() if available
                .unwrap_or_else(|| err.to_string()),
                data: None,
            };
            if err.sqlite_error().map_or(false, |e| e.code == rusqlite::ErrorCode::ConstraintViolation) {
                response.description = "Spending category already exists".to_string();
                return HttpResponse::BadRequest().json(response);
            }else{
                return HttpResponse::InternalServerError().json(response)
            }
        }
    }
}

pub async fn delete_spending_api(spending_id: web::Path<String>) -> HttpResponse {
    let conn = establish_connection().expect("Failed to connect to database");

    let _result = delete_spending(&conn, &spending_id.into_inner());

    match _result {
        Ok(_) => {
            let response = Response {
                status: "Success".to_string(),
                code: crate::helper::response_code::RESPONSE_CODE_DATA_DELETION_SUCCESS,
                message: "Spending deleted successfully".to_string(),
                description: "".to_string(),
                data: None,
            };
            HttpResponse::Ok().json(response)
        },
        Err(err) => {
            let response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_DELETION_FAILED,
                message: "Failed to delete spending".to_string(),
                description: err.sqlite_error().map(|e| format!("{:?}", e)) // or e.to_string() if available
                .unwrap_or_else(|| err.to_string()),
                data: None,
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
}

pub async fn delete_spending_category_api(path: web::Path<String>) -> HttpResponse {
    let conn = establish_connection().expect("Failed to connect to database");
    let spending_category = path.into_inner();
    let _result = delete_spending_category(&conn, &spending_category);

    match _result {
        Ok(_) => {
            let response = Response {
                status: "Success".to_string(),
                code: crate::helper::response_code::RESPONSE_CODE_DATA_DELETION_SUCCESS,
                message: "Spending category deleted successfully".to_string(),
                description: "".to_string(),
                data: None,
            };
            HttpResponse::Ok().json(response)
        },
        Err(err) => {
            let response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_DELETION_FAILED,
                message: "Failed to delete spending category".to_string(),
                description: err.sqlite_error().map(|e| format!("{:?}", e)) // or e.to_string() if available
                .unwrap_or_else(|| err.to_string()),
                data: None,
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
}