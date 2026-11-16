//! The accounting engine behind the Rune UI. Actions go through `dispatch` in-process.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde_json::{Value, json};

pub struct Engine {
    pub dir: PathBuf,
    pub legacy_location: bool,
    pub open_error: Option<String>,
    store: Mutex<Option<sarv_quote::store::Store>>,
}

/// Resolve the books directory for `os` using `get` for environment variables.
pub fn books_dir(os: &str, get: &dyn Fn(&str) -> Option<String>) -> Result<PathBuf, String> {
    let non_empty = |k: &str| get(k).filter(|v| !v.is_empty());
    if let Some(v) = non_empty("SARV_DATA_DIR") {
        return Ok(PathBuf::from(v));
    }
    match os {
        "windows" => non_empty("APPDATA").map(|v| PathBuf::from(v).join("ir.sarv.accounting").join("books")),
        "macos" => non_empty("HOME").map(|v| PathBuf::from(v).join("Library/Application Support/ir.sarv.accounting/books")),
        _ => non_empty("XDG_DATA_HOME")
            .map(|v| PathBuf::from(v).join("ir.sarv.accounting/books"))
            .or_else(|| non_empty("HOME").map(|v| PathBuf::from(v).join(".local/share/ir.sarv.accounting/books"))),
    }
    .ok_or_else(|| "No home folder found. Set SARV_DATA_DIR.".into())
}

impl Engine {
    /// `SARV_DATA_DIR`, else per-OS user data directory (`ir.sarv.accounting/books`).
    pub fn open() -> Self {
        let mut legacy_location = false;
        let (dir, open_error) = match books_dir(std::env::consts::OS, &|k| std::env::var(k).ok()) {
            Ok(mut dir) => {
                if std::env::var_os("SARV_DATA_DIR").is_none() && !dir.join("sarv.sqlite3").exists() {
                    if std::env::consts::OS == "macos" {
                        if let Some(home) = std::env::var("HOME").ok().filter(|v| !v.is_empty()) {
                            let legacy = PathBuf::from(home).join(".local/share/ir.sarv.accounting/books");
                            if legacy.join("sarv.sqlite3").exists() {
                                dir = legacy;
                                legacy_location = true;
                            }
                        }
                    } else if std::env::consts::OS == "windows" {
                        let legacy = PathBuf::from("sarv_books");
                        if legacy.join("sarv.sqlite3").exists() {
                            dir = legacy;
                            legacy_location = true;
                        }
                    }
                }
                let err = std::fs::create_dir_all(&dir).err().map(|e| e.to_string());
                (dir, err)
            }
            Err(e) => (PathBuf::new(), Some(e)),
        };
        Engine {
            dir,
            legacy_location,
            open_error,
            store: Mutex::new(None),
        }
    }

    /// A throwaway profile seeded with the QA fixture, for screenshots and tests.
    pub fn demo() -> Self {
        use std::sync::Once;
        use std::sync::atomic::{AtomicU64, Ordering};
        static PIN_CLOCK: Once = Once::new();
        PIN_CLOCK.call_once(|| {
            if std::env::var_os("SARV_TODAY").is_none() {
                // Runs once on first demo/test engine init before worker threads read SARV_TODAY.
                unsafe { std::env::set_var("SARV_TODAY", "2026-10-08") };
            }
        });
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        let count = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("sarv-demo-{}-{}", std::process::id(), count));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let _ = sarv_quote::desktop::seed_fixture(&dir);
        Engine {
            dir,
            legacy_location: false,
            open_error: None,
            store: Mutex::new(None),
        }
    }

    pub fn with_dir(dir: impl Into<PathBuf>) -> Self {
        Engine {
            dir: dir.into(),
            legacy_location: false,
            open_error: None,
            store: Mutex::new(None),
        }
    }

    fn ensure_store<'a>(&self, slot: &'a mut Option<sarv_quote::store::Store>) -> Result<&'a mut sarv_quote::store::Store, String> {
        if slot.is_none() {
            *slot = Some(sarv_quote::store::Store::open(&self.dir).map_err(|e| e.to_string())?);
        }
        if let Err(e) = slot.as_mut().expect("store was just opened").refresh_if_stale() {
            *slot = None;
            return Err(e.to_string());
        }
        Ok(slot.as_mut().expect("store was just refreshed"))
    }

    /// Run an action; the error is the engine's user-facing message.
    pub fn call(&self, action: &str, payload: Value) -> Result<Value, String> {
        let mut slot = self.store.lock().map_err(|_| "Books are busy".to_string())?;
        match self.ensure_store(&mut slot) {
            Ok(store) => sarv_quote::desktop::dispatch_in(store, action, payload).map_err(|e| e.to_string()),
            Err(_) if matches!(action, "backup_list" | "backup_restore_preview" | "recover_from_backup") => {
                sarv_quote::desktop::dispatch(&self.dir, action, payload).map_err(|e| e.to_string())
            }
            Err(e) => Err(e),
        }
    }

    /// Drop the held SQLite store so the database file can be sealed cleanly.
    pub fn close_store(&self) {
        if let Ok(mut slot) = self.store.lock() {
            *slot = None;
        }
    }

    /// Read the current validated state from the held store.
    pub fn with_state<T>(&self, f: impl FnOnce(&sarv_quote::domain::State) -> Result<T, String>) -> Result<T, String> {
        let mut slot = self.store.lock().map_err(|_| "Books are busy".to_string())?;
        let store = self.ensure_store(&mut slot)?;
        f(&store.state)
    }
}

