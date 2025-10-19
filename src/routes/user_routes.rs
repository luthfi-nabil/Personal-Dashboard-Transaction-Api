use actix_web::web;

use crate::handlers::earning_handler::{health_check};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/health", web::get().to(health_check)),
    );
}