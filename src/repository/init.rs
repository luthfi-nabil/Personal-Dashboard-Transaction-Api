use mysql::PooledConn;

use crate::helper::connection::establish_connection_v2;
use crate::repository::app_setting_repository::{ensure_default_settings, init_setting_table};
use crate::repository::earning_repository_v2::{
    create_earning_category_table as create_earning_category_table_v2,
    create_earning_table as create_earning_table_v2,
};
use crate::repository::routine_repository::{create_routine_payment_table, create_routine_table};
use crate::repository::source_repository_v2::create_source_table as create_source_table_v2;
use crate::repository::spending_repository_v2::{
    create_spending_category_table as create_spending_category_table_v2,
    create_spending_table as create_spending_table_v2,
};
use crate::repository::wishlist_repository::{
    create_planned_expense_category_table, create_wishlist_table,
};
pub fn init_create_table_v2() {
    let mut conn: PooledConn = establish_connection_v2().expect("Failed to connect to database");
    create_spending_category_table_v2(&mut conn);
    create_source_table_v2(&mut conn);
    create_earning_category_table_v2(&mut conn);
    create_earning_table_v2(&mut conn);
    create_spending_table_v2(&mut conn);
    create_planned_expense_category_table(&mut conn)
        .expect("Failed to initialize planned expense category table");
    create_wishlist_table(&mut conn).expect("Failed to initialize wishlist table");
    create_routine_table(&mut conn).expect("Failed to initialize routine transaction table");
    create_routine_payment_table(&mut conn).expect("Failed to initialize routine payment table");
    init_setting_table(&mut conn).expect("Failed to initialize app settings table");
    ensure_default_settings(&mut conn).expect("Failed to ensure default app settings");
}
