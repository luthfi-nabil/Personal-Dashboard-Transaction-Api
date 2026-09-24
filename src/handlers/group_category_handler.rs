use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use chrono::Local;
use uuid::Uuid;

use crate::helper::connection::establish_connection_v2;
use crate::models::group_category::{GroupCategory, GroupCategoryInput};
use crate::models::responses::Response;
use crate::repository::group_category_repository::{
    deactivate_group_category, group_category_name_taken, select_group_categories_for_user,
    select_group_category_owner, upsert_group_category,
};
use crate::repository::group_repository::select_group_leader;
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

/// Only the group leader manages its categories: anyone else gets 403, and a
/// group that does not exist 404.
fn require_leader(
    conn: &mut mysql::PooledConn,
    group_id: Uuid,
    username: &str,
) -> Result<(), HttpResponse> {
    match select_group_leader(conn, group_id) {
        Ok(Some(leader)) if leader.eq_ignore_ascii_case(username) => Ok(()),
        Ok(Some(_)) => Err(HttpResponse::Forbidden().json(err_response(
            "Only the group admin can manage group categories",
            "".to_string(),
        ))),
        Ok(None) => Err(HttpResponse::NotFound()
            .json(err_response("Group not found", "".to_string()))),
        Err(err) => Err(server_error("Failed to read group", err)),
    }
}

/// `GET /api/user/group-categories` - categories of every group the caller
/// belongs to.
pub async fn get_group_categories_api(req: HttpRequest) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    match select_group_categories_for_user(&mut conn, &username) {
        Ok(items) => HttpResponse::Ok().json(ok_response(
            "Success get group categories",
            Some(serde_json::to_value(items).unwrap()),
        )),
        Err(err) => server_error("Failed to retrieve group categories", err),
    }
}

/// `POST /api/user/groups/{group_id}/categories` - leader only. Creates the
/// category, or renames it when the id already exists in this group.
pub async fn post_group_category_api(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<GroupCategoryInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let group_id = parse_id(&path.into_inner());
    if let Err(response) = require_leader(&mut conn, group_id, &username) {
        return response;
    }

    let name = body.category_name.trim().to_string();
    if name.is_empty() {
        return HttpResponse::BadRequest()
            .json(err_response("Category name is required", "".to_string()));
    }
    let kind = match body.kind.as_deref().map(str::trim) {
        None | Some("") | Some("spending") => "spending",
        Some("earning") => "earning",
        Some(other) => {
            return HttpResponse::BadRequest().json(err_response(
                "Unknown category kind",
                format!("'{other}' is neither spending nor earning."),
            ));
        }
    };
    let category_id = body.category_id.unwrap_or_else(Uuid::new_v4);

    match select_group_category_owner(&mut conn, category_id) {
        Ok(Some(owner)) if parse_id(&owner) != group_id => {
            return HttpResponse::Conflict()
                .json(err_response("Category id already used", "".to_string()));
        }
        Ok(_) => {}
        Err(err) => return server_error("Failed to save group category", err),
    }
    match group_category_name_taken(&mut conn, group_id, &name, kind, category_id) {
        Ok(true) => {
            return HttpResponse::Conflict().json(err_response(
                "Category already exists",
                format!("This group already has a {kind} category named '{name}'."),
            ));
        }
        Ok(false) => {}
        Err(err) => return server_error("Failed to save group category", err),
    }

    let category = GroupCategory {
        category_id,
        group_id,
        category_name: name,
        kind: kind.to_string(),
        created_by: username,
        created_date: Local::now().naive_local(),
    };
    match upsert_group_category(&mut conn, &category) {
        Ok(()) => HttpResponse::Ok().json(ok_response(
            "Group category saved",
            Some(serde_json::to_value(category).unwrap()),
        )),
        Err(err) => server_error("Failed to save group category", err),
    }
}

/// `DELETE /api/user/groups/{group_id}/categories/{category_id}` - leader
/// only. Past transactions keep the category name they were saved with.
pub async fn delete_group_category_api(
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let (group_id, category_id) = path.into_inner();
    let (group_id, category_id) = (parse_id(&group_id), parse_id(&category_id));
    if let Err(response) = require_leader(&mut conn, group_id, &username) {
        return response;
    }
    match deactivate_group_category(&mut conn, group_id, category_id) {
        Ok(true) => HttpResponse::Ok().json(ok_response("Group category removed", None)),
        Ok(false) => HttpResponse::NotFound()
            .json(err_response("Category not found", "".to_string())),
        Err(err) => server_error("Failed to remove group category", err),
    }
}
