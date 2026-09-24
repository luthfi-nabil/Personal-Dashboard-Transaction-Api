use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use chrono::Local;
use mysql::PooledConn;
use uuid::Uuid;

use crate::helper::connection::establish_connection_v2;
use crate::helper::settings_client::global_category_wiring;
use crate::models::group_settlement::{
    GroupReimbursement, GroupReimbursementInput, GroupSettlements, GroupSplitManualPaymentInput,
    GroupSplitPayment, GroupSplitPaymentInput, GroupSplitReviewInput, GroupSplitShare,
    GroupSplitShareInput,
};
use crate::models::responses::Response;
use crate::repository::group_plan_repository::select_owned_source_name;
use crate::repository::group_repository::is_group_member;
use crate::repository::group_settlement_repository::{
    LegTexts, SettleOutcome, Source, approve_split_payment, close_split_payment,
    delete_group_contact_name, insert_manual_split_payment, insert_reimbursement,
    insert_split_payment_request, insert_split_share, remove_split_share,
    select_group_contact_names, select_group_member_name, select_group_reimbursements,
    select_group_spending, select_group_split_payments, select_group_split_shares,
    select_reimbursement, select_split_payment, select_split_share, split_share_id_owner,
};
use crate::repository::member_transfer_repository::TransferCategory;
use crate::route_middleware::get_user::CreatedBy;

/// Shown as the "source" of money taken from a group balance.
const GROUP_BALANCE_SOURCE: &str = "Group balance";

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

fn bad_request(message: &str, description: &str) -> HttpResponse {
    HttpResponse::BadRequest().json(err_response(message, description.to_string()))
}

fn forbidden(message: &str) -> HttpResponse {
    HttpResponse::Forbidden().json(err_response(message, "".to_string()))
}

fn ok_json(message: &str, value: impl serde::Serialize) -> HttpResponse {
    HttpResponse::Ok().json(ok_response(
        message,
        Some(serde_json::to_value(value).unwrap()),
    ))
}

fn require_member(conn: &mut PooledConn, group_id: Uuid, username: &str) -> Result<(), HttpResponse> {
    match is_group_member(conn, group_id, username) {
        Ok(true) => Ok(()),
        Ok(false) => Err(HttpResponse::NotFound()
            .json(err_response("Group not found", "".to_string()))),
        Err(err) => Err(server_error("Failed to read group", err)),
    }
}

/// The server's Transfer category; the personal-source side of a settlement
/// is filed under it, like a transfer between members.
async fn transfer_category() -> Option<(Uuid, String)> {
    let wiring = global_category_wiring().await;
    if wiring.transfer_category_id == Uuid::nil() {
        return None;
    }
    let name = if wiring.transfer_category_name.is_empty() {
        "Transfer".to_string()
    } else {
        wiring.transfer_category_name.clone()
    };
    Some((wiring.transfer_category_id, name))
}

fn no_transfer_category() -> HttpResponse {
    HttpResponse::ServiceUnavailable().json(err_response(
        "Transfer category is not configured on the server",
        "Use a group balance instead, or ask the admin to set the Transfer category.".to_string(),
    ))
}

/// `source_id` checked to be one of `username`'s own active sources.
fn owned_source(
    conn: &mut PooledConn,
    source_id: Uuid,
    username: &str,
    not_found: &str,
) -> Result<Source, HttpResponse> {
    match select_owned_source_name(conn, source_id, username) {
        Ok(Some(name)) => Ok((source_id, name)),
        Ok(None) => Err(bad_request("Source not found", not_found)),
        Err(err) => Err(server_error("Failed to read source", err)),
    }
}

