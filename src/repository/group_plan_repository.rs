use chrono::NaiveDateTime;
use mysql::prelude::*;
use mysql::*;
use std::error::Error;
use uuid::Uuid;

use crate::models::group_balance::GroupBalanceEntry;
use crate::models::group_plan::{GroupPlannedExpense, GroupRoutine, GroupRoutinePayment};
use crate::repository::group_balance_repository::spend_group_balance;

fn parse_uuid(value: &str) -> Uuid {
    Uuid::parse_str(value).unwrap_or_else(|_| Uuid::nil())
}

pub fn create_group_plan_tables(conn: &mut PooledConn) -> Result<()> {
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS group_routine (
            routine_id CHAR(36) PRIMARY KEY,
            group_id CHAR(36) NOT NULL,
            item_name VARCHAR(255) NOT NULL,
            price DOUBLE NOT NULL,
            reminder VARCHAR(64) NOT NULL,
            spending_category_id CHAR(36) NOT NULL,
            spending_category VARCHAR(255) NOT NULL,
            created_by VARCHAR(255) NOT NULL,
            created_date DATETIME NOT NULL,
            updated_date DATETIME NOT NULL,
            is_active TINYINT(1) NOT NULL DEFAULT 1,
            INDEX idx_group_routine_group (group_id)
        )",
    )?;
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS group_routine_payment (
            payment_id CHAR(36) PRIMARY KEY,
            routine_id CHAR(36) NOT NULL,
            group_id CHAR(36) NOT NULL,
            spending_id CHAR(36) NOT NULL,
            item_name VARCHAR(255) NOT NULL,
            price DOUBLE NOT NULL,
            source_id CHAR(36) NOT NULL,
            source VARCHAR(255) NOT NULL,
            paid_by VARCHAR(255) NOT NULL,
            paid_at DATETIME NOT NULL,
            INDEX idx_group_routine_payment_group (group_id, paid_at)
        )",
    )?;
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS group_planned_expense (
            planned_expense_id CHAR(36) PRIMARY KEY,
            group_id CHAR(36) NOT NULL,
            item_name VARCHAR(255) NOT NULL,
            price DOUBLE NOT NULL,
            spending_category_id CHAR(36) NOT NULL,
            spending_category VARCHAR(255) NOT NULL,
            notes TEXT,
            status VARCHAR(16) NOT NULL,
            requested_by VARCHAR(255) NOT NULL,
            reviewed_by VARCHAR(255) NULL,
            reviewed_at DATETIME NULL,
            fulfilled_by VARCHAR(255) NULL,
            fulfilled_price DOUBLE NULL,
            fulfilled_at DATETIME NULL,
            spending_id CHAR(36) NULL,
            created_date DATETIME NOT NULL,
            updated_date DATETIME NOT NULL,
            INDEX idx_group_planned_expense_group (group_id)
        )",
    )?;
    Ok(())
}

/// Name of `source_id` when it is one of `username`'s own active sources.
/// A member can only ever pay out of their own balance.
pub fn select_owned_source_name<Q: Queryable>(
    conn: &mut Q,
    source_id: Uuid,
    username: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    let name: Option<String> = conn.exec_first(
        "SELECT source FROM source
         WHERE source_id = :id AND created_by = :username AND is_active = 1",
        params! { "id" => source_id.to_string(), "username" => username },
    )?;
    Ok(name)
}

/// A spending in `spent_by`'s own records, created on their behalf by a group
/// action (paying a routine, fulfilling a planned expense).
pub struct MemberSpending<'a> {
    pub spending_id: Uuid,
    pub group_id: Uuid,
    pub total_amount: f64,
    pub description: &'a str,
    pub spending_category_id: Uuid,
    pub spending_category: &'a str,
    pub source_id: Uuid,
    pub source: &'a str,
    pub spent_by: &'a str,
    pub created_date: NaiveDateTime,
}

