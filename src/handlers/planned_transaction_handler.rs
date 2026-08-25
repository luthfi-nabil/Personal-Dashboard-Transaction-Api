use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use chrono::Local;
use uuid::Uuid;

use crate::helper::connection::establish_connection_v2;
use crate::models::planned_transaction::{
    PlannedTransaction, PlannedTransactionDetail, PlannedTransactionDetailInput,
    PlannedTransactionDetailQuery, PlannedTransactionInput,
};
use crate::models::responses::Response;
use crate::repository::planned_transaction_repository::{
    planned_transaction_exists, select_planned_transaction_details, select_planned_transactions,
    upsert_planned_transaction, upsert_planned_transaction_detail,
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

pub async fn get_planned_transactions_api(req: HttpRequest) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();

    match select_planned_transactions(&mut conn, &created_by) {
        Ok(items) => HttpResponse::Ok().json(ok_response(
            "Success get planned transactions",
            Some(serde_json::to_value(items).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError().json(err_response(
            "Failed to retrieve planned transactions",
            err.to_string(),
        )),
    }
}

/// `POST /api/user/planned-transactions`. Posting an id that already exists
/// updates its name, so a write queued offline can be retried safely.
pub async fn post_planned_transaction_api(
    req: HttpRequest,
    body: web::Json<PlannedTransactionInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let name = body.name.trim();
    if name.is_empty() {
        return HttpResponse::BadRequest()
            .json(err_response("Invalid planned transaction", "name is required".to_string()));
    }
    let now = body
        .created_date
        .unwrap_or_else(|| Local::now().naive_local());

    let item = PlannedTransaction {
        planned_transaction_id: body.planned_transaction_id.unwrap_or_else(Uuid::new_v4),
        name: name.to_string(),
        created_date: now,
        updated_date: Local::now().naive_local(),
        created_by,
        is_active: 1,
    };

    match upsert_planned_transaction(&mut conn, &item) {
        Ok(_) => HttpResponse::Ok().json(ok_response(
            "Planned transaction saved",
            Some(serde_json::to_value(item).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError().json(err_response(
            "Failed to save planned transaction",
            err.to_string(),
        )),
    }
}

/// `GET /api/user/planned-transaction-details[?planned_transaction_id=...]`
pub async fn get_planned_transaction_details_api(
    req: HttpRequest,
    query: web::Query<PlannedTransactionDetailQuery>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();

    match select_planned_transaction_details(&mut conn, query.planned_transaction_id, &created_by)
    {
        Ok(items) => HttpResponse::Ok().json(ok_response(
            "Success get planned transaction details",
            Some(serde_json::to_value(items).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError().json(err_response(
            "Failed to retrieve planned transaction details",
            err.to_string(),
        )),
    }
}

/// `POST /api/user/planned-transactions/{planned_transaction_id}/details`
pub async fn post_planned_transaction_detail_api(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<PlannedTransactionDetailInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let planned_transaction_id = match Uuid::parse_str(&path.into_inner()) {
        Ok(id) => id,
        Err(err) => {
            return HttpResponse::BadRequest().json(err_response(
                "Invalid planned transaction id",
                err.to_string(),
            ));
        }
    };

    match planned_transaction_exists(&mut conn, planned_transaction_id, &created_by) {
        Ok(false) => {
            return HttpResponse::NotFound().json(err_response(
                "Planned transaction not found",
                "No planned transaction with that id belongs to this user".to_string(),
            ));
        }
        Err(err) => {
            return HttpResponse::InternalServerError().json(err_response(
                "Failed to look up planned transaction",
                err.to_string(),
            ));
        }
        Ok(true) => {}
    }

    let name = body.item_name.trim();
    if name.is_empty() {
        return HttpResponse::BadRequest().json(err_response(
            "Invalid planned transaction detail",
            "item_name is required".to_string(),
        ));
    }
    let amount = if body.amount != 0.0 {
        body.amount
    } else {
        body.quantity * body.unit_price
    };
    let now = body
        .created_date
        .unwrap_or_else(|| Local::now().naive_local());

    let item = PlannedTransactionDetail {
        planned_transaction_detail_id: body
            .planned_transaction_detail_id
            .unwrap_or_else(Uuid::new_v4),
        planned_transaction_id,
        item_name: name.to_string(),
        quantity: if body.quantity == 0.0 { 1.0 } else { body.quantity },
        unit_price: body.unit_price,
        amount,
        note: body.note.clone(),
        spending_id: body.spending_id,
        spending_detail_id: body.spending_detail_id,
        created_date: now,
        created_by,
        is_active: 1,
    };

    match upsert_planned_transaction_detail(&mut conn, &item) {
        Ok(_) => HttpResponse::Ok().json(ok_response(
            "Planned transaction detail saved",
            Some(serde_json::to_value(item).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError().json(err_response(
            "Failed to save planned transaction detail",
            err.to_string(),
        )),
    }
}
