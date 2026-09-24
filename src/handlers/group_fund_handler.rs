use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use chrono::Local;
use mysql::PooledConn;
use uuid::Uuid;

use crate::helper::connection::establish_connection_v2;
use crate::helper::settings_client::global_category_wiring;
use crate::models::group_fund::{
    FundFulfilInput, FundRequestInput, FundRequestsView, FundWaiveInput, GroupFundRequest,
};
use crate::models::member_transfer::MemberTransfer;
use crate::models::responses::Response;
use crate::repository::group_fund_repository::{
    FundOutcome, FundTexts, close_fund_request, fulfil_fund_request, insert_fund_request,
    insert_sent_fund, select_fund_request, select_fund_requests_for_user,
    select_group_member_name, tag_balances, waive_fund_request,
};
use crate::repository::group_plan_repository::select_owned_source_name;
use crate::repository::group_repository::{is_group_member, select_group_leader};
use crate::repository::member_transfer_repository::TransferCategory;
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

fn server_error(message: &str, err: impl ToString) -> HttpResponse {
    HttpResponse::InternalServerError().json(err_response(message, err.to_string()))
}

/// 404 unless the caller is in `group_id`; otherwise whether they lead it.
fn membership(conn: &mut PooledConn, group_id: Uuid, username: &str) -> Result<bool, HttpResponse> {
    match is_group_member(conn, group_id, username) {
        Ok(true) => {}
        Ok(false) => {
            return Err(HttpResponse::NotFound()
                .json(err_response("Group not found", "".to_string())));
        }
        Err(err) => return Err(server_error("Failed to read group", err)),
    }
    match select_group_leader(conn, group_id) {
        Ok(Some(leader)) => Ok(leader.eq_ignore_ascii_case(username)),
        Ok(None) => Err(HttpResponse::NotFound().json(err_response("Group not found", "".to_string()))),
        Err(err) => Err(server_error("Failed to read group", err)),
    }
}

/// `username`'s own source `source_id`, by name.
fn own_source(
    conn: &mut PooledConn,
    source_id: Uuid,
    username: &str,
    missing: &str,
) -> Result<String, HttpResponse> {
    match select_owned_source_name(conn, source_id, username) {
        Ok(Some(name)) => Ok(name),
        Ok(None) => Err(HttpResponse::BadRequest().json(err_response(missing, "".to_string()))),
        Err(err) => Err(server_error("Failed to read source", err)),
    }
}

/// The Transfer category both halves of a fund transfer are filed under, so
/// the payer's side reads as a transfer rather than an expense.
async fn transfer_category() -> Result<(Uuid, String), HttpResponse> {
    let wiring = global_category_wiring().await;
    if wiring.transfer_category_id == Uuid::nil() {
        return Err(HttpResponse::ServiceUnavailable().json(err_response(
            "Transfer category is not configured on the server",
            "".to_string(),
        )));
    }
    let name = if wiring.transfer_category_name.is_empty() {
        "Transfer".to_string()
    } else {
        wiring.transfer_category_name.clone()
    };
    Ok((wiring.transfer_category_id, name))
}

fn fund_texts(r: &GroupFundRequest) -> FundTexts {
    let with_note = |text: String| {
        if r.note.trim().is_empty() {
            text
        } else {
            format!("{text}: {}", r.note.trim())
        }
    };
    FundTexts {
        payer: with_note(format!("Funds to {} for {}", r.requester, r.tag)),
        receiver: with_note(format!("Funds from {} for {}", r.payer, r.tag)),
    }
}

fn fund_transfer(
    r: &GroupFundRequest,
    from: (Uuid, String),
    to: (Uuid, String),
    at: chrono::NaiveDateTime,
) -> MemberTransfer {
    MemberTransfer {
        transfer_id: Uuid::new_v4(),
        from_user: r.payer.clone(),
        from_source_id: from.0,
        from_source: from.1,
        to_user: r.requester.clone(),
        to_source_id: to.0,
        to_source: to.1,
        amount: r.amount,
        description: format!("{} funds{}", r.tag, if r.note.is_empty() { String::new() } else { format!(": {}", r.note) }),
        spending_id: Uuid::new_v4(),
        earning_id: Uuid::new_v4(),
        created_date: at,
    }
}

