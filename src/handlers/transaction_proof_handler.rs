use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use chrono::Local;
use uuid::Uuid;

use crate::helper::connection::establish_connection_v2;
use crate::models::responses::Response;
use crate::models::transaction_proof::{
    TransactionProof, TransactionProofInput, TransactionProofQuery,
};
use crate::repository::group_repository::is_group_member;
use crate::repository::group_settlement_repository::{select_reimbursement, select_split_payment};
use crate::repository::transaction_proof_repository::{
    deactivate_proof, insert_proof, select_proof, select_proofs_for, select_transaction_group,
    select_transaction_owner,
};
use crate::route_middleware::get_user::CreatedBy;

/// Largest image accepted, as base64 text (about 1.5 MB of JPEG). The app
/// compresses to well under this before uploading.
const MAX_BASE64_LEN: usize = 2_000_000;

const MIME_TYPES: [&str; 3] = ["image/jpeg", "image/png", "image/webp"];

fn ok_response(message: &str, data: Option<serde_json::Value>) -> Response {
    Response {
        status: "Success".to_string(),
        code: crate::helper::response_code::RESPONSE_CODE_DATA_RETRIEVAL_SUCCESS,
        message: message.to_string(),
        description: "".to_string(),
        data,
        success: true,
    }
}

fn err_response(message: &str, description: String) -> Response {
    Response {
        status: "Error".to_string(),
        code: crate::helper::response_code::ERROR_CODE_DATA_RETRIEVAL_FAILED,
        message: message.to_string(),
        description,
        data: None,
        success: false,
    }
}

fn caller(req: &HttpRequest) -> String {
    req.extensions().get::<CreatedBy>().unwrap().0.clone()
}

fn parse_id(raw: &str) -> Uuid {
    Uuid::parse_str(raw).unwrap_or_else(|_| Uuid::nil())
}

fn not_found() -> HttpResponse {
    HttpResponse::NotFound().json(err_response("Record not found", "".to_string()))
}

fn server_error(message: &str, err: Box<dyn std::error::Error>) -> HttpResponse {
    HttpResponse::InternalServerError().json(err_response(message, err.to_string()))
}

/// Decoded size of a base64 string, or `None` when it is not base64.
fn base64_size(text: &str) -> Option<i64> {
    let valid = !text.is_empty()
        && text.len() % 4 == 0
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=');
    if !valid {
        return None;
    }
    let padding = text.bytes().rev().take_while(|&b| b == b'=').count();
    Some((text.len() / 4 * 3 - padding) as i64)
}

/// Checks the caller may attach a proof to `ref_type`/`ref_id` and returns
/// the group whose members may see it. The group is always derived here,
/// never taken from the client.
fn upload_scope(
    conn: &mut mysql::PooledConn,
    ref_type: &str,
    ref_id: Uuid,
    username: &str,
) -> Result<Option<Uuid>, HttpResponse> {
    let is = |name: &str| name.eq_ignore_ascii_case(username);
    match ref_type {
        "spending" | "earning" => {
            match select_transaction_owner(conn, ref_type, ref_id) {
                Ok(Some(owner)) if is(&owner) => {}
                Ok(_) => return Err(not_found()),
                Err(err) => return Err(server_error("Failed to read transaction", err)),
            }
            select_transaction_group(conn, ref_type, ref_id)
                .map_err(|err| server_error("Failed to read transaction", err))
        }
        "reimbursement" => match select_reimbursement(conn, ref_id) {
            Ok(Some(r)) if is(&r.paid_by) || is(&r.owner) => Ok(Some(r.group_id)),
            Ok(_) => Err(not_found()),
            Err(err) => Err(server_error("Failed to read reimbursement", err)),
        },
        "split_payment" => match select_split_payment(conn, ref_id) {
            Ok(Some(p)) if p.paid_by.as_deref().is_some_and(is) || is(&p.owner) => {
                Ok(Some(p.group_id))
            }
            Ok(_) => Err(not_found()),
            Err(err) => Err(server_error("Failed to read split payment", err)),
        },
        _ => Err(HttpResponse::BadRequest().json(err_response(
            "Unknown ref_type",
            "Use spending, earning, reimbursement or split_payment.".to_string(),
        ))),
    }
}

/// The uploader, and the members of the proof's group, may see a proof.
fn can_view(
    conn: &mut mysql::PooledConn,
    proof: &TransactionProof,
    username: &str,
) -> Result<bool, HttpResponse> {
    if proof.uploaded_by.eq_ignore_ascii_case(username) {
        return Ok(true);
    }
    match proof.group_id {
        None => Ok(false),
        Some(group_id) => is_group_member(conn, group_id, username)
            .map_err(|err| server_error("Failed to read group", err)),
    }
}

