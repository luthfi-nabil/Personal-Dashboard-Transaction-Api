mod handlers;
mod helper;
mod models;
mod repository;
mod route_middleware;
mod routes;
use actix_cors::Cors;
use actix_web::{App, HttpServer, middleware::Logger};
use dotenv::dotenv;
use route_middleware::json_error::JsonErrorMiddleware;
use std::env;
use time::format_description::well_known::Rfc3339;
use tracing_appender::rolling;
use tracing_subscriber::fmt;

use repository::init::init_create_table_v2;
use routes::main_route::init;
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    init_create_table_v2();
    init_logger();
    println!("Created tables");
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());

    println!("Server running at http://{}:{}", host, port);

    let app_env = env::var("APP_ENV").unwrap_or_else(|_| "production".to_string());
    if app_env == "development" {
        println!("Swagger UI   →  http://{}:{}/docs", host, port);
        println!("OpenAPI spec →  http://{}:{}/docs/openapi.yaml", host, port);
    }

    HttpServer::new(|| {
        App::new()
            .wrap(JsonErrorMiddleware)
            .wrap(Logger::new(
                r#"%t | %a | %r | %s | %b | %T | %{User-Agent}i"#,
            ))
            // Outermost: handle CORS preflight (OPTIONS) before it can 404
            .wrap(Cors::permissive())
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
