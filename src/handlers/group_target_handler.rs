use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use chrono::Local;
use uuid::Uuid;

use crate::helper::connection::establish_connection_v2;
use crate::models::group_target::{
    GroupMemberTarget, GroupTargetInput, GroupTargetSettingInput, GroupTargets,
};
use crate::models::responses::Response;
use crate::repository::group_repository::{is_group_member, select_group_leader};
use crate::repository::group_target_repository::{
    select_group_target_enabled, select_group_targets, upsert_group_member_target,
    upsert_group_target_setting,
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

fn parse_id(raw: &str) -> Uuid {
    Uuid::parse_str(raw).unwrap_or_else(|_| Uuid::nil())
}

/// Checks membership and says whether the caller leads the group.
fn member_role(
    conn: &mut mysql::PooledConn,
    group_id: Uuid,
    username: &str,
) -> Result<bool, HttpResponse> {
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
        Ok(leader) => Ok(leader
            .map(|l| l.eq_ignore_ascii_case(username))
            .unwrap_or(false)),
        Err(err) => Err(HttpResponse::InternalServerError()
            .json(err_response("Failed to read group", err.to_string()))),
    }
}

fn load_targets(
    conn: &mut mysql::PooledConn,
    group_id: Uuid,
    username: &str,
    is_leader: bool,
) -> HttpResponse {
    let enabled = match select_group_target_enabled(conn, group_id) {
        Ok(enabled) => enabled,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(err_response("Failed to read target setting", err.to_string()));
        }
    };
    let only_user = (!is_leader).then_some(username);
    match select_group_targets(conn, group_id, only_user) {
        Ok(targets) => HttpResponse::Ok().json(ok_response(
            "Success get group targets",
            Some(
                serde_json::to_value(GroupTargets {
                    group_id,
                    is_leader,
                    enabled,
                    targets,
                })
                .unwrap(),
            ),
        )),
        Err(err) => HttpResponse::InternalServerError()
            .json(err_response("Failed to read targets", err.to_string())),
    }
}

/// `GET /api/user/groups/{group_id}/targets` - the leader sees every
/// member's monthly target; anyone else only their own.
pub async fn get_group_targets_api(req: HttpRequest, path: web::Path<String>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let group_id = parse_id(&path.into_inner());
    let is_leader = match member_role(&mut conn, group_id, &username) {
        Ok(is_leader) => is_leader,
        Err(response) => return response,
    };
    load_targets(&mut conn, group_id, &username, is_leader)
}

/// `PUT /api/user/groups/{group_id}/target` - sets the caller's own monthly
/// target. Zero clears it. Refused while the leader has targets switched off.
pub async fn put_group_target_api(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<GroupTargetInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let group_id = parse_id(&path.into_inner());
    let is_leader = match member_role(&mut conn, group_id, &username) {
        Ok(is_leader) => is_leader,
        Err(response) => return response,
    };
    if !body.amount.is_finite() || body.amount < 0.0 {
        return HttpResponse::BadRequest()
            .json(err_response("Target cannot be negative", "".to_string()));
    }
    match select_group_target_enabled(&mut conn, group_id) {
        Ok(true) => {}
        Ok(false) => {
            return HttpResponse::Conflict().json(err_response(
                "Target spendings are turned off for this group",
                "".to_string(),
            ));
        }
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(err_response("Failed to read target setting", err.to_string()));
        }
    }
    let target = GroupMemberTarget {
        group_id,
        username: username.clone(),
        amount: body.amount,
        updated_date: Local::now().naive_local(),
    };
    if let Err(err) = upsert_group_member_target(&mut conn, &target) {
        return HttpResponse::InternalServerError()
            .json(err_response("Failed to save target", err.to_string()));
    }
    load_targets(&mut conn, group_id, &username, is_leader)
}

/// `PUT /api/user/groups/{group_id}/target-setting` - leader only: turns
/// target spendings on or off for the whole group. Targets already set are
/// kept, so switching back on restores them.
pub async fn put_group_target_setting_api(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<GroupTargetSettingInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let group_id = parse_id(&path.into_inner());
    match member_role(&mut conn, group_id, &username) {
        Ok(true) => {}
        Ok(false) => {
            return HttpResponse::Forbidden().json(err_response(
                "Only the group leader can switch target spendings",
                "".to_string(),
            ));
        }
        Err(response) => return response,
    }
    if let Err(err) = upsert_group_target_setting(
        &mut conn,
        group_id,
        body.enabled,
        &username,
        Local::now().naive_local(),
    ) {
        return HttpResponse::InternalServerError()
            .json(err_response("Failed to save target setting", err.to_string()));
    }
    load_targets(&mut conn, group_id, &username, true)
}
