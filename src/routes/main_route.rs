use actix_web::web;
use std::env;

use crate::handlers::activity_handler::{
    delete_activity_category_api, get_activity_categories_api, post_activity_category_api,
};
use crate::handlers::app_setting_handler::get_all_setting_api;
use crate::handlers::debt_handler::{get_debt, post_debt_api, update_debt_status};
use crate::handlers::earning_handler_v2::{
    delete_earning_category_api_v2, get_all_earning_categories_api_v2, get_all_earnings_api_v2,
    post_earning_api_v2, post_earning_category_api_v2,
};
use crate::handlers::flutter_sync_handler::{get_sync, post_sync_push};
use crate::handlers::routine_handler::{
    delete_routine_api, get_routine_payments_api, get_routines_api, post_routine_api,
    post_routine_payment_api,
};
use crate::handlers::source_handler_v2::{
    delete_source_api_v2, get_all_source_balance, get_all_sources_api_v2, post_source_api_v2,
};
use crate::handlers::spending_handler_v2::{
    delete_spending_category_api_v2, get_all_spending_categories_api_v2, get_all_spendings_api_v2,
    post_spending_api_v2, post_spending_category_api_v2,
};
use crate::handlers::swagger_handler::{get_swagger_ui, get_swagger_yaml};
use crate::handlers::wishlist_handler::{
    delete_planned_expense_api, delete_planned_expense_category_api,
    get_planned_expense_categories_api, get_planned_expenses_api, post_planned_expense_api,
    post_planned_expense_category_api, put_planned_expense_status_api,
};
use crate::route_middleware::get_user::CreatedByMiddleware;
pub fn init(cfg: &mut web::ServiceConfig) {
    // ── Swagger UI (development only) ────────────────────────────────────────
    let app_env = env::var("APP_ENV").unwrap_or_else(|_| "production".to_string());
    if app_env == "development" {
        cfg.service(
            web::scope("/docs")
                .route("", web::get().to(get_swagger_ui))
                .route("/", web::get().to(get_swagger_ui))
                .route("/openapi.yaml", web::get().to(get_swagger_yaml)),
        );
    }

    // ── API Routes ───────────────────────────────────────────────────────────
    cfg.service(
        web::scope("/api/user")
            .wrap(CreatedByMiddleware)
            .route("/source", web::get().to(get_all_sources_api_v2))
            .route("/source-balance", web::get().to(get_all_source_balance))
            .route("/source", web::post().to(post_source_api_v2))
            .route("/source/{source}", web::delete().to(delete_source_api_v2))
            .route("/earnings", web::get().to(get_all_earnings_api_v2))
            .route("/earnings", web::post().to(post_earning_api_v2))
            .route(
                "/earning-categories",
                web::get().to(get_all_earning_categories_api_v2),
            )
            .route(
                "/earning-categories",
                web::post().to(post_earning_category_api_v2),
            )
            .route(
                "/earning-categories/{category}",
                web::delete().to(delete_earning_category_api_v2),
            )
            .route("/spendings", web::get().to(get_all_spendings_api_v2))
            .route("/spendings", web::post().to(post_spending_api_v2))
            .route(
                "/spending-categories",
                web::get().to(get_all_spending_categories_api_v2),
            )
            .route(
                "/spending-categories",
                web::post().to(post_spending_category_api_v2),
            )
            .route(
                "/spending-categories/{category}",
                web::delete().to(delete_spending_category_api_v2),
            )
            .route("/debt", web::get().to(get_debt))
            .route("/debt", web::post().to(post_debt_api))
            .route("/debt-status", web::put().to(update_debt_status))
            .route("/settings", web::get().to(get_all_setting_api))
            .route("/planned-expenses", web::get().to(get_planned_expenses_api))
            .route(
                "/planned-expenses",
                web::post().to(post_planned_expense_api),
            )
            .route(
                "/planned-expense-categories",
                web::get().to(get_planned_expense_categories_api),
            )
            .route(
                "/planned-expense-categories",
                web::post().to(post_planned_expense_category_api),
            )
            .route(
                "/planned-expense-categories/{category}",
                web::delete().to(delete_planned_expense_category_api),
            )
            .route(
                "/activity-categories",
                web::get().to(get_activity_categories_api),
            )
            .route(
                "/activity-categories",
                web::post().to(post_activity_category_api),
            )
            .route(
                "/activity-categories/{category}",
                web::delete().to(delete_activity_category_api),
            )
            .route(
                "/planned-expenses/{planned_expense_id}/status",
                web::put().to(put_planned_expense_status_api),
            )
            .route(
                "/planned-expenses/{planned_expense_id}",
                web::delete().to(delete_planned_expense_api),
            )
            .route("/wishlist", web::get().to(get_planned_expenses_api))
            .route("/wishlist", web::post().to(post_planned_expense_api))
            .route(
                "/wishlist/{wishlist_id}/status",
                web::put().to(put_planned_expense_status_api),
            )
            .route(
                "/wishlist/{wishlist_id}",
                web::delete().to(delete_planned_expense_api),
            )
            .route("/routines", web::get().to(get_routines_api))
            .route("/routines", web::post().to(post_routine_api))
            .route(
                "/routines/payments",
                web::get().to(get_routine_payments_api),
            )
            .route(
                "/routines/{routine_id}/payments",
                web::post().to(post_routine_payment_api),
            )
            .route(
                "/routines/{routine_id}",
                web::delete().to(delete_routine_api),
            ),
    );
    cfg.service(
        web::scope("/api/flutter")
            .route("/sync", web::get().to(get_sync))
            .route("/sync/push", web::post().to(post_sync_push)),
    );
    cfg.service(web::scope("/api").route("/settings", web::get().to(get_all_setting_api)));
}
