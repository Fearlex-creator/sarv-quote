//! Native connected purchases and stock workspaces. Mutations cross the root worker boundary.
use super::accounting::{DateField, date_field, date_value, form_dialog};
use super::core::RunCommand;
use super::*;
use chrono::Local;
use sarv_quote::accounting::{AccountingState, Money, MoneyUnit};
use sarv_quote::core::*;
use sarv_quote::procurement::OrderLine;
use sarv_quote::procurement::*;
use std::{rc::Weak, sync::Arc};
use uuid::Uuid;

const ACTOR: &str = "local-single-user";
const PAGE_SIZE: usize = 25;
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PageKind {
    Purchases,
    Inventory,
}
fn named_entry(caption: &str, name: &str, value: &str, ltr: bool) -> gtk::Entry {
    let e = entry(caption);
    e.set_widget_name(name);
    e.set_text(value);
    accessible_name(&e, caption);
    if ltr {
        e.set_direction(gtk::TextDirection::Ltr);
    }
    e
}
fn search_entry(caption: &str, name: &str) -> gtk::SearchEntry {
    let e = gtk::SearchEntry::new();
    e.set_search_delay(50);
    e.set_widget_name(name);
    e.set_placeholder_text(Some(caption));
    accessible_name(&e, caption);
    e
}
fn choice(caption: &str, name: &str, values: &[(String, String)]) -> gtk::DropDown {
    let labels = values
        .iter()
        .map(|(id, label)| format!("{label} • {}", isolate(id)))
        .collect::<Vec<_>>();
    let d = gtk::DropDown::from_strings(&labels.iter().map(String::as_str).collect::<Vec<_>>());
    d.set_enable_search(true);
    d.set_widget_name(name);
    accessible_name(&d, caption);
    d
}
fn selected(choice: &gtk::DropDown, ids: &[String], caption: &str) -> Result<String> {
    ids.get(choice.selected() as usize)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("{caption} را انتخاب کنید"))
}
fn isolate(s: &str) -> String {
    format!("\u{2068}{s}\u{2069}")
}
fn show_error(error: &gtk::Label, e: impl std::fmt::Display) {
    error.set_text(&e.to_string());
    error.set_visible(true);
}
fn money_unit(choice: &gtk::DropDown) -> MoneyUnit {
    if choice.selected() == 0 {
        MoneyUnit::Rial
    } else {
        MoneyUnit::Toman
    }
}
fn money_entry(entry: &gtk::Entry, unit: MoneyUnit) -> Result<Money> {
    let result = Money::parse(entry.text().trim(), unit).and_then(|m| {
        anyhow::ensure!(
            m.rials() >= 0 && m.rials() <= 9_000_000_000_000_000,
            "مبلغ باید نامنفی و در دامنهٔ مجاز باشد"
        );
        Ok(m)
    });
    entry_validity(
        entry,
        result.as_ref().err().map(ToString::to_string).as_deref(),
        "ریال صحیح یا تومان دقیق؛ کسر ریال پذیرفته نیست",
    );
    if result.is_err() {
        entry.grab_focus();
    }
    result
}
fn financial_date(ui: &Ui, date: &str) -> String {
    let s = &ui.store.borrow().state.settings;
    isolate(&managers::display_date(
        date,
        s.display_digits,
        s.date_display,
    ))
}
fn display_money(settings: &Settings, money: Money, unit: MoneyUnit) -> String {
    format!(
        "{} {}",
        isolate(&settings.display_digits.format(money.display(unit))),
        if unit == MoneyUnit::Rial {
            "ریال"
        } else {
            "تومان"
        }
    )
}
fn wrapping_tabs(stack: &gtk::Stack, tabs: &[(&str, &str)]) -> gtk::FlowBox {
    let flow = gtk::FlowBox::builder()
        .accessible_role(gtk::AccessibleRole::TabList)
        .selection_mode(gtk::SelectionMode::None)
        .min_children_per_line(1)
        .max_children_per_line(5)
        .column_spacing(6)
        .row_spacing(6)
        .build();
    for (name, title) in tabs {
        let name = (*name).to_string();
        let b = gtk::ToggleButton::builder()
            .label(*title)
            .accessible_role(gtk::AccessibleRole::Tab)
            .build();
        b.set_widget_name(&format!("procurement-tab-{name}"));
        let weak = stack.downgrade();
        let target = name.clone();
        b.connect_clicked(move |b| {
            if let Some(stack) = weak.upgrade() {
                stack.set_visible_child_name(&target);
                b.set_active(true);
            }
        });
        b.connect_toggled(|b| {
            b.update_state(&[gtk::accessible::State::Selected(Some(b.is_active()))])
        });
        let weak = b.downgrade();
        stack.connect_visible_child_name_notify(move |stack| {
            if let Some(b) = weak.upgrade() {
                b.set_active(stack.visible_child_name().as_deref() == Some(name.as_str()));
            }
        });
        insert_action(&flow, &b);
        if let Some(wrapper) = b.parent() {
            wrapper.set_focusable(false);
        }
    }
    flow
}

#[derive(Clone)]
struct CatalogPick {
    catalog_id: String,
    catalog_name: String,
    fingerprint: String,
    product: Product,
}
struct CatalogSnapshot {
    catalog: Catalog,
    index: SearchIndex,
}
type CatalogReady = Box<dyn FnOnce(Arc<CatalogSnapshot>)>;
#[derive(Default)]
struct CatalogCache {
    ready: RefCell<Option<Arc<CatalogSnapshot>>>,
    pending: RefCell<Option<(String, String)>>,
    callbacks: RefCell<Vec<CatalogReady>>,
}
thread_local! {static PROCUREMENT_CATALOG_CACHE:Rc<CatalogCache>=Rc::new(CatalogCache::default());}
fn cached_catalog(
    ui: &Rc<Ui>,
    catalog_id: &str,
    callback: impl FnOnce(Arc<CatalogSnapshot>) + 'static,
) {
    if ui.background_busy.get() {
        let weak = Rc::downgrade(ui);
        let id = catalog_id.to_string();
        let pending = RefCell::new(Some(callback));
        glib::timeout_add_local(std::time::Duration::from_millis(60), move || {
            let Some(ui) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            if ui.background_busy.get() {
                return glib::ControlFlow::Continue;
            }
            if let Some(callback) = pending.borrow_mut().take() {
                cached_catalog(&ui, &id, callback);
            }
            glib::ControlFlow::Break
        });
        return;
    }
    let cache = PROCUREMENT_CATALOG_CACHE.with(Clone::clone);
    let key = {
        let store = ui.store.borrow();
        let Some(catalog) = store.state.catalogs.iter().find(|c| c.id == catalog_id) else {
            ui.message(Err::<String, _>(anyhow::anyhow!("فهرست قیمت پیدا نشد")));
            return;
        };
        (catalog.id.clone(), catalog.fingerprint.clone())
    };
    if let Some(ready) = cache.ready.borrow().as_ref()
        && (
            ready.catalog.id.as_str(),
            ready.catalog.fingerprint.as_str(),
        ) == (key.0.as_str(), key.1.as_str())
    {
        callback(ready.clone());
        return;
    }
    if cache.pending.borrow().as_ref() == Some(&key) {
        cache.callbacks.borrow_mut().push(Box::new(callback));
        return;
    }
    cache.callbacks.borrow_mut().clear();
    cache.callbacks.borrow_mut().push(Box::new(callback));
    *cache.pending.borrow_mut() = Some(key.clone());
    let catalog = ui
        .store
        .borrow()
        .state
        .catalogs
        .iter()
        .find(|c| c.id == catalog_id)
        .expect("source checked")
        .clone();
    let complete_cache = cache.clone();
    let started = ui.run_background(
        "آماده‌سازی جست‌وجوی قیمت سفارش خرید…",
        move || {
            let index = catalog.search_index();
            Ok(Arc::new(CatalogSnapshot { catalog, index }))
        },
        move |ui, result| {
            if complete_cache.pending.borrow().as_ref() != Some(&key) {
                ui.message(Ok(
                    "آماده‌سازی قیمت قبلی پایان یافت؛ منبع انتخاب‌شده تغییر کرد.",
                ));
                return;
            }
            complete_cache.pending.borrow_mut().take();
            match result {
                Ok(snapshot) => {
                    *complete_cache.ready.borrow_mut() = Some(snapshot.clone());
                    ui.message(Ok("جست‌وجوی قیمت سفارش خرید آماده است."));
                    let callbacks = complete_cache
                        .callbacks
                        .borrow_mut()
                        .drain(..)
                        .collect::<Vec<_>>();
                    for callback in callbacks {
                        callback(snapshot.clone());
                    }
                }
                Err(error) => {
                    complete_cache.callbacks.borrow_mut().clear();
                    ui.message(Err::<String, _>(error));
                }
            }
        },
    );
    if !started {
        cache.pending.borrow_mut().take();
        cache.callbacks.borrow_mut().clear();
    }
}
fn clear_list(list: &gtk::ListBox) {
    while let Some(c) = list.first_child() {
        list.remove(&c);
    }
}
fn catalog_picker(
    ui: &Rc<Ui>,
    parent: &adw::Dialog,
    on_choose: Rc<dyn Fn(CatalogPick) -> Result<()>>,
) {
    let catalogs = ui
        .store
        .borrow()
        .state
        .catalogs
        .iter()
        .map(|c| {
            (
                c.id.clone(),
                format!("{} • {}", c.name, financial_date(ui, &c.imported_at)),
            )
        })
        .collect::<Vec<_>>();
    if catalogs.is_empty() {
        ui.message(Err::<String, _>(anyhow::anyhow!(
            "فهرست قیمت تأییدشده‌ای موجود نیست؛ قیمت خرید را با منبع و دلیل دستی وارد کنید"
        )));
        return;
    }
    let dialog = adw::Dialog::builder()
        .title("کالا و قیمت برای سفارش خرید")
        .content_width(660)
        .content_height(680)
        .build();
    dialog.set_widget_name("procurement-price-picker");
    let root = column();
    root.set_margin_top(16);
    root.set_margin_bottom(16);
    root.set_margin_start(16);
    root.set_margin_end(16);
    root.append(&label("این انتخاب فهرست یا پیشنهاد فروش را تغییر نمی‌دهد. کالا و واحد هسته را در سطر سفارش صریحاً تطبیق دهید؛ قیمت به ازای همان واحد منبع است."));
    let sources = choice("فهرست قیمت", "procurement-price-catalog", &catalogs);
    root.append(&field("فهرست قیمت و تاریخ منبع", &sources));
    let search = search_entry("کد، نام یا نام مستعار کالا", "procurement-price-search");
    root.append(&search);
    let hint = label("در حال آماده‌سازی فهرست…");
    root.append(&hint);
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.set_activate_on_single_click(false);
    list.set_widget_name("procurement-price-results");
    root.append(&scroll(&list));
    let error = label("");
    error.add_css_class("error");
    error.set_visible(false);
    root.append(&error);
    root.append(&label(
        "↑/↓ حرکت بین نتایج • Enter انتخاب صریح • Escape بازگشت",
    ));
    let actions = row();
    let choose = button("انتخاب قیمت منبع");
    choose.set_widget_name("procurement-price-choose");
    choose.add_css_class("suggested-action");
    let cancel = button("بازگشت به سفارش");
    let weak = dialog.downgrade();
    cancel.connect_clicked(move |_| {
        if let Some(d) = weak.upgrade() {
            d.close();
        }
    });
    actions.append(&choose);
    actions.append(&cancel);
    root.append(&actions);
    dialog.set_child(Some(&dialog_toolbar(&root)));
    let loaded = Rc::new(RefCell::new(None::<Arc<CatalogSnapshot>>));
    let ids = Rc::new(RefCell::new(Vec::<usize>::new()));
    let settings = ui.store.borrow().state.settings.clone();
    let refresh: Rc<dyn Fn()> = {
        let loaded = loaded.clone();
        let ids = ids.clone();
        let list = list.clone();
        let hint = hint.clone();
        let search = search.clone();
        Rc::new(move || {
            let loaded = loaded.borrow();
            let Some(snapshot) = loaded.as_ref() else {
                return;
            };
            let (matches, total) = snapshot.index.search_page(&search.text(), 0, 30);
            clear_list(&list);
            *ids.borrow_mut() = matches.clone();
            for index in matches {
                let p = &snapshot.catalog.products[index];
                let row = adw::ActionRow::builder()
                    .title(&p.name)
                    .subtitle(format!(
                        "{} • واحد کالا {} • {}\nمنبع {} • واحد مبلغ اصلی {}",
                        isolate(&p.code),
                        p.measure,
                        display_money(&settings, Money::from_rials(p.price_rial), MoneyUnit::Rial),
                        isolate(&settings.date_display.format(&p.source_at)),
                        p.source_unit.label()
                    ))
                    .title_lines(2)
                    .subtitle_lines(3)
                    .activatable(true)
                    .build();
                row.set_use_markup(false);
                list.append(&row);
            }
            hint.set_text(&format!(
                "{} نتیجه • نمایش حداکثر ۳۰؛ برای محدود کردن نام یا کد را دقیق‌تر بنویسید",
                settings.display_digits.format(total)
            ));
            if let Some(row) = list.row_at_index(0) {
                list.select_row(Some(&row));
            }
        })
    };
    let changed = refresh.clone();
    search.connect_search_changed(move |_| changed());
    let weak = Rc::downgrade(ui);
    let weak_dialog = dialog.downgrade();
    let loaded_activate = loaded.clone();
    let ids_activate = ids.clone();
    list.connect_row_activated(move |_, row| {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let data = loaded_activate.borrow();
        let Some(snapshot) = data.as_ref() else {
            return;
        };
        let Some(index) = ids_activate.borrow().get(row.index() as usize).copied() else {
            return;
        };
        let current =
            ui.store.borrow().state.catalogs.iter().any(|c| {
                c.id == snapshot.catalog.id && c.fingerprint == snapshot.catalog.fingerprint
            });
        if !current {
            show_error(&error, "فهرست قیمت تغییر کرده؛ دوباره منبع را انتخاب کنید");
            return;
        }
        let pick = CatalogPick {
            catalog_id: snapshot.catalog.id.clone(),
            catalog_name: snapshot.catalog.name.clone(),
            fingerprint: snapshot.catalog.fingerprint.clone(),
            product: snapshot.catalog.products[index].clone(),
        };
        match on_choose(pick) {
            Ok(()) => {
                if let Some(d) = weak_dialog.upgrade() {
                    d.close();
                }
            }
            Err(e) => show_error(&error, e),
        }
    });
    let selected_list = list.clone();
    choose.connect_clicked(move |_| {
        if let Some(row) = selected_list.selected_row() {
            selected_list.emit_by_name::<()>("row-activated", &[&row]);
        }
    });
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let key_list = list.clone();
    let weak = dialog.downgrade();
    keys.connect_key_pressed(move |_, key, _, _| {
        match key {
            gtk::gdk::Key::Down | gtk::gdk::Key::Up => {
                let index = key_list.selected_row().map(|r| r.index()).unwrap_or(0);
                let index = if key == gtk::gdk::Key::Down {
                    index + 1
                } else {
                    (index - 1).max(0)
                };
                if let Some(row) = key_list.row_at_index(index) {
                    key_list.select_row(Some(&row));
                }
            }
            gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter => {
                if let Some(row) = key_list.selected_row() {
                    key_list.emit_by_name::<()>("row-activated", &[&row]);
                }
            }
            gtk::gdk::Key::Escape => {
                if let Some(d) = weak.upgrade() {
                    d.close();
                }
            }
            _ => return glib::Propagation::Proceed,
        }
        glib::Propagation::Stop
    });
    search.add_controller(keys);
    let ui_weak = Rc::downgrade(ui);
    let loaded_change = loaded.clone();
    let refresh_change = refresh.clone();
    let source_ids = catalogs.iter().map(|x| x.0.clone()).collect::<Vec<_>>();
    let load: Rc<dyn Fn(u32)> = {
        let weak_dialog = dialog.downgrade();
        Rc::new(move |index| {
            let Some(ui) = ui_weak.upgrade() else {
                return;
            };
            let Some(id) = source_ids.get(index as usize) else {
                return;
            };
            let loaded = loaded_change.clone();
            let refresh = refresh_change.clone();
            let weak_dialog = weak_dialog.clone();
            cached_catalog(&ui, id, move |snapshot| {
                if weak_dialog.upgrade().is_some_and(|d| d.is_visible()) {
                    *loaded.borrow_mut() = Some(snapshot);
                    refresh();
                }
            });
        })
    };
    let load_change = load.clone();
    sources.connect_selected_notify(move |s| load_change(s.selected()));
    ui.present_dialog(&dialog);
    let weak_parent = parent.downgrade();
    let weak_ui = Rc::downgrade(ui);
    dialog.connect_closed(move |_| {
        if let (Some(ui), Some(parent)) = (weak_ui.upgrade(), weak_parent.upgrade())
            && parent.is_visible()
        {
            *ui.active_dialog.borrow_mut() = Some(parent);
        }
    });
    search.grab_focus();
    load(sources.selected());
}

struct ProcurementPhase {
    phase: &'static str,
    operation: &'static str,
    path: PathBuf,
    began: std::time::Instant,
    last: Cell<std::time::Instant>,
    gaps: RefCell<Vec<f64>>,
    done: Cell<bool>,
}
impl ProcurementPhase {
    fn start(
        ui: &Rc<Ui>,
        phase: &'static str,
        post: bool,
        operation: &'static str,
    ) -> Option<Rc<Self>> {
        if std::env::var("SARV_PROCUREMENT_PERFORMANCE_TRACE").as_deref() != Ok("1")
            || std::env::var("SARV_TEST_INSTANCE").is_err()
        {
            return None;
        }
        let directory = ui.store.borrow().directory.canonicalize().ok()?;
        if !(directory.starts_with("/tmp")
            || directory.starts_with(Path::new(env!("CARGO_MANIFEST_DIR")).join("artifacts")))
        {
            return None;
        }
        let now = std::time::Instant::now();
        let trace = Rc::new(Self {
            phase,
            operation,
            path: directory.join("procurement-performance-phases.jsonl"),
            began: now,
            last: Cell::new(now),
            gaps: RefCell::new(Vec::new()),
            done: Cell::new(false),
        });
        let weak_ui = Rc::downgrade(ui);
        let measure = trace.clone();
        let saw_busy = Cell::new(false);
        glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
            if measure.done.get() {
                return glib::ControlFlow::Break;
            }
            let now = std::time::Instant::now();
            measure
                .gaps
                .borrow_mut()
                .push(now.duration_since(measure.last.replace(now)).as_secs_f64() * 1000.0);
            let Some(ui) = weak_ui.upgrade() else {
                measure.finish("closed");
                return glib::ControlFlow::Break;
            };
            if post {
                if ui.persistence_busy.get() {
                    saw_busy.set(true);
                } else if saw_busy.get() {
                    measure.finish("worker_finished");
                    return glib::ControlFlow::Break;
                } else if measure.began.elapsed().as_millis() > 64 {
                    measure.finish("not_started");
                    return glib::ControlFlow::Break;
                }
            }
            if measure.began.elapsed().as_secs() > 120 {
                measure.finish("timeout");
                return glib::ControlFlow::Break;
            }
            glib::ControlFlow::Continue
        });
        Some(trace)
    }
    fn finish(&self, result: &str) {
        if self.done.replace(true) {
            return;
        }
        let now = std::time::Instant::now();
        let mut gaps = self.gaps.borrow().clone();
        gaps.push(now.duration_since(self.last.get()).as_secs_f64() * 1000.0);
        let value = serde_json::json!({"phase":self.phase,"operation":self.operation,"elapsed_ms":now.duration_since(self.began).as_secs_f64()*1000.0,"ticks":gaps.len()-1,"interval_ms":16,"max_gap_ms":gaps.iter().copied().fold(0.0,f64::max),"gaps_ms":gaps,"result":result,"boundary":if self.phase=="preview"{"dispatch through completed UI result rendering"}else{"dispatch through observed persistence busy release; durable state independently checked"}});
        let path = self.path.clone();
        let _ = std::thread::Builder::new()
            .name("sarv-procurement-metrics".into())
            .spawn(move || {
                use std::io::Write;
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                {
                    let _ = writeln!(file, "{value}");
                }
            });
    }
}
fn finish_monitor(monitor: &Option<Rc<ProcurementPhase>>, result: &str) {
    if let Some(monitor) = monitor {
        monitor.finish(result);
    }
}

