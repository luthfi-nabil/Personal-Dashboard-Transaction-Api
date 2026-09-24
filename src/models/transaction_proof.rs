use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A picture proving a transaction or a group settlement happened - a
/// transfer receipt, a photo of the bill.
///
/// `ref_type` says what it belongs to: `spending` / `earning` (one of the
/// uploader's own transactions), `reimbursement` or `split_payment`. The
/// image is stored as the base64 the client sent (already compressed to a
/// JPEG on the device) and only returned by the single-proof endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionProof {
    pub proof_id: Uuid,
    pub ref_type: String,
    pub ref_id: Uuid,
    /// The group whose members may also see it: the group a transaction is
    /// tagged into, or the settlement's group. `None` = uploader only.
    pub group_id: Option<Uuid>,
    pub mime_type: String,
    pub size_bytes: i64,
    pub uploaded_by: String,
    pub created_date: NaiveDateTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_base64: Option<String>,
}

/// Request body for `POST /api/user/proofs`. `proof_id` is client-generated
/// so a retried upload is stored once.
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionProofInput {
    pub proof_id: Option<Uuid>,
    pub ref_type: String,
    pub ref_id: Uuid,
    #[serde(default = "default_mime")]
    pub mime_type: String,
    pub image_base64: String,
}

fn default_mime() -> String {
    "image/jpeg".to_string()
}

/// Query of `GET /api/user/proofs`.
#[derive(Debug, Deserialize)]
pub struct TransactionProofQuery {
    pub ref_type: String,
    pub ref_id: String,
}
