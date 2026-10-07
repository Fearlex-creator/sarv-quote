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
        format: "sarv-quote-backup-v1",
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
    pub directory: PathBuf,
}
/// An immutable, owned candidate validated and serialized off the GTK thread.
/// Private fields prevent changing either its data or generation after preparation.
pub struct PreparedChange {
    candidate: State,
    payload: String,
    expected_generation: i64,
}
impl Store {
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
        let state: State =
            serde_json::from_str(&payload).context("دادهٔ محلی نامعتبر؛ فایل را نگه دارید")?;
        state.validate()?;
        Ok(Self {
            conn,
            state,
            generation,
            directory: directory.to_path_buf(),
        })
    }
    pub fn change<T>(&mut self, f: impl FnOnce(&mut State) -> Result<T>) -> Result<T> {
        let mut candidate = self.state.clone();
        let result = f(&mut candidate)?;
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
        let tx = self.conn.transaction()?;
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
        candidate.validate()?;
        let payload = serde_json::to_string(&candidate)?;
        serde_json::to_writer_pretty(BackupSize(0), &backup_view(&candidate, payload.as_bytes()))
            .context("ذخیره انجام نشد؛ داده باید در پشتیبان ۶۴ مگابایتی جا بگیرد")?;
        Ok(PreparedChange {
            candidate,
            payload,
            expected_generation,
        })
    }
    pub fn commit_prepared(&mut self, prepared: PreparedChange) -> Result<()> {
        self.check_generation(prepared.expected_generation)?;
        let next_generation = self
            .generation
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("سرریز نسخهٔ ذخیره‌سازی"))?;
        let tx = self.conn.transaction()?;
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
        ensure!(b.format == "sarv-quote-backup-v1", "قالب پشتیبان ناشناخته");
        ensure!(
            b.checksum == hash(&serde_json::to_vec(&b.state)?),
            "اثر انگشت پشتیبان تطبیق ندارد"
        );
        b.state.validate()?;
        Ok(b.state)
    }
    pub fn restore(&mut self, candidate: State) -> Result<PathBuf> {
        candidate.validate()?;
        let safety = self
            .directory
            .join(format!("before-restore-{}.json", crate::domain::id()));
        self.backup(&safety)?;
        self.change(|s| {
            *s = candidate;
            Ok(())
        })?;
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
