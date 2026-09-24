use chrono::NaiveDateTime;
use mysql::prelude::*;
use mysql::*;
use std::collections::HashMap;
use std::error::Error;
use uuid::Uuid;

use crate::models::group_fund::{FundUsage, GroupFundRequest, TagBalance};
use crate::models::member_transfer::MemberTransfer;
use crate::repository::member_transfer_repository::{TransferCategory, write_member_transfer};

fn parse_uuid(value: &str) -> Uuid {
    Uuid::parse_str(value).unwrap_or_else(|_| Uuid::nil())
}

fn parse_opt_uuid(value: Option<String>) -> Option<Uuid> {
    value.and_then(|v| Uuid::parse_str(&v).ok())
}

/// Leaves room for float rounding when a fund is used up to zero.
const EPSILON: f64 = 0.000_001;

pub fn create_group_fund_table(conn: &mut PooledConn) -> Result<()> {
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS group_fund_request (
            request_id CHAR(36) PRIMARY KEY,
            group_id CHAR(36) NOT NULL,
            kind VARCHAR(16) NOT NULL,
            requester VARCHAR(255) NOT NULL,
            payer VARCHAR(255) NOT NULL,
            amount DOUBLE NOT NULL,
            tag VARCHAR(255) NOT NULL,
            note TEXT,
            tracked TINYINT(1) NOT NULL DEFAULT 0,
            status VARCHAR(16) NOT NULL,
            to_source_id CHAR(36) NULL,
            to_source VARCHAR(255) NULL,
            from_source_id CHAR(36) NULL,
            from_source VARCHAR(255) NULL,
            transfer_id CHAR(36) NULL,
            created_by VARCHAR(255) NOT NULL,
            created_date DATETIME NOT NULL,
            responded_at DATETIME NULL,
            sent_at DATETIME NULL,
            waived_by VARCHAR(255) NULL,
            waived_at DATETIME NULL,
            waive_note TEXT NULL,
            INDEX idx_group_fund_group (group_id),
            INDEX idx_group_fund_requester (requester),
            INDEX idx_group_fund_payer (payer)
        )",
    )?;
    Ok(())
}

const SELECT_FUND: &str = "SELECT r.request_id, r.group_id, r.kind, r.requester, r.payer,
    r.amount, r.tag, COALESCE(r.note, ''), r.tracked, r.status, r.to_source_id, r.to_source,
    r.from_source_id, r.from_source, r.transfer_id, r.created_by, r.created_date,
    r.responded_at, r.sent_at, r.waived_by, r.waived_at, r.waive_note
    FROM group_fund_request r";

fn fund_from_row(row: Row) -> GroupFundRequest {
    let text = |i: usize| -> String { row.get::<Option<String>, _>(i).flatten().unwrap_or_default() };
    let opt = |i: usize| -> Option<String> { row.get::<Option<String>, _>(i).flatten() };
    let date = |i: usize| -> Option<NaiveDateTime> {
        row.get::<Option<NaiveDateTime>, _>(i).flatten()
    };
    GroupFundRequest {
        request_id: parse_uuid(&text(0)),
        group_id: parse_uuid(&text(1)),
        kind: text(2),
        requester: text(3),
        payer: text(4),
        amount: row.get(5).unwrap_or(0.0),
        tag: text(6),
        note: text(7),
        tracked: row.get::<i64, _>(8).unwrap_or(0) != 0,
        status: text(9),
        to_source_id: parse_opt_uuid(opt(10)),
        to_source: opt(11),
        from_source_id: parse_opt_uuid(opt(12)),
        from_source: opt(13),
        transfer_id: parse_opt_uuid(opt(14)),
        created_by: text(15),
        created_date: date(16).unwrap_or_default(),
        responded_at: date(17),
        sent_at: date(18),
        waived_by: opt(19),
        waived_at: date(20),
        waive_note: opt(21),
        spent: 0.0,
        remaining: 0.0,
        waived_amount: 0.0,
        usage: Vec::new(),
    }
}

pub fn select_fund_request<Q: Queryable>(
    conn: &mut Q,
    request_id: Uuid,
) -> Result<Option<GroupFundRequest>, Box<dyn Error>> {
    let row: Option<Row> = conn.exec_first(
        format!("{SELECT_FUND} WHERE r.request_id = :id"),
        params! { "id" => request_id.to_string() },
    )?;
    Ok(row.map(fund_from_row))
}

