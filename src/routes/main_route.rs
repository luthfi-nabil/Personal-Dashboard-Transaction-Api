use actix_web::web;

use crate::handlers::earning_handler::{get_all_earning_categories_api, get_all_earnings_api,post_earning_api, post_earning_category_api};
use crate::handlers::source_handler::{get_all_sources_api, post_source_api};
pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/earnings", web::get().to(get_all_earnings_api))
            .route("/earnings", web::post().to(post_earning_api))
            .route("/earning-categories", web::get().to(get_all_earning_categories_api))
            .route("/earning-categories", web::post().to(post_earning_category_api))
            .route("/source", web::get().to(get_all_sources_api))
            .route("/source", web::post().to(post_source_api))
    );
}