/// Where a payer's money comes from: `Some(source)` or `None` for their group
/// balance.
fn payer_source(
    conn: &mut PooledConn,
    source_id: Option<Uuid>,
    from_group_balance: bool,
    username: &str,
) -> Result<Option<Source>, HttpResponse> {
    if from_group_balance {
        return Ok(None);
    }
    let Some(source_id) = source_id else {
        return Err(bad_request(
            "Choose where the money comes from",
            "Send a from_source_id or set from_group_balance.",
        ));
    };
    owned_source(conn, source_id, username, "Pay from one of your own sources.").map(Some)
}

/// Where the owner takes the money: `Some(source)` or `None` for their group
/// balance.
fn owner_destination(
    conn: &mut PooledConn,
    source_id: Option<Uuid>,
    to_group_balance: bool,
    owner: &str,
) -> Result<Option<Source>, HttpResponse> {
    if to_group_balance {
        return Ok(None);
    }
    let Some(source_id) = source_id else {
        return Err(bad_request(
            "Choose where the money goes",
            "Send a to_source_id or set to_group_balance.",
        ));
    };
    owned_source(conn, source_id, owner, "Pick one of your own sources.").map(Some)
}

/// `"<text>: <spending description>"`, or just `<text>`.
fn about(text: String, spending: &str) -> String {
    if spending.trim().is_empty() {
        text
    } else {
        format!("{text}: {}", spending.trim())
    }
}

fn outcome_error(outcome: SettleOutcome) -> HttpResponse {
    match outcome {
        SettleOutcome::Done => HttpResponse::Ok().finish(),
        SettleOutcome::SpendingGone => HttpResponse::NotFound()
            .json(err_response("Transaction not found", "".to_string())),
        SettleOutcome::OverLimit(left) => HttpResponse::Conflict().json(err_response(
            "Amount is more than what is left",
            format!("Only {left} is left to settle."),
        )),
        SettleOutcome::NotEnoughBalance => HttpResponse::Conflict().json(err_response(
            "Not enough group balance",
            "The payer's group balance does not cover this amount.".to_string(),
        )),
        SettleOutcome::NotPending => HttpResponse::Conflict().json(err_response(
            "Payment is no longer pending",
            "".to_string(),
        )),
    }
}

// ── Read ─────────────────────────────────────────────────────────────────────

/// `GET /api/user/groups/{group_id}/settlements` - every reimbursement, split
/// share and split payment of the group, and its saved names. Any member can
/// read them, like the group's transaction history.
pub async fn get_group_settlements_api(req: HttpRequest, path: web::Path<String>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let group_id = parse_id(&path.into_inner());
    if let Err(response) = require_member(&mut conn, group_id, &username) {
        return response;
    }
    let reimbursements = match select_group_reimbursements(&mut conn, group_id) {
        Ok(rows) => rows,
        Err(err) => return server_error("Failed to read reimbursements", err),
    };
    let shares = match select_group_split_shares(&mut conn, group_id) {
        Ok(rows) => rows,
        Err(err) => return server_error("Failed to read split bills", err),
    };
    let payments = match select_group_split_payments(&mut conn, group_id) {
        Ok(rows) => rows,
        Err(err) => return server_error("Failed to read split bill payments", err),
    };
    let names = match select_group_contact_names(&mut conn, group_id) {
        Ok(rows) => rows,
        Err(err) => return server_error("Failed to read names", err),
    };
    ok_json(
        "Success get group settlements",
        GroupSettlements {
            group_id,
            reimbursements,
            shares,
            payments,
            names,
        },
    )
}

// ── Reimburse ────────────────────────────────────────────────────────────────