/// The requests `username` may see, across every group they are in: in a
/// group they lead, all of them; elsewhere, the ones they asked or were
/// asked. Newest first, with tracked usage filled in.
pub fn select_fund_requests_for_user(
    conn: &mut PooledConn,
    username: &str,
) -> Result<Vec<GroupFundRequest>, Box<dyn Error>> {
    let mut rows: Vec<GroupFundRequest> = conn.exec_map(
        format!(
            "{SELECT_FUND}
             JOIN spending_group g ON g.group_id = r.group_id
             WHERE r.group_id IN (SELECT group_id FROM spending_group_member
                                  WHERE LOWER(username) = LOWER(:u1))
               AND (LOWER(g.leader) = LOWER(:u2) OR LOWER(r.requester) = LOWER(:u3)
                    OR LOWER(r.payer) = LOWER(:u4))
             ORDER BY r.created_date DESC"
        ),
        params! { "u1" => username, "u2" => username, "u3" => username, "u4" => username },
        fund_from_row,
    )?;
    fill_usage(conn, &mut rows)?;
    Ok(rows)
}

/// `other` exactly as stored in `group_id`'s member list, if they are in it.
pub fn select_group_member_name(
    conn: &mut PooledConn,
    group_id: Uuid,
    other: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    let found: Option<String> = conn.exec_first(
        "SELECT username FROM spending_group_member
         WHERE group_id = :id AND LOWER(username) = LOWER(:other) LIMIT 1",
        params! { "id" => group_id.to_string(), "other" => other },
    )?;
    Ok(found)
}

fn write_fund_row<Q: Queryable>(tx: &mut Q, r: &GroupFundRequest) -> Result<(), Box<dyn Error>> {
    tx.exec_drop(
        "INSERT INTO group_fund_request
            (request_id, group_id, kind, requester, payer, amount, tag, note, tracked, status,
             to_source_id, to_source, from_source_id, from_source, transfer_id, created_by,
             created_date, responded_at, sent_at)
         VALUES (:id, :group_id, :kind, :requester, :payer, :amount, :tag, :note, :tracked,
                 :status, :to_source_id, :to_source, :from_source_id, :from_source,
                 :transfer_id, :created_by, :created_date, :responded_at, :sent_at)",
        params! {
            "id" => r.request_id.to_string(),
            "group_id" => r.group_id.to_string(),
            "kind" => &r.kind,
            "requester" => &r.requester,
            "payer" => &r.payer,
            "amount" => r.amount,
            "tag" => &r.tag,
            "note" => &r.note,
            "tracked" => r.tracked as i32,
            "status" => &r.status,
            "to_source_id" => r.to_source_id.map(|v| v.to_string()),
            "to_source" => r.to_source.clone(),
            "from_source_id" => r.from_source_id.map(|v| v.to_string()),
            "from_source" => r.from_source.clone(),
            "transfer_id" => r.transfer_id.map(|v| v.to_string()),
            "created_by" => &r.created_by,
            "created_date" => r.created_date.to_string(),
            "responded_at" => r.responded_at.map(|v| v.to_string()),
            "sent_at" => r.sent_at.map(|v| v.to_string()),
        },
    )?;
    Ok(())
}

/// Saves a new request that waits for the payer.
pub fn insert_fund_request(
    conn: &mut PooledConn,
    r: &GroupFundRequest,
) -> Result<(), Box<dyn Error>> {
    write_fund_row(conn, r)
}

/// The descriptions written on the payer's spending and the recipient's
/// earning for a fund.
pub struct FundTexts {
    pub payer: String,
    pub receiver: String,
}

/// A direct send: the transfer and the fund row, all or nothing.
pub fn insert_sent_fund(
    conn: &mut PooledConn,
    r: &GroupFundRequest,
    transfer: &MemberTransfer,
    category: &TransferCategory,
    texts: &FundTexts,
) -> Result<(), Box<dyn Error>> {
    let mut tx = conn.start_transaction(TxOpts::default())?;
    write_member_transfer(&mut tx, transfer, category, &texts.payer, &texts.receiver)?;
    write_fund_row(&mut tx, r)?;
    tx.commit()?;
    Ok(())
}

/// Why a status change wrote nothing.
#[derive(Debug, PartialEq)]
pub enum FundOutcome {
    Done,
    /// It is no longer in the state the action needs.
    WrongStatus(String),
    Gone,
}

