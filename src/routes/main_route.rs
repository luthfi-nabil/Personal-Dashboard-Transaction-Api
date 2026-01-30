use actix_web::web;

use crate::handlers::source_handler_v2::{get_all_sources_api_v2, get_all_source_balance, post_source_api_v2, delete_source_api_v2};
use crate::handlers::earning_handler_v2::{post_earning_category_api_v2, get_all_earnings_api_v2, post_earning_api_v2,get_all_earning_categories_api_v2, delete_earning_category_api_v2};
use crate::handlers::spending_handler_v2::{post_spending_category_api_v2, get_all_spendings_api_v2, post_spending_api_v2,get_all_spending_categories_api_v2, delete_spending_category_api_v2};
use crate::handlers::debt_handler::{get_debt, post_debt_api, update_debt_status};
use crate::handlers::app_setting_handler::{get_all_setting_api};
use crate::route_middleware::get_user::CreatedByMiddleware;
pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/user/{created_by}")
            .wrap(CreatedByMiddleware)
            .route("/settings",web::get().to(get_all_sources_api_v2))
            .route("/source",web::get().to(get_all_sources_api_v2))
            .route("/source-balance",web::get().to(get_all_source_balance))
            .route("/source", web::post().to(post_source_api_v2))
            .route("/source/{source}", web::delete().to(delete_source_api_v2))
            .route("/earnings", web::get().to(get_all_earnings_api_v2))
            .route("/earnings", web::post().to(post_earning_api_v2))
            .route("/earning-categories", web::get().to(get_all_earning_categories_api_v2))
            .route("/earning-categories", web::post().to(post_earning_category_api_v2))
            .route("/earning-categories/{category}", web::delete().to(delete_earning_category_api_v2))
            .route("/spendings", web::get().to(get_all_spendings_api_v2))
            .route("/spendings", web::post().to(post_spending_api_v2))
            .route("/spending-categories", web::get().to(get_all_spending_categories_api_v2))
            .route("/spending-categories", web::post().to(post_spending_category_api_v2))
            .route("/spending-categories/{category}", web::delete().to(delete_spending_category_api_v2))
            .route("/debt", web::get().to(get_debt))
            .route("/debt", web::post().to(post_debt_api))
            .route("/debt-status", web::put().to(update_debt_status))
            .route("/settings", web::get().to(get_all_setting_api))
    );
    cfg.service(
        web::scope("/api")
        .route("/settings", web::get().to(get_all_setting_api))
    );
}