use crate::helper::connection::establish_connection_v2;
use crate::helper::settings_client::global_category_wiring;
use crate::models::earning::{EarningCategoryV2, EarningParam, EarningV2};
use crate::models::responses::{DatabaseResult, Response};
use crate::models::source::SourceV2;
use crate::repository::earning_repository_v2::{
    delete_earning, delete_earning_category, insert_earning, insert_earning_category,
    select_all_earning_categories, select_earning_category, select_earnings,
};
use crate::repository::source_repository_v2::select_source;
use crate::route_middleware::get_user::CreatedBy;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use chrono::Local;
use uuid::Uuid;

pub async fn get_all_earnings_api_v2(
    req: HttpRequest,
    query: web::Query<EarningParam>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let _result = select_earnings(&mut conn, &query, Some(created_by));

    match _result {
        Ok(sources) => {
            let response = Response {
                status: "Success".to_string(),
                code: crate::helper::response_code::RESPONSE_CODE_DATA_RETRIEVAL_SUCCESS,
                message: "Success get sources".to_string(),
                description: "".to_string(),
                data: Some(serde_json::to_value(sources).unwrap()),
                success: true,
            };
            HttpResponse::Ok().json(response)
        }
        Err(err) => {
            let response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_RETRIEVAL_FAILED,
                message: "Failed to retrieve sources".to_string(),
                description: err.to_string(),
                data: None,
                success: false,
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
}

pub async fn get_all_earning_categories_api_v2(req: HttpRequest) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let earning_categories = EarningCategoryV2 {
        earning_category_id: Uuid::nil(),
        earning_category: "".to_string(),
        created_date: Local::now().naive_local(),
        created_by: created_by,
        is_active: 1,
    };
    match select_all_earning_categories(&mut conn, &earning_categories) {
        Ok(categories) => {
            let response = Response {
                status: "Success".to_string(),
                code: crate::helper::response_code::RESPONSE_CODE_DATA_RETRIEVAL_SUCCESS,
                message: "Success get earning categories".to_string(),
                description: "".to_string(),
                data: Some(serde_json::to_value(categories).unwrap()),
                success: true,
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
                success: false,
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
}

pub async fn post_earning_api_v2(req: HttpRequest, earning: web::Json<EarningV2>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let mut new_earning = EarningV2 {
        earning_id: Uuid::new_v4(),
        total_amount: earning.total_amount,
        description: earning.description.clone(),
        earning_category_id: earning.earning_category_id,
        earning_category: earning.earning_category.clone(),
        source_id: earning.source_id,
        source: earning.source.clone(),
        created_date: Local::now().naive_local(),
        created_by: created_by.clone().to_string(),
        is_active: 1,
    };

    let source: SourceV2 = SourceV2 {
        source_id: earning.source_id.clone(),
        source: "".to_string(),
        created_date: Local::now().naive_local(),
        created_by: created_by.clone().to_string(),
        is_active: 1,
    };

    let earning_category: EarningCategoryV2 = EarningCategoryV2 {
        earning_category_id: new_earning.earning_category_id,
        earning_category: "".to_string(),
        created_date: Local::now().naive_local(),
        created_by: created_by.clone().to_string(),
        is_active: 1,
    };

    let _check_source = select_source(&mut conn, &source);
    let _check_category = select_earning_category(&mut conn, &earning_category);

    // Transfer / recount / debt categories are server-owned wiring that lives in
    // login-api's `app_settings`. When the posted category is one of them the
    // name is taken from there and the local category check is bypassed.
    let wiring = global_category_wiring().await;
    let settings_bypass = match wiring.resolve_name(new_earning.earning_category_id) {
        Some(name) => {
            new_earning.earning_category = name;
            true
        }
        None => false,
    };
    let mut response = Response {
        status: "Success".to_string(),
        code: crate::helper::response_code::RESPONSE_CODE_DATA_INSERTION_SUCCESS,
        message: "Earning category created successfully".to_string(),
        description: "".to_string(),
        data: None,
        success: true,
    };
    if _check_source.is_ok()
        && _check_category.is_ok()
        && (_check_category.as_ref().unwrap().len() > 0 || settings_bypass)
        && _check_source.as_ref().unwrap().len() > 0
    {
        let _result = insert_earning(&mut conn, &new_earning);

        if _result.is_err() {
            response = Response {
                status: "Error".to_string(),
                message: "Failed to create earning ".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                description: _result.err().unwrap().to_string(),
                data: None,
                success: false,
            };
        } else {
            response.data = Some(serde_json::to_value(new_earning).unwrap());
        }
    } else {
        if _check_category.is_err() {
            response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                message: "Failed to create earning [Category]".to_string(),
                description: _check_category.err().unwrap().to_string(),
                data: None,
                success: false,
            };
        } else if _check_category.as_ref().unwrap().len() == 0 {
            response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                message: "Earning category not found".to_string(),
                description: "Please create the earning category first.".to_string(),
                data: None,
                success: false,
            };
        } else if _check_source.is_err() {
            response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                message: "Failed to create earning [Source]".to_string(),
                description: _check_source.err().unwrap().to_string(),
                data: None,
                success: false,
            };
        } else if _check_source.as_ref().unwrap().len() == 0 {
            response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                message: "Source not found".to_string(),
                description: "Please create the source first.".to_string(),
                data: None,
                success: false,
            };
        }
    }
    if response.code == crate::helper::response_code::RESPONSE_CODE_DATA_INSERTION_SUCCESS {
        HttpResponse::Created().json(response)
    } else {
        response.success = false;
        HttpResponse::BadRequest().json(response)
    }
}

