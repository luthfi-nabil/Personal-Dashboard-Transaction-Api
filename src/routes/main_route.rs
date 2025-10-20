use actix_web::web;

use crate::handlers::earning_handler::{get_all_earning_categories, get_all_earnings,post_earning, post_earning_category};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/earnings", web::get().to(get_all_earnings))
            .route("/earnings", web::post().to(post_earning))
            .route("/earning-categories", web::get().to(get_all_earning_categories))
            .route("/earning-categories", web::post().to(post_earning_category))
    );
}