fn reveal_review(output: &gtk::Box) {
    if let Some(first) = output.first_child() {
        first.set_focusable(true);
        first.grab_focus();
    }
    let weak = output.downgrade();
    glib::timeout_add_local_once(std::time::Duration::from_millis(35), move || {
        let Some(output) = weak.upgrade() else { return };
        let Some(body) = output.parent() else { return };
        let Some(bounds) = output.compute_bounds(&body) else {
            return;
        };
        let mut parent = body.parent();
        while let Some(widget) = parent {
            parent = widget.parent();
            if let Ok(scrolled) = widget.downcast::<gtk::ScrolledWindow>() {
                let a = scrolled.vadjustment();
                a.set_value(
                    (f64::from(bounds.y()) - 8.)
                        .max(a.lower())
                        .min((a.upper() - a.page_size()).max(a.lower())),
                );
                break;
            }
        }
    });
}
trait ReviewedCommand {
    fn operation(&self) -> &'static str;
}
impl ReviewedCommand for ProcurementCommand {
    fn operation(&self) -> &'static str {
        let c = self;
        match c {
            ProcurementCommand::Resources(_) => "resources",
            ProcurementCommand::Order(_) => "order",
            ProcurementCommand::Receive(_) => "receipt",
            ProcurementCommand::Bill(_) => "bill",
            ProcurementCommand::Transfer(_) => "transfer",
        }
    }
}
type RowRefresh = Rc<RefCell<Option<Rc<dyn Fn()>>>>;
type Collect<C> = Rc<dyn Fn() -> Result<C>>;
type Validate<C, P> = Arc<dyn Fn(&AccountingState, &C) -> Result<P> + Send + Sync>;
type Post<C> = Rc<dyn Fn(C) -> super::core::Mutation>;
type Render<P> = Rc<dyn Fn(&gtk::Box, &P, &Settings)>;
struct Review<C: ReviewedCommand + Clone + PartialEq + Send + 'static, P: Send + 'static> {
    ui: Weak<Ui>,
    dialog: adw::Dialog,
    error: gtk::Label,
    save: gtk::Button,
    output: gtk::Box,
    footer_total: gtk::Label,
    collect: Collect<C>,
    validate: Validate<C, P>,
    post: Post<C>,
    render: Render<P>,
    run: RunCommand,
    verified: RefCell<Option<C>>,
    revision: Cell<u64>,
    message: &'static str,
}
impl<C: ReviewedCommand + Clone + PartialEq + Send + 'static, P: Send + 'static> Review<C, P> {
    fn invalidate(&self) {
        self.revision.set(self.revision.get() + 1);
        self.verified.borrow_mut().take();
        self.save.set_sensitive(false);
        self.footer_total.set_visible(false);
        clear(&self.output);
        self.output.append(&label(
            "ورودی تغییر کرده است؛ نسخهٔ فعلی را دوباره بازبینی کنید.",
        ));
    }
    fn check(self: &Rc<Self>) {
        let command = match (self.collect)() {
            Ok(c) => c,
            Err(error) => {
                show_error(&self.error, error);
                return;
            }
        };
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        self.invalidate();
        self.error.set_visible(false);
        let revision = self.revision.get();
        let state = ui.store.borrow().state.accounting.clone();
        let validated = command.clone();
        let validate = self.validate.clone();
        let weak = Rc::downgrade(self);
        let monitor = ProcurementPhase::start(&ui, "preview", false, command.operation());
        let completion = monitor.clone();
        let launched = ui.run_background(
            "بررسی سند، مقدار و اثر حسابداری…",
            move || validate(&state, &command),
            move |ui, result| {
                let Some(review) = weak.upgrade() else {
                    finish_monitor(&completion, "closed");
                    ui.message(Ok(
                        "بازبینی پایان یافت؛ پنجره بسته شد و در این بازبینی سندی ثبت نشد.",
                    ));
                    return;
                };
                if !review.dialog.is_visible() || review.revision.get() != revision {
                    finish_monitor(&completion, "stale_or_closed");
                    ui.message(Ok(if review.dialog.is_visible() {
                        "بازبینی قبلی پایان یافت؛ ورودی تغییر کرده است و باید دوباره بازبینی شود."
                    } else {
                        "بازبینی پایان یافت؛ پنجره بسته شد و در این بازبینی سندی ثبت نشد."
                    }));
                    return;
                }
                let phase_result = if result.is_ok() {
                    "verified"
                } else {
                    "rejected"
                };
                clear(&review.output);
                match result {
                    Ok(preview) => {
                        let settings = ui.store.borrow().state.settings.clone();
                        (review.render)(&review.output, &preview, &settings);
                        *review.verified.borrow_mut() = Some(validated);
                        review.save.set_sensitive(true);
                        reveal_review(&review.output);
                        ui.message(Ok("بازبینی سند آماده است؛ هنوز ثبت نشده است."));
                    }
                    Err(error) => {
                        show_error(&review.error, &error);
                        review.output.append(&label(&error.to_string()));
                        ui.message(Err::<String, _>(error));
                    }
                }
                finish_monitor(&completion, phase_result);
            },
        );
        if !launched {
            finish_monitor(&monitor, "not_started");
            show_error(
                &self.error,
                "خواندن یا بررسی دیگری در حال انجام است؛ ورودی حفظ شد. پس از آماده‌شدن دوباره بازبینی کنید.",
            );
        }
    }
    fn commit(&self) {
        match (self.collect)() {
            Ok(command) if self.verified.borrow().as_ref() == Some(&command) => {
                if let Some(ui) = self.ui.upgrade() {
                    let _ = ProcurementPhase::start(&ui, "post", true, command.operation());
                }
                (self.run)(
                    self.dialog.clone(),
                    self.error.clone(),
                    self.save.clone(),
                    self.message,
                    (self.post)(command),
                );
            }
            Ok(_) => {
                self.invalidate();
                show_error(&self.error, "ابتدا مقادیر فعلی و اثر سند را بازبینی کنید.");
            }
            Err(error) => show_error(&self.error, error),
        }
    }
    fn text(self: &Rc<Self>, entry: &gtk::Entry) {
        let weak = Rc::downgrade(self);
        entry.connect_changed(move |_| {
            if let Some(review) = weak.upgrade() {
                review.invalidate();
            }
        });
    }
    fn choice(self: &Rc<Self>, choice: &gtk::DropDown) {
        let weak = Rc::downgrade(self);
        choice.connect_selected_notify(move |_| {
            if let Some(review) = weak.upgrade() {
                review.invalidate();
            }
        });
    }
    fn date(self: &Rc<Self>, date: &DateField) {
        self.text(&date.entry);
        self.choice(&date.calendar);
    }
    fn connect(self: &Rc<Self>, check: &gtk::Button) {
        self.save.set_sensitive(false);
        let weak = Rc::downgrade(self);
        check.connect_clicked(move |_| {
            if let Some(review) = weak.upgrade() {
                review.check();
            }
        });
        let review = self.clone();
        let handler = self.save.connect_clicked(move |_| review.commit());
        let weak = Rc::downgrade(self);
        let handler = RefCell::new(Some(handler));
        self.dialog.connect_closed(move |_| {
            if let Some(review) = weak.upgrade() {
                review.verified.borrow_mut().take();
                if let Some(handler) = handler.borrow_mut().take() {
                    review.save.disconnect(handler);
                }
            }
        });
        self.invalidate();
    }
}

#[derive(Clone)]
struct MoneyFields {
    choice: gtk::DropDown,
    entries: Vec<gtk::Entry>,
    current: Rc<Cell<MoneyUnit>>,
    changing: Rc<Cell<bool>>,
}
impl MoneyFields {
    fn new(name: &str, entries: Vec<gtk::Entry>, error: &gtk::Label) -> Self {
        let choice = gtk::DropDown::from_strings(&["ریال • IRR", "تومان • ۱ تومان = ۱۰ ریال"]);
        choice.set_widget_name(name);
        accessible_name(&choice, "واحد مبلغ‌های ورودی");
        let fields = Self {
            choice: choice.clone(),
            entries,
            current: Rc::new(Cell::new(MoneyUnit::Rial)),
            changing: Rc::new(Cell::new(false)),
        };
        let controls = fields.clone();
        let error = error.clone();
        choice.connect_selected_notify(move |choice| {
            if controls.changing.get() {
                return;
            }
            let unit = money_unit(choice);
            let old = controls.current.get();
            if unit == old {
                return;
            }
            let values = controls
                .entries
                .iter()
                .map(|entry| {
                    if entry.text().trim().is_empty() {
                        Ok(None)
                    } else {
                        money_entry(entry, old).map(Some)
                    }
                })
                .collect::<Result<Vec<_>>>();
            match values {
                Ok(values) => {
                    controls.current.set(unit);
                    for (entry, value) in controls.entries.iter().zip(values) {
                        if let Some(value) = value {
                            entry.set_text(&value.display(unit));
                        }
                    }
                }
                Err(err) => {
                    controls.changing.set(true);
                    choice.set_selected(u32::from(old == MoneyUnit::Toman));
                    controls.changing.set(false);
                    show_error(
                        &error,
                        format!("{err}؛ برای تغییر واحد ابتدا مبلغ مشخص‌شده را اصلاح کنید."),
                    );
                }
            }
        });
        fields
    }
    fn unit(&self) -> MoneyUnit {
        self.current.get()
    }
}

