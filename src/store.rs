use crate::{domain::State, imports::hash};
use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};
const MAX_BACKUP: u64 = 64 * 1024 * 1024;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Backup {
    format: String,
    checksum: String,
    state: State,
}
#[derive(Serialize)]
struct BackupView<'a> {
    format: &'static str,
    checksum: String,
    state: &'a State,
}
fn backup_view<'a>(state: &'a State, payload: &[u8]) -> BackupView<'a> {
    BackupView {
        format: if state.schema == 1 {
            "sarv-quote-backup-v1"
        } else {
            "sarv-accounting-backup-v2"
        },
        checksum: hash(payload),
        state,
    }
}
struct BackupSize(usize);
impl Write for BackupSize {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self.0.saturating_add(bytes.len());
        if self.0 as u64 > MAX_BACKUP {
            return Err(std::io::Error::other("پشتیبان بیش از ۶۴ مگابایت است"));
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
pub struct Store {
    conn: Connection,
    pub state: State,
    generation: i64,
    write_pending: bool,
    pub directory: PathBuf,
}
/// An immutable, owned candidate validated and serialized off the GTK thread.
/// Private fields prevent changing either its data or generation after preparation.
pub struct PreparedChange {
    candidate: State,
    payload: String,
    expected_generation: i64,
    recovery_hash: Option<String>,
}
/// Receipt returned only after a detached SQLite transaction durably commits.
/// The GTK owner adopts this already validated state without doing disk work.
pub struct CommittedChange {
    candidate: State,
    expected_generation: i64,
    generation: i64,
    directory: PathBuf,
}
impl Store {
    /// Reserve the owner while a worker saves. Other command paths cannot
    /// mutate or export an unacknowledged snapshot before its receipt arrives.
    pub fn begin_detached(&mut self) -> Result<(i64, State)> {
        self.ensure_idle()?;
        self.write_pending = true;
        Ok((self.generation, self.state.clone()))
    }

    pub fn finish_detached(&mut self, result: Result<CommittedChange>) -> Result<()> {
        self.write_pending = false;
        self.adopt_committed(result?)
    }

    fn ensure_idle(&self) -> Result<()> {
        ensure!(
            !self.write_pending,
            "ذخیره در حال انجام است؛ پس از پایان دوباره درخواست کنید"
        );
        Ok(())
    }

    /// The owner's last acknowledged generation, without querying SQLite.
    pub fn local_generation(&self) -> i64 {
        self.generation
    }

    /// Run on a worker. Never creates a missing database or replaces a newer
    /// snapshot. Losing a race leaves both the database and owner unchanged.
    pub fn commit_detached(
        directory: PathBuf,
        prepared: PreparedChange,
    ) -> Result<CommittedChange> {
        let generation = prepared
            .expected_generation
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("سرریز نسخهٔ ذخیره‌سازی"))?;
        let mut conn = Connection::open_with_flags(
            directory.join("sarv.sqlite3"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE,
        )?;
        conn.busy_timeout(Duration::from_secs(3))?;
        conn.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let (current_generation, current_payload): (i64, String) =
            tx.query_row("SELECT generation,payload FROM state WHERE id=1", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?;
        ensure!(
            current_generation == prepared.expected_generation,
            "داده در پنجرهٔ دیگری تغییر کرده است؛ برنامه را دوباره باز کنید"
        );
        let current: State = serde_json::from_str(&current_payload)?;
        Self::validate_prepared_transition(&current, &prepared)?;
        let changed = tx.execute(
            "UPDATE state SET generation=generation+1,payload=?1 WHERE id=1 AND generation=?2",
            params![prepared.payload, prepared.expected_generation],
        )?;
        ensure!(
            changed == 1,
            "داده در پنجرهٔ دیگری تغییر کرده است؛ برنامه را دوباره باز کنید"
        );
        tx.commit()?;
        Ok(CommittedChange {
            candidate: prepared.candidate,
            expected_generation: prepared.expected_generation,
            generation,
            directory,
        })
    }

    /// Call only after the worker succeeds. A mismatched owner/profile is never
    /// overwritten. Subsequent writes still verify the database generation.
    pub fn adopt_committed(&mut self, committed: CommittedChange) -> Result<()> {
        ensure!(
            self.directory == committed.directory,
            "نسخهٔ ذخیره‌شده متعلق به این پوشه نیست"
        );
        self.check_generation(committed.expected_generation)?;
        self.state = committed.candidate;
        self.generation = committed.generation;
        Ok(())
    }
    pub fn open(directory: &Path) -> Result<Self> {
        fs::create_dir_all(directory)?;
        let conn = Connection::open(directory.join("sarv.sqlite3"))?;
        conn.busy_timeout(Duration::from_secs(3))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; CREATE TABLE IF NOT EXISTS state (id INTEGER PRIMARY KEY CHECK(id=1), generation INTEGER NOT NULL, payload TEXT NOT NULL);")?;
        let json = serde_json::to_string(&State::default())?;
        conn.execute("INSERT OR IGNORE INTO state VALUES(1,0,?1)", [json])?;
        let (generation, payload): (i64, String) =
            conn.query_row("SELECT generation,payload FROM state WHERE id=1", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?;
        ensure!(generation >= 0, "نسخهٔ ذخیره‌سازی نامعتبر است");
        let mut state: State =
            serde_json::from_str(&payload).context("دادهٔ محلی نامعتبر؛ فایل را نگه دارید")?;
        state.validate()?;
        let generation = if state.schema == 1 {
            // Preserve the exact old logical snapshot before touching its row.
            // A failed or interrupted backup leaves the legacy database usable.
            let recovery = directory.join(format!("before-schema2-{}.json", crate::domain::id()));
            backup_state(&state, &recovery)?;
            let verified = Self::restore_preview_legacy(&recovery)?;
            ensure!(
                serde_json::to_vec(&verified)? == serde_json::to_vec(&state)?,
                "نسخهٔ بازیابی تطبیق ندارد"
            );
            state.schema = 2;
            state.validate()?;
            let next = generation
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("سرریز نسخهٔ ذخیره‌سازی"))?;
            let upgraded = serde_json::to_string(&state)?;
            let changed = conn.execute(
                "UPDATE state SET generation=?1,payload=?2 WHERE id=1 AND generation=?3",
                params![next, upgraded, generation],
            )?;
            ensure!(
                changed == 1,
                "داده هنگام ارتقا تغییر کرده است؛ دوباره باز کنید"
            );
            next
        } else {
            generation
        };
        Ok(Self {
            conn,
            state,
            generation,
            write_pending: false,
            directory: directory.to_path_buf(),
        })
    }
    pub fn change<T>(&mut self, f: impl FnOnce(&mut State) -> Result<T>) -> Result<T> {
        self.ensure_idle()?;
        let mut candidate = self.state.clone();
        let result = f(&mut candidate)?;
        ensure!(
            candidate.schema == 2,
            "کاهش نسخهٔ داده مجاز نیست؛ از مسیر بازیابی استفاده کنید"
        );
        candidate.validate()?;
        let payload = serde_json::to_string(&candidate)?;
        if payload == serde_json::to_string(&self.state)? {
            self.ensure_current()?;
            return Ok(result);
        }
        // Never commit a state which this version cannot safely back up. Count
        // the exact pretty JSON envelope without allocating another large copy.
        serde_json::to_writer_pretty(BackupSize(0), &backup_view(&candidate, payload.as_bytes()))
            .context("ذخیره انجام نشد؛ داده باید در پشتیبان ۶۴ مگابایتی جا بگیرد")?;
        let next_generation = self
            .generation
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("سرریز نسخهٔ ذخیره‌سازی"))?;
        let tx = self
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let (persisted_generation, persisted_payload): (i64, String) = tx.query_row(
            "SELECT generation,payload FROM state WHERE id=1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        ensure!(
            persisted_generation == self.generation,
            "داده در پنجرهٔ دیگری تغییر کرده است؛ برنامه را دوباره باز کنید"
        );
        let persisted: State = serde_json::from_str(&persisted_payload)?;
        persisted
            .accounting
            .validate_transition(&candidate.accounting)?;
        let changed = tx.execute(
            "UPDATE state SET generation=generation+1,payload=?1 WHERE id=1 AND generation=?2",
            params![payload, self.generation],
        )?;
        ensure!(
            changed == 1,
            "داده در پنجرهٔ دیگری تغییر کرده است؛ برنامه را دوباره باز کنید"
        );
        tx.commit()?;
        self.state = candidate;
        self.generation = next_generation;
        Ok(result)
    }
    pub fn snapshot(&self) -> Result<(i64, State)> {
        self.ensure_current()?;
        Ok((self.generation, self.state.clone()))
    }
    pub fn prepare_candidate(expected_generation: i64, candidate: State) -> Result<PreparedChange> {
        ensure!(expected_generation >= 0, "نسخهٔ ذخیره‌سازی نامعتبر است");
        ensure!(candidate.schema == 2, "نسخهٔ نامعتبر برای ذخیره‌سازی");
        candidate.validate()?;
        let payload = serde_json::to_string(&candidate)?;
        serde_json::to_writer_pretty(BackupSize(0), &backup_view(&candidate, payload.as_bytes()))
            .context("ذخیره انجام نشد؛ داده باید در پشتیبان ۶۴ مگابایتی جا بگیرد")?;
        Ok(PreparedChange {
            candidate,
            payload,
            expected_generation,
            recovery_hash: None,
        })
    }
    /// Explicit whole-profile replacement is permitted only with a verified
    /// recovery copy of the exact generation being replaced.
    pub fn prepare_replacement(
        expected_generation: i64,
        original: &State,
        candidate: State,
        safety: &Path,
    ) -> Result<PreparedChange> {
        let recovery = Self::restore_preview_legacy(safety)?;
        let bytes = serde_json::to_vec(original)?;
        ensure!(
            serde_json::to_vec(&recovery)? == bytes,
            "پشتیبان ایمنی با دادهٔ جاری تطبیق ندارد"
        );
        let mut prepared = Self::prepare_candidate(expected_generation, candidate)?;
        prepared.recovery_hash = Some(hash(&bytes));
        Ok(prepared)
    }
    fn validate_prepared_transition(current: &State, prepared: &PreparedChange) -> Result<()> {
        if let Some(recovery_hash) = &prepared.recovery_hash {
            ensure!(
                &hash(&serde_json::to_vec(current)?) == recovery_hash,
                "نسخهٔ بازیابی متعلق به دادهٔ جاری نیست"
            );
        } else {
            current
                .accounting
                .validate_transition(&prepared.candidate.accounting)?;
        }
        Ok(())
    }
    pub fn commit_prepared(&mut self, prepared: PreparedChange) -> Result<()> {
        self.ensure_idle()?;
        self.check_generation(prepared.expected_generation)?;
        let next_generation = self
            .generation
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("سرریز نسخهٔ ذخیره‌سازی"))?;
        let tx = self
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let (persisted_generation, persisted_payload): (i64, String) = tx.query_row(
            "SELECT generation,payload FROM state WHERE id=1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        ensure!(
            persisted_generation == prepared.expected_generation,
            "داده در پنجرهٔ دیگری تغییر کرده است؛ برنامه را دوباره باز کنید"
        );
        let persisted: State = serde_json::from_str(&persisted_payload)?;
        Self::validate_prepared_transition(&persisted, &prepared)?;
        let changed = tx.execute(
            "UPDATE state SET generation=generation+1,payload=?1 WHERE id=1 AND generation=?2",
            params![prepared.payload, prepared.expected_generation],
        )?;
        ensure!(
            changed == 1,
            "داده در پنجرهٔ دیگری تغییر کرده است؛ برنامه را دوباره باز کنید"
        );
        tx.commit()?;
        self.state = prepared.candidate;
        self.generation = next_generation;
        Ok(())
    }
    pub fn backup(&self, path: &Path) -> Result<()> {
        self.ensure_idle()?;
        // A backup must contain committed data, not an old window's snapshot.
        let (generation, payload): (i64, String) = self.conn.query_row(
            "SELECT generation,payload FROM state WHERE id=1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        self.check_generation(generation)?;
        backup_state(&serde_json::from_str::<State>(&payload)?, path)
    }
    fn check_generation(&self, generation: i64) -> Result<()> {
        ensure!(
            generation == self.generation,
            "داده در پنجرهٔ دیگری تغییر کرده است؛ برنامه را دوباره باز کنید"
        );
        Ok(())
    }
    fn ensure_current(&self) -> Result<()> {
        self.ensure_idle()?;
        let generation =
            self.conn
                .query_row("SELECT generation FROM state WHERE id=1", [], |row| {
                    row.get(0)
                })?;
        self.check_generation(generation)
    }
    /// Check an unchanged UI draft against the committed generation without
    /// cloning or serializing the state. A stale window still must be reopened.
    pub fn verify_current(&self) -> Result<()> {
        self.ensure_current()
    }
    pub fn restore_preview(path: &Path) -> Result<State> {
        let mut state = Self::restore_preview_legacy(path)?;
        state.schema = 2;
        state.validate()?;
        Ok(state)
    }
    fn restore_preview_legacy(path: &Path) -> Result<State> {
        let mut file = open_regular(path)?;
        ensure!(
            file.metadata()?.len() <= MAX_BACKUP,
            "پشتیبان بیش از ۶۴ مگابایت است"
        );
        let mut bytes = vec![];
        Read::by_ref(&mut file)
            .take(MAX_BACKUP + 1)
            .read_to_end(&mut bytes)?;
        ensure!(bytes.len() as u64 <= MAX_BACKUP, "پشتیبان بزرگ است");
        let b: Backup = serde_json::from_slice(&bytes).context("قالب پشتیبان نامعتبر")?;
        ensure!(
            (b.format == "sarv-quote-backup-v1" && b.state.schema == 1)
                || (b.format == "sarv-accounting-backup-v2" && b.state.schema == 2),
            "قالب پشتیبان ناشناخته"
        );
        ensure!(
            b.checksum == hash(&serde_json::to_vec(&b.state)?),
            "اثر انگشت پشتیبان تطبیق ندارد"
        );
        b.state.validate()?;
        Ok(b.state)
    }
    pub fn restore(&mut self, mut candidate: State) -> Result<PathBuf> {
        self.ensure_idle()?;
        candidate.validate()?;
        candidate.schema = 2;
        let safety = self
            .directory
            .join(format!("before-restore-{}.json", crate::domain::id()));
        self.backup(&safety)?;
        let prepared = Self::prepare_replacement(self.generation, &self.state, candidate, &safety)?;
        self.commit_prepared(prepared)?;
        Ok(safety)
    }
}
pub fn backup_state(state: &State, path: &Path) -> Result<()> {
    state.validate()?;
    let payload = serde_json::to_vec(state)?;
    let b = backup_view(state, &payload);
    let bytes = serde_json::to_vec_pretty(&b)?;
    ensure!(
        bytes.len() as u64 <= MAX_BACKUP,
        "پشتیبان بیش از حد بزرگ است"
    );
    write_new(path, &bytes)
}
pub fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    // Same-directory staging plus a no-replace hard link publishes only complete,
    // synced data. Neither an existing file nor a symlink can be overwritten.
    let staging = parent.join(format!(".sarv-write-{}.tmp", crate::domain::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staging)
        .context("فایل موجود است یا مسیر قابل نوشتن نیست؛ نام تازه انتخاب کنید")?;
    if let Err(e) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(&staging);
        return Err(e.into());
    }
    drop(file);
    if let Err(error) = fs::hard_link(&staging, path) {
        let _ = fs::remove_file(&staging);
        return Err(error).context("فایل موجود است یا مسیر قابل نوشتن نیست؛ نام تازه انتخاب کنید");
    }
    fs::remove_file(&staging)?;
    fs::File::open(parent)?.sync_all()?;
    Ok(())
}
fn open_regular(path: &Path) -> Result<fs::File> {
    ensure!(fs::metadata(path)?.is_file(), "مسیر باید فایل معمولی باشد");
    let file = fs::File::open(path)?;
    ensure!(file.metadata()?.is_file(), "مسیر باید فایل معمولی باشد");
    Ok(file)
}
pub fn read_import(path: &Path) -> Result<String> {
    let f = open_regular(path)?;
    ensure!(
        f.metadata()?.len() <= crate::imports::MAX_IMPORT as u64,
        "فایل بیش از ۱۰ مگابایت است"
    );
    let mut s = String::new();
    f.take(crate::imports::MAX_IMPORT as u64 + 1)
        .read_to_string(&mut s)?;
    ensure!(s.len() <= crate::imports::MAX_IMPORT, "فایل بزرگ است");
    Ok(s)
}
