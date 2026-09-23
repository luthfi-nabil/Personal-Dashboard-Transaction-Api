use mysql::prelude::*;
use mysql::*;
use std::error::Error;
use uuid::Uuid;

use crate::models::member_transfer::{MemberSource, MemberTransfer};

fn parse_uuid(value: &str) -> Uuid {
    Uuid::parse_str(value).unwrap_or_else(|_| Uuid::nil())
}

pub fn create_member_transfer_table(conn: &mut PooledConn) -> Result<()> {
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS member_transfer (
            transfer_id CHAR(36) PRIMARY KEY,
            from_user VARCHAR(255) NOT NULL,
            from_source_id CHAR(36) NOT NULL,
            from_source VARCHAR(255) NOT NULL,
            to_user VARCHAR(255) NOT NULL,
            to_source_id CHAR(36) NOT NULL,
            to_source VARCHAR(255) NOT NULL,
            amount DOUBLE NOT NULL,
            description TEXT,
            spending_id CHAR(36) NOT NULL,
            earning_id CHAR(36) NOT NULL,
            created_date DATETIME NOT NULL,
            INDEX idx_member_transfer_from (from_user),
            INDEX idx_member_transfer_to (to_user)
        )",
    )?;
    Ok(())
}

/// `other` exactly as stored in a group `username` also belongs to, or `None`
/// when the two share no group. Transfers only go between group mates.
pub fn select_shared_member(
    conn: &mut PooledConn,
    username: &str,
    other: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    let found: Option<String> = conn.exec_first(
        "SELECT m.username FROM spending_group_member m
         WHERE LOWER(m.username) = LOWER(:other)
           AND m.group_id IN (SELECT group_id FROM spending_group_member WHERE username = :username)
         LIMIT 1",
        params! { "other" => other, "username" => username },
    )?;
    Ok(found)
}

pub fn select_member_sources(
    conn: &mut PooledConn,
    username: &str,
) -> Result<Vec<MemberSource>, Box<dyn Error>> {
    let rows = conn.exec_map(
        "SELECT source_id, source FROM source
         WHERE created_by = :username AND is_active = 1
         ORDER BY source ASC",
        params! { "username" => username },
        |(source_id, source): (String, String)| MemberSource {
            source_id: parse_uuid(&source_id),
            source,
        },
    )?;
    Ok(rows)
}

pub fn select_member_transfer(
    conn: &mut PooledConn,
    transfer_id: Uuid,
) -> Result<Option<MemberTransfer>, Box<dyn Error>> {
    let row: Option<Row> = conn.exec_first(
        format!("{SELECT_TRANSFER} WHERE transfer_id = :id"),
        params! { "id" => transfer_id.to_string() },
    )?;
    Ok(row.map(transfer_from_row))
}

/// Every transfer `username` sent or received, newest first.
pub fn select_member_transfers(
    conn: &mut PooledConn,
    username: &str,
) -> Result<Vec<MemberTransfer>, Box<dyn Error>> {
    let rows = conn.exec_map(
        format!(
            "{SELECT_TRANSFER} WHERE from_user = :u1 OR to_user = :u2 ORDER BY created_date DESC"
        ),
        params! { "u1" => username, "u2" => username },
        transfer_from_row,
    )?;
    Ok(rows)
}

const SELECT_TRANSFER: &str = "SELECT transfer_id, from_user, from_source_id, from_source,
    to_user, to_source_id, to_source, amount, COALESCE(description, ''), spending_id,
    earning_id, created_date FROM member_transfer";

fn transfer_from_row(row: Row) -> MemberTransfer {
    let text = |i: usize| -> String { row.get::<String, _>(i).unwrap_or_default() };
    MemberTransfer {
        transfer_id: parse_uuid(&text(0)),
        from_user: text(1),
        from_source_id: parse_uuid(&text(2)),
        from_source: text(3),
        to_user: text(4),
        to_source_id: parse_uuid(&text(5)),
        to_source: text(6),
        amount: row.get(7).unwrap_or(0.0),
        description: text(8),
        spending_id: parse_uuid(&text(9)),
        earning_id: parse_uuid(&text(10)),
        created_date: row.get(11).unwrap(),
    }
}

/// The Transfer category both halves are filed under.
pub struct TransferCategory<'a> {
    pub id: Uuid,
    pub name: &'a str,
}

/// Writes the sender's spending, the recipient's earning and the transfer
/// record in one DB transaction, so money never leaves one side without
/// arriving on the other.
pub fn insert_member_transfer(
    conn: &mut PooledConn,
    transfer: &MemberTransfer,
    category: &TransferCategory,
    sender_description: &str,
    recipient_description: &str,
) -> Result<(), Box<dyn Error>> {
    let created = transfer.created_date.to_string();
    let mut tx = conn.start_transaction(TxOpts::default())?;
    tx.exec_drop(
        "INSERT INTO spending
            (spending_id, total_amount, description, spending_category_id, spending_category,
             source_id, source, created_date, created_by, is_active)
         VALUES (:id, :total, :description, :cat_id, :cat, :src_id, :src, :created, :by, 1)",
        params! {
            "id" => transfer.spending_id.to_string(),
            "total" => transfer.amount,
            "description" => sender_description,
            "cat_id" => category.id.to_string(),
            "cat" => category.name,
            "src_id" => transfer.from_source_id.to_string(),
            "src" => &transfer.from_source,
            "created" => &created,
            "by" => &transfer.from_user,
        },
    )?;
    tx.exec_drop(
        "INSERT INTO earning
            (earning_id, total_amount, description, earning_category_id, earning_category,
             source_id, source, created_date, created_by, is_active)
         VALUES (:id, :total, :description, :cat_id, :cat, :src_id, :src, :created, :by, 1)",
        params! {
            "id" => transfer.earning_id.to_string(),
            "total" => transfer.amount,
            "description" => recipient_description,
            "cat_id" => category.id.to_string(),
            "cat" => category.name,
            "src_id" => transfer.to_source_id.to_string(),
            "src" => &transfer.to_source,
            "created" => &created,
            "by" => &transfer.to_user,
        },
    )?;
    tx.exec_drop(
        "INSERT INTO member_transfer
            (transfer_id, from_user, from_source_id, from_source, to_user, to_source_id,
             to_source, amount, description, spending_id, earning_id, created_date)
         VALUES (:id, :from_user, :from_source_id, :from_source, :to_user, :to_source_id,
                 :to_source, :amount, :description, :spending_id, :earning_id, :created)",
        params! {
            "id" => transfer.transfer_id.to_string(),
            "from_user" => &transfer.from_user,
            "from_source_id" => transfer.from_source_id.to_string(),
            "from_source" => &transfer.from_source,
            "to_user" => &transfer.to_user,
            "to_source_id" => transfer.to_source_id.to_string(),
            "to_source" => &transfer.to_source,
            "amount" => transfer.amount,
            "description" => &transfer.description,
            "spending_id" => transfer.spending_id.to_string(),
            "earning_id" => transfer.earning_id.to_string(),
            "created" => &created,
        },
    )?;
    tx.commit()?;
    Ok(())
}