pub async fn delete_earning_api_v2(req: HttpRequest, path: web::Path<String>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let earning = EarningV2 {
        earning_id: Uuid::parse_str(&path.into_inner()).unwrap_or_else(|_| Uuid::nil()),
        total_amount: 0.0,
        description: "".to_string(),
        earning_category_id: Uuid::nil(),
        earning_category: "".to_string(),
        source_id: Uuid::nil(),
        source: "".to_string(),
        created_date: Local::now().naive_local(),
        created_by,
        is_active: 1,
    };

    match delete_earning(&mut conn, &earning) {
        Ok(_) => {
            let response = Response {
                status: "Success".to_string(),
                code: crate::helper::response_code::RESPONSE_CODE_DATA_RETRIEVAL_SUCCESS,
                message: "Success delete earning".to_string(),
                description: "".to_string(),
                data: None,
                success: true,
            };
            HttpResponse::Ok().json(response)
        }
        Err(err) => {
            let response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_RETRIEVAL_FAILED,
                message: "Failed to delete earning".to_string(),
                description: err.to_string(),
                data: None,
                success: false,
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
}

pub async fn post_earning_category_api_v2(
    req: HttpRequest,
    category: web::Json<EarningCategoryV2>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let new_category = EarningCategoryV2 {
        earning_category_id: Uuid::new_v4(),
        earning_category: category.earning_category.clone(),
        created_date: Local::now().naive_local(),
        created_by: created_by.clone(),
        is_active: 1,
    };

    let _result = insert_earning_category(&mut conn, &new_category);
    match _result {
        Ok(e) => match e {
            DatabaseResult::Inserted => {
                let response = Response {
                    status: "Success".to_string(),
                    code: crate::helper::response_code::RESPONSE_CODE_DATA_INSERTION_SUCCESS,
                    message: "Spending category inserted successfully".to_string(),
                    description: "".to_string(),
                    data: Some(serde_json::to_value(new_category).unwrap()),
                    success: true,
                };
                HttpResponse::Ok().json(response)
            }
            DatabaseResult::Duplicate => {
                let response = Response {
                    status: "Error".to_string(),
                    code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                    message: "Failed to insert earning category".to_string(),
                    description: "Earning category already exists".to_string(),
                    data: None,
                    success: false,
                };
                HttpResponse::BadRequest().json(response)
            }
        },
        Err(err) => {
            let response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                message: "Failed to insert earning category".to_string(),
                description: err.to_string(),
                data: None,
                success: false,
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
}

pub async fn delete_earning_category_api_v2(
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let earning_category = path.into_inner();
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let earning_category = EarningCategoryV2 {
        earning_category_id: Uuid::parse_str(&earning_category).unwrap_or_else(|_| Uuid::nil()),
        earning_category: "".to_string(),
        created_date: Local::now().naive_local(),
        created_by: created_by.clone(),
        is_active: 1,
    };
    let result = delete_earning_category(&mut conn, &earning_category);

    match result {
        Ok(_) => {
            let response = Response {
                status: "Success".to_string(),
                code: crate::helper::response_code::RESPONSE_CODE_DATA_RETRIEVAL_SUCCESS,
                message: "Success delete earning category".to_string(),
                description: "".to_string(),
                data: None,
                success: true,
            };
            HttpResponse::Ok().json(response)
        }
        Err(err) => {
            let response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_RETRIEVAL_FAILED,
                message: "Failed to delete earning category".to_string(),
                description: err.to_string(),
                data: None,
                success: false,
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
}
