//! Versioned SQLite schema. Journal rows are immutable after posting.
use anyhow::{Result, ensure};
use rusqlite::Connection;

pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA busy_timeout=5000;")?;
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    ensure!(version <= 1, "Database was created by a newer application");
    if version == 1 {
        return Ok(());
    }
    conn.execute_batch(r#"
    BEGIN IMMEDIATE;
    CREATE TABLE accounts (
      id TEXT PRIMARY KEY, name TEXT NOT NULL, kind TEXT NOT NULL,
      currency TEXT NOT NULL, group_name TEXT NOT NULL DEFAULT '',
      archived INTEGER NOT NULL DEFAULT 0 CHECK(archived IN (0,1)),
      sort_order INTEGER NOT NULL DEFAULT 0, system INTEGER NOT NULL DEFAULT 0,
      reconciled_through TEXT, created_at TEXT NOT NULL);
    CREATE TABLE contacts (
      id TEXT PRIMARY KEY, name TEXT NOT NULL, customer INTEGER NOT NULL DEFAULT 0,
      supplier INTEGER NOT NULL DEFAULT 0, phone TEXT NOT NULL DEFAULT '',
      email TEXT NOT NULL DEFAULT '', address TEXT NOT NULL DEFAULT '',
      tax_id TEXT NOT NULL DEFAULT '', notes TEXT NOT NULL DEFAULT '');
    CREATE TABLE categories (id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE, kind TEXT NOT NULL CHECK(kind IN ('income','expense')));
    CREATE TABLE transactions (
      id TEXT PRIMARY KEY, date TEXT NOT NULL, kind TEXT NOT NULL,
      payee TEXT NOT NULL, memo TEXT NOT NULL DEFAULT '', currency TEXT NOT NULL,
      amount_minor INTEGER NOT NULL DEFAULT 0, paid_minor INTEGER NOT NULL DEFAULT 0,
      contact_id TEXT REFERENCES contacts(id), due_date TEXT, tags TEXT NOT NULL DEFAULT '',
      input_json TEXT NOT NULL, state TEXT NOT NULL DEFAULT 'draft' CHECK(state IN ('draft','posted')),
      reverses_id TEXT UNIQUE REFERENCES transactions(id), replaces_id TEXT REFERENCES transactions(id),
      document_type TEXT NOT NULL DEFAULT 'cash', document_id TEXT,
      recorded_at TEXT NOT NULL, seq INTEGER NOT NULL UNIQUE);
    CREATE TABLE journal_lines (
      id INTEGER PRIMARY KEY, transaction_id TEXT NOT NULL REFERENCES transactions(id),
      account_id TEXT NOT NULL REFERENCES accounts(id), category_id TEXT REFERENCES categories(id),
      debit INTEGER NOT NULL DEFAULT 0 CHECK(debit BETWEEN 0 AND 9000000000000000),
      credit INTEGER NOT NULL DEFAULT 0 CHECK(credit BETWEEN 0 AND 9000000000000000),
      CHECK((debit>0 AND credit=0) OR (credit>0 AND debit=0)));
    CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
    CREATE TABLE attachments (id TEXT PRIMARY KEY, transaction_id TEXT NOT NULL REFERENCES transactions(id), name TEXT NOT NULL, mime TEXT NOT NULL, data BLOB NOT NULL);
    CREATE TABLE audit_events (id INTEGER PRIMARY KEY, at TEXT NOT NULL, action TEXT NOT NULL, entity_id TEXT NOT NULL, detail TEXT NOT NULL);
    CREATE TABLE saved_views (id TEXT PRIMARY KEY, name TEXT NOT NULL, filters TEXT NOT NULL);
    CREATE INDEX journal_account ON journal_lines(account_id,transaction_id);
    CREATE INDEX journal_transaction ON journal_lines(transaction_id);
    CREATE INDEX transactions_date ON transactions(date,seq);
    CREATE INDEX transactions_contact ON transactions(contact_id,date);
    CREATE INDEX transactions_currency ON transactions(currency,date,seq);
    CREATE INDEX transactions_replaces ON transactions(replaces_id);
    CREATE TRIGGER balanced_post BEFORE UPDATE OF state ON transactions WHEN NEW.state='posted' BEGIN
      SELECT CASE WHEN (SELECT COUNT(*) FROM journal_lines WHERE transaction_id=NEW.id)<2
        OR (SELECT COALESCE(SUM(debit-credit),1) FROM journal_lines WHERE transaction_id=NEW.id)!=0
        THEN RAISE(ABORT,'Journal must balance') END;
      SELECT CASE WHEN EXISTS(SELECT 1 FROM journal_lines l JOIN accounts a ON a.id=l.account_id WHERE l.transaction_id=NEW.id AND a.currency!=NEW.currency)
        THEN RAISE(ABORT,'Journal currency mismatch') END;
    END;
    CREATE TRIGGER immutable_transaction BEFORE UPDATE ON transactions WHEN OLD.state='posted' BEGIN SELECT RAISE(ABORT,'Posted transactions are immutable'); END;
    CREATE TRIGGER keep_transaction BEFORE DELETE ON transactions BEGIN SELECT RAISE(ABORT,'Transactions cannot be deleted'); END;
    CREATE TRIGGER immutable_line_update BEFORE UPDATE ON journal_lines WHEN (SELECT state FROM transactions WHERE id=OLD.transaction_id)='posted' BEGIN SELECT RAISE(ABORT,'Posted lines are immutable'); END;
    CREATE TRIGGER immutable_line_delete BEFORE DELETE ON journal_lines WHEN (SELECT state FROM transactions WHERE id=OLD.transaction_id)='posted' BEGIN SELECT RAISE(ABORT,'Posted lines are immutable'); END;
    CREATE TRIGGER immutable_line_insert BEFORE INSERT ON journal_lines WHEN (SELECT state FROM transactions WHERE id=NEW.transaction_id)='posted' BEGIN SELECT RAISE(ABORT,'Posted lines are immutable'); END;
    CREATE TRIGGER account_currency_guard BEFORE UPDATE OF currency ON accounts WHEN NEW.currency!=OLD.currency AND EXISTS(SELECT 1 FROM journal_lines WHERE account_id=OLD.id) BEGIN SELECT RAISE(ABORT,'An account with postings cannot change currency'); END;
    CREATE TRIGGER keep_account BEFORE DELETE ON accounts BEGIN SELECT RAISE(ABORT,'Archive accounts instead'); END;
    CREATE TRIGGER audit_update BEFORE UPDATE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are immutable'); END;
    CREATE TRIGGER audit_delete BEFORE DELETE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are immutable'); END;
    PRAGMA user_version=1;
    COMMIT;
    "#)?;
    Ok(())
}