fn fund_json(conn: &mut PooledConn, username: &str, request_id: Uuid) -> Option<serde_json::Value> {
    // Re-read through the list query so usage is filled in the same way.
    select_fund_requests_for_user(conn, username)
        .ok()
        .and_then(|rows| rows.into_iter().find(|r| r.request_id == request_id))
        .map(|r| serde_json::to_value(r).unwrap())
}

/// `GET /api/user/fund-requests` - the requests the caller may see in all
/// their groups (leader: everything in their groups; member: the ones they
/// asked or were asked), plus the tag balances of the tracked ones.
pub async fn get_fund_requests_api(req: HttpRequest) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    match select_fund_requests_for_user(&mut conn, &caller(&req)) {
        Ok(requests) => {
            let balances = tag_balances(&requests);
            HttpResponse::Ok().json(ok_response(
                "Success get fund requests",
                Some(serde_json::to_value(FundRequestsView { requests, balances }).unwrap()),
            ))
        }
        Err(err) => server_error("Failed to retrieve fund requests", err),
    }
}

/// `POST /api/user/groups/{group_id}/fund-requests` - ask a group mate for
/// money (`kind: request`) or send them money now (`kind: send`), for a
/// tag, optionally tracked.
pub async fn post_fund_request_api(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<FundRequestInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let group_id = parse_id(&path.into_inner());
    if let Err(resp) = membership(&mut conn, group_id, &username) {
        return resp;
    }
    let kind = body.kind.trim().to_lowercase();
    if kind != "request" && kind != "send" {
        return HttpResponse::BadRequest()
            .json(err_response("kind must be request or send", "".to_string()));
    }
    if body.amount <= 0.0 {
        return HttpResponse::BadRequest().json(err_response("Amount must be positive", "".to_string()));
    }
    let tag = body.tag.trim().to_string();
    if tag.is_empty() {
        return HttpResponse::BadRequest()
            .json(err_response("Pick a tag (what the money is for)", "".to_string()));
    }

    let request_id = body.request_id.unwrap_or_else(Uuid::new_v4);
    match select_fund_request(&mut conn, request_id) {
        Ok(Some(existing)) if existing.created_by == username && existing.group_id == group_id => {
            // A retry of a request that already went through.
            return HttpResponse::Ok().json(ok_response(
                "Fund request saved",
                fund_json(&mut conn, &username, request_id),
            ));
        }
        Ok(Some(_)) => {
            return HttpResponse::Conflict().json(err_response("Request id already used", "".to_string()));
        }
        Ok(None) => {}
        Err(err) => return server_error("Failed to save fund request", err),
    }

    let other = match select_group_member_name(&mut conn, group_id, body.username.trim()) {
        Ok(Some(name)) if name.eq_ignore_ascii_case(&username) => {
            return HttpResponse::BadRequest().json(err_response("Choose someone else", "".to_string()));
        }
        Ok(Some(name)) => name,
        Ok(None) => {
            return HttpResponse::NotFound().json(err_response(
                "Member not found",
                "They must be a member of this group.".to_string(),
            ));
        }
        Err(err) => return server_error("Failed to read group", err),
    };

    let now = Local::now().naive_local();
    let (requester, payer) = if kind == "request" {
        (username.clone(), other)
    } else {
        (other, username.clone())
    };
    let mut fund = GroupFundRequest {
        request_id,
        group_id,
        kind: kind.clone(),
        requester,
        payer,
        amount: body.amount,
        tag,
        note: body.note.trim().to_string(),
        tracked: body.tracked,
        status: "requested".to_string(),
        to_source_id: None,
        to_source: None,
        from_source_id: None,
        from_source: None,
        transfer_id: None,
        created_by: username.clone(),
        created_date: now,
        responded_at: None,
        sent_at: None,
        waived_by: None,
        waived_at: None,
        waive_note: None,
        spent: 0.0,
        remaining: 0.0,
        waived_amount: 0.0,
        usage: Vec::new(),
    };

    if kind == "request" {
        // Where the money should land: optional, one of the caller's own.
        if let Some(to_id) = body.to_source_id {
            match own_source(&mut conn, to_id, &username, "Receive into one of your own sources") {
                Ok(name) => {
                    fund.to_source_id = Some(to_id);
                    fund.to_source = Some(name);
                }
                Err(resp) => return resp,
            }
        }
        return match insert_fund_request(&mut conn, &fund) {
            Ok(_) => HttpResponse::Ok().json(ok_response(
                "Fund request sent",
                fund_json(&mut conn, &username, request_id),
            )),
            Err(err) => server_error("Failed to save fund request", err),
        };
    }

    // kind == "send": the money moves now.
    let (Some(from_id), Some(to_id)) = (body.from_source_id, body.to_source_id) else {
        return HttpResponse::BadRequest().json(err_response(
            "Choose where the money comes from and where it goes",
            "Send from_source_id (yours) and to_source_id (theirs).".to_string(),
        ));
    };
    let from_name = match own_source(&mut conn, from_id, &fund.payer, "Send from one of your own sources") {
        Ok(name) => name,
        Err(resp) => return resp,
    };
    let to_name = match own_source(
        &mut conn,
        to_id,
        &fund.requester,
        &format!("Pick one of {}'s sources", fund.requester),
    ) {
        Ok(name) => name,
        Err(resp) => return resp,
    };
    let (cat_id, cat_name) = match transfer_category().await {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let transfer = fund_transfer(&fund, (from_id, from_name), (to_id, to_name), now);
    fund.status = "sent".to_string();
    fund.from_source_id = Some(transfer.from_source_id);
    fund.from_source = Some(transfer.from_source.clone());
    fund.to_source_id = Some(transfer.to_source_id);
    fund.to_source = Some(transfer.to_source.clone());
    fund.transfer_id = Some(transfer.transfer_id);
    fund.responded_at = Some(now);
    fund.sent_at = Some(now);
    let texts = fund_texts(&fund);
    match insert_sent_fund(
        &mut conn,
        &fund,
        &transfer,
        &TransferCategory { id: cat_id, name: &cat_name },
        &texts,
    ) {
        Ok(_) => HttpResponse::Ok().json(ok_response(
            "Funds sent",
            fund_json(&mut conn, &username, request_id),
        )),
        Err(err) => server_error("Failed to send funds", err),
    }
}

fn load_in_group(
    conn: &mut PooledConn,
    group_id: Uuid,
    request_id: Uuid,
) -> Result<GroupFundRequest, HttpResponse> {
    match select_fund_request(conn, request_id) {
        Ok(Some(r)) if r.group_id == group_id => Ok(r),
        Ok(_) => Err(HttpResponse::NotFound().json(err_response("Request not found", "".to_string()))),
        Err(err) => Err(server_error("Failed to read request", err)),
    }
}

fn outcome_response(outcome: FundOutcome, done: HttpResponse) -> HttpResponse {
    match outcome {
        FundOutcome::Done => done,
        FundOutcome::Gone => HttpResponse::NotFound().json(err_response("Request not found", "".to_string())),
        FundOutcome::WrongStatus(status) => HttpResponse::Conflict().json(err_response(
            "This request can no longer be changed",
            format!("It is {status}."),
        )),
    }
}

/// `PUT /api/user/groups/{group_id}/fund-requests/{request_id}/fulfill` -
/// the payer sends a waiting request from one of their own sources.
pub async fn put_fund_request_fulfill_api(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Json<FundFulfilInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let (group_raw, request_raw) = path.into_inner();
    let (group_id, request_id) = (parse_id(&group_raw), parse_id(&request_raw));
    if let Err(resp) = membership(&mut conn, group_id, &username) {
        return resp;
    }
    let fund = match load_in_group(&mut conn, group_id, request_id) {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    if !fund.payer.eq_ignore_ascii_case(&username) {
        return HttpResponse::Forbidden().json(err_response(
            "Only the person asked can send it",
            "".to_string(),
        ));
    }
    if fund.status != "requested" {
        return outcome_response(FundOutcome::WrongStatus(fund.status), HttpResponse::Ok().finish());
    }
    let from_name = match own_source(&mut conn, body.from_source_id, &username, "Send from one of your own sources") {
        Ok(name) => name,
        Err(resp) => return resp,
    };
    let to_id = match fund.to_source_id.or(body.to_source_id) {
        Some(id) => id,
        None => {
            return HttpResponse::BadRequest().json(err_response(
                "Choose which of their sources receives it",
                "".to_string(),
            ));
        }
    };
    let to_name = match own_source(
        &mut conn,
        to_id,
        &fund.requester,
        &format!("Pick one of {}'s sources", fund.requester),
    ) {
        Ok(name) => name,
        Err(resp) => return resp,
    };
    let (cat_id, cat_name) = match transfer_category().await {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let now = Local::now().naive_local();
    let transfer = fund_transfer(&fund, (body.from_source_id, from_name), (to_id, to_name), now);
    let texts = fund_texts(&fund);
    match fulfil_fund_request(
        &mut conn,
        request_id,
        &transfer,
        &TransferCategory { id: cat_id, name: &cat_name },
        &texts,
    ) {
        Ok(outcome) => {
            let data = fund_json(&mut conn, &username, request_id);
            outcome_response(outcome, HttpResponse::Ok().json(ok_response("Funds sent", data)))
        }
        Err(err) => server_error("Failed to send funds", err),
    }
}

/// `PUT /api/user/groups/{group_id}/fund-requests/{request_id}/reject` -
/// the payer turns a waiting request down.
pub async fn put_fund_request_reject_api(
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    close_request(req, path, "rejected").await
}

/// `DELETE /api/user/groups/{group_id}/fund-requests/{request_id}` - the
/// requester withdraws a request that is still waiting.
pub async fn delete_fund_request_api(req: HttpRequest, path: web::Path<(String, String)>) -> HttpResponse {
    close_request(req, path, "canceled").await
}

async fn close_request(req: HttpRequest, path: web::Path<(String, String)>, status: &str) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let (group_raw, request_raw) = path.into_inner();
    let (group_id, request_id) = (parse_id(&group_raw), parse_id(&request_raw));
    if let Err(resp) = membership(&mut conn, group_id, &username) {
        return resp;
    }
    let fund = match load_in_group(&mut conn, group_id, request_id) {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    let allowed = if status == "rejected" {
        fund.payer.eq_ignore_ascii_case(&username)
    } else {
        fund.requester.eq_ignore_ascii_case(&username)
    };
    if !allowed {
        return HttpResponse::Forbidden().json(err_response(
            if status == "rejected" {
                "Only the person asked can reject it"
            } else {
                "Only the person who asked can withdraw it"
            },
            "".to_string(),
        ));
    }
    match close_fund_request(&mut conn, request_id, status, Local::now().naive_local()) {
        Ok(outcome) => {
            let data = fund_json(&mut conn, &username, request_id);
            outcome_response(outcome, HttpResponse::Ok().json(ok_response("Request updated", data)))
        }
        Err(err) => server_error("Failed to update request", err),
    }
}

/// `PUT /api/user/groups/{group_id}/fund-requests/{request_id}/waive` -
/// group leader only. The unspent part of a sent, tracked fund is dropped
/// from the recipient's tag balance; the transfer and their spendings stay.
pub async fn put_fund_request_waive_api(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    body: Option<web::Json<FundWaiveInput>>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let (group_raw, request_raw) = path.into_inner();
    let (group_id, request_id) = (parse_id(&group_raw), parse_id(&request_raw));
    match membership(&mut conn, group_id, &username) {
        Ok(true) => {}
        Ok(false) => {
            return HttpResponse::Forbidden().json(err_response(
                "Only the group admin can waive a fund",
                "".to_string(),
            ));
        }
        Err(resp) => return resp,
    }
    if let Err(resp) = load_in_group(&mut conn, group_id, request_id) {
        return resp;
    }
    let note = body.map(|b| b.into_inner().note).unwrap_or_default();
    match waive_fund_request(&mut conn, request_id, &username, note.trim(), Local::now().naive_local()) {
        Ok(outcome) => {
            let data = fund_json(&mut conn, &username, request_id);
            outcome_response(outcome, HttpResponse::Ok().json(ok_response("Fund waived", data)))
        }
        Err(err) => server_error("Failed to waive fund", err),
    }
}
