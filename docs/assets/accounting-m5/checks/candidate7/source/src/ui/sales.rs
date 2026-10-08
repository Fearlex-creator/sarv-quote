//! Native goods sales workspace; commands use the shared worker boundary.
use super::accounting::{DateField, date_field, date_value, form_dialog};
use super::core::RunCommand;
use super::*;
use sarv_quote::accounting::{AccountingState, Money, MoneyUnit};
use sarv_quote::procurement::{ProcurementContext, RegisterResources};
use sarv_quote::sales::*;
use std::{rc::Weak, sync::Arc};
use uuid::Uuid;
const ACTOR: &str = "local-single-user";
const PAGE_SIZE: usize = 25;
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
thread_local! {static SALES_CATALOG_CACHE:Rc<CatalogCache>=Rc::new(CatalogCache::default());}
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
    let cache = SALES_CATALOG_CACHE.with(Clone::clone);
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
        "آماده‌سازی جست‌وجوی قیمت سفارش فروش…",
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
                    ui.message(Ok("جست‌وجوی قیمت سفارش فروش آماده است."));
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
            "فهرست قیمت تأییدشده‌ای موجود نیست؛ قیمت فروش را با منبع و دلیل دستی وارد کنید"
        )));
        return;
    }
    let dialog = adw::Dialog::builder()
        .title("کالا و قیمت برای سفارش فروش")
        .content_width(660)
        .content_height(680)
        .build();
    dialog.set_widget_name("sales-price-picker");
    let root = column();
    root.set_margin_top(16);
    root.set_margin_bottom(16);
    root.set_margin_start(16);
    root.set_margin_end(16);
    root.append(&label("این انتخاب فهرست یا پیشنهاد فروش را تغییر نمی‌دهد. کالا و واحد هسته را در سطر سفارش صریحاً تطبیق دهید؛ قیمت به ازای همان واحد منبع است."));
    let sources = choice("فهرست قیمت", "sales-price-catalog", &catalogs);
    root.append(&field("فهرست قیمت و تاریخ منبع", &sources));
    let search = search_entry("کد، نام یا نام مستعار کالا", "sales-price-search");
    root.append(&search);
    let hint = label("در حال آماده‌سازی فهرست…");
    root.append(&hint);
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.set_activate_on_single_click(false);
    list.set_widget_name("sales-price-results");
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
    choose.set_widget_name("sales-price-choose");
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
        if !expanded
            && let Some(window) = button.root().and_downcast::<gtk::Window>()
            && gtk::prelude::GtkWindowExt::focus(&window)
                .is_some_and(|focus| focus == child || focus.is_ancestor(&child))
        {
            button.grab_focus();
        }
        child.set_visible(expanded);
        button.update_state(&[gtk::accessible::State::Expanded(Some(expanded))]);
    });
    box_
}
fn quantities(value: &str) -> Result<i64> {
    sarv_quote::domain::quantity(value.trim())
}
fn quantity_text(value: i64) -> String {
    format!("{}.{:03}", value / 1000, value % 1000)
}
fn resources_choices(r: &RegisterResources, kind: &str) -> Vec<(String, String)> {
    match kind {
        "customer" => r
            .parties
            .iter()
            .filter(|x| x.customer)
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

type Collect<C> = Rc<dyn Fn() -> Result<C>>;
type Validate<C, P> = Arc<dyn Fn(&AccountingState, &C) -> Result<P> + Send + Sync>;
type Post<C> = Rc<dyn Fn(C) -> super::core::Mutation>;
type Render<P> = Rc<dyn Fn(&gtk::Box, &P, &Settings)>;
trait ReviewedCommand {
    fn operation(&self) -> &'static str;
}
impl ReviewedCommand for SalesCommand {
    fn operation(&self) -> &'static str {
        self.event()
    }
}
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
        let monitor = SalesPhase::start(&ui, "preview", false, command.operation());
        let completion = monitor.clone();
        let state = ui.store.borrow().state.accounting.clone();
        let validated = command.clone();
        let validate = self.validate.clone();
        let weak = Rc::downgrade(self);

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
                    let _ = SalesPhase::start(&ui, "post", true, command.operation());
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

fn required(e: &gtk::Entry, title: &str) -> Result<String> {
    let t = e.text().trim().to_string();
    if t.is_empty() {
        e.grab_focus();
        anyhow::bail!("{title} را وارد کنید")
    }
    Ok(t)
}
fn review(
    body: &gtk::Box,
    ui: &Rc<Ui>,
    dialog: &adw::Dialog,
    error: &gtk::Label,
    save: &gtk::Button,
    run: RunCommand,
    collect: Collect<SalesCommand>,
) -> Rc<Review<SalesCommand, SalesPreview>> {
    let check = action("بازبینی مقدار و اثر سند", "sales-preview");
    body.append(&check);
    let output = column();
    output.set_widget_name("sales-preview-output");
    body.append(&output);
    save.set_label("تأیید و ثبت نسخهٔ بازبینی‌شده");
    let footer_total = label("");
    footer_total.set_widget_name("sales-post-total");
    footer_total.set_visible(false);
    // Stacked totals remain readable in narrow native forms.
    if let Some(parent) = save.parent().and_then(|w| w.downcast::<gtk::Box>().ok()) {
        parent.set_orientation(gtk::Orientation::Vertical);
        parent.append(&footer_total);
    }
    let display_ui = Rc::downgrade(ui);
    let shown = footer_total.clone();
    let r = Rc::new(Review {
        ui: Rc::downgrade(ui),
        dialog: dialog.clone(),
        error: error.clone(),
        save: save.clone(),
        output,
        footer_total,
        collect,
        validate: Arc::new(|s, c| s.preview_sales(c)),
        post: Rc::new(|c| Box::new(move |s| s.post_sales_command(c).map(|_| ()))),
        render: Rc::new(move |body, p, settings| {
            if let Some(ui) = display_ui.upgrade() {
                render_preview(body, p, settings, &ui.store.borrow().state.accounting);
                shown.set_text(&format!(
                    "{} • {}",
                    display_money(settings, p.total, MoneyUnit::Rial),
                    if p.nonfinancial {
                        "غیرمالی"
                    } else {
                        "سند مالی"
                    }
                ));
                shown.set_visible(true);
            }
        }),
        run,
        verified: RefCell::new(None),
        revision: Cell::new(0),
        message: "سند فروش و اثر تأییدشده ذخیره شد.",
    });
    r.connect(&check);
    let weak = Rc::downgrade(&r);
    let keys = gtk::EventControllerKey::new();
    keys.connect_key_pressed(move |_, key, _, mods| {
        if mods.contains(gtk::gdk::ModifierType::CONTROL_MASK)
            && matches!(key, gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter)
        {
            if let Some(r) = weak.upgrade() {
                r.commit();
            }
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    dialog.add_controller(keys);
    r
}
fn resource_name(r: &RegisterResources, kind: &str, id: &str) -> String {
    match kind {
        "item" => r
            .items
            .iter()
            .find(|x| x.id == id)
            .map(|x| format!("{} • {}", isolate(&x.sku), x.name)),
        "unit" => r.units.iter().find(|x| x.id == id).map(|x| x.name.clone()),
        "party" => r
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

use sarv_quote::sales_draft::{RawSalesDraft, RawSalesLine};
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::mpsc,
    thread,
    time::Duration,
};

/// One detached transaction for raw drafts and their atomic submission. GTK input
/// stays owned by the editor; an older receipt updates only the durable revision.
fn run_state<T: Send + 'static>(
    ui: &Rc<Ui>,
    dialog: &adw::Dialog,
    error: &gtk::Label,
    save: &gtk::Button,
    freeze: bool,
    work: impl FnOnce(&mut State) -> Result<T> + Send + 'static,
    complete: impl FnOnce(&Rc<Ui>, T) + 'static,
) {
    if ui.persistence_busy.get() || ui.background_busy.get() {
        show_error(
            error,
            "ذخیره یا بررسی دیگری در حال انجام است؛ ورودی حفظ شد. دوباره تلاش کنید.",
        );
        return;
    }
    let reserved = ui.store.borrow_mut().begin_detached();
    let (generation, mut candidate) = match reserved {
        Ok(x) => x,
        Err(e) => {
            show_error(error, e);
            return;
        }
    };
    let directory = ui.store.borrow().directory.clone();
    let _monitor = SalesPhase::start(
        ui,
        "post",
        true,
        if freeze { "order" } else { "draft-save" },
    );
    ui.persistence_busy.set(true);
    if freeze && let Some(content) = dialog.child() {
        content.set_sensitive(false);
    }
    save.set_sensitive(false);
    dialog.set_can_close(false);
    let (sender, receiver) = mpsc::sync_channel(1);
    let spawned = thread::Builder::new()
        .name("sarv-sales-draft".into())
        .spawn(move || {
            let result = catch_unwind(AssertUnwindSafe(|| {
                let value = work(&mut candidate)?;
                let prepared = Store::prepare_candidate(generation, candidate)?;
                let receipt = Store::commit_detached(directory, prepared)?;
                Ok((receipt, value))
            }))
            .unwrap_or_else(|_| Err(anyhow::anyhow!("ذخیره قطع شد؛ ورودی حفظ شده است")));
            let _ = sender.send(result);
        });
    if let Err(e) = spawned {
        let result = ui.store.borrow_mut().finish_detached(Err(e.into()));
        ui.persistence_busy.set(false);
        if freeze && let Some(content) = dialog.child() {
            content.set_sensitive(true);
        }
        save.set_sensitive(true);
        dialog.set_can_close(true);
        if let Err(e) = result {
            show_error(error, e);
        }
        return;
    }
    let ui = ui.clone();
    let dialog = dialog.clone();
    let error = error.clone();
    let save = save.clone();
    let mut complete = Some(complete);
    glib::timeout_add_local(Duration::from_millis(16), move || {
        let result = match receiver.try_recv() {
            Ok(r) => r,
            Err(mpsc::TryRecvError::Empty) => return glib::ControlFlow::Continue,
            Err(_) => Err(anyhow::anyhow!("ذخیره قطع شد؛ دوباره تلاش کنید")),
        };
        let (receipt, value) = match result {
            Ok((r, v)) => (Ok(r), Some(v)),
            Err(e) => (Err(e), None),
        };
        let result = ui.store.borrow_mut().finish_detached(receipt);
        ui.persistence_busy.set(false);
        if freeze && let Some(content) = dialog.child() {
            content.set_sensitive(true);
        }
        save.set_sensitive(true);
        dialog.set_can_close(true);
        match result {
            Ok(()) => {
                if let (Some(f), Some(value)) = (complete.take(), value) {
                    f(&ui, value);
                }
                refresh_all(&ui);
                ui.refresh_lists();
            }
            Err(e) => {
                if has_unsaved(&ui) {
                    dialog.set_can_close(false);
                }
                show_error(&error, format!("{e:#}"));
            }
        };
        glib::ControlFlow::Break
    });
}
thread_local! {static PAGES:RefCell<Vec<Weak<Page>>>=const{RefCell::new(Vec::new())};}
struct SalesCard {
    card: gtk::Box,
    title: gtk::Label,
    summary: gtk::Label,
    open: gtk::Button,
    current: RefCell<Option<(String, bool)>>,
}
struct SalesLineCard {
    card: gtk::Box,
    summary: gtk::Label,
    actions: Vec<gtk::Button>,
    current_id: RefCell<Option<String>>,
}
fn update_text(label: &gtk::Label, text: &str) {
    if label.text().as_str() != text {
        label.set_text(text);
    }
}
fn update_button(button: &gtk::Button, text: &str) {
    if button.label().as_deref() != Some(text) {
        button.set_label(text);
    }
}
fn update_name(widget: &impl IsA<gtk::Widget>, name: &str) {
    if widget.as_ref().widget_name().as_str() != name {
        widget.as_ref().set_widget_name(name);
    }
}
struct Page {
    ui: Weak<Ui>,
    root: glib::WeakRef<gtk::Box>,
    company: gtk::DropDown,
    company_ids: RefCell<Vec<String>>,
    search: gtk::SearchEntry,
    as_of: DateField,
    unit: gtk::DropDown,
    cards: Vec<Rc<SalesCard>>,
    empty: gtk::Label,
    previous: gtk::Button,
    next: gtk::Button,
    status: gtk::Label,
    offset: Cell<usize>,
    refreshing: Cell<bool>,
    pending: Cell<bool>,
    revision: Cell<u64>,
    snapshot: RefCell<Option<Arc<PageSnapshot>>>,
}
struct PageSnapshot {
    state: State,
    resources: RegisterResources,
    rows: Vec<(String, String, String, bool)>,
    index: SearchIndex,
}
pub(super) fn build(ui: &Rc<Ui>) -> gtk::Widget {
    let root = column();
    root.add_css_class("page");
    root.set_widget_name("sales-workspace");
    root.append(&heading("فروش‌ها • سفارش، تحویل و برگشت"));
    root.append(&label("تحویل و صورتحساب هم‌زمان ثبت می‌شوند • ریال پایه • مالیات غیرفعال • وصول و پرداخت در مرحلهٔ بعد"));
    let company = choice("شرکت", "sales-company", &[]);
    root.append(&field("شرکت", &company));
    let as_of = date_field(
        ui,
        "پایان روز گزارش",
        "sales-as-of",
        &chrono::Local::now().date_naive().to_string(),
    );
    root.append(&disclosure("تاریخ و واحد نمایش", &as_of.widget));
    let unit = gtk::DropDown::from_strings(&["نمایش ریال • IRR", "نمایش تومان • ۱۰ ریال"]);
    accessible_name(&unit, "واحد نمایش فروش");
    unit.set_widget_name("sales-display-unit");
    root.append(&unit);
    let flow = column();
    let new = action("سفارش فروش تازه", "sales-new");
    new.add_css_class("suggested-action");
    flow.append(&new);
    let quote = action("از پیشنهاد تأییدشده", "sales-from-quote");
    flow.append(&quote);
    let refresh = action("به‌روزرسانی", "sales-refresh");
    flow.append(&refresh);
    root.append(&flow);
    let search = search_entry("جست‌وجوی شماره، مشتری، کالا یا پیش‌نویس", "sales-search");
    root.append(&search);
    let status = label("در حال آماده‌سازی…");
    status.set_widget_name("sales-status");
    root.append(&status);
    let body = column();
    let cards = (0..PAGE_SIZE)
        .map(|_| {
            let card = column();
            let title = heading("");
            let summary = label("");
            let open = button("جزئیات و جریان کار");
            card.append(&title);
            card.append(&summary);
            card.append(&open);
            card.add_css_class("card");
            card.set_margin_bottom(6);
            card.set_visible(false);
            open.set_sensitive(false);
            body.append(&card);
            Rc::new(SalesCard {
                card,
                title,
                summary,
                open,
                current: RefCell::new(None),
            })
        })
        .collect::<Vec<_>>();
    let empty = label("سندی با این جست‌وجو وجود ندارد.");
    empty.set_visible(false);
    body.append(&empty);
    let pager = row();
    let previous = button("صفحهٔ قبل");
    let next = button("صفحهٔ بعد");
    previous.set_sensitive(false);
    next.set_sensitive(false);
    pager.append(&previous);
    pager.append(&next);
    body.append(&pager);
    root.append(&scroll(&body));
    let page = Rc::new(Page {
        ui: Rc::downgrade(ui),
        root: root.downgrade(),
        company: company.clone(),
        company_ids: RefCell::new(Vec::new()),
        search: search.clone(),
        as_of: as_of.clone(),
        unit: unit.clone(),
        cards,
        empty,
        previous,
        next,
        status,
        offset: Cell::new(0),
        refreshing: Cell::new(false),
        pending: Cell::new(false),
        revision: Cell::new(0),
        snapshot: RefCell::new(None),
    });
    for slot in &page.cards {
        let current = Rc::downgrade(slot);
        let weak = Rc::downgrade(&page);
        slot.open.connect_clicked(move |button| {
            let Some(page) = weak.upgrade() else { return };
            let Some(current) = current.upgrade() else {
                return;
            };
            if page.current_ui().is_none() || !current.card.is_visible() || !button.is_sensitive() {
                return;
            }
            let Some((id, draft)) = current.current.borrow().clone() else {
                return;
            };
            if draft {
                page.edit_draft(&id);
            } else if let Some(ui) = page.current_ui() {
                open_document(&ui, &id);
            }
        });
    }
    for (button, forward) in [(&page.previous, false), (&page.next, true)] {
        let weak = Rc::downgrade(&page);
        button.connect_clicked(move |button| {
            if !button.is_sensitive() {
                return;
            }
            if let Some(page) = weak.upgrade()
                && page.current_ui().is_some()
            {
                page.offset.set(if forward {
                    page.offset.get() + PAGE_SIZE
                } else {
                    page.offset.get().saturating_sub(PAGE_SIZE)
                });
                page.render();
            }
        });
    }
    PAGES.with(|p| p.borrow_mut().push(Rc::downgrade(&page)));
    let keeper = page.clone();
    root.connect_destroy(move |_| {
        let _ = &keeper;
    });
    let weak = Rc::downgrade(&page);
    new.connect_clicked(move |_| {
        if let Some(p) = weak.upgrade() {
            p.new_order();
        }
    });
    let weak = Rc::downgrade(&page);
    quote.connect_clicked(move |_| {
        if let Some(p) = weak.upgrade() {
            p.from_quote();
        }
    });
    let weak = Rc::downgrade(&page);
    refresh.connect_clicked(move |_| {
        if let Some(p) = weak.upgrade() {
            p.refresh();
        }
    });
    let weak = Rc::downgrade(&page);
    company.connect_selected_notify(move |_| {
        if let Some(p) = weak.upgrade()
            && !p.refreshing.get()
        {
            p.offset.set(0);
            p.refresh();
        }
    });
    let weak = Rc::downgrade(&page);
    search.connect_search_changed(move |_| {
        if let Some(p) = weak.upgrade() {
            p.offset.set(0);
            p.render();
        }
    });
    let weak = Rc::downgrade(&page);
    unit.connect_selected_notify(move |_| {
        if let Some(p) = weak.upgrade() {
            p.render();
        }
    });
    let weak = Rc::downgrade(&page);
    as_of.entry.connect_changed(move |_| {
        if let Some(p) = weak.upgrade()
            && !p.refreshing.get()
        {
            p.refresh();
        }
    });
    let weak = Rc::downgrade(&page);
    as_of.calendar.connect_selected_notify(move |_| {
        if let Some(p) = weak.upgrade()
            && !p.refreshing.get()
        {
            p.refresh();
        }
    });
    let weak = Rc::downgrade(&page);
    glib::idle_add_local_once(move || {
        if let Some(p) = weak.upgrade() {
            p.refresh();
        }
    });
    root.upcast()
}
pub(super) fn refresh_all(ui: &Rc<Ui>) {
    let current = PAGES.with(|pages| {
        let mut pages = pages.borrow_mut();
        pages.retain(|weak| weak.upgrade().is_some_and(|p| p.current_ui().is_some()));
        pages
            .iter()
            .filter_map(Weak::upgrade)
            .filter(|p| p.current_ui().is_some_and(|owner| Rc::ptr_eq(&owner, ui)))
            .collect::<Vec<_>>()
    });
    for page in current {
        page.refresh();
    }
}
fn page_for(ui: &Rc<Ui>) -> Option<Rc<Page>> {
    PAGES.with(|pages| {
        let mut pages = pages.borrow_mut();
        pages.retain(|weak| weak.upgrade().is_some_and(|p| p.current_ui().is_some()));
        pages
            .iter()
            .filter_map(Weak::upgrade)
            .find(|p| p.current_ui().is_some_and(|owner| Rc::ptr_eq(&owner, ui)))
    })
}
pub(super) fn new_order(ui: &Rc<Ui>) {
    if let Some(p) = page_for(ui) {
        p.new_order();
    }
}
impl Page {
    fn current_ui(&self) -> Option<Rc<Ui>> {
        let ui = self.ui.upgrade()?;
        let root = self.root.upgrade()?;
        (ui.stack.child_by_name("sales").as_ref() == Some(root.upcast_ref::<gtk::Widget>()))
            .then_some(ui)
    }
    fn preserve_focus_before_hiding(&self, widget: &impl IsA<gtk::Widget>) {
        let Some(ui) = self.current_ui() else { return };
        if gtk::prelude::GtkWindowExt::focus(&ui.window)
            .is_some_and(|focus| focus == *widget.as_ref() || focus.is_ancestor(widget))
            && self.search.is_mapped()
        {
            self.search.grab_focus();
        }
    }
    fn invalidate_rows(&self) {
        self.snapshot.borrow_mut().take();
        for slot in &self.cards {
            self.preserve_focus_before_hiding(&slot.card);
            slot.current.borrow_mut().take();
            slot.open.set_sensitive(false);
            slot.card.set_visible(false);
        }
        self.empty.set_visible(false);
        self.preserve_focus_before_hiding(&self.previous);
        self.preserve_focus_before_hiding(&self.next);
        self.previous.set_sensitive(false);
        self.next.set_sensitive(false);
    }
    fn company_id(&self) -> Result<String> {
        selected(&self.company, &self.company_ids.borrow(), "شرکت")
    }
    fn refresh(self: &Rc<Self>) {
        let Some(ui) = self.current_ui() else { return };
        // A request invalidates any older worker even when this request must
        // wait for another worker or its current date/company is invalid.
        self.revision.set(self.revision.get() + 1);
        self.invalidate_rows();
        if ui.background_busy.get() || ui.persistence_busy.get() {
            update_text(&self.status, "در انتظار پایان کار جاری برای خواندن اسناد…");
            if !self.pending.replace(true) {
                let weak = Rc::downgrade(self);
                glib::timeout_add_local(Duration::from_millis(60), move || {
                    let Some(p) = weak.upgrade() else {
                        return glib::ControlFlow::Break;
                    };
                    let Some(ui) = p.current_ui() else {
                        return glib::ControlFlow::Break;
                    };
                    if ui.background_busy.get() || ui.persistence_busy.get() {
                        return glib::ControlFlow::Continue;
                    }
                    p.pending.set(false);
                    p.refresh();
                    glib::ControlFlow::Break
                });
            }
            return;
        }
        self.refreshing.set(true);
        let old = self.company_id().ok();
        let companies = ui.store.borrow().state.accounting.companies.clone();
        self.company.set_model(Some(&gtk::StringList::new(
            &companies
                .iter()
                .map(|x| x.name.as_str())
                .collect::<Vec<_>>(),
        )));
        *self.company_ids.borrow_mut() = companies.iter().map(|c| c.id.clone()).collect();
        if let Some(i) = old.and_then(|id| companies.iter().position(|c| c.id == id)) {
            self.company.set_selected(i as u32);
        }
        self.refreshing.set(false);
        let company = match self.company_id() {
            Ok(x) => x,
            Err(_) => {
                self.invalidate_rows();
                self.status
                    .set_text("ابتدا از دفتر حسابداری شرکت و افتتاحیه را راه‌اندازی کنید.");
                return;
            }
        };
        let cutoff = match date_value(&self.as_of) {
            Ok(x) => x,
            Err(e) => {
                self.status.set_text(&e.to_string());
                return;
            }
        };
        let state = ui.store.borrow().state.clone();
        let revision = self.revision.get();
        self.status.set_text("در حال خواندن اسناد…");
        let weak = Rc::downgrade(self);
        ui.run_background(
            "خواندن اسناد فروش…",
            move || {
                let resources = state.accounting.procurement_resources(&company)?;
                let mut rows = Vec::new();
                for d in state
                    .sales_drafts
                    .iter()
                    .filter(|d| d.company_id == company)
                {
                    rows.push((
                        d.id.clone(),
                        format!("پیش‌نویس • {}", d.reference),
                        format!(
                            "{} • ذخیره‌شده، غیرمالی • نسخه {}",
                            resource_name(&resources, "party", &d.customer_id),
                            d.revision
                        ),
                        true,
                    ));
                }
                for r in state.accounting.sales.records.iter().filter(|r| {
                    r.command.context().company_id == company && r.command.context().date <= cutoff
                }) {
                    let (title, summary) =
                        record_summary(r, &resources, &state.settings, MoneyUnit::Rial);
                    rows.push((r.id.clone(), title, summary, false));
                }
                let index =
                    SearchIndex::new(rows.iter().map(|r| format!("{} {} {}", r.0, r.1, r.2)));
                Ok(Arc::new(PageSnapshot {
                    state,
                    resources,
                    rows,
                    index,
                }))
            },
            move |ui, result| {
                let Some(p) = weak.upgrade() else {
                    ui.message(result.map(|_| "خواندن اسناد فروش پایان یافت."));
                    return;
                };
                if p.current_ui().is_none() || p.revision.get() != revision {
                    ui.message(Ok(
                        "خواندن نسخهٔ قبلی پایان یافت؛ نتیجه برای فیلتر فعلی نمایش داده نشد.",
                    ));
                    return;
                }
                match result {
                    Ok(snapshot) => {
                        *p.snapshot.borrow_mut() = Some(snapshot);
                        p.render();
                        ui.message(Ok("اسناد فروش آماده‌اند."));
                    }
                    Err(e) => {
                        p.invalidate_rows();
                        p.status.set_text(&format!("{e:#}"));
                        ui.message(Err::<String, _>(e));
                    }
                }
            },
        );
    }
    fn render(self: &Rc<Self>) {
        let Some(ui) = self.current_ui() else { return };
        let data = self.snapshot.borrow();
        let Some(snapshot) = data.as_ref() else {
            return;
        };
        let (mut indices, total) =
            snapshot
                .index
                .search_page(&self.search.text(), self.offset.get(), PAGE_SIZE);
        let last = total.saturating_sub(1) / PAGE_SIZE * PAGE_SIZE;
        if self.offset.get() > last {
            self.offset.set(last);
            indices = snapshot
                .index
                .search_page(&self.search.text(), last, PAGE_SIZE)
                .0;
        }
        update_text(
            &self.status,
            &format!(
                "{} نتیجه • اسناد تا {} • پیش‌نویس‌های ذخیره‌شده • مبالغ {}",
                fa(total),
                financial_date(&ui, &date_value(&self.as_of).unwrap_or_default()),
                if money_unit(&self.unit) == MoneyUnit::Rial {
                    "ریال"
                } else {
                    "تومان"
                }
            ),
        );
        for (position, slot) in self.cards.iter().enumerate() {
            let Some(&index) = indices.get(position) else {
                self.preserve_focus_before_hiding(&slot.card);
                slot.current.borrow_mut().take();
                slot.open.set_sensitive(false);
                slot.card.set_visible(false);
                continue;
            };
            let (id, title, subtitle, draft) = &snapshot.rows[index];
            let shown = if !*draft {
                snapshot
                    .state
                    .accounting
                    .sales
                    .records
                    .iter()
                    .find(|record| record.id == *id)
                    .map(|record| {
                        record_summary(
                            record,
                            &snapshot.resources,
                            &snapshot.state.settings,
                            money_unit(&self.unit),
                        )
                        .1
                    })
                    .unwrap_or_else(|| subtitle.clone())
            } else {
                subtitle.clone()
            };
            *slot.current.borrow_mut() = Some((id.clone(), *draft));
            update_text(&slot.title, title);
            update_text(&slot.summary, &shown);
            update_button(
                &slot.open,
                if *draft {
                    "ادامهٔ پیش‌نویس"
                } else {
                    "جزئیات و جریان کار"
                },
            );
            update_name(&slot.open, &format!("sales-open-{id}"));
            slot.open.set_sensitive(true);
            slot.card.set_visible(true);
        }
        self.empty.set_visible(total == 0);
        if self.offset.get() == 0 {
            self.preserve_focus_before_hiding(&self.previous);
        }
        if self.offset.get() + PAGE_SIZE >= total {
            self.preserve_focus_before_hiding(&self.next);
        }
        self.previous.set_sensitive(self.offset.get() > 0);
        self.next
            .set_sensitive(self.offset.get() + PAGE_SIZE < total);
    }
    fn edit_draft(&self, id: &str) {
        let Some(ui) = self.current_ui() else { return };
        let draft = ui
            .store
            .borrow()
            .state
            .sales_drafts
            .iter()
            .find(|d| d.id == id)
            .cloned();
        if let Some(draft) = draft {
            let resources = ui
                .store
                .borrow()
                .state
                .accounting
                .procurement_resources(&draft.company_id);
            match resources {
                Ok(r) => order_dialog(&ui, draft, r),
                Err(e) => ui.message(Err::<String, _>(e)),
            };
        }
    }
    fn new_order(&self) {
        let Some(ui) = self.current_ui() else { return };
        let result: Result<_> = (|| {
            let company = self.company_id()?;
            let date = date_value(&self.as_of)?;
            let c = context(&ui, &company, &date)?;
            let resources = ui
                .store
                .borrow()
                .state
                .accounting
                .procurement_resources(&company)?;
            let settings = ui.store.borrow().state.settings.clone();
            Ok((
                RawSalesDraft {
                    id: c.source_id,
                    revision: 0,
                    company_id: c.company_id,
                    period_id: c.period_id,
                    setup_id: c.setup_id,
                    customer_id: resources
                        .parties
                        .iter()
                        .find(|p| p.customer)
                        .map(|p| p.id.clone())
                        .unwrap_or_default(),
                    reference: String::new(),
                    document_date: settings.date_display.format(&date),
                    financial_date: settings.date_display.format(&date),
                    date_display: settings.date_display,
                    display_unit: MoneyUnit::Rial,
                    document_discount: "0".into(),
                    supplied_total: String::new(),
                    reason: String::new(),
                    quotation: None,
                    lines: Vec::new(),
                },
                resources,
            ))
        })();
        match result {
            Ok((d, r)) => order_dialog(&ui, d, r),
            Err(e) => ui.message(Err::<String, _>(e)),
        };
    }
    fn from_quote(self: &Rc<Self>) {
        let Some(ui) = self.current_ui() else { return };
        let company = match self.company_id() {
            Ok(c) => c,
            Err(e) => {
                ui.message(Err::<String, _>(e));
                return;
            }
        };
        let quotes = ui
            .store
            .borrow()
            .state
            .quotes
            .iter()
            .map(|q| {
                (
                    q.id.clone(),
                    format!("{} • {} • {} ریال", q.number, q.customer, q.total_rial),
                )
            })
            .collect::<Vec<_>>();
        let (dialog, body, error, save) =
            form_dialog(&ui, "تبدیل پیشنهاد تأییدشده", "sales-quotation");
        let choice = choice("پیشنهاد تأییدشده", "sales-quotation-choice", &quotes);
        body.append(&choice);
        body.append(&label("پیشنهاد اصلی حفظ می‌شود؛ مالیات‌دار یا نگاشت مبهم قابل تبدیل نیست. انبار و مشتری را پیش از ذخیرهٔ سفارش بررسی کنید."));
        save.set_label("بازکردن پیش‌نویس مرتبط");
        let weak = Rc::downgrade(&ui);
        let d = dialog.clone();
        save.connect_clicked(move |_| {
            let Some(ui) = weak.upgrade() else { return };
            let result: Result<_> = (|| {
                let id = selected(&choice, &ids(&quotes), "پیشنهاد")?;
                let state = &ui.store.borrow().state;
                let draft = state.sales_draft_from_quote(&id, &company)?;
                let resources = state.accounting.procurement_resources(&company)?;
                Ok((draft, resources))
            })();
            match result {
                Ok((draft, resources)) => {
                    d.close();
                    order_dialog(&ui, draft, resources);
                }
                Err(e) => show_error(&error, e),
            };
        });
    }
}

struct Editor {
    ui: Weak<Ui>,
    dialog: adw::Dialog,
    error: gtk::Label,
    save: gtk::Button,
    post: gtk::Button,
    output: gtk::Box,
    status: gtk::Label,
    base: RefCell<RawSalesDraft>,
    expected: Cell<Option<u64>>,
    customer: gtk::DropDown,
    customers: Vec<(String, String)>,
    reference: gtk::Entry,
    financial_date: DateField,
    document_date: DateField,
    reason: gtk::Entry,
    unit: gtk::DropDown,
    unit_current: Cell<MoneyUnit>,
    unit_changing: Cell<bool>,
    discount: gtk::Entry,
    total: gtk::Entry,
    lines: Rc<RefCell<Vec<RawSalesLine>>>,
    rows: gtk::Box,
    line_cards: RefCell<Vec<Rc<SalesLineCard>>>,
    line_previous: gtk::Button,
    line_next: gtk::Button,
    line_summary: gtk::Label,
    line_page: Cell<usize>,
    epoch: Cell<u64>,
    saved_epoch: Cell<u64>,
    verified: RefCell<Option<(RawSalesDraft, SalesCommand)>>,
    resources: RegisterResources,
    editing_line: Cell<bool>,
}
thread_local! {static EDITORS:RefCell<Vec<Weak<Editor>>>=const{RefCell::new(Vec::new())};}
fn raw_date(field: &DateField, dates: DateDisplay) -> String {
    match date_value(field) {
        Ok(canonical) => dates.format(&canonical),
        Err(_) => field.entry.text().to_string(),
    }
}
fn restore_raw_date(field: &DateField, value: &str, dates: DateDisplay) {
    field
        .calendar
        .set_selected(u32::from(dates == DateDisplay::Jalali));
    let shown = if dates == DateDisplay::DayMonthYear {
        chrono::NaiveDate::parse_from_str(&digits(value), "%d/%m/%Y")
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|_| value.to_owned())
    } else {
        value.to_owned()
    };
    field.entry.set_text(&shown);
}
fn order_dialog(ui: &Rc<Ui>, draft: RawSalesDraft, resources: RegisterResources) {
    let (dialog, body, error, post) =
        form_dialog(ui, "سفارش فروش • پیش‌نویس و بازبینی", "sales-order");
    if let Some(parent) = post.parent().and_then(|w| w.downcast::<gtk::Box>().ok()) {
        parent.set_orientation(gtk::Orientation::Vertical);
    }
    let save = action("ذخیرهٔ پیش‌نویس", "sales-draft-save");
    if let Some(parent) = post.parent().and_then(|w| w.downcast::<gtk::Box>().ok()) {
        parent.prepend(&save);
    }
    post.set_label("ثبت سفارش بازبینی‌شده");
    post.set_sensitive(false);
    let status = label(if draft.revision == 0 {
        "ذخیره‌نشده • هیچ اثر مالی یا موجودی ندارد"
    } else {
        "پیش‌نویس ذخیره‌شده • غیرمالی"
    });
    status.set_widget_name("sales-draft-status");
    body.append(&status);
    if let Some(q) = &draft.quotation {
        body.append(&label(&format!(
            "مرتبط با پیشنهاد {} • نسخه {}؛ قیمت‌ها و مقادیر اصلی باید حفظ شوند",
            isolate(&q.quote_number),
            q.quote_revision
        )));
    }
    let customers = resources_choices(&resources, "customer");
    let customer = choice("مشتری", "sales-order-customer", &customers);
    if let Some(i) = customers.iter().position(|x| x.0 == draft.customer_id) {
        customer.set_selected(i as u32);
    } else {
        customer.set_selected(gtk::INVALID_LIST_POSITION);
    }
    body.append(&field("مشتری", &customer));
    let reference = named_entry(
        "شماره یا مرجع سفارش",
        "sales-order-reference",
        &draft.reference,
        true,
    );
    body.append(&field("شماره یا مرجع سفارش", &reference));
    let financial_date = date_field(ui, "تاریخ مالی سفارش", "sales-order-date", "");
    restore_raw_date(&financial_date, &draft.financial_date, draft.date_display);
    body.append(&financial_date.widget);
    let document_date = date_field(ui, "تاریخ سند سفارش", "sales-order-document-date", "");
    restore_raw_date(&document_date, &draft.document_date, draft.date_display);
    body.append(&document_date.widget);
    let reason = named_entry("شرح و دلیل ثبت", "sales-order-reason", &draft.reason, false);
    body.append(&field("شرح و دلیل ثبت", &reason));
    let unit = gtk::DropDown::from_strings(&["ریال • IRR", "تومان • ۱ تومان = ۱۰ ریال"]);
    unit.set_selected(u32::from(draft.display_unit == MoneyUnit::Toman));
    unit.set_widget_name("sales-order-money-unit");
    accessible_name(&unit, "واحد مبلغ‌های ورودی");
    body.append(&unit);
    let discount = named_entry(
        "تخفیف کل سند",
        "sales-order-discount",
        &draft.document_discount,
        true,
    );
    let total = named_entry(
        "جمع خالص مستقل سفارش",
        "sales-order-total",
        &draft.supplied_total,
        true,
    );
    body.append(&field("تخفیف کل سند", &discount));
    body.append(&field("جمع خالص مستقل سفارش", &total));
    let add = action("افزودن کالا و قیمت", "sales-order-add-line");
    body.append(&add);
    let rows = column();
    body.append(&rows);
    let check = action("بازبینی سفارش", "sales-order-preview");
    // Keep review reachable beside save and post even when the line list and
    // source fields extend below the viewport.
    if let Some(footer) = post.parent().and_then(|w| w.downcast::<gtk::Box>().ok()) {
        footer.insert_child_after(&check, Some(&save));
    } else {
        body.append(&check);
    }
    let output = column();
    output.set_widget_name("sales-order-preview-output");
    body.append(&output);
    let expected = (draft.revision > 0).then_some(draft.revision);
    let editor = Rc::new(Editor {
        ui: Rc::downgrade(ui),
        dialog: dialog.clone(),
        error: error.clone(),
        save: save.clone(),
        post: post.clone(),
        output,
        status,
        base: RefCell::new(draft.clone()),
        expected: Cell::new(expected),
        customer: customer.clone(),
        customers,
        reference: reference.clone(),
        financial_date: financial_date.clone(),
        document_date: document_date.clone(),
        reason: reason.clone(),
        unit: unit.clone(),
        unit_current: Cell::new(draft.display_unit),
        unit_changing: Cell::new(false),
        discount: discount.clone(),
        total: total.clone(),
        lines: Rc::new(RefCell::new(draft.lines)),
        rows,
        line_cards: RefCell::new(Vec::new()),
        line_previous: button("ردیف‌های قبل"),
        line_next: button("ردیف‌های بعد"),
        line_summary: label(""),
        line_page: Cell::new(0),
        epoch: Cell::new(0),
        saved_epoch: Cell::new(0),
        verified: RefCell::new(None),
        resources,
        editing_line: Cell::new(false),
    });
    EDITORS.with(|v| v.borrow_mut().push(Rc::downgrade(&editor)));
    for e in [
        &reference,
        &reason,
        &discount,
        &total,
        &financial_date.entry,
        &document_date.entry,
    ] {
        let w = Rc::downgrade(&editor);
        e.connect_changed(move |_| {
            if let Some(e) = w.upgrade() {
                e.changed();
            }
        });
    }
    for c in [&customer, &financial_date.calendar, &document_date.calendar] {
        let w = Rc::downgrade(&editor);
        c.connect_selected_notify(move |_| {
            if let Some(e) = w.upgrade() {
                e.changed();
            }
        });
    }
    let e = Rc::downgrade(&editor);
    unit.connect_selected_notify(move |_| {
        if let Some(e) = e.upgrade() {
            e.change_unit();
        }
    });
    for (control, which) in [(&save, 0), (&post, 1), (&check, 2), (&add, 3)] {
        let e = Rc::downgrade(&editor);
        control.connect_clicked(move |_| {
            if let Some(e) = e.upgrade() {
                match which {
                    0 => e.persist(),
                    1 => e.submit(),
                    2 => e.check(),
                    _ => line_dialog(&e, None),
                }
            }
        });
    }
    let e = Rc::downgrade(&editor);
    let keys = gtk::EventControllerKey::new();
    keys.connect_key_pressed(move |_, key, _, mods| {
        let Some(e) = e.upgrade() else {
            return glib::Propagation::Proceed;
        };
        if mods.contains(gtk::gdk::ModifierType::CONTROL_MASK) {
            match key {
                gtk::gdk::Key::s | gtk::gdk::Key::S => e.persist(),
                gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter => e.submit(),
                _ => return glib::Propagation::Proceed,
            };
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    dialog.add_controller(keys);
    let weak = Rc::downgrade(&editor);
    dialog.connect_close_attempt(move |_| {
        if let Some(e) = weak.upgrade() {
            e.confirm_close();
        }
    });
    let weak_ui = Rc::downgrade(ui);
    dialog.connect_closed(move |_| {
        if let Some(ui) = weak_ui.upgrade() {
            refresh_all(&ui);
        }
    });
    // The dialog keeps one owner until close; all control callbacks use Weak.
    let keeper = Rc::new(RefCell::new(Some(editor.clone())));
    dialog.connect_closed(move |_| {
        keeper.borrow_mut().take();
    });
    editor.initialize_lines();
    editor.render_lines();
    reference.grab_focus();
}
impl Editor {
    fn changed(&self) {
        self.epoch.set(self.epoch.get() + 1);
        self.verified.borrow_mut().take();
        self.post.set_sensitive(false);
        clear(&self.output);
        self.status
            .set_text("تغییرات ذخیره‌نشده • Ctrl+S ذخیرهٔ پیش‌نویس");
        self.dialog.set_can_close(false);
    }
    fn collect(&self) -> RawSalesDraft {
        let mut d = self.base.borrow().clone();
        d.revision = self.expected.get().unwrap_or(0);
        d.customer_id = self
            .customers
            .get(self.customer.selected() as usize)
            .map(|x| x.0.clone())
            .unwrap_or_default();
        d.reference = self.reference.text().to_string();
        d.financial_date = raw_date(&self.financial_date, d.date_display);
        d.document_date = raw_date(&self.document_date, d.date_display);
        d.reason = self.reason.text().to_string();
        d.display_unit = self.unit_current.get();
        d.document_discount = self.discount.text().to_string();
        d.supplied_total = self.total.text().to_string();
        d.lines = self.lines.borrow().clone();
        d
    }
    fn change_unit(self: &Rc<Self>) {
        if self.unit_changing.get() {
            return;
        }
        let next = money_unit(&self.unit);
        let old = self.unit_current.get();
        if next == old {
            return;
        }
        let result: Result<_> = (|| {
            let convert = |s: &str| -> Result<String> {
                if s.trim().is_empty() {
                    Ok(s.into())
                } else {
                    Ok(Money::parse(s, old)?.display(next))
                }
            };
            let discount = convert(&self.discount.text())?;
            let total = convert(&self.total.text())?;
            let mut lines = self.lines.borrow().clone();
            for l in &mut lines {
                l.unit_price = convert(&l.unit_price)?;
                l.line_discount = convert(&l.line_discount)?;
            }
            Ok((discount, total, lines))
        })();
        match result {
            Ok((d, t, lines)) => {
                self.unit_current.set(next);
                self.discount.set_text(&d);
                self.total.set_text(&t);
                *self.lines.borrow_mut() = lines;
                self.changed();
                self.render_lines();
            }
            Err(e) => {
                self.unit_changing.set(true);
                self.unit.set_selected(u32::from(old == MoneyUnit::Toman));
                self.unit_changing.set(false);
                show_error(&self.error, format!("{e}؛ ابتدا مبلغ نادرست را اصلاح کنید."));
            }
        }
    }
    fn persist(self: &Rc<Self>) {
        if self.editing_line.get() {
            show_error(&self.error, "ابتدا ویرایش ردیف را اعمال یا لغو کنید.");
            return;
        }
        let Some(ui) = self.ui.upgrade() else { return };
        let draft = self.collect();
        let expected = self.expected.get();
        let epoch = self.epoch.get();
        let weak = Rc::downgrade(self);
        self.status.set_text("در حال ذخیرهٔ پیش‌نویس…");
        run_state(
            &ui,
            &self.dialog,
            &self.error,
            &self.save,
            false,
            move |s| s.save_sales_draft(draft, expected),
            move |ui, revision| {
                let Some(e) = weak.upgrade() else { return };
                e.expected.set(Some(revision));
                e.base.borrow_mut().revision = revision;
                e.saved_epoch.set(epoch);
                e.error.set_visible(false);
                if e.epoch.get() == epoch {
                    e.status
                        .set_text(&format!("پیش‌نویس ذخیره شد • نسخه {} • غیرمالی", revision));
                    e.dialog.set_can_close(true);
                } else {
                    e.status
                        .set_text("نسخهٔ قبلی ذخیره شد؛ تغییرات تازه هنوز ذخیره‌نشده‌اند.");
                    e.dialog.set_can_close(false);
                }
                ui.message(Ok("پیش‌نویس فروش به‌صورت امن ذخیره شد."));
            },
        );
    }
    fn check(self: &Rc<Self>) {
        if self.editing_line.get() {
            show_error(&self.error, "ابتدا ویرایش ردیف را اعمال یا لغو کنید.");
            return;
        }
        let draft = self.collect();
        let Some(ui) = self.ui.upgrade() else { return };
        let mut state = ui.store.borrow().state.clone();
        let expected = self.expected.get();
        let monitor = SalesPhase::start(&ui, "preview", false, "order");
        let completion = monitor.clone();
        let working = draft.clone();
        let epoch = self.epoch.get();
        let weak = Rc::downgrade(self);
        self.post.set_sensitive(false);
        self.error.set_visible(false);
        let launched = ui.run_background(
            "بررسی سفارش فروش…",
            move || {
                let revision = state.save_sales_draft(working.clone(), expected)?;
                let command = SalesCommand::Order(state.sales_order_from_draft(
                    &working.id,
                    revision,
                    &format!("sales-order-{}", working.id),
                    ACTOR,
                )?);
                let preview = state.accounting.preview_sales(&command)?;
                Ok((command, preview))
            },
            move |ui, result| {
                let Some(e) = weak.upgrade() else {
                    finish_monitor(&completion, "closed");
                    ui.message(
                        result.map(|_| "بازبینی سفارش پایان یافت؛ پنجره بسته شد و سفارشی ثبت نشد."),
                    );
                    return;
                };
                if e.epoch.get() != epoch || !e.dialog.is_visible() {
                    finish_monitor(&completion, "stale_or_closed");
                    ui.message(Ok(
                        "بازبینی قبلی سفارش پایان یافت؛ نسخهٔ فعلی بازبینی نشده است.",
                    ));
                    return;
                }
                let phase_result = if result.is_ok() {
                    "verified"
                } else {
                    "rejected"
                };
                clear(&e.output);
                match result {
                    Ok((command, p)) => {
                        render_preview(
                            &e.output,
                            &p,
                            &ui.store.borrow().state.settings,
                            &ui.store.borrow().state.accounting,
                        );
                        *e.verified.borrow_mut() = Some((draft, command));
                        e.post.set_sensitive(true);
                        reveal_review(&e.output);
                        ui.message(Ok("بازبینی سفارش آماده است؛ هنوز ثبت نشده است."));
                    }
                    Err(error) => {
                        show_error(&e.error, format!("{error:#}"));
                        e.focus_invalid();
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
                "بررسی دیگری در حال انجام است؛ ورودی حفظ شد. دوباره بازبینی کنید.",
            );
        }
    }
    fn focus_invalid(&self) {
        if date_value(&self.financial_date).is_err() {
            return;
        }
        if date_value(&self.document_date).is_err() {
            return;
        }
        if money_entry(&self.discount, self.unit_current.get()).is_err() {
            return;
        }
        if money_entry(&self.total, self.unit_current.get()).is_err() {
            return;
        }
        self.reference.grab_focus();
    }
    fn submit(self: &Rc<Self>) {
        let draft = self.collect();
        if self
            .verified
            .borrow()
            .as_ref()
            .is_none_or(|(d, _)| d != &draft)
        {
            show_error(&self.error, "ابتدا نسخهٔ فعلی سفارش را بازبینی کنید.");
            return;
        }
        let Some(ui) = self.ui.upgrade() else { return };
        let expected = self.expected.get();
        let id = draft.id.clone();
        let key = format!("sales-order-{id}");
        let weak = Rc::downgrade(self);
        run_state(
            &ui,
            &self.dialog,
            &self.error,
            &self.post,
            true,
            move |s| {
                let revision = s.save_sales_draft(draft, expected)?;
                s.post_sales_from_draft(&id, revision, &key, ACTOR)
            },
            move |ui, _| {
                if let Some(e) = weak.upgrade() {
                    e.dialog.set_sensitive(true);
                    e.dialog.set_can_close(true);
                    e.dialog.close();
                }
                ui.message(Ok("سفارش فروش ثبت شد • هنوز تحویل یا صورتحساب نشده است."));
            },
        );
    }
    fn confirm_close(self: &Rc<Self>) {
        let Some(ui) = self.ui.upgrade() else { return };
        if ui.persistence_busy.get() {
            show_error(&self.error, "ذخیره در حال انجام است؛ منتظر نتیجه بمانید.");
            return;
        }
        let a = adw::AlertDialog::new(
            Some("تغییرات ذخیره‌نشدهٔ سفارش"),
            Some(
                "برای بازیابی پس از بستن برنامه پیش‌نویس را ذخیره کنید. کنارگذاشتن فقط تغییرات ذخیره‌نشده را حذف می‌کند.",
            ),
        );
        a.add_response("keep", "ادامهٔ ویرایش");
        a.add_response("discard", "کنارگذاشتن تغییرات");
        a.add_response("save", "ذخیرهٔ پیش‌نویس");
        a.set_default_response(Some("keep"));
        a.set_close_response("keep");
        let weak = Rc::downgrade(self);
        a.connect_response(None, move |_, response| {
            if let Some(e) = weak.upgrade() {
                match response {
                    "discard" => {
                        e.dialog.set_can_close(true);
                        e.dialog.close();
                    }
                    "save" => e.persist(),
                    _ => {}
                }
            }
        });
        ui.present_dialog(a.upcast_ref());
    }
    fn initialize_lines(self: &Rc<Self>) {
        let mut cards = self.line_cards.borrow_mut();
        for _ in 0..PAGE_SIZE {
            let card = column();
            let summary = label("");
            card.append(&summary);
            let actions = row();
            let buttons = ["ویرایش ردیف", "حذف ردیف", "بالا", "پایین"]
                .map(button)
                .to_vec();
            for button in &buttons {
                actions.append(button);
            }
            card.append(&actions);
            card.set_visible(false);
            self.rows.append(&card);
            let slot = Rc::new(SalesLineCard {
                card,
                summary,
                actions: buttons,
                current_id: RefCell::new(None),
            });
            for (kind, button) in slot.actions.iter().enumerate() {
                let current = Rc::downgrade(&slot);
                let weak = Rc::downgrade(self);
                button.connect_clicked(move |button| {
                    let Some(editor) = weak.upgrade() else { return };
                    let Some(current) = current.upgrade() else {
                        return;
                    };
                    if editor.editing_line.get()
                        || !current.card.is_visible()
                        || !button.is_sensitive()
                    {
                        return;
                    }
                    let Some(id) = current.current_id.borrow().clone() else {
                        return;
                    };
                    let Some(index) = editor.lines.borrow().iter().position(|line| line.id == id)
                    else {
                        return;
                    };
                    if kind == 0 {
                        line_dialog(&editor, Some(index));
                        return;
                    }
                    let mut lines = editor.lines.borrow_mut();
                    match kind {
                        1 => {
                            lines.remove(index);
                        }
                        2 if index > 0 => lines.swap(index, index - 1),
                        3 if index + 1 < lines.len() => lines.swap(index, index + 1),
                        _ => return,
                    }
                    drop(lines);
                    editor.changed();
                    editor.render_lines();
                });
            }
            cards.push(slot);
        }
        let pager = row();
        for (button, forward) in [(&self.line_previous, false), (&self.line_next, true)] {
            let weak = Rc::downgrade(self);
            button.connect_clicked(move |button| {
                if !button.is_sensitive() {
                    return;
                }
                if let Some(editor) = weak.upgrade() {
                    editor.line_page.set(if forward {
                        editor.line_page.get() + 1
                    } else {
                        editor.line_page.get().saturating_sub(1)
                    });
                    editor.render_lines();
                }
            });
            pager.append(button);
        }
        self.rows.append(&pager);
        self.rows.append(&self.line_summary);
    }
    fn render_lines(self: &Rc<Self>) {
        let lines = self.lines.borrow();
        let count = lines.len();
        let page = self
            .line_page
            .get()
            .min(count.saturating_sub(1) / PAGE_SIZE);
        self.line_page.set(page);
        let start = page * PAGE_SIZE;
        for (position, slot) in self.line_cards.borrow().iter().enumerate() {
            let index = start + position;
            let Some(line) = lines.get(index) else {
                if self.ui.upgrade().is_some_and(|ui| {
                    gtk::prelude::GtkWindowExt::focus(&ui.window)
                        .is_some_and(|focus| focus == slot.card || focus.is_ancestor(&slot.card))
                }) {
                    self.reference.grab_focus();
                }
                slot.current_id.borrow_mut().take();
                for button in &slot.actions {
                    button.set_sensitive(false);
                }
                slot.card.set_visible(false);
                continue;
            };
            *slot.current_id.borrow_mut() = Some(line.id.clone());
            update_text(
                &slot.summary,
                &format!(
                    "{} • {} • مقدار {} • قیمت {} {}",
                    resource_name(&self.resources, "item", &line.item_id),
                    resource_name(&self.resources, "unit", &line.unit_id),
                    isolate(&line.quantity),
                    isolate(&line.unit_price),
                    if self.unit_current.get() == MoneyUnit::Rial {
                        "ریال"
                    } else {
                        "تومان"
                    }
                ),
            );
            for (kind, button) in slot.actions.iter().enumerate() {
                update_name(button, &format!("sales-line-{kind}-{index}"));
                button.set_sensitive(match kind {
                    2 => index > 0,
                    3 => index + 1 < count,
                    _ => true,
                });
            }
            slot.card.set_visible(true);
        }
        self.line_previous.set_sensitive(start > 0);
        self.line_next.set_sensitive(start + PAGE_SIZE < count);
        update_text(
            &self.line_summary,
            &format!("{} ردیف • صفحه {}", fa(count), fa(page + 1)),
        );
    }
}
pub(super) fn has_unsaved(ui: &Rc<Ui>) -> bool {
    EDITORS.with(|v| {
        v.borrow().iter().filter_map(Weak::upgrade).any(|e| {
            e.ui.upgrade().is_some_and(|u| Rc::ptr_eq(&u, ui))
                && e.dialog.is_visible()
                && (e.expected.get().is_none() || e.epoch.get() != e.saved_epoch.get())
        })
    })
}
pub(super) fn explain_unsaved(ui: &Rc<Ui>) {
    EDITORS.with(|v| {
        if let Some(e) =
            v.borrow().iter().filter_map(Weak::upgrade).find(|e| {
                e.ui.upgrade().is_some_and(|u| Rc::ptr_eq(&u, ui)) && e.dialog.is_visible()
            })
        {
            e.confirm_close();
        }
    });
}

fn line_dialog(editor: &Rc<Editor>, index: Option<usize>) {
    let Some(ui) = editor.ui.upgrade() else {
        return;
    };
    editor.editing_line.set(true);
    let old = index.and_then(|i| editor.lines.borrow().get(i).cloned());
    let dates = editor.base.borrow().date_display;
    let amount_unit = editor.unit_current.get();
    let (dialog, body, error, save) =
        form_dialog(&ui, "ردیف فروش • کالا و قیمت", "sales-order-line");
    save.set_label("اعمال ردیف در پیش‌نویس");
    body.append(&label("مقادیر ناقص در پیش‌نویس قابل نگهداری‌اند؛ ثبت سفارش فقط پس از بازبینی مقدار، قیمت، منبع و واحد ممکن است."));
    let items = resources_choices(&editor.resources, "item");
    let units = resources_choices(&editor.resources, "unit");
    let warehouses = resources_choices(&editor.resources, "warehouse");
    let item = choice("کالای هسته", "sales-line-item", &items);
    let unit = choice("واحد ردیف", "sales-line-unit", &units);
    let warehouse = choice("انبار فروش", "sales-line-warehouse", &warehouses);
    for (title, c) in [
        ("کالای هسته", &item),
        ("واحد مقدار و قیمت", &unit),
        ("انبار خروج کالا", &warehouse),
    ] {
        body.append(&field(title, c));
    }
    let qty = named_entry(
        "مقدار در واحد ردیف",
        "sales-line-quantity",
        old.as_ref().map(|l| l.quantity.as_str()).unwrap_or(""),
        true,
    );
    let price = named_entry(
        "قیمت هر واحد ردیف",
        "sales-line-price",
        old.as_ref().map(|l| l.unit_price.as_str()).unwrap_or(""),
        true,
    );
    let discount = named_entry(
        "تخفیف همین ردیف",
        "sales-line-discount",
        old.as_ref()
            .map(|l| l.line_discount.as_str())
            .unwrap_or("0"),
        true,
    );
    body.append(&field("مقدار در واحد مشخص‌شده (تا سه رقم اعشار)", &qty));
    body.append(&label(if amount_unit == MoneyUnit::Rial {
        "مبلغ‌های این ردیف: ریال • IRR"
    } else {
        "مبلغ‌های این ردیف: تومان • ۱۰ ریال"
    }));
    body.append(&field("قیمت هر واحد ردیف", &price));
    body.append(&field("تخفیف همین ردیف", &discount));
    let source = named_entry(
        "مرجع قیمت دستی یا منبع فهرست",
        "sales-line-price-reference",
        old.as_ref()
            .map(|l| l.price_reference.as_str())
            .unwrap_or(""),
        false,
    );
    body.append(&field("مرجع قیمت", &source));
    let price_date = date_field(&ui, "تاریخ قیمت", "sales-line-price-date", "");
    restore_raw_date(
        &price_date,
        old.as_ref()
            .map(|l| l.price_date.as_str())
            .unwrap_or(&editor.collect().document_date),
        dates,
    );
    body.append(&price_date.widget);
    let reason = named_entry(
        "دلیل قیمت دستی یا تفاوت نگاشت",
        "sales-line-override-reason",
        old.as_ref()
            .map(|l| l.override_reason.as_str())
            .unwrap_or(""),
        false,
    );
    body.append(&field(
        "دلیل قیمت دستی یا تغییر قیمت، کالا یا واحد منبع",
        &reason,
    ));
    let pick = action("انتخاب از کالا و قیمت فهرست", "sales-line-catalog-picker");
    body.prepend(&pick);
    if let Some(l) = &old {
        for (c, values, id) in [
            (&item, &items, &l.item_id),
            (&unit, &units, &l.unit_id),
            (&warehouse, &warehouses, &l.warehouse_id),
        ] {
            c.set_selected(
                values
                    .iter()
                    .position(|x| &x.0 == id)
                    .map(|i| i as u32)
                    .unwrap_or(gtk::INVALID_LIST_POSITION),
            );
        }
    }
    let baseline = Rc::new(RefCell::new(None::<(Money, String, String, String)>));
    let p = price.clone();
    let s = source.clone();
    let d = price_date.clone();
    let target = item.clone();
    let target_unit = unit.clone();
    let res = editor.resources.clone();
    let bas = baseline.clone();
    let weak = Rc::downgrade(&ui);
    let parent = dialog.downgrade();
    pick.connect_clicked(move |_|{if let (Some(ui),Some(parent))=(weak.upgrade(),parent.upgrade()){let p=p.clone();let s=s.clone();let d=d.clone();let target=target.clone();let target_unit=target_unit.clone();let res=res.clone();let bas=bas.clone();catalog_picker(&ui,&parent,Rc::new(move|pick|{
        let value=Money::from_rials(pick.product.price_rial);p.set_text(&value.display(amount_unit));s.set_text(&format!("فهرست {} | catalog={} | SHA256={} | SKU={} | نام={} | اندازه={} | واحدمنبع={} | قیمت‌ریال={} | تاریخ‌منبع={}",pick.catalog_name,pick.catalog_id,pick.fingerprint,pick.product.code,pick.product.name,pick.product.measure,pick.product.source_unit.label(),pick.product.price_rial,pick.product.source_at));s.set_editable(false);
        let canonical=pick.product.source_at.get(..10).unwrap_or("");d.set_canonical(canonical)?;
        target.set_selected(res.items.iter().position(|x|x.sku==pick.product.code).map(|i|i as u32).unwrap_or(gtk::INVALID_LIST_POSITION));let matches=res.units.iter().enumerate().filter(|(_,u)|normalize(&u.name)==normalize(&pick.product.measure)).map(|(i,_)|i).collect::<Vec<_>>();target_unit.set_selected(if matches.len()==1{matches[0]as u32}else{gtk::INVALID_LIST_POSITION});*bas.borrow_mut()=Some((value,pick.product.code,pick.product.measure,canonical.into()));Ok(())}));}});
    let parent = editor.dialog.downgrade();
    let weak = Rc::downgrade(editor);
    dialog.connect_closed(move |_| {
        if let Some(e) = weak.upgrade() {
            e.editing_line.set(false);
        }
        if let Some(p) = parent.upgrade() {
            p.set_can_close(false);
        }
    });
    qty.grab_focus();
    let e = editor.clone();
    let d = dialog.clone();
    save.connect_clicked(move |_| {
        let result: Result<_> = (|| {
            let item_id = selected(&item, &ids(&items), "کالا")?;
            let unit_id = selected(&unit, &ids(&units), "واحد")?;
            let warehouse_id = selected(&warehouse, &ids(&warehouses), "انبار")?;
            let mut why = reason.text().to_string();
            if let Some((original, sku, measure, dt)) = baseline.borrow().as_ref() {
                let mapping_changed = e
                    .resources
                    .items
                    .iter()
                    .find(|i| i.id == item_id)
                    .is_none_or(|i| &i.sku != sku)
                    || e.resources
                        .units
                        .iter()
                        .find(|u| u.id == unit_id)
                        .is_none_or(|u| normalize(&u.name) != normalize(measure))
                    || date_value(&price_date).is_ok_and(|d| &d != dt);
                let changed =
                    Money::parse(&price.text(), amount_unit).is_ok_and(|v| v != *original);
                if mapping_changed || changed {
                    why = required(&reason, "دلیل تغییر قیمت یا تفاوت نگاشت")?;
                }
            }
            let line = RawSalesLine {
                id: old
                    .as_ref()
                    .map(|l| l.id.clone())
                    .unwrap_or_else(|| Uuid::new_v4().to_string()),
                item_id,
                warehouse_id,
                unit_id,
                quantity: qty.text().to_string(),
                unit_price: price.text().to_string(),
                line_discount: discount.text().to_string(),
                price_reference: source.text().to_string(),
                price_date: raw_date(&price_date, dates),
                override_reason: why,
            };
            if let Some(index) = index {
                e.lines.borrow_mut()[index] = line;
            } else {
                e.lines.borrow_mut().push(line);
                e.line_page.set((e.lines.borrow().len() - 1) / PAGE_SIZE);
            }
            e.changed();
            e.render_lines();
            d.close();
            Ok(())
        })();
        if let Err(err) = result {
            show_error(&error, err);
        }
    });
}

fn record_summary(
    record: &SalesRecord,
    r: &RegisterResources,
    settings: &Settings,
    unit: MoneyUnit,
) -> (String, String) {
    let date = isolate(&managers::display_date(
        &record.command.context().date,
        settings.display_digits,
        settings.date_display,
    ));
    let posted = if record.journal_id.is_some() {
        "ثبت دفتر کل"
    } else {
        "غیرمالی"
    };
    match (&record.command, &record.data) {
        (SalesCommand::Order(c), SalesData::Order(o)) => (
            format!("سفارش • {}", c.reference),
            format!(
                "{} • {} • {} • {} • {}",
                o.customer.name,
                date,
                display_money(settings, o.total, unit),
                posted,
                o.lines
                    .iter()
                    .map(|l| format!("{} {}", l.item.sku, l.item.name))
                    .collect::<Vec<_>>()
                    .join("، ")
            ),
        ),
        (SalesCommand::Fulfill(c), SalesData::Fulfill(f)) => (
            format!(
                "تحویل {} • صورتحساب {}",
                c.delivery_reference, c.invoice_reference
            ),
            format!(
                "{} • {} • {} • بهای {} • {}؛ وصول انجام نشده",
                resource_name(r, "party", &f.customer.id),
                date,
                display_money(settings, f.total, unit),
                display_money(settings, f.cost, unit),
                posted
            ),
        ),
        (SalesCommand::Return(c), SalesData::Return(v)) => (
            format!(
                "برگشت {} • بستانکار {}",
                c.return_reference, c.credit_reference
            ),
            format!(
                "{} • اعتبار {} • برگشت بهای اصلی {} • {}",
                date,
                display_money(settings, v.total, unit),
                display_money(settings, v.cost, unit),
                posted
            ),
        ),
        _ => ("سند فروش".into(), date),
    }
}
fn render_preview(body: &gtk::Box, p: &SalesPreview, s: &Settings, state: &AccountingState) {
    body.append(&heading(if p.nonfinancial {
        "سفارش غیرمالی • بدون تغییر دفتر یا موجودی"
    } else {
        "ثبت مالی و حرکت کالا در یک رویداد"
    }));
    body.append(&label("در تحویل جزئی، خالص و تخفیف از تخصیص اصلی سفارش به‌صورت تجمعی گرد می‌شوند؛ آخرین بخش باقیماندهٔ دقیق را می‌گیرد. ناخالص تخصیص‌یافته برابر خالص و تخفیف‌هاست؛ قیمت اصلی سفارش حفظ می‌شود."));
    body.append(&label(&format!(
        "خالص {} • بهای کالا {} • {} حرکت انبار",
        display_money(s, p.total, MoneyUnit::Rial),
        display_money(s, p.cost, MoneyUnit::Rial),
        fa(p.movements.len())
    )));
    if let SalesData::Order(o) = &p.data {
        for l in o.lines.iter().take(PAGE_SIZE) {
            body.append(&label(&format!(
                "{} • پایه {} • ناخالص تخصیص‌یافته از سفارش {} • تخفیف ردیف {} • تخفیف سند {} • خالص {}",
                l.item.name,
                isolate(&quantity_text(l.base_quantity_thousandths)),
                display_money(s, l.gross, MoneyUnit::Rial),
                display_money(s, l.input.line_discount, MoneyUnit::Rial),
                display_money(s, l.document_discount, MoneyUnit::Rial),
                display_money(s, l.net, MoneyUnit::Rial)
            )));
        }
        if o.lines.len() > PAGE_SIZE {
            body.append(&label("بقیهٔ ردیف‌ها در صفحه‌های ویرایشگر قابل بازبینی‌اند."));
        }
    }
    match &p.data {
        SalesData::Fulfill(f) => {
            for l in &f.lines {
                body.append(&label(&format!("{} • مقدار پایه {} • ناخالص تخصیص‌یافته از سفارش {} • تخفیف ردیف {} • تخفیف سند {} • خالص {} • بهای خروج {}",l.item.name,isolate(&quantity_text(l.quantity_thousandths)),display_money(s,l.gross,MoneyUnit::Rial),display_money(s,l.line_discount,MoneyUnit::Rial),display_money(s,l.document_discount,MoneyUnit::Rial),display_money(s,l.net,MoneyUnit::Rial),display_money(s,l.cost,MoneyUnit::Rial))));
                if let Some(original) = state
                    .sales
                    .records
                    .iter()
                    .find(|r| r.id == f.order_id)
                    .and_then(|r| match &r.data {
                        SalesData::Order(o) => {
                            o.lines.iter().find(|x| x.input.id == l.order_line_id)
                        }
                        _ => None,
                    })
                {
                    body.append(&label(&format!(
                        "قیمت اصلی سفارش {} برای هر واحد {}",
                        display_money(s, original.input.unit_price, MoneyUnit::Rial),
                        isolate(&original.input.unit_id)
                    )));
                }
            }
        }
        SalesData::Return(v) => {
            for l in &v.lines {
                body.append(&label(&format!("مقدار برگشت {} • ناخالص برگشت از تخصیص اصلی {} • تخفیف ردیف {} • تخفیف سند {} • اعتبار {} • بهای اصلی {}",isolate(&quantity_text(l.quantity_thousandths)),display_money(s,l.gross,MoneyUnit::Rial),display_money(s,l.line_discount,MoneyUnit::Rial),display_money(s,l.document_discount,MoneyUnit::Rial),display_money(s,l.net,MoneyUnit::Rial),display_money(s,l.cost,MoneyUnit::Rial))));
            }
        }
        _ => {}
    }
    for m in &p.movements {
        body.append(&label(&format!(
            "انبار {} • مقدار {} ← {} • ارزش {} ← {}",
            isolate(&m.warehouse_id),
            isolate(&quantity_text(m.before.quantity_thousandths)),
            isolate(&quantity_text(m.after.quantity_thousandths)),
            display_money(s, m.before.value, MoneyUnit::Rial),
            display_money(s, m.after.value, MoneyUnit::Rial)
        )));
    }
    for l in &p.journal_lines {
        let account = state
            .accounts
            .iter()
            .find(|a| a.id == l.account_id)
            .map(|a| format!("{} • {}", a.code, a.name))
            .unwrap_or_else(|| isolate(&l.account_id));
        body.append(&label(&format!(
            "{} • بدهکار {} • بستانکار {}",
            account,
            display_money(s, l.debit, MoneyUnit::Rial),
            display_money(s, l.credit, MoneyUnit::Rial)
        )));
    }
}

pub(super) fn open_document(ui: &Rc<Ui>, record_id: &str) {
    if ui.background_busy.get() || ui.persistence_busy.get() {
        ui.message(Ok(
            "خواندن یا ذخیره در حال انجام است؛ پس از پایان دوباره سند را باز کنید.",
        ));
        return;
    }
    let state = ui.store.borrow().state.clone();
    let record = match state.accounting.sales_document(record_id) {
        Ok(r) => r.clone(),
        Err(e) => {
            ui.message(Err::<String, _>(e));
            return;
        }
    };
    let (dialog, body, error, save) = form_dialog(ui, "منبع فروش و جریان کار", "sales-detail");
    save.set_visible(false);
    body.append(&label("در حال خواندن منبع، مانده و پیوندها…"));
    let weak = dialog.downgrade();
    let id = record.id.clone();
    let company = record.command.context().company_id.clone();
    ui.run_background("خواندن منبع فروش…",move||{let resources=state.accounting.procurement_resources(&company)?;let order=if matches!(record.data,SalesData::Order(_)){state.accounting.sales_order_remaining(&id)?}else{vec![]};let returns=if matches!(record.data,SalesData::Fulfill(_)){state.accounting.sales_invoice_returnable(&id)?}else{vec![]};let report=sarv_quote::sales_export::document_preview(&state.accounting,&company,&id,MoneyUnit::Rial,state.settings.date_display,state.settings.display_digits)?;Ok((state,record,resources,order,returns,report))},move|ui,result|{let Some(dialog)=weak.upgrade().filter(|d|d.is_visible())else{ui.message(result.map(|_| "خواندن منبع فروش پایان یافت؛ پنجره بسته شده است."));return};clear(&body);match result{Err(e)=>{show_error(&error,format!("{e:#}"));ui.message(Err::<String,_>(e));},Ok((state,record,resources,order,returns,report))=>{
        body.append(&heading(&report.title));body.append(&label(&format!("{} • {} • {}",report.company,financial_date(ui,&report.as_of),report.status)));
        body.append(&label(&format!("منبع ثابت {} • ترتیب {} • سیاست {}",isolate(&record.id),fa(record.sequence),record.frozen_setup.command.policy.version)));
        match &record.data {SalesData::Order(_)=>{let complete=order.iter().all(|l|l.remaining_quantity_thousandths==0);body.append(&label(if complete{"سفارش کاملاً تحویل و صورتحساب شده؛ برگشت مقدار سفارش را باز نمی‌کند."}else{"سفارش تأیید‌شده • مانده برای تحویل و صورتحساب • بدون رزرو موجودی"}));
            for l in &order{body.append(&label(&format!("{} • سفارش {} • تحویل {} • مانده {} {} • خالص مانده {} ریال",l.item.name,isolate(&quantity_text(l.original_quantity_thousandths)),isolate(&quantity_text(l.fulfilled_quantity_thousandths)),isolate(&quantity_text(l.remaining_quantity_thousandths)),resource_name(&resources,"unit",&l.item.base_unit_id),l.remaining_net.rials())));}
            if !complete{let b=action("تحویل جزئی و صورتحساب هم‌زمان","sales-fulfill");let id=record.id.clone();let weak=Rc::downgrade(ui);b.connect_clicked(move |_|{if let Some(ui)=weak.upgrade() && let Err(e)=fulfill_dialog(&ui,&id){ui.message(Err::<String,_>(e));}});body.append(&b);}
        },SalesData::Fulfill(f)=>{body.append(&label(&format!("تحویل انجام شده • صورتحساب ثبت‌شده • وصول انجام نشده\nشناسهٔ تحویل {}\nشناسهٔ صورتحساب {}",isolate(&f.delivery_id),isolate(&f.invoice_id))));
            let remaining=returns.iter().try_fold(Money::ZERO,|s,l|s.checked_add(l.net)).unwrap_or(Money::ZERO);body.append(&label(&format!("ماندهٔ دریافتنی این صورتحساب {} ریال • فقط بستانکارهای مرتبط کسر شده‌اند؛ پرداختی در این مرحله ثبت نمی‌شود",remaining.rials())));
            for l in &returns{body.append(&label(&format!("{} • مقدار قابل برگشت {} {} • اعتبار مانده {} ریال • بهای اصلی مانده {} ریال",l.item.name,isolate(&quantity_text(l.quantity_thousandths)),resource_name(&resources,"unit",&l.item.base_unit_id),l.net.rials(),l.cost.rials())));}
            if returns.iter().any(|l|l.quantity_thousandths>0){let b=action("برگشت کالا و بستانکار مرتبط","sales-return");let id=record.id.clone();let weak=Rc::downgrade(ui);b.connect_clicked(move |_|{if let Some(ui)=weak.upgrade() && let Err(e)=return_dialog(&ui,&id){ui.message(Err::<String,_>(e));}});body.append(&b);}
            add_link(&body,ui,"سفارش اصلی",&f.order_id);
        },SalesData::Return(v)=>{body.append(&label(&format!("کالا به انبار اصلی بازگشت • بستانکار ثبت‌شده\nشناسهٔ برگشت {}\nشناسهٔ بستانکار {}\nاعتبار {} ریال • بهای اصلی {} ریال",isolate(&v.return_id),isolate(&v.credit_id),v.total.rials(),v.cost.rials())));add_link(&body,ui,"تحویل و صورتحساب اصلی",&v.fulfillment_id);}}
        let links=column();for child in state.accounting.sales.records.iter(){let linked=match &child.data{SalesData::Fulfill(f)=>f.order_id==record.id,SalesData::Return(v)=>v.fulfillment_id==record.id,_=>false};if linked{let (title,_)=record_summary(child,&resources,&state.settings,MoneyUnit::Rial);add_link(&links,ui,&title,&child.id);}}if links.first_child().is_some(){body.append(&heading("اسناد مرتبط"));body.append(&links);}
        body.append(&label("تحویل جزئی از تخصیص اصلی سفارش استفاده می‌کند. خالص و تخفیف‌ها جداگانه و تجمعی گرد می‌شوند؛ ناخالص تخصیص‌یافته جمع آن‌هاست و آخرین بخش باقیماندهٔ دقیق را می‌گیرد. قیمت اصلی سفارش حفظ شده است."));let rows=column();paged_text(&rows,report.lines);body.append(&disclosure("ردیف‌ها، اثر دفتر و جزئیات منبع",&rows));
        let flow=row();for pdf in [false,true]{let b=action(if pdf{"خروجی سند PDF"}else{"خروجی سند CSV"},if pdf{"sales-export-pdf"}else{"sales-export-csv"});let weak=Rc::downgrade(ui);let id=record.id.clone();let company=record.command.context().company_id.clone();b.connect_clicked(move |_|{if let Some(ui)=weak.upgrade(){export_document(&ui,&company,&id,pdf);}});flow.append(&b);}body.append(&flow);
        let weak_ui=Rc::downgrade(ui);dialog.connect_closed(move |_|{if let Some(ui)=weak_ui.upgrade(){refresh_all(&ui);}});ui.message(Ok("منبع فروش و جریان کار آماده‌اند."));
    }}});
}
fn add_link(body: &gtk::Box, ui: &Rc<Ui>, title: &str, id: &str) {
    let b = button(title);
    let weak = Rc::downgrade(ui);
    let id = id.to_owned();
    b.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            open_document(&ui, &id);
        }
    });
    body.append(&b);
}
fn paged_text(body: &gtk::Box, lines: Vec<String>) {
    let data = Rc::new(lines);
    let offset = Rc::new(Cell::new(0usize));
    let slot = Rc::new(RefCell::new(None::<Rc<dyn Fn()>>));
    let weak = Rc::downgrade(&slot);
    let body_for_destroy = body.clone();
    let body_weak = body.downgrade();
    let refresh: Rc<dyn Fn()> = Rc::new(move || {
        let Some(body) = body_weak.upgrade() else {
            return;
        };
        clear(&body);
        for line in data.iter().skip(offset.get()).take(PAGE_SIZE) {
            let l = label(line);
            l.set_selectable(true);
            body.append(&l);
        }
        let flow = row();
        for (caption, forward) in [("جزئیات قبل", false), ("جزئیات بعد", true)] {
            let b = button(caption);
            b.set_sensitive(if forward {
                offset.get() + PAGE_SIZE < data.len()
            } else {
                offset.get() > 0
            });
            let offset = offset.clone();
            let weak = weak.clone();
            b.connect_clicked(move |_| {
                offset.set(if forward {
                    offset.get() + PAGE_SIZE
                } else {
                    offset.get().saturating_sub(PAGE_SIZE)
                });
                if let Some(slot) = weak.upgrade()
                    && let Some(refresh) = slot.borrow().as_ref()
                {
                    refresh();
                }
            });
            flow.append(&b);
        }
        body.append(&flow);
    });
    *slot.borrow_mut() = Some(refresh.clone());
    refresh();
    let keep = slot;
    body_for_destroy.connect_destroy(move |_| {
        keep.borrow_mut().take();
    });
}
fn export_document(ui: &Rc<Ui>, company: &str, id: &str, pdf: bool) {
    let state = ui.store.borrow().state.accounting.clone();
    let settings = ui.store.borrow().state.settings.clone();
    let company = company.to_owned();
    let id = id.to_owned();
    let weak = Rc::downgrade(ui);
    choose_file(
        &ui.window,
        true,
        if pdf { "sales.pdf" } else { "sales.csv" },
        move |path| {
            let Some(ui) = weak.upgrade() else { return };
            let state = state.clone();
            let settings = settings.clone();
            let company = company.clone();
            let id = id.clone();
            ui.run_background(
                "تهیهٔ خروجی فروش…",
                move || {
                    use sarv_quote::sales_export::*;
                    if pdf {
                        document_pdf(
                            &state,
                            &company,
                            &id,
                            MoneyUnit::Rial,
                            settings.date_display,
                            settings.display_digits,
                            &path,
                        )
                    } else {
                        document_csv(
                            &state,
                            &company,
                            &id,
                            MoneyUnit::Rial,
                            settings.date_display,
                            settings.display_digits,
                            &path,
                        )
                    }
                },
                move |ui, result| ui.message(result.map(|_| "خروجی فروش ذخیره شد.")),
            );
        },
    );
}

#[derive(Clone)]
struct Common {
    base: SalesContext,
    date: DateField,
    document_date: DateField,
    reason: gtk::Entry,
}
impl Common {
    fn new(ui: &Rc<Ui>, body: &gtk::Box, company: &str, date: &str, prefix: &str) -> Result<Self> {
        let base = context(ui, company, date)?;
        let date = date_field(ui, "تاریخ مالی مشترک", &format!("{prefix}-date"), date);
        let document_date = date_field(
            ui,
            "تاریخ سند",
            &format!("{prefix}-document-date"),
            &base.date,
        );
        let reason = named_entry("شرح و دلیل ثبت", &format!("{prefix}-reason"), "", false);
        body.append(&date.widget);
        body.append(&document_date.widget);
        body.append(&field("شرح و دلیل ثبت", &reason));
        Ok(Self {
            base,
            date,
            document_date,
            reason,
        })
    }
    fn collect(&self) -> Result<SalesContext> {
        let mut c = self.base.clone();
        c.date = date_value(&self.date)?;
        c.reason = required(&self.reason, "شرح و دلیل ثبت")?;
        Ok(c)
    }
    fn connect(&self, r: &Rc<Review<SalesCommand, SalesPreview>>) {
        r.date(&self.date);
        r.date(&self.document_date);
        r.text(&self.reason);
    }
}
fn selected_quantity(q: &gtk::Entry) -> Result<i64> {
    let result = quantities(&q.text());
    entry_validity(
        q,
        result.as_ref().err().map(ToString::to_string).as_deref(),
        "مقدار مثبت تا سه رقم اعشار",
    );
    if result.is_err() {
        q.grab_focus();
    }
    result
}
fn command_date(ui: &Rc<Ui>, original: &str) -> String {
    page_for(ui)
        .and_then(|p| date_value(&p.as_of).ok())
        .unwrap_or_else(|| original.to_owned())
}
fn finish_command_dialog(ui: &Rc<Ui>, dialog: &adw::Dialog) {
    let weak = Rc::downgrade(ui);
    dialog.connect_closed(move |_| {
        if let Some(ui) = weak.upgrade() {
            refresh_all(&ui);
        }
    });
}
fn fulfill_dialog(ui: &Rc<Ui>, id: &str) -> Result<()> {
    let state = ui.store.borrow().state.accounting.clone();
    let record = state.sales_document(id)?.clone();
    let SalesData::Order(_) = &record.data else {
        anyhow::bail!("منبع سفارش نیست")
    };
    let remaining = state
        .sales_order_remaining(id)?
        .into_iter()
        .filter(|r| r.remaining_quantity_thousandths > 0)
        .collect::<Vec<_>>();
    anyhow::ensure!(!remaining.is_empty(), "سفارش ماندهٔ تحویل ندارد");
    let resources = state.procurement_resources(&record.command.context().company_id)?;
    let date = command_date(ui, &record.command.context().date);
    let (dialog, body, error, save) =
        form_dialog(ui, "تحویل جزئی و صورتحساب هم‌زمان", "sales-fulfill");
    body.append(&label("مقدار مثبت را در واحد پایه وارد کنید. در یک ثبت اتمی، تحویل و صورتحساب با شناسه‌های جدا ایجاد می‌شوند؛ دریافت وجه ثبت نمی‌شود."));
    let common = Common::new(
        ui,
        &body,
        &record.command.context().company_id,
        &date,
        "sales-fulfill",
    )?;
    let options = remaining
        .iter()
        .map(|r| {
            (
                r.id.clone(),
                format!(
                    "{} • مانده {} {} • {} ریال",
                    r.item.name,
                    quantity_text(r.remaining_quantity_thousandths),
                    resource_name(&resources, "unit", &r.item.base_unit_id),
                    r.remaining_net.rials()
                ),
            )
        })
        .collect::<Vec<_>>();
    let line = choice("ردیف ماندهٔ سفارش", "sales-fulfill-line", &options);
    body.append(&field("ردیف ماندهٔ سفارش", &line));
    let qty = named_entry(
        "مقدار تحویل در واحد پایه",
        "sales-fulfill-quantity",
        "",
        true,
    );
    body.append(&field("مقدار تحویل در واحد پایه", &qty));
    let delivery = named_entry(
        "مرجع تحویل کالا",
        "sales-fulfill-delivery-reference",
        "",
        true,
    );
    let invoice = named_entry(
        "مرجع صورتحساب فروش",
        "sales-fulfill-invoice-reference",
        "",
        true,
    );
    body.append(&field("مرجع تحویل کالا", &delivery));
    body.append(&field("مرجع صورتحساب فروش", &invoice));
    let due = date_field(ui, "سررسید صورتحساب", "sales-fulfill-due-date", &date);
    body.append(&due.widget);
    let total = named_entry("جمع خالص مستقل صورتحساب", "sales-fulfill-total", "", true);
    let money = MoneyFields::new("sales-fulfill-money-unit", vec![total.clone()], &error);
    body.append(&money.choice);
    body.append(&field("جمع خالص مستقل صورتحساب", &total));
    let c = common.clone();
    let l = line.clone();
    let q = qty.clone();
    let d = delivery.clone();
    let i = invoice.clone();
    let due_c = due.clone();
    let t = total.clone();
    let m = money.clone();
    let order_id = id.to_owned();
    let delivery_id = Uuid::new_v4().to_string();
    let invoice_id = Uuid::new_v4().to_string();
    let line_id = Uuid::new_v4().to_string();
    let collect = Rc::new(move || {
        let index = l.selected() as usize;
        let r = remaining
            .get(index)
            .ok_or_else(|| anyhow::anyhow!("ردیف سفارش را انتخاب کنید"))?;
        Ok(SalesCommand::Fulfill(FulfillOrder {
            context: c.collect()?,
            order_id: order_id.clone(),
            delivery_id: delivery_id.clone(),
            invoice_id: invoice_id.clone(),
            delivery_reference: required(&d, "مرجع تحویل")?,
            invoice_reference: required(&i, "مرجع صورتحساب")?,
            document_date: date_value(&c.document_date)?,
            due_date: date_value(&due_c)?,
            lines: vec![FulfillLine {
                id: line_id.clone(),
                order_line_id: r.id.clone(),
                warehouse_id: r.warehouse_id.clone(),
                unit_id: r.item.base_unit_id.clone(),
                quantity_thousandths: selected_quantity(&q)?,
            }],
            supplied_total: money_entry(&t, m.unit())?,
        }))
    });
    let r = review(
        &body,
        ui,
        &dialog,
        &error,
        &save,
        super::accounting::command_runner(ui),
        collect,
    );
    common.connect(&r);
    r.choice(&line);
    r.text(&qty);
    r.text(&delivery);
    r.text(&invoice);
    r.date(&due);
    r.text(&total);
    r.choice(&money.choice);
    finish_command_dialog(ui, &dialog);
    qty.grab_focus();
    Ok(())
}
fn return_dialog(ui: &Rc<Ui>, id: &str) -> Result<()> {
    let state = ui.store.borrow().state.accounting.clone();
    let record = state.sales_document(id)?.clone();
    let remaining = state
        .sales_invoice_returnable(id)?
        .into_iter()
        .filter(|r| r.quantity_thousandths > 0)
        .collect::<Vec<_>>();
    anyhow::ensure!(!remaining.is_empty(), "صورتحساب مقدار قابل برگشت ندارد");
    let resources = state.procurement_resources(&record.command.context().company_id)?;
    let date = command_date(ui, &record.command.context().date);
    let (dialog, body, error, save) = form_dialog(ui, "برگشت کالا و بستانکار مرتبط", "sales-return");
    body.append(&label("فقط کالای سالم به انبار اصلی برمی‌گردد. اعتبار و بهای برگشت از تخصیص اصلی است؛ پرداخت یا استرداد وجه در این مرحله انجام نمی‌شود."));
    let common = Common::new(
        ui,
        &body,
        &record.command.context().company_id,
        &date,
        "sales-return",
    )?;
    let options = remaining
        .iter()
        .map(|r| {
            (
                r.id.clone(),
                format!(
                    "{} • مانده {} {} • اعتبار {} ریال • بهای اصلی {} ریال",
                    r.item.name,
                    quantity_text(r.quantity_thousandths),
                    resource_name(&resources, "unit", &r.item.base_unit_id),
                    r.net.rials(),
                    r.cost.rials()
                ),
            )
        })
        .collect::<Vec<_>>();
    let line = choice("ردیف اصلی قابل برگشت", "sales-return-line", &options);
    body.append(&field("ردیف اصلی قابل برگشت", &line));
    let qty = named_entry(
        "مقدار برگشت در واحد پایه",
        "sales-return-quantity",
        "",
        true,
    );
    body.append(&field("مقدار برگشت در واحد پایه", &qty));
    let reference = named_entry("مرجع برگشت کالا", "sales-return-reference", "", true);
    let credit_reference = named_entry(
        "مرجع بستانکار فروش",
        "sales-return-credit-reference",
        "",
        true,
    );
    body.append(&field("مرجع برگشت کالا", &reference));
    body.append(&field("مرجع بستانکار فروش", &credit_reference));
    let total = named_entry("اعتبار مستقل این برگشت", "sales-return-credit", "", true);
    let cost = named_entry("بهای اصلی مستقل این برگشت", "sales-return-cost", "", true);
    let money = MoneyFields::new(
        "sales-return-money-unit",
        vec![total.clone(), cost.clone()],
        &error,
    );
    body.append(&money.choice);
    body.append(&field("اعتبار مستقل این برگشت", &total));
    body.append(&field("بهای اصلی مستقل این برگشت", &cost));
    let c = common.clone();
    let l = line.clone();
    let q = qty.clone();
    let rf = reference.clone();
    let cr = credit_reference.clone();
    let t = total.clone();
    let co = cost.clone();
    let m = money.clone();
    let fulfillment_id = id.to_owned();
    let return_id = Uuid::new_v4().to_string();
    let credit_id = Uuid::new_v4().to_string();
    let line_id = Uuid::new_v4().to_string();
    let collect = Rc::new(move || {
        let r = remaining
            .get(l.selected() as usize)
            .ok_or_else(|| anyhow::anyhow!("ردیف اصلی را انتخاب کنید"))?;
        let credit = money_entry(&t, m.unit())?;
        let cost = money_entry(&co, m.unit())?;
        Ok(SalesCommand::Return(ReturnSale {
            context: c.collect()?,
            fulfillment_id: fulfillment_id.clone(),
            return_id: return_id.clone(),
            credit_id: credit_id.clone(),
            return_reference: required(&rf, "مرجع برگشت")?,
            credit_reference: required(&cr, "مرجع بستانکار")?,
            document_date: date_value(&c.document_date)?,
            lines: vec![ReturnLine {
                id: line_id.clone(),
                fulfillment_line_id: r.id.clone(),
                quantity_thousandths: selected_quantity(&q)?,
                supplied_credit: credit,
                supplied_cost: cost,
            }],
            supplied_credit: credit,
            supplied_cost: cost,
        }))
    });
    let r = review(
        &body,
        ui,
        &dialog,
        &error,
        &save,
        super::accounting::command_runner(ui),
        collect,
    );
    common.connect(&r);
    r.choice(&line);
    r.text(&qty);
    r.text(&reference);
    r.text(&credit_reference);
    r.text(&total);
    r.text(&cost);
    r.choice(&money.choice);
    finish_command_dialog(ui, &dialog);
    qty.grab_focus();
    Ok(())
}

struct SalesPhase {
    phase: &'static str,
    operation: &'static str,
    path: PathBuf,
    began: std::time::Instant,
    last: Cell<std::time::Instant>,
    gaps: RefCell<Vec<f64>>,
    done: Cell<bool>,
}
impl SalesPhase {
    fn start(
        ui: &Rc<Ui>,
        phase: &'static str,
        post: bool,
        operation: &'static str,
    ) -> Option<Rc<Self>> {
        if std::env::var("SARV_SALES_PERFORMANCE_TRACE").as_deref() != Ok("1")
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
            path: directory.join("sales-performance-phases.jsonl"),
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
            .name("sarv-sales-metrics".into())
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
fn finish_monitor(monitor: &Option<Rc<SalesPhase>>, result: &str) {
    if let Some(monitor) = monitor {
        monitor.finish(result);
    }
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
fn qa_editor(ui: &Rc<Ui>) -> Result<Rc<Editor>> {
    EDITORS.with(|v| {
        v.borrow()
            .iter()
            .filter_map(Weak::upgrade)
            .find(|e| e.ui.upgrade().is_some_and(|u| Rc::ptr_eq(&u, ui)) && e.dialog.is_visible())
            .ok_or_else(|| anyhow::anyhow!("native sales editor absent"))
    })
}
fn qa_click(d: &impl IsA<gtk::Widget>, name: &str) -> Result<()> {
    qa_widget::<gtk::Button>(d, name)?.emit_clicked();
    Ok(())
}
fn qa_common(d: &adw::Dialog, prefix: &str, date: &str) -> Result<()> {
    qa_date(d, &format!("{prefix}-date"), date)?;
    qa_date(d, &format!("{prefix}-document-date"), date)?;
    qa_text(
        d,
        &format!("{prefix}-reason"),
        "سند ساختگی برای بررسی مستقل M5",
    )
}
fn qa_record(ui: &Rc<Ui>, kind: &str) -> Option<String> {
    ui.store
        .borrow()
        .state
        .accounting
        .sales
        .records
        .iter()
        .rev()
        .find(|r| {
            matches!(
                (&r.data, kind),
                (SalesData::Order(_), "Order")
                    | (SalesData::Fulfill(_), "Fulfill")
                    | (SalesData::Return(_), "Return")
            )
        })
        .map(|r| r.id.clone())
}
fn qa_snapshot(directory: &Path, name: &str) -> Result<bool> {
    if std::env::var("SARV_SALES_CAPTURE").as_deref() != Ok("1") {
        return Ok(true);
    }
    let path = directory.join(name);
    if path.exists() {
        return Ok(true);
    }
    thread_local! {static BEGAN:RefCell<std::collections::HashMap<String,std::time::Instant>>=RefCell::new(std::collections::HashMap::new());}
    let settled = BEGAN.with(|times| {
        times
            .borrow_mut()
            .entry(name.into())
            .or_insert_with(std::time::Instant::now)
            .elapsed()
            .as_millis()
            >= 350
    });
    if !settled {
        return Ok(false);
    }
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
        "QA active window belongs to another process"
    );
    anyhow::ensure!(
        std::process::Command::new("/usr/bin/import")
            .args(["-window", id])
            .arg(path)
            .status()?
            .success(),
        "native QA capture failed"
    );
    Ok(true)
}
/// Disposable opt-in drives actual native form signals and the durable worker.
pub(super) fn schedule(ui: &Rc<Ui>) {
    let Ok(mode) = std::env::var("SARV_SALES_SMOKE") else {
        return;
    };
    if !matches!(mode.as_str(), "full" | "stage" | "draft" | "posted") {
        return;
    }
    let directory = ui.store.borrow().directory.clone();
    let safe = directory.starts_with("/tmp")
        || directory.starts_with(Path::new(env!("CARGO_MANIFEST_DIR")).join("artifacts"));
    let synthetic = ui
        .store
        .borrow()
        .state
        .accounting
        .companies
        .iter()
        .any(|c| c.name == "F2-core-v1 • شرکت ساختگی");
    if !safe || !synthetic || std::env::var("SARV_TEST_INSTANCE").is_err() {
        ui.message(Err::<String, _>(anyhow::anyhow!(
            "sales smoke requires disposable synthetic M4 fixture"
        )));
        return;
    }
    let weak = Rc::downgrade(ui);
    let began = std::time::Instant::now();
    let mut stage = 0usize;
    let mut order = String::new();
    let mut pair = String::new();
    let mut checks = Vec::<String>::new();
    let mut newer_revision = 0u64;
    let mut cancelled_source = None;
    let mut retired_page: Option<Weak<Page>> = None;
    let mut restored_company = String::new();
    glib::timeout_add_local(Duration::from_millis(160), move || {
        let Some(ui) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        if began.elapsed().as_secs() > 180 {
            let _ = std::fs::write(
                directory.join("sales-native-failure.txt"),
                format!("timeout stage {stage}: {}", ui.status.text()),
            );
            return glib::ControlFlow::Break;
        }
        if ui.background_busy.get() || ui.persistence_busy.get() {
            return glib::ControlFlow::Continue;
        }
        let result: Result<bool> = (|| {
            let page = page_for(&ui).ok_or_else(|| anyhow::anyhow!("native Sales page absent"))?;
            if stage >= 91 && std::env::var("SARV_SALES_FOCUS_DIAGNOSTIC").as_deref() == Ok("1") {
                let focus = gtk::prelude::GtkWindowExt::focus(&ui.window);
                let mut ancestors = Vec::new();
                let mut current = focus.clone();
                while let Some(widget) = current {
                    ancestors.push(serde_json::json!({
                        "type": widget.type_().name(), "name": widget.widget_name().as_str(),
                        "visible": widget.is_visible(), "mapped": widget.is_mapped(),
                        "sensitive": widget.is_sensitive(), "focusable": widget.is_focusable(),
                    }));
                    current = widget.parent();
                }
                let receipt = serde_json::json!({"stage":stage,"focus":ancestors,
                    "visible_dialog":ui.visible_dialog().is_some(),
                    "active_dialog":ui.active_dialog.borrow().is_some()});
                use std::io::Write;
                writeln!(
                    std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(directory.join("sales-focus-diagnostic.jsonl"))?,
                    "{receipt}"
                )?;
            }
            match stage {
                0 => {
                    ui.stack.set_visible_child_name("sales");
                    page.as_of.set_canonical("2026-03-22")?;
                    page.refresh();
                    stage = if mode == "posted" { 90 } else { 1 };
                }
                1 => {
                    ui.import_catalog(&directory.join("M5-synthetic-prices.csv"));
                    stage = 2;
                }
                2 => {
                    let d = qa_dialog(&ui)?;
                    let Some(b) = qa_button(&d, "منبع و قیمت‌ها را تأیید می‌کنم")
                    else {
                        return Ok(false);
                    };
                    b.emit_clicked();
                    stage = 3;
                }
                3 => {
                    if ui.active_dialog.borrow().is_some() {
                        return Ok(false);
                    }
                    page.new_order();
                    let d = qa_dialog(&ui)?;
                    qa_text(
                        &d,
                        "sales-order-reference",
                        if mode == "draft" {
                            "DRAFT-INCOMPLETE-M5"
                        } else {
                            "SO-M5"
                        },
                    )?;
                    qa_common(&d, "sales-order", "2026-03-22")?;
                    qa_text(&d, "sales-order-discount", "1")?;
                    qa_text(&d, "sales-order-total", "199")?;
                    qa_click(&d, "sales-order-add-line")?;
                    stage = 4;
                }
                4 => {
                    let d = qa_dialog(&ui)?;
                    qa_text(&d, "sales-line-quantity", "10")?;
                    qa_click(&d, "sales-line-catalog-picker")?;
                    let picker = qa_dialog(&ui)?;
                    let catalogs = ui.store.borrow().state.catalogs.len();
                    qa_widget::<gtk::DropDown>(&picker, "sales-price-catalog")?
                        .set_selected(catalogs.saturating_sub(1) as u32);
                    qa_widget::<gtk::SearchEntry>(&picker, "sales-price-search")?.set_text("SKU-A");
                    stage = 5;
                }
                5 => {
                    let picker = qa_dialog(&ui)?;
                    let list = qa_widget::<gtk::ListBox>(&picker, "sales-price-results")?;
                    if list.row_at_index(0).is_none() {
                        return Ok(false);
                    }
                    if !qa_snapshot(&directory, "M5-catalog-picker.png")? {
                        return Ok(false);
                    }
                    qa_click(&picker, "sales-price-choose")?;
                    stage = 6;
                }
                6 => {
                    let d = qa_dialog(&ui)?;
                    anyhow::ensure!(
                        qa_widget::<gtk::Entry>(&d, "sales-line-price")?.text() == "20",
                        "actual catalog20 must be selected"
                    );
                    checks.push("actual sales catalog price20/core SKU+unit mapping".into());
                    if !qa_snapshot(&directory, "M5-frozen-catalog-line.png")? {
                        return Ok(false);
                    }
                    if mode == "draft" {
                        qa_text(&d, "sales-line-quantity", "-۱")?;
                        qa_text(&d, "sales-line-price", "0.1")?;
                    }
                    qa_click(&d, "sales-order-line-save")?;
                    stage = 7;
                }
                7 => {
                    let e = qa_editor(&ui)?;
                    e.persist();
                    e.reason.set_text("تغییر تازه هنگام ذخیرهٔ نسخهٔ قبلی");
                    stage = 8;
                }
                8 => {
                    let e = qa_editor(&ui)?;
                    anyhow::ensure!(
                        e.expected.get().is_some()
                            && e.epoch.get() != e.saved_epoch.get()
                            && e.reason.text() == "تغییر تازه هنگام ذخیرهٔ نسخهٔ قبلی",
                        "newer GTK edit must survive older receipt"
                    );
                    checks.push("older durable draft receipt retains newer GTK input".into());
                    newer_revision = e.expected.get().unwrap();
                    e.persist();
                    stage = 9;
                }
                9 => {
                    let e = qa_editor(&ui)?;
                    let draft = ui
                        .store
                        .borrow()
                        .state
                        .sales_drafts
                        .iter()
                        .find(|d| d.id == e.base.borrow().id)
                        .cloned()
                        .ok_or_else(|| anyhow::anyhow!("saved draft absent"))?;
                    anyhow::ensure!(
                        draft.revision > newer_revision
                            && draft.reason == e.reason.text()
                            && e.epoch.get() == e.saved_epoch.get(),
                        "newer raw draft persisted"
                    );
                    checks.push(
                        "explicit raw draft saves durable revision and current header/lines".into(),
                    );
                    if mode == "stage" || mode == "draft" {
                        if !qa_snapshot(&directory, "M5-saved-raw-draft.png")? {
                            return Ok(false);
                        }
                        if mode == "draft" {
                            anyhow::ensure!(
                                draft.lines[0].quantity == "-۱"
                                    && draft.lines[0].unit_price == "0.1",
                                "incomplete raw draft lost"
                            );
                        }
                        let value = serde_json::json!({"ready":true,"synthetic_only":true,"mode":mode,"stage":"saved-native-raw-draft","draft_id":draft.id,"company_id":draft.company_id,"reference":draft.reference,"revision":draft.revision,"checks":checks,"note":"no further smoke edits; external observer owns preview/submit/keyboard"});
                        std::fs::write(
                            directory.join("sales-native-verification.json"),
                            serde_json::to_vec_pretty(&value)?,
                        )?;
                        std::fs::write(
                            directory.join("sales-stage-ready.json"),
                            serde_json::to_vec_pretty(&value)?,
                        )?;
                        return Ok(true);
                    }
                    e.check();
                    stage = 10;
                }
                10 => {
                    let e = qa_editor(&ui)?;
                    if !e.post.is_sensitive() {
                        anyhow::ensure!(
                            !e.error.is_visible(),
                            "order preview failed: {}",
                            e.error.text()
                        );
                        return Ok(false);
                    }
                    if !qa_snapshot(&directory, "M5-order-preview.png")? {
                        return Ok(false);
                    }
                    e.submit();
                    stage = 11;
                }
                11 => {
                    if ui.active_dialog.borrow().is_some() {
                        return Ok(false);
                    }
                    order = qa_record(&ui, "Order")
                        .ok_or_else(|| anyhow::anyhow!("accepted order absent"))?;
                    anyhow::ensure!(
                        ui.store.borrow().state.sales_drafts.is_empty(),
                        "accepted draft must be consumed"
                    );
                    checks.push("order199 nonfinancial and raw draft consumed atomically".into());
                    fulfill_dialog(&ui, &order)?;
                    let d = qa_dialog(&ui)?;
                    qa_common(&d, "sales-fulfill", "2026-03-28")?;
                    qa_date(&d, "sales-fulfill-due-date", "2026-04-22")?;
                    qa_text(&d, "sales-fulfill-quantity", "5")?;
                    qa_text(&d, "sales-fulfill-delivery-reference", "DEL-M5-A")?;
                    qa_text(&d, "sales-fulfill-invoice-reference", "INV-M5-A")?;
                    qa_text(&d, "sales-fulfill-total", "100")?;
                    qa_click(&d, "sales-preview")?;
                    stage = 12;
                }
                12 => {
                    let d = qa_dialog(&ui)?;
                    let b = qa_widget::<gtk::Button>(&d, "sales-fulfill-save")?;
                    if !b.is_sensitive() {
                        let error = qa_widget::<gtk::Label>(&d, "accounting-form-error")?;
                        anyhow::ensure!(
                            !error.is_visible(),
                            "native paired preview failed: {}",
                            error.text()
                        );
                        return Ok(false);
                    }
                    if !qa_snapshot(&directory, "M5-first-pair-preview.png")? {
                        return Ok(false);
                    }
                    b.emit_clicked();
                    stage = 13;
                }
                13 => {
                    if ui.active_dialog.borrow().is_some() {
                        return Ok(false);
                    }
                    pair =
                        qa_record(&ui, "Fulfill").ok_or_else(|| anyhow::anyhow!("pair absent"))?;
                    checks.push(
                        "partial paired delivery/invoice100 with separate IDs and original cost58"
                            .into(),
                    );
                    return_dialog(&ui, &pair)?;
                    let d = qa_dialog(&ui)?;
                    qa_common(&d, "sales-return", "2026-03-29")?;
                    qa_text(&d, "sales-return-quantity", "2")?;
                    qa_text(&d, "sales-return-reference", "RET-M5-A")?;
                    qa_text(&d, "sales-return-credit-reference", "CN-M5-A")?;
                    qa_text(&d, "sales-return-credit", "40")?;
                    qa_text(&d, "sales-return-cost", "23")?;
                    qa_click(&d, "sales-preview")?;
                    stage = 14;
                }
                14 => {
                    let d = qa_dialog(&ui)?;
                    let b = qa_widget::<gtk::Button>(&d, "sales-return-save")?;
                    if !b.is_sensitive() {
                        let error = qa_widget::<gtk::Label>(&d, "accounting-form-error")?;
                        anyhow::ensure!(
                            !error.is_visible(),
                            "native paired preview failed: {}",
                            error.text()
                        );
                        return Ok(false);
                    }
                    if !qa_snapshot(&directory, "M5-first-return-preview.png")? {
                        return Ok(false);
                    }
                    b.emit_clicked();
                    stage = 15;
                }
                15 => {
                    if ui.active_dialog.borrow().is_some() {
                        return Ok(false);
                    }
                    return_dialog(&ui, &pair)?;
                    let d = qa_dialog(&ui)?;
                    qa_common(&d, "sales-return", "2026-03-30")?;
                    qa_text(&d, "sales-return-quantity", "3")?;
                    qa_text(&d, "sales-return-reference", "RET-M5-B")?;
                    qa_text(&d, "sales-return-credit-reference", "CN-M5-B")?;
                    qa_text(&d, "sales-return-credit", "60")?;
                    qa_text(&d, "sales-return-cost", "35")?;
                    qa_click(&d, "sales-preview")?;
                    stage = 16;
                }
                16 => {
                    let d = qa_dialog(&ui)?;
                    let b = qa_widget::<gtk::Button>(&d, "sales-return-save")?;
                    if !b.is_sensitive() {
                        let error = qa_widget::<gtk::Label>(&d, "accounting-form-error")?;
                        anyhow::ensure!(
                            !error.is_visible(),
                            "native paired preview failed: {}",
                            error.text()
                        );
                        return Ok(false);
                    }
                    if !qa_snapshot(&directory, "M5-final-return-preview.png")? {
                        return Ok(false);
                    }
                    b.emit_clicked();
                    stage = 17;
                }
                17 => {
                    if ui.active_dialog.borrow().is_some() {
                        return Ok(false);
                    }
                    checks.push("split original-cost return23+35 and exact credit40+60".into());
                    let remains = ui
                        .store
                        .borrow()
                        .state
                        .accounting
                        .sales_order_remaining(&order)?;
                    anyhow::ensure!(
                        remains[0].remaining_quantity_thousandths == 5000,
                        "returns must not reopen order"
                    );
                    fulfill_dialog(&ui, &order)?;
                    let d = qa_dialog(&ui)?;
                    qa_common(&d, "sales-fulfill", "2026-03-31")?;
                    qa_date(&d, "sales-fulfill-due-date", "2026-04-25")?;
                    qa_text(&d, "sales-fulfill-quantity", "5")?;
                    qa_text(&d, "sales-fulfill-delivery-reference", "DEL-M5-B")?;
                    qa_text(&d, "sales-fulfill-invoice-reference", "INV-M5-B")?;
                    qa_text(&d, "sales-fulfill-total", "99")?;
                    qa_click(&d, "sales-preview")?;
                    stage = 18;
                }
                18 => {
                    let d = qa_dialog(&ui)?;
                    let b = qa_widget::<gtk::Button>(&d, "sales-fulfill-save")?;
                    if !b.is_sensitive() {
                        let error = qa_widget::<gtk::Label>(&d, "accounting-form-error")?;
                        anyhow::ensure!(
                            !error.is_visible(),
                            "native paired preview failed: {}",
                            error.text()
                        );
                        return Ok(false);
                    }
                    if !qa_snapshot(&directory, "M5-final-pair-preview.png")? {
                        return Ok(false);
                    }
                    b.emit_clicked();
                    stage = 19;
                }
                19 => {
                    if ui.active_dialog.borrow().is_some() {
                        return Ok(false);
                    }
                    checks.push("final fulfillment original discount residual99 and cost58".into());
                    ui.stack.set_visible_child_name("order");
                    ui.new_draft();
                    let company = page.company_id()?;
                    let resources = ui
                        .store
                        .borrow()
                        .state
                        .accounting
                        .procurement_resources(&company)?;
                    ui.customer.set_text(
                        &resources
                            .parties
                            .iter()
                            .find(|p| p.customer)
                            .ok_or_else(|| anyhow::anyhow!("fixture customer absent"))?
                            .name,
                    );
                    ui.reference.set_text("Q-M5");
                    ui.quote_date.set_canonical("2026-03-31")?;
                    ui.valid_until.set_canonical("2026-04-25")?;
                    ui.unit.set_selected(1);
                    ui.discount.set_text("0");
                    ui.new_item.set_text("SKU-A");
                    ui.new_quantity.set_text("1");
                    ui.add_line();
                    stage = 20;
                }
                20 => {
                    if ui.visual_pending.get() {
                        return Ok(false);
                    }
                    if !qa_snapshot(&directory, "M5-authentic-quotation.png")? {
                        return Ok(false);
                    }
                    // Review the actual resolved source before acknowledging its age.
                    // Adding a line deliberately invalidates earlier acknowledgment.
                    ui.stale.set_active(true);
                    ui.approve();
                    anyhow::ensure!(
                        !ui.store.borrow().state.quotes.is_empty(),
                        "actual native quotation approval failed: {}",
                        ui.status.text()
                    );
                    ui.stack.set_visible_child_name("sales");
                    page.from_quote();
                    stage = 21;
                }
                21 => {
                    let d = qa_dialog(&ui)?;
                    qa_click(&d, "sales-quotation-save")?;
                    stage = 22;
                }
                22 => {
                    let e = qa_editor(&ui)?;
                    anyhow::ensure!(
                        e.base.borrow().quotation.is_some(),
                        "authentic retained quote link absent"
                    );
                    e.reference.set_text("SO-Q-M5");
                    e.reason.set_text("تبدیل پیشنهاد تأییدشدهٔ ساختگی");
                    qa_date(&e.dialog, "sales-order-date", "2026-03-31")?;
                    e.total.set_text("20");
                    line_dialog(&e, Some(0));
                    stage = 23;
                }
                23 => {
                    let d = qa_dialog(&ui)?;
                    qa_widget::<gtk::DropDown>(&d, "sales-line-warehouse")?.set_selected(0);
                    qa_click(&d, "sales-order-line-save")?;
                    stage = 24;
                }
                24 => {
                    let e = qa_editor(&ui)?;
                    e.check();
                    stage = 25;
                }
                25 => {
                    let e = qa_editor(&ui)?;
                    if !e.post.is_sensitive() {
                        anyhow::ensure!(
                            !e.error.is_visible(),
                            "quote order preview: {}",
                            e.error.text()
                        );
                        return Ok(false);
                    }
                    if !qa_snapshot(&directory, "M5-converted-quotation-preview.png")? {
                        return Ok(false);
                    }
                    e.submit();
                    stage = 26;
                }
                26 => {
                    if ui.active_dialog.borrow().is_some() {
                        return Ok(false);
                    }
                    checks.push("authentic approved quotation converted through native selector/mapping without GL or original quote edits".into());
                    stage = 90;
                }
                90 => {
                    page.as_of.set_canonical("2026-03-31")?;
                    page.refresh();
                    stage = 91;
                }
                91 => {
                    if !qa_snapshot(&directory, "M5-final-controls.png")? {
                        return Ok(false);
                    }
                    if mode == "posted" {
                        order = qa_record(&ui, "Order")
                            .ok_or_else(|| anyhow::anyhow!("posted source absent"))?;
                    }
                    let s = ui.store.borrow().state.clone();
                    let company = page.company_id()?;
                    let report = s.accounting.sales_reconcile(&company, "2026-03-31")?;
                    anyhow::ensure!(report.reconciled, "sales control reconciliation mismatch");
                    for pdf in [false, true] {
                        let path = directory.join(if pdf {
                            "M5-controls.pdf"
                        } else {
                            "M5-controls.csv"
                        });
                        if pdf {
                            sarv_quote::sales_export::reconciliation_pdf(
                                &s.accounting,
                                &company,
                                "2026-03-31",
                                MoneyUnit::Rial,
                                s.settings.date_display,
                                s.settings.display_digits,
                                &path,
                            )?;
                        } else {
                            sarv_quote::sales_export::reconciliation_csv(
                                &s.accounting,
                                &company,
                                "2026-03-31",
                                MoneyUnit::Rial,
                                s.settings.date_display,
                                s.settings.display_digits,
                                &path,
                            )?;
                        }
                    }
                    cancelled_source = Some(serde_json::to_vec(&s.accounting)?);
                    open_document(&ui, &order);
                    anyhow::ensure!(
                        ui.background_busy.get(),
                        "source reader must be pending before cancellation"
                    );
                    qa_dialog(&ui)?.close();
                    stage = 92;
                }
                92 => {
                    let s = ui.store.borrow().state.clone();
                    anyhow::ensure!(
                        cancelled_source.as_ref() == Some(&serde_json::to_vec(&s.accounting)?),
                        "pending source cancellation changed books"
                    );
                    anyhow::ensure!(
                        !ui.status.text().contains("خواندن منبع فروش…"),
                        "closed source reader stranded global loading status"
                    );
                    checks.push("cancelled pending native source read preserves books and clears global loading status".into());
                    let focused_card = &page.cards[0].open;
                    anyhow::ensure!(
                        focused_card.is_mapped() && focused_card.is_sensitive(),
                        "current native card must be usable before invalidation"
                    );
                    focused_card.grab_focus();
                    anyhow::ensure!(
                        gtk::prelude::GtkWindowExt::focus(&ui.window)
                            .is_some_and(|focus| focus == *focused_card),
                        "actual window did not focus current card button"
                    );
                    page.refresh();
                    anyhow::ensure!(
                        gtk::prelude::GtkWindowExt::focus(&ui.window)
                            .is_some_and(|focus| focus.is_mapped()
                                && focus.is_sensitive()
                                && (focus == page.search || focus.is_ancestor(&page.search))),
                        "hiding persistent focused card did not hand focus to live search"
                    );
                    checks.push("actual window focus on a persistent card moves to live Sales search before row invalidation hides and disables that card".into());
                    anyhow::ensure!(
                        ui.background_busy.get(),
                        "page reader must be pending before changing its cutoff"
                    );
                    let old_revision = page.revision.get();
                    page.as_of.entry.set_text("INVALID-M5-PENDING-CUTOFF");
                    anyhow::ensure!(
                        page.revision.get() > old_revision,
                        "queued cutoff request did not invalidate its older reader"
                    );
                    stage = 93;
                }
                93 => {
                    anyhow::ensure!(
                        page.snapshot.borrow().is_none()
                            && page.cards.iter().all(|slot| slot.current.borrow().is_none()
                                && !slot.card.is_visible()
                                && !slot.open.is_sensitive()),
                        "older reader republished usable slots after invalid cutoff request"
                    );
                    anyhow::ensure!(
                        cancelled_source.as_ref()
                            == Some(&serde_json::to_vec(&ui.store.borrow().state.accounting)?),
                        "cancelled page cutoff changed books"
                    );
                    checks.push("invalid cutoff requested while actual page reader is pending rejects older completion and clears all usable slot identities without writes".into());
                    page.as_of.set_canonical("2026-03-31")?;
                    page.refresh();
                    stage = 94;
                }
                94 => {
                    if page.snapshot.borrow().is_none() {
                        return Ok(false);
                    }
                    anyhow::ensure!(
                        gtk::prelude::GtkWindowExt::focus(&ui.window)
                            .is_some_and(|focus| focus.is_mapped() && focus.is_sensitive()),
                        "collapsed invalid cutoff left hidden native focus after reader completed"
                    );
                    if mode == "full" {
                        retired_page = Some(Rc::downgrade(&page));
                        restored_company = page.company_id()?;
                        let backup = directory.join("M5-lifecycle-unchanged-backup.json");
                        ui.store.borrow().backup(&backup)?;
                        ui.restore(&backup);
                        stage = 95;
                    } else {
                        stage = 98;
                    }
                }
                95 => {
                    let dialog = qa_dialog(&ui)?;
                    anyhow::ensure!(
                        dialog.title().as_str() == "بررسی بازیابی",
                        "actual backup preview did not present restore dialog"
                    );
                    qa_button(&dialog, "بازیابی و ذخیرهٔ نسخهٔ ایمنی")
                        .ok_or_else(|| anyhow::anyhow!("native restore confirmation absent"))?
                        .emit_clicked();
                    stage = 96;
                }
                96 => {
                    if ui.active_dialog.borrow().is_some() || ui.visible_dialog().is_some() {
                        return Ok(false);
                    }
                    anyhow::ensure!(
                        retired_page
                            .as_ref()
                            .is_some_and(|old| old.upgrade().is_none()),
                        "retired Sales page remains owned after actual restore/rebuild"
                    );
                    anyhow::ensure!(
                        page.current_ui().is_some() && page.company_id()? == restored_company,
                        "shortcut lookup did not select actual attached restored Sales page"
                    );
                    anyhow::ensure!(
                        cancelled_source.as_ref()
                            == Some(&serde_json::to_vec(&ui.store.borrow().state.accounting)?),
                        "actual restore changed posted snapshot"
                    );
                    ui.stack.set_visible_child_name("sales");
                    page.as_of.set_canonical("2026-04-01")?;
                    page.refresh();
                    stage = 97;
                }
                97 => {
                    if page.snapshot.borrow().is_none() {
                        return Ok(false);
                    }
                    new_order(&ui);
                    let editor = qa_editor(&ui)?;
                    anyhow::ensure!(
                        editor.base.borrow().company_id == restored_company
                            && date_value(&editor.financial_date)? == "2026-04-01"
                            && date_value(&editor.document_date)? == "2026-04-01",
                        "new order did not bind restored current company and cutoff"
                    );
                    editor.dialog.close();
                    checks.push("actual backup preview and confirmed restore rebuild Sales, retire the old Weak page, and route a new native draft to the attached current company and changed 2026-04-01 cutoff with posted books unchanged".into());
                    stage = 98;
                }
                98 => {
                    if ui.active_dialog.borrow().is_some() || ui.visible_dialog().is_some() {
                        return Ok(false);
                    }
                    let s = ui.store.borrow().state.clone();
                    anyhow::ensure!(
                        cancelled_source.as_ref() == Some(&serde_json::to_vec(&s.accounting)?),
                        "native lifecycle proof changed posted books"
                    );
                    let manifest = serde_json::json!({"ready":true,"synthetic_only":true,"mode":mode,"checks":checks,"order_id":order,"first_pair_id":pair,"record_count":s.accounting.sales.records.len(),"source":"native dialogs, real catalog picker, raw draft State worker, approved quotation conversion","search_name":"sales-search","restored_company":restored_company,"restored_cutoff":if mode == "full" {"2026-04-01"} else {"2026-03-31"},"note":"no further smoke edits; external observer owns keyboard/layout/raw-state checks"});
                    std::fs::write(
                        directory.join("sales-native-verification.json"),
                        serde_json::to_vec_pretty(&manifest)?,
                    )?;
                    return Ok(true);
                }
                _ => anyhow::bail!("unknown native sales stage {stage}"),
            };
            Ok(false)
        })();
        match result {
            Ok(true) => glib::ControlFlow::Break,
            Ok(false) => glib::ControlFlow::Continue,
            Err(error) => {
                let _ = std::fs::write(
                    directory.join("sales-native-failure.txt"),
                    format!("stage {stage}: {error:#}"),
                );
                ui.message(Err::<String, _>(error));
                glib::ControlFlow::Break
            }
        }
    });
}
