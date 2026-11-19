//! Application state: navigation, language/appearance, the books, shared form fields, toasts.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static KEY_COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_key_seq() -> u64 {
    KEY_COUNTER.fetch_add(1, Ordering::Relaxed) + 1
}

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

    /// Sidebar navigation label key (translated through [`Fmt::t`]).
    pub fn sidebar_label(self) -> &'static str {
        self.label()
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

    /// Sidebar groups: daily work, products & people, books.
    /// `(key, label, pages)` — label is an English message key.
    pub const GROUPS: [(&'static str, &'static str, &'static [Page]); 3] = [
        (
            "daily",
            "Daily work",
            &[
                Page::Overview,
                Page::Sales,
                Page::Pos,
                Page::Purchases,
                Page::Payments,
                Page::Banking,
            ],
        ),
        ("people", "Products & people", &[Page::Inventory, Page::Contacts]),
        ("books", "Books", &[Page::Reports, Page::Accounting, Page::Sheets]),
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
    pub open_path: Option<PathBuf>,
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
    pub fn clear(&mut self, keys: &[&str]) {
        for k in keys {
            self.map.remove(*k);
        }
    }
    pub fn any_with_prefix(&self, prefix: &str) -> bool {
        self.map.iter().any(|(k, v)| k.starts_with(prefix) && !v.text.trim().is_empty())
    }
    pub fn any_focused(&self) -> bool {
        self.map.values().any(|e| e.focused)
    }
    pub fn snapshot_prefix(&self, prefix: &str) -> String {
        let mut entries: Vec<_> = self
            .map
            .iter()
            .filter(|(k, v)| k.starts_with(prefix) && !v.text.is_empty())
            .map(|(k, v)| format!("{k}={}", v.text))
            .collect();
        entries.sort();
        entries.join("\n")
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
    pub palette_cursor: usize,
    pub palette_last_query: String,
    pub period_switch_open: bool,
    pub company_switch_open: bool,
    pub confirm: Option<crate::ui::confirm::Confirm>,
    pub detail: crate::ui::detail::DetailState,
    pub operation: crate::ui::operation_dialog::OperationState,
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
    pub recovery: views::recovery::RecoveryState,
    pub lock_state: views::lock::LockState,
    pub locked: bool,
    pub app_lock_cached: bool,
    pub active_dek: Option<zeroize::Zeroizing<[u8; 32]>>,
    pub startup_summary_open: bool,
    /// Open state for kit `Select` fields, keyed by select id.
    pub selects: HashMap<String, bool>,
    /// Counter page. Loaded from settings; off until the owner turns it on.
    pub show_pos: bool,
    /// Monotonic counter bumped whenever books, language, or formatting change.
    pub stamp: u64,
}

impl Drop for State {
    fn drop(&mut self) {
        if !self.locked
            && let Some(dek) = self.active_dek.take()
            && self.engine.close_store().is_ok()
        {
            let _ = crate::security::seal_workspace_data(&self.engine.dir, &dek);
        }
    }
}

impl State {
    pub fn with_engine(engine: Engine, dark: bool) -> Self {
        crate::logfile::init(&engine.dir);
        crate::logfile::line(
            "INFO",
            &format!(
                "start version={} ({}) os={} books={}",
                env!("CARGO_PKG_VERSION"),
                env!("SARV_GIT_SHA"),
                std::env::consts::OS,
                engine.dir.display()
            ),
        );
        let locked = crate::security::is_enabled(&engine.dir);
        let mut books = Books::default();
        if !locked {
            books.refresh(&engine);
        }
        let startup_summary_open = !locked && books.has_company() && books.initialized();
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
            palette_cursor: 0,
            palette_last_query: String::new(),
            period_switch_open: false,
            company_switch_open: false,
            confirm: None,
            detail: Default::default(),
            operation: Default::default(),
            sidebar: PaneState::new(260.0).range(220.0, 360.0),
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
            recovery: Default::default(),
            lock_state: Default::default(),
            locked,
            app_lock_cached: locked,
            active_dek: None,
            startup_summary_open,
            selects: HashMap::new(),
            show_pos: false,
            stamp: 0,
        };
        if locked {
            s.lock_state.sync_from_disk(&s.engine.dir);
            s.forms.edit("lock-password").focused = true;
        } else {
            s.apply_settings();
            s.refresh_recovery_if_failed();
        }
        if let Some(crash_path) = crate::logfile::take_unseen_crash() {
            let msg = s.fmt.t("Sarv closed unexpectedly last time. A report was saved on this device.");
            s.push_toast(Tone::Info, msg, Some(crash_path));
        }
        s
    }

    /// Opens the books named by `SARV_DATA_DIR`; `SARV_DEMO=1` seeds a throwaway QA profile.
    pub fn open() -> Self {
        let engine = if std::env::var_os("SARV_DEMO").is_some() {
            Engine::demo()
        } else {
            Engine::open()
        };
        Self::with_engine(engine, true)
    }

    fn apply_settings_fields(&mut self) {
        let st = &self.books.workspace["settings"];
        match st["text_direction"].as_str() {
            Some("Ltr") => self.fmt.lang = Lang::En,
            _ => self.fmt.lang = Lang::Fa,
        }
        if let Some(d) = st["display_digits"].as_str() {
            self.fmt.persian_digits = d == "Persian";
        } else {
            self.fmt.persian_digits = self.fmt.lang == Lang::Fa;
        }
        if let Some(u) = st["default_currency"].as_str() {
            self.fmt.unit = if u == "Rial" { Unit::Rial } else { Unit::Toman };
        }
        if let Some(c) = st["date_display"].as_str() {
            self.fmt.calendar = if c == "Jalali" {
                CalendarKind::Jalali
            } else {
                CalendarKind::Gregorian
            };
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

    /// Digits, unit, calendar and appearance follow the saved settings.
    pub fn apply_settings(&mut self) {
        self.apply_settings_fields();
        self.stamp += 1;
        views::ensure_page(self, self.page);
    }

    pub fn lang(&self) -> Lang {
        self.fmt.lang
    }

    pub fn set_lang(&mut self, lang: Lang) {
        self.fmt.lang = lang;
        self.stamp += 1;
        views::ensure_page(self, self.page);
    }

    /// Persist appearance, language, digits, unit, calendar, and feature toggles to the books.
    pub fn save_preferences(&mut self) -> bool {
        if !self.books.has_company() {
            return true;
        }
        let appearance = if self.dark { "Dark" } else { "Light" };
        let digits = if self.fmt.persian_digits { "Persian" } else { "Latin" };
        let currency = if self.fmt.unit == Unit::Rial { "Rial" } else { "Toman" };
        let date_display = if self.fmt.calendar == CalendarKind::Jalali {
            "Jalali"
        } else {
            "Iso"
        };
        let mut settings = self.books.workspace["settings"].clone();
        if settings.is_null() {
            settings = serde_json::json!({});
        }
        settings["appearance"] = serde_json::json!(appearance);
        settings["display_digits"] = serde_json::json!(digits);
        settings["default_currency"] = serde_json::json!(currency);
        settings["date_display"] = serde_json::json!(date_display);
        settings["text_direction"] = serde_json::json!(if self.fmt.lang == Lang::En { "Ltr" } else { "Rtl" });
        settings["show_pos"] = serde_json::json!(self.show_pos);
        for (k, d) in [
            ("next_number", "1"),
            ("default_validity_days", "14"),
            ("default_discount_rial", "0"),
            ("tax_basis_points", "0"),
        ] {
            if settings[k].as_str().is_none() {
                settings[k] = serde_json::json!(settings[k].as_u64().map(|n| n.to_string()).unwrap_or_else(|| d.into()));
            }
        }
        match self.engine.call("settings", settings.clone()) {
            Ok(_) => {
                self.books.workspace["settings"] = settings;
                self.stamp += 1;
                views::ensure_page(self, self.page);
                true
            }
            Err(e) => {
                self.toast_err(e);
                false
            }
        }
    }

    pub fn nav(&mut self, page: Page) {
        self.nav_then(page, |_| {});
    }

    pub fn nav_then(&mut self, page: Page, after: impl Fn(&mut State) + 'static) {
        if page == Page::Pos && !self.show_pos {
            self.toast(Tone::Info, self.fmt.t("Turn on Point of sale in Settings."));
            return;
        }
        if self.page == Page::Sheets && page != Page::Sheets && self.sheets.dirty {
            self.confirm = Some(crate::ui::confirm::Confirm::one(
                self.fmt.t("Leave without saving the sheet?"),
                self.fmt.t("Unsaved changes on this sheet will be lost."),
                Vec::new(),
                "Leave",
                self.fmt.t("Leave"),
                true,
                move |st| {
                    st.apply_nav(page);
                    after(st);
                },
            ));
            return;
        }
        self.apply_nav(page);
        after(self);
    }

    pub fn refresh_lock_status(&mut self) {
        self.app_lock_cached = crate::security::is_enabled(&self.engine.dir);
    }

    pub fn app_lock_enabled(&self) -> bool {
        self.app_lock_cached
    }

    pub fn lock_app(&mut self) {
        self.refresh_lock_status();
        if !self.app_lock_enabled() {
            return;
        }
        if let Err(e) = self.engine.close_store() {
            crate::logfile::line("ERROR", &format!("close_store failed during lock: {e}"));
            self.toast(Tone::Danger, e);
            return;
        }
        if let Some(dek) = self.active_dek.as_ref()
            && let Err(e) = crate::security::seal_workspace_data(&self.engine.dir, dek)
        {
            crate::logfile::line("ERROR", &format!("seal_workspace_data failed: {e}"));
            self.toast(Tone::Danger, e);
            return;
        }
        self.active_dek = None;
        self.locked = true;
        self.lock_state = Default::default();
        self.lock_state.sync_from_disk(&self.engine.dir);
        self.books = Books::default();
        self.forms = Forms::default();
        self.toasts.clear();
        self.detail = Default::default();
        self.operation = Default::default();
        self.confirm = None;
        self.palette_open = false;
        self.period_switch_open = false;
        self.company_switch_open = false;
        self.startup_summary_open = false;
        self.selects.clear();
        self.sales = Default::default();
        self.purchases = Default::default();
        self.payments = Default::default();
        self.banking = Default::default();
        self.contacts = Default::default();
        self.inventory = Default::default();
        self.accounting = Default::default();
        self.reports = Default::default();
        self.sheets = Default::default();
        self.settings = Default::default();
        self.overview = Default::default();
        self.pos = Default::default();
        self.forms.edit("lock-password").focused = true;
    }

    fn apply_nav(&mut self, page: Page) {
        self.page = page;
        self.palette_open = false;
        views::ensure_page(self, page);
        if page == Page::Pos {
            // Barcode scans land in the catalog search as soon as POS opens.
            self.forms.edit("pos-search").focused = true;
        }
    }

    pub fn books_ready(&self) -> bool {
        !self.books.load_failed() && self.books.has_company() && self.books.initialized() && !self.setup.adding_company
    }

    pub fn any_dialog_open(&self) -> bool {
        self.confirm.is_some()
            || self.detail.open
            || self.operation.open
            || self.palette_open
            || self.period_switch_open
            || self.company_switch_open
            || self.sales.creating_item_for_line.is_some()
            || self.purchases.creating_item_for_line.is_some()
            || self.accounting.ledger_open
            || self.accounting.period_add_open
            || self.accounting.acct_archive_open
            || self.accounting.acct_open
            || self.settings.restore_open
            || self.settings.sql_import_open
            || self.settings.lock_dialog.is_some()
            || self.startup_summary_open
            || self.overview.starter_open
            || self.overview.named_open
            || self.sales.modal_open
            || self.purchases.modal_open
            || self.payments.spend_open
            || self.payments.modal_open
            || self.accounting.modal_open
            || self.banking.modal_open
            || self.banking.rec_open
            || self.contacts.modal_open
            || !self.contacts.open_id.is_empty()
            || self.inventory.modal_open
            || self.inventory.profile_open
            || self.inventory.transfer_open
            || self.settings.business_open
    }

    /// Close the topmost open overlay/dialog, running dirty checks for form dialogs.
    pub fn request_close_top_dialog(&mut self) -> bool {
        if self.confirm.is_some() {
            self.confirm = None;
            return true;
        }
        if self.palette_open {
            self.palette_open = false;
            return true;
        }
        if self.startup_summary_open {
            self.startup_summary_open = false;
            return true;
        }
        if self.company_switch_open {
            self.company_switch_open = false;
            return true;
        }
        if self.period_switch_open {
            self.period_switch_open = false;
            return true;
        }
        if self.accounting.period_add_open {
            self.accounting.period_add_open = false;
            return true;
        }
        if self.accounting.ledger_open {
            self.accounting.ledger_open = false;
            return true;
        }
        if self.detail.open {
            self.detail.open = false;
            return true;
        }
        if self.operation.open {
            self.operation.open = false;
            return true;
        }
        if self.settings.lock_dialog.is_some() {
            self.settings.lock_dialog = None;
            self.settings.lock_error.clear();
            self.forms.clear(&["sec-cur-pass", "sec-new-pass", "sec-confirm-pass"]);
            return true;
        }
        if self.accounting.acct_archive_open {
            self.accounting.acct_archive_open = false;
            return true;
        }
        if self.accounting.acct_open {
            crate::ui::views::accounting::close_acct_or_confirm(self);
            return true;
        }
        if self.sales.creating_item_for_line.is_some() {
            self.sales.creating_item_for_line = None;
            return true;
        }
        if self.purchases.creating_item_for_line.is_some() {
            self.purchases.creating_item_for_line = None;
            return true;
        }
        if self.settings.restore_open {
            self.settings.restore_open = false;
            return true;
        }
        if self.settings.sql_import_open {
            self.settings.sql_import_open = false;
            self.settings.sql_import_preview = None;
            return true;
        }
        if self.overview.starter_open {
            if !self.overview.starter_busy {
                self.overview.starter_open = false;
            }
            return true;
        }
        if self.overview.named_open {
            self.overview.named_open = false;
            return true;
        }
        if self.sales.modal_open {
            views::sales::close_or_confirm(self);
            return true;
        }
        if self.purchases.modal_open {
            views::purchases::close_or_confirm(self);
            return true;
        }
        if self.payments.spend_open {
            views::payments::close_spend_or_confirm(self);
            return true;
        }
        if self.payments.modal_open {
            views::payments::close_or_confirm(self);
            return true;
        }
        if self.accounting.modal_open {
            views::accounting::close_or_confirm(self);
            return true;
        }
        if self.banking.rec_open {
            self.banking.rec_open = false;
            return true;
        }
        if self.banking.modal_open {
            views::banking::close_or_confirm(self);
            return true;
        }
        if self.contacts.modal_open {
            views::contacts::close_or_confirm(self);
            return true;
        }
        if !self.contacts.open_id.is_empty() {
            views::contacts::close_profile_or_confirm(self);
            return true;
        }
        if self.inventory.modal_open {
            views::inventory::close_new_or_confirm(self);
            return true;
        }
        if self.inventory.profile_open {
            views::inventory::close_profile_or_confirm(self);
            return true;
        }
        if self.inventory.transfer_open {
            views::inventory::close_transfer_or_confirm(self);
            return true;
        }
        if self.settings.business_open {
            views::settings::close_business_or_confirm(self);
            return true;
        }
        false
    }

    pub fn toggle_sidebar(&mut self) {
        self.sidebar.toggle();
    }

    pub fn sidebar_open(&self) -> bool {
        self.sidebar.is_open()
    }

    pub fn open_palette(&mut self) {
        self.forms.set("palette-query", "");
        self.forms.edit("palette-query").focused = true;
        self.palette_cursor = 0;
        self.palette_last_query.clear();
        self.palette_open = true;
    }

    fn push_toast(&mut self, tone: Tone, message: impl Into<String>, open_path: Option<PathBuf>) {
        let id = self.next_toast;
        self.next_toast += 1;
        self.toasts.push(Toast {
            id,
            tone,
            message: message.into(),
            open_path,
        });
        if self.toasts.len() > 4 {
            self.toasts.remove(0);
        }
    }

    pub fn toast(&mut self, tone: Tone, message: impl Into<String>) {
        self.push_toast(tone, message, None);
    }

    pub fn toast_file(&mut self, path: PathBuf) {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
        let msg = format!("{} {}", self.fmt.t("Saved"), crate::i18n::isolate(name));
        self.push_toast(Tone::Success, msg, Some(path));
    }

    /// Parse a form date field in the active calendar. Empty defaults to today; invalid toasts an error.
    #[allow(clippy::result_unit_err)]
    pub fn form_date(&mut self, key: &str) -> Result<String, ()> {
        let raw = self.forms.get(key);
        if raw.trim().is_empty() {
            return Ok(crate::data::today());
        }
        match self.fmt.parse_date(&raw) {
            Ok(iso) => Ok(iso),
            Err(msg) => {
                self.toast(Tone::Danger, msg);
                Err(())
            }
        }
    }

    /// A request key unique within and across runs: prefix, process id, nanoseconds, counter.
    pub fn mint_key(&mut self, prefix: &str) -> String {
        format!(
            "{prefix}-{}-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            next_key_seq()
        )
    }

    pub fn refresh(&mut self) {
        self.books.refresh(&self.engine);
        self.stamp += 1;
        views::ensure_page(self, self.page);
        self.refresh_recovery_if_failed();
    }

    pub fn refresh_and_apply_settings(&mut self) {
        self.books.refresh(&self.engine);
        self.apply_settings_fields();
        self.stamp += 1;
        views::ensure_page(self, self.page);
        self.refresh_recovery_if_failed();
    }

    pub(crate) fn refresh_recovery_if_failed(&mut self) {
        if self.books.load_failed() {
            self.recovery.backups = self.query("backup_list", serde_json::json!({})).unwrap_or_default();
        } else {
            self.recovery = Default::default();
        }
    }

    pub fn select_company(&mut self, id: &str) {
        if self.page == Page::Sheets && self.sheets.dirty {
            let id = id.to_string();
            self.confirm = Some(crate::ui::confirm::Confirm::one(
                self.fmt.t("Leave without saving the sheet?"),
                self.fmt.t("Unsaved changes on this sheet will be lost."),
                Vec::new(),
                "Leave",
                self.fmt.t("Leave"),
                true,
                move |st| st.apply_select_company(&id),
            ));
            return;
        }
        self.apply_select_company(id);
    }

    fn apply_select_company(&mut self, id: &str) {
        self.books.company_id = id.to_string();
        self.books.period_id.clear();
        self.sheets.loaded = false;
        self.sheets.dirty = false;
        self.pos = views::pos::PosState::default();
        self.refresh_and_apply_settings();
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
            payload["reason"] = Value::String(self.fmt.t("Recorded in Sarv"));
        }
        if blank(&payload["source_id"]) || blank(&payload["request_key"]) {
            let key = format!(
                "ui-{}-{}-{}",
                std::process::id(),
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                next_key_seq()
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
                crate::logfile::line("ERROR", &format!("action={action} message={e}"));
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
