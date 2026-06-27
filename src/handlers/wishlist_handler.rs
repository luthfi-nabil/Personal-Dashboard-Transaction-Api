use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use chrono::Local;
use uuid::Uuid;

use crate::helper::connection::establish_connection_v2;
use crate::models::responses::Response;
use crate::models::wishlist::{
    PlannedExpenseCategory, PlannedExpenseCategoryInput, PlannedExpenseInput, PlannedExpenseItem,
    PlannedExpenseStatusInput,
};
use crate::repository::wishlist_repository::{
    delete_planned_expense_category, insert_planned_expense_category, remove_wishlist,
    select_planned_expense_categories, select_wishlist, update_wishlist_status, upsert_wishlist,
};
use crate::route_middleware::get_user::CreatedBy;

fn ok_response(message: &str, data: Option<serde_json::Value>) -> Response {
    Response {
        status: "Success".to_string(),
        code: crate::helper::response_code::RESPONSE_CODE_DATA_RETRIEVAL_SUCCESS,
        message: message.to_string(),
        description: "".to_string(),
        data,
        success: true,
    }
}

fn err_response(message: &str, description: String) -> Response {
    Response {
        status: "Error".to_string(),
        code: crate::helper::response_code::ERROR_CODE_DATA_RETRIEVAL_FAILED,
        message: message.to_string(),
        description,
        data: None,
        success: false,
    }
}

pub async fn get_planned_expenses_api(req: HttpRequest) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();

    match select_wishlist(&mut conn, &created_by) {
        Ok(items) => HttpResponse::Ok().json(ok_response(
            "Success get planned expenses",
            Some(serde_json::to_value(items).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError().json(err_response(
            "Failed to retrieve planned expenses",
            err.to_string(),
        )),
    }
}

pub async fn post_planned_expense_api(
    req: HttpRequest,
    body: web::Json<PlannedExpenseInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let now = Local::now().naive_local();
    let transaction_type = body
        .transaction_type
        .clone()
        .unwrap_or_else(|| "spending".to_string());
    if transaction_type != "spending" && transaction_type != "earning" {
        return HttpResponse::BadRequest().json(err_response(
            "Invalid planned expense type",
            "transaction_type must be spending or earning".to_string(),
        ));
    }

    let item = PlannedExpenseItem {
        planned_expense_id: body.planned_expense_id.unwrap_or_else(Uuid::new_v4),
        item_name: body.item_name.clone(),
        price: body.price,
        transaction_type,
        category_id: body.category_id,
        category: body.category.clone(),
        notes: body.notes.clone(),
        priority: body.priority.clone(),
        status: "active".to_string(),
        fulfilled_price: None,
        fulfilled_at: None,
        canceled_at: None,
        created_date: now,
        updated_date: now,
        created_by,
        is_active: 1,
    };

    match upsert_wishlist(&mut conn, &item) {
        Ok(_) => HttpResponse::Ok().json(ok_response(
            "Planned expense saved successfully",
            Some(serde_json::to_value(item).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError().json(err_response(
            "Failed to save planned expense",
            err.to_string(),
        )),
    }
}

pub async fn put_planned_expense_status_api(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<PlannedExpenseStatusInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let status = body.status.as_str();
    if status != "active" && status != "fulfilled" && status != "canceled" {
        return HttpResponse::BadRequest().json(err_response(
            "Invalid planned expense status",
            "Status must be active, fulfilled, or canceled".to_string(),
        ));
    }

    match update_wishlist_status(
        &mut conn,
        &path.into_inner(),
        &created_by,
        status,
        body.fulfilled_price,
    ) {
        Ok(_) => HttpResponse::Ok().json(ok_response("Planned expense status updated", None)),
        Err(err) => HttpResponse::InternalServerError().json(err_response(
            "Failed to update planned expense status",
            err.to_string(),
        )),
    }
}

pub async fn delete_planned_expense_api(req: HttpRequest, path: web::Path<String>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();

    match remove_wishlist(&mut conn, &path.into_inner(), &created_by) {
        Ok(_) => HttpResponse::Ok().json(ok_response("Planned expense removed", None)),
        Err(err) => HttpResponse::InternalServerError().json(err_response(
            "Failed to remove planned expense",
            err.to_string(),
        )),
    }
}

pub async fn get_planned_expense_categories_api(req: HttpRequest) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();

    match select_planned_expense_categories(&mut conn, &created_by) {
        Ok(categories) => HttpResponse::Ok().json(ok_response(
            "Success get planned expense categories",
            Some(serde_json::to_value(categories).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError().json(err_response(
            "Failed to retrieve planned expense categories",
            err.to_string(),
        )),
    }
}

pub async fn post_planned_expense_category_api(
    req: HttpRequest,
    body: web::Json<PlannedExpenseCategoryInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let category = PlannedExpenseCategory {
        planned_expense_category_id: Uuid::new_v4(),
        planned_expense_category: body.planned_expense_category.clone(),
        created_date: Local::now().naive_local(),
        created_by,
        is_active: 1,
    };

    match insert_planned_expense_category(&mut conn, &category) {
        Ok(_) => HttpResponse::Ok().json(ok_response(
            "Planned expense category saved successfully",
            Some(serde_json::to_value(category).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError().json(err_response(
            "Failed to save planned expense category",
            err.to_string(),
        )),
    }
}

pub async fn delete_planned_expense_category_api(
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();

    match delete_planned_expense_category(&mut conn, &path.into_inner(), &created_by) {
        Ok(_) => HttpResponse::Ok().json(ok_response("Planned expense category removed", None)),
        Err(err) => HttpResponse::InternalServerError().json(err_response(
            "Failed to remove planned expense category",
            err.to_string(),
        )),
    }
}