/// Writes the spending to the payer's own ledger - which is what lowers their
/// source balance - and tags it into the group so it is part of the group's
/// transaction history as well.
pub fn insert_member_spending<Q: Queryable>(
    conn: &mut Q,
    spending: &MemberSpending,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "INSERT INTO spending
            (spending_id, total_amount, description, spending_category_id, spending_category,
             source_id, source, created_date, created_by, is_active)
         VALUES (:id, :total, :description, :cat_id, :cat, :src_id, :src, :created, :by, 1)",
        params! {
            "id" => spending.spending_id.to_string(),
            "total" => spending.total_amount,
            "description" => spending.description,
            "cat_id" => spending.spending_category_id.to_string(),
            "cat" => spending.spending_category,
            "src_id" => spending.source_id.to_string(),
            "src" => spending.source,
            "created" => spending.created_date.to_string(),
            "by" => spending.spent_by,
        },
    )?;
    conn.exec_drop(
        "INSERT INTO spending_group_transaction
            (transaction_type, transaction_id, group_id, created_by, created_date)
         VALUES ('spending', :txn_id, :group_id, :created_by, :created_date)",
        params! {
            "txn_id" => spending.spending_id.to_string(),
            "group_id" => spending.group_id.to_string(),
            "created_by" => spending.spent_by,
            "created_date" => spending.created_date.to_string(),
        },
    )?;
    Ok(())
}

/// Where the money for a routine payment or planned purchase comes from.
pub enum Payer<'a> {
    /// One of the payer's own sources: a spending in their personal records.
    Source(MemberSpending<'a>),
    /// The payer's balance in this group: a negative balance entry.
    GroupBalance(GroupBalanceEntry),
}

impl Payer<'_> {
    fn paid_by(&self) -> &str {
        match self {
            Payer::Source(s) => s.spent_by,
            Payer::GroupBalance(e) => &e.username,
        }
    }

    fn amount(&self) -> f64 {
        match self {
            Payer::Source(s) => s.total_amount,
            Payer::GroupBalance(e) => -e.amount,
        }
    }

    fn at(&self) -> NaiveDateTime {
        match self {
            Payer::Source(s) => s.created_date,
            Payer::GroupBalance(e) => e.created_date,
        }
    }

    /// The spending id, or the balance entry id.
    fn record_id(&self) -> Uuid {
        match self {
            Payer::Source(s) => s.spending_id,
            Payer::GroupBalance(e) => e.entry_id,
        }
    }
}

/// Takes the money. `false` means the group balance did not cover it and
/// nothing was written.
fn take_payment<Q: Queryable>(conn: &mut Q, payer: &Payer) -> Result<bool, Box<dyn Error>> {
    match payer {
        Payer::Source(spending) => {
            insert_member_spending(conn, spending)?;
            Ok(true)
        }
        Payer::GroupBalance(entry) => spend_group_balance(conn, entry),
    }
}

// ── Routines ────────────────────────────────────────────────────────────

/// Every active routine across `username`'s groups, with who paid it last.
pub fn select_group_routines(
    conn: &mut PooledConn,
    username: &str,
) -> Result<Vec<GroupRoutine>, Box<dyn Error>> {
    let rows = conn.exec_map(
        "SELECT r.routine_id, r.group_id, r.item_name, r.price, r.reminder,
                r.spending_category_id, r.spending_category,
                (SELECT p.paid_at FROM group_routine_payment p
                 WHERE p.routine_id = r.routine_id ORDER BY p.paid_at DESC LIMIT 1),
                (SELECT p.paid_by FROM group_routine_payment p
                 WHERE p.routine_id = r.routine_id ORDER BY p.paid_at DESC LIMIT 1),
                r.created_by, r.created_date, r.updated_date
         FROM group_routine r
         WHERE r.is_active = 1
           AND r.group_id IN (SELECT group_id FROM spending_group_member WHERE username = :username)
         ORDER BY r.item_name ASC",
        params! { "username" => username },
        |row: Row| {
            let (routine_id, group_id, item_name, price, reminder, cat_id, cat): (
                String,
                String,
                String,
                f64,
                String,
                String,
                String,
            ) = (
                row.get(0).unwrap(),
                row.get(1).unwrap(),
                row.get(2).unwrap(),
                row.get(3).unwrap(),
                row.get(4).unwrap(),
                row.get(5).unwrap(),
                row.get(6).unwrap(),
            );
            GroupRoutine {
                routine_id: parse_uuid(&routine_id),
                group_id: parse_uuid(&group_id),
                item_name,
                price,
                reminder,
                spending_category_id: parse_uuid(&cat_id),
                spending_category: cat,
                last_paid_at: row.get::<Option<NaiveDateTime>, _>(7).flatten(),
                last_paid_by: row.get::<Option<String>, _>(8).flatten(),
                created_by: row.get(9).unwrap(),
                created_date: row.get(10).unwrap(),
                updated_date: row.get(11).unwrap(),
            }
        },
    )?;
    Ok(rows)
}

