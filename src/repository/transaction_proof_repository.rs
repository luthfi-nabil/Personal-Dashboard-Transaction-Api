use mysql::prelude::*;
use mysql::*;
use std::error::Error;
use uuid::Uuid;

use crate::models::transaction_proof::TransactionProof;

fn parse_uuid(value: &str) -> Uuid {
    Uuid::parse_str(value).unwrap_or_else(|_| Uuid::nil())
}

pub fn create_transaction_proof_table(conn: &mut PooledConn) -> Result<()> {
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS transaction_proof (
            proof_id CHAR(36) PRIMARY KEY,
            ref_type VARCHAR(32) NOT NULL,
            ref_id CHAR(36) NOT NULL,
            group_id CHAR(36) NULL,
            mime_type VARCHAR(64) NOT NULL,
            size_bytes BIGINT NOT NULL,
            image_base64 LONGTEXT NOT NULL,
            uploaded_by VARCHAR(255) NOT NULL,
            created_date DATETIME NOT NULL,
            is_active TINYINT NOT NULL DEFAULT 1,
            INDEX idx_transaction_proof_ref (ref_type, ref_id)
        )",
    )?;
    Ok(())
}

const SELECT_PROOF: &str = "SELECT proof_id, ref_type, ref_id, group_id, mime_type,
    size_bytes, uploaded_by, created_date FROM transaction_proof";

fn proof_from_row(row: Row) -> TransactionProof {
    let text = |i: usize| -> String { row.get::<String, _>(i).unwrap_or_default() };
    TransactionProof {
        proof_id: parse_uuid(&text(0)),
        ref_type: text(1),
        ref_id: parse_uuid(&text(2)),
        group_id: row
            .get::<Option<String>, _>(3)
            .flatten()
            .map(|id| parse_uuid(&id)),
        mime_type: text(4),
        size_bytes: row.get(5).unwrap_or(0),
        uploaded_by: text(6),
        created_date: row.get(7).unwrap(),
        image_base64: None,
    }
}

/// Who created an active spending or earning, or `None` when there is none.
/// `ref_type` must already be validated as `spending` or `earning`.
pub fn select_transaction_owner(
    conn: &mut PooledConn,
    ref_type: &str,
    ref_id: Uuid,
) -> Result<Option<String>, Box<dyn Error>> {
    let sql = if ref_type == "earning" {
        "SELECT created_by FROM earning WHERE earning_id = :id AND is_active = 1"
    } else {
        "SELECT created_by FROM spending WHERE spending_id = :id AND is_active = 1"
    };
    let owner: Option<String> = conn.exec_first(sql, params! { "id" => ref_id.to_string() })?;
    Ok(owner)
}

/// The group a spending/earning is tagged into, if any.
pub fn select_transaction_group(
    conn: &mut PooledConn,
    ref_type: &str,
    ref_id: Uuid,
) -> Result<Option<Uuid>, Box<dyn Error>> {
    let group: Option<String> = conn.exec_first(
        "SELECT group_id FROM spending_group_transaction
         WHERE transaction_type = :type AND transaction_id = :id",
        params! { "type" => ref_type, "id" => ref_id.to_string() },
    )?;
    Ok(group.map(|g| parse_uuid(&g)))
}

pub fn select_proof(
    conn: &mut PooledConn,
    proof_id: Uuid,
    with_image: bool,
) -> Result<Option<TransactionProof>, Box<dyn Error>> {
    let row: Option<Row> = conn.exec_first(
        format!("{SELECT_PROOF} WHERE proof_id = :id AND is_active = 1"),
        params! { "id" => proof_id.to_string() },
    )?;
    let Some(mut proof) = row.map(proof_from_row) else {
        return Ok(None);
    };
    if with_image {
        let image: Option<String> = conn.exec_first(
            "SELECT image_base64 FROM transaction_proof WHERE proof_id = :id",
            params! { "id" => proof_id.to_string() },
        )?;
        proof.image_base64 = image;
    }
    Ok(Some(proof))
}

/// Active proofs of one record, oldest first, without their images.
pub fn select_proofs_for(
    conn: &mut PooledConn,
    ref_type: &str,
    ref_id: Uuid,
) -> Result<Vec<TransactionProof>, Box<dyn Error>> {
    let rows: Vec<Row> = conn.exec(
        format!(
            "{SELECT_PROOF} WHERE ref_type = :type AND ref_id = :id AND is_active = 1
             ORDER BY created_date"
        ),
        params! { "type" => ref_type, "id" => ref_id.to_string() },
    )?;
    Ok(rows.into_iter().map(proof_from_row).collect())
}

pub fn insert_proof(
    conn: &mut PooledConn,
    proof: &TransactionProof,
    image_base64: &str,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "INSERT INTO transaction_proof
            (proof_id, ref_type, ref_id, group_id, mime_type, size_bytes,
             image_base64, uploaded_by, created_date, is_active)
         VALUES (:id, :type, :ref_id, :group_id, :mime, :size, :image, :by, :created, 1)",
        params! {
            "id" => proof.proof_id.to_string(),
            "type" => &proof.ref_type,
            "ref_id" => proof.ref_id.to_string(),
            "group_id" => proof.group_id.map(|g| g.to_string()),
            "mime" => &proof.mime_type,
            "size" => proof.size_bytes,
            "image" => image_base64,
            "by" => &proof.uploaded_by,
            "created" => proof.created_date.to_string(),
        },
    )?;
    Ok(())
}

pub fn deactivate_proof(conn: &mut PooledConn, proof_id: Uuid) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "UPDATE transaction_proof SET is_active = 0, image_base64 = ''
         WHERE proof_id = :id",
        params! { "id" => proof_id.to_string() },
    )?;
    Ok(())
}
