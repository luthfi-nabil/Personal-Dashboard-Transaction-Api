mod handlers;
mod models;
mod routes;
mod repository;
mod helper;
mod route_middleware;
use route_middleware::json_error::JsonErrorMiddleware;
use actix_web::{App, HttpServer, middleware::{Logger}};
use dotenv::dotenv;
use std::env;
use tracing_subscriber::{fmt};
use tracing_appender::rolling;
use time::format_description::well_known::Rfc3339;

use routes::main_route::init;
use repository::init::{init_create_table_v2};
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    init_create_table_v2();
    init_logger();
    println!("Created tables");
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());

    println!("Server running at http://{}:{}", host, port);

    HttpServer::new(|| {
        App::new()
            .wrap(JsonErrorMiddleware)
            .wrap(
                Logger::new(
                    r#"%t | %a | %m | %U | %s | %b | %T | %{User-Agent}i"#,
                )
            )
            .configure(init) // Initialize routes
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await
}

fn init_logger() {
    let file_appender = rolling::daily("logs", "actix.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    Box::leak(Box::new(guard));

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_timer(fmt::time::UtcTime::new(Rfc3339))
        .with_target(false)
        .compact() // better for inline logs
        .init();
}