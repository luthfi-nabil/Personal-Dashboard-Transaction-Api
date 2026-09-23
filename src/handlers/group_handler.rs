use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use chrono::Local;
use uuid::Uuid;

use crate::helper::connection::establish_connection_v2;
use crate::helper::user_client::{UserLookup, lookup_user};
use crate::models::group::{GroupInput, GroupMemberInput, GroupStatusChange, GroupStatusInput};
use crate::models::responses::Response;
use crate::repository::group_repository::{
    insert_group_member, insert_group_status, is_group_member, select_group_leader,
    select_group_transactions, select_groups_for_user, upsert_group,
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

fn parse_group_id(raw: String) -> Uuid {
    Uuid::parse_str(&raw).unwrap_or_else(|_| Uuid::nil())
}

/// `GET /api/user/groups` - every group the caller belongs to.
pub async fn get_groups_api(req: HttpRequest) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = req.extensions().get::<CreatedBy>().unwrap().0.clone();

    match select_groups_for_user(&mut conn, &username) {
        Ok(items) => HttpResponse::Ok().json(ok_response(
            "Success get groups",
            Some(serde_json::to_value(items).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError()
            .json(err_response("Failed to retrieve groups", err.to_string())),
    }
}

/// `GET /api/user/group-transactions` - every member's tagged transactions
/// across all of the caller's groups.
pub async fn get_group_transactions_api(req: HttpRequest) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = req.extensions().get::<CreatedBy>().unwrap().0.clone();

    match select_group_transactions(&mut conn, &username) {
        Ok(items) => HttpResponse::Ok().json(ok_response(
            "Success get group transactions",
            Some(serde_json::to_value(items).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError().json(err_response(
            "Failed to retrieve group transactions",
            err.to_string(),
        )),
    }
}

/// `POST /api/user/groups` - creates a group led by the caller, or renames one
/// the caller leads. Upserts on the client-generated id.
pub async fn post_group_api(req: HttpRequest, body: web::Json<GroupInput>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let name = body.group_name.trim().to_string();
    if name.is_empty() {
        return HttpResponse::BadRequest().json(err_response(
            "Group name is required",
            "".to_string(),
        ));
    }
    let group_id = body.group_id.unwrap_or_else(Uuid::new_v4);

    match select_group_leader(&mut conn, group_id) {
        Ok(Some(leader)) if leader != username => {
            return HttpResponse::Forbidden().json(err_response(
                "Only the group leader can change this group",
                "".to_string(),
            ));
        }
        Ok(_) => {}
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(err_response("Failed to save group", err.to_string()));
        }
    }

    let created_date = body
        .created_date
        .unwrap_or_else(|| Local::now().naive_local());
    if let Err(err) = upsert_group(&mut conn, group_id, &name, &username, created_date) {
        return HttpResponse::InternalServerError()
            .json(err_response("Failed to save group", err.to_string()));
    }

    match select_groups_for_user(&mut conn, &username) {
        Ok(groups) => {
            let saved = groups.into_iter().find(|g| g.group_id == group_id);
            HttpResponse::Ok().json(ok_response(
                "Group saved successfully",
                saved.map(|g| serde_json::to_value(g).unwrap()),
            ))
        }
        Err(err) => HttpResponse::InternalServerError()
            .json(err_response("Failed to read saved group", err.to_string())),
    }
}

/// `POST /api/user/groups/{group_id}/members` - any member can add another
/// user by username.
pub async fn post_group_member_api(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<GroupMemberInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let group_id = parse_group_id(path.into_inner());
    let new_member = body.username.trim().to_string();
    if new_member.is_empty() {
        return HttpResponse::BadRequest()
            .json(err_response("Username is required", "".to_string()));
    }

    match is_group_member(&mut conn, group_id, &username) {
        Ok(true) => {}
        Ok(false) => {
            return HttpResponse::NotFound()
                .json(err_response("Group not found", "".to_string()));
        }
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(err_response("Failed to add member", err.to_string()));
        }
    }

    // The account must exist in login-api. Its stored spelling is what gets
    // saved, so "Alice" and "alice" can never become two members.
    let authorization = req
        .headers()
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let new_member = match lookup_user(&new_member, &authorization).await {
        UserLookup::Found(canonical) => canonical,
        UserLookup::NotFound => {
            return HttpResponse::NotFound().json(err_response(
                "User not found",
                format!("No account is named '{}'.", new_member),
            ));
        }
        UserLookup::Unavailable(err) => {
            tracing::warn!("User lookup for group member failed: {}", err);
            return HttpResponse::ServiceUnavailable().json(err_response(
                "Cannot verify the user right now",
                "The account service is unreachable. Try again later.".to_string(),
            ));
        }
    };

    match is_group_member(&mut conn, group_id, &new_member) {
        Ok(true) => {
            return HttpResponse::Conflict().json(err_response(
                "Already a member",
                format!("{} is already in this group.", new_member),
            ));
        }
        Ok(false) => {}
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(err_response("Failed to add member", err.to_string()));
        }
    }

    let added_date = body
        .added_date
        .unwrap_or_else(|| Local::now().naive_local());
    match insert_group_member(&mut conn, group_id, &new_member, &username, added_date) {
        Ok(_) => HttpResponse::Ok().json(ok_response(
            "Member added",
            Some(serde_json::json!({ "username": new_member })),
        )),
        Err(err) => HttpResponse::InternalServerError()
            .json(err_response("Failed to add member", err.to_string())),
    }
}

/// `PUT /api/user/groups/{group_id}/status` - leader-only on/off switch.
pub async fn put_group_status_api(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<GroupStatusInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let group_id = parse_group_id(path.into_inner());

    match select_group_leader(&mut conn, group_id) {
        Ok(Some(leader)) if leader == username => {}
        Ok(Some(_)) => {
            return HttpResponse::Forbidden().json(err_response(
                "Only the group leader can switch this group on or off",
                "".to_string(),
            ));
        }
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(err_response("Group not found", "".to_string()));
        }
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(err_response("Failed to update group", err.to_string()));
        }
    }

    let change = GroupStatusChange {
        status_id: body.status_id.unwrap_or_else(Uuid::new_v4),
        group_id,
        is_active: body.is_active,
        changed_by: username,
        changed_at: body
            .changed_at
            .unwrap_or_else(|| Local::now().naive_local()),
    };
    match insert_group_status(&mut conn, &change) {
        Ok(_) => HttpResponse::Ok().json(ok_response(
            "Group status updated",
            Some(serde_json::to_value(change).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError()
            .json(err_response("Failed to update group", err.to_string())),
    }
}
