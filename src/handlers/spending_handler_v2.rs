use actix_web::{web, HttpResponse, HttpRequest, HttpMessage};
use chrono::{Local};
use uuid::Uuid;
use crate::models::source::{SourceV2};
use crate::models::spending::{SpendingV2, SpendingParam, SpendingCategoryV2};
use crate::models::app_setting::{AppSettings};
use crate::models::responses::{Response};
use crate::helper::connection::{establish_connection_v2};
use crate::repository::source_repository_v2::{select_source};
use crate::repository::spending_repository_v2::{select_all_spending_categories, insert_spending, select_spendings, select_spending_category, delete_spending_category, insert_spending_category};
use crate::route_middleware::get_user::CreatedBy;
use crate::repository::app_setting_repository::{select_all_settings};
pub async fn get_all_spendings_api_v2(req: HttpRequest,query: web::Query<SpendingParam>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    
    let _result = select_spendings(&mut conn, &query, Some(created_by));

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

pub async fn get_all_spending_categories_api_v2(req: HttpRequest) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    
    match select_all_spending_categories(&mut conn, &SpendingCategoryV2 {
        spending_category_id: Uuid::nil(),
        spending_category: "".to_string(),
        created_date: Local::now().naive_local(),
        created_by: created_by.clone(),
        is_active: 1
    }) {
        Ok(categories) => {
            let response = Response {
                status: "Success".to_string(),
                code: crate::helper::response_code::RESPONSE_CODE_DATA_RETRIEVAL_SUCCESS,
                message: "Success get spending categories".to_string(),
                description: "".to_string(),
                data: Some(serde_json::to_value(categories).unwrap()),
                success: true
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
                success: false
            };
            HttpResponse::InternalServerError().json(response)
        },
    }
}

pub async fn post_spending_api_v2(req: HttpRequest, spending: web::Json<SpendingV2>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let mut new_spending = SpendingV2 {
        spending_id: Uuid::new_v4(),
        total_amount: spending.total_amount,
        description: spending.description.clone(),
        spending_category_id: spending.spending_category_id,
        spending_category: spending.spending_category.clone(),
        source_id: spending.source_id,
        source: spending.source.clone(),
        created_date: Local::now().naive_local(),
        created_by: created_by.clone(),
        is_active: 1
    };
    
    let source = SourceV2 {
        source_id: new_spending.source_id,
        source: "".to_string(),
        created_date: Local::now().naive_local(),
        created_by: created_by.clone(),
        is_active: 1
    };

    let category = SpendingCategoryV2 {
        spending_category_id: new_spending.spending_category_id,
        spending_category: "".to_string(),
        created_date: Local::now().naive_local(),
        created_by: created_by.clone(),
        is_active: 1
    };

    
    let _check_source = select_source(&mut conn, &source);
    let _check_category = select_spending_category(&mut conn, &category);
    // If transfer, get transfer category from app settings
    let app_setting: AppSettings = AppSettings {
        app_setting_id: Uuid::nil(),
        app_setting_key: "".to_string(),
        app_setting_value: "".to_string(),
        is_active: 1
    };

    let _get_app_setting = select_all_settings(&mut conn, &app_setting);
    let mut settings_bypass = false;
    if _get_app_setting.is_ok() {
        let mut recount_category_id = Uuid::nil();
        let mut recount_category_name = String::new();
        let mut transfer_category_id = Uuid::nil();
        let mut transfer_category_name = String::new();
        let mut debt_category_id = Uuid::nil();
        let mut debt_category_name = String::new();
        for setting in _get_app_setting.unwrap() {
            if setting.app_setting_key == "TRANSFER_CATEGORY_ID" {
                transfer_category_id = Uuid::parse_str(&setting.app_setting_value).unwrap_or_else(|_| Uuid::nil());
            }else if setting.app_setting_key == "TRANSFER_CATEGORY_NAME" {
                transfer_category_name = setting.app_setting_value.clone();
            }   
            if setting.app_setting_key == "RECOUNT_CATEGORY_ID" {
                recount_category_id = Uuid::parse_str(&setting.app_setting_value).unwrap_or_else(|_| Uuid::nil());
            }else if setting.app_setting_key == "RECOUNT_CATEGORY_NAME" {
                recount_category_name = setting.app_setting_value.clone();
            }
            if setting.app_setting_key == "DEBT_CATEGORY_ID" {
                debt_category_id = Uuid::parse_str(&setting.app_setting_value).unwrap_or_else(|_| Uuid::nil());
            }else if setting.app_setting_key == "DEBT_CATEGORY_NAME" {
                debt_category_name = setting.app_setting_value.clone();
            }
        }
        if transfer_category_id != Uuid::nil() && transfer_category_id == new_spending.spending_category_id {
            new_spending.spending_category = transfer_category_name.clone(); 
            settings_bypass = true;
        }
        if recount_category_id != Uuid::nil() && recount_category_id == new_spending.spending_category_id {
            new_spending.spending_category = recount_category_name.clone(); 
            settings_bypass = true;
        }
        if debt_category_id != Uuid::nil() && debt_category_id == new_spending.spending_category_id {
            new_spending.spending_category = debt_category_name.clone(); 
            settings_bypass = true;
        }
    }
    let mut response = Response {
        status: "Success".to_string(),
        code: crate::helper::response_code::RESPONSE_CODE_DATA_INSERTION_SUCCESS,
        message: "Spending category created successfully".to_string(),
        description: "".to_string(),
        data: None,
        success: true
    };
    if _check_source.is_ok() && _check_category.is_ok() && (_check_category.as_ref().unwrap().len() > 0 || settings_bypass) && _check_source.as_ref().unwrap().len() > 0 {
        let _result  = insert_spending(&mut conn, &new_spending);
        
        if _result.is_err() {
            response = Response {
                status: "Error".to_string(),
                message: "Failed to create spending ".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                description: _result.err().unwrap().to_string(),
                data: None,
                success: false
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
                success: false
            };
        }else if _check_category.as_ref().unwrap().len() == 0 {
            response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                message: "Spending category not found".to_string(),
                description: "Please create the spending category first.".to_string(),
                data: None,
                success: false
            };
        }else if _check_source.is_err() {
            response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                message: "Failed to create spending [Source]".to_string(),
                description: _check_source.err().unwrap().to_string(),
                data: None,
                success: false
            };
        }else if _check_source.as_ref().unwrap().len() == 0 {
            response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                message: "Source not found".to_string(),
                description: "Please create the source first.".to_string(),
                data: None,
                success: false
            };
        }
    }
    if response.code == crate::helper::response_code::RESPONSE_CODE_DATA_INSERTION_SUCCESS {
        HttpResponse::Created().json(response)
    }else{
        response.success = false;
        HttpResponse::BadRequest().json(response)
    }
    
}