pub fn select_group_routine(
    conn: &mut PooledConn,
    group_id: Uuid,
    routine_id: Uuid,
) -> Result<Option<(String, String, Uuid, String)>, Box<dyn Error>> {
    let row: Option<(String, String, String, String)> = conn.exec_first(
        "SELECT item_name, reminder, spending_category_id, spending_category
         FROM group_routine
         WHERE routine_id = :id AND group_id = :group_id AND is_active = 1",
        params! { "id" => routine_id.to_string(), "group_id" => group_id.to_string() },
    )?;
    Ok(row.map(|(name, reminder, cat_id, cat)| (name, reminder, parse_uuid(&cat_id), cat)))
}

/// Creates the routine, or edits it when the id already exists in that group.
pub fn upsert_group_routine(
    conn: &mut PooledConn,
    routine: &GroupRoutine,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "INSERT INTO group_routine
            (routine_id, group_id, item_name, price, reminder, spending_category_id,
             spending_category, created_by, created_date, updated_date, is_active)
         VALUES (:id, :group_id, :name, :price, :reminder, :cat_id, :cat, :by, :created, :updated, 1)
         ON DUPLICATE KEY UPDATE
            item_name = IF(group_id = VALUES(group_id), VALUES(item_name), item_name),
            price = IF(group_id = VALUES(group_id), VALUES(price), price),
            reminder = IF(group_id = VALUES(group_id), VALUES(reminder), reminder),
            spending_category_id = IF(group_id = VALUES(group_id), VALUES(spending_category_id), spending_category_id),
            spending_category = IF(group_id = VALUES(group_id), VALUES(spending_category), spending_category),
            updated_date = IF(group_id = VALUES(group_id), VALUES(updated_date), updated_date)",
        params! {
            "id" => routine.routine_id.to_string(),
            "group_id" => routine.group_id.to_string(),
            "name" => &routine.item_name,
            "price" => routine.price,
            "reminder" => &routine.reminder,
            "cat_id" => routine.spending_category_id.to_string(),
            "cat" => &routine.spending_category,
            "by" => &routine.created_by,
            "created" => routine.created_date.to_string(),
            "updated" => routine.updated_date.to_string(),
        },
    )?;
    Ok(())
}

pub fn remove_group_routine(
    conn: &mut PooledConn,
    group_id: Uuid,
    routine_id: Uuid,
    now: NaiveDateTime,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "UPDATE group_routine SET is_active = 0, updated_date = :now
         WHERE routine_id = :id AND group_id = :group_id",
        params! {
            "id" => routine_id.to_string(),
            "group_id" => group_id.to_string(),
            "now" => now.to_string(),
        },
    )?;
    Ok(())
}

pub fn group_routine_payment_exists(
    conn: &mut PooledConn,
    payment_id: Uuid,
) -> Result<bool, Box<dyn Error>> {
    let found: Option<u8> = conn.exec_first(
        "SELECT 1 FROM group_routine_payment WHERE payment_id = :id",
        params! { "id" => payment_id.to_string() },
    )?;
    Ok(found.is_some())
}

/// Records one payment: the money taken (see [`Payer`]) and the payment row,
/// all or nothing. `false` means the group balance was too low.
pub fn insert_group_routine_payment(
    conn: &mut PooledConn,
    payment: &GroupRoutinePayment,
    payer: &Payer,
) -> Result<bool, Box<dyn Error>> {
    let mut tx = conn.start_transaction(TxOpts::default())?;
    if !take_payment(&mut tx, payer)? {
        tx.rollback()?;
        return Ok(false);
    }
    tx.exec_drop(
        "INSERT INTO group_routine_payment
            (payment_id, routine_id, group_id, spending_id, item_name, price,
             source_id, source, paid_by, paid_at)
         VALUES (:id, :routine_id, :group_id, :spending_id, :name, :price,
                 :source_id, :source, :paid_by, :paid_at)",
        params! {
            "id" => payment.payment_id.to_string(),
            "routine_id" => payment.routine_id.to_string(),
            "group_id" => payment.group_id.to_string(),
            "spending_id" => payment.spending_id.to_string(),
            "name" => &payment.item_name,
            "price" => payment.price,
            "source_id" => payment.source_id.to_string(),
            "source" => &payment.source,
            "paid_by" => &payment.paid_by,
            "paid_at" => payment.paid_at.to_string(),
        },
    )?;
    tx.commit()?;
    Ok(true)
}

