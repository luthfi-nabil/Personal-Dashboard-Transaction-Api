use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use chrono::Local;
use uuid::Uuid;

use crate::helper::connection::establish_connection_v2;
use crate::models::consumable::parse_date;
use crate::models::investment::{
    INVESTMENT_KINDS, Investment, InvestmentInput, InvestmentPriceInput,
};
use crate::models::responses::Response;
use crate::repository::investment_repository::{
    remove_investment, select_investments, update_investment_price, upsert_investment,
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

pub async fn get_investments_api(req: HttpRequest) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();

    match select_investments(&mut conn, &created_by) {
        Ok(items) => HttpResponse::Ok().json(ok_response(
            "Success get investments",
            Some(serde_json::to_value(items).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError()
            .json(err_response("Failed to retrieve investments", err.to_string())),
    }
}

/// `POST /api/user/investments`
///
/// Creates or edits one holding, keyed on the client-generated
/// `investment_id`, so a write queued while the API was unreachable can be
/// replayed safely.
pub async fn post_investment_api(
    req: HttpRequest,
    body: web::Json<InvestmentInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let now = Local::now().naive_local();

    let name = body.name.trim();
    if name.is_empty() {
        return HttpResponse::BadRequest().json(err_response(
            "Invalid investment",
            "name is required".to_string(),
        ));
    }

    let kind = body.kind.trim().to_lowercase();
    if !INVESTMENT_KINDS.contains(&kind.as_str()) {
        return HttpResponse::BadRequest().json(err_response(
            "Invalid investment",
            format!("kind must be one of {}", INVESTMENT_KINDS.join(", ")),
        ));
    }

    if body.units < 0.0 || body.buy_unit_price < 0.0 {
        return HttpResponse::BadRequest().json(err_response(
            "Invalid investment",
            "units and buy_unit_price cannot be negative".to_string(),
        ));
    }

    // A holding recorded without a current price is worth what it cost until
    // the first refresh lands, which beats showing a 100% loss.
    let last_unit_price = match body.last_unit_price {
        Some(price) if price > 0.0 => price,
        _ => body.buy_unit_price,
    };

    let item = Investment {
        investment_id: body.investment_id.unwrap_or_else(Uuid::new_v4),
        kind,
        name: name.to_string(),
        provider: body.provider.trim().to_string(),
        units: body.units,
        buy_unit_price: body.buy_unit_price,
        last_unit_price,
        price_source: body.price_source.trim().to_string(),
        price_updated_date: parse_date(&body.price_updated_date),
        notes: body.notes.clone(),
        // An unreadable or absent date means "bought now", which is right for
        // a holding added by hand today.
        acquired_date: parse_date(&body.acquired_date).unwrap_or(now),
        created_date: now,
        updated_date: now,
        created_by,
        is_active: 1,
    };

    match upsert_investment(&mut conn, &item) {
        Ok(_) => HttpResponse::Ok().json(ok_response(
            "Investment saved",
            Some(serde_json::to_value(item).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError()
            .json(err_response("Failed to save investment", err.to_string())),
    }
}

/// `PUT /api/user/investments/{investment_id}/price`
///
/// Records a fresh valuation - a NAB the user read off their broker app, or a
/// gold/silver price the client pulled from a price API. Units and cost basis
/// are untouchable through this route on purpose: a bad price refresh should
/// never be able to rewrite what was actually bought.
pub async fn put_investment_price_api(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<InvestmentPriceInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();

    if body.last_unit_price < 0.0 {
        return HttpResponse::BadRequest().json(err_response(
            "Invalid price",
            "last_unit_price cannot be negative".to_string(),
        ));
    }

    let price_updated = parse_date(&body.price_updated_date).unwrap_or(Local::now().naive_local());

    match update_investment_price(
        &mut conn,
        &path.into_inner(),
        &created_by,
        body.last_unit_price,
        body.price_source.trim(),
        price_updated,
    ) {
        Ok(0) => HttpResponse::NotFound().json(err_response(
            "Investment not found",
            "No investment with that id belongs to this user".to_string(),
        )),
        Ok(_) => HttpResponse::Ok().json(ok_response("Investment price updated", None)),
        Err(err) => HttpResponse::InternalServerError()
            .json(err_response("Failed to update investment price", err.to_string())),
    }
}

pub async fn delete_investment_api(req: HttpRequest, path: web::Path<String>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();

    match remove_investment(&mut conn, &path.into_inner(), &created_by) {
        Ok(_) => HttpResponse::Ok().json(ok_response("Investment removed", None)),
        Err(err) => HttpResponse::InternalServerError()
            .json(err_response("Failed to remove investment", err.to_string())),
    }
}