/// `POST /api/user/groups/{group_id}/reimbursements` - the caller pays the
/// owner of a group spending back. The caller pays from their own source or
/// group balance; the owner receives `personal_amount` in one of their
/// sources. With `return_balance: true` the amount is also returned to the
/// owner's group balance. (Older clients send a `group_amount` paid into the
/// owner's group balance instead of `return_balance`.)
pub async fn post_group_reimbursement_api(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<GroupReimbursementInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let group_id = parse_id(&path.into_inner());
    if let Err(response) = require_member(&mut conn, group_id, &username) {
        return response;
    }
    let return_balance = body.return_balance.unwrap_or(false);
    let group_amount = if body.return_balance.is_some() {
        0.0
    } else {
        body.group_amount
    };
    if body.personal_amount < 0.0 || group_amount < 0.0 {
        return bad_request("Amounts cannot be negative", "");
    }
    let amount = body.personal_amount + group_amount;
    if amount <= 0.0 {
        return bad_request("Amount must be positive", "");
    }

    let reimbursement_id = body.reimbursement_id.unwrap_or_else(Uuid::new_v4);
    match select_reimbursement(&mut conn, reimbursement_id) {
        Ok(Some(existing)) if existing.paid_by == username => {
            return ok_json("Reimbursed", existing);
        }
        Ok(Some(_)) => {
            return HttpResponse::Conflict()
                .json(err_response("Reimbursement id already used", "".to_string()));
        }
        Ok(None) => {}
        Err(err) => return server_error("Failed to reimburse", err),
    }

    let spending = match select_group_spending(&mut conn, group_id, body.transaction_id) {
        Ok(Some(spending)) => spending,
        Ok(None) => {
            return HttpResponse::NotFound().json(err_response(
                "Transaction not found",
                "Only spendings of this group can be reimbursed.".to_string(),
            ));
        }
        Err(err) => return server_error("Failed to reimburse", err),
    };
    if spending.owner.eq_ignore_ascii_case(&username) {
        return bad_request(
            "You cannot reimburse your own transaction",
            "Someone else pays you back.",
        );
    }

    let from = match payer_source(&mut conn, body.from_source_id, body.from_group_balance, &username) {
        Ok(from) => from,
        Err(response) => return response,
    };
    let to = if body.personal_amount > 0.0 {
        let Some(source_id) = body.to_source_id else {
            return bad_request(
                "Choose the owner's source",
                "The personal part needs a to_source_id.",
            );
        };
        let not_found = format!("Pick one of {}'s sources.", spending.owner);
        match owned_source(&mut conn, source_id, &spending.owner, &not_found) {
            Ok(source) => Some(source),
            Err(response) => return response,
        }
    } else {
        None
    };

    let category = if from.is_some() || to.is_some() {
        match transfer_category().await {
            Some(category) => Some(category),
            None => return no_transfer_category(),
        }
    } else {
        None
    };

    let note = body.description.trim().to_string();
    let texts = LegTexts {
        payer: about(format!("Reimburse {}", spending.owner), &spending.description),
        receiver: about(format!("Reimbursement from {username}"), &spending.description),
    };
    let reimbursement = GroupReimbursement {
        reimbursement_id,
        group_id,
        transaction_id: body.transaction_id,
        owner: spending.owner,
        paid_by: username,
        from_source_id: from.as_ref().map(|s| s.0),
        from_source: from
            .map(|s| s.1)
            .unwrap_or_else(|| GROUP_BALANCE_SOURCE.to_string()),
        personal_amount: body.personal_amount,
        to_source_id: to.as_ref().map(|s| s.0),
        to_source: to.map(|s| s.1),
        group_amount,
        amount,
        return_balance,
        description: note,
        created_date: Local::now().naive_local(),
    };
    let category = category.as_ref().map(|(id, name)| TransferCategory { id: *id, name });
    match insert_reimbursement(&mut conn, &reimbursement, category.as_ref(), &texts) {
        Ok(SettleOutcome::Done) => ok_json("Reimbursed", reimbursement),
        Ok(outcome) => outcome_error(outcome),
        Err(err) => server_error("Failed to reimburse", err),
    }
}

// ── Split bill shares ────────────────────────────────────────────────────────