/// Every routine payment across `username`'s groups, newest first.
pub fn select_group_routine_payments(
    conn: &mut PooledConn,
    username: &str,
) -> Result<Vec<GroupRoutinePayment>, Box<dyn Error>> {
    let rows = conn.exec_map(
        "SELECT payment_id, routine_id, group_id, spending_id, item_name, price,
                source_id, source, paid_by, paid_at
         FROM group_routine_payment
         WHERE group_id IN (SELECT group_id FROM spending_group_member WHERE username = :username)
         ORDER BY paid_at DESC",
        params! { "username" => username },
        |(payment_id, routine_id, group_id, spending_id, item_name, price, source_id, source, paid_by, paid_at): (
            String,
            String,
            String,
            String,
            String,
            f64,
            String,
            String,
            String,
            NaiveDateTime,
        )| GroupRoutinePayment {
            payment_id: parse_uuid(&payment_id),
            routine_id: parse_uuid(&routine_id),
            group_id: parse_uuid(&group_id),
            spending_id: parse_uuid(&spending_id),
            item_name,
            price,
            source_id: parse_uuid(&source_id),
            source,
            paid_by,
            paid_at,
        },
    )?;
    Ok(rows)
}

// ── Planned expenses ────────────────────────────────────────────────────

const PLANNED_EXPENSE_COLUMNS: &str = "planned_expense_id, group_id, item_name, price,
    spending_category_id, spending_category, COALESCE(notes, ''), status, requested_by,
    reviewed_by, reviewed_at, fulfilled_by, fulfilled_price, fulfilled_at, spending_id,
    created_date, updated_date";

fn planned_expense_from_row(row: Row) -> GroupPlannedExpense {
    let text = |i: usize| -> String { row.get::<String, _>(i).unwrap_or_default() };
    GroupPlannedExpense {
        planned_expense_id: parse_uuid(&text(0)),
        group_id: parse_uuid(&text(1)),
        item_name: text(2),
        price: row.get(3).unwrap_or(0.0),
        spending_category_id: parse_uuid(&text(4)),
        spending_category: text(5),
        notes: text(6),
        status: text(7),
        requested_by: text(8),
        reviewed_by: row.get::<Option<String>, _>(9).flatten(),
        reviewed_at: row.get::<Option<NaiveDateTime>, _>(10).flatten(),
        fulfilled_by: row.get::<Option<String>, _>(11).flatten(),
        fulfilled_price: row.get::<Option<f64>, _>(12).flatten(),
        fulfilled_at: row.get::<Option<NaiveDateTime>, _>(13).flatten(),
        spending_id: row
            .get::<Option<String>, _>(14)
            .flatten()
            .map(|id| parse_uuid(&id)),
        created_date: row.get(15).unwrap(),
        updated_date: row.get(16).unwrap(),
    }
}

/// Every planned expense (any status) across `username`'s groups.
pub fn select_group_planned_expenses(
    conn: &mut PooledConn,
    username: &str,
) -> Result<Vec<GroupPlannedExpense>, Box<dyn Error>> {
    let rows = conn.exec_map(
        format!(
            "SELECT {PLANNED_EXPENSE_COLUMNS} FROM group_planned_expense
             WHERE group_id IN (SELECT group_id FROM spending_group_member WHERE username = :username)
             ORDER BY created_date DESC"
        ),
        params! { "username" => username },
        planned_expense_from_row,
    )?;
    Ok(rows)
}

