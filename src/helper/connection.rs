use mysql::*;
use std::env;

pub fn establish_connection_v2() -> Result<PooledConn, Box<dyn std::error::Error>>{
    let host = env::var("DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: String = env::var("DB_PORT").unwrap_or_else(|_| "3306".to_string());
    let user: String = env::var("DB_USER").unwrap_or_else(|_| "root".to_string());
    let pass: String = env::var("DB_PASS").unwrap_or_else(|_| "123456".to_string());
    let database: String = env::var("DB_NAME").unwrap_or_else(|_| "transaction".to_string());
    let url = format!("mysql://{}:{}@{}:{}/{}",user, pass, host, port, database);
    println!("{}", url);
    let pool = Pool::new(url.as_str())?;
    let conn = pool.get_conn()?;
    Ok(conn)
}