/// `POST /api/user/proofs` - attaches a picture to one of the caller's
/// transactions, or to a reimbursement / split payment they take part in.
pub async fn post_transaction_proof_api(
    req: HttpRequest,
    body: web::Json<TransactionProofInput>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let ref_type = body.ref_type.trim().to_lowercase();
    let mime_type = body.mime_type.trim().to_lowercase();
    if !MIME_TYPES.contains(&mime_type.as_str()) {
        return HttpResponse::BadRequest().json(err_response(
            "Unsupported image type",
            "Send a JPEG, PNG or WebP image.".to_string(),
        ));
    }
    let image = body.image_base64.trim();
    if image.len() > MAX_BASE64_LEN {
        return HttpResponse::PayloadTooLarge()
            .json(err_response("Image is too large", "".to_string()));
    }
    let Some(size_bytes) = base64_size(image) else {
        return HttpResponse::BadRequest()
            .json(err_response("Image must be base64", "".to_string()));
    };

    let proof_id = body.proof_id.unwrap_or_else(Uuid::new_v4);
    match select_proof(&mut conn, proof_id, false) {
        Ok(Some(existing)) if existing.uploaded_by.eq_ignore_ascii_case(&username) => {
            return HttpResponse::Ok().json(ok_response(
                "Proof uploaded",
                Some(serde_json::to_value(existing).unwrap()),
            ));
        }
        Ok(Some(_)) => {
            return HttpResponse::Conflict()
                .json(err_response("Proof id already used", "".to_string()));
        }
        Ok(None) => {}
        Err(err) => return server_error("Failed to upload proof", err),
    }

    let group_id = match upload_scope(&mut conn, &ref_type, body.ref_id, &username) {
        Ok(group_id) => group_id,
        Err(response) => return response,
    };
    let proof = TransactionProof {
        proof_id,
        ref_type,
        ref_id: body.ref_id,
        group_id,
        mime_type,
        size_bytes,
        uploaded_by: username,
        created_date: Local::now().naive_local(),
        image_base64: None,
    };
    match insert_proof(&mut conn, &proof, image) {
        Ok(_) => HttpResponse::Ok().json(ok_response(
            "Proof uploaded",
            Some(serde_json::to_value(proof).unwrap()),
        )),
        Err(err) => server_error("Failed to upload proof", err),
    }
}

/// `GET /api/user/proofs?ref_type=&ref_id=` - the proofs of one record the
/// caller may see, without their images.
pub async fn get_transaction_proofs_api(
    req: HttpRequest,
    query: web::Query<TransactionProofQuery>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let ref_type = query.ref_type.trim().to_lowercase();
    let proofs = match select_proofs_for(&mut conn, &ref_type, parse_id(&query.ref_id)) {
        Ok(proofs) => proofs,
        Err(err) => return server_error("Failed to read proofs", err),
    };
    let mut visible = Vec::with_capacity(proofs.len());
    for proof in proofs {
        match can_view(&mut conn, &proof, &username) {
            Ok(true) => visible.push(proof),
            Ok(false) => {}
            Err(response) => return response,
        }
    }
    HttpResponse::Ok().json(ok_response(
        "Success get proofs",
        Some(serde_json::to_value(visible).unwrap()),
    ))
}

/// `GET /api/user/proofs/{proof_id}` - one proof with its image.
pub async fn get_transaction_proof_api(req: HttpRequest, path: web::Path<String>) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let proof = match select_proof(&mut conn, parse_id(&path.into_inner()), true) {
        Ok(Some(proof)) => proof,
        Ok(None) => return not_found(),
        Err(err) => return server_error("Failed to read proof", err),
    };
    match can_view(&mut conn, &proof, &username) {
        Ok(true) => HttpResponse::Ok().json(ok_response(
            "Success get proof",
            Some(serde_json::to_value(proof).unwrap()),
        )),
        Ok(false) => not_found(),
        Err(response) => response,
    }
}

/// `DELETE /api/user/proofs/{proof_id}` - only the uploader can remove one.
pub async fn delete_transaction_proof_api(
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let mut conn = establish_connection_v2().expect("Failed to connect to database");
    let username = caller(&req);
    let proof_id = parse_id(&path.into_inner());
    match select_proof(&mut conn, proof_id, false) {
        Ok(Some(proof)) if proof.uploaded_by.eq_ignore_ascii_case(&username) => {}
        Ok(_) => return not_found(),
        Err(err) => return server_error("Failed to read proof", err),
    }
    match deactivate_proof(&mut conn, proof_id) {
        Ok(_) => HttpResponse::Ok().json(ok_response("Proof removed", None)),
        Err(err) => server_error("Failed to remove proof", err),
    }
}
