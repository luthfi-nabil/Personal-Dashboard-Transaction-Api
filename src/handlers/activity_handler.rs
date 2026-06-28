use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use chrono::Local;
use uuid::Uuid;

use crate::helper::connection::establish_connection_v2;
use crate::models::activity::{ActivityCategory, ActivityCategoryInput};
use crate::models::responses::Response;
use crate::repository::activity_repository::{
    delete_activity_category, select_activity_categories, select_activity_category_by_name,
    upsert_activity_category,
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

pub async fn get_activity_categories_api(req: HttpRequest) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();

    match select_activity_categories(&mut conn, &created_by) {
        Ok(categories) => HttpResponse::Ok().json(ok_response(
            "Success get activity categories",
            Some(serde_json::to_value(categories).unwrap()),
        )),
        Err(err) => HttpResponse::InternalServerError().json(err_response(
            "Failed to retrieve activity categories",
            err.to_string(),
        )),
    }
}

pub async fn post_activity_category_api(
    req: HttpRequest,
    body: web::Json<ActivityCategoryInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();
    let name = body.activity_category.trim();

    if name.is_empty() {
        return HttpResponse::BadRequest().json(err_response(
            "Invalid activity category",
            "activity_category is required".to_string(),
        ));
    }

    let category = ActivityCategory {
        activity_category_id: Uuid::new_v4(),
        activity_category: name.to_string(),
        created_date: Local::now().naive_local(),
        created_by: created_by.clone(),
        is_active: 1,
    };

    match upsert_activity_category(&mut conn, &category)
        .and_then(|_| select_activity_category_by_name(&mut conn, name, &created_by))
    {
        Ok(Some(saved)) => HttpResponse::Ok().json(ok_response(
            "Activity category saved successfully",
            Some(serde_json::to_value(saved).unwrap()),
        )),
        Ok(None) => HttpResponse::InternalServerError().json(err_response(
            "Failed to save activity category",
            "Saved category could not be read back".to_string(),
        )),
        Err(err) => HttpResponse::InternalServerError().json(err_response(
            "Failed to save activity category",
            err.to_string(),
        )),
    }
}

pub async fn delete_activity_category_api(
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let created_by = req.extensions().get::<CreatedBy>().unwrap().0.clone();

    match delete_activity_category(&mut conn, &path.into_inner(), &created_by) {
        Ok(_) => HttpResponse::Ok().json(ok_response("Activity category removed", None)),
        Err(err) => HttpResponse::InternalServerError().json(err_response(
            "Failed to remove activity category",
            err.to_string(),
        )),
    }
}
