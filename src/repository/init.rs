use mysql::PooledConn;

use super::source_repository::{create_source_table};
use super::earning_repository::{create_earning_table, create_earning_category_table};
use super::spending_repository::{create_spending_category_table, create_spending_table};
use crate::helper::connection::{establish_connection, establish_connection_v2};
use crate::repository::spending_repository_v2::{create_spending_category_table as create_spending_category_table_v2, create_spending_table as create_spending_table_v2};
use crate::repository::earning_repository_v2::{create_earning_category_table as create_earning_category_table_v2, create_earning_table as create_earning_table_v2};
use crate::repository::source_repository_v2::{create_source_table as create_source_table_v2,};
pub fn init_create_table() {
    let conn = establish_connection().expect("Failed to connect to database");
    create_earning_category_table(&conn);
    create_earning_table(&conn);
    create_source_table(&conn);
    create_spending_category_table(&conn);
    create_spending_table(&conn);
}

pub fn init_create_table_v2(){
    let mut conn: PooledConn = establish_connection_v2().expect("Failed to connect to database");
    create_spending_category_table_v2(&mut conn);
    create_source_table_v2(&mut conn);
    create_earning_category_table_v2(&mut conn);
    create_earning_table_v2(&mut conn);
    create_spending_table_v2(&mut conn);
}