/// `POST /api/user/groups/{group_id}/split-shares` - the owner of a group
/// spending says who else pays part of it: a group member (`username`) who
/// pays through the app, or any `name` whose payments the owner records.
pub async fn post_group_split_share_api(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<GroupSplitShareInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let group_id = parse_id(&path.into_inner());
    if let Err(response) = require_member(&mut conn, group_id, &username) {
        return response;
    }
    if body.amount <= 0.0 {
        return bad_request("Amount must be positive", "");
    }
    let share_id = body.share_id.unwrap_or_else(Uuid::new_v4);
    match split_share_id_owner(&mut conn, share_id) {
        Ok(Some(owner)) if owner == username => {
            return match select_split_share(&mut conn, share_id) {
                Ok(Some(share)) => ok_json("Split bill saved", share),
                Ok(None) => HttpResponse::Conflict()
                    .json(err_response("Share was removed", "".to_string())),
                Err(err) => server_error("Failed to save split bill", err),
            };
        }
        Ok(Some(_)) => {
            return HttpResponse::Conflict()
                .json(err_response("Share id already used", "".to_string()));
        }
        Ok(None) => {}
        Err(err) => return server_error("Failed to save split bill", err),
    }

    let spending = match select_group_spending(&mut conn, group_id, body.transaction_id) {
        Ok(Some(spending)) => spending,
        Ok(None) => {
            return HttpResponse::NotFound().json(err_response(
                "Transaction not found",
                "Only spendings of this group can be split.".to_string(),
            ));
        }
        Err(err) => return server_error("Failed to save split bill", err),
    };
    if spending.owner != username {
        return forbidden("Only the person who recorded this transaction can split it");
    }

    let registered = body.username.as_deref().map(str::trim).filter(|u| !u.is_empty());
    let (member, name) = match registered {
        Some(wanted) => match select_group_member_name(&mut conn, group_id, wanted) {
            Ok(Some(member)) if member.eq_ignore_ascii_case(&username) => {
                return bad_request("Choose someone else", "You already paid your part.");
            }
            Ok(Some(member)) => (Some(member.clone()), member),
            Ok(None) => {
                return HttpResponse::NotFound().json(err_response(
                    "Member not found",
                    "Registered users must be members of this group. Use a name instead."
                        .to_string(),
                ));
            }
            Err(err) => return server_error("Failed to save split bill", err),
        },
        None => {
            let name = body.name.as_deref().unwrap_or("").trim();
            if name.is_empty() {
                return bad_request("Enter a name", "Send a username or a name.");
            }
            if name.chars().count() > 255 {
                return bad_request("Name is too long", "");
            }
            (None, name.to_string())
        }
    };

    let share = GroupSplitShare {
        share_id,
        group_id,
        transaction_id: body.transaction_id,
        owner: username,
        username: member,
        name,
        amount: body.amount,
        paid_amount: 0.0,
        pending_amount: 0.0,
        created_date: Local::now().naive_local(),
    };
    match insert_split_share(&mut conn, &share) {
        Ok(SettleOutcome::Done) => ok_json("Split bill saved", share),
        Ok(outcome) => outcome_error(outcome),
        Err(err) => server_error("Failed to save split bill", err),
    }
}

/// `DELETE /api/user/groups/{group_id}/split-shares/{share_id}` - the owner
/// removes a share nobody has paid yet. Pending requests are cancelled.
pub async fn delete_group_split_share_api(
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let (group_id, share_id) = path.into_inner();
    let (group_id, share_id) = (parse_id(&group_id), parse_id(&share_id));
    if let Err(response) = require_member(&mut conn, group_id, &username) {
        return response;
    }
    let share = match select_split_share(&mut conn, share_id) {
        Ok(Some(share)) if share.group_id == group_id => share,
        Ok(_) => {
            return HttpResponse::NotFound()
                .json(err_response("Share not found", "".to_string()));
        }
        Err(err) => return server_error("Failed to remove share", err),
    };
    if share.owner != username {
        return forbidden("Only the owner of the transaction can remove a share");
    }
    match remove_split_share(&mut conn, share_id) {
        Ok(true) => ok_json("Share removed", share),
        Ok(false) => HttpResponse::Conflict().json(err_response(
            "Share already has payments",
            "A share that was partly paid cannot be removed.".to_string(),
        )),
        Err(err) => server_error("Failed to remove share", err),
    }
}

