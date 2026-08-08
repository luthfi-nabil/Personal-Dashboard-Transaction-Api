use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use chrono::Local;
use uuid::Uuid;

use crate::helper::connection::establish_connection_v2;
use crate::models::consumable::{Consumable, ConsumableInput, ConsumableOutInput, parse_date};
use crate::models::responses::Response;
use crate::repository::consumable_repository::{
    remove_consumable, select_consumables, update_consumable_out_date, upsert_consumable,
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

pub async fn get_consumables_api(req: HttpRequest) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();

    match select_consumables(&mut conn, &created_by) {
        Ok(items) => HttpResponse::Ok().json(ok_response(
            "Success get consumables",
            Some(serde_json::to_value(items).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError()
            .json(err_response("Failed to retrieve consumables", err.to_string())),
    }
}

/// `POST /api/user/consumables`
///
/// One call creates one unit. A multi-pack is posted as several calls, each
/// with its own id and `unit_index`, so every unit can be finished on its own
/// date.
pub async fn post_consumable_api(
    req: HttpRequest,
    body: web::Json<ConsumableInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let now = Local::now().naive_local();

    let name = body.item_name.trim();
    if name.is_empty() {
        return HttpResponse::BadRequest().json(err_response(
            "Invalid consumable",
            "item_name is required".to_string(),
        ));
    }

    let item = Consumable {
        consumable_id: body.consumable_id.unwrap_or_else(Uuid::new_v4),
        item_name: name.to_string(),
        notes: body.notes.clone(),
        unit_index: if body.unit_index < 1 { 1 } else { body.unit_index },
        unit_total: if body.unit_total < 1 { 1 } else { body.unit_total },
        price: body.price,
        // An unreadable or absent date means "it came in now", which is right
        // for a unit added by hand and harmless for one built from a receipt.
        in_date: parse_date(&body.in_date).unwrap_or(now),
        out_date: parse_date(&body.out_date),
        spending_id: body.spending_id,
        spending_detail_id: body.spending_detail_id,
        created_date: now,
        updated_date: now,
        created_by,
        is_active: 1,
    };

    match upsert_consumable(&mut conn, &item) {
        Ok(_) => HttpResponse::Ok().json(ok_response(
            "Consumable saved",
            Some(serde_json::to_value(item).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError()
            .json(err_response("Failed to save consumable", err.to_string())),
    }
}

/// `PUT /api/user/consumables/{consumable_id}/out`
///
/// Records the date a unit ran out. Posting `{"out_date": null}` puts it back
/// in use, which is the undo for a mistaken tap.
pub async fn put_consumable_out_api(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<ConsumableOutInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let out_date = parse_date(&body.out_date);
    if body.out_date.is_some() && out_date.is_none() {
        return HttpResponse::BadRequest().json(err_response(
            "Invalid out date",
            "out_date could not be read as a date".to_string(),
        ));
    }

    match update_consumable_out_date(&mut conn, &path.into_inner(), &created_by, out_date) {
        Ok(0) => HttpResponse::NotFound().json(err_response(
            "Consumable not found",
            "No consumable with that id belongs to this user".to_string(),
        )),
        Ok(_) => HttpResponse::Ok().json(ok_response("Consumable updated", None)),
        Err(err) => HttpResponse::InternalServerError()
            .json(err_response("Failed to update consumable", err.to_string())),
    }
}

pub async fn delete_consumable_api(req: HttpRequest, path: web::Path<String>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();

    match remove_consumable(&mut conn, &path.into_inner(), &created_by) {
        Ok(_) => HttpResponse::Ok().json(ok_response("Consumable removed", None)),
        Err(err) => HttpResponse::InternalServerError()
            .json(err_response("Failed to remove consumable", err.to_string())),
    }
}
