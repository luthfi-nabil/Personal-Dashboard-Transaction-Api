use actix_web::web;

use crate::handlers::earning_handler::{delete_earning_category_api, get_all_earning_categories_api, get_all_earnings_api, post_earning_api, post_earning_category_api};
use crate::handlers::spending_handler::{get_all_spending_categories_api, get_all_spendings_api, post_spending_api, post_spending_category_api, delete_spending_category_api};
use crate::handlers::source_handler::{get_all_sources_api, post_source_api, delete_source_api};
use crate::handlers::source_handler_v2::{get_all_sources_api_v2, post_source_api_v2, delete_source_api_v2};
use crate::handlers::earning_handler_v2::{post_earning_category_api_v2, get_all_earnings_api_v2, post_earning_api_v2,get_all_earning_categories_api_v2, delete_earning_category_api_v2};
use crate::handlers::spending_handler_v2::{post_spending_category_api_v2, get_all_spendings_api_v2, post_spending_api_v2,get_all_spending_categories_api_v2, delete_spending_category_api_v2};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/earnings", web::get().to(get_all_earnings_api))
            .route("/earnings", web::post().to(post_earning_api))
            .route("/earning-categories", web::get().to(get_all_earning_categories_api))
            .route("/earning-categories", web::post().to(post_earning_category_api))
            .route("/earning-categories/{category}", web::delete().to(delete_earning_category_api))
            .route("/spendings", web::get().to(get_all_spendings_api))
            .route("/spendings", web::post().to(post_spending_api))
            .route("/spending-categories", web::get().to(get_all_spending_categories_api))
            .route("/spending-categories", web::post().to(post_spending_category_api))
            .route("/spending-categories/{category}", web::delete().to(delete_spending_category_api))
            .route("/source", web::get().to(get_all_sources_api))
            .route("/source", web::post().to(post_source_api))
            .route("/source/{source}", web::delete().to(delete_source_api))
            .route("/source-v2",web::get().to(get_all_sources_api_v2))
            .route("/source-v2", web::post().to(post_source_api_v2))
            .route("/source-v2/{source}", web::delete().to(delete_source_api_v2))
            .route("/earnings-v2", web::get().to(get_all_earnings_api_v2))
            .route("/earnings-v2", web::post().to(post_earning_api_v2))
            .route("/earning-categories-v2", web::get().to(get_all_earning_categories_api_v2))
            .route("/earning-categories-v2", web::post().to(post_earning_category_api_v2))
            .route("/earning-categories-v2/{category}", web::delete().to(delete_earning_category_api_v2))
            .route("/spendings-v2", web::get().to(get_all_spendings_api_v2))
            .route("/spendings-v2", web::post().to(post_spending_api_v2))
            .route("/spending-categories-v2", web::get().to(get_all_spending_categories_api_v2))
            .route("/spending-categories-v2", web::post().to(post_spending_category_api_v2))
            .route("/spending-categories-v2/{category}", web::delete().to(delete_spending_category_api_v2))
    );
}