/// `DELETE /api/user/groups/{group_id}/names/{name}` - forgets a saved name.
/// Shares that already use it keep it.
pub async fn delete_group_contact_name_api(
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let (group_id, name) = path.into_inner();
    let group_id = parse_id(&group_id);
    if let Err(response) = require_member(&mut conn, group_id, &username) {
        return response;
    }
    match delete_group_contact_name(&mut conn, group_id, &name) {
        Ok(_) => ok_json("Name removed", name),
        Err(err) => server_error("Failed to remove name", err),
    }
}

// ── Split bill payments ──────────────────────────────────────────────────────

/// A retried payment with the same id: fine when the caller made it.
fn replay_payment(
    conn: &mut PooledConn,
    payment_id: Uuid,
    made_by: impl Fn(&GroupSplitPayment) -> bool,
    message: &str,
) -> Result<(), HttpResponse> {
    match select_split_payment(conn, payment_id) {
        Ok(Some(existing)) if made_by(&existing) => Err(ok_json(message, existing)),
        Ok(Some(_)) => Err(HttpResponse::Conflict()
            .json(err_response("Payment id already used", "".to_string()))),
        Ok(None) => Ok(()),
        Err(err) => Err(server_error("Failed to save payment", err)),
    }
}

fn share_in_group(
    conn: &mut PooledConn,
    share_id: Uuid,
    group_id: Uuid,
) -> Result<GroupSplitShare, HttpResponse> {
    match select_split_share(conn, share_id) {
        Ok(Some(share)) if share.group_id == group_id => Ok(share),
        Ok(_) => Err(HttpResponse::NotFound()
            .json(err_response("Share not found", "".to_string()))),
        Err(err) => Err(server_error("Failed to read share", err)),
    }
}

/// `POST /api/user/groups/{group_id}/split-shares/{share_id}/payments` - the
/// member a share belongs to pays part or all of it. The request waits for
/// the owner's approval; no money moves before that.
pub async fn post_group_split_payment_api(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Json<GroupSplitPaymentInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let (group_id, share_id) = path.into_inner();
    let (group_id, share_id) = (parse_id(&group_id), parse_id(&share_id));
    if let Err(response) = require_member(&mut conn, group_id, &username) {
        return response;
    }
    if body.amount <= 0.0 {
        return bad_request("Amount must be positive", "");
    }
    let payment_id = body.payment_id.unwrap_or_else(Uuid::new_v4);
    if let Err(response) = replay_payment(
        &mut conn,
        payment_id,
        |p| p.paid_by.as_deref() == Some(username.as_str()),
        "Payment sent for approval",
    ) {
        return response;
    }

    let share = match share_in_group(&mut conn, share_id, group_id) {
        Ok(share) => share,
        Err(response) => return response,
    };
    match &share.username {
        Some(member) if member.eq_ignore_ascii_case(&username) => {}
        _ => return forbidden("This share is not yours to pay"),
    }
    let from = match payer_source(&mut conn, body.from_source_id, body.from_group_balance, &username) {
        Ok(from) => from,
        Err(response) => return response,
    };

    let payment = GroupSplitPayment {
        payment_id,
        share_id,
        group_id,
        transaction_id: share.transaction_id,
        owner: share.owner,
        paid_by: Some(username.clone()),
        payer_name: username,
        amount: body.amount,
        status: "pending".to_string(),
        from_source_id: from.as_ref().map(|s| s.0),
        from_source: Some(
            from.map(|s| s.1)
                .unwrap_or_else(|| GROUP_BALANCE_SOURCE.to_string()),
        ),
        to_source_id: None,
        to_source: None,
        note: body.note.trim().to_string(),
        requested_at: Local::now().naive_local(),
        reviewed_at: None,
    };
    match insert_split_payment_request(&mut conn, &payment) {
        Ok(SettleOutcome::Done) => ok_json("Payment sent for approval", payment),
        Ok(outcome) => outcome_error(outcome),
        Err(err) => server_error("Failed to save payment", err),
    }
}