fn action(caption: &str, name: &str) -> gtk::Button {
    let b = button(caption);
    b.set_widget_name(name);

    b
}
fn insert_action(flow: &gtk::FlowBox, button: &impl IsA<gtk::Widget>) {
    flow.insert(button, -1);
    if let Some(parent) = button.as_ref().parent() {
        parent.set_focusable(false);
    }
}
fn disclosure(caption: &str, child: &impl IsA<gtk::Widget>) -> gtk::Box {
    let box_ = column();
    let toggle = button(caption);
    toggle.update_state(&[gtk::accessible::State::Expanded(Some(false))]);
    let child = child.as_ref().clone();
    child.set_visible(false);
    box_.append(&toggle);
    box_.append(&child);
    toggle.connect_clicked(move |button| {
        let expanded = !child.is_visible();
        child.set_visible(expanded);
        button.update_state(&[gtk::accessible::State::Expanded(Some(expanded))]);
    });
    box_
}
fn actions() -> gtk::FlowBox {
    let b = gtk::FlowBox::new();
    b.set_selection_mode(gtk::SelectionMode::None);
    b.set_min_children_per_line(1);
    b.set_max_children_per_line(4);
    b.set_column_spacing(8);
    b.set_row_spacing(8);
    b
}
fn quantities(value: &str) -> Result<i64> {
    sarv_quote::domain::quantity(value.trim())
}
fn quantity_text(value: i64) -> String {
    format!("{}.{:03}", value / 1000, value % 1000)
}
fn resources_choices(r: &RegisterResources, kind: &str) -> Vec<(String, String)> {
    match kind {
        "supplier" => r
            .parties
            .iter()
            .filter(|x| x.supplier)
            .map(|x| (x.id.clone(), x.name.clone()))
            .collect(),
        "item" => r
            .items
            .iter()
            .map(|x| (x.id.clone(), format!("{} • {}", x.sku, x.name)))
            .collect(),
        "unit" => r
            .units
            .iter()
            .map(|x| (x.id.clone(), x.name.clone()))
            .collect(),
        _ => r
            .warehouses
            .iter()
            .map(|x| (x.id.clone(), x.name.clone()))
            .collect(),
    }
}
fn ids(v: &[(String, String)]) -> Vec<String> {
    v.iter().map(|x| x.0.clone()).collect()
}
fn context(ui: &Ui, company: &str, date: &str) -> Result<ProcurementContext> {
    let store = ui.store.borrow();
    let s = &store.state.accounting;
    let setup = s
        .core
        .setups
        .iter()
        .find(|s| s.command.company_id == company)
        .ok_or_else(|| anyhow::anyhow!("ابتدا راه‌اندازی هسته را کامل کنید"))?;
    anyhow::ensure!(
        s.core
            .openings
            .iter()
            .any(|o| o.command.company_id == company),
        "ابتدا افتتاحیه را تأیید کنید"
    );
    let p = s
        .periods
        .iter()
        .find(|p| {
            p.company_id == company && p.start_date.as_str() <= date && p.end_date.as_str() >= date
        })
        .ok_or_else(|| anyhow::anyhow!("تاریخ در سال مالی شرکت نیست"))?;
    Ok(ProcurementContext {
        company_id: company.into(),
        period_id: p.id.clone(),
        setup_id: setup.id.clone(),
        source_id: Uuid::new_v4().to_string(),
        source_revision: 1,
        date: date.into(),
        request_key: Uuid::new_v4().to_string(),
        actor: ACTOR.into(),
        reason: String::new(),
    })
}
fn review(
    body: &gtk::Box,
    ui: &Rc<Ui>,
    dialog: &adw::Dialog,
    error: &gtk::Label,
    save: &gtk::Button,
    run: RunCommand,
    collect: Collect<ProcurementCommand>,
) -> Rc<Review<ProcurementCommand, ProcurementPreview>> {
    let check = action("بازبینی مقدار و اثر سند", "procurement-preview");
    body.append(&check);
    let output = gtk::Box::new(gtk::Orientation::Vertical, 8);
    output.set_widget_name("procurement-preview-output");
    body.append(&output);
    save.set_label("تأیید و ثبت نسخهٔ بازبینی‌شده");
    let display_ui = Rc::downgrade(ui);
    let footer_total = label("");
    footer_total.set_widget_name("procurement-post-total");
    footer_total.set_visible(false);
    if let Some(parent) = save.parent().and_then(|w| w.downcast::<gtk::Box>().ok()) {
        parent.append(&footer_total);
    }
    let shown_total = footer_total.clone();
    let r = Rc::new(Review {
        ui: Rc::downgrade(ui),
        dialog: dialog.clone(),
        error: error.clone(),
        save: save.clone(),
        output,
        footer_total,
        collect,
        validate: Arc::new(|s, c| s.preview_procurement(c)),
        post: Rc::new(|c| Box::new(move |s| s.post_procurement_command(c).map(|_| ()))),
        render: Rc::new(move |body, p, s| {
            if let Some(ui) = display_ui.upgrade() {
                render_preview(body, p, s, &ui.store.borrow().state.accounting);
                shown_total.set_text(&format!(
                    "{} • {}",
                    display_money(s, p.total, MoneyUnit::Rial),
                    if p.nonfinancial {
                        "غیرمالی"
                    } else {
                        "سند مالی"
                    }
                ));
                shown_total.set_visible(true);
            }
        }),
        run,
        verified: RefCell::new(None),
        revision: Cell::new(0),
        message: "ثبت منبع و اثر تأیید‌شده…",
    });
    r.connect(&check);
    r
}
fn friendly_resource(state: &AccountingState, kind: &str, id: &str) -> String {
    for record in state.procurement.records.iter().rev() {
        if let ProcurementData::Resources(r) = &record.data {
            let found = match kind {
                "item" => r
                    .items
                    .iter()
                    .find(|x| x.id == id)
                    .map(|x| format!("{} • {}", x.sku, x.name)),
                "warehouse" => r
                    .warehouses
                    .iter()
                    .find(|x| x.id == id)
                    .map(|x| x.name.clone()),
                _ => r
                    .parties
                    .iter()
                    .find(|x| x.id == id)
                    .map(|x| x.name.clone()),
            };
            if let Some(name) = found {
                return name;
            }
        }
    }
    for o in &state.core.openings {
        let found = match kind {
            "item" => o
                .command
                .items
                .iter()
                .find(|x| x.id == id)
                .map(|x| format!("{} • {}", x.sku, x.name)),
            "warehouse" => o
                .command
                .warehouses
                .iter()
                .find(|x| x.id == id)
                .map(|x| x.name.clone()),
            _ => o
                .command
                .parties
                .iter()
                .find(|x| x.id == id)
                .map(|x| x.name.clone()),
        };
        if let Some(name) = found {
            return name;
        }
    }
    isolate(id)
}
fn paper_receipt_date(state: &AccountingState, id: &str) -> Option<String> {
    for record in &state.procurement.records {
        if let (ProcurementData::Receive(r), ProcurementCommand::Receive(c)) =
            (&record.data, &record.command)
            && r.lines.iter().any(|l| l.id == id)
        {
            return Some(c.document_date.clone());
        }
    }
    state.core.openings.iter().find_map(|o| {
        o.command
            .grni
            .iter()
            .find(|r| r.id == id)
            .map(|r| r.received_date.clone())
    })
}
fn friendly_receipt(state: &AccountingState, id: &str) -> String {
    for record in &state.procurement.records {
        if let (ProcurementData::Receive(r), ProcurementCommand::Receive(c)) =
            (&record.data, &record.command)
            && r.lines.iter().any(|l| l.id == id)
        {
            return format!("{} • {}", isolate(&c.reference), isolate(id));
        }
    }
    for o in &state.core.openings {
        if let Some(r) = o.command.grni.iter().find(|x| x.id == id) {
            return format!(
                "افتتاحیه {} • {}",
                isolate(&r.receipt_reference),
                isolate(id)
            );
        }
    }
    isolate(id)
}
fn render_preview(body: &gtk::Box, p: &ProcurementPreview, s: &Settings, state: &AccountingState) {
    body.append(&label(if p.nonfinancial {
        "این رویداد غیرمالی است؛ سند مالی خالی ایجاد نمی‌شود."
    } else {
        "یک سند مالی متوازن و منبع ثابت ثبت خواهد شد."
    }));
    body.append(&label(&format!(
        "ارزش {} • {} ردیف دفتر کل • {} حرکت انبار",
        display_money(s, p.total, MoneyUnit::Rial),
        p.journal_lines.len(),
        p.movements.len()
    )));
    match &p.data {
        ProcurementData::Order(o) => {
            for l in &o.lines {
                body.append(&label(&format!(
                    "{} • پایه {} • خالص {}",
                    format_args!(
                        "{} • {} • قیمت هر واحد {}",
                        l.item.sku,
                        l.item.name,
                        display_money(s, l.input.unit_price, MoneyUnit::Rial)
                    ),
                    isolate(&quantity_text(l.base_quantity_thousandths)),
                    display_money(s, l.net, MoneyUnit::Rial)
                )));
            }
        }
        ProcurementData::Receive(r) => {
            for l in &r.lines {
                body.append(&label(&format!(
                    "ردیف {} • مقدار پایه {} • ارزش {}",
                    friendly_resource(state, "item", &l.item_id),
                    isolate(&quantity_text(l.quantity_thousandths)),
                    display_money(s, l.value, MoneyUnit::Rial)
                )));
            }
        }
        ProcurementData::Bill(b) => {
            for l in &b.lines {
                body.append(&label(&format!(
                    "منبع رسید {} • مقدار {} • تطبیق {}",
                    friendly_receipt(state, &l.receipt_line_id),
                    isolate(&quantity_text(l.quantity_thousandths)),
                    display_money(s, l.value, MoneyUnit::Rial)
                )));
            }
        }
        ProcurementData::Resources(r) => {
            for x in &r.parties {
                body.append(&label(&format!(
                    "طرف {} • {} • مشتری {} • تأمین‌کننده {}",
                    x.name,
                    isolate(&x.id),
                    yes_no(x.customer),
                    yes_no(x.supplier)
                )));
            }
            for x in &r.units {
                body.append(&label(&format!("واحد {} • {}", x.name, isolate(&x.id))));
            }
            for x in &r.items {
                body.append(&label(&format!(
                    "کالا {} • {} • واحد پایه {} • تجزیه‌ناپذیر {}",
                    x.sku,
                    x.name,
                    isolate(&x.base_unit_id),
                    yes_no(x.indivisible)
                )));
                for pack in &x.packs {
                    body.append(&label(&format!(
                        "بسته {} = {} / {} واحد پایه",
                        isolate(&pack.unit_id),
                        pack.numerator,
                        pack.denominator
                    )));
                }
            }
            for x in &r.warehouses {
                body.append(&label(&format!("انبار {} • {}", x.name, isolate(&x.id))));
            }
        }
        _ => (),
    };
    for m in &p.movements {
        body.append(&label(&format!(
            "انبار {} • مقدار قبل {} • مقدار بعد {} • ارزش قبل {} • ارزش بعد {}",
            friendly_resource(state, "warehouse", &m.warehouse_id),
            isolate(&quantity_text(m.before.quantity_thousandths)),
            isolate(&quantity_text(m.after.quantity_thousandths)),
            display_money(s, m.before.value, MoneyUnit::Rial),
            display_money(s, m.after.value, MoneyUnit::Rial)
        )));
    }
    for l in &p.journal_lines {
        body.append(&label(&format!(
            "حساب {} • بدهکار {} • بستانکار {}",
            state
                .accounts
                .iter()
                .find(|a| a.id == l.account_id)
                .map(|a| format!("{} • {}", a.code, a.name))
                .unwrap_or_else(|| isolate(&l.account_id)),
            display_money(s, l.debit, MoneyUnit::Rial),
            display_money(s, l.credit, MoneyUnit::Rial)
        )));
    }
}
thread_local! {static PAGES:RefCell<Vec<Weak<Page>>>=const{RefCell::new(Vec::new())};}
struct Page {
    ui: Weak<Ui>,
    kind: PageKind,
    run: RunCommand,
    company: gtk::DropDown,
    company_ids: RefCell<Vec<String>>,
    tabs: gtk::Stack,
    documents: gtk::Box,
    controls: gtk::Box,
    masters: gtk::Box,
    search: gtk::SearchEntry,
    as_of: DateField,
    unit: gtk::DropDown,
    page: Cell<usize>,
    master_page: Cell<usize>,
    controls_page: Cell<usize>,
    revision: Cell<u64>,
    refreshing: Cell<bool>,
    refresh_pending: Cell<bool>,
    resources: RefCell<Option<Arc<RegisterResources>>>,
    document_index: RefCell<SearchIndex>,
    rendered_document_query: RefCell<String>,
    first_document_action: RefCell<Option<glib::WeakRef<gtk::Button>>>,
    master_index: RefCell<SearchIndex>,
    master_rows: RefCell<Vec<(String, String)>>,
    records: RefCell<Vec<ProcurementRecord>>,
    reconciliation: RefCell<Option<ProcurementReconciliation>>,
    status: gtk::Label,
}
pub(super) fn build(ui: &Rc<Ui>, kind: PageKind, run: RunCommand) -> gtk::Widget {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
    root.set_margin_top(18);
    root.set_margin_bottom(18);
    root.set_margin_start(18);
    root.set_margin_end(18);
    root.set_widget_name(if kind == PageKind::Purchases {
        "procurement-workspace"
    } else {
        "inventory-workspace"
    });
    root.append(&label(if kind == PageKind::Purchases {
        "خرید • سفارش، رسید جزئی و تطبیق صورتحساب"
    } else {
        "انبار • موجودی، انتقال و کنترل مانده‌ها"
    }));
    root.append(&label("ریال پایه • مقدار تا سه رقم اعشار • مالیات غیرفعال • تاریخ مؤثر مدنی و ثبت UTC • پس از افتتاحیهٔ تأیید‌شده"));
    let company = choice("شرکت", "procurement-company", &[]);
    root.append(&company);
    let as_of = date_field(
        ui,
        "پایان روز گزارش",
        "procurement-as-of",
        &Local::now().date_naive().to_string(),
    );
    root.append(&as_of.widget);
    let unit = gtk::DropDown::from_strings(&["نمایش ریال", "نمایش تومان • ۱۰ ریال"]);
    unit.set_widget_name("procurement-display-unit");
    accessible_name(&unit, "واحد نمایش گزارش");
    root.append(&unit);
    let status = label("");
    status.set_widget_name("procurement-status");
    root.append(&status);
    let search = search_entry("جست‌وجوی شماره، منبع، کالا یا طرف", "procurement-search");
    root.append(&search);
    let stack = gtk::Stack::new();
    stack.set_vexpand(true);
    stack.set_vhomogeneous(false);
    let documents = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let controls = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let masters = gtk::Box::new(gtk::Orientation::Vertical, 8);
    for (name, title, b) in [
        ("documents", "اسناد و جریان کار", &documents),
        ("controls", "کنترل و تفصیل پایان روز", &controls),
        ("masters", "طرف‌ها، کالا، واحد و انبار", &masters),
    ] {
        stack.add_titled(b, Some(name), title);
    }
    root.append(&wrapping_tabs(
        &stack,
        &[
            ("documents", "اسناد"),
            ("controls", "کنترل و تفصیل"),
            ("masters", "تعریف منابع"),
        ],
    ));
    root.append(&stack);
    let p = Rc::new(Page {
        ui: Rc::downgrade(ui),
        kind,
        run,
        company: company.clone(),
        company_ids: RefCell::new(Vec::new()),
        tabs: stack,
        documents,
        controls,
        masters,
        search: search.clone(),
        as_of,
        unit: unit.clone(),
        page: Cell::new(0),
        master_page: Cell::new(0),
        controls_page: Cell::new(0),
        revision: Cell::new(0),
        refreshing: Cell::new(false),
        refresh_pending: Cell::new(false),
        resources: RefCell::new(None),
        document_index: RefCell::new(SearchIndex::new(Vec::<String>::new())),
        rendered_document_query: RefCell::new(String::new()),
        first_document_action: RefCell::new(None),
        master_index: RefCell::new(SearchIndex::new(Vec::<String>::new())),
        master_rows: RefCell::new(Vec::new()),
        records: RefCell::new(Vec::new()),
        reconciliation: RefCell::new(None),
        status,
    });
    PAGES.with(|v| v.borrow_mut().push(Rc::downgrade(&p)));
    let life = p.clone();
    root.connect_destroy(move |_| {
        let _ = &life;
    });
    let weak = Rc::downgrade(&p);
    company.connect_selected_notify(move |_| {
        if let Some(p) = weak.upgrade()
            && !p.refreshing.get()
        {
            p.page.set(0);
            p.refresh();
        }
    });
    let weak = Rc::downgrade(&p);
    search.connect_search_changed(move |_| {
        if let Some(p) = weak.upgrade() {
            p.page.set(0);
            p.master_page.set(0);
            p.controls_page.set(0);
            p.render();
        }
    });
    // A filtered document lookup leads directly to its first usable result.
    // Backward/modified Tab and unfiltered or not-yet-rendered queries keep
    // GTK's normal route through the report controls and document actions.
    let result_keys = gtk::EventControllerKey::new();
    result_keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let weak = Rc::downgrade(&p);
    result_keys.connect_key_pressed(move |_, key, _, modifiers| {
        let Some(p) = weak.upgrade() else {
            return glib::Propagation::Proceed;
        };
        let query = p.search.text().to_lowercase();
        if key != gtk::gdk::Key::Tab
            || modifiers.intersects(
                gtk::gdk::ModifierType::SHIFT_MASK
                    | gtk::gdk::ModifierType::CONTROL_MASK
                    | gtk::gdk::ModifierType::ALT_MASK
                    | gtk::gdk::ModifierType::SUPER_MASK
                    | gtk::gdk::ModifierType::HYPER_MASK
                    | gtk::gdk::ModifierType::META_MASK,
            )
            || p.tabs.visible_child_name().as_deref() != Some("documents")
            || p.resources.borrow().is_none()
            || p.ui
                .upgrade()
                .is_some_and(|ui| ui.background_busy.get() || ui.persistence_busy.get())
            || query.trim().is_empty()
            || *p.rendered_document_query.borrow() != query
        {
            return glib::Propagation::Proceed;
        }
        let target = p
            .first_document_action
            .borrow()
            .as_ref()
            .and_then(glib::WeakRef::upgrade);
        if let Some(target) = target
            && target.is_mapped()
            && target.is_sensitive()
            && target.grab_focus()
        {
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    search.add_controller(result_keys);
    let weak = Rc::downgrade(&p);
    unit.connect_selected_notify(move |_| {
        if let Some(p) = weak.upgrade() {
            p.render();
        }
    });
    let b = action("به‌روزرسانی پایان روز", "procurement-refresh");
    root.append(&b);
    let weak = Rc::downgrade(&p);
    b.connect_clicked(move |_| {
        if let Some(p) = weak.upgrade() {
            p.refresh();
        }
    });
    let weak = Rc::downgrade(&p);
    root.connect_map(move |_| {
        if let Some(p) = weak.upgrade() {
            p.refresh();
        }
    });
    p.refresh();
    // One scroll surface keeps the header and bounded/paged records reachable
    // in short windows without competing nested focus-scroll animations.
    let viewport = gtk::Viewport::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    viewport.set_scroll_to_focus(true);
    viewport.set_child(Some(&root));
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&viewport)
        .build();
    scroll.set_widget_name(if kind == PageKind::Purchases {
        "procurement-page-scroll"
    } else {
        "inventory-page-scroll"
    });
    let weak_viewport = viewport.downgrade();
    let weak_root = root.downgrade();
    ui.window
        .connect_notify_local(Some("focus-widget"), move |window, _| {
            let (Some(viewport), Some(root)) = (weak_viewport.upgrade(), weak_root.upgrade())
            else {
                return;
            };
            if !viewport.is_mapped() {
                return;
            }
            let Some(focus) = gtk::prelude::RootExt::focus(window) else {
                return;
            };
            if !focus.is_ancestor(&root) || focus.native() != viewport.native() {
                return;
            }
            let weak_focus = focus.downgrade();
            let weak_viewport = viewport.downgrade();
            // Focus flags and nested viewport allocation settle after notification.
            // Reveal the complete control without altering normal key activation.
            glib::idle_add_local_once(move || {
                let (Some(focus), Some(viewport)) = (weak_focus.upgrade(), weak_viewport.upgrade())
                else {
                    return;
                };
                if !focus.is_mapped() || !viewport.is_mapped() {
                    return;
                }
                if let Some(bounds) = focus.compute_bounds(&viewport) {
                    let Some(adjustment) = viewport.vadjustment() else {
                        return;
                    };
                    let top = adjustment.value() + f64::from(bounds.y());
                    adjustment.clamp_page(top, top + f64::from(bounds.height()));
                }
            });
        });
    scroll.upcast()
}
pub(super) fn refresh_all(ui: &Rc<Ui>) {
    PAGES.with(|pages| {
        pages.borrow_mut().retain(|w| {
            w.upgrade().is_some_and(|p| {
                if p.ui.upgrade().is_some_and(|u| Rc::ptr_eq(&u, ui)) {
                    p.refresh();
                }
                true
            })
        });
    });
}
impl Page {
    fn company_id(&self) -> Result<String> {
        selected(&self.company, &self.company_ids.borrow(), "شرکت")
    }
    fn refresh(self: &Rc<Self>) {
        let Some(ui) = self.ui.upgrade() else { return };
        if ui.background_busy.get() || ui.persistence_busy.get() {
            if !self.refresh_pending.replace(true) {
                let weak = Rc::downgrade(self);
                glib::timeout_add_local(std::time::Duration::from_millis(60), move || {
                    let Some(p) = weak.upgrade() else {
                        return glib::ControlFlow::Break;
                    };
                    let Some(ui) = p.ui.upgrade() else {
                        return glib::ControlFlow::Break;
                    };
                    if ui.background_busy.get() || ui.persistence_busy.get() {
                        return glib::ControlFlow::Continue;
                    }
                    p.refresh_pending.set(false);
                    p.refresh();
                    glib::ControlFlow::Break
                });
            }
            return;
        }
        let old = self.company_id().ok();
        let companies = ui.store.borrow().state.accounting.companies.clone();
        self.refreshing.set(true);
        let labels = companies
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>();
        self.company.set_model(Some(&gtk::StringList::new(&labels)));
        *self.company_ids.borrow_mut() = companies.iter().map(|c| c.id.clone()).collect();
        if let Some(i) = old.and_then(|id| companies.iter().position(|c| c.id == id)) {
            self.company.set_selected(i as u32);
        }
        self.refreshing.set(false);
        let company = match self.company_id() {
            Ok(c) => c,
            Err(_) => {
                self.status
                    .set_text("در صفحهٔ حسابداری شرکت، سال مالی و افتتاحیه را کامل کنید.");
                return;
            }
        };
        let date = match date_value(&self.as_of) {
            Ok(d) => d,
            Err(e) => {
                self.status.set_text(&e.to_string());
                return;
            }
        };
        let state = ui.store.borrow().state.accounting.clone();
        self.revision.set(self.revision.get() + 1);
        let revision = self.revision.get();
        let weak = Rc::downgrade(self);
        self.status.set_text("خواندن منابع و کنترل پایان روز…");
        let _ = ui.run_background(
            "خواندن منابع و کنترل پایان روز…",
            move || {
                let r = state.procurement_resources(&company)?;
                let c = state.procurement_reconcile(&company, &date)?;
                let mut records = state
                    .procurement
                    .records
                    .into_iter()
                    .filter(|r| r.command.context().company_id == company)
                    .collect::<Vec<_>>();
                records.sort_by_key(|record| {
                    (
                        matches!(record.data, ProcurementData::Resources(_)),
                        std::cmp::Reverse(record.sequence),
                    )
                });
                let document_index = SearchIndex::new(records.iter().map(|record| {
                    let ctx = record.command.context();
                    let reference = match &record.command {
                        ProcurementCommand::Order(x) => &x.reference,
                        ProcurementCommand::Receive(x) => &x.reference,
                        ProcurementCommand::Bill(x) => &x.reference,
                        _ => &ctx.reason,
                    };
                    let supplier = match &record.data {
                        ProcurementData::Order(x) => x.supplier.name.clone(),
                        ProcurementData::Receive(x) => {
                            resource_name(&r, "supplier", &x.supplier_id)
                        }
                        ProcurementData::Bill(x) => resource_name(&r, "supplier", &x.supplier_id),
                        _ => String::new(),
                    };
                    format!(
                        "{} {} {} {} {}",
                        record.id,
                        reference,
                        ctx.date,
                        record_title(&record.data),
                        supplier
                    )
                }));
                let mut masters = Vec::new();
                for p in &r.parties {
                    masters.push((
                        p.id.clone(),
                        format!(
                            "طرف {} • مشتری {} • تأمین‌کننده {}",
                            p.name,
                            yes_no(p.customer),
                            yes_no(p.supplier)
                        ),
                    ));
                }
                for i in &r.items {
                    masters.push((i.id.clone(), format!("کالا {} • {}", i.sku, i.name)));
                }
                for u in &r.units {
                    masters.push((u.id.clone(), format!("واحد {}", u.name)));
                }
                for w in &r.warehouses {
                    masters.push((w.id.clone(), format!("انبار {}", w.name)));
                }
                let master_index =
                    SearchIndex::new(masters.iter().map(|(id, name)| format!("{id} {name}")));
                Ok((r, c, records, document_index, master_index, masters))
            },
            move |ui, result| {
                let Some(p) = weak.upgrade() else {
                    ui.message(Ok("خواندن منابع پایان یافت؛ صفحه بسته شده است."));
                    return;
                };
                if p.revision.get() != revision {
                    ui.message(Ok(
                        "خواندن نسخهٔ قبلی منابع پایان یافت؛ گزارش جاری تغییر کرده است.",
                    ));
                    return;
                }
                match result {
                    Ok((r, c, records, document_index, master_index, masters)) => {
                        *p.resources.borrow_mut() = Some(Arc::new(r));
                        *p.document_index.borrow_mut() = document_index;
                        *p.master_index.borrow_mut() = master_index;
                        *p.master_rows.borrow_mut() = masters;
                        *p.reconciliation.borrow_mut() = Some(c);
                        *p.records.borrow_mut() = records;
                        p.status.set_text("منابع و گزارش پایان روز آماده است.");
                        p.render();
                        ui.message(Ok("منابع و گزارش پایان روز آماده است."));
                    }
                    Err(e) => {
                        p.resources.borrow_mut().take();
                        p.reconciliation.borrow_mut().take();
                        p.records.borrow_mut().clear();
                        p.master_rows.borrow_mut().clear();
                        *p.document_index.borrow_mut() = SearchIndex::new(Vec::<String>::new());
                        *p.master_index.borrow_mut() = SearchIndex::new(Vec::<String>::new());
                        p.status.set_text(&e.to_string());
                        clear(&p.documents);
                        clear(&p.controls);
                        clear(&p.masters);
                        ui.message(Err::<String, _>(e));
                    }
                }
            },
        );
    }
    fn render(self: &Rc<Self>) {
        let Some(ui) = self.ui.upgrade() else { return };
        let Some(resources) = self.resources.borrow().clone() else {
            return;
        };
        let settings = ui.store.borrow().state.settings.clone();
        let unit = money_unit(&self.unit);
        let query = self.search.text().to_lowercase();
        *self.rendered_document_query.borrow_mut() = query.clone();
        self.first_document_action.borrow_mut().take();
        clear(&self.documents);
        let bar = actions();
        self.documents.append(&bar);
        for (caption, name, kind) in if self.kind == PageKind::Purchases {
            vec![
                ("سفارش خرید تازه", "procurement-new-order", 0),
                ("صورتحساب و تطبیق رسید", "procurement-new-bill", 1),
            ]
        } else {
            vec![("انتقال بین انبارها", "procurement-new-transfer", 2)]
        } {
            let b = action(caption, name);
            insert_action(&bar, &b);
            let weak = Rc::downgrade(self);
            b.connect_clicked(move |_| {
                if let Some(p) = weak.upgrade() {
                    p.open(kind, None);
                }
            });
        }
        let start = self.page.get() * PAGE_SIZE;
        let (indices, total) = self
            .document_index
            .borrow()
            .search_page(&query, start, PAGE_SIZE);
        let records = self.records.borrow();
        let resource_events = records
            .iter()
            .filter(|r| matches!(r.data, ProcurementData::Resources(_)))
            .count();
        self.documents.append(&label(&format!(
            "{} سند عملیاتی • {} رویداد تعریف منابع • {} نتیجه • صفحهٔ {}",
            records.len() - resource_events,
            resource_events,
            total,
            self.page.get() + 1
        )));
        let documents = gtk::ListBox::new();
        documents.set_selection_mode(gtk::SelectionMode::None);
        documents.add_css_class("boxed-list");
        let history = gtk::ListBox::new();
        history.set_selection_mode(gtk::SelectionMode::None);
        history.add_css_class("boxed-list");
        let mut has_documents = false;
        let mut has_history = false;
        for record in indices.iter().filter_map(|i| records.get(*i)) {
            let row = adw::ActionRow::builder()
                .title(record_title(&record.data))
                .subtitle(record_summary(record, &resources, &ui))
                .title_lines(2)
                .subtitle_lines(3)
                .build();
            row.set_use_markup(false);
            let b = action("جزئیات و خروجی", "procurement-document-detail");
            if self.first_document_action.borrow().is_none() {
                *self.first_document_action.borrow_mut() = Some(b.downgrade());
            }
            row.add_suffix(&b);
            let id = record.id.clone();
            let weak = Rc::downgrade(self);
            b.connect_clicked(move |_| {
                if let Some(p) = weak.upgrade() {
                    p.detail(&id);
                }
            });
            if matches!(record.data, ProcurementData::Order(_)) {
                let receive = action("رسید جزئی", "procurement-receive-order");
                row.add_suffix(&receive);
                let id = record.id.clone();
                let weak = Rc::downgrade(self);
                receive.connect_clicked(move |_| {
                    if let Some(p) = weak.upgrade() {
                        p.open(3, Some(id.clone()));
                    }
                });
            }
            row.set_activatable(false);
            row.set_selectable(false);
            if matches!(record.data, ProcurementData::Resources(_)) {
                history.append(&row);
                has_history = true;
            } else {
                documents.append(&row);
                has_documents = true;
            }
        }
        if has_documents {
            self.documents
                .append(&heading("اسناد عملیاتی • تازه‌ترین ثبت‌ها در ابتدا"));
            self.documents.append(&documents);
        }
        if has_history {
            self.documents.append(&heading(
                "تاریخچهٔ تعریف منابع • جزئیات تعاریف در زبانهٔ منابع",
            ));
            self.documents.append(&history);
        }
        let paging = actions();
        for (caption, delta) in [("صفحهٔ پیشین", -1i64), ("صفحهٔ بعد", 1)] {
            let b = action(caption, "procurement-page");
            b.set_sensitive(if delta < 0 {
                start > 0
            } else {
                start + PAGE_SIZE < total
            });
            if !b.is_sensitive() {
                continue;
            }
            let weak = Rc::downgrade(self);
            b.connect_clicked(move |_| {
                if let Some(p) = weak.upgrade() {
                    p.page.set((p.page.get() as i64 + delta).max(0) as usize);
                    p.render();
                }
            });
            insert_action(&paging, &b);
        }
        if total > PAGE_SIZE {
            self.documents.append(&paging);
        }
        clear(&self.controls);
        if let Some(c) = self.reconciliation.borrow().as_ref() {
            self.controls.append(&label(&format!(
                "پایان روز {} • {} • تراز {} / {}",
                financial_date(&ui, &c.as_of),
                if c.reconciled {
                    "همهٔ کنترل‌ها برابرند"
                } else {
                    "اختلاف کنترل؛ بررسی لازم است"
                },
                display_money(&settings, c.total_debit, unit),
                display_money(&settings, c.total_credit, unit)
            )));
            for x in &c.controls {
                self.controls.append(&label(&format!(
                    "{} • دفتر {} • تفصیل {} • اختلاف {}",
                    x.role.label(),
                    display_money(&settings, x.ledger, unit),
                    display_money(&settings, x.detail, unit),
                    display_money(&settings, x.difference, unit)
                )));
            }
            let detail_start = self.controls_page.get() * PAGE_SIZE;
            self.controls.append(&label(&format!(
                "صفحهٔ تفصیل {} • موجودی {} • رسید باز {} • پرداختنی {}",
                self.controls_page.get() + 1,
                c.stock.len(),
                c.receipts.len(),
                c.payables.len()
            )));
            for x in c.stock.iter().skip(detail_start).take(PAGE_SIZE) {
                self.controls.append(&label(&format!(
                    "{} • {} • مقدار پایه {} • ارزش {}",
                    resource_name(&resources, "item", &x.item_id),
                    resource_name(&resources, "warehouse", &x.warehouse_id),
                    isolate(&quantity_text(x.quantity_thousandths)),
                    display_money(&settings, x.value, unit)
                )));
            }
            for x in c.receipts.iter().skip(detail_start).take(PAGE_SIZE) {
                self.controls.append(&label(&format!(
                    "رسید تطبیق‌نشده {} • {} • دسترسی مالی {} • مانده {} / {}",
                    isolate(&x.reference),
                    resource_name(&resources, "supplier", &x.supplier_id),
                    financial_date(&ui, &x.date),
                    isolate(&quantity_text(x.quantity_thousandths)),
                    display_money(&settings, x.value, unit)
                )));
                if let Some(date) = paper_receipt_date(&ui.store.borrow().state.accounting, &x.id) {
                    self.controls.append(&label(&format!(
                        "تاریخ برگهٔ منبع {}",
                        financial_date(&ui, &date)
                    )));
                }
            }
            for x in c.payables.iter().skip(detail_start).take(PAGE_SIZE) {
                self.controls.append(&label(&format!(
                    "پرداختنی {} • {} • سررسید {} • {}",
                    isolate(&x.reference),
                    resource_name(&resources, "supplier", &x.party_id),
                    financial_date(&ui, &x.due_date),
                    display_money(&settings, x.remaining, unit)
                )));
            }
            let exports = actions();
            for pdf in [false, true] {
                let b = action(
                    if pdf {
                        "خروجی کنترل PDF"
                    } else {
                        "خروجی کنترل CSV"
                    },
                    "procurement-export-controls",
                );
                let weak = Rc::downgrade(self);
                b.connect_clicked(move |_| {
                    if let Some(p) = weak.upgrade() {
                        p.export(None, pdf);
                    }
                });
                insert_action(&exports, &b);
            }
            self.controls.append(&exports);
            self.pager(
                &self.controls,
                2,
                c.stock.len().max(c.receipts.len()).max(c.payables.len()),
            );
        }
        clear(&self.masters);
        let add = action("تعریف طرف، کالا، واحد یا انبار", "procurement-new-resource");
        let weak = Rc::downgrade(self);
        add.connect_clicked(move |_| {
            if let Some(p) = weak.upgrade() {
                p.open(4, None);
            }
        });
        self.masters.append(&add);
        let (indices, total) = self.master_index.borrow().search_page(
            &query,
            self.master_page.get() * PAGE_SIZE,
            PAGE_SIZE,
        );
        self.masters.append(&label(&format!(
            "{} منبع • نمایش تا {} ردیف در صفحه",
            total, PAGE_SIZE
        )));
        let rows = self.master_rows.borrow();
        let masters = gtk::ListBox::new();
        masters.set_selection_mode(gtk::SelectionMode::None);
        masters.add_css_class("boxed-list");
        self.masters.append(&masters);
        for i in indices {
            if let Some((id, name)) = rows.get(i) {
                let row = adw::ActionRow::builder()
                    .title(name)
                    .subtitle(isolate(id))
                    .build();
                row.set_use_markup(false);
                row.set_activatable(false);
                row.set_selectable(false);
                masters.append(&row);
            }
        }
        self.pager(&self.masters, 1, total);
    }
    fn pager(self: &Rc<Self>, body: &gtk::Box, kind: u8, total: usize) {
        if total <= PAGE_SIZE {
            return;
        }
        let current = if kind == 1 {
            self.master_page.get()
        } else {
            self.controls_page.get()
        };
        let bar = actions();
        for (caption, delta) in [("تفصیل پیشین", -1i64), ("تفصیل بعد", 1)] {
            let b = action(caption, "procurement-detail-paging");
            b.set_sensitive(if delta < 0 {
                current > 0
            } else {
                (current + 1) * PAGE_SIZE < total
            });
            if !b.is_sensitive() {
                continue;
            }
            let weak = Rc::downgrade(self);
            b.connect_clicked(move |_| {
                if let Some(p) = weak.upgrade() {
                    let page = if kind == 1 {
                        &p.master_page
                    } else {
                        &p.controls_page
                    };
                    page.set((page.get() as i64 + delta).max(0) as usize);
                    p.render();
                }
            });
            insert_action(&bar, &b);
        }
        body.append(&bar);
    }
}
fn report_rows(body: &gtk::Box, lines: Vec<String>) {
    let rows = gtk::Box::new(gtk::Orientation::Vertical, 6);
    body.append(&rows);
    let page = Rc::new(Cell::new(0usize));
    let slot: RowRefresh = Rc::new(RefCell::new(None));
    let update_slot = Rc::downgrade(&slot);
    let update: Rc<dyn Fn()> = Rc::new(move || {
        clear(&rows);
        let start = page.get() * PAGE_SIZE;
        rows.append(&label(&format!(
            "جزئیات {} تا {} از {}",
            start + 1,
            (start + PAGE_SIZE).min(lines.len()),
            lines.len()
        )));
        for text in lines.iter().skip(start).take(PAGE_SIZE) {
            rows.append(&label(text));
        }
        if lines.len() <= PAGE_SIZE {
            return;
        }
        let bar = actions();
        for (caption, delta) in [("جزئیات پیشین", -1i64), ("جزئیات بعد", 1)] {
            let b = action(caption, "procurement-detail-page");
            b.set_sensitive(if delta < 0 {
                start > 0
            } else {
                start + PAGE_SIZE < lines.len()
            });
            if !b.is_sensitive() {
                continue;
            }
            let p = page.clone();
            let slot = update_slot.clone();
            b.connect_clicked(move |_| {
                p.set((p.get() as i64 + delta).max(0) as usize);
                if let Some(s) = slot.upgrade()
                    && let Some(f) = s.borrow().as_ref()
                {
                    f();
                }
            });
            insert_action(&bar, &b);
        }
        rows.append(&bar);
    });
    *slot.borrow_mut() = Some(update.clone());
    update();
    let life = slot;
    body.connect_destroy(move |_| {
        let _ = &life;
    });
}
pub(super) fn open_document(ui: &Rc<Ui>, record_id: &str) {
    let company = match ui
        .store
        .borrow()
        .state
        .accounting
        .procurement_document(record_id)
    {
        Ok(r) => r.command.context().company_id.clone(),
        Err(e) => {
            ui.message(Err::<String, _>(e));
            return;
        }
    };
    ui.stack.set_visible_child_name("procurement");
    if let Ok(p) = qa_page(ui, PageKind::Purchases) {
        let companies = ui.store.borrow().state.accounting.companies.clone();
        p.refreshing.set(true);
        p.company.set_model(Some(&gtk::StringList::new(
            &companies
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>(),
        )));
        *p.company_ids.borrow_mut() = companies.iter().map(|c| c.id.clone()).collect();
        if let Some(i) = companies.iter().position(|c| c.id == company) {
            p.company.set_selected(i as u32);
        }
        p.refreshing.set(false);
        p.detail(record_id);
    }
}
fn yes_no(value: bool) -> &'static str {
    if value { "بله" } else { "خیر" }
}
fn resource_name(r: &RegisterResources, kind: &str, id: &str) -> String {
    match kind {
        "item" => r
            .items
            .iter()
            .find(|x| x.id == id)
            .map(|x| format!("{} • {}", x.sku, x.name)),
        "unit" => r.units.iter().find(|x| x.id == id).map(|x| x.name.clone()),
        "supplier" => r
            .parties
            .iter()
            .find(|x| x.id == id)
            .map(|x| x.name.clone()),
        _ => r
            .warehouses
            .iter()
            .find(|x| x.id == id)
            .map(|x| x.name.clone()),
    }
    .unwrap_or_else(|| isolate(id))
}

fn record_title(d: &ProcurementData) -> &'static str {
    match d {
        ProcurementData::Resources(_) => "تعریف منابع",
        ProcurementData::Order(_) => "سفارش خرید",
        ProcurementData::Receive(_) => "رسید انبار",
        ProcurementData::Bill(_) => "صورتحساب تطبیق‌شده",
        ProcurementData::Transfer(_) => "انتقال غیرمالی انبار",
    }
}
fn record_summary(r: &ProcurementRecord, resources: &RegisterResources, ui: &Ui) -> String {
    let c = r.command.context();
    let settings = ui.store.borrow().state.settings.clone();
    let (reference, party, value) = match &r.data {
        ProcurementData::Order(x) => (
            match &r.command {
                ProcurementCommand::Order(c) => c.reference.clone(),
                _ => String::new(),
            },
            x.supplier.name.clone(),
            Some(x.total),
        ),
        ProcurementData::Receive(x) => (
            match &r.command {
                ProcurementCommand::Receive(c) => c.reference.clone(),
                _ => String::new(),
            },
            resource_name(resources, "supplier", &x.supplier_id),
            Some(x.total),
        ),
        ProcurementData::Bill(x) => (
            match &r.command {
                ProcurementCommand::Bill(c) => c.reference.clone(),
                _ => String::new(),
            },
            resource_name(resources, "supplier", &x.supplier_id),
            Some(x.total),
        ),
        ProcurementData::Transfer(x) => (
            resource_name(resources, "item", &x.item_id),
            match &r.command {
                ProcurementCommand::Transfer(c) => format!(
                    "از انبار {} به انبار {}",
                    resource_name(resources, "warehouse", &c.source_warehouse_id),
                    resource_name(resources, "warehouse", &c.target_warehouse_id)
                ),
                _ => String::new(),
            },
            Some(x.value),
        ),
        ProcurementData::Resources(x) => (
            x.parties
                .iter()
                .map(|p| p.name.as_str())
                .chain(x.items.iter().map(|i| i.name.as_str()))
                .chain(x.units.iter().map(|u| u.name.as_str()))
                .chain(x.warehouses.iter().map(|w| w.name.as_str()))
                .collect::<Vec<_>>()
                .join("، "),
            "تغییر تعاریف پایه".into(),
            None,
        ),
    };
    let amount = value
        .map(|v| display_money(&settings, v, MoneyUnit::Rial))
        .unwrap_or_else(|| "بدون اثر مالی".into());
    format!(
        "{} • {} • {} • {} • {}",
        isolate(&reference),
        party,
        financial_date(ui, &c.date),
        amount,
        if r.journal_id.is_some() {
            "دارای سند مالی"
        } else {
            "غیرمالی"
        }
    )
}

