use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use chrono::Local;
use uuid::Uuid;

use crate::helper::connection::establish_connection_v2;
use crate::models::responses::Response;
use crate::models::routine::{
    RoutineInput, RoutinePayment, RoutinePaymentInput, RoutineTransaction,
};
use crate::repository::routine_repository::{
    insert_routine_payment, remove_routine, select_routine_payments, select_routines,
    upsert_routine,
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

pub async fn get_routines_api(req: HttpRequest) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();

    match select_routines(&mut conn, &created_by) {
        Ok(items) => HttpResponse::Ok().json(ok_response(
            "Success get routine transactions",
            Some(serde_json::to_value(items).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError().json(err_response(
            "Failed to retrieve routine transactions",
            err.to_string(),
        )),
    }
}

pub async fn get_routine_payments_api(req: HttpRequest) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();

    match select_routine_payments(&mut conn, &created_by) {
        Ok(items) => HttpResponse::Ok().json(ok_response(
            "Success get routine payments",
            Some(serde_json::to_value(items).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError().json(err_response(
            "Failed to retrieve routine payments",
            err.to_string(),
        )),
    }
}

pub async fn post_routine_api(req: HttpRequest, body: web::Json<RoutineInput>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let now = Local::now().naive_local();
    let item = RoutineTransaction {
        routine_id: body.routine_id.unwrap_or_else(Uuid::new_v4),
        item_name: body.item_name.clone(),
        price: body.price,
        reminder: body.reminder.clone(),
        spending_category_id: body.spending_category_id,
        spending_category: body.spending_category.clone(),
        status: "active".to_string(),
        last_bought_at: None,
        created_date: now,
        updated_date: now,
        created_by,
        is_active: 1,
    };

    match upsert_routine(&mut conn, &item) {
        Ok(_) => HttpResponse::Ok().json(ok_response(
            "Routine transaction saved successfully",
            Some(serde_json::to_value(item).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError().json(err_response(
            "Failed to save routine transaction",
            err.to_string(),
        )),
    }
}

pub async fn post_routine_payment_api(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<RoutinePaymentInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let routine_id = Uuid::parse_str(&path.into_inner()).unwrap_or_else(|_| Uuid::nil());
    let routines = match select_routines(&mut conn, &created_by) {
        Ok(items) => items,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(err_response("Failed to retrieve routine", err.to_string()));
        }
    };
    let routine = match routines.into_iter().find(|r| r.routine_id == routine_id) {
        Some(item) => item,
        None => {
            return HttpResponse::NotFound().json(err_response(
                "Routine transaction not found",
                "".to_string(),
            ));
        }
    };
    let payment = RoutinePayment {
        routine_payment_id: body.routine_payment_id.unwrap_or_else(Uuid::new_v4),
        routine_id,
        item_name: routine.item_name,
        price: body.price,
        spending_category_id: routine.spending_category_id,
        spending_category: routine.spending_category,
        source_id: body.source_id,
        source: body.source.clone(),
        bought_at: Local::now().naive_local(),
        created_by,
        is_active: 1,
    };

    match insert_routine_payment(&mut conn, &payment) {
        Ok(_) => HttpResponse::Ok().json(ok_response(
            "Routine payment saved successfully",
            Some(serde_json::to_value(payment).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError().json(err_response(
            "Failed to save routine payment",
            err.to_string(),
        )),
    }
}

pub async fn delete_routine_api(req: HttpRequest, path: web::Path<String>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();

    match remove_routine(&mut conn, &path.into_inner(), &created_by) {
        Ok(_) => HttpResponse::Ok().json(ok_response("Routine transaction removed", None)),
        Err(err) => HttpResponse::InternalServerError().json(err_response(
            "Failed to remove routine transaction",
            err.to_string(),
        )),
    }
}
