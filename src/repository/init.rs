use super::source_repository::{create_source_table};
use super::earning_repository::{create_earning_table, create_earning_category_table};
use crate::helper::connection::establish_connection;
pub fn init_create_table() {
    let conn = establish_connection().expect("Failed to connect to database");
    create_earning_category_table(&conn);
    create_earning_table(&conn);
    create_source_table(&conn);
}