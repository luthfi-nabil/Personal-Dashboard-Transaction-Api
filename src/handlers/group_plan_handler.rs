use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use chrono::Local;
use mysql::PooledConn;
use uuid::Uuid;

use crate::helper::connection::establish_connection_v2;
use crate::models::group_plan::{
    GroupPaymentInput, GroupPlannedExpense, GroupPlannedExpenseInput,
    GroupPlannedExpenseReviewInput, GroupRoutine, GroupRoutineInput, GroupRoutinePayment,
};
use crate::models::responses::Response;
use crate::repository::group_balance_repository::balance_spending;
use crate::repository::group_plan_repository::{
    FulfilOutcome, MemberSpending, Payer, fulfil_group_planned_expense, group_routine_payment_exists,
    insert_group_planned_expense, insert_group_routine_payment, remove_group_routine,
    select_group_planned_expense, select_group_planned_expenses, select_group_routine,
    select_group_routine_payments, select_group_routines, select_owned_source_name,
    update_group_planned_expense_status, upsert_group_routine,
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

fn parse_id(raw: &str) -> Uuid {
    Uuid::parse_str(raw).unwrap_or_else(|_| Uuid::nil())
}

fn caller(req: &HttpRequest) -> String {
    req.extensions().get::<CreatedBy>().unwrap().0.clone()
}

/// The caller's role in `group_id`, or the response to send back when they
/// have none.
enum Role {
    Leader,
    Member,
}

fn role_in_group(
    conn: &mut PooledConn,
    group_id: Uuid,
    username: &str,
) -> Result<Role, HttpResponse> {
    match is_group_member(conn, group_id, username) {
        Ok(true) => {}
        Ok(false) => {
            return Err(HttpResponse::NotFound()
                .json(err_response("Group not found", "".to_string())));
        }
        Err(err) => {
            return Err(HttpResponse::InternalServerError()
                .json(err_response("Failed to read group", err.to_string())));
        }
    }
    match select_group_leader(conn, group_id) {
        Ok(Some(leader)) if leader == username => Ok(Role::Leader),
        Ok(_) => Ok(Role::Member),
        Err(err) => Err(HttpResponse::InternalServerError()
            .json(err_response("Failed to read group", err.to_string()))),
    }
}

/// Resolves the payer's own source, rejecting anyone else's. `None` means
/// the request named no source at all.
fn payer_source(
    conn: &mut PooledConn,
    source_id: Option<Uuid>,
    username: &str,
) -> Result<String, HttpResponse> {
    let Some(source_id) = source_id else {
        return Err(HttpResponse::BadRequest().json(err_response(
            "Choose where the money comes from",
            "Send a source_id or set from_group_balance.".to_string(),
        )));
    };
    match select_owned_source_name(conn, source_id, username) {
        Ok(Some(name)) => Ok(name),
        Ok(None) => Err(HttpResponse::BadRequest().json(err_response(
            "Source not found",
            "Pay from one of your own sources.".to_string(),
        ))),
        Err(err) => Err(HttpResponse::InternalServerError()
            .json(err_response("Failed to read source", err.to_string()))),
    }
}

/// Shown as the "source" of a payment made from the payer's group balance.
const GROUP_BALANCE_SOURCE: &str = "Group balance";

/// The payment as a personal spending, or - with `from_group_balance` - as
/// the same amount spent from the payer's group balance instead.
fn make_payer(from_group_balance: bool, spending: MemberSpending<'_>) -> Payer<'_> {
    if !from_group_balance {
        return Payer::Source(spending);
    }
    Payer::GroupBalance(balance_spending(
        spending.spending_id,
        spending.group_id,
        spending.spent_by,
        spending.total_amount,
        spending.description,
        spending.spending_category_id,
        spending.spending_category,
        spending.created_date,
    ))
}

fn insufficient_balance() -> HttpResponse {
    HttpResponse::Conflict().json(err_response(
        "Not enough group balance",
        "Add balance first, or pay from one of your sources.".to_string(),
    ))
}

// ── Routines ────────────────────────────────────────────────────────────

/// `GET /api/user/group-routines` - routines across all the caller's groups.
pub async fn get_group_routines_api(req: HttpRequest) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    match select_group_routines(&mut conn, &caller(&req)) {
        Ok(items) => HttpResponse::Ok().json(ok_response(
            "Success get group routines",
            Some(serde_json::to_value(items).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError()
            .json(err_response("Failed to retrieve group routines", err.to_string())),
    }
}

/// `GET /api/user/group-routine-payments` - payments across all the caller's
/// groups, newest first.
pub async fn get_group_routine_payments_api(req: HttpRequest) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    match select_group_routine_payments(&mut conn, &caller(&req)) {
        Ok(items) => HttpResponse::Ok().json(ok_response(
            "Success get group routine payments",
            Some(serde_json::to_value(items).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError().json(err_response(
            "Failed to retrieve group routine payments",
            err.to_string(),
        )),
    }
}

/// `POST /api/user/groups/{group_id}/routines` - leader only.
pub async fn post_group_routine_api(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<GroupRoutineInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let group_id = parse_id(&path.into_inner());
    match role_in_group(&mut conn, group_id, &username) {
        Ok(Role::Leader) => {}
        Ok(Role::Member) => {
            return HttpResponse::Forbidden().json(err_response(
                "Only the group leader can manage routines",
                "".to_string(),
            ));
        }
        Err(response) => return response,
    }
    let name = body.item_name.trim().to_string();
    if name.is_empty() || body.price <= 0.0 {
        return HttpResponse::BadRequest().json(err_response(
            "Name and a positive price are required",
            "".to_string(),
        ));
    }

    let now = Local::now().naive_local();
    let routine = GroupRoutine {
        routine_id: body.routine_id.unwrap_or_else(Uuid::new_v4),
        group_id,
        item_name: name,
        price: body.price,
        reminder: body.reminder.clone(),
        spending_category_id: body.spending_category_id,
        spending_category: body.spending_category.clone(),
        last_paid_at: None,
        last_paid_by: None,
        created_by: username,
        created_date: now,
        updated_date: now,
    };
    match upsert_group_routine(&mut conn, &routine) {
        Ok(_) => HttpResponse::Ok().json(ok_response(
            "Group routine saved",
            Some(serde_json::to_value(routine).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError()
            .json(err_response("Failed to save group routine", err.to_string())),
    }
}

/// `DELETE /api/user/groups/{group_id}/routines/{routine_id}` - leader only.
/// Past payments stay, since they are already somebody's spending.
pub async fn delete_group_routine_api(
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let (group_id, routine_id) = path.into_inner();
    let group_id = parse_id(&group_id);
    match role_in_group(&mut conn, group_id, &username) {
        Ok(Role::Leader) => {}
        Ok(Role::Member) => {
            return HttpResponse::Forbidden().json(err_response(
                "Only the group leader can manage routines",
                "".to_string(),
            ));
        }
        Err(response) => return response,
    }
    match remove_group_routine(
        &mut conn,
        group_id,
        parse_id(&routine_id),
        Local::now().naive_local(),
    ) {
        Ok(_) => HttpResponse::Ok().json(ok_response("Group routine removed", None)),
        Err(err) => HttpResponse::InternalServerError()
            .json(err_response("Failed to remove group routine", err.to_string())),
    }
}

/// `POST /api/user/groups/{group_id}/routines/{routine_id}/payments` - any
/// member pays the routine out of one of their own sources.
pub async fn post_group_routine_payment_api(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Json<GroupPaymentInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let (group_id, routine_id) = path.into_inner();
    let group_id = parse_id(&group_id);
    let routine_id = parse_id(&routine_id);
    if let Err(response) = role_in_group(&mut conn, group_id, &username) {
        return response;
    }
    if body.price <= 0.0 {
        return HttpResponse::BadRequest()
            .json(err_response("Price must be positive", "".to_string()));
    }

    let payment_id = body.payment_id.unwrap_or_else(Uuid::new_v4);
    match group_routine_payment_exists(&mut conn, payment_id) {
        // A retry of a payment that already went through.
        Ok(true) => return HttpResponse::Ok().json(ok_response("Already paid", None)),
        Ok(false) => {}
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(err_response("Failed to pay routine", err.to_string()));
        }
    }
    let (item_name, _reminder, category_id, category) =
        match select_group_routine(&mut conn, group_id, routine_id) {
            Ok(Some(row)) => row,
            Ok(None) => {
                return HttpResponse::NotFound()
                    .json(err_response("Group routine not found", "".to_string()));
            }
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .json(err_response("Failed to pay routine", err.to_string()));
            }
        };
    let source = if body.from_group_balance {
        GROUP_BALANCE_SOURCE.to_string()
    } else {
        match payer_source(&mut conn, body.source_id, &username) {
            Ok(name) => name,
            Err(response) => return response,
        }
    };

    let paid_at = body.paid_at.unwrap_or_else(|| Local::now().naive_local());
    let description = body
        .description
        .clone()
        .filter(|d| !d.trim().is_empty())
        .unwrap_or_else(|| item_name.clone());
    let payment = GroupRoutinePayment {
        payment_id,
        routine_id,
        group_id,
        spending_id: Uuid::new_v4(),
        item_name,
        price: body.price,
        source_id: if body.from_group_balance {
            Uuid::nil()
        } else {
            body.source_id.unwrap_or_default()
        },
        source: source.clone(),
        paid_by: username.clone(),
        paid_at,
    };
    let payer = make_payer(
        body.from_group_balance,
        MemberSpending {
            spending_id: payment.spending_id,
            group_id,
            total_amount: body.price,
            description: &description,
            spending_category_id: category_id,
            spending_category: &category,
            source_id: payment.source_id,
            source: &source,
            spent_by: &username,
            created_date: paid_at,
        },
    );
    match insert_group_routine_payment(&mut conn, &payment, &payer) {
        Ok(true) => HttpResponse::Ok().json(ok_response(
            "Group routine paid",
            Some(serde_json::to_value(payment).unwrap()),
        )),
        Ok(false) => insufficient_balance(),
        Err(err) => HttpResponse::InternalServerError()
            .json(err_response("Failed to pay routine", err.to_string())),
    }
}

// ── Planned expenses ────────────────────────────────────────────────────

/// `GET /api/user/group-planned-expenses` - every item, any status, across the
/// caller's groups.
pub async fn get_group_planned_expenses_api(req: HttpRequest) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    match select_group_planned_expenses(&mut conn, &caller(&req)) {
        Ok(items) => HttpResponse::Ok().json(ok_response(
            "Success get group planned expenses",
            Some(serde_json::to_value(items).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError().json(err_response(
            "Failed to retrieve group planned expenses",
            err.to_string(),
        )),
    }
}

/// `POST /api/user/groups/{group_id}/planned-expenses` - the leader adds a
/// `planned` item straight away; anyone else files a `requested` one.
pub async fn post_group_planned_expense_api(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<GroupPlannedExpenseInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let group_id = parse_id(&path.into_inner());
    let role = match role_in_group(&mut conn, group_id, &username) {
        Ok(role) => role,
        Err(response) => return response,
    };
    let name = body.item_name.trim().to_string();
    if name.is_empty() || body.price <= 0.0 {
        return HttpResponse::BadRequest().json(err_response(
            "Name and a positive price are required",
            "".to_string(),
        ));
    }
    let id = body.planned_expense_id.unwrap_or_else(Uuid::new_v4);
    match select_group_planned_expense(&mut conn, group_id, id) {
        Ok(Some(existing)) => {
            // Retried create: hand back what is already there.
            return HttpResponse::Ok().json(ok_response(
                "Group planned expense saved",
                Some(serde_json::to_value(existing).unwrap()),
            ));
        }
        Ok(None) => {}
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(err_response("Failed to save planned expense", err.to_string()));
        }
    }

    let now = Local::now().naive_local();
    let is_leader = matches!(role, Role::Leader);
    let item = GroupPlannedExpense {
        planned_expense_id: id,
        group_id,
        item_name: name,
        price: body.price,
        spending_category_id: body.spending_category_id,
        spending_category: body.spending_category.clone(),
        notes: body.notes.trim().to_string(),
        status: if is_leader { "planned" } else { "requested" }.to_string(),
        requested_by: username.clone(),
        reviewed_by: is_leader.then(|| username.clone()),
        reviewed_at: is_leader.then_some(now),
        fulfilled_by: None,
        fulfilled_price: None,
        fulfilled_at: None,
        spending_id: None,
        created_date: now,
        updated_date: now,
    };
    match insert_group_planned_expense(&mut conn, &item) {
        Ok(_) => HttpResponse::Ok().json(ok_response(
            if is_leader {
                "Group planned expense added"
            } else {
                "Request sent to the group leader"
            },
            Some(serde_json::to_value(item).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError()
            .json(err_response("Failed to save planned expense", err.to_string())),
    }
}

/// `PUT /api/user/groups/{group_id}/planned-expenses/{id}/review` - the
/// leader approves or rejects a member's request.
pub async fn put_group_planned_expense_review_api(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Json<GroupPlannedExpenseReviewInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let (group_id, id) = path.into_inner();
    let group_id = parse_id(&group_id);
    let id = parse_id(&id);
    match role_in_group(&mut conn, group_id, &username) {
        Ok(Role::Leader) => {}
        Ok(Role::Member) => {
            return HttpResponse::Forbidden().json(err_response(
                "Only the group leader can review requests",
                "".to_string(),
            ));
        }
        Err(response) => return response,
    }
    let to_status = if body.approve { "planned" } else { "rejected" };
    let now = Local::now().naive_local();
    match update_group_planned_expense_status(
        &mut conn,
        id,
        "requested",
        to_status,
        Some(&username),
        now,
    ) {
        Ok(true) => respond_with_item(&mut conn, group_id, id, "Request reviewed"),
        Ok(false) => HttpResponse::Conflict().json(err_response(
            "This request was already handled",
            "".to_string(),
        )),
        Err(err) => HttpResponse::InternalServerError()
            .json(err_response("Failed to review request", err.to_string())),
    }
}

/// `PUT /api/user/groups/{group_id}/planned-expenses/{id}/fulfil` - any
/// member buys a `planned` item out of one of their own sources.
pub async fn put_group_planned_expense_fulfil_api(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Json<GroupPaymentInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let (group_id, id) = path.into_inner();
    let group_id = parse_id(&group_id);
    let id = parse_id(&id);
    if let Err(response) = role_in_group(&mut conn, group_id, &username) {
        return response;
    }
    if body.price <= 0.0 {
        return HttpResponse::BadRequest()
            .json(err_response("Price must be positive", "".to_string()));
    }
    let item = match select_group_planned_expense(&mut conn, group_id, id) {
        Ok(Some(item)) => item,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(err_response("Planned expense not found", "".to_string()));
        }
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(err_response("Failed to fulfil planned expense", err.to_string()));
        }
    };
    if item.status == "fulfilled" && item.fulfilled_by.as_deref() == Some(username.as_str()) {
        // A retry of this member's own fulfilment.
        return HttpResponse::Ok().json(ok_response(
            "Planned expense fulfilled",
            Some(serde_json::to_value(item).unwrap()),
        ));
    }
    if item.status != "planned" {
        return HttpResponse::Conflict().json(err_response(
            "This item cannot be bought right now",
            format!("It is {}.", item.status),
        ));
    }
    let source = if body.from_group_balance {
        GROUP_BALANCE_SOURCE.to_string()
    } else {
        match payer_source(&mut conn, body.source_id, &username) {
            Ok(name) => name,
            Err(response) => return response,
        }
    };

    let description = body
        .description
        .clone()
        .filter(|d| !d.trim().is_empty())
        .unwrap_or_else(|| item.item_name.clone());
    let payer = make_payer(
        body.from_group_balance,
        MemberSpending {
            spending_id: Uuid::new_v4(),
            group_id,
            total_amount: body.price,
            description: &description,
            spending_category_id: item.spending_category_id,
            spending_category: &item.spending_category,
            source_id: body.source_id.unwrap_or_default(),
            source: &source,
            spent_by: &username,
            created_date: body.paid_at.unwrap_or_else(|| Local::now().naive_local()),
        },
    );
    match fulfil_group_planned_expense(&mut conn, id, &payer) {
        Ok(FulfilOutcome::Done) => {
            respond_with_item(&mut conn, group_id, id, "Planned expense fulfilled")
        }
        Ok(FulfilOutcome::NotPlanned) => HttpResponse::Conflict().json(err_response(
            "Someone already bought this item",
            "".to_string(),
        )),
        Ok(FulfilOutcome::InsufficientBalance) => insufficient_balance(),
        Err(err) => HttpResponse::InternalServerError()
            .json(err_response("Failed to fulfil planned expense", err.to_string())),
    }
}

/// `DELETE /api/user/groups/{group_id}/planned-expenses/{id}` - cancels an
/// open item. The leader can cancel anything open; a member only their own
/// request that has not been reviewed yet.
pub async fn delete_group_planned_expense_api(
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let (group_id, id) = path.into_inner();
    let group_id = parse_id(&group_id);
    let id = parse_id(&id);
    let role = match role_in_group(&mut conn, group_id, &username) {
        Ok(role) => role,
        Err(response) => return response,
    };
    let item = match select_group_planned_expense(&mut conn, group_id, id) {
        Ok(Some(item)) => item,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(err_response("Planned expense not found", "".to_string()));
        }
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(err_response("Failed to cancel planned expense", err.to_string()));
        }
    };
    let allowed = match role {
        Role::Leader => item.status == "planned" || item.status == "requested",
        Role::Member => item.status == "requested" && item.requested_by == username,
    };
    if !allowed {
        return HttpResponse::Forbidden().json(err_response(
            "You cannot cancel this item",
            "".to_string(),
        ));
    }
    match update_group_planned_expense_status(
        &mut conn,
        id,
        &item.status,
        "canceled",
        None,
        Local::now().naive_local(),
    ) {
        Ok(true) => respond_with_item(&mut conn, group_id, id, "Planned expense canceled"),
        Ok(false) => HttpResponse::Conflict().json(err_response(
            "This item changed meanwhile - refresh and try again",
            "".to_string(),
        )),
        Err(err) => HttpResponse::InternalServerError()
            .json(err_response("Failed to cancel planned expense", err.to_string())),
    }
}

fn respond_with_item(
    conn: &mut PooledConn,
    group_id: Uuid,
    id: Uuid,
    message: &str,
) -> HttpResponse {
    match select_group_planned_expense(conn, group_id, id) {
        Ok(item) => HttpResponse::Ok().json(ok_response(
            message,
            item.map(|i| serde_json::to_value(i).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError()
            .json(err_response("Failed to read planned expense", err.to_string())),
    }
}
