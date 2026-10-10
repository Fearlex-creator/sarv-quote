//! The Phase 1 ledger, independent of desktop/web transports. Future documents
//! use `post` inside the same unit of work, never a parallel module cashbook.
pub mod db;
pub mod model;
pub mod money;
mod mutations;
mod pdf;
mod views;

use anyhow::{Context, Result, ensure};
use chrono::{NaiveDate, Utc};
use model::*;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use std::{path::Path, sync::Mutex};

pub struct Ledger {
    connection: Mutex<Connection>,
}

impl Ledger {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        if let Some(parent) = path.as_ref().parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(path)?;
        db::migrate(&connection)?;
        if connection.query_row("SELECT COUNT(*) FROM settings", [], |r| r.get::<_, i64>(0))? == 0 {
            let tx = connection.unchecked_transaction()?;
            save_settings(&tx, &Settings::default())?;
            for (id, name, kind) in [
                ("sales", "Sales & services", "income"),
                ("salary", "Salary", "income"),
                ("other-income", "Other income", "income"),
                ("office", "Office & supplies", "expense"),
                ("food", "Food & dining", "expense"),
                ("travel", "Travel & transport", "expense"),
                ("utilities", "Utilities", "expense"),
                ("other-expense", "Other expenses", "expense"),
            ] {
                tx.execute(
                    "INSERT INTO categories VALUES(?1,?2,?3)",
                    params![id, name, kind],
                )?;
            }
            audit(&tx, "workspace.created", "workspace", &json!({"schema":1}))?;
            tx.commit()?;
        }
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn read(&self, action: &str, payload: Value) -> Result<Value> {
        let conn = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
        match action {
            "workspace" => views::workspace(&conn, &payload),
            "ledger" => views::ledger(&conn, serde_json::from_value(payload)?),
            "account" => views::account(&conn, text(&payload, "id")?),
            "detail" => views::detail(&conn, text(&payload, "id")?),
            "export" => views::export(&conn, &payload),
            "attachment" => views::attachment(&conn, text(&payload, "id")?),
            "preview" => {
                let tx = conn.unchecked_transaction()?;
                let mut input: TransactionInput = serde_json::from_value(payload)?;
                input.correction_id.clear();
                let tx_id = mutations::create_transaction(&tx, input)?;
                let result = views::detail(&tx, &tx_id)?;
                tx.rollback()?;
                Ok(result)
            }
            _ => anyhow::bail!("Unknown view: {action}"),
        }
    }

    pub fn mutate(&self, action: &str, payload: Value) -> Result<Value> {
        let mut conn = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let result = mutations::dispatch(&tx, action, payload)?;
        tx.commit()?;
        Ok(result)
    }
}

