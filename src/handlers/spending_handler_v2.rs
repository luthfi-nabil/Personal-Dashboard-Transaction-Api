use crate::helper::connection::establish_connection_v2;
use crate::helper::settings_client::global_category_wiring;
use crate::models::responses::Response;
use crate::models::source::SourceV2;
use crate::models::spending::{
    SpendingCategoryV2, SpendingCreateV2, SpendingDetailCheckedInput, SpendingDetailParamQuery,
    SpendingDetailV2, SpendingParam, SpendingV2,
};
use crate::repository::source_repository_v2::select_source;
use crate::repository::spending_repository_v2::{
    delete_spending, delete_spending_category, delete_spending_details, insert_spending,
    insert_spending_category, insert_spending_details, select_all_spending_categories,
    select_spending_category, select_spending_details, select_spendings,
    update_spending_detail_checked,
};
use crate::route_middleware::get_user::CreatedBy;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use chrono::Local;
use uuid::Uuid;
pub async fn get_all_spendings_api_v2(
    req: HttpRequest,
    query: web::Query<SpendingParam>,
) -> HttpResponse {
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

pub async fn get_all_spending_categories_api_v2(req: HttpRequest) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();

    match select_all_spending_categories(
        &mut conn,
        &SpendingCategoryV2 {
            spending_category_id: Uuid::nil(),
            spending_category: "".to_string(),
            created_date: Local::now().naive_local(),
            created_by: created_by.clone(),
            is_active: 1,
        },
    ) {
        Ok(categories) => {
            let response = Response {
                status: "Success".to_string(),
                code: crate::helper::response_code::RESPONSE_CODE_DATA_RETRIEVAL_SUCCESS,
                message: "Success get spending categories".to_string(),
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
                message: "Failed to retrieve spending categories".to_string(),
                description: err.to_string(),
                data: None,
                success: false,
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
}

pub async fn post_spending_api_v2(
    req: HttpRequest,
    spending: web::Json<SpendingCreateV2>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let new_spending_id = Uuid::new_v4();
    let created_date = Local::now().naive_local();

    // Line items are optional. When the client sends details but leaves
    // total_amount at 0 (e.g. the receipt scanner), the total is derived from
    // the confirmed items so the header and its detail always agree. Unticked
    // items stay in the breakdown but are not part of what was paid.
    let details_total: f64 = spending
        .details
        .iter()
        .filter(|d| d.checked)
        .map(|d| {
            if d.amount != 0.0 {
                d.amount
            } else {
                d.quantity * d.unit_price
            }
        })
        .sum();
    let resolved_total = if spending.total_amount > 0.0 {
        spending.total_amount
    } else {
        details_total
    };

    let new_details: Vec<SpendingDetailV2> = spending
        .details
        .iter()
        .map(|d| {
            let amount = if d.amount != 0.0 {
                d.amount
            } else {
                d.quantity * d.unit_price
            };
            let unit_price = if d.unit_price != 0.0 {
                d.unit_price
            } else if d.quantity != 0.0 {
                amount / d.quantity
            } else {
                amount
            };
            SpendingDetailV2 {
                spending_detail_id: Uuid::new_v4(),
                spending_id: new_spending_id,
                item_name: d.item_name.clone(),
                quantity: if d.quantity == 0.0 { 1.0 } else { d.quantity },
                unit_price,
                amount,
                note: d.note.clone(),
                is_checked: d.checked,
                created_date,
                created_by: created_by.clone(),
                is_active: 1,
            }
        })
        .collect();

    let mut new_spending = SpendingV2 {
        spending_id: new_spending_id,
        total_amount: resolved_total,
        description: spending.description.clone(),
        spending_category_id: spending.spending_category_id,
        spending_category: spending.spending_category.clone(),
        source_id: spending.source_id,
        source: spending.source.clone(),
        created_date,
        created_by: created_by.clone(),
        is_active: 1,
    };

    let source = SourceV2 {
        source_id: new_spending.source_id,
        source: "".to_string(),
        created_date: Local::now().naive_local(),
        created_by: created_by.clone(),
        is_active: 1,
    };

    let category = SpendingCategoryV2 {
        spending_category_id: new_spending.spending_category_id,
        spending_category: "".to_string(),
        created_date: Local::now().naive_local(),
        created_by: created_by.clone(),
        is_active: 1,
    };

    let _check_source = select_source(&mut conn, &source);
    let _check_category = select_spending_category(&mut conn, &category);
    // Transfer / recount / debt categories are server-owned wiring that lives in
    // login-api's `app_settings`. When the posted category is one of them the
    // name is taken from there and the local category check is bypassed.
    let wiring = global_category_wiring().await;
    let settings_bypass = match wiring.resolve_name(new_spending.spending_category_id) {
        Some(name) => {
            new_spending.spending_category = name;
            true
        }
        None => false,
    };
    let mut response = Response {
        status: "Success".to_string(),
        code: crate::helper::response_code::RESPONSE_CODE_DATA_INSERTION_SUCCESS,
        message: "Spending category created successfully".to_string(),
        description: "".to_string(),
        data: None,
        success: true,
    };
    if _check_source.is_ok()
        && _check_category.is_ok()
        && (_check_category.as_ref().unwrap().len() > 0 || settings_bypass)
        && _check_source.as_ref().unwrap().len() > 0
    {
        let _result = insert_spending(&mut conn, &new_spending);

        if _result.is_err() {
            response = Response {
                status: "Error".to_string(),
                message: "Failed to create spending ".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                description: _result.err().unwrap().to_string(),
                data: None,
                success: false,
            };
        } else {
            // The spending header is in; now persist its line items. If the
            // detail insert fails the header is rolled back so the caller never
            // ends up with a total that has no matching breakdown.
            let _detail_result = insert_spending_details(&mut conn, &new_details);
            if let Err(err) = _detail_result {
                let _ = delete_spending(&mut conn, &new_spending);
                response = Response {
                    status: "Error".to_string(),
                    message: "Failed to create spending [Detail]".to_string(),
                    code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                    description: err.to_string(),
                    data: None,
                    success: false,
                };
            } else {
                let mut payload = serde_json::to_value(new_spending).unwrap();
                if let Some(obj) = payload.as_object_mut() {
                    obj.insert(
                        "details".to_string(),
                        serde_json::to_value(&new_details).unwrap(),
                    );
                }
                response.data = Some(payload);
            }
        }
    } else {
        if _check_category.is_err() {
            response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                message: "Failed to create spending [Category]".to_string(),
                description: _check_category.err().unwrap().to_string(),
                data: None,
                success: false,
            };
        } else if _check_category.as_ref().unwrap().len() == 0 {
            response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                message: "Spending category not found".to_string(),
                description: "Please create the spending category first.".to_string(),
                data: None,
                success: false,
            };
        } else if _check_source.is_err() {
            response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                message: "Failed to create spending [Source]".to_string(),
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

/// `GET /api/user/spending-details[?spending_id=...]`
///
/// Without `spending_id` this returns every line item the user owns, which is
/// what the Flutter client pulls on sync so transaction details are available
/// offline.
pub async fn get_spending_details_api_v2(
    req: HttpRequest,
    query: web::Query<SpendingDetailParamQuery>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();

    match select_spending_details(&mut conn, query.spending_id, Some(created_by)) {
        Ok(details) => {
            let response = Response {
                status: "Success".to_string(),
                code: crate::helper::response_code::RESPONSE_CODE_DATA_RETRIEVAL_SUCCESS,
                message: "Success get spending details".to_string(),
                description: "".to_string(),
                data: Some(serde_json::to_value(details).unwrap()),
                success: true,
            };
            HttpResponse::Ok().json(response)
        }
        Err(err) => {
            let response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_RETRIEVAL_FAILED,
                message: "Failed to retrieve spending details".to_string(),
                description: err.to_string(),
                data: None,
                success: false,
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
}

/// `GET /api/user/spendings/{spending_id}/details`
pub async fn get_spending_details_by_id_api_v2(
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let spending_id = Uuid::parse_str(&path.into_inner()).unwrap_or_else(|_| Uuid::nil());

    match select_spending_details(&mut conn, Some(spending_id), Some(created_by)) {
        Ok(details) => {
            let response = Response {
                status: "Success".to_string(),
                code: crate::helper::response_code::RESPONSE_CODE_DATA_RETRIEVAL_SUCCESS,
                message: "Success get spending details".to_string(),
                description: "".to_string(),
                data: Some(serde_json::to_value(details).unwrap()),
                success: true,
            };
            HttpResponse::Ok().json(response)
        }
        Err(err) => {
            let response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_RETRIEVAL_FAILED,
                message: "Failed to retrieve spending details".to_string(),
                description: err.to_string(),
                data: None,
                success: false,
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
}

/// `PUT /api/user/spending-details/{spending_detail_id}/checked`
///
/// Ticks or unticks one line item. The spending header itself is immutable, so
/// this is the only field of a saved breakdown a client can change.
pub async fn put_spending_detail_checked_api_v2(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<SpendingDetailCheckedInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let detail_id = match Uuid::parse_str(&path.into_inner()) {
        Ok(id) => id,
        Err(err) => {
            return HttpResponse::BadRequest().json(Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_UPDATE_FAILED,
                message: "Invalid spending detail id".to_string(),
                description: err.to_string(),
                data: None,
                success: false,
            });
        }
    };

    match update_spending_detail_checked(&mut conn, detail_id, &created_by, body.checked) {
        Ok(0) => HttpResponse::NotFound().json(Response {
            status: "Error".to_string(),
            code: crate::helper::response_code::ERROR_CODE_DATA_UPDATE_FAILED,
            message: "Spending detail not found".to_string(),
            description: "No line item with that id belongs to this user".to_string(),
            data: None,
            success: false,
        }),
        Ok(_) => HttpResponse::Ok().json(Response {
            status: "Success".to_string(),
            code: crate::helper::response_code::RESPONSE_CODE_DATA_UPDATE_SUCCESS,
            message: "Spending detail updated".to_string(),
            description: "".to_string(),
            data: None,
            success: true,
        }),
        Err(err) => HttpResponse::InternalServerError().json(Response {
            status: "Error".to_string(),
            code: crate::helper::response_code::ERROR_CODE_DATA_UPDATE_FAILED,
            message: "Failed to update spending detail".to_string(),
            description: err.to_string(),
            data: None,
            success: false,
        }),
    }
}

pub async fn delete_spending_api_v2(req: HttpRequest, path: web::Path<String>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let spending = SpendingV2 {
        spending_id: Uuid::parse_str(&path.into_inner()).unwrap_or_else(|_| Uuid::nil()),
        total_amount: 0.0,
        description: "".to_string(),
        spending_category_id: Uuid::nil(),
        spending_category: "".to_string(),
        source_id: Uuid::nil(),
        source: "".to_string(),
        created_date: Local::now().naive_local(),
        created_by,
        is_active: 1,
    };

    // Detail rows have no FK cascade (the table is created by the app, not by a
    // migration tool), so clear them explicitly before dropping the header.
    let _ = delete_spending_details(&mut conn, spending.spending_id, &spending.created_by);

    match delete_spending(&mut conn, &spending) {
        Ok(_) => {
            let response = Response {
                status: "Success".to_string(),
                code: crate::helper::response_code::RESPONSE_CODE_DATA_RETRIEVAL_SUCCESS,
                message: "Success delete spending".to_string(),
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
                message: "Failed to delete spending".to_string(),
                description: err.to_string(),
                data: None,
                success: false,
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
}

pub async fn post_spending_category_api_v2(
    req: HttpRequest,
    category: web::Json<SpendingCategoryV2>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let new_category = SpendingCategoryV2 {
        spending_category_id: Uuid::new_v4(),
        spending_category: category.spending_category.clone(),
        created_date: Local::now().naive_local(),
        created_by: created_by.clone(),
        is_active: 1,
    };

    let _result = insert_spending_category(&mut conn, &new_category);

    match _result {
        Ok(_) => {
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
        Err(err) => {
            let response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_INSERTION_FAILED,
                message: "Failed to insert spending category".to_string(),
                description: err.to_string(),
                data: None,
                success: false,
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
}

pub async fn delete_spending_category_api_v2(
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let spending_category_name = path.into_inner();
    let spending_category = SpendingCategoryV2 {
        spending_category_id: Uuid::nil(),
        spending_category: spending_category_name,
        created_date: Local::now().naive_local(),
        created_by: created_by.clone(),
        is_active: 1,
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
                success: true,
            };
            HttpResponse::Ok().json(response)
        }
        Err(err) => {
            let response = Response {
                status: "Error".to_string(),
                code: crate::helper::response_code::ERROR_CODE_DATA_RETRIEVAL_FAILED,
                message: "Failed to delete spending category".to_string(),
                description: err.to_string(),
                data: None,
                success: false,
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
}