/// The payer sends a waiting request: the transfer and the status change in
/// one DB transaction, under a row lock, so it can only be paid once.
pub fn fulfil_fund_request(
    conn: &mut PooledConn,
    request_id: Uuid,
    transfer: &MemberTransfer,
    category: &TransferCategory,
    texts: &FundTexts,
) -> Result<FundOutcome, Box<dyn Error>> {
    let mut tx = conn.start_transaction(TxOpts::default())?;
    let status: Option<String> = tx.exec_first(
        "SELECT status FROM group_fund_request WHERE request_id = :id FOR UPDATE",
        params! { "id" => request_id.to_string() },
    )?;
    match status {
        None => {
            tx.rollback()?;
            return Ok(FundOutcome::Gone);
        }
        Some(s) if s != "requested" => {
            tx.rollback()?;
            return Ok(FundOutcome::WrongStatus(s));
        }
        Some(_) => {}
    }
    write_member_transfer(&mut tx, transfer, category, &texts.payer, &texts.receiver)?;
    tx.exec_drop(
        "UPDATE group_fund_request
         SET status = 'sent', from_source_id = :from_id, from_source = :from_name,
             to_source_id = :to_id, to_source = :to_name, transfer_id = :transfer_id,
             responded_at = :at, sent_at = :at2
         WHERE request_id = :id",
        params! {
            "from_id" => transfer.from_source_id.to_string(),
            "from_name" => &transfer.from_source,
            "to_id" => transfer.to_source_id.to_string(),
            "to_name" => &transfer.to_source,
            "transfer_id" => transfer.transfer_id.to_string(),
            "at" => transfer.created_date.to_string(),
            "at2" => transfer.created_date.to_string(),
            "id" => request_id.to_string(),
        },
    )?;
    tx.commit()?;
    Ok(FundOutcome::Done)
}

/// Moves a waiting request to `rejected` or `canceled`.
pub fn close_fund_request(
    conn: &mut PooledConn,
    request_id: Uuid,
    status: &str,
    at: NaiveDateTime,
) -> Result<FundOutcome, Box<dyn Error>> {
    conn.exec_drop(
        "UPDATE group_fund_request SET status = :status, responded_at = :at
         WHERE request_id = :id AND status = 'requested'",
        params! { "status" => status, "at" => at.to_string(), "id" => request_id.to_string() },
    )?;
    if conn.affected_rows() == 1 {
        return Ok(FundOutcome::Done);
    }
    Ok(match select_fund_request(conn, request_id)? {
        Some(r) => FundOutcome::WrongStatus(r.status),
        None => FundOutcome::Gone,
    })
}

/// The leader waives a sent, tracked fund. Only the earmark goes: the
/// transfer and the recipient's spendings stay as they are.
pub fn waive_fund_request(
    conn: &mut PooledConn,
    request_id: Uuid,
    by: &str,
    note: &str,
    at: NaiveDateTime,
) -> Result<FundOutcome, Box<dyn Error>> {
    conn.exec_drop(
        "UPDATE group_fund_request
         SET status = 'waived', waived_by = :by, waived_at = :at, waive_note = :note
         WHERE request_id = :id AND status = 'sent' AND tracked = 1",
        params! { "by" => by, "at" => at.to_string(), "note" => note, "id" => request_id.to_string() },
    )?;
    if conn.affected_rows() == 1 {
        return Ok(FundOutcome::Done);
    }
    Ok(match select_fund_request(conn, request_id)? {
        Some(r) if r.status == "sent" => FundOutcome::WrongStatus("untracked".to_string()),
        Some(r) => FundOutcome::WrongStatus(r.status),
        None => FundOutcome::Gone,
    })
}

// ── Tracking ─────────────────────────────────────────────────────────────

/// One of the recipient's spendings in a tag's category.
#[derive(Debug, Clone)]
pub struct TagSpending {
    pub spending_id: Uuid,
    pub description: String,
    pub amount: f64,
    pub created_date: NaiveDateTime,
}

/// One tracked fund as the allocation sees it.
#[derive(Debug, Clone)]
pub struct FundWindow {
    pub amount: f64,
    pub sent_at: NaiveDateTime,
    /// A waived fund stops taking spendings at the moment it was waived.
    pub closed_at: Option<NaiveDateTime>,
}

