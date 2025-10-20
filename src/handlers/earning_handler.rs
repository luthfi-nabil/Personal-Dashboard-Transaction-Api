use actix_web::{web, HttpResponse};
use chrono::Utc;
use uuid::Uuid;

use crate::models::earning::{Earning, EarningCategory};
use crate::models::responses::{Response};
use crate::repository::earning_repository::{select_earnings, select_earning_categories, insert_earning, insert_earning_category};
use crate::helper::connection::{establish_connection};

pub async fn get_all_earnings() -> HttpResponse {
    let connection = establish_connection().expect("Failed to connect to database");
    match select_earnings(&connection) {
        Ok(earnings) => HttpResponse::Ok().json(earnings),
        Err(_) => HttpResponse::InternalServerError().body("Error retrieving earnings"),
    }
}

pub async fn get_all_earning_categories() -> HttpResponse {
    let connection = establish_connection().expect("Failed to connect to database");
    match select_earning_categories(&connection) {
        Ok(categories) => HttpResponse::Ok().json(categories),
        Err(err) => {
            let response = Response {
                status: "Error".to_string(),
                message: "Failed to retrieve earning categories".to_string(),
                description: err.sqlite_error().map(|e| format!("{:?}", e)) // or e.to_string() if available
                .unwrap_or_else(|| err.to_string()),
            };
            HttpResponse::InternalServerError().json(response)
        },
    }
}

pub async fn post_earning(earning: web::Json<Earning>) -> HttpResponse {

    let connection = establish_connection().expect("Failed to connect to database");
    let new_earning = Earning {
        earning_id: Uuid::new_v4(),
        total_amount: earning.total_amount,
        description: earning.description.clone(),
        earning_category_id: earning.earning_category_id,
        earning_category: earning.earning_category.clone(),
        source_id: earning.source_id,
        source: earning.source.clone(),
        created_date: Utc::now(),
        created_by: "User".to_string(),
    };

    let _result  = insert_earning(&connection, &new_earning);

    HttpResponse::Created().json(new_earning)
}

pub async fn post_earning_category(category: web::Json<EarningCategory>) -> HttpResponse {
    let connection = establish_connection().expect("Failed to connect to database");
    let new_category = EarningCategory {
        earning_category_id: Uuid::new_v4(),
        earning_category: category.earning_category.clone(),
        created_date: Utc::now(),
        created_by: "User".to_string(),
    };

    let _result  = insert_earning_category(&connection, &new_category);

    HttpResponse::Created().json(new_category)
}

pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json("API is up and running!")
}