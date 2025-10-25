use actix_web::web;

use crate::handlers::earning_handler::{get_all_earning_categories_api, get_all_earnings_api,post_earning_api, post_earning_category_api};
use crate::handlers::spending_handler::{get_all_spending_categories_api, get_all_spendings_api, post_spending_api, post_spending_category_api};
use crate::handlers::source_handler::{get_all_sources_api, post_source_api, delete_source_api};
pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/earnings", web::get().to(get_all_earnings_api))
            .route("/earnings", web::post().to(post_earning_api))
            .route("/earning-categories", web::get().to(get_all_earning_categories_api))
            .route("/earning-categories", web::post().to(post_earning_category_api))
            .route("/spendings", web::get().to(get_all_spendings_api))
            .route("/spendings", web::post().to(post_spending_api))
            .route("/spending-categories", web::get().to(get_all_spending_categories_api))
            .route("/spending-categories", web::post().to(post_spending_category_api))
            .route("/source", web::get().to(get_all_sources_api))
            .route("/source", web::post().to(post_source_api))
            .route("/source/{source}", web::delete().to(delete_source_api))
    );
}