/// Uses `funds` up with `spendings`, oldest fund first (FIFO).
///
/// A spending only counts against funds that were already sent when it
/// happened (and not yet waived); whatever part of it no fund covers was
/// paid with the recipient's own money. Returns, per fund, how much was
/// spent and from which spendings. Pure, so it is unit-tested below.
pub fn allocate_funds(funds: &[FundWindow], spendings: &[TagSpending]) -> Vec<(f64, Vec<FundUsage>)> {
    let mut order: Vec<usize> = (0..funds.len()).collect();
    order.sort_by_key(|&i| funds[i].sent_at);
    let mut left: Vec<f64> = funds.iter().map(|f| f.amount).collect();
    let mut out: Vec<(f64, Vec<FundUsage>)> = funds.iter().map(|_| (0.0, Vec::new())).collect();

    let mut sorted: Vec<&TagSpending> = spendings.iter().collect();
    sorted.sort_by_key(|s| s.created_date);
    for s in sorted {
        let mut rest = s.amount;
        for &i in &order {
            if rest <= EPSILON {
                break;
            }
            let f = &funds[i];
            let open = s.created_date >= f.sent_at && f.closed_at.is_none_or(|c| s.created_date < c);
            if !open || left[i] <= EPSILON {
                continue;
            }
            let take = rest.min(left[i]);
            left[i] -= take;
            rest -= take;
            out[i].0 += take;
            out[i].1.push(FundUsage {
                spending_id: s.spending_id,
                description: s.description.clone(),
                spending_amount: s.amount,
                amount: take,
                created_date: s.created_date,
            });
        }
    }
    out
}

/// Fills `spent` / `remaining` / `waived_amount` / `usage` on the tracked,
/// sent or waived rows of `rows`. Every one of the recipient's tracked
/// funds with the same tag takes part - across all their groups - so a
/// spending is never counted against two funds.
fn fill_usage(conn: &mut PooledConn, rows: &mut [GroupFundRequest]) -> Result<(), Box<dyn Error>> {
    let counts = |r: &GroupFundRequest| r.tracked && (r.status == "sent" || r.status == "waived");
    let mut keys: Vec<(String, String)> = rows
        .iter()
        .filter(|r| counts(r))
        .map(|r| (r.requester.to_lowercase(), r.tag.trim().to_lowercase()))
        .collect();
    keys.sort();
    keys.dedup();

    let mut results: HashMap<Uuid, (f64, Vec<FundUsage>)> = HashMap::new();
    for (user, tag) in keys {
        let funds: Vec<GroupFundRequest> = conn.exec_map(
            format!(
                "{SELECT_FUND}
                 WHERE LOWER(r.requester) = :u AND LOWER(TRIM(r.tag)) = :t AND r.tracked = 1
                   AND r.status IN ('sent', 'waived') AND r.sent_at IS NOT NULL"
            ),
            params! { "u" => &user, "t" => &tag },
            fund_from_row,
        )?;
        let Some(since) = funds.iter().filter_map(|f| f.sent_at).min() else {
            continue;
        };
        let spendings: Vec<TagSpending> = conn.exec_map(
            "SELECT spending_id, COALESCE(description, ''), total_amount, created_date
             FROM spending
             WHERE LOWER(created_by) = :u AND is_active = 1
               AND LOWER(TRIM(spending_category)) = :t AND created_date >= :since
             ORDER BY created_date, spending_id",
            params! { "u" => &user, "t" => &tag, "since" => since.to_string() },
            |(id, description, amount, created_date): (String, String, f64, NaiveDateTime)| {
                TagSpending {
                    spending_id: parse_uuid(&id),
                    description,
                    amount,
                    created_date,
                }
            },
        )?;
        let windows: Vec<FundWindow> = funds
            .iter()
            .map(|f| FundWindow {
                amount: f.amount,
                sent_at: f.sent_at.unwrap_or(f.created_date),
                closed_at: if f.status == "waived" { f.waived_at } else { None },
            })
            .collect();
        for (fund, used) in funds.iter().zip(allocate_funds(&windows, &spendings)) {
            results.insert(fund.request_id, used);
        }
    }

    for r in rows.iter_mut().filter(|r| counts(r)) {
        let (spent, usage) = results.remove(&r.request_id).unwrap_or_default();
        let unspent = (r.amount - spent).max(0.0);
        r.spent = spent;
        r.usage = usage;
        if r.status == "waived" {
            r.remaining = 0.0;
            r.waived_amount = unspent;
        } else {
            r.remaining = unspent;
        }
    }
    Ok(())
}