/// Everything the screens read: the workspace snapshot for the selected company and its report.
#[derive(Default)]
pub struct Books {
    pub workspace: Value,
    pub report: Value,
    pub banking: Value,
    pub party_index: HashMap<String, String>,
    pub company_id: String,
    pub period_id: String,
    pub error: String,
    pub loaded: bool,
}

/// Today's date as ISO `YYYY-MM-DD`. `SARV_TODAY` overrides the clock (tests, demos).
pub fn today() -> String {
    if let Ok(v) = std::env::var("SARV_TODAY")
        && chrono::NaiveDate::parse_from_str(v.trim(), "%Y-%m-%d").is_ok()
    {
        return v.trim().to_string();
    }
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

impl Books {
    /// Reload the workspace (and the report for the period) from the engine.
    pub fn refresh(&mut self, engine: &Engine) {
        if let Some(err) = &engine.open_error {
            crate::logfile::line("ERROR", &format!("refresh open_error message={err}"));
            self.error = err.clone();
            self.loaded = true;
            return;
        }
        let first = engine.call("workspace", json!({ "company_id": self.company_id }));
        match first {
            Ok(w) => {
                self.error.clear();
                if self.company_id.is_empty() {
                    if let Some(id) = w["companies"][0]["id"].as_str() {
                        self.company_id = id.to_string();
                        return self.refresh(engine);
                    }
                }
                self.workspace = w;
                self.pick_period();
                self.report = Value::Null;
                if !self.company_id.is_empty() {
                    let (from, to) = self.range();
                    match engine.call("report", json!({ "company_id": self.company_id, "from": from, "to": to })) {
                        Ok(r) => self.report = r,
                        Err(e) => {
                            crate::logfile::line("ERROR", &format!("refresh action=report message={e}"));
                            self.error = e;
                        }
                    }
                    match engine.call("banking", json!({ "company_id": self.company_id, "from": from, "to": to })) {
                        Ok(b) => self.banking = b,
                        Err(e) => {
                            crate::logfile::line("ERROR", &format!("refresh action=banking message={e}"));
                            self.banking = Value::Null;
                        }
                    }
                }
                self.index_parties();
            }
            Err(e) => {
                crate::logfile::line("ERROR", &format!("refresh action=workspace message={e}"));
                self.error = e;
            }
        }
        self.loaded = true;
    }

    fn pick_period(&mut self) {
        let periods = self.workspace["periods"].as_array().cloned().unwrap_or_default();
        if periods.iter().any(|p| p["id"].as_str() == Some(self.period_id.as_str())) {
            return;
        }
        let now = today();
        let latest = periods.iter().max_by_key(|p| p["end_date"].as_str().unwrap_or(""));
        let chosen = periods
            .iter()
            .find(|p| {
                p["start_date"].as_str().is_some_and(|s| s <= now.as_str()) && p["end_date"].as_str().is_some_and(|e| e >= now.as_str())
            })
            .or_else(|| {
                if latest.is_some_and(|p| p["end_date"].as_str().is_some_and(|e| now.as_str() > e)) {
                    latest
                } else {
                    periods.first()
                }
            });
        self.period_id = chosen.and_then(|p| p["id"].as_str()).unwrap_or("").to_string();
    }

    /// The selected period's dates, or the whole current year.
    pub fn range(&self) -> (String, String) {
        let periods = self.workspace["periods"].as_array().cloned().unwrap_or_default();
        if let Some(p) = periods.iter().find(|p| p["id"].as_str() == Some(self.period_id.as_str())) {
            return (
                p["start_date"].as_str().unwrap_or("").into(),
                p["end_date"].as_str().unwrap_or("").into(),
            );
        }
        let y = &today()[..4];
        (format!("{y}-01-01"), format!("{y}-12-31"))
    }

    pub fn load_failed(&self) -> bool {
        !self.error.is_empty() && self.workspace.is_null()
    }

    pub fn initialized(&self) -> bool {
        self.workspace["initialized"].as_bool().unwrap_or(false)
    }

    pub fn has_company(&self) -> bool {
        self.workspace["companies"].as_array().is_some_and(|c| !c.is_empty())
    }

    pub fn companies(&self) -> Vec<(String, String)> {
        self.workspace["companies"]
            .as_array()
            .map(|c| c.iter().map(|c| (s(&c["id"]), s(&c["name"]))).collect())
            .unwrap_or_default()
    }

    pub fn company_name(&self) -> String {
        self.companies()
            .into_iter()
            .find(|(id, _)| *id == self.company_id)
            .map(|(_, n)| n)
            .unwrap_or_default()
    }

    pub fn period_name(&self) -> String {
        self.workspace["periods"]
            .as_array()
            .and_then(|p| p.iter().find(|p| p["id"].as_str() == Some(self.period_id.as_str())))
            .map(|p| s(&p["name"]))
            .unwrap_or_default()
    }

    /// Cash+bank, receivables and payables from the report's `position`.
    pub fn position(&self, which: &str) -> String {
        let v = &self.report["position"][which];
        if v.is_null() { "0".into() } else { s(v) }
    }

    pub fn list(&self, key: &str) -> Vec<Value> {
        self.workspace[key].as_array().cloned().unwrap_or_default()
    }

    /// Load the next page of `key` (`journals`, `sales`, `purchases`, `payments`, `drafts`)
    /// from the engine and append the returned rows to `self.workspace[key]`.
    pub fn load_more(&mut self, engine: &Engine, key: &str) -> Result<usize, String> {
        let current_len = self.workspace[key].as_array().map(|a| a.len()).unwrap_or(0);
        let resp = engine.call(
            "records",
            json!({
                "company_id": self.company_id,
                "kind": key,
                "offset": current_len,
                "limit": 100,
            }),
        )?;
        let rows = resp["rows"].as_array().cloned().unwrap_or_default();
        let added = rows.len();
        if !rows.is_empty() {
            if let Some(arr) = self.workspace[key].as_array_mut() {
                arr.extend(rows);
            } else {
                self.workspace[key] = Value::Array(rows);
            }
        }
        let count_key = match key {
            "journals" => Some("journal_count"),
            "sales" => Some("sales_count"),
            "purchases" => Some("purchase_count"),
            "payments" => Some("payment_count"),
            "drafts" => Some("drafts_count"),
            _ => None,
        };
        if let Some(ck) = count_key
            && !resp["count"].is_null()
        {
            self.workspace[ck] = resp["count"].clone();
        }
        Ok(added)
    }

    pub fn count(&self, key: &str) -> usize {
        match &self.workspace[key] {
            Value::Number(n) => n.as_u64().unwrap_or(0) as usize,
            Value::String(s) => s.parse().unwrap_or(0),
            _ => 0,
        }
    }

    fn index_parties(&mut self) {
        self.party_index.clear();
        if let Some(arr) = self.workspace["resources"]["parties"].as_array() {
            for p in arr {
                self.party_index.insert(s(&p["id"]), s(&p["name"]));
            }
        }
    }

    /// Party name by id, honouring the party profile's display name.
    pub fn party_name(&self, id: &str) -> String {
        if let Some(p) = self.workspace["party_profiles"][id]["display_name"].as_str() {
            if !p.trim().is_empty() {
                return p.to_string();
            }
        }
        self.party_index.get(id).cloned().unwrap_or_default()
    }
}

/// A JSON scalar as text (integers arrive as strings, but be lenient).
pub fn s(v: &Value) -> String {
    match v {
        Value::String(x) => x.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::books_dir;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn from_map(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = vars.iter().map(|(k, v)| ((*k).into(), (*v).into())).collect();
        move |k: &str| map.get(k).cloned()
    }

    #[test]
    fn books_dir_linux_xdg() {
        let get = from_map(&[("XDG_DATA_HOME", "/xdg"), ("HOME", "/home/u")]);
        assert_eq!(books_dir("linux", &get).unwrap(), PathBuf::from("/xdg/ir.sarv.accounting/books"));
    }

    #[test]
    fn books_dir_linux_home() {
        let get = from_map(&[("HOME", "/home/u")]);
        assert_eq!(
            books_dir("linux", &get).unwrap(),
            PathBuf::from("/home/u/.local/share/ir.sarv.accounting/books")
        );
    }

    #[test]
    fn books_dir_windows_appdata() {
        let get = from_map(&[("APPDATA", r"C:\Users\u\AppData\Roaming"), ("HOME", "/home/u")]);
        assert_eq!(
            books_dir("windows", &get).unwrap(),
            PathBuf::from(r"C:\Users\u\AppData\Roaming")
                .join("ir.sarv.accounting")
                .join("books")
        );
    }

    #[test]
    fn books_dir_macos() {
        let get = from_map(&[("HOME", "/Users/u"), ("XDG_DATA_HOME", "/xdg")]);
        assert_eq!(
            books_dir("macos", &get).unwrap(),
            PathBuf::from("/Users/u/Library/Application Support/ir.sarv.accounting/books")
        );
    }

    #[test]
    fn books_dir_override() {
        let get = from_map(&[
            ("SARV_DATA_DIR", "/custom/books"),
            ("XDG_DATA_HOME", "/xdg"),
            ("APPDATA", r"C:\AppData"),
            ("HOME", "/home/u"),
        ]);
        for os in ["linux", "macos", "windows"] {
            assert_eq!(books_dir(os, &get).unwrap(), PathBuf::from("/custom/books"));
        }
    }

    #[test]
    fn books_dir_nothing_set_returns_err() {
        let get = from_map(&[]);
        for os in ["linux", "macos", "windows"] {
            assert!(books_dir(os, &get).is_err());
        }
    }
}