#[derive(Clone)]
struct Common {
    base: ProcurementContext,
    date: DateField,
    document_date: DateField,
    reference: gtk::Entry,
    reference_field: gtk::Box,
    reason: gtk::Entry,
}
impl Common {
    fn new(ui: &Rc<Ui>, body: &gtk::Box, company: &str, date: &str, prefix: &str) -> Result<Self> {
        let base = context(ui, company, date)?;
        let date = date_field(ui, "تاریخ مؤثر مدنی", &format!("{prefix}-date"), date);
        let document_date = date_field(
            ui,
            "تاریخ سند طرف",
            &format!("{prefix}-document-date"),
            &base.date,
        );
        let reference = named_entry(
            "شماره یا مرجع سند",
            &format!("{prefix}-reference"),
            "",
            true,
        );
        let reason = named_entry("شرح و دلیل ثبت", &format!("{prefix}-reason"), "", false);
        body.append(&date.widget);
        body.append(&document_date.widget);
        let reference_field = field("شماره یا مرجع سند", &reference);
        body.append(&reference_field);
        body.append(&field("شرح و دلیل ثبت", &reason));
        let provenance = label(&format!(
            "منبع ثابت {} • نسخه ۱ • سال مالی {}",
            isolate(&base.source_id),
            isolate(&base.period_id)
        ));
        provenance.set_selectable(true);
        body.append(&disclosure("شناسهٔ منبع و سال مالی", &provenance));
        Ok(Self {
            base,
            date,
            document_date,
            reference,
            reference_field,
            reason,
        })
    }
    fn collect(&self) -> Result<ProcurementContext> {
        let mut c = self.base.clone();
        c.date = date_value(&self.date)?;
        c.reason = required(&self.reason, "شرح ثبت")?;
        Ok(c)
    }
    fn connect(&self, r: &Rc<Review<ProcurementCommand, ProcurementPreview>>) {
        r.date(&self.date);
        r.date(&self.document_date);
        r.text(&self.reference);
        r.text(&self.reason);
    }
}
fn required(e: &gtk::Entry, title: &str) -> Result<String> {
    let t = e.text().trim().to_string();
    if t.is_empty() {
        e.grab_focus();
        anyhow::bail!("{title} را وارد کنید")
    }
    Ok(t)
}
impl Page {
    fn open(self: &Rc<Self>, kind: u8, id: Option<String>) {
        let Some(ui) = self.ui.upgrade() else { return };
        let result: Result<()> = (|| {
            let company = self.company_id()?;
            let date = date_value(&self.as_of)?;
            let resources = self
                .resources
                .borrow()
                .as_ref()
                .map(|r| r.as_ref().clone())
                .ok_or_else(|| anyhow::anyhow!("منابع آماده نیست"))?;
            match kind {
                0 => order_dialog(&ui, &company, &date, resources, self.run.clone()),
                1 => bill_dialog(
                    &ui,
                    &company,
                    &date,
                    resources,
                    self.reconciliation
                        .borrow()
                        .as_ref()
                        .map(|c| c.receipts.clone())
                        .unwrap_or_default(),
                    self.run.clone(),
                ),
                2 => transfer_dialog(&ui, &company, &date, resources, self.run.clone()),
                3 => {
                    let id = id.ok_or_else(|| anyhow::anyhow!("سفارش انتخاب نشده"))?;
                    let record = self
                        .records
                        .borrow()
                        .iter()
                        .find(|r| r.id == id)
                        .cloned()
                        .ok_or_else(|| anyhow::anyhow!("سفارش پیدا نشد"))?;
                    receive_dialog(&ui, &company, &date, resources, record, self.run.clone())
                }
                _ => resource_dialog(&ui, &company, &date, resources, self.run.clone()),
            }?;
            Ok(())
        })();
        if let Err(e) = result {
            self.status.set_text(&e.to_string());
        }
    }
    fn detail(self: &Rc<Self>, id: &str) {
        let Some(ui) = self.ui.upgrade() else { return };
        if ui.background_busy.get() {
            let weak = Rc::downgrade(self);
            let id = id.to_string();
            glib::timeout_add_local_once(std::time::Duration::from_millis(60), move || {
                if let Some(p) = weak.upgrade() {
                    p.detail(&id);
                }
            });
            return;
        }
        let snapshot = ui.store.borrow().state.clone();
        let record = match snapshot.accounting.procurement_document(id) {
            Ok(r) => r,
            Err(e) => {
                ui.message(Err::<String, _>(e));
                return;
            }
        };
        let company = record.command.context().company_id.clone();
        let id = id.to_string();
        let (dialog, body, error, save) =
            form_dialog(&ui, "منبع ثبت‌شده • فقط خواندنی", "procurement-detail");
        save.set_visible(false);
        body.append(&label("در حال خواندن منبع ثابت، اثر ثبت‌شده و جزئیات…"));
        let weak_dialog = dialog.downgrade();
        let unit = money_unit(&self.unit);
        let weak_page = Rc::downgrade(self);
        let document = id.clone();
        let _=ui.run_background("خواندن منبع ثبت‌شده…",move||{let concise=concise_document(&snapshot.accounting,&company,&id,unit,&snapshot.settings)?;let report=sarv_quote::procurement_export::document_preview(&snapshot.accounting,&company,&id,unit,snapshot.settings.date_display,snapshot.settings.display_digits)?;Ok((report,concise))},move|ui,result|{
            if !weak_dialog.upgrade().is_some_and(|d|d.is_visible()){ui.message(Ok("نمایش منبع لغو شد؛ ثبت‌ها تغییر نکردند."));return;}clear(&body);
            match result{Ok((report,concise))=>{body.append(&heading(&report.title));body.append(&label(&format!("{} • {} • {}",report.company,isolate(&report.as_of),report.status)));body.append(&label("نسخهٔ ثبت‌شده ثابت است؛ بازبینی مجدد یا ثبت دیگری از این پنجره انجام نمی‌شود."));
                report_rows(&body,concise);let trace=gtk::Box::new(gtk::Orientation::Vertical,8);report_rows(&trace,report.lines);body.append(&disclosure("جزئیات کامل منبع، شناسه‌ها و اثر ثبت‌شده",&trace));
                let bar=actions();for pdf in[false,true]{let b=action(if pdf{"خروجی سند PDF"}else{"خروجی سند CSV"},"procurement-export-document");let weak=weak_page.clone();let id=document.clone();b.connect_clicked(move |_|{if let Some(p)=weak.upgrade(){p.export(Some(id.clone()),pdf);}});insert_action(&bar,&b);}body.append(&bar);ui.message(Ok("منبع ثبت‌شده آماده است • فقط خواندنی"));
            },Err(e)=>{show_error(&error,&e);ui.message(Err::<String,_>(e));}}
        });
    }
    fn export(self: &Rc<Self>, document: Option<String>, pdf: bool) {
        let Some(ui) = self.ui.upgrade() else { return };
        let Ok(company) = self.company_id() else {
            return;
        };
        let date = match date_value(&self.as_of) {
            Ok(d) => d,
            Err(e) => {
                self.status.set_text(&e.to_string());
                return;
            }
        };
        let state = ui.store.borrow().state.accounting.clone();
        let settings = ui.store.borrow().state.settings.clone();
        let unit = money_unit(&self.unit);
        let file = gtk::FileDialog::builder()
            .title("ذخیرهٔ گزارش خرید و انبار")
            .initial_name(if pdf {
                "procurement.pdf"
            } else {
                "procurement.csv"
            })
            .build();
        let weak = Rc::downgrade(&ui);
        file.save(Some(&ui.window), gio::Cancellable::NONE, move |result| {
            let Some(ui) = weak.upgrade() else { return };
            let Ok(file) = result else { return };
            let Some(path) = file.path() else { return };
            let _ = ui.run_background(
                "تهیهٔ گزارش از تصویر ثابت…",
                move || {
                    use sarv_quote::procurement_export::*;
                    match (document, pdf) {
                        (Some(id), true) => document_pdf(
                            &state,
                            &company,
                            &id,
                            unit,
                            settings.date_display,
                            settings.display_digits,
                            &path,
                        ),
                        (Some(id), false) => document_csv(
                            &state,
                            &company,
                            &id,
                            unit,
                            settings.date_display,
                            settings.display_digits,
                            &path,
                        ),
                        (None, true) => reconciliation_pdf(
                            &state,
                            &company,
                            &date,
                            unit,
                            settings.date_display,
                            settings.display_digits,
                            &path,
                        ),
                        (None, false) => reconciliation_csv(
                            &state,
                            &company,
                            &date,
                            unit,
                            settings.date_display,
                            settings.display_digits,
                            &path,
                        ),
                    }
                },
                move |ui, result| match result {
                    Ok(()) => ui.message(Ok("گزارش ذخیره شد")),
                    Err(e) => ui.message(Err::<String, _>(e)),
                },
            );
        });
    }
}
fn transfer_dialog(
    ui: &Rc<Ui>,
    company: &str,
    date: &str,
    r: RegisterResources,
    run: RunCommand,
) -> Result<()> {
    let (dialog, body, error, save) =
        form_dialog(ui, "انتقال موجودی بین انبارها", "procurement-transfer");
    let common = Common::new(ui, &body, company, date, "procurement-transfer")?;
    common.reference_field.set_visible(false);
    common.document_date.widget.set_visible(false);
    body.append(&label(
        "انتقال با میانگین موزون جاری منبع؛ مقدار و ارزش کل تغییر نمی‌کند و سند مالی ایجاد نمی‌شود.",
    ));
    let items = resources_choices(&r, "item");
    let warehouses = resources_choices(&r, "warehouse");
    let units = resources_choices(&r, "unit");
    let item = choice("کالا", "procurement-transfer-item", &items);
    let from = choice("انبار مبدأ", "procurement-transfer-from", &warehouses);
    let to = choice("انبار مقصد", "procurement-transfer-to", &warehouses);
    to.set_selected(1);
    let unit = choice("واحد مقدار", "procurement-transfer-unit", &units);
    let qty = named_entry("مقدار انتقال", "procurement-transfer-quantity", "", true);
    for (caption, w) in [
        ("کالا", &item),
        ("انبار مبدأ", &from),
        ("انبار مقصد", &to),
        ("واحد مقدار", &unit),
    ] {
        body.append(&label(caption));
        body.append(w);
    }
    body.append(&field("مقدار در واحد مشخص‌شده (تا سه رقم اعشار)", &qty));
    let c = common.clone();
    let i = item.clone();
    let f = from.clone();
    let t = to.clone();
    let u = unit.clone();
    let q = qty.clone();
    let collect = Rc::new(move || {
        Ok(ProcurementCommand::Transfer(TransferStock {
            context: c.collect()?,
            item_id: selected(&i, &ids(&items), "کالا")?,
            source_warehouse_id: selected(&f, &ids(&warehouses), "انبار مبدأ")?,
            target_warehouse_id: selected(&t, &ids(&warehouses), "انبار مقصد")?,
            unit_id: selected(&u, &ids(&units), "واحد")?,
            quantity_thousandths: quantities(&q.text())?,
        }))
    });
    let review = review(&body, ui, &dialog, &error, &save, run, collect);
    common.connect(&review);
    for d in [&item, &from, &to, &unit] {
        review.choice(d);
    }
    review.text(&qty);
    Ok(())
}
fn receive_dialog(
    ui: &Rc<Ui>,
    company: &str,
    date: &str,
    r: RegisterResources,
    record: ProcurementRecord,
    run: RunCommand,
) -> Result<()> {
    let ProcurementData::Order(order) = record.data else {
        anyhow::bail!("منبع، سفارش خرید نیست")
    };
    let (dialog, body, error, save) =
        form_dialog(ui, "رسید جزئی از سفارش ثابت", "procurement-receive");
    let common = Common::new(ui, &body, company, date, "procurement-receive")?;
    body.append(&label("مقدار هر ردیف در واحد انتخاب‌شده است؛ ردیف خالی دریافت نمی‌شود. ارزش جزئی از هزینهٔ اصلی سفارش تخصیص می‌یابد؛ اضافه‌دریافت و اختلاف مبلغ رد می‌شود."));
    let total = named_entry(
        "جمع ارزش دریافت مستقل",
        "procurement-receive-total",
        "",
        true,
    );
    let money = MoneyFields::new(
        "procurement-receive-money-unit",
        vec![total.clone()],
        &error,
    );
    body.append(&money.choice);
    body.append(&field("جمع مستقل سند در واحد مبلغ انتخاب‌شده", &total));
    let mut fields = Vec::new();
    for line in order.lines {
        body.append(&label(&format!(
            "{} • سفارش پایه {} • خالص {}",
            line.item.name,
            isolate(&quantity_text(line.base_quantity_thousandths)),
            line.net.display(MoneyUnit::Rial)
        )));
        let units = r
            .units
            .iter()
            .filter(|u| {
                u.id == line.item.base_unit_id || line.item.packs.iter().any(|p| p.unit_id == u.id)
            })
            .map(|u| (u.id.clone(), u.name.clone()))
            .collect::<Vec<_>>();
        let warehouses = resources_choices(&r, "warehouse");
        let unit = choice(
            "واحد دریافت",
            &format!("procurement-receive-unit-{}", line.input.id),
            &units,
        );
        if let Some(i) = units.iter().position(|x| x.0 == line.item.base_unit_id) {
            unit.set_selected(i as u32);
        }
        let warehouse = choice(
            "انبار دریافت",
            &format!("procurement-receive-warehouse-{}", line.input.id),
            &warehouses,
        );
        if let Some(i) = warehouses
            .iter()
            .position(|x| x.0 == line.input.warehouse_id)
        {
            warehouse.set_selected(i as u32);
        }
        let qty = named_entry(
            "مقدار این دریافت",
            &format!("procurement-receive-quantity-{}", line.input.id),
            "",
            true,
        );
        body.append(&unit);
        body.append(&warehouse);
        body.append(&field("مقدار در واحد مشخص‌شده (تا سه رقم اعشار)", &qty));
        fields.push((
            Uuid::new_v4().to_string(),
            line.input.id,
            qty,
            unit,
            ids(&units),
            warehouse,
            ids(&warehouses),
        ));
    }
    let c = common.clone();
    let f = fields.clone();
    let t = total.clone();
    let m = money.clone();
    let order_id = record.id;
    let collect = Rc::new(move || {
        let lines = f
            .iter()
            .filter(|x| !x.2.text().trim().is_empty())
            .map(|x| {
                Ok(ReceiveLine {
                    id: x.0.clone(),
                    order_line_id: x.1.clone(),
                    quantity_thousandths: quantities(&x.2.text())?,
                    unit_id: selected(&x.3, &x.4, "واحد")?,
                    warehouse_id: selected(&x.5, &x.6, "انبار")?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        anyhow::ensure!(!lines.is_empty(), "حداقل یک مقدار دریافت را وارد کنید");
        Ok(ProcurementCommand::Receive(ReceiveOrder {
            context: c.collect()?,
            order_id: order_id.clone(),
            reference: required(&c.reference, "مرجع رسید")?,
            document_date: date_value(&c.document_date)?,
            lines,
            supplied_total: money_entry(&t, m.unit())?,
        }))
    });
    let review = review(&body, ui, &dialog, &error, &save, run, collect);
    common.connect(&review);
    review.text(&total);
    review.choice(&money.choice);
    for f in fields {
        review.text(&f.2);
        review.choice(&f.3);
        review.choice(&f.5);
    }
    Ok(())
}
fn bill_dialog(
    ui: &Rc<Ui>,
    company: &str,
    date: &str,
    r: RegisterResources,
    remainders: Vec<ReceiptRemainder>,
    run: RunCommand,
) -> Result<()> {
    anyhow::ensure!(
        !remainders.is_empty(),
        "رسید تطبیق‌نشده وجود ندارد؛ ابتدا رسید انبار را ثبت کنید"
    );
    let (dialog, body, error, save) =
        form_dialog(ui, "صورتحساب تأمین‌کننده و تطبیق جزئی", "procurement-bill");
    let common = Common::new(ui, &body, company, date, "procurement-bill")?;
    let due = date_field(ui, "سررسید صورتحساب", "procurement-bill-due", date);
    body.append(&due.widget);
    let suppliers = resources_choices(&r, "supplier");
    let supplier = choice("تأمین‌کننده", "procurement-bill-supplier", &suppliers);
    body.append(&label("تأمین‌کننده"));
    body.append(&supplier);
    body.append(&label("برای ردیف‌های همین تأمین‌کننده مقدار پایه و ارزش مستقل وارد کنید؛ ردیف خالی تطبیق نمی‌شود. هزینه باید دقیقاً برابر سهم منبع ثابت باشد؛ موجودی دوباره ساخته نمی‌شود."));
    let total = named_entry("جمع مستقل صورتحساب", "procurement-bill-total", "", true);
    let mut money_entries = vec![total.clone()];
    let mut fields = Vec::new();
    for rem in remainders {
        body.append(&label(&format!(
            "{} • {} • {} • ماندهٔ پایه {} / {} ریال • اصل {} / {} ریال",
            isolate(&rem.reference),
            resource_name(&r, "supplier", &rem.supplier_id),
            resource_name(&r, "item", &rem.item_id),
            isolate(&quantity_text(rem.quantity_thousandths)),
            isolate(&rem.value.display(MoneyUnit::Rial)),
            isolate(&quantity_text(rem.original_quantity_thousandths)),
            isolate(&rem.original_value.display(MoneyUnit::Rial))
        )));
        body.append(&label(&format!(
            "دسترسی مالی رسید {}",
            financial_date(ui, &rem.date)
        )));
        if let Some(date) = paper_receipt_date(&ui.store.borrow().state.accounting, &rem.id) {
            body.append(&label(&format!(
                "تاریخ برگهٔ منبع {}",
                financial_date(ui, &date)
            )));
        }
        let qty = named_entry(
            "مقدار پایه برای تطبیق",
            &format!("procurement-bill-quantity-{}", rem.id),
            "",
            true,
        );
        let value = named_entry(
            "ارزش مستقل این ردیف",
            &format!("procurement-bill-value-{}", rem.id),
            "",
            true,
        );
        body.append(&field("مقدار در واحد مشخص‌شده (تا سه رقم اعشار)", &qty));
        body.append(&field("ارزش مستقل این ردیف", &value));
        money_entries.push(value.clone());
        fields.push((Uuid::new_v4().to_string(), rem, qty, value));
    }
    let money = MoneyFields::new("procurement-bill-money-unit", money_entries, &error);
    body.append(&money.choice);
    body.append(&field("جمع مستقل سند در واحد مبلغ انتخاب‌شده", &total));
    let c = common.clone();
    let f = fields.clone();
    let d = due.clone();
    let s = supplier.clone();
    let t = total.clone();
    let m = money.clone();
    let collect = Rc::new(move || {
        let supplier_id = selected(&s, &ids(&suppliers), "تأمین‌کننده")?;
        let lines = f
            .iter()
            .filter(|x| !x.2.text().trim().is_empty() || !x.3.text().trim().is_empty())
            .map(|x| {
                anyhow::ensure!(
                    x.1.supplier_id == supplier_id,
                    "ردیف {} متعلق به تأمین‌کنندهٔ انتخاب‌شده نیست؛ ورودی حفظ شد",
                    x.1.reference
                );
                Ok(BillLine {
                    id: x.0.clone(),
                    receipt_line_id: x.1.id.clone(),
                    quantity_thousandths: quantities(&x.2.text())?,
                    supplied_value: money_entry(&x.3, m.unit())?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        anyhow::ensure!(!lines.is_empty(), "حداقل یک ردیف تطبیق وارد کنید");
        Ok(ProcurementCommand::Bill(MatchBill {
            context: c.collect()?,
            supplier_id,
            reference: required(&c.reference, "شمارهٔ صورتحساب")?,
            document_date: date_value(&c.document_date)?,
            due_date: date_value(&d)?,
            lines,
            supplied_total: money_entry(&t, m.unit())?,
        }))
    });
    let review = review(&body, ui, &dialog, &error, &save, run, collect);
    common.connect(&review);
    review.date(&due);
    review.choice(&supplier);
    review.choice(&money.choice);
    review.text(&total);
    for (_, _, q, v) in fields {
        review.text(&q);
        review.text(&v);
    }
    Ok(())
}
fn resource_dialog(
    ui: &Rc<Ui>,
    company: &str,
    date: &str,
    r: RegisterResources,
    run: RunCommand,
) -> Result<()> {
    let (dialog, body, error, save) =
        form_dialog(ui, "تعریف منبع پس از افتتاحیه", "procurement-resource");
    let common = Common::new(ui, &body, company, date, "procurement-resource")?;
    common.reference_field.set_visible(false);
    common.document_date.widget.set_visible(false);
    body.append(&label("تعریف جدید افزوده می‌شود؛ افتتاحیه و اسناد قبلی تغییر نمی‌کنند. شناسهٔ موجود قابل جایگزینی نیست."));
    let kind = gtk::DropDown::from_strings(&["طرف حساب", "واحد مقدار", "کالا و نسبت بسته", "انبار"]);
    kind.set_widget_name("procurement-resource-kind");
    body.append(&kind);
    let id = named_entry("شناسهٔ ثابت و یکتا", "procurement-resource-id", "", true);
    let name = named_entry("نام", "procurement-resource-name", "", false);
    body.append(&field("شناسهٔ ثابت و یکتا", &id));
    body.append(&field("نام منبع", &name));
    let stack = gtk::Stack::new();
    let party = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let customer = gtk::CheckButton::with_label("مشتری");
    let supplier = gtk::CheckButton::with_label("تأمین‌کننده");
    supplier.set_active(true);
    customer.set_widget_name("procurement-resource-customer");
    supplier.set_widget_name("procurement-resource-supplier");
    party.append(&customer);
    party.append(&supplier);
    let empty = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let item = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let sku = named_entry("کد کالا / SKU", "procurement-resource-sku", "", true);
    let units = resources_choices(&r, "unit");
    let base = choice("واحد پایه", "procurement-resource-base-unit", &units);
    let indivisible = gtk::CheckButton::with_label("واحد پایه تجزیه‌ناپذیر است");
    indivisible.set_active(true);
    let pack = choice(
        "واحد بستهٔ اختیاری",
        "procurement-resource-pack-unit",
        &units,
    );
    let numerator = named_entry(
        "صورت نسبت بسته (خالی: بدون بسته)",
        "procurement-resource-numerator",
        "",
        true,
    );
    let denominator = named_entry(
        "مخرج نسبت بسته",
        "procurement-resource-denominator",
        "1",
        true,
    );
    item.append(&field("کد کالا / SKU", &sku));
    item.append(&label("واحد پایه"));
    item.append(&base);
    item.append(&indivisible);
    item.append(&label(
        "بستهٔ اختیاری: صورت / مخرج واحد پایه؛ نسبت دقیق و مثبت",
    ));
    item.append(&pack);
    item.append(&field("صورت نسبت بسته؛ خالی برای بدون بسته", &numerator));
    item.append(&field("مخرج نسبت بسته", &denominator));
    for (n, b) in [("0", &party), ("1", &empty), ("2", &item)] {
        stack.add_named(b, Some(n));
    }
    let warehouse = gtk::Box::new(gtk::Orientation::Vertical, 6);
    stack.add_named(&warehouse, Some("3"));
    body.append(&stack);
    let stack_change = stack.clone();
    kind.connect_selected_notify(move |d| {
        stack_change.set_visible_child_name(&d.selected().to_string())
    });
    let c = common.clone();
    let k = kind.clone();
    let i = id.clone();
    let n = name.clone();
    let cu = customer.clone();
    let su = supplier.clone();
    let sk = sku.clone();
    let b = base.clone();
    let pa = pack.clone();
    let nu = numerator.clone();
    let de = denominator.clone();
    let ind = indivisible.clone();
    let collect = Rc::new(move || {
        let context = c.collect()?;
        let id = required(&i, "شناسه")?;
        let name = required(&n, "نام")?;
        let company_id = context.company_id.clone();
        let mut resources = RegisterResources {
            context,
            parties: Vec::new(),
            units: Vec::new(),
            items: Vec::new(),
            warehouses: Vec::new(),
        };
        match k.selected() {
            0 => resources.parties.push(CoreParty {
                id,
                company_id,
                name,
                customer: cu.is_active(),
                supplier: su.is_active(),
            }),
            1 => resources.units.push(CoreUnit {
                id,
                company_id,
                name,
            }),
            2 => {
                let packs =
                    if nu.text().trim().is_empty() {
                        Vec::new()
                    } else {
                        vec![PackRatio {
                            unit_id: selected(&pa, &ids(&units), "واحد بسته")?,
                            numerator: nu.text().trim().parse().map_err(|_| {
                                anyhow::anyhow!("صورت نسبت باید عدد صحیح مثبت باشد")
                            })?,
                            denominator: de.text().trim().parse().map_err(|_| {
                                anyhow::anyhow!("مخرج نسبت باید عدد صحیح مثبت باشد")
                            })?,
                        }]
                    };
                resources.items.push(CoreItem {
                    id,
                    company_id,
                    name,
                    sku: required(&sk, "کد کالا")?,
                    base_unit_id: selected(&b, &ids(&units), "واحد پایه")?,
                    indivisible: ind.is_active(),
                    packs,
                });
            }
            _ => resources.warehouses.push(CoreWarehouse {
                id,
                company_id,
                name,
            }),
        }
        Ok(ProcurementCommand::Resources(resources))
    });
    let review = review(&body, ui, &dialog, &error, &save, run, collect);
    common.connect(&review);
    for e in [&id, &name, &sku, &numerator, &denominator] {
        review.text(e);
    }
    for d in [&kind, &base, &pack] {
        review.choice(d);
    }
    for check in [customer, supplier, indivisible] {
        let weak = Rc::downgrade(&review);
        check.connect_toggled(move |_| {
            if let Some(r) = weak.upgrade() {
                r.invalidate();
            }
        });
    }
    Ok(())
}
fn order_dialog(
    ui: &Rc<Ui>,
    company: &str,
    date: &str,
    resources: RegisterResources,
    run: RunCommand,
) -> Result<()> {
    let (dialog, body, error, save) = form_dialog(
        ui,
        "سفارش خرید • منبع ثابت و قیمت منجمد",
        "procurement-order",
    );
    let common = Common::new(ui, &body, company, date, "procurement-order")?;
    let suppliers = resources_choices(&resources, "supplier");
    let supplier = choice("تأمین‌کننده", "procurement-order-supplier", &suppliers);
    body.append(&label("تأمین‌کننده"));
    body.append(&supplier);
    body.append(&label("ورودی و ردیف‌های این پنجره هنوز ثبت نشده‌اند؛ انصراف یا بستن برنامه آن‌ها را کنار می‌گذارد. سفارش غیرمالی است. هر ردیف به کالای هسته، انبار و واحد دقیق نگاشت می‌شود. قیمت فهرست با منبع و تاریخ ثابت حفظ می‌شود؛ تغییر قیمت نیاز به دلیل دارد."));
    let discount = named_entry("تخفیف کل سند", "procurement-order-discount", "0", true);
    let total = named_entry("جمع خالص مستقل سفارش", "procurement-order-total", "", true);
    let money = MoneyFields::new(
        "procurement-order-money-unit",
        vec![discount.clone(), total.clone()],
        &error,
    );
    body.append(&money.choice);
    body.append(&field("تخفیف در واحد مبلغ انتخاب‌شده", &discount));
    body.append(&field("جمع مستقل سند در واحد مبلغ انتخاب‌شده", &total));
    let add = action("افزودن کالا و قیمت", "procurement-order-add-line");
    body.append(&add);
    let row_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
    body.append(&row_box);
    let lines = Rc::new(RefCell::new(Vec::<OrderLine>::new()));
    let editing = Rc::new(Cell::new(false));
    let c = common.clone();
    let s = supplier.clone();
    let d = discount.clone();
    let t = total.clone();
    let m = money.clone();
    let l = lines.clone();
    let dirty = editing.clone();
    let collect = Rc::new(move || {
        anyhow::ensure!(!dirty.get(), "ویرایش ردیف را ذخیره یا صریحاً لغو کنید");
        anyhow::ensure!(!l.borrow().is_empty(), "حداقل یک ردیف کالا اضافه کنید");
        Ok(ProcurementCommand::Order(PurchaseOrder {
            context: c.collect()?,
            supplier_id: selected(&s, &ids(&suppliers), "تأمین‌کننده")?,
            reference: required(&c.reference, "شمارهٔ سفارش")?,
            document_date: date_value(&c.document_date)?,
            lines: l.borrow().clone(),
            document_discount: money_entry(&d, m.unit())?,
            supplied_total: money_entry(&t, m.unit())?,
        }))
    });
    let review = review(&body, ui, &dialog, &error, &save, run, collect);
    common.connect(&review);
    review.choice(&supplier);
    review.choice(&money.choice);
    review.text(&discount);
    review.text(&total);
    let refresh_slot = Rc::new(RefCell::new(None::<Rc<dyn Fn()>>));
    let weak_ui = Rc::downgrade(ui);
    let parent = dialog.downgrade();
    let rows = row_box.clone();
    let ls = lines.clone();
    let res = resources.clone();
    let rev = Rc::downgrade(&review);
    let slot = Rc::downgrade(&refresh_slot);
    let edit = editing.clone();
    let date = date.to_string();
    let refresh: Rc<dyn Fn()> = Rc::new(move || {
        clear(&rows);
        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::None);
        list.add_css_class("boxed-list");
        rows.append(&list);
        for (index, line) in ls.borrow().iter().enumerate() {
            let row = adw::ActionRow::builder()
                .title(resource_name(&res, "item", &line.item_id))
                .subtitle(format!(
                    "{} • مقدار {} • قیمت {} ریال • تخفیف {} ریال",
                    resource_name(&res, "unit", &line.unit_id),
                    isolate(&quantity_text(line.quantity_thousandths)),
                    isolate(&line.unit_price.display(MoneyUnit::Rial)),
                    isolate(&line.line_discount.display(MoneyUnit::Rial))
                ))
                .build();
            row.set_use_markup(false);
            let remove = action("حذف ردیف", "procurement-order-remove-line");
            row.add_suffix(&remove);
            let l = ls.clone();
            let w = rev.clone();
            let sl = slot.clone();
            remove.connect_clicked(move |_| {
                l.borrow_mut().remove(index);
                if let Some(r) = w.upgrade() {
                    r.invalidate();
                }
                if let Some(s) = sl.upgrade()
                    && let Some(f) = s.borrow().as_ref()
                {
                    f();
                }
            });
            let edit_button = action("ویرایش ردیف", "procurement-order-edit-line");
            row.add_suffix(&edit_button);
            let u = weak_ui.clone();
            let pa = parent.clone();
            let l = ls.clone();
            let rs = res.clone();
            let w = rev.clone();
            let sl = slot.clone();
            let e = edit.clone();
            let dt = date.clone();
            edit_button.connect_clicked(move |_| {
                if let (Some(ui), Some(parent), Some(r), Some(slot)) =
                    (u.upgrade(), pa.upgrade(), w.upgrade(), sl.upgrade())
                {
                    line_dialog(
                        &ui,
                        &parent,
                        rs.clone(),
                        &dt,
                        l.clone(),
                        Some(index),
                        r,
                        e.clone(),
                        slot.clone(),
                    );
                }
            });
            row.set_activatable(false);
            row.set_selectable(false);
            list.append(&row);
        }
    });
    *refresh_slot.borrow_mut() = Some(refresh.clone());
    refresh();
    let u = Rc::downgrade(ui);
    let p = dialog.downgrade();
    let r = Rc::downgrade(&review);
    let date = date_value(&common.date)?;
    add.connect_clicked(move |_| {
        if let (Some(ui), Some(parent), Some(review)) = (u.upgrade(), p.upgrade(), r.upgrade()) {
            line_dialog(
                &ui,
                &parent,
                resources.clone(),
                &date,
                lines.clone(),
                None,
                review,
                editing.clone(),
                refresh_slot.clone(),
            );
        }
    });
    Ok(())
}
fn source_part(reference: &str, key: &str) -> Option<String> {
    reference
        .split('|')
        .map(str::trim)
        .find_map(|p| p.strip_prefix(key).map(str::to_string))
}
fn friendly_price_source(reference: &str) -> String {
    reference
        .split('|')
        .filter(|s| !s.trim().starts_with("catalog=") && !s.trim().starts_with("SHA256="))
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" • ")
}
#[allow(clippy::too_many_arguments)]
fn line_dialog(
    ui: &Rc<Ui>,
    parent: &adw::Dialog,
    resources: RegisterResources,
    date: &str,
    lines: Rc<RefCell<Vec<OrderLine>>>,
    index: Option<usize>,
    review: Rc<Review<ProcurementCommand, ProcurementPreview>>,
    editing: Rc<Cell<bool>>,
    refresh: RowRefresh,
) {
    editing.set(true);
    review.invalidate();
    let old = index.and_then(|i| lines.borrow().get(i).cloned());
    let (dialog, body, error, save) =
        form_dialog(ui, "ردیف کالا • نگاشت واحد و قیمت", "procurement-order-line");
    save.set_label(if index.is_some() {
        "اعمال ویرایش ردیف"
    } else {
        "افزودن ردیف به سفارش"
    });
    body.append(&label("انتخاب فهرست قیمت مقدار یا واحد هسته را تغییر نمی‌دهد. تفاوت واحد، کالا یا تاریخ منبع نیاز به دلیل دارد؛ واحد فهرست و واحد ردیف را صریحاً بررسی کنید؛ انصراف، فقط این ویرایش ردیف را کنار می‌گذارد."));
    let items = resources_choices(&resources, "item");
    let units = resources_choices(&resources, "unit");
    let warehouses = resources_choices(&resources, "warehouse");
    let item = choice("کالای هسته", "procurement-line-item", &items);
    let unit = choice("واحد ردیف", "procurement-line-unit", &units);
    let warehouse = choice("انبار مقصد", "procurement-line-warehouse", &warehouses);
    for (caption, d) in [
        ("کالای هسته", &item),
        ("واحد ردیف؛ نگاشت صریح", &unit),
        ("انبار مقصد", &warehouse),
    ] {
        body.append(&label(caption));
        body.append(d);
    }
    let qty = named_entry(
        "مقدار در واحد ردیف",
        "procurement-line-quantity",
        &old.as_ref()
            .map(|l| quantity_text(l.quantity_thousandths))
            .unwrap_or_default(),
        true,
    );
    let price = named_entry(
        "قیمت هر واحد ردیف",
        "procurement-line-price",
        &old.as_ref()
            .map(|l| l.unit_price.display(MoneyUnit::Rial))
            .unwrap_or_default(),
        true,
    );
    let discount = named_entry(
        "تخفیف همین ردیف",
        "procurement-line-discount",
        &old.as_ref()
            .map(|l| l.line_discount.display(MoneyUnit::Rial))
            .unwrap_or_else(|| "0".into()),
        true,
    );
    let money = MoneyFields::new(
        "procurement-line-money-unit",
        vec![price.clone(), discount.clone()],
        &error,
    );
    body.append(&money.choice);
    body.append(&field("مقدار در واحد مشخص‌شده (تا سه رقم اعشار)", &qty));
    body.append(&field("قیمت هر واحد ردیف", &price));
    body.append(&field("تخفیف در واحد مبلغ انتخاب‌شده", &discount));
    let source = named_entry(
        "مرجع قیمت دستی یا منبع فهرست",
        "procurement-line-price-reference",
        old.as_ref()
            .map(|l| l.price_reference.as_str())
            .unwrap_or(""),
        false,
    );
    let price_date = date_field(
        ui,
        "تاریخ قیمت",
        "procurement-line-price-date",
        old.as_ref().map(|l| l.price_date.as_str()).unwrap_or(date),
    );
    let reason = named_entry(
        "دلیل قیمت دستی یا تغییر قیمت منبع",
        "procurement-line-override-reason",
        old.as_ref()
            .and_then(|l| l.override_reason.as_deref())
            .unwrap_or(""),
        false,
    );
    let source_field = field("مرجع قیمت دستی", &source);
    body.append(&source_field);
    let source_summary =
        label("برای انتخاب قیمت ثابت، فهرست را باز کنید؛ قیمت دستی نیاز به مرجع و دلیل دارد.");
    source_summary.set_widget_name("procurement-line-source-summary");
    body.append(&source_summary);
    let provenance = label("");
    provenance.set_selectable(true);
    let provenance_expander = disclosure("جزئیات مرجع قیمت و شناسه‌های منبع", &provenance);
    body.append(&provenance_expander);
    if let Some(l) = old
        .as_ref()
        .filter(|l| l.price_reference.starts_with("فهرست "))
    {
        source_field.set_visible(false);
        source.set_editable(false);
        source_summary.set_text(&friendly_price_source(&l.price_reference));
        provenance.set_text(&l.price_reference);
    }

    body.append(&price_date.widget);
    body.append(&field("دلیل قیمت دستی یا تغییر قیمت منبع", &reason));
    let baseline = Rc::new(RefCell::new(
        old.as_ref()
            .filter(|l| l.price_reference.starts_with("فهرست "))
            .and_then(|l| source_part(&l.price_reference, "قیمت‌ریال="))
            .and_then(|v| v.parse::<i64>().ok())
            .filter(|v| *v >= 0)
            .map(Money::from_rials),
    ));
    let pick = action(
        "انتخاب از کالا و قیمت فهرست",
        "procurement-line-catalog-picker",
    );
    body.append(&pick);
    if let Some(l) = &old {
        for (d, v, id) in [
            (&item, &items, &l.item_id),
            (&unit, &units, &l.unit_id),
            (&warehouse, &warehouses, &l.warehouse_id),
        ] {
            if let Some(i) = v.iter().position(|x| &x.0 == id) {
                d.set_selected(i as u32);
            }
        }
    }
    let uiw = Rc::downgrade(ui);
    let dialogw = dialog.downgrade();
    let p = price.clone();
    let so = source.clone();
    let dt = price_date.clone();
    let bas = baseline.clone();
    let mf = money.clone();
    let target = item.clone();
    let target_unit = unit.clone();
    let core_units = resources.units.clone();
    let summary = source_summary.clone();
    let details = provenance.clone();
    let manual = source_field.clone();
    let core_items = resources.items.clone();
    pick.connect_clicked(move |_| {
        if let (Some(ui),Some(dialog))=(uiw.upgrade(),dialogw.upgrade()) {
            let price=p.clone();let source=so.clone();let date=dt.clone();let baseline=bas.clone();let mf=mf.clone();let item=target.clone();let core_items=core_items.clone();let unit=target_unit.clone();let core_units=core_units.clone();
            let summary=summary.clone();let details=details.clone();let manual=manual.clone();
            catalog_picker(&ui,&dialog,Rc::new(move |selected| {
                let m=Money::from_rials(selected.product.price_rial);*baseline.borrow_mut()=Some(m);price.set_text(&m.display(mf.unit()));
                source.set_text(&format!("فهرست {} | catalog={} | SHA256={} | SKU={} | نام={} | اندازه={} | واحدمنبع={} | قیمت‌ریال={} | تاریخ‌منبع={}",selected.catalog_name,selected.catalog_id,selected.fingerprint,selected.product.code,selected.product.name,selected.product.measure,selected.product.source_unit.label(),selected.product.price_rial,selected.product.source_at));
                source.set_editable(false);summary.set_text(&friendly_price_source(&source.text()));details.set_text(&source.text());manual.set_visible(false);
                date.set_canonical(selected.product.source_at.get(..10).unwrap_or(""))?;
                item.set_selected(core_items.iter().position(|x|x.sku==selected.product.code).map(|i|i as u32).unwrap_or(gtk::INVALID_LIST_POSITION));
                let matched_units=core_units.iter().enumerate().filter(|(_,u)|sarv_quote::domain::normalize(&u.name)==sarv_quote::domain::normalize(&selected.product.measure)).map(|(i,_)|i).collect::<Vec<_>>();
                unit.set_selected(if matched_units.len()==1{matched_units[0]as u32}else{gtk::INVALID_LIST_POSITION});Ok(())
            }));
        }
    });

    let id = old
        .as_ref()
        .map(|l| l.id.clone())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let pa = parent.downgrade();
    let dw = dialog.downgrade();
    let err = error.clone();
    let r = review.clone();
    save.connect_clicked(move |_| {
        let result: Result<()> = (|| {
            let value = money_entry(&price, money.unit())?;
            let item_id = selected(&item, &ids(&items), "کالای هسته")?;
            let unit_id = selected(&unit, &ids(&units), "واحد ردیف")?;
            let mapped_item = resources
                .items
                .iter()
                .find(|x| x.id == item_id)
                .ok_or_else(|| anyhow::anyhow!("کالای هسته پیدا نشد"))?;
            let unit_name = units
                .iter()
                .find(|x| x.0 == unit_id)
                .map(|x| x.1.as_str())
                .unwrap_or("");
            let source_measure = source_part(&source.text(), "اندازه=");
            let source_sku = source_part(&source.text(), "SKU=");
            let source_date = source_part(&source.text(), "تاریخ‌منبع=");
            let picked_date = date_value(&price_date)?;
            let mapping_changed = source_measure.as_ref().is_some_and(|m| {
                sarv_quote::domain::normalize(m) != sarv_quote::domain::normalize(unit_name)
            }) || source_sku
                .as_ref()
                .is_some_and(|sku| sku != &mapped_item.sku)
                || source_date
                    .as_ref()
                    .is_some_and(|d| d.get(..10) != Some(picked_date.as_str()));
            let manual_or_changed = baseline.borrow().is_none_or(|b| b != value) || mapping_changed;
            let why = if manual_or_changed {
                Some(required(
                    &reason,
                    "دلیل قیمت دستی، تغییر قیمت یا تفاوت نگاشت واحد/کالا/تاریخ منبع",
                )?)
            } else {
                let s = reason.text().trim().to_string();
                (!s.is_empty()).then_some(s)
            };
            let provenance = required(&source, "مرجع قیمت")?;
            let provenance = provenance
                .split('|')
                .filter(|s| !s.trim().starts_with("نگاشت‌"))
                .map(str::trim)
                .collect::<Vec<_>>()
                .join(" | ");
            let line = OrderLine {
                id: id.clone(),
                item_id,
                warehouse_id: selected(&warehouse, &ids(&warehouses), "انبار")?,
                unit_id: unit_id.clone(),
                quantity_thousandths: quantities(&qty.text())?,
                unit_price: value,
                line_discount: money_entry(&discount, money.unit())?,
                price_reference: format!(
                    "{provenance} | نگاشت‌کالا={} | نگاشت‌واحد={}",
                    mapped_item.sku, unit_id
                ),
                price_date: picked_date,
                override_reason: why,
            };
            if let Some(i) = index {
                lines.borrow_mut()[i] = line;
            } else {
                lines.borrow_mut().push(line);
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                r.invalidate();
                if let Some(f) = refresh.borrow().as_ref() {
                    f();
                }
                if let Some(d) = dw.upgrade() {
                    d.close();
                }
            }
            Err(e) => show_error(&err, e),
        }
    });
    let uiw = Rc::downgrade(ui);
    dialog.connect_closed(move |_| {
        editing.set(false);
        review.invalidate();
        if let (Some(ui), Some(parent)) = (uiw.upgrade(), pa.upgrade())
            && parent.is_visible()
        {
            *ui.active_dialog.borrow_mut() = Some(parent);
        }
    });
}
fn qa_widget<T: IsA<gtk::Widget> + Clone + glib::object::IsClass>(
    root: &impl IsA<gtk::Widget>,
    name: &str,
) -> Result<T> {
    let root = root.as_ref();
    if root.widget_name() == name
        && let Ok(t) = root.clone().downcast::<T>()
    {
        return Ok(t);
    }
    let mut child = root.first_child();
    while let Some(w) = child {
        child = w.next_sibling();
        if let Ok(t) = qa_widget(&w, name) {
            return Ok(t);
        }
    }
    anyhow::bail!("native widget {name} absent")
}
fn qa_button(root: &impl IsA<gtk::Widget>, caption: &str) -> Option<gtk::Button> {
    let w = root.as_ref();
    if let Ok(b) = w.clone().downcast::<gtk::Button>()
        && b.label().as_deref() == Some(caption)
    {
        return Some(b);
    }
    let mut child = w.first_child();
    while let Some(w) = child {
        child = w.next_sibling();
        if let Some(b) = qa_button(&w, caption) {
            return Some(b);
        }
    }
    None
}
fn qa_dialog(ui: &Ui) -> Result<adw::Dialog> {
    ui.active_dialog
        .borrow()
        .clone()
        .ok_or_else(|| anyhow::anyhow!("native dialog absent"))
}
fn qa_text(d: &adw::Dialog, name: &str, value: &str) -> Result<()> {
    qa_widget::<gtk::Entry>(d, name)?.set_text(value);
    Ok(())
}
fn qa_date(d: &adw::Dialog, name: &str, value: &str) -> Result<()> {
    qa_widget::<gtk::DropDown>(d, &format!("{name}-calendar"))?.set_selected(0);
    qa_text(d, name, value)
}
fn qa_common(d: &adw::Dialog, prefix: &str, reference: &str, date: &str) -> Result<()> {
    qa_date(d, &format!("{prefix}-date"), date)?;
    qa_date(d, &format!("{prefix}-document-date"), date)?;
    qa_text(d, &format!("{prefix}-reference"), reference)?;
    qa_text(
        d,
        &format!("{prefix}-reason"),
        "سند ساختگی برای بررسی مستقل M4",
    )
}
fn qa_page(ui: &Rc<Ui>, kind: PageKind) -> Result<Rc<Page>> {
    PAGES.with(|v| {
        v.borrow()
            .iter()
            .filter_map(Weak::upgrade)
            .find(|p| p.kind == kind && p.ui.upgrade().is_some_and(|u| Rc::ptr_eq(&u, ui)))
            .ok_or_else(|| anyhow::anyhow!("native procurement page absent"))
    })
}
fn qa_snapshot(_ui: &Rc<Ui>, directory: &Path, name: &str) -> Result<()> {
    // QA-only X11 capture. WidgetPaintable can be empty inside an animated Dialog callback.
    // The external window must carry this exact owned process ID; no other window is selected.
    let path = directory.join(name);
    if path.exists() {
        return Ok(());
    }
    thread_local! {static SHOT_BEGAN:RefCell<std::collections::HashMap<String,std::time::Instant>>=RefCell::new(std::collections::HashMap::new());}
    let settled = SHOT_BEGAN.with(|times| {
        times
            .borrow_mut()
            .entry(path.to_string_lossy().into_owned())
            .or_insert_with(std::time::Instant::now)
            .elapsed()
            .as_millis()
            >= 350
    });
    anyhow::ensure!(
        settled,
        "QA capture waits for settled native dialog frames; excluded from performance timing"
    );
    let active = std::process::Command::new("xprop")
        .args(["-root", "_NET_ACTIVE_WINDOW"])
        .output()?;
    let text = String::from_utf8(active.stdout)?;
    let id = text
        .split_whitespace()
        .find(|w| w.starts_with("0x"))
        .ok_or_else(|| anyhow::anyhow!("owned QA window not active"))?;
    let owner = std::process::Command::new("xprop")
        .args(["-id", id, "_NET_WM_PID"])
        .output()?;
    let pid = String::from_utf8(owner.stdout)?
        .split('=')
        .nth(1)
        .and_then(|s| s.trim().parse::<u32>().ok());
    anyhow::ensure!(
        pid == Some(std::process::id()),
        "active window belongs to another process"
    );
    let status = std::process::Command::new("/usr/bin/import")
        .args(["-window", id])
        .arg(path)
        .status()?;
    anyhow::ensure!(status.success(), "native QA window capture failed");
    Ok(())
}

/// Opt-in synthetic instrumentation only. It drives the same form signals and worker adapter.
pub(super) fn schedule(ui: &Rc<Ui>) {
    let Ok(mode) = std::env::var("SARV_PROCUREMENT_SMOKE") else {
        return;
    };
    if !matches!(mode.as_str(), "full" | "stage" | "posted") {
        return;
    }
    let directory = ui.store.borrow().directory.clone();
    let safe =
        directory.starts_with("/tmp") || directory.starts_with("/workspace/sarv-quote/artifacts");
    let fixture = ui
        .store
        .borrow()
        .state
        .accounting
        .companies
        .iter()
        .any(|c| c.name == "F2-core-v1 • شرکت ساختگی");
    if !safe || !fixture || std::env::var("SARV_TEST_INSTANCE").is_err() {
        ui.message(Err::<String, _>(anyhow::anyhow!(
            "procurement smoke requires disposable synthetic M3 fixture"
        )));
        return;
    }
    let weak = Rc::downgrade(ui);
    let started = std::time::Instant::now();
    let mut stage = 0usize;
    let mut resource = 0usize;
    let mut order_id = String::new();
    let mut receipt_a = String::new();
    let mut receipt_b = String::new();
    let mut checks = Vec::<String>::new();
    glib::timeout_add_local(std::time::Duration::from_millis(160), move || {
        let Some(ui) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        if started.elapsed().as_secs() > 150 {
            let _ = std::fs::write(
                directory.join("procurement-native-failure.txt"),
                format!("timeout stage {stage}: {}", ui.status.text()),
            );
            return glib::ControlFlow::Break;
        }
        if ui.background_busy.get() || ui.persistence_busy.get() {
            return glib::ControlFlow::Continue;
        }
        let result: Result<bool> = (|| {
            let page = qa_page(&ui, PageKind::Purchases)?;
            match stage {
                0 => {
                    ui.stack.set_visible_child_name("procurement");
                    page.as_of.set_canonical("2026-03-22")?;
                    page.refresh();
                    if mode == "posted" {
                        stage = 60
                    } else {
                        anyhow::ensure!(
                            ui.store.borrow().state.accounting.procurement.is_empty(),
                            "fixture already contains operational records"
                        );
                        stage = 200
                    }
                }
                200 => {
                    ui.import_catalog(&directory.join("M4-synthetic-prices.csv"));
                    stage = 1;
                }
                1 => {
                    let d = qa_dialog(&ui)?;
                    let Some(b) = qa_button(&d, "منبع و قیمت‌ها را تأیید می‌کنم")
                    else {
                        return Ok(false);
                    };
                    b.emit_clicked();
                    stage = 2
                }
                2 => {
                    if ui.active_dialog.borrow().is_some() {
                        return Ok(false);
                    }
                    page.open(4, None);
                    let d = qa_dialog(&ui)?;
                    qa_common(&d, "procurement-resource", "", "2026-03-22")?;
                    match resource {
                        0 => {
                            qa_text(&d, "procurement-resource-id", "SU-C")?;
                            qa_text(
                                &d,
                                "procurement-resource-name",
                                "تأمین‌کننده جدید پس از افتتاحیه",
                            )?
                        }
                        1 => {
                            qa_widget::<gtk::DropDown>(&d, "procurement-resource-kind")?
                                .set_selected(1);
                            qa_text(&d, "procurement-resource-id", "PALLET")?;
                            qa_text(&d, "procurement-resource-name", "پالت")?
                        }
                        2 => {
                            qa_widget::<gtk::DropDown>(&d, "procurement-resource-kind")?
                                .set_selected(2);
                            qa_text(&d, "procurement-resource-id", "ITEM-B")?;
                            qa_text(&d, "procurement-resource-name", "کالای ب پس از افتتاحیه")?;
                            qa_text(&d, "procurement-resource-sku", "SKU-B")?;
                            qa_widget::<gtk::DropDown>(&d, "procurement-resource-pack-unit")?
                                .set_selected(1);
                            qa_text(&d, "procurement-resource-numerator", "2")?
                        }
                        _ => {
                            qa_widget::<gtk::DropDown>(&d, "procurement-resource-kind")?
                                .set_selected(3);
                            qa_text(&d, "procurement-resource-id", "W3")?;
                            qa_text(&d, "procurement-resource-name", "انبار سوم پس از افتتاحیه")?
                        }
                    }
                    qa_widget::<gtk::Button>(&d, "procurement-preview")?.emit_clicked();
                    stage = 3
                }
                3 => {
                    let d = qa_dialog(&ui)?;
                    let save = qa_widget::<gtk::Button>(&d, "procurement-resource-save")?;
                    anyhow::ensure!(
                        save.is_sensitive(),
                        "resource preview: {}",
                        qa_widget::<gtk::Label>(&d, "accounting-form-error")?.text()
                    );
                    save.emit_clicked();
                    resource += 1;
                    stage = if resource < 4 { 2 } else { 4 }
                }
                4 => {
                    if ui.active_dialog.borrow().is_some() {
                        return Ok(false);
                    }
                    page.open(0, None);
                    let d = qa_dialog(&ui)?;
                    qa_common(&d, "procurement-order", "PO-M4-12", "2026-03-22")?;
                    qa_widget::<gtk::DropDown>(&d, "procurement-order-supplier")?.set_selected(1);
                    qa_text(&d, "procurement-order-total", "180")?;
                    qa_widget::<gtk::Button>(&d, "procurement-order-add-line")?.emit_clicked();
                    let d = qa_dialog(&ui)?;
                    qa_text(&d, "procurement-line-quantity", "12")?;
                    qa_widget::<gtk::Button>(&d, "procurement-line-catalog-picker")?.emit_clicked();
                    stage = 5
                }
                5 => {
                    let d = qa_dialog(&ui)?;
                    let list = qa_widget::<gtk::ListBox>(&d, "procurement-price-results")?;
                    if list.row_at_index(0).is_none() {
                        return Ok(false);
                    }
                    list.select_row(list.row_at_index(0).as_ref());
                    qa_widget::<gtk::Button>(&d, "procurement-price-choose")?.emit_clicked();
                    stage = 6
                }
                6 => {
                    let d = qa_dialog(&ui)?;
                    qa_text(&d, "procurement-line-price", "0.1")?;
                    qa_widget::<gtk::Button>(&d, "procurement-order-line-save")?.emit_clicked();
                    anyhow::ensure!(
                        qa_widget::<gtk::Entry>(&d, "procurement-line-price")?.text() == "0.1"
                            && qa_widget::<gtk::Label>(&d, "accounting-form-error")?.is_visible(),
                        "invalid amount lost"
                    );
                    if qa_snapshot(&ui, &directory, "procurement-retained-price.png").is_err() {
                        return Ok(false);
                    }
                    checks.push("fractional IRR rejected and retained in native line form".into());
                    qa_widget::<gtk::DropDown>(&d, "procurement-line-money-unit")?.set_selected(1);
                    anyhow::ensure!(
                        qa_widget::<gtk::DropDown>(&d, "procurement-line-money-unit")?.selected()
                            == 0
                            && qa_widget::<gtk::Entry>(&d, "procurement-line-price")?.text()
                                == "0.1",
                        "invalid source amount lost on currency switch"
                    );
                    qa_text(&d, "procurement-line-price", "15")?;
                    qa_widget::<gtk::DropDown>(&d, "procurement-line-money-unit")?.set_selected(1);
                    anyhow::ensure!(
                        qa_widget::<gtk::Entry>(&d, "procurement-line-price")?.text() == "1.5",
                        "exact Toman conversion failed"
                    );
                    qa_widget::<gtk::DropDown>(&d, "procurement-line-money-unit")?.set_selected(0);
                    anyhow::ensure!(
                        qa_widget::<gtk::Entry>(&d, "procurement-line-price")?.text() == "15",
                        "Rial conversion changed source price"
                    );
                    checks.push(
                        "Rial15↔Toman1.5 exact; invalidIRR0.1 retained and unit switch refused"
                            .into(),
                    );
                    qa_widget::<gtk::DropDown>(&d, "procurement-line-unit")?.set_selected(1);
                    qa_widget::<gtk::Button>(&d, "procurement-order-line-save")?.emit_clicked();
                    anyhow::ensure!(
                        qa_dialog(&ui)?.widget_name() == "procurement-order-line"
                            && qa_widget::<gtk::Label>(&d, "accounting-form-error")?.is_visible(),
                        "source piece/carton mismatch accepted without reason"
                    );
                    checks.push(
                        "catalog15/piece cannot silently become15/carton; requires override reason"
                            .into(),
                    );
                    qa_widget::<gtk::DropDown>(&d, "procurement-line-unit")?.set_selected(0);
                    qa_text(&d, "procurement-line-price", "30")?;
                    qa_text(
                        &d,
                        "procurement-line-override-reason",
                        "QA explicit override30 from frozen source15",
                    )?;
                    qa_widget::<gtk::Button>(&d, "procurement-order-line-save")?.emit_clicked();
                    stage = 207;
                }
                207 => {
                    let d = qa_dialog(&ui)?;
                    qa_widget::<gtk::Button>(&d, "procurement-order-edit-line")?.emit_clicked();
                    let d = qa_dialog(&ui)?;
                    qa_text(&d, "procurement-line-quantity", "12")?;
                    qa_widget::<gtk::Button>(&d, "procurement-order-line-save")?.emit_clicked();
                    stage = 208;
                }
                208 => {
                    let d = qa_dialog(&ui)?;
                    qa_widget::<gtk::Button>(&d, "procurement-order-edit-line")?.emit_clicked();
                    let d = qa_dialog(&ui)?;
                    anyhow::ensure!(
                        qa_widget::<gtk::Entry>(&d, "procurement-line-override-reason")?.text()
                            == "QA explicit override30 from frozen source15"
                            && qa_widget::<gtk::Entry>(&d, "procurement-line-price")?.text()
                                == "30"
                            && qa_widget::<gtk::Entry>(&d, "procurement-line-price-reference")?
                                .text()
                                .contains("قیمت‌ریال=15"),
                        "quantity-only edit lost source15/override30/reason"
                    );
                    checks.push("quantity-only edit preserves original catalog15, overridden30 and required reason".into());
                    qa_text(&d, "procurement-line-price", "15")?;
                    qa_text(&d, "procurement-line-override-reason", "")?;
                    qa_widget::<gtk::Button>(&d, "procurement-order-line-save")?.emit_clicked();
                    stage = 7;
                }
                7 => {
                    let d = qa_dialog(&ui)?;
                    anyhow::ensure!(
                        d.widget_name() == "procurement-order",
                        "line form did not return to order"
                    );
                    if qa_snapshot(&ui, &directory, "procurement-staged-order.png").is_err() {
                        return Ok(false);
                    }
                    if mode == "stage" {
                        let manifest = serde_json::json!({"ready":true,"phase":"unposted-order-native-forms","resource_records":4,"opening_journals":1,"catalog_count":ui.store.borrow().state.catalogs.len(),"source_snapshot":"catalog ID/name/fingerprint/product/unit/date/IRR frozen through native picker","preview_name":"procurement-preview","post_name":"procurement-order-save","search_name":"procurement-search","note":"no smoke changes after ready; external observer owns preview/post"});
                        std::fs::write(
                            directory.join("procurement-stage-ready.json"),
                            serde_json::to_vec_pretty(&manifest)?,
                        )?;
                        return Ok(true);
                    }
                    qa_widget::<gtk::Button>(&d, "procurement-preview")?.emit_clicked();
                    stage = 8
                }
                8 => {
                    let d = qa_dialog(&ui)?;
                    let save = qa_widget::<gtk::Button>(&d, "procurement-order-save")?;
                    anyhow::ensure!(
                        save.is_sensitive(),
                        "order preview: {}",
                        qa_widget::<gtk::Label>(&d, "accounting-form-error")?.text()
                    );
                    if qa_snapshot(&ui, &directory, "procurement-order-preview.png").is_err() {
                        return Ok(false);
                    }
                    save.emit_clicked();
                    stage = 9
                }
                9 => {
                    if ui.active_dialog.borrow().is_some() {
                        return Ok(false);
                    }
                    order_id = ui
                        .store
                        .borrow()
                        .state
                        .accounting
                        .procurement
                        .records
                        .iter()
                        .find(|r| matches!(r.data, ProcurementData::Order(_)))
                        .ok_or_else(|| anyhow::anyhow!("order missing"))?
                        .id
                        .clone();
                    checks.push("PO12pieces at15, immutable source, no financial journal".into());
                    page.open(3, Some(order_id.clone()));
                    let d = qa_dialog(&ui)?;
                    qa_common(&d, "procurement-receive", "REC-A", "2026-03-23")?;
                    let line_id = match &ui
                        .store
                        .borrow()
                        .state
                        .accounting
                        .procurement
                        .records
                        .iter()
                        .find(|r| r.id == order_id)
                        .unwrap()
                        .data
                    {
                        ProcurementData::Order(o) => o.lines[0].input.id.clone(),
                        _ => unreachable!(),
                    };
                    qa_text(&d, &format!("procurement-receive-quantity-{line_id}"), "5")?;
                    qa_text(&d, "procurement-receive-total", "74")?;
                    qa_widget::<gtk::Button>(&d, "procurement-preview")?.emit_clicked();
                    stage = 10
                }
                10 => {
                    let d = qa_dialog(&ui)?;
                    anyhow::ensure!(
                        !qa_widget::<gtk::Button>(&d, "procurement-receive-save")?.is_sensitive()
                            && qa_widget::<gtk::Entry>(&d, "procurement-receive-total")?.text()
                                == "74",
                        "cost variance not retained"
                    );
                    if qa_snapshot(&ui, &directory, "procurement-receipt-cost-error.png").is_err() {
                        return Ok(false);
                    }
                    checks.push(
                        "partial receipt independent74 mismatches75; retained and no post".into(),
                    );
                    qa_text(&d, "procurement-receive-total", "75")?;
                    qa_widget::<gtk::Button>(&d, "procurement-preview")?.emit_clicked();
                    stage = 11
                }
                11 => {
                    let d = qa_dialog(&ui)?;
                    let b = qa_widget::<gtk::Button>(&d, "procurement-receive-save")?;
                    anyhow::ensure!(b.is_sensitive(), "receipt preview failed");
                    b.emit_clicked();
                    stage = 12
                }
                12 => {
                    if ui.active_dialog.borrow().is_some() {
                        return Ok(false);
                    }
                    receipt_a = ui
                        .store
                        .borrow()
                        .state
                        .accounting
                        .procurement
                        .records
                        .iter()
                        .find_map(|r| match &r.data {
                            ProcurementData::Receive(x) => Some(x.lines[0].id.clone()),
                            _ => None,
                        })
                        .ok_or_else(|| anyhow::anyhow!("receipt A missing"))?;
                    page.open(3, Some(order_id.clone()));
                    let d = qa_dialog(&ui)?;
                    qa_common(&d, "procurement-receive", "REC-B", "2026-03-24")?;
                    let line_id = match &ui
                        .store
                        .borrow()
                        .state
                        .accounting
                        .procurement
                        .records
                        .iter()
                        .find(|r| r.id == order_id)
                        .unwrap()
                        .data
                    {
                        ProcurementData::Order(o) => o.lines[0].input.id.clone(),
                        _ => unreachable!(),
                    };
                    qa_text(&d, &format!("procurement-receive-quantity-{line_id}"), "7")?;
                    qa_text(&d, "procurement-receive-total", "105")?;
                    qa_widget::<gtk::Button>(&d, "procurement-preview")?.emit_clicked();
                    stage = 13
                }
                13 => {
                    let d = qa_dialog(&ui)?;
                    let b = qa_widget::<gtk::Button>(&d, "procurement-receive-save")?;
                    anyhow::ensure!(b.is_sensitive(), "receipt B failed");
                    b.emit_clicked();
                    stage = 14
                }
                14 => {
                    if ui.active_dialog.borrow().is_some() {
                        return Ok(false);
                    }
                    receipt_b = ui
                        .store
                        .borrow()
                        .state
                        .accounting
                        .procurement
                        .records
                        .iter()
                        .rev()
                        .find_map(|r| match &r.data {
                            ProcurementData::Receive(x) => Some(x.lines[0].id.clone()),
                            _ => None,
                        })
                        .ok_or_else(|| anyhow::anyhow!("receipt B missing"))?;
                    page.as_of.set_canonical("2026-03-25")?;
                    page.refresh();
                    stage = 15
                }
                15 => {
                    page.open(1, None);
                    let d = qa_dialog(&ui)?;
                    qa_common(&d, "procurement-bill", "BILL-A", "2026-03-25")?;
                    qa_date(&d, "procurement-bill-due", "2026-04-01")?;
                    qa_widget::<gtk::DropDown>(&d, "procurement-bill-supplier")?.set_selected(1);
                    qa_text(&d, &format!("procurement-bill-quantity-{receipt_a}"), "2")?;
                    qa_text(&d, &format!("procurement-bill-value-{receipt_a}"), "30")?;
                    qa_text(&d, "procurement-bill-total", "30")?;
                    qa_widget::<gtk::Button>(&d, "procurement-preview")?.emit_clicked();
                    stage = 16
                }
                16 => {
                    let d = qa_dialog(&ui)?;
                    let b = qa_widget::<gtk::Button>(&d, "procurement-bill-save")?;
                    anyhow::ensure!(
                        b.is_sensitive(),
                        "bill A failed: {}",
                        qa_widget::<gtk::Label>(&d, "accounting-form-error")?.text()
                    );
                    b.emit_clicked();
                    stage = 17
                }
                17 => {
                    if ui.active_dialog.borrow().is_some() {
                        return Ok(false);
                    }
                    page.as_of.set_canonical("2026-03-26")?;
                    page.refresh();
                    stage = 18
                }
                18 => {
                    page.open(1, None);
                    let d = qa_dialog(&ui)?;
                    qa_common(&d, "procurement-bill", "BILL-B", "2026-03-26")?;
                    qa_date(&d, "procurement-bill-due", "2026-04-02")?;
                    qa_widget::<gtk::DropDown>(&d, "procurement-bill-supplier")?.set_selected(1);
                    for (id, q, v) in [(&receipt_a, "3", "45"), (&receipt_b, "7", "105")] {
                        qa_text(&d, &format!("procurement-bill-quantity-{id}"), q)?;
                        qa_text(&d, &format!("procurement-bill-value-{id}"), v)?;
                    }
                    qa_text(&d, "procurement-bill-total", "150")?;
                    qa_widget::<gtk::Button>(&d, "procurement-preview")?.emit_clicked();
                    stage = 19
                }
                19 => {
                    let d = qa_dialog(&ui)?;
                    let b = qa_widget::<gtk::Button>(&d, "procurement-bill-save")?;
                    anyhow::ensure!(
                        b.is_sensitive(),
                        "bill B failed: {}",
                        qa_widget::<gtk::Label>(&d, "accounting-form-error")?.text()
                    );
                    if qa_snapshot(&ui, &directory, "procurement-bill-preview.png").is_err() {
                        return Ok(false);
                    }
                    b.emit_clicked();
                    stage = 20
                }
                20 => {
                    if ui.active_dialog.borrow().is_some() {
                        return Ok(false);
                    }
                    ui.stack.set_visible_child_name("inventory");
                    let inv = qa_page(&ui, PageKind::Inventory)?;
                    inv.as_of.set_canonical("2026-03-27")?;
                    inv.refresh();
                    stage = 220;
                }
                220 => {
                    let inv = qa_page(&ui, PageKind::Inventory)?;
                    inv.open(2, None);
                    let d = qa_dialog(&ui)?;
                    qa_common(&d, "procurement-transfer", "", "2026-03-27")?;
                    qa_text(&d, "procurement-transfer-quantity", "6")?;
                    qa_widget::<gtk::Button>(&d, "procurement-preview")?.emit_clicked();
                    stage = 21
                }
                21 => {
                    let d = qa_dialog(&ui)?;
                    let b = qa_widget::<gtk::Button>(&d, "procurement-transfer-save")?;
                    anyhow::ensure!(
                        b.is_sensitive(),
                        "transfer failed: {}",
                        qa_widget::<gtk::Label>(&d, "accounting-form-error")?.text()
                    );
                    if qa_snapshot(&ui, &directory, "procurement-transfer-preview.png").is_err() {
                        return Ok(false);
                    }
                    b.emit_clicked();
                    stage = 22
                }
                22 => {
                    if ui.active_dialog.borrow().is_some() {
                        return Ok(false);
                    }
                    checks.push("two partial receipts75/105, two matched bills30/150 and paired nonfinancial transfer70".into());
                    page.as_of.set_canonical("2026-03-27")?;
                    page.refresh();
                    let inv = qa_page(&ui, PageKind::Inventory)?;
                    inv.as_of.set_canonical("2026-03-27")?;
                    inv.refresh();
                    stage = 60
                }
                60 => {
                    let company = page.company_id()?;
                    let state = ui.store.borrow().state.accounting.clone();
                    let dates = ui.store.borrow().state.settings.date_display;
                    let digits = ui.store.borrow().state.settings.display_digits;
                    let csv = directory.join("M4-controls.csv");
                    let pdf = directory.join("M4-controls.pdf");
                    let doc = directory.join("M4-order.pdf");
                    let first = state
                        .procurement
                        .records
                        .iter()
                        .find(|r| matches!(r.data, ProcurementData::Order(_)))
                        .map(|r| r.id.clone());
                    let _ = ui.run_background(
                        "تولید گزارش‌های ساختگی M4…",
                        move || {
                            sarv_quote::procurement_export::reconciliation_csv(
                                &state,
                                &company,
                                "2026-03-27",
                                MoneyUnit::Rial,
                                dates,
                                digits,
                                &csv,
                            )?;
                            sarv_quote::procurement_export::reconciliation_pdf(
                                &state,
                                &company,
                                "2026-03-27",
                                MoneyUnit::Toman,
                                dates,
                                digits,
                                &pdf,
                            )?;
                            if let Some(id) = first {
                                sarv_quote::procurement_export::document_pdf(
                                    &state,
                                    &company,
                                    &id,
                                    MoneyUnit::Rial,
                                    dates,
                                    digits,
                                    &doc,
                                )?;
                            }
                            Ok(())
                        },
                        move |ui, r| ui.message(r.map(|_| "گزارش‌های ساختگی M4 ذخیره شدند")),
                    );
                    stage = 61
                }
                61 => {
                    ui.stack.set_visible_child_name("inventory");
                    qa_page(&ui, PageKind::Inventory)?
                        .tabs
                        .set_visible_child_name("controls");
                    if qa_snapshot(&ui, &directory, "procurement-final-controls.png").is_err() {
                        return Ok(false);
                    }
                    let state = &ui.store.borrow().state.accounting;
                    let manifest = serde_json::json!({"ready":true,"phase":"posted-literal-M4","checks":checks,"source":"native forms; real catalog import and explicit price picker; root worker preview/post", "record_count":state.procurement.records.len(),"journal_count":state.journals.len(),"movement_count":state.procurement.records.iter().map(|r|r.movements.len()).sum::<usize>(),"search_name":"procurement-search","queries":["REC-A","REC-B"],"party_queries":["CU-A","CU-B"],"item_queries":["SKU-A","SKU-B"],"reports":["M4-controls.csv","M4-controls.pdf","M4-order.pdf"],"note":"160ms smoke observation polling is not worker latency; process remains alive for independent keyboard/RSS"});
                    std::fs::write(
                        directory.join("procurement-native-verification.json"),
                        serde_json::to_vec_pretty(&manifest)?,
                    )?;
                    return Ok(true);
                }
                _ => (),
            };
            Ok(false)
        })();
        match result {
            Ok(true) => glib::ControlFlow::Break,
            Ok(false) => glib::ControlFlow::Continue,
            Err(e) => {
                let _ = std::fs::write(
                    directory.join("procurement-native-failure.txt"),
                    format!("stage {stage}: {e:#}"),
                );
                glib::ControlFlow::Break
            }
        }
    });
}
fn concise_document(
    state: &AccountingState,
    company: &str,
    id: &str,
    unit: MoneyUnit,
    settings: &Settings,
) -> Result<Vec<String>> {
    let record = state.procurement_document(id)?;
    let resources = state.procurement_resources(company)?;
    let c = record.command.context();
    let money = |m| display_money(settings, m, unit);
    let date = |d: &str| {
        isolate(&managers::display_date(
            d,
            settings.display_digits,
            settings.date_display,
        ))
    };
    let mut out = vec![format!("تاریخ مؤثر مدنی {} • {}", date(&c.date), c.reason)];
    match (&record.command, &record.data) {
        (ProcurementCommand::Order(c), ProcurementData::Order(o)) => {
            out.push(format!(
                "سفارش {} • {} • تاریخ سند {} • جمع {}",
                isolate(&c.reference),
                o.supplier.name,
                date(&c.document_date),
                money(o.total)
            ));
            for l in &o.lines {
                out.push(format!(
                    "{} • {} • مقدار {} {} • پایه {} • قیمت {} • خالص {}",
                    l.item.sku,
                    l.item.name,
                    isolate(&quantity_text(l.input.quantity_thousandths)),
                    resource_name(&resources, "unit", &l.input.unit_id),
                    isolate(&quantity_text(l.base_quantity_thousandths)),
                    money(l.input.unit_price),
                    money(l.net)
                ));
                out.push(format!(
                    "انبار {} • منبع قیمت {} • دلیل تغییر {}",
                    resource_name(&resources, "warehouse", &l.input.warehouse_id),
                    friendly_price_source(&l.input.price_reference),
                    l.input.override_reason.as_deref().unwrap_or("بدون تغییر")
                ));
            }
        }
        (ProcurementCommand::Receive(c), ProcurementData::Receive(r)) => {
            let reference = state
                .procurement_document(&r.order_id)
                .ok()
                .and_then(|r| match &r.command {
                    ProcurementCommand::Order(o) => Some(o.reference.as_str()),
                    _ => None,
                })
                .unwrap_or("");
            out.push(format!(
                "رسید {} • سفارش {} • {} • تاریخ سند {} • جمع {}",
                isolate(&c.reference),
                isolate(reference),
                resource_name(&resources, "supplier", &r.supplier_id),
                date(&c.document_date),
                money(r.total)
            ));
            for l in &r.lines {
                out.push(format!(
                    "{} • {} • مقدار پایه {} • ارزش {}",
                    resource_name(&resources, "item", &l.item_id),
                    resource_name(&resources, "warehouse", &l.warehouse_id),
                    isolate(&quantity_text(l.quantity_thousandths)),
                    money(l.value)
                ));
            }
        }
        (ProcurementCommand::Bill(c), ProcurementData::Bill(b)) => {
            out.push(format!(
                "صورتحساب {} • {} • تاریخ سند {} • سررسید {} • جمع {}",
                isolate(&c.reference),
                resource_name(&resources, "supplier", &b.supplier_id),
                date(&c.document_date),
                date(&c.due_date),
                money(b.total)
            ));
            for l in &b.lines {
                out.push(format!(
                    "تطبیق رسید {} • مقدار پایه {} • ارزش {}",
                    isolate(&receipt_reference(state, &l.receipt_line_id)),
                    isolate(&quantity_text(l.quantity_thousandths)),
                    money(l.value)
                ));
            }
        }
        (ProcurementCommand::Transfer(c), ProcurementData::Transfer(t)) => {
            out.push(format!(
                "انتقال {} • مبدأ {} • مقصد {} • مقدار پایه {} • ارزش {} • بدون سند مالی",
                resource_name(&resources, "item", &c.item_id),
                resource_name(&resources, "warehouse", &c.source_warehouse_id),
                resource_name(&resources, "warehouse", &c.target_warehouse_id),
                isolate(&quantity_text(t.quantity_thousandths)),
                money(t.value)
            ));
        }
        (_, ProcurementData::Resources(r)) => {
            for p in &r.parties {
                out.push(format!(
                    "طرف {} • مشتری {} • تأمین‌کننده {}",
                    p.name,
                    yes_no(p.customer),
                    yes_no(p.supplier)
                ));
            }
            for i in &r.items {
                out.push(format!("کالا {} • {}", i.sku, i.name));
            }
            for u in &r.units {
                out.push(format!("واحد {}", u.name));
            }
            for w in &r.warehouses {
                out.push(format!("انبار {}", w.name));
            }
        }
        _ => (),
    }
    for m in &record.movements {
        out.push(format!(
            "انبار {} • مقدار قبل {} • مقدار بعد {} • ارزش قبل {} • ارزش بعد {}",
            resource_name(&resources, "warehouse", &m.warehouse_id),
            isolate(&quantity_text(m.before.quantity_thousandths)),
            isolate(&quantity_text(m.after.quantity_thousandths)),
            money(m.before.value),
            money(m.after.value)
        ));
    }
    if let Some(journal) = record
        .journal_id
        .as_ref()
        .and_then(|id| state.journals.iter().find(|j| &j.id == id))
    {
        for l in &journal.lines {
            out.push(format!(
                "{} • {} • بدهکار {} • بستانکار {}",
                l.account.code,
                l.account.name,
                money(l.debit),
                money(l.credit)
            ));
        }
    }
    Ok(out)
}
fn receipt_reference(state: &AccountingState, id: &str) -> String {
    for r in &state.procurement.records {
        if let (ProcurementData::Receive(x), ProcurementCommand::Receive(c)) = (&r.data, &r.command)
            && x.lines.iter().any(|l| l.id == id)
        {
            return c.reference.clone();
        }
    }
    state
        .core
        .openings
        .iter()
        .find_map(|o| {
            o.command
                .grni
                .iter()
                .find(|r| r.id == id)
                .map(|r| r.receipt_reference.clone())
        })
        .unwrap_or_else(|| id.into())
}