/// `POST /api/user/groups/{group_id}/split-shares/{share_id}/manual-payments`
/// - the owner records money received from a name-only person. It counts as
/// approved at once and is credited to the owner.
pub async fn post_group_split_manual_payment_api(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Json<GroupSplitManualPaymentInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let (group_id, share_id) = path.into_inner();
    let (group_id, share_id) = (parse_id(&group_id), parse_id(&share_id));
    if let Err(response) = require_member(&mut conn, group_id, &username) {
        return response;
    }
    if body.amount <= 0.0 {
        return bad_request("Amount must be positive", "");
    }
    let payment_id = body.payment_id.unwrap_or_else(Uuid::new_v4);
    if let Err(response) = replay_payment(
        &mut conn,
        payment_id,
        |p| p.owner == username && p.paid_by.is_none(),
        "Payment recorded",
    ) {
        return response;
    }

    let share = match share_in_group(&mut conn, share_id, group_id) {
        Ok(share) => share,
        Err(response) => return response,
    };
    if share.owner != username {
        return forbidden("Only the owner of the transaction can record payments");
    }
    if share.username.is_some() {
        return bad_request(
            "This person pays through the app",
            "Registered users send their own payment for you to approve.",
        );
    }
    let to = match owner_destination(&mut conn, body.to_source_id, body.to_group_balance, &username)
    {
        Ok(to) => to,
        Err(response) => return response,
    };
    let category = match (&to, transfer_category().await) {
        (Some(_), None) => return no_transfer_category(),
        (_, category) => category,
    };
    let description = spending_description(&mut conn, group_id, share.transaction_id);
    let texts = LegTexts {
        payer: String::new(),
        receiver: about(format!("Split bill from {}", share.name), &description),
    };

    let payment = GroupSplitPayment {
        payment_id,
        share_id,
        group_id,
        transaction_id: share.transaction_id,
        owner: username,
        paid_by: None,
        payer_name: share.name,
        amount: body.amount,
        status: "approved".to_string(),
        from_source_id: None,
        from_source: None,
        to_source_id: to.as_ref().map(|s| s.0),
        to_source: Some(
            to.as_ref()
                .map(|s| s.1.clone())
                .unwrap_or_else(|| GROUP_BALANCE_SOURCE.to_string()),
        ),
        note: body.note.trim().to_string(),
        requested_at: Local::now().naive_local(),
        reviewed_at: Some(Local::now().naive_local()),
    };
    let category = category.as_ref().map(|(id, name)| TransferCategory { id: *id, name });
    match insert_manual_split_payment(&mut conn, &payment, to.as_ref(), category.as_ref(), &texts)
    {
        Ok(SettleOutcome::Done) => ok_json("Payment recorded", payment),
        Ok(outcome) => outcome_error(outcome),
        Err(err) => server_error("Failed to record payment", err),
    }
}

/// The settled spending's description, for the records a settlement writes.
fn spending_description(conn: &mut PooledConn, group_id: Uuid, transaction_id: Uuid) -> String {
    select_group_spending(conn, group_id, transaction_id)
        .ok()
        .flatten()
        .map(|s| s.description)
        .unwrap_or_default()
}

fn payment_in_group(
    conn: &mut PooledConn,
    payment_id: Uuid,
    group_id: Uuid,
) -> Result<GroupSplitPayment, HttpResponse> {
    match select_split_payment(conn, payment_id) {
        Ok(Some(payment)) if payment.group_id == group_id => Ok(payment),
        Ok(_) => Err(HttpResponse::NotFound()
            .json(err_response("Payment not found", "".to_string()))),
        Err(err) => Err(server_error("Failed to read payment", err)),
    }
}

