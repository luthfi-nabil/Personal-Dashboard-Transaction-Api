use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use chrono::Local;
use uuid::Uuid;

use crate::helper::connection::establish_connection_v2;
use crate::models::group_balance::{
    GroupBalanceEntry, GroupBalanceSpendingInput, GroupBalances, GroupTopUpInput,
};
use crate::models::responses::Response;
use crate::repository::group_balance_repository::{
    balance_spending, group_balance_entry_exists, insert_group_balance_spending,
    insert_group_top_up, select_group_balance_entries, select_group_balances,
};
use crate::repository::group_repository::{is_group_member, select_group_leader};
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

fn parse_id(raw: &str) -> Uuid {
    Uuid::parse_str(raw).unwrap_or_else(|_| Uuid::nil())
}

fn require_member(
    conn: &mut mysql::PooledConn,
    group_id: Uuid,
    username: &str,
) -> Result<(), HttpResponse> {
    match is_group_member(conn, group_id, username) {
        Ok(true) => Ok(()),
        Ok(false) => Err(HttpResponse::NotFound()
            .json(err_response("Group not found", "".to_string()))),
        Err(err) => Err(HttpResponse::InternalServerError()
            .json(err_response("Failed to read group", err.to_string()))),
    }
}

/// A retried write with the same id: fine when it is the caller's own entry,
/// a conflict otherwise.
fn replay(existing: GroupBalanceEntry, username: &str, message: &str) -> HttpResponse {
    if existing.username == username {
        HttpResponse::Ok().json(ok_response(
            message,
            Some(serde_json::to_value(existing).unwrap()),
        ))
    } else {
        HttpResponse::Conflict().json(err_response("Entry id already used", "".to_string()))
    }
}

/// `GET /api/user/groups/{group_id}/balances` - the leader sees every
/// member's balance and history; anyone else only their own.
pub async fn get_group_balances_api(req: HttpRequest, path: web::Path<String>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let group_id = parse_id(&path.into_inner());
    if let Err(response) = require_member(&mut conn, group_id, &username) {
        return response;
    }
    let is_leader = match select_group_leader(&mut conn, group_id) {
        Ok(leader) => leader.as_deref() == Some(username.as_str()),
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(err_response("Failed to read group", err.to_string()));
        }
    };
    let only_user = (!is_leader).then_some(username.as_str());

    let balances = match select_group_balances(&mut conn, group_id, only_user) {
        Ok(rows) => rows,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(err_response("Failed to read balances", err.to_string()));
        }
    };
    let entries = match select_group_balance_entries(&mut conn, group_id, only_user) {
        Ok(rows) => rows,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(err_response("Failed to read balance history", err.to_string()));
        }
    };
    HttpResponse::Ok().json(ok_response(
        "Success get group balances",
        Some(
            serde_json::to_value(GroupBalances {
                group_id,
                is_leader,
                balances,
                entries,
            })
            .unwrap(),
        ),
    ))
}

/// `POST /api/user/groups/{group_id}/balance/top-ups` - adds to the caller's
/// own group balance. The body has no username on purpose: nobody can top up
/// another member's balance.
pub async fn post_group_top_up_api(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<GroupTopUpInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let group_id = parse_id(&path.into_inner());
    if let Err(response) = require_member(&mut conn, group_id, &username) {
        return response;
    }
    if body.amount <= 0.0 {
        return HttpResponse::BadRequest()
            .json(err_response("Amount must be positive", "".to_string()));
    }
    let entry_id = body.entry_id.unwrap_or_else(Uuid::new_v4);
    match group_balance_entry_exists(&mut conn, entry_id) {
        Ok(Some(existing)) => return replay(existing, &username, "Balance added"),
        Ok(None) => {}
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(err_response("Failed to add balance", err.to_string()));
        }
    }

    let description = body.description.trim();
    let entry = GroupBalanceEntry {
        entry_id,
        group_id,
        username,
        amount: body.amount,
        entry_type: "top_up".to_string(),
        description: if description.is_empty() {
            "Add balance".to_string()
        } else {
            description.to_string()
        },
        spending_category_id: None,
        spending_category: None,
        created_date: Local::now().naive_local(),
    };
    match insert_group_top_up(&mut conn, &entry) {
        Ok(_) => HttpResponse::Ok().json(ok_response(
            "Balance added",
            Some(serde_json::to_value(entry).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError()
            .json(err_response("Failed to add balance", err.to_string())),
    }
}

/// `POST /api/user/groups/{group_id}/balance/spendings` - a group transaction
/// paid from the caller's group balance. It shows in the group's transaction
/// history; the caller's personal sources are untouched.
pub async fn post_group_balance_spending_api(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<GroupBalanceSpendingInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let group_id = parse_id(&path.into_inner());
    if let Err(response) = require_member(&mut conn, group_id, &username) {
        return response;
    }
    if body.amount <= 0.0 {
        return HttpResponse::BadRequest()
            .json(err_response("Amount must be positive", "".to_string()));
    }
    let entry_id = body.entry_id.unwrap_or_else(Uuid::new_v4);
    match group_balance_entry_exists(&mut conn, entry_id) {
        Ok(Some(existing)) => return replay(existing, &username, "Group transaction saved"),
        Ok(None) => {}
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(err_response("Failed to save group transaction", err.to_string()));
        }
    }

    let description = if body.description.trim().is_empty() {
        body.spending_category.clone()
    } else {
        body.description.trim().to_string()
    };
    let entry = balance_spending(
        entry_id,
        group_id,
        &username,
        body.amount,
        &description,
        body.spending_category_id,
        &body.spending_category,
        body.created_date
            .unwrap_or_else(|| Local::now().naive_local()),
    );
    match insert_group_balance_spending(&mut conn, &entry) {
        Ok(true) => HttpResponse::Ok().json(ok_response(
            "Group transaction saved",
            Some(serde_json::to_value(entry).unwrap()),
        )),
        Ok(false) => HttpResponse::Conflict().json(err_response(
            "Not enough group balance",
            "Add balance first.".to_string(),
        )),
        Err(err) => HttpResponse::InternalServerError()
            .json(err_response("Failed to save group transaction", err.to_string())),
    }
}
