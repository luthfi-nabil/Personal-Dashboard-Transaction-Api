use actix_web::{web, HttpResponse, HttpRequest, HttpMessage};
use chrono::{Local};
use uuid::Uuid;
use crate::models::{app_setting, debt};
use crate::models::debt::{Debt, DebtParam};
use crate::models::source::{SourceV2};
use crate::models::spending::{SpendingParam};
use crate::models::earning::{EarningV2, EarningParam, EarningCategoryV2};
use crate::models::responses::{Response, DatabaseResult};
use crate::helper::connection::{establish_connection_v2};
use crate::repository::debt_repository::{insert_debt, select_debt, update_debt};
use crate::repository::earning_repository_v2::{select_all_earning_categories, insert_earning, select_earnings, select_earning_category, delete_earning_category, insert_earning_category};
use crate::repository::app_setting_repository::{select_all_settings};
use crate::route_middleware::get_user::CreatedBy;

pub async fn get_debt(req: HttpRequest,query: web::Query<DebtParam>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let _result = select_debt(&mut conn, &query, Some(created_by));

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

pub async fn post_debt_api(req: HttpRequest,debt: web::Json<Debt>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let mut new_debt = Debt {
        debt_id: Uuid::new_v4(),
        amount: debt.amount,
        description: debt.description.clone(),
        debt_type: debt.debt_type,
        debt_earning_id: Some(debt.debt_earning_id.unwrap_or_else(|| Uuid::nil())),
        debt_spending_id: Some(debt.debt_spending_id.unwrap_or_else(|| Uuid::nil())),
        status: 1,
        created_date: Local::now().naive_local(),
        created_by: created_by.clone().to_string(),
        is_active: 1
    };
    println!("Debug: New debt data {:?}", new_debt);
    let mut is_statement_valid = false;
    //Debt Type 1 = Kredit/Pinjaman
    //Debt Type 2 = Debit/Hutang
    if new_debt.debt_type == 1 {
        let mut earning_param = EarningParam {
            description: None,
            earning_category: None,
            earning_category_id: None,
            source_id: None,
            source: None,
            month: None,
            year: None,
            day: None,
            earning_id: Some(new_debt.debt_earning_id.unwrap_or_else(|| Uuid::nil()))
        };
        let _check_earning = select_earnings(&mut conn, &earning_param, Some(created_by.clone()));
        if _check_earning.is_ok() && _check_earning.as_ref().unwrap().len() > 0 {
            is_statement_valid = true;
        }
    }else if new_debt.debt_type == 2 {
        let mut spending_param = SpendingParam {
            description: None,
            spending_category: None,
            spending_category_id: None,
            source_id: None,
            source: None,
            month: None,
            year: None,
            day: None,
            spending_id: Some(new_debt.debt_spending_id.unwrap_or_else(|| Uuid::nil()))
        };
        
        let _check_spending = crate::repository::spending_repository_v2::select_spendings(&mut conn, &spending_param, Some(created_by.clone()));
        println!("Debug: Check spending result {:?}", _check_spending.as_ref().unwrap().len() > 0);
        if _check_spending.is_ok() && _check_spending.as_ref().unwrap().len() > 0 {
            is_statement_valid = true;
        }
    }
    let mut response = Response {
        status: "Success".to_string(),
        message: "Success create debt".to_string(),
        code: crate::helper::response_code::RESPONSE_CODE_DATA_INSERTION_SUCCESS,
        description: "".to_string(),
        data: None,
        success: true
    };
    if is_statement_valid {
        let _result  = insert_debt(&mut conn, &new_debt);
        
        if _result.is_err() {
            response = Response {
                status: "Error".to_string(),
                message: "Failed to create debt ".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                description: _result.err().unwrap().to_string(),
                data: None,
                success: false
            };
        }else{
            response.data = Some(serde_json::to_value(new_debt).unwrap());
        }
    }else{
         response = Response {
            status: "Error".to_string(),
            code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
            message: "Failed to create debt".to_string(),
            description: "Statement Invalid, no transaction found".to_string(),
            data: None,
            success: false
        };
    }
    if response.code == crate::helper::response_code::RESPONSE_CODE_DATA_INSERTION_SUCCESS {
        HttpResponse::Created().json(response)
    }else{
        response.success = false;
        HttpResponse::BadRequest().json(response)
    }
}

pub async fn update_debt_status(req: HttpRequest, param: web::Json<DebtParam>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let mut _debt = select_debt(&mut conn, &DebtParam {
        debt_id: Some(param.debt_id.clone().unwrap_or_else(|| "".to_string())),
        debt_type: None,
        description: None,
        debt_earning_id: None,
        debt_spending_id: None,
        status: None,
    }, Some(created_by.clone()));
    if _debt.is_err() || _debt.as_ref().unwrap().len() == 0 {
        let response = Response {
            status: "Error".to_string(),
            code: crate::helper::response_code::ERROR_CODE_DATA_RETRIEVAL_FAILED,
            message: "Failed to retrieve debt".to_string(),
            description: "Debt not found".to_string(),
            data: None,
            success: false
        };
        return HttpResponse::BadRequest().json(response);
    }
    let mut update_debt_param = _debt.unwrap().get(0).unwrap().to_owned();
    let mut status_transaction_data = false;
    //Opposite of insert for assigning debt_earning_id and debt_spending_id
    if update_debt_param.debt_type == 1{
        let spending_data = crate::repository::spending_repository_v2::select_spendings(&mut conn, &SpendingParam {
            description: None,
            spending_category: None,
            spending_category_id: None,
            source_id: None,
            source: None,
            month: None,
            year: None,
            day: None,
            spending_id: Some(update_debt_param.debt_earning_id.unwrap_or_else(|| Uuid::nil())),
        }, Some(created_by.clone()));
        if spending_data.is_ok() && spending_data.as_ref().unwrap().len() > 0 {
            status_transaction_data = true;
            update_debt_param.debt_spending_id = param.debt_spending_id.as_ref().and_then(|id| Uuid::parse_str(id).ok());
        }
        
    }else if update_debt_param.debt_type == 2{
        let earning_data = crate::repository::earning_repository_v2::select_earnings(&mut conn, &EarningParam {
            description: None,
            earning_category: None,
            earning_category_id: None,
            source_id: None,
            source: None,
            month: None,
            year: None,
            day: None,
            earning_id: Some(update_debt_param.debt_spending_id.unwrap_or_else(|| Uuid::nil())),
        }, Some(created_by.clone()));
        if earning_data.is_ok() && earning_data.as_ref().unwrap().len() > 0 {
            status_transaction_data = true;
            update_debt_param.debt_earning_id = param.debt_earning_id.as_ref().and_then(|id| Uuid::parse_str(id).ok());
        }
    }
    if !status_transaction_data {
        let response = Response {
            status: "Error".to_string(),
            code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
            message: "Failed to update debt".to_string(),
            description: "Transaction data not found".to_string(),
            data: None,
            success: false
        };
        return HttpResponse::BadRequest().json(response);
    }
    let _result = update_debt(&mut conn, &update_debt_param);

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
                message: "Failed to update debt".to_string(),
                description: err.to_string(),
                data: None,
                success: false
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
}