/// `PUT /api/user/groups/{group_id}/split-payments/{payment_id}/review` - the
/// owner approves (the money moves now, into `to_source_id` or their group
/// balance) or rejects a pending payment.
pub async fn put_group_split_payment_review_api(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Json<GroupSplitReviewInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let (group_id, payment_id) = path.into_inner();
    let (group_id, payment_id) = (parse_id(&group_id), parse_id(&payment_id));
    if let Err(response) = require_member(&mut conn, group_id, &username) {
        return response;
    }
    let payment = match payment_in_group(&mut conn, payment_id, group_id) {
        Ok(payment) => payment,
        Err(response) => return response,
    };
    if payment.owner != username {
        return forbidden("Only the owner of the transaction can review payments");
    }
    let now = Local::now().naive_local();

    if !body.approve {
        return match close_split_payment(&mut conn, payment_id, "rejected", now) {
            Ok(true) => reread_payment(&mut conn, payment_id, "Payment rejected"),
            Ok(false) => outcome_error(SettleOutcome::NotPending),
            Err(err) => server_error("Failed to reject payment", err),
        };
    }
    if payment.status != "pending" {
        return outcome_error(SettleOutcome::NotPending);
    }

    let to = match owner_destination(&mut conn, body.to_source_id, body.to_group_balance, &username)
    {
        Ok(to) => to,
        Err(response) => return response,
    };
    let needs_category = to.is_some() || payment.from_source_id.is_some();
    let category = match (needs_category, transfer_category().await) {
        (true, None) => return no_transfer_category(),
        (_, category) => category,
    };
    let description = spending_description(&mut conn, group_id, payment.transaction_id);
    let texts = LegTexts {
        payer: about(format!("Split bill to {}", payment.owner), &description),
        receiver: about(format!("Split bill from {}", payment.payer_name), &description),
    };
    let category = category.as_ref().map(|(id, name)| TransferCategory { id: *id, name });
    match approve_split_payment(&mut conn, payment_id, to.as_ref(), category.as_ref(), &texts, now)
    {
        Ok(SettleOutcome::Done) => reread_payment(&mut conn, payment_id, "Payment approved"),
        Ok(outcome) => outcome_error(outcome),
        Err(err) => server_error("Failed to approve payment", err),
    }
}

/// `DELETE /api/user/groups/{group_id}/split-payments/{payment_id}` - the
/// payer withdraws a payment the owner has not reviewed yet.
pub async fn delete_group_split_payment_api(
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let (group_id, payment_id) = path.into_inner();
    let (group_id, payment_id) = (parse_id(&group_id), parse_id(&payment_id));
    if let Err(response) = require_member(&mut conn, group_id, &username) {
        return response;
    }
    let payment = match payment_in_group(&mut conn, payment_id, group_id) {
        Ok(payment) => payment,
        Err(response) => return response,
    };
    if payment.paid_by.as_deref() != Some(username.as_str()) {
        return forbidden("Only the payer can withdraw a payment");
    }
    match close_split_payment(&mut conn, payment_id, "cancelled", Local::now().naive_local()) {
        Ok(true) => reread_payment(&mut conn, payment_id, "Payment withdrawn"),
        Ok(false) => outcome_error(SettleOutcome::NotPending),
        Err(err) => server_error("Failed to withdraw payment", err),
    }
}

fn reread_payment(conn: &mut PooledConn, payment_id: Uuid, message: &str) -> HttpResponse {
    match select_split_payment(conn, payment_id) {
        Ok(Some(payment)) => ok_json(message, payment),
        Ok(None) => HttpResponse::NotFound().json(err_response("Payment not found", "".to_string())),
        Err(err) => server_error("Failed to read payment", err),
    }
}
