//! Application state: navigation, language/appearance, the books, shared form fields, toasts.

use std::collections::HashMap;

use rune::TextEdit;
use rune_kit::PaneState;
use serde_json::Value;

use crate::data::{Books, Engine};
use crate::i18n::{CalendarKind, Fmt, Lang, Unit};
use crate::ui::icons;
use crate::ui::views;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Page {
    Overview,
    Sales,
    Pos,
    Purchases,
    Payments,
    Banking,
    Contacts,
    Inventory,
    Accounting,
    Reports,
    Sheets,
    Settings,
}

impl Page {
    pub const ALL: [Page; 12] = [
        Page::Overview,
        Page::Sales,
        Page::Pos,
        Page::Purchases,
        Page::Payments,
        Page::Banking,
        Page::Contacts,
        Page::Inventory,
        Page::Accounting,
        Page::Reports,
        Page::Sheets,
        Page::Settings,
    ];

    /// English message key (translated through [`Fmt::t`]).
    pub fn label(self) -> &'static str {
        match self {
            Page::Overview => "Overview",
            Page::Sales => "Sales",
            Page::Pos => "POS",
            Page::Purchases => "Purchases",
            Page::Payments => "Payments",
            Page::Banking => "Banking",
            Page::Contacts => "Contacts",
            Page::Inventory => "Inventory",
            Page::Accounting => "Accounting",
            Page::Reports => "Reports",
            Page::Sheets => "Sheets",
            Page::Settings => "Settings",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Page::Overview => "nav-overview",
            Page::Sales => "nav-sales",
            Page::Pos => "nav-pos",
            Page::Purchases => "nav-purchases",
            Page::Payments => "nav-payments",
            Page::Banking => "nav-banking",
            Page::Contacts => "nav-contacts",
            Page::Inventory => "nav-inventory",
            Page::Accounting => "nav-accounting",
            Page::Reports => "nav-reports",
            Page::Sheets => "nav-sheets",
            Page::Settings => "nav-settings",
        }
    }

    pub fn icon(self) -> &'static rune::Icon {
        match self {
            Page::Overview => &icons::OVERVIEW,
            Page::Sales => &icons::SALES,
            Page::Pos => &icons::POS,
            Page::Purchases => &icons::PURCHASES,
            Page::Payments => &icons::PAYMENTS,
            Page::Banking => &icons::BANKING,
            Page::Contacts => &icons::CONTACTS,
            Page::Inventory => &icons::INVENTORY,
            Page::Accounting => &icons::ACCOUNTING,
            Page::Reports => &icons::REPORTS,
            Page::Sheets => &icons::SHEETS,
            Page::Settings => &icons::SETTINGS,
        }
    }

    /// Sidebar groups: daily work, people, books, workspace.
    /// `(key, label, pages)` — label is an English message key.
    pub const GROUPS: [(&'static str, &'static str, &'static [Page]); 4] = [
        ("daily", "Daily work", &[Page::Overview, Page::Sales, Page::Pos, Page::Purchases, Page::Payments, Page::Banking]),
        ("people", "Stock & people", &[Page::Inventory, Page::Contacts]),
        ("books", "Books", &[Page::Accounting, Page::Reports, Page::Sheets]),
        ("workspace", "Workspace", &[Page::Settings]),
    ];

    /// Whether this page belongs in the sidebar for the current feature flags.
    pub fn nav_pages(self, show_pos: bool) -> bool {
        match self {
            Page::Pos => show_pos,
            _ => true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Info,
    Success,
    Warning,
    Danger,
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub id: u64,
    pub tone: Tone,
    pub message: String,
}

/// Text fields of every screen, by key. A missing key is an empty field.
#[derive(Default)]
pub struct Forms {
    map: HashMap<String, TextEdit>,
}

impl Forms {
    pub fn edit(&mut self, key: &str) -> &mut TextEdit {
        self.map.entry(key.to_string()).or_insert_with(|| TextEdit::new(""))
    }
    pub fn view(&self, key: &str) -> TextEdit {
        self.map.get(key).cloned().unwrap_or_else(|| TextEdit::new(""))
    }
    pub fn get(&self, key: &str) -> String {
        self.map.get(key).map(|e| e.text.clone()).unwrap_or_default()
    }
    pub fn set(&mut self, key: &str, value: &str) {
        self.edit(key).replace_text(value);
    }
    pub fn clear_prefix(&mut self, prefix: &str) {
        self.map.retain(|k, _| !k.starts_with(prefix));
    }
}

pub struct State {
    pub engine: Engine,
    pub books: Books,
    pub page: Page,
    pub dark: bool,
    pub fmt: Fmt,
    pub forms: Forms,
    pub toasts: Vec<Toast>,
    next_toast: u64,
    pub palette_open: bool,
    pub sidebar: PaneState,
    pub guide_open: bool,
    pub overview: views::overview::OverviewState,
    pub sales: views::sales::SalesState,
    pub pos: views::pos::PosState,
    pub purchases: views::purchases::PurchasesState,
    pub payments: views::payments::PaymentsState,
    pub banking: views::banking::BankingState,
    pub contacts: views::contacts::ContactsState,
    pub inventory: views::inventory::InventoryState,
    pub accounting: views::accounting::AccountingState,
    pub reports: views::reports::ReportsState,
    pub sheets: views::sheets::SheetsState,
    pub settings: views::settings::SettingsState,
    pub setup: views::setup::SetupState,
    /// Open state for kit `Select` fields, keyed by select id.
    pub selects: HashMap<String, bool>,
    /// Counter page. Loaded from settings; off until the owner turns it on.
    pub show_pos: bool,
}

impl State {
    pub fn with_engine(engine: Engine, dark: bool) -> Self {
        let mut books = Books::default();
        books.refresh(&engine);
        let mut s = State {
            engine,
            books,
            page: Page::Overview,
            dark,
            fmt: Fmt::default(),
            forms: Forms::default(),
            toasts: Vec::new(),
            next_toast: 1,
            palette_open: false,
            // Match rune-kit shell example: full close to 0, resize 136…640.
            sidebar: PaneState::new(276.0).range(136.0, 640.0),
            guide_open: true,
            overview: Default::default(),
            sales: Default::default(),
            pos: Default::default(),
            purchases: Default::default(),
            payments: Default::default(),
            banking: Default::default(),
            contacts: Default::default(),
            inventory: Default::default(),
            accounting: Default::default(),
            reports: Default::default(),
            sheets: Default::default(),
            settings: Default::default(),
            setup: Default::default(),
            selects: HashMap::new(),
            show_pos: false,
        };
        s.apply_settings();
        s
    }

    /// Opens the books named by `SARV_DATA_DIR`; `SARV_DEMO=1` seeds a throwaway QA profile.
    pub fn open() -> Self {
        let engine = if std::env::var_os("SARV_DEMO").is_some() { Engine::demo() } else { Engine::open() };
        Self::with_engine(engine, true)
    }

    /// Digits, unit, calendar and appearance follow the saved settings.
    pub fn apply_settings(&mut self) {
        let st = &self.books.workspace["settings"];
        match st["text_direction"].as_str() {
            Some("Ltr") => self.fmt.lang = Lang::En,
            _ => self.fmt.lang = Lang::Fa,
        }
        if let Some(d) = st["display_digits"].as_str() {
            self.fmt.persian_digits = d == "Persian";
        }
        if let Some(u) = st["default_currency"].as_str() {
            self.fmt.unit = if u == "Rial" { Unit::Rial } else { Unit::Toman };
        }
        if let Some(c) = st["date_display"].as_str() {
            self.fmt.calendar = if c == "Jalali" { CalendarKind::Jalali } else { CalendarKind::Gregorian };
        }
        match st["appearance"].as_str() {
            Some("Light") => self.dark = false,
            Some("Dark") => self.dark = true,
            _ => {}
        }
        self.show_pos = st["show_pos"].as_bool().unwrap_or(false);
        if self.page == Page::Pos && !self.show_pos {
            self.page = Page::Overview;
        }
    }

    pub fn lang(&self) -> Lang {
        self.fmt.lang
    }

    pub fn set_lang(&mut self, lang: Lang) {
        self.fmt.lang = lang;
        self.fmt.persian_digits = lang == Lang::Fa;
    }

    pub fn nav(&mut self, page: Page) {
        if page == Page::Pos && !self.show_pos {
            self.toast(Tone::Info, self.fmt.t("Turn on Point of sale in Settings."));
            return;
        }
        self.page = page;
        self.palette_open = false;
        if page == Page::Sheets {
            views::sheets::ensure(self);
        }
        if page == Page::Pos {
            // Barcode scans land in the catalog search as soon as POS opens.
            self.forms.edit("pos-search").focused = true;
        }
    }

    pub fn toggle_sidebar(&mut self) {
        self.sidebar.toggle();
    }

    pub fn sidebar_open(&self) -> bool {
        self.sidebar.is_open()
    }

    pub fn open_palette(&mut self) {
        self.forms.set("palette-query", "");
        self.palette_open = true;
    }

    pub fn toast(&mut self, tone: Tone, message: impl Into<String>) {
        let id = self.next_toast;
        self.next_toast += 1;
        self.toasts.push(Toast { id, tone, message: message.into() });
        if self.toasts.len() > 4 {
            self.toasts.remove(0);
        }
    }

    pub fn refresh(&mut self) {
        self.books.refresh(&self.engine);
    }

    pub fn select_company(&mut self, id: &str) {
        self.books.company_id = id.to_string();
        self.books.period_id.clear();
        self.sheets.loaded = false;
        self.pos = views::pos::PosState::default();
        self.refresh();
    }

    /// Fill company/period and the usual procurement context fields when a screen omits them.
    fn fill_books_context(&self, payload: &mut Value) {
        let blank = |v: &Value| v.is_null() || v.as_str().is_none_or(|s| s.trim().is_empty());
        if blank(&payload["company_id"]) {
            payload["company_id"] = Value::String(self.books.company_id.clone());
        }
        if blank(&payload["period_id"]) {
            payload["period_id"] = Value::String(self.books.period_id.clone());
        }
        if blank(&payload["date"]) {
            payload["date"] = Value::String(crate::data::today());
        }
        if blank(&payload["reason"]) {
            payload["reason"] = Value::String("Desktop".into());
        }
        if blank(&payload["source_id"]) || blank(&payload["request_key"]) {
            let key = format!(
                "ui-{}-{}",
                std::process::id(),
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            );
            if blank(&payload["source_id"]) {
                payload["source_id"] = Value::String(key.clone());
            }
            if blank(&payload["request_key"]) {
                payload["request_key"] = Value::String(key);
            }
        }
    }

    /// Call an action against the selected company. On success the books reload and
    /// `success` (an English message key) is toasted; on failure the engine's message is.
    pub fn run(&mut self, action: &str, mut payload: Value, success: Option<&str>) -> Result<Value, String> {
        self.fill_books_context(&mut payload);
        match self.engine.call(action, payload) {
            Ok(v) => {
                self.refresh();
                if let Some(m) = success {
                    let msg = self.fmt.t(m);
                    self.toast(Tone::Success, msg);
                }
                Ok(v)
            }
            Err(e) => {
                self.toast(Tone::Danger, e.clone());
                Err(e)
            }
        }
    }

    /// Read-only call that never refreshes or toasts (previews, details, exports).
    pub fn query(&self, action: &str, mut payload: Value) -> Result<Value, String> {
        self.fill_books_context(&mut payload);
        self.engine.call(action, payload)
    }
}
