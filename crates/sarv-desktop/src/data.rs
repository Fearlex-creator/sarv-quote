//! The accounting engine behind the Rune UI. Actions go through `dispatch` in-process.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde_json::{Value, json};

pub struct Engine {
    pub dir: PathBuf,
    store: Mutex<Option<sarv_quote::store::Store>>,
}

impl Engine {
    /// `SARV_DATA_DIR`, else `ir.sarv.accounting/books` under the XDG data directory.
    pub fn open() -> Self {
        let dir = if let Ok(v) = std::env::var("SARV_DATA_DIR") {
            PathBuf::from(v)
        } else if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            PathBuf::from(xdg).join("ir.sarv.accounting/books")
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".local/share/ir.sarv.accounting/books")
        } else {
            PathBuf::from("sarv_books")
        };
        let _ = std::fs::create_dir_all(&dir);
        Engine {
            dir,
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
            store: Mutex::new(None),
        }
    }

    pub fn with_dir(dir: impl Into<PathBuf>) -> Self {
        Engine {
            dir: dir.into(),
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
                        Err(e) => self.error = e,
                    }
                    match engine.call("banking", json!({ "company_id": self.company_id, "from": from, "to": to })) {
                        Ok(b) => self.banking = b,
                        Err(_) => self.banking = Value::Null,
                    }
                }
                self.index_parties();
            }
            Err(e) => self.error = e,
        }
        self.loaded = true;
    }

    fn pick_period(&mut self) {
        let periods = self.workspace["periods"].as_array().cloned().unwrap_or_default();
        if periods.iter().any(|p| p["id"].as_str() == Some(self.period_id.as_str())) {
            return;
        }
        let now = today();
        let chosen = periods
            .iter()
            .find(|p| {
                p["start_date"].as_str().is_some_and(|s| s <= now.as_str()) && p["end_date"].as_str().is_some_and(|e| e >= now.as_str())
            })
            .or_else(|| periods.first());
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