pub fn select_group_planned_expense(
    conn: &mut PooledConn,
    group_id: Uuid,
    planned_expense_id: Uuid,
) -> Result<Option<GroupPlannedExpense>, Box<dyn Error>> {
    let row: Option<Row> = conn.exec_first(
        format!(
            "SELECT {PLANNED_EXPENSE_COLUMNS} FROM group_planned_expense
             WHERE planned_expense_id = :id AND group_id = :group_id"
        ),
        params! { "id" => planned_expense_id.to_string(), "group_id" => group_id.to_string() },
    )?;
    Ok(row.map(planned_expense_from_row))
}

pub fn insert_group_planned_expense(
    conn: &mut PooledConn,
    item: &GroupPlannedExpense,
) -> Result<(), Box<dyn Error>> {
    conn.exec_drop(
        "INSERT INTO group_planned_expense
            (planned_expense_id, group_id, item_name, price, spending_category_id,
             spending_category, notes, status, requested_by, reviewed_by, reviewed_at,
             created_date, updated_date)
         VALUES (:id, :group_id, :name, :price, :cat_id, :cat, :notes, :status, :requested_by,
                 :reviewed_by, :reviewed_at, :created, :updated)",
        params! {
            "id" => item.planned_expense_id.to_string(),
            "group_id" => item.group_id.to_string(),
            "name" => &item.item_name,
            "price" => item.price,
            "cat_id" => item.spending_category_id.to_string(),
            "cat" => &item.spending_category,
            "notes" => &item.notes,
            "status" => &item.status,
            "requested_by" => &item.requested_by,
            "reviewed_by" => &item.reviewed_by,
            "reviewed_at" => item.reviewed_at.map(|d| d.to_string()),
            "created" => item.created_date.to_string(),
            "updated" => item.updated_date.to_string(),
        },
    )?;
    Ok(())
}

/// Moves an item from `from_status` to `to_status`. Returns `false` when the
/// item was no longer in `from_status` (someone else got there first).
pub fn update_group_planned_expense_status(
    conn: &mut PooledConn,
    planned_expense_id: Uuid,
    from_status: &str,
    to_status: &str,
    reviewed_by: Option<&str>,
    now: NaiveDateTime,
) -> Result<bool, Box<dyn Error>> {
    conn.exec_drop(
        "UPDATE group_planned_expense
         SET status = :to_status,
             reviewed_by = COALESCE(:reviewed_by, reviewed_by),
             reviewed_at = IF(:has_reviewer = 1, :now, reviewed_at),
             updated_date = :now2
         WHERE planned_expense_id = :id AND status = :from_status",
        params! {
            "to_status" => to_status,
            "reviewed_by" => reviewed_by,
            "has_reviewer" => i32::from(reviewed_by.is_some()),
            "now" => now.to_string(),
            "now2" => now.to_string(),
            "id" => planned_expense_id.to_string(),
            "from_status" => from_status,
        },
    )?;
    Ok(conn.affected_rows() > 0)
}

pub enum FulfilOutcome {
    Done,
    /// The item was no longer `planned` (someone else bought or canceled it).
    NotPlanned,
    /// Paid from the group balance, which did not cover the price.
    InsufficientBalance,
}

/// Fulfils a `planned` item: the money taken (see [`Payer`]) and the status
/// change, all or nothing.
pub fn fulfil_group_planned_expense(
    conn: &mut PooledConn,
    planned_expense_id: Uuid,
    payer: &Payer,
) -> Result<FulfilOutcome, Box<dyn Error>> {
    let mut tx = conn.start_transaction(TxOpts::default())?;
    tx.exec_drop(
        "UPDATE group_planned_expense
         SET status = 'fulfilled', fulfilled_by = :by, fulfilled_price = :price,
             fulfilled_at = :at, spending_id = :spending_id, updated_date = :at2
         WHERE planned_expense_id = :id AND status = 'planned'",
        params! {
            "by" => payer.paid_by(),
            "price" => payer.amount(),
            "at" => payer.at().to_string(),
            "at2" => payer.at().to_string(),
            "spending_id" => payer.record_id().to_string(),
            "id" => planned_expense_id.to_string(),
        },
    )?;
    if tx.affected_rows() == 0 {
        tx.rollback()?;
        return Ok(FulfilOutcome::NotPlanned);
    }
    if !take_payment(&mut tx, payer)? {
        tx.rollback()?;
        return Ok(FulfilOutcome::InsufficientBalance);
    }
    tx.commit()?;
    Ok(FulfilOutcome::Done)
}