/// Per group, member and tag: what their tracked funds add up to.
pub fn tag_balances(rows: &[GroupFundRequest]) -> Vec<TagBalance> {
    let mut map: HashMap<(Uuid, String, String), TagBalance> = HashMap::new();
    for r in rows
        .iter()
        .filter(|r| r.tracked && (r.status == "sent" || r.status == "waived"))
    {
        let key = (r.group_id, r.requester.to_lowercase(), r.tag.trim().to_lowercase());
        let entry = map.entry(key).or_insert_with(|| TagBalance {
            group_id: r.group_id,
            username: r.requester.clone(),
            tag: r.tag.trim().to_string(),
            received: 0.0,
            spent: 0.0,
            waived: 0.0,
            balance: 0.0,
        });
        entry.received += r.amount;
        entry.spent += r.spent;
        entry.waived += r.waived_amount;
        entry.balance = entry.received - entry.spent - entry.waived;
    }
    let mut out: Vec<TagBalance> = map.into_values().collect();
    out.sort_by(|a, b| {
        (a.username.to_lowercase(), a.tag.to_lowercase())
            .cmp(&(b.username.to_lowercase(), b.tag.to_lowercase()))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn at(day: u32, hour: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 9, day).unwrap().and_hms_opt(hour, 0, 0).unwrap()
    }

    fn spend(day: u32, amount: f64) -> TagSpending {
        TagSpending {
            spending_id: Uuid::new_v4(),
            description: String::new(),
            amount,
            created_date: at(day, 12),
        }
    }

    #[test]
    fn spendings_before_the_fund_do_not_count() {
        let funds = [FundWindow { amount: 100.0, sent_at: at(10, 9), closed_at: None }];
        let out = allocate_funds(&funds, &[spend(9, 50.0), spend(11, 30.0)]);
        assert_eq!(out[0].0, 30.0);
        assert_eq!(out[0].1.len(), 1);
    }

    #[test]
    fn oldest_fund_is_used_first_and_overflow_moves_on() {
        let funds = [
            FundWindow { amount: 50.0, sent_at: at(12, 9), closed_at: None },
            FundWindow { amount: 100.0, sent_at: at(10, 9), closed_at: None },
        ];
        let out = allocate_funds(&funds, &[spend(13, 120.0), spend(14, 40.0)]);
        // The fund sent on the 10th fills first (100), the rest goes to the
        // fund of the 12th (20 + 30 of the next spending), 10 is own money.
        assert_eq!(out[1].0, 100.0);
        assert_eq!(out[0].0, 50.0);
    }

    #[test]
    fn a_waived_fund_stops_taking_spendings() {
        let funds = [
            FundWindow { amount: 100.0, sent_at: at(10, 9), closed_at: Some(at(15, 0)) },
            FundWindow { amount: 100.0, sent_at: at(10, 10), closed_at: None },
        ];
        let out = allocate_funds(&funds, &[spend(11, 30.0), spend(16, 50.0)]);
        assert_eq!(out[0].0, 30.0);
        assert_eq!(out[1].0, 50.0);
    }

    #[test]
    fn balances_add_up_per_member_and_tag() {
        let base = GroupFundRequest {
            request_id: Uuid::new_v4(),
            group_id: Uuid::nil(),
            kind: "request".into(),
            requester: "bob".into(),
            payer: "alice".into(),
            amount: 100.0,
            tag: "Transportation".into(),
            note: String::new(),
            tracked: true,
            status: "sent".into(),
            to_source_id: None,
            to_source: None,
            from_source_id: None,
            from_source: None,
            transfer_id: None,
            created_by: "bob".into(),
            created_date: at(10, 9),
            responded_at: None,
            sent_at: Some(at(10, 9)),
            waived_by: None,
            waived_at: None,
            waive_note: None,
            spent: 30.0,
            remaining: 70.0,
            waived_amount: 0.0,
            usage: vec![],
        };
        let waived = GroupFundRequest {
            request_id: Uuid::new_v4(),
            status: "waived".into(),
            tag: " transportation ".into(),
            spent: 20.0,
            remaining: 0.0,
            waived_amount: 30.0,
            amount: 50.0,
            ..base.clone()
        };
        let untracked = GroupFundRequest { request_id: Uuid::new_v4(), tracked: false, ..base.clone() };
        let out = tag_balances(&[base, waived, untracked]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].received, 150.0);
        assert_eq!(out[0].spent, 50.0);
        assert_eq!(out[0].waived, 30.0);
        assert_eq!(out[0].balance, 70.0);
    }
}
