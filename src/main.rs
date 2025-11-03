mod handlers;
mod models;
mod routes;
mod repository;
mod helper;
use actix_web::{App, HttpServer};
use dotenv::dotenv;
use std::env;

use routes::main_route::init;
use repository::init::{init_create_table_v2};
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    init_create_table_v2();
    println!("Created tables");
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());

    println!("Server running at http://{}:{}", host, port);

    HttpServer::new(|| {
        App::new()
            .configure(init) // Initialize routes
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await
}