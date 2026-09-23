use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use chrono::Local;
use uuid::Uuid;

use crate::helper::connection::establish_connection_v2;
use crate::helper::settings_client::global_category_wiring;
use crate::models::member_transfer::{MemberTransfer, MemberTransferInput};
use crate::models::responses::Response;
use crate::repository::group_plan_repository::select_owned_source_name;
use crate::repository::member_transfer_repository::{
    TransferCategory, insert_member_transfer, select_member_sources, select_member_transfer,
    select_member_transfers, select_shared_member,
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

fn caller(req: &HttpRequest) -> String {
    req.extensions().get::<CreatedBy>().unwrap().0.clone()
}

/// `GET /api/user/member-sources/{username}` - the names of a group mate's
/// sources, so the sender can pick where a transfer lands. Balances are never
/// exposed, and only people who share a group with the caller can be read.
pub async fn get_member_sources_api(req: HttpRequest, path: web::Path<String>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let member = match select_shared_member(&mut conn, &username, path.into_inner().trim()) {
        Ok(Some(member)) => member,
        Ok(None) => {
            return HttpResponse::NotFound().json(err_response(
                "Member not found",
                "You can only transfer to people in one of your groups.".to_string(),
            ));
        }
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(err_response("Failed to read member", err.to_string()));
        }
    };
    match select_member_sources(&mut conn, &member) {
        Ok(items) => HttpResponse::Ok().json(ok_response(
            "Success get member sources",
            Some(serde_json::to_value(items).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError()
            .json(err_response("Failed to read member sources", err.to_string())),
    }
}

/// `GET /api/user/member-transfers` - transfers the caller sent or received.
pub async fn get_member_transfers_api(req: HttpRequest) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    match select_member_transfers(&mut conn, &caller(&req)) {
        Ok(items) => HttpResponse::Ok().json(ok_response(
            "Success get member transfers",
            Some(serde_json::to_value(items).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError()
            .json(err_response("Failed to retrieve member transfers", err.to_string())),
    }
}

/// `POST /api/user/member-transfers` - the caller sends money from one of
/// their own sources to one of a group mate's sources. Only the sender can
/// start a transfer; nothing can be pulled from someone else's source.
pub async fn post_member_transfer_api(
    req: HttpRequest,
    body: web::Json<MemberTransferInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    if body.amount <= 0.0 {
        return HttpResponse::BadRequest()
            .json(err_response("Amount must be positive", "".to_string()));
    }

    let transfer_id = body.transfer_id.unwrap_or_else(Uuid::new_v4);
    match select_member_transfer(&mut conn, transfer_id) {
        Ok(Some(existing)) if existing.from_user == username => {
            // A retry of a transfer that already went through.
            return HttpResponse::Ok().json(ok_response(
                "Transfer sent",
                Some(serde_json::to_value(existing).unwrap()),
            ));
        }
        Ok(Some(_)) => {
            return HttpResponse::Conflict()
                .json(err_response("Transfer id already used", "".to_string()));
        }
        Ok(None) => {}
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(err_response("Failed to send transfer", err.to_string()));
        }
    }

    let recipient = match select_shared_member(&mut conn, &username, body.to_username.trim()) {
        Ok(Some(member)) if member.eq_ignore_ascii_case(&username) => {
            return HttpResponse::BadRequest().json(err_response(
                "Choose someone else",
                "Use a normal transfer to move money between your own sources.".to_string(),
            ));
        }
        Ok(Some(member)) => member,
        Ok(None) => {
            return HttpResponse::NotFound().json(err_response(
                "Member not found",
                "You can only transfer to people in one of your groups.".to_string(),
            ));
        }
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(err_response("Failed to send transfer", err.to_string()));
        }
    };

    let from_source = match select_owned_source_name(&mut conn, body.from_source_id, &username) {
        Ok(Some(name)) => name,
        Ok(None) => {
            return HttpResponse::BadRequest().json(err_response(
                "Source not found",
                "Send from one of your own sources.".to_string(),
            ));
        }
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(err_response("Failed to send transfer", err.to_string()));
        }
    };
    let to_source = match select_owned_source_name(&mut conn, body.to_source_id, &recipient) {
        Ok(Some(name)) => name,
        Ok(None) => {
            return HttpResponse::BadRequest().json(err_response(
                "Destination source not found",
                format!("Pick one of {}'s sources.", recipient),
            ));
        }
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(err_response("Failed to send transfer", err.to_string()));
        }
    };

    let wiring = global_category_wiring().await;
    if wiring.transfer_category_id == Uuid::nil() {
        return HttpResponse::ServiceUnavailable().json(err_response(
            "Transfer category is not configured on the server",
            "".to_string(),
        ));
    }
    let category_name = if wiring.transfer_category_name.is_empty() {
        "Transfer".to_string()
    } else {
        wiring.transfer_category_name.clone()
    };

    let description = body.description.trim().to_string();
    let with_note = |text: String| {
        if description.is_empty() {
            text
        } else {
            format!("{text}: {description}")
        }
    };
    let sender_description = with_note(format!("Transfer to {recipient}"));
    let recipient_description = with_note(format!("Transfer from {username}"));

    let transfer = MemberTransfer {
        transfer_id,
        from_user: username,
        from_source_id: body.from_source_id,
        from_source,
        to_user: recipient,
        to_source_id: body.to_source_id,
        to_source,
        amount: body.amount,
        description,
        spending_id: Uuid::new_v4(),
        earning_id: Uuid::new_v4(),
        created_date: body
            .created_date
            .unwrap_or_else(|| Local::now().naive_local()),
    };
    match insert_member_transfer(
        &mut conn,
        &transfer,
        &TransferCategory {
            id: wiring.transfer_category_id,
            name: &category_name,
        },
        &sender_description,
        &recipient_description,
    ) {
        Ok(_) => HttpResponse::Ok().json(ok_response(
            "Transfer sent",
            Some(serde_json::to_value(transfer).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError()
            .json(err_response("Failed to send transfer", err.to_string())),
    }
}
