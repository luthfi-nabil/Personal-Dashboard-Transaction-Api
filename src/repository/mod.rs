use mysql::prelude::*;
use mysql::*;

pub mod activity_repository;
pub mod consumable_repository;
pub mod debt_repository;
pub mod earning_repository_v2;
pub mod init;
pub mod investment_repository;
pub mod routine_repository;
pub mod source_repository_v2;
pub mod spending_repository_v2;
pub mod wishlist_repository;

/// Runs `alter_sql` only when `column` is not already on `table`.
///
/// The tables here are created by the API itself (`CREATE TABLE IF NOT EXISTS`)
/// rather than by a migration tool, so a new column has to be added on boot for
/// databases that predate it.
pub(crate) fn add_column_if_missing(
    conn: &mut PooledConn,
    table: &str,
    column: &str,
    alter_sql: &str,
) -> Result<()> {
    let exists: Option<u8> = conn.exec_first(
        "SELECT 1
         FROM INFORMATION_SCHEMA.COLUMNS
         WHERE TABLE_SCHEMA = DATABASE()
           AND TABLE_NAME = :table
           AND COLUMN_NAME = :column
         LIMIT 1",
        params! {
            "table" => table,
            "column" => column,
        },
    )?;
    if exists.is_none() {
        conn.query_drop(alter_sql)?;
    }
    Ok(())
}
