use actix_web::web;

use crate::handlers::earning_handler::{post_earning, post_earning_category};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/earnings", web::post().to(post_earning))
            .route("/earning-categories", web::post().to(post_earning_category))
    );
}