pub async fn post_spending_category_api_v2(req: HttpRequest, category: web::Json<SpendingCategoryV2>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let new_category = SpendingCategoryV2 {
        spending_category_id: Uuid::new_v4(),
        spending_category: category.spending_category.clone(),
        created_date: Local::now().naive_local(),
        created_by: created_by.clone(),
        is_active: 1
    };
    
    let _result  = insert_spending_category(&mut conn, &new_category);

    match _result {
        Ok(_) => {
            let response = Response {
                status: "Success".to_string(),
                code: crate::helper::response_code::RESPONSE_CODE_DATA_INSERTION_SUCCESS,
                message: "Spending category inserted successfully".to_string(),
                description: "".to_string(),
                data: Some(serde_json::to_value(new_category).unwrap()),
                success: true
            };
            HttpResponse::Ok().json(response)
        },
        Err(err) => {
            let response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                message: "Failed to insert spending category".to_string(),
                description: err.to_string(),
                data: None,
                success: false
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
    
}

pub async fn delete_spending_category_api_v2(req: HttpRequest, path: web::Path<String>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let spending_category_name = path.into_inner();
    let spending_category = SpendingCategoryV2 {
        spending_category_id: Uuid::nil(),
        spending_category: spending_category_name,
        created_date: Local::now().naive_local(),
        created_by: created_by.clone(),
        is_active: 1
    };
    let result = delete_spending_category(&mut conn, &spending_category);

    match result {
        Ok(_) => {
            let response = Response {
                status: "Success".to_string(),
                code: crate::helper::response_code::RESPONSE_CODE_DATA_RETRIEVAL_SUCCESS,
                message: "Success delete spending category".to_string(),
                description: "".to_string(),
                data: None,
                success: true
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
                success: false
            };
            HttpResponse::InternalServerError().json(response)
        },
    }
}