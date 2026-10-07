mod background;
mod catalog_preview;
mod components;
mod editor;
mod managers;
mod settings;
mod smoke;
use adw::prelude::*;
use anyhow::Result;
use components::*;
use gtk::{gio, glib};
use sarv_quote::{
    domain::*,
    export, imports,
    store::{Store, read_import},
};
use std::{
    cell::{Cell, RefCell},
    path::{Path, PathBuf},
    rc::Rc,
};

struct Ui {
    store: RefCell<Store>,
    window: gtk::ApplicationWindow,
    stack: gtk::Stack,
    split: adw::NavigationSplitView,
    status: gtk::Label,
    customer: gtk::Entry,
    phone: gtk::Entry,
    order: gtk::TextView,
    discount: gtk::Entry,
    unit: gtk::DropDown,
    stale: gtk::CheckButton,
    catalog_choice: gtk::DropDown,
    review: gtk::Box,
    total: gtk::Label,
    subtotal: gtk::Label,
    tax: gtk::Label,
    document_state: gtk::Label,
    source_hint: gtk::Label,
    reference: gtk::Entry,
    quote_date: gtk::Entry,
    valid_until: gtk::Entry,
    notes: gtk::TextView,
    new_item: gtk::Entry,
    new_quantity: gtk::Entry,
    undo_button: gtk::Button,
    row_editors: RefCell<Vec<editor::RowEditor>>,
    table_dirty: Cell<bool>,
    visual_pending: Cell<bool>,
    deleted_lines: RefCell<Option<(usize, OrderLine)>>,
    catalog_list: gtk::Box,
    customer_list: gtk::Box,
    quote_list: gtk::Box,
    search_catalog: gtk::SearchEntry,
    search_customer: gtk::SearchEntry,
    search_quote: gtk::SearchEntry,
    loading: Cell<bool>,
    displayed_unit: Cell<Unit>,
    pending: RefCell<Option<glib::SourceId>>,
    active_dialog: RefCell<Option<adw::Dialog>>,
    background_busy: Cell<bool>,
}
fn csv_order(lines: &[OrderLine]) -> String {
    let mut w = csv::Writer::from_writer(vec![]);
    let _ = w.write_record(["item", "quantity"]);
    for l in lines {
        let _ = w.write_record([
            l.query.clone(),
            format!("{}.{:03}", l.quantity_milli / 1000, l.quantity_milli % 1000),
        ]);
    }
    String::from_utf8(w.into_inner().unwrap_or_default()).unwrap_or_default()
}
fn choose_file(
    window: &gtk::ApplicationWindow,
    save: bool,
    name: &str,
    callback: impl Fn(PathBuf) + 'static,
) {
    let d = gtk::FileDialog::builder()
        .title(if save {
            "ذخیرهٔ فایل با نام تازه"
        } else {
            "انتخاب فایل"
        })
        .build();
    if save {
        d.set_initial_name(Some(name));
        d.save(Some(window), None::<&gio::Cancellable>, move |r| {
            if let Ok(f) = r
                && let Some(p) = f.path()
            {
                callback(p);
            }
        });
    } else {
        d.open(Some(window), None::<&gio::Cancellable>, move |r| {
            if let Ok(f) = r
                && let Some(p) = f.path()
            {
                callback(p);
            }
        });
    }
}
fn dialog_toolbar(content: &impl IsA<gtk::Widget>) -> adw::ToolbarView {
    let view = adw::ToolbarView::new();
    view.add_top_bar(&adw::HeaderBar::new());
    view.set_content(Some(content));
    view
}
fn activate_named_button(widget: &impl IsA<gtk::Widget>, name: &str) -> bool {
    if let Some(button) = widget.as_ref().downcast_ref::<gtk::Button>()
        && button.widget_name() == name
    {
        button.emit_clicked();
        return true;
    }
    let mut child = widget.as_ref().first_child();
    while let Some(w) = child {
        child = w.next_sibling();
        if activate_named_button(&w, name) {
            return true;
        }
    }
    false
}
fn document_preview(paths: Vec<PathBuf>) -> adw::ToolbarView {
    let view = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    let zoom = gtk::DropDown::from_strings(&["۷۵٪", "۱۰۰٪", "۱۲۵٪"]);
    zoom.set_selected(1);
    zoom.set_tooltip_text(Some("بزرگ‌نمایی پیش‌نمایش"));
    header.pack_end(&zoom);
    view.add_top_bar(&header);
    let pages = column();
    pages.set_margin_top(12);
    pages.set_margin_bottom(12);
    pages.set_spacing(16);
    let mut pictures = vec![];
    for path in paths {
        let picture = gtk::Picture::for_filename(path);
        picture.set_can_shrink(true);
        picture.set_size_request(700, 990);
        picture.set_valign(gtk::Align::Start);
        picture.set_vexpand(false);
        pages.append(&picture);
        pictures.push(picture);
    }
    zoom.connect_selected_notify(move |w| {
        let width = match w.selected() {
            0 => 525,
            2 => 875,
            _ => 700,
        };
        for p in &pictures {
            p.set_size_request(width, width * 842 / 595);
        }
    });
    let scrolled = scroll(&pages);
    scrolled.set_hscrollbar_policy(gtk::PolicyType::Automatic);
    view.set_content(Some(&scrolled));
    view
}
impl Ui {
    fn message(&self, r: Result<impl ToString>) {
        match r {
            Ok(s) => {
                self.status.remove_css_class("error");
                self.status.set_text(&s.to_string());
            }
            Err(e) => {
                self.status.add_css_class("error");
                self.status.set_text(&format!("{e:#}"));
            }
        }
    }
    fn save_form(&self) -> Result<()> {
        let text = self.order.buffer().text(
            &self.order.buffer().start_iter(),
            &self.order.buffer().end_iter(),
            true,
        );
        let lines = if self.table_dirty.get() {
            self.row_editors
                .borrow()
                .iter()
                .map(|r| {
                    Ok(OrderLine {
                        query: r.query.text().to_string(),
                        quantity_milli: quantity(&r.quantity.text())?,
                        chosen_code: None,
                    })
                })
                .collect::<Result<Vec<_>>>()?
        } else if text.trim() == "item,quantity" || text.trim().is_empty() {
            vec![]
        } else {
            imports::order_csv(&text)?
        };
        let unit = if self.unit.selected() == 0 {
            Unit::Toman
        } else {
            Unit::Rial
        };
        let discount_text = self.discount.text();
        let discount = unit.nonnegative(if discount_text.trim().is_empty() {
            "0"
        } else {
            &discount_text
        })?;
        let name = self.customer.text().to_string();
        let phone = self.phone.text().to_string();
        let ack = self.stale.is_active();
        let index = self.catalog_choice.selected() as usize;
        let reference = self.reference.text().to_string();
        let quote_date = self.quote_date.text().to_string();
        let valid_until = self.valid_until.text().to_string();
        let notes = self
            .notes
            .buffer()
            .text(
                &self.notes.buffer().start_iter(),
                &self.notes.buffer().end_iter(),
                true,
            )
            .to_string();
        self.store.borrow_mut().change(|s| {
            let catalog = s
                .catalogs
                .get(index)
                .map(|c| c.id.clone())
                .unwrap_or_default();
            let same_catalog = catalog == s.draft.catalog_id;
            let same_sources = same_catalog
                && lines.len() == s.draft.lines.len()
                && lines
                    .iter()
                    .zip(&s.draft.lines)
                    .all(|(a, b)| a.query == b.query);
            let mut lines = lines;
            if same_catalog {
                for (i, l) in lines.iter_mut().enumerate() {
                    if let Some(old) = s.draft.lines.get(i)
                        && old.query == l.query
                    {
                        l.chosen_code = old.chosen_code.clone();
                    }
                }
            }
            s.draft.catalog_id = catalog;
            s.draft.lines = lines;
            s.draft.customer = name;
            s.draft.phone = phone;
            s.draft.discount_rial = discount;
            s.draft.display_unit = unit;
            s.draft.stale_ack = ack && same_sources;
            s.draft.reference = reference;
            s.draft.quote_date = quote_date;
            s.draft.valid_until = valid_until;
            s.draft.notes = notes;
            Ok(())
        })?;
        self.loading.set(true);
        self.order
            .buffer()
            .set_text(&csv_order(&self.store.borrow().state.draft.lines));
        self.stale
            .set_active(self.store.borrow().state.draft.stale_ack);
        self.loading.set(false);
        self.table_dirty.set(false);
        Ok(())
    }
    fn schedule(self: &Rc<Self>) {
        if self.loading.get() {
            return;
        }
        self.status.set_text("تغییرات ذخیره‌نشده…");
        self.document_state.set_text("در حال ویرایش");
        if let Some(p) = self.pending.borrow_mut().take() {
            p.remove();
        }
        let weak = Rc::downgrade(self);
        *self.pending.borrow_mut() = Some(glib::timeout_add_local_once(
            std::time::Duration::from_millis(500),
            move || {
                if let Some(ui) = weak.upgrade() {
                    ui.pending.borrow_mut().take();
                    let r = ui.save_form();
                    if r.is_ok() {
                        ui.refresh_totals();
                        if !ui.table_has_focus() {
                            ui.refresh_review();
                        }
                        ui.message(Ok("پیش‌نویس ذخیره شد • آفلاین"));
                    } else {
                        ui.message(r.map(|()| ""));
                    }
                }
            },
        ));
    }
    fn fill_form(&self) {
        self.loading.set(true);
        let store = self.store.borrow();
        let s = &store.state;
        let d = &s.draft;
        let names = s
            .catalogs
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>();
        self.catalog_choice
            .set_model(Some(&gtk::StringList::new(&names)));
        self.catalog_choice.set_selected(
            s.catalogs
                .iter()
                .position(|c| c.id == d.catalog_id)
                .map(|i| i as u32)
                .unwrap_or(gtk::INVALID_LIST_POSITION),
        );
        self.customer.set_text(&d.customer);
        self.phone.set_text(&d.phone);
        self.order.buffer().set_text(&csv_order(&d.lines));
        self.unit
            .set_selected(if d.display_unit == Unit::Toman { 0 } else { 1 });
        self.displayed_unit.set(d.display_unit);
        self.discount
            .set_text(&d.display_unit.number(d.discount_rial));
        self.stale.set_active(d.stale_ack);
        self.reference.set_text(&d.reference);
        self.quote_date.set_text(&d.quote_date);
        self.valid_until.set_text(&d.valid_until);
        self.notes.buffer().set_text(&d.notes);
        self.table_dirty.set(false);
        drop(store);
        self.loading.set(false);
    }
    fn refresh_lists(self: &Rc<Self>) {
        self.refresh_managers();
        settings::apply(self);
    }
    fn refresh_settings(self: &Rc<Self>) {
        if let Some(old) = self.stack.child_by_name("settings") {
            self.stack.remove(&old);
        }
        self.stack
            .add_named(&settings::build(self), Some("settings"));
        settings::apply(self);
    }
    fn approve(self: &Rc<Self>) {
        if let Err(e) = self.save_form() {
            self.message(Err::<String, _>(e));
            return;
        }
        if self.visual_pending.get() {
            self.refresh_review();
            self.message(Ok(
                "اقلام به‌روز شدند؛ قیمت و شرح را بررسی و دوباره تأیید کنید",
            ));
            return;
        }
        let result = self.store.borrow_mut().change(State::approve);
        self.message(result.map(|id| format!("پیشنهاد تأیید و ذخیره شد: {id}")));
        self.refresh_review();
        self.refresh_lists();
    }
    fn preserve_draft(&self, reason: &str) -> Result<()> {
        let store = self.store.borrow();
        let draft = &store.state.draft;
        if !draft.lines.is_empty()
            || !draft.customer.trim().is_empty()
            || !draft.phone.trim().is_empty()
            || !draft.reference.trim().is_empty()
            || !draft.notes.trim().is_empty()
        {
            store.backup(
                &store
                    .directory
                    .join(format!("before-{reason}-{}.json", id())),
            )?;
        }
        Ok(())
    }
    fn present_dialog(self: &Rc<Self>, dialog: &adw::Dialog) {
        *self.active_dialog.borrow_mut() = Some(dialog.clone());
        let weak = Rc::downgrade(self);
        dialog.connect_closed(move |d| {
            d.set_child(None::<&gtk::Widget>);
            if let Some(ui) = weak.upgrade() {
                let current = ui.active_dialog.borrow().as_ref() == Some(d);
                if current {
                    ui.active_dialog.borrow_mut().take();
                }
            }
        });
        dialog.present(Some(&self.window));
    }
    fn new_draft(self: &Rc<Self>) {
        let r = self
            .save_form()
            .and_then(|()| self.preserve_draft("new-draft"))
            .and_then(|()| self.store.borrow_mut().change(State::new_draft));
        let success = r.is_ok();
        self.message(r.map(|()| "پیش‌نویس تازه"));
        if !success {
            return;
        }
        self.deleted_lines.borrow_mut().take();
        self.undo_button.set_sensitive(false);
        self.fill_form();
        self.refresh_review();
        self.stack.set_visible_child_name("order");
        self.split.set_show_content(true);
        self.customer.grab_focus();
    }
    fn import_catalog(self: &Rc<Self>, path: &Path) {
        let result = read_import(path).and_then(|s| {
            imports::catalog_csv(
                &s,
                path.file_stem().and_then(|s| s.to_str()).unwrap_or("فهرست"),
            )
        });
        match result {
            Err(e) => self.message(Err::<String, _>(e)),
            Ok(c) => {
                let dialog = adw::Dialog::builder()
                    .title("پیش‌نمایش فهرست • تأیید منبع قیمت")
                    .content_width(760)
                    .content_height(650)
                    .build();
                let root = column();
                root.set_margin_top(24);
                root.set_margin_bottom(24);
                root.set_margin_start(24);
                root.set_margin_end(24);
                root.append(&heading("پیش‌نمایش قیمت‌ها قبل از ورود"));
                root.append(&label("فقط در صورت تأیید منبع، واحد و نام‌های مستعار ادامه دهید. قیمت‌ها ساخته یا جایگزین نمی‌شوند."));
                let name = entry("نام فهرست");
                name.set_text(&c.name);
                root.append(&field("نام فهرست", &name));
                root.append(&catalog_preview::preview(&c));
                let ok = button("منبع و قیمت‌ها را تأیید می‌کنم");
                ok.add_css_class("suggested-action");
                root.append(&ok);
                let weak = Rc::downgrade(self);
                let d = dialog.clone();
                ok.connect_clicked(move |_| {
                    if let Some(ui) = weak.upgrade() {
                        let mut c = c.clone();
                        c.name = name.text().to_string();
                        let r = ui.save_form().and_then(|()| {
                            ui.store
                                .borrow_mut()
                                .change(|s| imports::accept_catalog(s, c))
                        });
                        let success = r.is_ok();
                        ui.message(r.map(|_| "فهرست تأیید و ذخیره شد"));
                        if success {
                            d.close();
                            ui.fill_form();
                            ui.refresh_review();
                            ui.refresh_lists();
                        }
                    }
                });
                dialog.set_child(Some(&dialog_toolbar(&root)));
                self.present_dialog(&dialog);
            }
        }
    }
    fn restore(self: &Rc<Self>, path: &Path) {
        match Store::restore_preview(path) {
            Err(e) => self.message(Err::<String, _>(e)),
            Ok(candidate) => {
                let d = adw::Dialog::builder()
                    .title("بررسی بازیابی")
                    .content_width(550)
                    .content_height(300)
                    .build();
                let b = card();
                b.append(&heading("بازیابی دادهٔ تأییدشده"));
                b.append(&label(&format!("{} فهرست • {} مشتری • {} پیشنهاد\nدادهٔ جاری با این پشتیبان جایگزین می‌شود. قبل از جایگزینی، نسخهٔ ایمنی خودکار ذخیره می‌شود.",fa(candidate.catalogs.len()),fa(candidate.customers.len()),fa(candidate.quotes.len()))));
                let ok = button("بازیابی و ذخیرهٔ نسخهٔ ایمنی");
                b.append(&ok);
                let weak = Rc::downgrade(self);
                let dialog = d.clone();
                ok.connect_clicked(move |_| {
                    if let Some(ui) = weak.upgrade() {
                        if let Some(p) = ui.pending.borrow_mut().take() {
                            p.remove();
                        }
                        let r = ui
                            .save_form()
                            .and_then(|()| ui.store.borrow_mut().restore(candidate.clone()));
                        let success = r.is_ok();
                        ui.message(
                            r.map(|p| format!("بازیابی انجام شد؛ نسخهٔ ایمنی: {}", p.display())),
                        );
                        if success {
                            ui.deleted_lines.borrow_mut().take();
                            ui.undo_button.set_sensitive(false);
                            dialog.close();
                            ui.refresh_settings();
                            ui.fill_form();
                            ui.refresh_review();
                            ui.refresh_lists();
                        }
                    }
                });
                d.set_child(Some(&dialog_toolbar(&b)));
                self.present_dialog(&d);
            }
        }
    }
}
pub fn run() -> Result<()> {
    let data = std::env::var_os("SARV_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data"));
    let mut opened = Store::open(&data)?;
    if let Some(path) = std::env::var_os("SARV_SEED_BACKUP") {
        anyhow::ensure!(
            opened.state.quotes.is_empty()
                && opened.state.catalogs.is_empty()
                && opened.state.customers.is_empty()
                && opened.state.draft.lines.is_empty(),
            "test seed requires an empty profile"
        );
        opened.restore(Store::restore_preview(Path::new(&path))?)?;
    }
    let store = Rc::new(RefCell::new(Some(opened)));
    let application_id = if std::env::var_os("SARV_TEST_INSTANCE").is_some() {
        "ir.sarv.quote.verification"
    } else {
        "ir.sarv.quote.v1"
    };
    let app = adw::Application::builder()
        .application_id(application_id)
        .build();
    app.connect_activate(move |app| {
        if let Some(store) = store.borrow_mut().take() {
            build(app.upcast_ref(), store);
        } else if let Some(w) = app.active_window() {
            w.present();
        }
    });
    app.run_with_args::<&str>(&[]);
    Ok(())
}
fn build(app: &gtk::Application, store: Store) {
    gtk::Widget::set_default_direction(gtk::TextDirection::Rtl);
    let css = gtk::CssProvider::new();
    css.load_from_data(include_str!("style.css"));
    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().expect("display"),
        &css,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    let adw_window = adw::ApplicationWindow::builder()
        .application(app)
        .title("سرو | آماده‌سازی پیشنهاد قیمت")
        .default_width(1400)
        .default_height(940)
        .build();
    let window = adw_window.clone().upcast::<gtk::ApplicationWindow>();
    let root = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&adw::WindowTitle::new("سرو", "پیشنهاد قیمت • آفلاین")));
    let navigation = gtk::Button::with_label("فهرست");
    navigation.set_tooltip_text(Some("میز کار"));
    header.pack_start(&navigation);
    root.add_top_bar(&header);
    let body = adw::NavigationSplitView::new();
    body.set_min_sidebar_width(190.0);
    body.set_max_sidebar_width(220.0);
    body.set_sidebar_width_fraction(0.16);
    body.set_show_content(true);
    let split = body.clone();
    navigation.connect_clicked(move |_| split.set_show_content(!split.shows_content()));
    navigation.set_visible(body.is_collapsed());
    let nav_toggle = navigation.clone();
    body.connect_collapsed_notify(move |split| nav_toggle.set_visible(split.is_collapsed()));
    let breakpoint = adw::Breakpoint::new(
        adw::BreakpointCondition::parse("max-width: 1050px").expect("constant breakpoint"),
    );
    breakpoint.add_setter(&body, "collapsed", Some(&true.to_value()));
    adw_window.add_breakpoint(breakpoint);
    let nav = column();
    nav.add_css_class("sidebar");
    nav.set_width_request(166);
    nav.append(&label("میز کار"));
    let stack = gtk::Stack::new();
    stack.set_hexpand(true);
    stack.set_vexpand(true);
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    body.set_sidebar(Some(&adw::NavigationPage::new(&nav, "میز کار")));
    body.set_content(Some(&adw::NavigationPage::new(&stack, "پیشنهاد قیمت")));
    root.set_content(Some(&body));
    let status = label("آماده • قیمت‌ها تنها از فهرست تأییدشده می‌آیند");
    status.add_css_class("status");
    root.add_bottom_bar(&status);
    let order_page = column();
    order_page.add_css_class("page");
    order_page.set_spacing(8);
    let actions = toolbar();
    actions.append(&page_title("پیشنهاد قیمت"));
    let document_state = status_label("پیش‌نویس", StatusTone::Neutral);
    actions.append(&document_state);
    let spacer = row();
    spacer.set_hexpand(true);
    actions.append(&spacer);
    let new = button("تازه");
    new.set_tooltip_text(Some("Ctrl+N"));
    let save = button("ذخیرهٔ پیش‌نویس");
    save.set_tooltip_text(Some("Ctrl+S"));
    let preview = button("پیش‌نمایش چاپ");
    let approve = button("تأیید نسخه");
    approve.add_css_class("suggested-action");
    approve.set_tooltip_text(Some("Ctrl+Enter • ثبت نسخهٔ محفوظ پیشنهاد"));
    for b in [&new, &save, &preview, &approve] {
        actions.append(b);
    }
    order_page.append(&actions);
    let metadata = gtk::Grid::new();
    metadata.set_column_spacing(18);
    metadata.set_row_spacing(10);
    metadata.set_column_homogeneous(true);
    metadata.add_css_class("metadata");
    let customer = entry("نام مشتری یا شرکت");
    let phone = entry("۰۹۱۲…");
    phone.set_direction(gtk::TextDirection::Ltr);
    let catalog_choice = gtk::DropDown::from_strings(&[]);
    catalog_choice.set_enable_search(true);
    let unit = gtk::DropDown::from_strings(&["تومان", "ریال"]);
    let reference = entry("شمارهٔ سفارش مشتری (اختیاری)");
    let quote_date = entry("YYYY-MM-DD");
    quote_date.set_direction(gtk::TextDirection::Ltr);
    let valid_until = entry("YYYY-MM-DD • اختیاری");
    valid_until.set_direction(gtk::TextDirection::Ltr);
    let customer_save = button("ذخیرهٔ مشتری");
    metadata.attach(&field("مشتری", &customer), 0, 0, 1, 1);
    metadata.attach(&field("تلفن", &phone), 1, 0, 1, 1);
    metadata.attach(&field("فهرست قیمت تأییدشده", &catalog_choice), 2, 0, 1, 1);
    metadata.attach(&field("واحد مبلغ", &unit), 3, 0, 1, 1);
    metadata.attach(&field("مرجع مشتری", &reference), 0, 1, 1, 1);
    metadata.attach(&field("تاریخ پیشنهاد • میلادی", &quote_date), 1, 1, 1, 1);
    metadata.attach(&field("اعتبار تا • میلادی", &valid_until), 2, 1, 1, 1);
    customer_save.set_valign(gtk::Align::End);
    metadata.attach(&customer_save, 3, 1, 1, 1);
    order_page.append(&metadata);
    let source_hint = label("");
    source_hint.add_css_class("muted");
    order_page.append(&source_hint);
    let items_bar = row();
    items_bar.append(&heading("اقلام پیشنهاد"));
    let spacer = row();
    spacer.set_hexpand(true);
    items_bar.append(&spacer);
    let undo_button = button("برگرداندن حذف");
    undo_button.set_sensitive(false);
    let import_order = button("ورود سفارش CSV");
    items_bar.append(&undo_button);
    items_bar.append(&import_order);
    order_page.append(&items_bar);
    let review = column();
    review.set_spacing(0);
    let table_scroll = scroll(&review);
    table_scroll.set_hscrollbar_policy(gtk::PolicyType::Automatic);
    table_scroll.set_min_content_height(180);
    table_scroll.add_css_class("table");
    order_page.append(&table_scroll);
    let add_bar = row();
    let new_item = entry("کد کالا یا نام مستعار تأییدشده…");
    let new_quantity = entry("تعداد");
    accessible_name(&new_item, "کد کالا یا نام مستعار برای ردیف تازه");
    accessible_name(&new_quantity, "تعداد ردیف تازه");
    new_quantity.set_text("۱");
    new_quantity.set_hexpand(false);
    new_quantity.set_width_chars(8);
    new_quantity.set_max_width_chars(10);
    new_quantity.set_direction(gtk::TextDirection::Ltr);
    let add_item = button("افزودن ردیف");
    add_bar.append(&new_item);
    add_bar.append(&new_quantity);
    add_bar.append(&add_item);
    order_page.append(&add_bar);
    let stale = gtk::CheckButton::with_label("منبع قیمت‌های قدیمی را بررسی و تأیید کرده‌ام");
    order_page.append(&stale);
    let summary = row();
    summary.add_css_class("quote-summary");
    summary.set_valign(gtk::Align::End);
    let remarks = column();
    remarks.set_hexpand(true);
    let notes = gtk::TextView::new();
    accessible_name(&notes, "یادداشت برای مشتری");
    notes.set_wrap_mode(gtk::WrapMode::WordChar);
    let note_scroll = scroll(&notes);
    note_scroll.set_height_request(55);
    note_scroll.set_vexpand(false);
    let note_expander = gtk::Expander::new(Some("یادداشت برای مشتری"));
    note_expander.set_child(Some(&note_scroll));
    remarks.append(&note_expander);
    let order = gtk::TextView::new();
    accessible_name(&order, "متن CSV سفارش • item,quantity");
    order.set_direction(gtk::TextDirection::Ltr);
    order.set_monospace(true);
    let order_scroll = scroll(&order);
    order_scroll.set_height_request(95);
    order_scroll.set_vexpand(false);
    let advanced = gtk::Expander::new(Some("ویرایش متن CSV سفارش"));
    advanced.set_child(Some(&order_scroll));
    remarks.append(&advanced);
    remarks.append(&label(
        "قیمت فقط از فهرست تأییدشده • این پیشنهاد سند حسابداری نیست",
    ));
    summary.append(&remarks);
    let totals = column();
    totals.set_width_request(290);
    totals.set_hexpand(false);
    let subtotal = numeric_cell("جمع اقلام —");
    totals.append(&subtotal);
    let discount = entry("۰");
    discount.set_direction(gtk::TextDirection::Ltr);
    let discount_row = row();
    discount_row.append(&label("تخفیف کل"));
    discount_row.append(&discount);
    totals.append(&discount_row);
    let tax = numeric_cell("");
    tax.set_visible(false);
    totals.append(&tax);
    let total = heading("جمع نهایی —");
    total.add_css_class("total");
    total.set_xalign(1.0);
    totals.append(&total);
    summary.append(&totals);
    order_page.append(&summary);
    stack.add_named(&order_page, Some("order"));
    let catalog_list = column();
    let customer_list = column();
    let quote_list = column();
    let search_catalog = gtk::SearchEntry::new();
    let search_customer = gtk::SearchEntry::new();
    let search_quote = gtk::SearchEntry::new();
    accessible_name(&search_catalog, "جست‌وجوی فهرست قیمت و کالا");
    accessible_name(&search_customer, "جست‌وجوی مشتری");
    accessible_name(&search_quote, "جست‌وجوی پیشنهاد و نسخه‌ها");
    let import_catalog = button("ورود فهرست قیمت و پیش‌نمایش");
    for (name, title, search, list) in [
        (
            "catalog",
            "فهرست‌های قیمت تأییدشده",
            &search_catalog,
            &catalog_list,
        ),
        (
            "customers",
            "مشتری‌های ذخیره‌شده",
            &search_customer,
            &customer_list,
        ),
        (
            "quotes",
            "بایگانی پیشنهادها و نسخه‌ها",
            &search_quote,
            &quote_list,
        ),
    ] {
        let p = column();
        p.add_css_class("page");
        p.append(&heading(title));
        p.append(search);
        if name == "catalog" {
            p.append(&import_catalog);
        }
        p.append(&scroll(list));
        stack.add_named(&p, Some(name));
    }
    let backup_page = column();
    backup_page.add_css_class("page");
    backup_page.append(&heading("پشتیبان و بازیابی"));
    backup_page.append(&label("داده در همین رایانه ذخیره می‌شود. پشتیبان شامل فهرست‌ها، مشتری‌ها، پیشنهادهای تأییدشده و پیش‌نویس است.\nبازیابی فقط پس از اعتبارسنجی قالب، اثر انگشت، مبلغ‌ها و ارتباط داده‌ها انجام می‌شود."));
    let backup = button("ساخت پشتیبان JSON");
    let restore = button("انتخاب پشتیبان و بررسی بازیابی");
    backup_page.append(&backup);
    backup_page.append(&restore);
    backup_page.append(&label(&format!("مسیر داده: {}\n\nمیانبرها: Ctrl+S ذخیره • Ctrl+Enter تأیید • Ctrl+N سفارش تازه\nAlt+1…5 جابه‌جایی صفحه • Tab پیمایش • Enter فعال‌سازی\n\nاین محصول پیشنهاد قیمت v1 است. موجودی، پرداخت، مالیات و سند حسابداری ثبت نمی‌کند.",store.directory.display())));
    stack.add_named(&backup_page, Some("backup"));
    for (i, name, caption) in [
        (1, "order", "پیشنهاد قیمت"),
        (2, "catalog", "فهرست قیمت"),
        (3, "customers", "مشتری‌ها"),
        (4, "quotes", "بایگانی پیشنهادها"),
        (5, "backup", "پشتیبان و بازیابی"),
        (6, "settings", "تنظیمات"),
    ] {
        let b = gtk::ToggleButton::with_label(caption);
        b.add_css_class("flat");
        b.set_tooltip_text(Some(&format!("Alt+{i}")));
        b.add_css_class("nav");
        let st = stack.clone();
        let split = body.clone();
        b.connect_clicked(move |b| {
            b.set_active(true);
            st.set_visible_child_name(name);
            split.set_show_content(true);
        });
        let nav_button = b.clone();
        stack.connect_visible_child_name_notify(move |st| {
            if st.visible_child_name().as_deref() == Some(name) {
                nav_button.set_active(true);
            } else {
                nav_button.set_active(false);
            }
        });
        if name == "order" {
            b.set_active(true);
        }
        nav.append(&b);
    }
    let space = column();
    space.set_vexpand(true);
    nav.append(&space);
    let local = label("ذخیرهٔ محلی");
    local.add_css_class("muted");
    nav.append(&local);
    let ui = Rc::new(Ui {
        store: RefCell::new(store),
        window: window.clone(),
        stack,
        split: body.clone(),
        status,
        customer,
        phone,
        order,
        discount,
        unit,
        stale,
        catalog_choice,
        review,
        total,
        subtotal,
        tax,
        document_state,
        source_hint,
        reference,
        quote_date,
        valid_until,
        notes,
        new_item,
        new_quantity,
        undo_button,
        row_editors: RefCell::new(vec![]),
        table_dirty: Cell::new(false),
        visual_pending: Cell::new(false),
        deleted_lines: RefCell::new(None),
        catalog_list,
        customer_list,
        quote_list,
        search_catalog,
        search_customer,
        search_quote,
        loading: Cell::new(false),
        displayed_unit: Cell::new(Unit::Toman),
        pending: RefCell::new(None),
        active_dialog: RefCell::new(None),
        background_busy: Cell::new(false),
    });
    ui.refresh_settings();
    ui.fill_form();
    ui.refresh_review();
    ui.refresh_lists();
    for e in [
        &ui.customer,
        &ui.phone,
        &ui.discount,
        &ui.reference,
        &ui.quote_date,
        &ui.valid_until,
    ] {
        let weak = Rc::downgrade(&ui);
        e.connect_changed(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.schedule();
            }
        });
    }
    let weak = Rc::downgrade(&ui);
    ui.order.buffer().connect_changed(move |_| {
        if let Some(ui) = weak.upgrade() {
            if !ui.loading.get() {
                ui.table_dirty.set(false);
                ui.visual_pending.set(true);
                ui.deleted_lines.borrow_mut().take();
                ui.undo_button.set_sensitive(false);
            }
            ui.schedule();
        }
    });
    {
        let dd = &ui.catalog_choice;
        let weak = Rc::downgrade(&ui);
        dd.connect_selected_notify(move |_| {
            if let Some(ui) = weak.upgrade() {
                if !ui.loading.get() {
                    ui.stale.set_active(false);
                    ui.visual_pending.set(true);
                    ui.deleted_lines.borrow_mut().take();
                    ui.undo_button.set_sensitive(false);
                }
                ui.schedule();
            }
        });
    }
    let weak = Rc::downgrade(&ui);
    ui.unit.connect_selected_notify(move |w| {
        if let Some(ui) = weak.upgrade() {
            if ui.loading.get() {
                return;
            }
            let next = if w.selected() == 0 {
                Unit::Toman
            } else {
                Unit::Rial
            };
            let current = ui.displayed_unit.get();
            let input = ui.discount.text();
            let amount = current.nonnegative(if input.trim().is_empty() { "0" } else { &input });
            ui.loading.set(true);
            match amount {
                Ok(rial) => {
                    ui.discount.set_text(&next.number(rial));
                    ui.displayed_unit.set(next);
                }
                Err(e) => {
                    w.set_selected(if current == Unit::Toman { 0 } else { 1 });
                    ui.message(Err::<String, _>(e));
                }
            }
            ui.loading.set(false);
            ui.schedule();
        }
    });
    let weak = Rc::downgrade(&ui);
    ui.stale.connect_toggled(move |_| {
        if let Some(ui) = weak.upgrade() {
            ui.schedule();
        }
    });
    for search in [&ui.search_catalog, &ui.search_customer, &ui.search_quote] {
        let weak = Rc::downgrade(&ui);
        search.connect_search_changed(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.refresh_lists();
            }
        });
    }
    let u = ui.clone();
    save.connect_clicked(move |_| {
        let r = u.save_form();
        let success = r.is_ok();
        u.message(r.map(|()| "پیش‌نویس ذخیره شد"));
        if success {
            u.refresh_review();
        }
    });
    let u = ui.clone();
    approve.connect_clicked(move |_| u.approve());
    let u = ui.clone();
    new.connect_clicked(move |_| u.new_draft());
    let u = ui.clone();
    customer_save.connect_clicked(move |_| {
        let r = u.save_form().and_then(|()| {
            u.store.borrow_mut().change(|s| {
                text(&s.draft.customer)?;
                if !s.customers.iter().any(|c| {
                    normalize(&c.name) == normalize(&s.draft.customer) && c.phone == s.draft.phone
                }) {
                    s.customers.push(Customer {
                        id: id(),
                        name: s.draft.customer.clone(),
                        phone: s.draft.phone.clone(),
                    });
                }
                Ok(())
            })
        });
        u.message(r.map(|()| "مشتری ذخیره شد"));
        u.refresh_lists();
    });
    let u = ui.clone();
    import_catalog.connect_clicked(move |_| {
        let u2 = u.clone();
        choose_file(&u.window, false, "", move |p| u2.import_catalog(&p));
    });
    let u = ui.clone();
    import_order.connect_clicked(move |_| {
        let u2 = u.clone();
        choose_file(&u.window, false, "", move |p| {
            let r = read_import(&p).and_then(|s| {
                imports::order_csv(&s)?;
                u2.deleted_lines.borrow_mut().take();
                u2.undo_button.set_sensitive(false);
                u2.order.buffer().set_text(&s);
                u2.save_form()
            });
            let success = r.is_ok();
            u2.message(r.map(|()| "سفارش جایگزین و ذخیره شد"));
            if success {
                u2.refresh_review();
            }
        });
    });
    let u = ui.clone();
    backup.connect_clicked(move |_| {
        if let Err(e) = u.save_form() {
            u.message(Err::<String, _>(e));
            return;
        }
        let u2 = u.clone();
        choose_file(
            &u.window,
            true,
            &format!(
                "sarv-backup-{}.json",
                chrono::Utc::now().format("%Y%m%d-%H%M%S")
            ),
            move |p| {
                u2.message(
                    u2.store
                        .borrow()
                        .backup(&p)
                        .map(|()| format!("پشتیبان ذخیره شد: {}", p.display())),
                )
            },
        );
    });
    let u = ui.clone();
    restore.connect_clicked(move |_| {
        let u2 = u.clone();
        choose_file(&u.window, false, "", move |p| u2.restore(&p));
    });
    let u = ui.clone();
    add_item.connect_clicked(move |_| u.add_line());
    let u = ui.clone();
    ui.new_quantity.connect_activate(move |_| u.add_line());
    let u = ui.clone();
    ui.new_item.connect_activate(move |_| {
        u.new_quantity.grab_focus();
    });
    let u = ui.clone();
    ui.undo_button.connect_clicked(move |_| u.undo_delete());
    let u = ui.clone();
    preview.connect_clicked(move |_| u.preview_document());
    let weak = Rc::downgrade(&ui);
    ui.notes.buffer().connect_changed(move |_| {
        if let Some(u) = weak.upgrade() {
            u.schedule();
        }
    });
    let keys = gtk::EventControllerKey::new();
    let u = ui.clone();
    keys.connect_key_pressed(move |_, key, _, mods| {
        if mods.contains(gtk::gdk::ModifierType::CONTROL_MASK) {
            match key {
                gtk::gdk::Key::Return => {
                    if u.stack.visible_child_name().as_deref() == Some("order")
                        && u.active_dialog.borrow().is_none()
                    {
                        u.approve();
                    } else {
                        return glib::Propagation::Proceed;
                    }
                }
                gtk::gdk::Key::s | gtk::gdk::Key::S => {
                    if u.stack.visible_child_name().as_deref() == Some("settings") {
                        if let Some(page) = u.stack.child_by_name("settings") {
                            activate_named_button(&page, "save-settings");
                        }
                        return glib::Propagation::Stop;
                    }
                    let r = u.save_form();
                    let success = r.is_ok();
                    u.message(r.map(|()| "ذخیره شد"));
                    if success {
                        u.refresh_review();
                    }
                }
                gtk::gdk::Key::n | gtk::gdk::Key::N => u.new_draft(),
                _ => return glib::Propagation::Proceed,
            }
            return glib::Propagation::Stop;
        }
        if mods.contains(gtk::gdk::ModifierType::ALT_MASK) {
            let page = match key {
                gtk::gdk::Key::_1 => "order",
                gtk::gdk::Key::_2 => "catalog",
                gtk::gdk::Key::_3 => "customers",
                gtk::gdk::Key::_4 => "quotes",
                gtk::gdk::Key::_5 => "backup",
                gtk::gdk::Key::_6 => "settings",
                _ => return glib::Propagation::Proceed,
            };
            u.stack.set_visible_child_name(page);
            u.split.set_show_content(true);
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    window.add_controller(keys);
    let u = ui.clone();
    window.connect_close_request(move |_| {
        if let Some(p) = u.pending.borrow_mut().take() {
            p.remove();
        }
        if let Err(e) = u.save_form() {
            u.message(Err::<String, _>(e));
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    adw_window.set_content(Some(&root));
    window.present();
    ui.customer.grab_focus();
    smoke::schedule(&ui, &approve);
}