fn text<'a>(v: &'a Value, name: &str) -> Result<&'a str> {
    v.get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("Missing {name}"))
}
fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn now() -> String {
    Utc::now().to_rfc3339()
}
pub fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}
fn valid_date(date: &str) -> Result<()> {
    let parsed = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .context("Use a Gregorian ISO date: YYYY-MM-DD")?;
    ensure!(
        parsed.format("%Y-%m-%d").to_string() == date
            && date >= "1900-01-01"
            && date <= "9999-12-31",
        "Invalid date"
    );
    Ok(())
}
fn settings(conn: &Connection) -> Result<Settings> {
    let s: String = conn.query_row("SELECT value FROM settings WHERE key='profile'", [], |r| {
        r.get(0)
    })?;
    Ok(serde_json::from_str(&s)?)
}
fn save_settings(conn: &Connection, s: &Settings) -> Result<()> {
    conn.execute("INSERT INTO settings(key,value) VALUES('profile',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[serde_json::to_string(s)?])?;
    Ok(())
}
fn unlocked(conn: &Connection, date: &str) -> Result<()> {
    valid_date(date)?;
    let closed = settings(conn)?.closed_through;
    ensure!(
        closed.is_empty() || date > closed.as_str(),
        "This period is closed through {closed}. Post in an open period."
    );
    Ok(())
}
fn audit(conn: &Connection, action: &str, entity: &str, detail: &Value) -> Result<()> {
    conn.execute(
        "INSERT INTO audit_events(at,action,entity_id,detail) VALUES(?1,?2,?3,?4)",
        params![now(), action, entity, detail.to_string()],
    )?;
    Ok(())
}
fn system_account(conn: &Connection, kind: &str, currency: &str, category: &str) -> Result<String> {
    let account_id = format!("system:{kind}:{currency}:{category}");
    conn.execute("INSERT OR IGNORE INTO accounts(id,name,kind,currency,system,created_at) VALUES(?1,?2,?3,?4,1,?5)",params![account_id,format!("{kind} {category}"),kind,currency,now()])?;
    Ok(account_id)
}
fn wallet(conn: &Connection, account: &str) -> Result<(String, String)> {
    conn.query_row(
        "SELECT name,currency FROM accounts WHERE id=?1 AND archived=0 AND system=0",
        [account],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .context("Select an active account")
}

struct Line {
    account: String,
    category: Option<String>,
    debit: i64,
    credit: i64,
}
fn line(account: String, signed: i64, category: Option<String>) -> Line {
    Line {
        account,
        category,
        debit: signed.max(0),
        credit: (-signed).max(0),
    }
}

/// Shared document posting boundary. The transaction owns the journal and audit.
fn post(
    conn: &Connection,
    input: &TransactionInput,
    currency: &str,
    amount: i64,
    paid: i64,
    lines: Vec<Line>,
    reverses: Option<&str>,
    replaces: Option<&str>,
) -> Result<String> {
    unlocked(conn, &input.date)?;
    ensure!(lines.len() >= 2, "At least two journal sides required");
    let total = lines
        .iter()
        .try_fold(0_i64, |v, l| {
            v.checked_add(l.debit).and_then(|v| v.checked_sub(l.credit))
        })
        .context("Journal overflow")?;
    ensure!(total == 0, "Journal is not balanced");
    for l in &lines {
        ensure!(
            l.debit >= 0
                && l.credit >= 0
                && l.debit <= money::LIMIT
                && l.credit <= money::LIMIT
                && ((l.debit > 0) != (l.credit > 0)),
            "Invalid journal line"
        );
    }
    // Bound gross turnover as well as individual amounts: every intermediate
    // SQLite SUM and historical running balance must fit an exact i64.
    let gross: i64 = conn.query_row("SELECT COALESCE(SUM(l.debit),0) FROM journal_lines l JOIN transactions t ON t.id=l.transaction_id WHERE t.currency=?1",[currency],|r|r.get(0))?;
    let new_debits = lines
        .iter()
        .try_fold(0_i64, |sum, l| sum.checked_add(l.debit))
        .context("Journal turnover overflow")?;
    ensure!(
        gross
            .checked_add(new_debits)
            .is_some_and(|sum| sum <= i64::MAX / 4),
        "Currency ledger capacity exceeded"
    );
    let tx_id = id();
    // Receipts live once in attachments, rather than repeated in metadata.
    let mut snapshot = input.clone();
    snapshot.receipt = None;
    snapshot.correction_id.clear();
    conn.execute("INSERT INTO transactions(id,date,kind,payee,memo,currency,amount_minor,paid_minor,contact_id,due_date,tags,input_json,reverses_id,replaces_id,recorded_at,seq) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,(SELECT COALESCE(MAX(seq),0)+1 FROM transactions))",params![tx_id,input.date,input.kind,input.payee,input.memo,currency,amount,paid,if input.contact_id.is_empty(){None}else{Some(input.contact_id.as_str())},if input.due_date.is_empty(){None}else{Some(input.due_date.as_str())},input.tags,serde_json::to_string(&snapshot)?,reverses,replaces,now()])?;
    let affected_accounts = lines
        .iter()
        .map(|l| l.account.clone())
        .collect::<std::collections::HashSet<_>>();
    for l in lines {
        conn.execute("INSERT INTO journal_lines(transaction_id,account_id,category_id,debit,credit) VALUES(?1,?2,?3,?4,?5)",params![tx_id,l.account,l.category,l.debit,l.credit])?;
    }
    // SQLite independently checks balance/currency before the posting transition.
    conn.execute(
        "UPDATE transactions SET state='posted' WHERE id=?1",
        [&tx_id],
    )?;
    for account in affected_accounts {
        let through = optional_string(
            conn,
            "SELECT reconciled_through FROM accounts WHERE id=?1 AND reconciled_through IS NOT NULL",
            &account,
        )?;
        if through.is_some_and(|through| input.date <= through) {
            conn.execute(
                "UPDATE accounts SET reconciled_through=NULL WHERE id=?1",
                [&account],
            )?;
            audit(
                conn,
                "account.reconciliation.invalidated",
                &account,
                &json!({"transactionId":tx_id,"date":input.date}),
            )?;
        }
    }
    if let Some(receipt) = &input.receipt {
        ensure!(
            receipt.bytes.len() <= 5 * 1024 * 1024 && !receipt.bytes.is_empty(),
            "Attachments must be between 1 byte and 5 MB"
        );
        ensure!(
            !receipt.name.is_empty() && receipt.name.len() <= 250,
            "Invalid attachment name"
        );
        conn.execute(
            "INSERT INTO attachments VALUES(?1,?2,?3,?4,?5)",
            params![id(), tx_id, receipt.name, receipt.mime, receipt.bytes],
        )?;
    }
    audit(
        conn,
        if reverses.is_some() {
            "transaction.reversed"
        } else if replaces.is_some() {
            "transaction.reposted"
        } else {
            "transaction.posted"
        },
        &tx_id,
        &json!({"input":snapshot,"reverses":reverses,"replaces":replaces}),
    )?;
    Ok(tx_id)
}

fn is_active(conn: &Connection, tx_id: &str) -> Result<()> {
    let exists = conn.query_row("SELECT EXISTS(SELECT 1 FROM transactions t WHERE t.id=?1 AND t.state='posted' AND t.kind NOT IN ('reversal','opening') AND NOT EXISTS(SELECT 1 FROM transactions r WHERE r.reverses_id=t.id))",[tx_id],|r|r.get::<_,bool>(0))?;
    ensure!(
        exists,
        "Only active transactions can be corrected or reversed"
    );
    Ok(())
}

fn reverse(conn: &Connection, tx_id: &str, date: &str, reason: &str) -> Result<String> {
    ensure!(
        !reason.trim().is_empty() && reason.len() <= 4000,
        "Enter a reason for the reversal"
    );
    is_active(conn, tx_id)?;
    let (currency, original_date, payee, contact_id, due_date): (
        String,
        String,
        String,
        Option<String>,
        Option<String>,
    ) = conn.query_row(
        "SELECT currency,date,payee,contact_id,due_date FROM transactions WHERE id=?1",
        [tx_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
    )?;
    // A closed document cannot be silently rewritten even with an open reversal date.
    unlocked(conn, &original_date)?;
    let mut query = conn.prepare(
        "SELECT account_id,category_id,credit,debit FROM journal_lines WHERE transaction_id=?1",
    )?;
    let lines = query
        .query_map([tx_id], |r| {
            Ok(Line {
                account: r.get(0)?,
                category: r.get(1)?,
                debit: r.get(2)?,
                credit: r.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let input = TransactionInput {
        date: date.into(),
        kind: "reversal".into(),
        payee,
        contact_id: contact_id.unwrap_or_default(),
        due_date: due_date.unwrap_or_default(),
        memo: reason.into(),
        ..Default::default()
    };
    post(conn, &input, &currency, 0, 0, lines, Some(tx_id), None)
}

fn optional_string(conn: &Connection, sql: &str, parameter: &str) -> Result<Option<String>> {
    Ok(conn.query_row(sql, [parameter], |r| r.get(0)).optional()?)
}
