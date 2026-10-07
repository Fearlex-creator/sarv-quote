mod accounting;
mod background;
mod catalog_editor;
mod catalog_preview;
mod components;
mod core;
mod editor;
mod managers;
mod pickers;
mod responsiveness;
mod settings;
mod smoke;
mod workspace;
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

type SaveAction = Box<dyn FnOnce(&Rc<Ui>)>;

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
    review_cache: RefCell<Option<Rc<editor::PreparedReview>>>,
    review_requested: Cell<Option<i64>>,
    review_request_serial: Cell<u64>,
    total: gtk::Label,
    subtotal: gtk::Label,
    tax: gtk::Label,
    document_state: gtk::Label,
    approve_control: gtk::Button,
    preview_control: gtk::Button,
    source_hint: gtk::Label,
    reference: gtk::Entry,
    quote_date: accounting::DateField,
    valid_until: accounting::DateField,
    notes: gtk::TextView,
    new_item: gtk::Entry,
    new_quantity: gtk::Entry,
    new_product_choice: RefCell<Option<(String, String)>>,
    customer_details: gtk::Label,
    undo_button: gtk::Button,
    row_editors: RefCell<Vec<editor::RowEditor>>,
    table_page: Cell<usize>,
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
    persistence_busy: Cell<bool>,
    edit_revision: Cell<u64>,
    close_after_save: Cell<bool>,
    pending_save_action: RefCell<Option<SaveAction>>,
    nav_counts: Vec<(&'static str, gtk::Label)>,
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
fn quote_same_content(a: &Quote, b: &Quote) -> bool {
    let (Ok(mut a), Ok(mut b)) = (serde_json::to_value(a), serde_json::to_value(b)) else {
        return false;
    };
    for key in ["id", "version", "approved_at"] {
        if let Some(value) = a.as_object_mut() {
            value.remove(key);
        }
        if let Some(value) = b.as_object_mut() {
            value.remove(key);
        }
    }
    a == b
}

/// Native list rows provide one keyboard stop per destination. Selected state
/// follows the visible workspace, independently of pointer or keyboard focus.
fn sidebar_destination(
    name: &str,
    caption: &str,
    icon_name: &str,
    shortcut: &str,
    description: Option<&str>,
) -> (gtk::ListBoxRow, gtk::Label) {
    let item = gtk::ListBoxRow::new();
    item.set_widget_name(&format!("navigation-{name}"));
    item.add_css_class("sarv-navigation-row");
    item.set_tooltip_text(Some(&format!("{caption} · {shortcut}")));
    accessible_name(&item, caption);
    item.update_property(&[gtk::accessible::Property::KeyShortcuts(shortcut)]);
    let content = row();
    content.set_spacing(12);
    let icon = gtk::Image::from_icon_name(icon_name);
    icon.set_pixel_size(18);
    icon.add_css_class("navigation-icon");
    content.append(&icon);
    let text = column();
    text.set_spacing(2);
    text.set_hexpand(true);
    let title = label(caption);
    title.add_css_class("navigation-label");
    title.set_wrap(false);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    text.append(&title);
    if let Some(description) = description {
        let description = label(description);
        description.add_css_class("navigation-description");
        description.set_wrap(false);
        description.set_ellipsize(gtk::pango::EllipsizeMode::End);
        text.append(&description);
    }
    content.append(&text);
    let count = label("");
    count.add_css_class("navigation-count");
    count.add_css_class("numeric");
    count.set_wrap(false);
    count.set_visible(false);
    content.append(&count);
    item.set_child(Some(&content));
    (item, count)
}

impl Ui {
    fn set_quote_date(&self, field: &accounting::DateField, canonical: &str) {
        if field.set_canonical(canonical).is_err() {
            // A legacy date outside the Jalali range stays visible and editable
            // in its explicitly labelled Gregorian mode. Never drop saved data.
            field.entry.set_text("");
            field.calendar.set_selected(0);
            if let Err(error) = field.set_canonical(canonical) {
                self.message(Err::<String, _>(error));
            }
        }
    }
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
    fn form_draft(&self) -> Result<(Draft, bool)> {
        let text = self.order.buffer().text(
            &self.order.buffer().start_iter(),
            &self.order.buffer().end_iter(),
            true,
        );
        let lines = if self.table_dirty.get() {
            // Only the visible page owns widgets. Preserve all canonical rows,
            // applying validated edits by their stable global order index.
            let mut lines = self.store.borrow().state.draft.lines.clone();
            for row in self.row_editors.borrow().iter() {
                let line = lines
                    .get_mut(row.index)
                    .ok_or_else(|| anyhow::anyhow!("ردیف تغییر کرده؛ دوباره بررسی کنید"))?;
                let query = row.query.text().to_string();
                if line.query != query {
                    line.chosen_code = None;
                }
                line.query = query;
                line.quantity_milli = quantity(&row.quantity.text()).inspect_err(|_| {
                    row.quantity.grab_focus();
                })?;
            }
            lines
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
        let quote_date = if self.quote_date.entry.text().trim().is_empty() {
            String::new()
        } else {
            accounting::date_value(&self.quote_date)?
        };
        let valid_until = if self.valid_until.entry.text().trim().is_empty() {
            String::new()
        } else {
            accounting::date_value(&self.valid_until)?
        };
        let notes = self
            .notes
            .buffer()
            .text(
                &self.notes.buffer().start_iter(),
                &self.notes.buffer().end_iter(),
                true,
            )
            .to_string();
        let (draft, unchanged) = {
            let store = self.store.borrow();
            let original = &store.state.draft;
            let mut draft = original.clone();
            let catalog = store
                .state
                .catalogs
                .get(index)
                .map(|c| c.id.clone())
                .unwrap_or_default();
            let same_catalog = catalog == original.catalog_id;
            let same_sources = same_catalog
                && lines.len() == original.lines.len()
                && lines
                    .iter()
                    .zip(&original.lines)
                    .all(|(a, b)| a.query == b.query);
            let mut lines = lines;
            if same_catalog {
                for (i, line) in lines.iter_mut().enumerate() {
                    if let Some(old) = original.lines.get(i)
                        && old.query == line.query
                    {
                        line.chosen_code = old.chosen_code.clone();
                    }
                }
            }
            draft.catalog_id = catalog;
            draft.lines = lines;
            draft.customer = name;
            draft.phone = phone;
            if draft.customer != original.customer || draft.phone != original.phone {
                draft.customer_id.clear();
                draft.customer_contact.clear();
                draft.customer_email.clear();
                draft.customer_address.clear();
            }
            draft.discount_rial = discount;
            draft.display_unit = unit;
            draft.stale_ack = ack && same_sources;
            draft.reference = reference;
            draft.quote_date = quote_date;
            draft.valid_until = valid_until;
            draft.notes = notes;
            let unchanged = serde_json::to_vec(&draft)? == serde_json::to_vec(original)?;
            (draft, unchanged)
        };
        Ok((draft, unchanged))
    }
    fn save_form(&self) -> Result<()> {
        anyhow::ensure!(
            !self.persistence_busy.get(),
            "ذخیره در حال انجام است؛ پس از پایان دوباره درخواست کنید"
        );
        let (draft, unchanged) = self.form_draft()?;
        if unchanged {
            self.store.borrow().verify_current()?;
        } else {
            self.store.borrow_mut().change(|state| {
                state.draft = draft;
                Ok(())
            })?;
        }
        self.finish_form_save();
        Ok(())
    }
    fn finish_form_save(&self) {
        {
            let store = self.store.borrow();
            let d = &store.state.draft;
            let details = [&d.customer_contact, &d.customer_email, &d.customer_address]
                .into_iter()
                .filter(|s| !s.is_empty())
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(" • ");
            self.customer_details.set_text(&details);
            self.customer_details.set_visible(!details.is_empty());
        }
        self.loading.set(true);
        self.order
            .buffer()
            .set_text(&csv_order(&self.store.borrow().state.draft.lines));
        self.stale
            .set_active(self.store.borrow().state.draft.stale_ack);
        self.loading.set(false);
        self.table_dirty.set(false);
    }
    fn schedule(self: &Rc<Self>) {
        if self.loading.get() {
            return;
        }
        self.edit_revision
            .set(self.edit_revision.get().wrapping_add(1));
        self.status.set_text("تغییرات ذخیره‌نشده…");
        self.document_state.set_text("در حال ویرایش");
        self.approve_control.add_css_class("suggested-action");
        self.preview_control.remove_css_class("suggested-action");
        if let Some(p) = self.pending.borrow_mut().take() {
            p.remove();
        }
        let weak = Rc::downgrade(self);
        *self.pending.borrow_mut() = Some(glib::timeout_add_local_once(
            std::time::Duration::from_millis(500),
            move || {
                if let Some(ui) = weak.upgrade() {
                    ui.pending.borrow_mut().take();
                    ui.save_form_async();
                }
            },
        ));
    }
    fn fill_form(&self) {
        self.loading.set(true);
        let store = self.store.borrow();
        let s = &store.state;
        let d = &s.draft;
        let mut names = s
            .catalogs
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>();
        if names.is_empty() {
            names.push("فهرست قیمتی ساخته نشده است");
        }
        self.catalog_choice
            .set_model(Some(&gtk::StringList::new(&names)));
        self.catalog_choice.set_sensitive(!s.catalogs.is_empty());
        self.catalog_choice.set_selected(
            s.catalogs
                .iter()
                .position(|c| c.id == d.catalog_id)
                .map(|i| i as u32)
                .unwrap_or(if s.catalogs.is_empty() {
                    0
                } else {
                    gtk::INVALID_LIST_POSITION
                }),
        );
        self.customer.set_text(&d.customer);
        self.phone.set_text(&d.phone);
        let details = [&d.customer_contact, &d.customer_email, &d.customer_address]
            .into_iter()
            .filter(|s| !s.is_empty())
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(" • ");
        self.customer_details.set_text(&details);
        self.customer_details.set_visible(!details.is_empty());
        self.order.buffer().set_text(&csv_order(&d.lines));
        self.unit
            .set_selected(if d.display_unit == Unit::Toman { 0 } else { 1 });
        self.displayed_unit.set(d.display_unit);
        self.discount
            .set_text(&d.display_unit.number(d.discount_rial));
        self.stale.set_active(d.stale_ack);
        self.reference.set_text(&d.reference);
        self.set_quote_date(&self.quote_date, &d.quote_date);
        self.set_quote_date(&self.valid_until, &d.valid_until);
        self.notes.buffer().set_text(&d.notes);
        self.table_dirty.set(false);
        drop(store);
        self.loading.set(false);
    }
    fn refresh_lists(self: &Rc<Self>) {
        self.refresh_managers();
        settings::apply(self);
        let store = self.store.borrow();
        for (name, count) in &self.nav_counts {
            let value = match *name {
                "catalog" => store.state.catalogs.iter().map(|c| c.products.len()).sum(),
                "customers" => store.state.customers.iter().filter(|c| !c.archived).count(),
                "quotes" => store.state.quotes.len(),
                _ => 0,
            };
            count.set_text(&store.state.settings.display_digits.format(value));
            count.set_visible(value > 0);
        }
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
        self.table_page.set(0);
        // A new document takes the current presentation default. Changing the
        // entry calendar does not alter any issued document's saved snapshot.
        self.loading.set(true);
        let calendar =
            u32::from(self.store.borrow().state.draft.date_display == DateDisplay::Jalali);
        for date in [&self.quote_date, &self.valid_until] {
            date.entry.set_text("");
            date.calendar.set_selected(calendar);
        }
        self.fill_form();
        self.refresh_review();
        self.stack.set_visible_child_name("order");
        self.split.set_show_content(true);
        self.customer.grab_focus();
    }
    fn import_catalog(self: &Rc<Self>, path: &Path) {
        let path = path.to_path_buf();
        self.run_background("در حال بررسی فایل و ردیف‌های فهرست…", move || {
            let csv = read_import(&path)?;
            Ok(imports::catalog_csv_report(&csv,
                path.file_stem().and_then(|s| s.to_str()).unwrap_or("فهرست")))
        }, |ui, result| {
            match result {
                Err(error) => ui.message(Err::<String, _>(error)),
                Ok(report) => if let Some(catalog) = report.value {
                    if ui.active_dialog.borrow().is_some() {
                        ui.message(Ok("بررسی پایان یافت؛ پنجرهٔ جاری را ببندید و دوباره فایل را انتخاب کنید"));
                    } else {
                        ui.present_catalog_review(Ok(catalog));
                    }
                } else {
                    ui.show_import_diagnostics("خطاهای فایل فهرست قیمت", &report.diagnostics, report.truncated);
                }
            }
        });
    }
    fn present_catalog_review(self: &Rc<Self>, result: Result<Catalog>) {
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
                root.append(&catalog_preview::preview(
                    &c,
                    self.store.borrow().state.settings.display_digits,
                    self.store.borrow().state.settings.date_display,
                ));
                let ok = button("منبع و قیمت‌ها را تأیید می‌کنم");
                ok.add_css_class("suggested-action");
                root.append(&ok);
                let weak = Rc::downgrade(self);
                let d = dialog.clone();
                ok.connect_clicked(move |_| {
                    if let Some(ui) = weak.upgrade() {
                        let mut c = c.clone();
                        c.name = name.text().to_string();
                        let dialog = d.clone();
                        ui.accept_catalog_async(c, move |_, success| {
                            if success {
                                dialog.close();
                            }
                        });
                    }
                });
                dialog.set_child(Some(&dialog_toolbar(&root)));
                self.present_dialog(&dialog);
            }
        }
    }
    fn accept_catalog_async(
        self: &Rc<Self>,
        catalog: Catalog,
        complete: impl FnOnce(&Rc<Ui>, bool) + 'static,
    ) {
        if !self.background_available() {
            return;
        }
        let snapshot = self
            .save_form()
            .and_then(|()| self.store.borrow().snapshot());
        let (generation, mut candidate) = match snapshot {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.message(Err::<String, _>(error));
                complete(self, false);
                return;
            }
        };
        self.run_background(
            "در حال اعتبارسنجی و آماده‌سازی فهرست…",
            move || {
                imports::accept_catalog(&mut candidate, catalog)?;
                Store::prepare_candidate(generation, candidate)
            },
            move |ui, result| {
                // Pending widget edits are flushed before committing: a changed
                // generation rejects this candidate rather than replacing newer work.
                let result = result.and_then(|prepared| {
                    ui.save_form()?;
                    ui.store.borrow_mut().commit_prepared(prepared)
                });
                let success = result.is_ok();
                ui.message(result.map(|()| "فهرست تأیید و ذخیره شد"));
                if success {
                    ui.fill_form();
                    ui.refresh_review();
                    ui.refresh_lists();
                }
                complete(ui, success);
            },
        );
    }
    fn import_order(self: &Rc<Self>, path: &Path) {
        let path = path.to_path_buf();
        self.run_background("در حال بررسی ردیف‌های سفارش…", move || {
            let csv = read_import(&path)?;
            Ok(imports::order_csv_report(&csv))
        }, |ui, result| {
            match result {
                Err(error) => ui.message(Err::<String, _>(error)),
                Ok(report) => if let Some(lines) = report.value {
                    if ui.active_dialog.borrow().is_some() {
                        ui.message(Ok("بررسی پایان یافت؛ پنجرهٔ جاری را ببندید و دوباره فایل را انتخاب کنید"));
                    } else {
                        ui.confirm_order_import(lines);
                    }
                } else {
                    ui.show_import_diagnostics("خطاهای فایل سفارش", &report.diagnostics, report.truncated);
                }
            }
        });
    }
    fn confirm_order_import(self: &Rc<Self>, lines: Vec<OrderLine>) {
        let count = lines.len();
        let dialog = adw::AlertDialog::new(
            Some("جایگزینی اقلام سفارش؟"),
            Some(&format!(
                "{} ردیف معتبر از فایل وارد می‌شود و اقلام فعلی را جایگزین می‌کند. پیش از جایگزینی، پیش‌نویس جاری ذخیره و پشتیبان ایمنی ساخته می‌شود. اطلاعات مشتری و سند حفظ می‌شود.",
                self.store
                    .borrow()
                    .state
                    .settings
                    .display_digits
                    .format(count.to_string())
            )),
        );
        dialog.add_response("cancel", "انصراف");
        dialog.add_response("replace", "پشتیبان و جایگزینی");
        dialog.set_response_appearance("replace", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        let weak = Rc::downgrade(self);
        dialog.connect_response(None, move |_, response| {
            if response != "replace" {
                return;
            }
            if let Some(ui) = weak.upgrade() {
                let lines = lines.clone();
                ui.replace_state_async(
                    "order-import",
                    move |mut state| {
                        state.draft.lines = lines;
                        state.draft.stale_ack = false;
                        Ok(state)
                    },
                    |ui, success| {
                        if success {
                            if let Some(pending) = ui.pending.borrow_mut().take() {
                                pending.remove();
                            }
                            ui.deleted_lines.borrow_mut().take();
                            ui.undo_button.set_sensitive(false);
                            ui.table_page.set(0);
                            ui.fill_form();
                            ui.visual_pending.set(true);
                            ui.refresh_review();
                        }
                    },
                );
            }
        });
        self.present_dialog(dialog.upcast_ref());
    }
    fn restore(self: &Rc<Self>, path: &Path) {
        let path = path.to_path_buf();
        self.run_background(
            "در حال بررسی پشتیبان…",
            move || Store::restore_preview(&path),
            |ui, result| {
                if ui.active_dialog.borrow().is_some() {
                    ui.message(Ok(
                        "بررسی پایان یافت؛ پنجرهٔ جاری را ببندید و پشتیبان را دوباره انتخاب کنید",
                    ));
                    return;
                }
                match result {
                    Err(error) => ui.message(Err::<String, _>(error)),
                    Ok(candidate) => ui.confirm_restore(candidate),
                }
            },
        );
    }
    fn confirm_restore(self: &Rc<Self>, candidate: State) {
        let dialog = adw::Dialog::builder()
            .title("بررسی بازیابی")
            .content_width(550)
            .content_height(430)
            .build();
        let content = card();
        content.append(&heading("بازیابی دادهٔ تأییدشده"));
        content.append(&label(&format!("{} فهرست • {} مشتری • {} پیشنهاد\nدادهٔ جاری جایگزین می‌شود؛ پیش از آن نسخهٔ ایمنی ساخته خواهد شد.",
            fa(candidate.catalogs.len()), fa(candidate.customers.len()), fa(candidate.quotes.len()))));
        content.append(&label(&format!("دفترهای پشتیبان: {} شرکت • {} دوره • {} حساب • {} سند ثبت‌شده\nدفتر جاری: {} سند ثبت‌شده\nاین عمل همهٔ دفترها و تاریخچهٔ جاری را با پشتیبان جایگزین می‌کند. نسخهٔ ایمنیِ تأییدشده پیش از جایگزینی نگه داشته می‌شود.",
            fa(candidate.accounting.companies.len()), fa(candidate.accounting.periods.len()), fa(candidate.accounting.accounts.len()), fa(candidate.accounting.journals.len()), fa(self.store.borrow().state.accounting.journals.len()))));
        let confirm = button("بازیابی و ذخیرهٔ نسخهٔ ایمنی");
        confirm.add_css_class("destructive-action");
        let cancel = button("انصراف");
        let controls = row();
        controls.append(&confirm);
        controls.append(&cancel);
        content.append(&controls);
        let close = dialog.clone();
        cancel.connect_clicked(move |_| {
            close.close();
        });
        let weak = Rc::downgrade(self);
        let close = dialog.clone();
        confirm.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                let candidate = candidate.clone();
                let close = close.clone();
                ui.replace_state_async(
                    "restore",
                    move |_| Ok(candidate),
                    move |ui, success| {
                        if success {
                            close.close();
                            ui.deleted_lines.borrow_mut().take();
                            ui.undo_button.set_sensitive(false);
                            ui.new_product_choice.borrow_mut().take();
                            ui.refresh_settings();
                            ui.table_page.set(0);
                            ui.fill_form();
                            ui.refresh_review();
                            ui.refresh_lists();
                            // Restoration can replace every company/period ID.
                            // Rebuild the workspace from the restored snapshot.
                            if let Some(page) = ui.stack.child_by_name("accounting") {
                                ui.stack.remove(&page);
                            }
                            ui.stack
                                .add_named(&accounting::build(ui), Some("accounting"));
                        }
                    },
                );
            }
        });
        dialog.set_child(Some(&dialog_toolbar(&content)));
        self.present_dialog(&dialog);
    }
    fn replace_state_async(
        self: &Rc<Self>,
        reason: &str,
        candidate: impl FnOnce(State) -> Result<State> + Send + 'static,
        complete: impl FnOnce(&Rc<Ui>, bool) + 'static,
    ) {
        if !self.background_available() {
            return;
        }
        if settings::has_unsaved(self) {
            self.message(Err::<String, _>(anyhow::anyhow!(
                "پیش از جایگزینی، تنظیمات تغییرکرده را ذخیره یا بازگردانید"
            )));
            complete(self, false);
            return;
        }
        let snapshot = self
            .save_form()
            .and_then(|()| self.store.borrow().snapshot());
        let (generation, original) = match snapshot {
            Ok(value) => value,
            Err(error) => {
                self.message(Err::<String, _>(error));
                complete(self, false);
                return;
            }
        };
        let safety = self
            .store
            .borrow()
            .directory
            .join(format!("before-{reason}-{}.json", id()));
        self.run_background(
            "در حال آماده‌سازی داده و پشتیبان ایمنی…",
            move || {
                sarv_quote::store::backup_state(&original, &safety)?;
                let replacement = candidate(original.clone())?;
                let prepared =
                    Store::prepare_replacement(generation, &original, replacement, &safety)?;
                Ok((prepared, safety))
            },
            move |ui, result| {
                let result = result.and_then(|(prepared, safety)| {
                    anyhow::ensure!(
                        !settings::has_unsaved(ui),
                        "تنظیمات جدید دارید؛ جایگزینی انجام نشد"
                    );
                    ui.save_form()?;
                    ui.store.borrow_mut().commit_prepared(prepared)?;
                    Ok(safety)
                });
                let success = result.is_ok();
                ui.message(
                    result.map(|safety| {
                        format!("جایگزینی ذخیره شد؛ نسخهٔ ایمنی: {}", safety.display())
                    }),
                );
                complete(ui, success);
            },
        );
    }
    fn backup_to(self: &Rc<Self>, path: PathBuf) {
        if !self.background_available() {
            return;
        }
        let snapshot = self
            .save_form()
            .and_then(|()| self.store.borrow().snapshot());
        match snapshot {
            Err(error) => self.message(Err::<String, _>(error)),
            Ok((_, state)) => {
                self.run_background(
                    "در حال ساخت پشتیبان…",
                    move || {
                        sarv_quote::store::backup_state(&state, &path)?;
                        Ok(path)
                    },
                    |ui, result| {
                        ui.message(
                            result.map(|path| format!("پشتیبان ذخیره شد: {}", path.display())),
                        )
                    },
                );
            }
        }
    }
}
pub fn run() -> Result<()> {
    let _fonts = workspace::install_fonts()?;
    let data = std::env::var_os("SARV_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data"));
    let mut opened = Store::open(&data)?;
    if std::env::var("SARV_UI_SMOKE").as_deref() == Ok("full") {
        anyhow::ensure!(
            opened.state.quotes.is_empty()
                && opened.state.catalogs.is_empty()
                && opened.state.customers.is_empty()
                && opened.state.draft.lines.is_empty()
                && opened.state.draft.customer.is_empty()
                && opened.state.draft.phone.is_empty()
                && opened.state.draft.reference.is_empty()
                && opened.state.draft.notes.is_empty(),
            "full native smoke requires a fresh, separate synthetic profile"
        );
    }
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
    window.add_css_class("sarv-app");
    let root = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    let window_title = adw::WindowTitle::new("پیشنهاد قیمت", "سرو · فضای کار محلی");
    header.set_title_widget(Some(&window_title));
    let navigation = gtk::Button::from_icon_name("sidebar-show-symbolic");
    navigation.set_widget_name("show-workspace-navigation");
    navigation.set_tooltip_text(Some("فهرست صفحه‌ها"));
    accessible_name(&navigation, "فهرست صفحه‌ها");
    header.pack_start(&navigation);
    root.add_top_bar(&header);
    let body = adw::NavigationSplitView::new();
    body.set_min_sidebar_width(240.0);
    body.set_max_sidebar_width(280.0);
    body.set_sidebar_width_fraction(0.20);
    body.set_show_content(true);
    navigation.set_visible(body.is_collapsed());
    let nav_toggle = navigation.clone();
    body.connect_collapsed_notify(move |split| nav_toggle.set_visible(split.is_collapsed()));
    let breakpoint = adw::Breakpoint::new(
        adw::BreakpointCondition::parse("max-width: 1050px").expect("constant breakpoint"),
    );
    breakpoint.add_setter(&body, "collapsed", Some(&true.to_value()));
    adw_window.add_breakpoint(breakpoint);
    let nav = column();
    nav.set_spacing(0);
    nav.add_css_class("sidebar");
    nav.append(&workspace::sidebar_header(
        &store.state.settings.business_name,
    ));
    let stack = gtk::Stack::new();
    stack.set_hexpand(true);
    stack.set_vexpand(true);
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    body.set_sidebar(Some(&adw::NavigationPage::new(&nav, "صفحه‌ها")));
    let content_page = adw::NavigationPage::new(&stack, "پیشنهاد قیمت");
    body.set_content(Some(&content_page));
    root.set_content(Some(&body));
    let status = label("آماده • قیمت‌ها تنها از فهرست تأییدشده می‌آیند");
    status.add_css_class("status");
    root.add_bottom_bar(&status);
    let order_shell = column();
    order_shell.set_spacing(0);
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
    let document_buttons = adw::WrapBox::new();
    document_buttons.set_child_spacing(8);
    document_buttons.set_line_spacing(6);
    for b in [&new, &save, &preview, &approve] {
        document_buttons.append(b);
    }
    actions.append(&document_buttons);
    actions.add_css_class("document-actions");
    order_shell.append(&actions);
    let metadata = gtk::Grid::new();
    metadata.set_column_spacing(18);
    metadata.set_row_spacing(10);
    metadata.set_column_homogeneous(true);
    metadata.add_css_class("metadata");
    let customer = entry("نام مشتری یا شرکت");
    let select_customer = gtk::Button::from_icon_name("system-search-symbolic");
    accessible_name(&select_customer, "انتخاب مشتری از دفتر مشتری‌ها");
    select_customer.set_tooltip_text(Some("جست‌وجو و انتخاب مشتری"));
    let customer_control = row();
    customer_control.append(&customer);
    customer_control.append(&select_customer);
    let customer_details = label("");
    customer_details.add_css_class("caption");
    customer_details.add_css_class("dimmed");
    customer_details.set_visible(false);
    let phone = entry("۰۹۱۲…");
    phone.set_direction(gtk::TextDirection::Ltr);
    let catalog_choice = gtk::DropDown::from_strings(&[]);
    catalog_choice.set_enable_search(true);
    let unit = gtk::DropDown::from_strings(&["تومان", "ریال"]);
    let reference = entry("شمارهٔ سفارش مشتری (اختیاری)");
    let quote_date = accounting::date_field_with_preferences(
        store.state.settings.date_display,
        store.state.settings.display_digits,
        "تاریخ پیشنهاد",
        "quote-date",
        &store.state.draft.quote_date,
    );
    let valid_until = accounting::date_field_with_preferences(
        store.state.settings.date_display,
        store.state.settings.display_digits,
        "اعتبار تا • اختیاری",
        "quote-valid-until",
        &store.state.draft.valid_until,
    );
    let customer_save = button("ذخیرهٔ مشتری");
    let metadata_fields: Vec<gtk::Widget> = vec![
        field("مشتری", &customer_control).upcast(),
        field("تلفن", &phone).upcast(),
        field("فهرست قیمت تأییدشده", &catalog_choice).upcast(),
        field("واحد مبلغ", &unit).upcast(),
    ];
    customer_save.set_valign(gtk::Align::End);
    for (i, widget) in metadata_fields.iter().enumerate() {
        metadata.attach(widget, (i % 4) as i32, (i / 4) as i32, 1, 1);
    }
    let adaptive_metadata = metadata.clone();
    body.connect_collapsed_notify(move |split| {
        let columns = if split.is_collapsed() { 2 } else { 4 };
        for widget in &metadata_fields {
            adaptive_metadata.remove(widget);
        }
        for (i, widget) in metadata_fields.iter().enumerate() {
            adaptive_metadata.attach(widget, (i % columns) as i32, (i / columns) as i32, 1, 1);
        }
    });
    order_page.append(&metadata);
    order_page.append(&customer_details);
    let optional = gtk::Grid::new();
    optional.set_column_spacing(12);
    optional.set_row_spacing(8);
    optional.set_column_homogeneous(true);
    let optional_fields: Vec<gtk::Widget> = vec![
        field("مرجع مشتری", &reference).upcast(),
        quote_date.widget.clone().upcast(),
        valid_until.widget.clone().upcast(),
        customer_save.clone().upcast(),
    ];
    for (i, widget) in optional_fields.iter().enumerate() {
        optional.attach(widget, (i % 4) as i32, (i / 4) as i32, 1, 1);
    }
    let adaptive_optional = optional.clone();
    body.connect_collapsed_notify(move |split| {
        let columns = if split.is_collapsed() { 2 } else { 4 };
        for widget in &optional_fields {
            adaptive_optional.remove(widget);
        }
        for (i, widget) in optional_fields.iter().enumerate() {
            adaptive_optional.attach(widget, (i % columns) as i32, (i / columns) as i32, 1, 1);
        }
    });
    let details = gtk::Expander::new(Some("جزئیات پیشنهاد • تاریخ، اعتبار و مرجع"));
    details.set_widget_name("quote-details");
    details.set_child(Some(&optional));
    order_page.append(&details);
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
    table_scroll.set_min_content_height(240);
    table_scroll.set_max_content_height(460);
    table_scroll.set_propagate_natural_height(true);
    table_scroll.set_vexpand(false);
    table_scroll.set_widget_name("quote-table-scroll");
    table_scroll.add_css_class("table");
    order_page.append(&table_scroll);
    let add_bar = row();
    let new_item = entry("جست‌وجوی نام، کد یا نام مستعار کالا…");
    let new_quantity = entry("تعداد");
    accessible_name(&new_item, "کد کالا یا نام مستعار برای ردیف تازه");
    accessible_name(&new_quantity, "تعداد ردیف تازه");
    new_quantity.set_text("۱");
    new_quantity.set_hexpand(false);
    new_quantity.set_width_chars(8);
    new_quantity.set_max_width_chars(10);
    new_quantity.set_direction(gtk::TextDirection::Ltr);
    let add_item = button("افزودن ردیف");
    let browse_products = gtk::Button::from_icon_name("system-search-symbolic");
    accessible_name(&browse_products, "انتخاب کالا از فهرست قیمت");
    browse_products.set_tooltip_text(Some("جست‌وجوی کالا با قیمت • Ctrl+K"));
    add_bar.append(&new_item);
    add_bar.append(&browse_products);
    add_bar.append(&new_quantity);
    add_bar.append(&add_item);
    order_page.append(&add_bar);
    let table_help = label("Tab جابه‌جایی ستون‌ها • Enter بررسی و ذخیره • Ctrl+↑/↓ جابه‌جایی ردیف");
    table_help.add_css_class("caption");
    table_help.add_css_class("dimmed");
    table_help.set_selectable(false);
    order_page.append(&table_help);
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
    notes.set_accepts_tab(false);
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
    order.set_accepts_tab(false);
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
    let document_scroll = scroll(&order_page);
    document_scroll.set_widget_name("quote-document-scroll");
    order_shell.append(&document_scroll);
    stack.add_named(&order_shell, Some("order"));
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
        p.append(&workspace::page_header(title, None));
        search.set_placeholder_text(Some(match name {
            "catalog" => "جست‌وجوی نام، کد، نام مستعار یا فهرست…",
            "customers" => "جست‌وجوی نام، تلفن، مخاطب یا نشانی…",
            _ => "جست‌وجوی شماره، مشتری، مرجع یا کالا…",
        }));
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
    backup_page.append(&label(&format!("مسیر داده: {}\n\nمیانبرها: Ctrl+S ذخیره • Ctrl+Enter تأیید • Ctrl+N سفارش تازه\nAlt+1…7 جابه‌جایی صفحه • Tab پیمایش • Enter فعال‌سازی\n\nپیشنهاد قیمت و دفتر حسابداری دستی: شرکت، سال مالی، حساب‌ها، ماندهٔ افتتاحیه و اسناد متوازن. موجودی، تسویه و ارسال مالیاتی هنوز فراهم نیست.",store.directory.display())));
    stack.add_named(&backup_page, Some("backup"));
    let destinations = gtk::ListBox::new();
    destinations.set_widget_name("workspace-destinations");
    destinations.add_css_class("navigation-sidebar");
    destinations.set_selection_mode(gtk::SelectionMode::Single);
    accessible_name(&destinations, "صفحه‌های فضای کار");
    destinations.set_header_func(|item, _| {
        let group = match item.widget_name().as_str() {
            "navigation-order" => Some("پیشنهادها"),
            "navigation-customers" => Some("کالا و مشتری"),
            "navigation-accounting" => Some("حسابداری"),
            _ => None,
        };
        if let Some(group) = group {
            let caption = label(group);
            caption.add_css_class("navigation-group");
            item.set_header(Some(&caption));
        } else {
            item.set_header(None::<&gtk::Widget>);
        }
    });
    let utilities = gtk::ListBox::new();
    utilities.add_css_class("navigation-sidebar");
    utilities.set_selection_mode(gtk::SelectionMode::Single);
    accessible_name(&utilities, "تنظیمات و نگهداری داده");
    let mut nav_counts = vec![];
    let mut nav_rows = vec![];
    for (i, name, caption, icon, _group, description, utility) in [
        (
            1,
            "order",
            "پیشنهاد قیمت",
            "document-edit-symbolic",
            Some("پیشنهادها"),
            None,
            false,
        ),
        (
            4,
            "quotes",
            "بایگانی پیشنهادها",
            "document-open-recent-symbolic",
            None,
            None,
            false,
        ),
        (
            3,
            "customers",
            "مشتری‌ها",
            "system-users-symbolic",
            Some("کالا و مشتری"),
            None,
            false,
        ),
        (
            2,
            "catalog",
            "کالاها و قیمت‌ها",
            "view-list-symbolic",
            None,
            None,
            false,
        ),
        (
            7,
            "accounting",
            "دفتر حسابداری",
            "accessories-calculator-symbolic",
            Some("حسابداری"),
            Some("شرکت، اسناد و تراز آزمایشی"),
            false,
        ),
        (
            5,
            "backup",
            "پشتیبان و بازیابی",
            "document-save-symbolic",
            None,
            None,
            true,
        ),
        (
            6,
            "settings",
            "تنظیمات",
            "preferences-system-symbolic",
            None,
            None,
            true,
        ),
    ] {
        let list = if utility { &utilities } else { &destinations };
        let (item, count) =
            sidebar_destination(name, caption, icon, &format!("Alt+{i}"), description);
        nav_counts.push((name, count));
        list.append(&item);
        nav_rows.push((name, caption, item, list.clone()));
    }
    for list in [&destinations, &utilities] {
        let st = stack.clone();
        let split = body.clone();
        list.connect_row_activated(move |_, item| {
            if let Some(name) = item.widget_name().strip_prefix("navigation-") {
                st.set_visible_child_name(name);
                split.set_show_content(true);
                if split.is_collapsed() {
                    st.child_focus(gtk::DirectionType::TabForward);
                }
            }
        });
    }
    let rows = nav_rows.clone();
    let primary = destinations.clone();
    let secondary = utilities.clone();
    let title = window_title.clone();
    stack.connect_visible_child_name_notify(move |st| {
        let Some(name) = st.visible_child_name() else {
            return;
        };
        primary.unselect_all();
        secondary.unselect_all();
        for (page, caption, item, list) in &rows {
            let current = *page == name.as_str();
            item.update_property(&[gtk::accessible::Property::Description(if current {
                "صفحهٔ کنونی"
            } else {
                "باز کردن صفحه با Enter"
            })]);
            if current {
                item.add_css_class("current-page");
                list.select_row(Some(item));
                title.set_title(caption);
                content_page.set_title(caption);
            } else {
                item.remove_css_class("current-page");
            }
        }
    });
    nav_rows[0].2.add_css_class("current-page");
    destinations.select_row(Some(&nav_rows[0].2));
    nav.append(&scroll(&destinations));
    let footer = column();
    footer.set_spacing(8);
    footer.add_css_class("navigation-footer");
    footer.append(&utilities);
    let local = row();
    local.set_spacing(6);
    local.add_css_class("navigation-local");
    let local_icon = gtk::Image::from_icon_name("computer-symbolic");
    local_icon.set_pixel_size(14);
    local.append(&local_icon);
    local.append(&label("داده‌ها روی این رایانه"));
    footer.append(&local);
    nav.append(&footer);
    let split = body.clone();
    let rows = nav_rows.clone();
    let st = stack.clone();
    navigation.connect_clicked(move |_| {
        split.set_show_content(!split.shows_content());
        if !split.shows_content() {
            if let Some((_, _, item, _)) = rows
                .iter()
                .find(|(name, _, _, _)| Some(*name) == st.visible_child_name().as_deref())
            {
                item.grab_focus();
            }
        } else {
            st.child_focus(gtk::DirectionType::TabForward);
        }
    });
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
        review_cache: RefCell::new(None),
        review_requested: Cell::new(None),
        review_request_serial: Cell::new(0),
        total,
        subtotal,
        tax,
        document_state,
        approve_control: approve.clone(),
        preview_control: preview.clone(),
        source_hint,
        reference,
        quote_date,
        valid_until,
        notes,
        new_item,
        new_quantity,
        new_product_choice: RefCell::new(None),
        customer_details,
        undo_button,
        row_editors: RefCell::new(vec![]),
        table_page: Cell::new(0),
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
        persistence_busy: Cell::new(false),
        edit_revision: Cell::new(0),
        close_after_save: Cell::new(false),
        pending_save_action: RefCell::new(None),
        nav_counts,
    });
    ui.refresh_settings();
    ui.stack
        .add_named(&accounting::build(&ui), Some("accounting"));
    ui.fill_form();
    ui.refresh_review();
    ui.refresh_lists();
    ui.wire_product_picker();
    let weak = Rc::downgrade(&ui);
    select_customer.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            ui.open_customer_picker();
        }
    });
    let weak = Rc::downgrade(&ui);
    browse_products.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            ui.open_product_picker();
        }
    });
    for e in [
        &ui.customer,
        &ui.phone,
        &ui.discount,
        &ui.reference,
        &ui.quote_date.entry,
        &ui.valid_until.entry,
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
        u.save_form_async_then(|ui| ui.refresh_review());
    });
    let u = ui.clone();
    approve.connect_clicked(move |_| u.approve());
    let u = ui.clone();
    new.connect_clicked(move |_| u.new_draft());
    let u = ui.clone();
    customer_save.connect_clicked(move |_| {
        let r = u.save_form().and_then(|()| {
            u.store.borrow_mut().change(|s| {
                let draft = s.draft.clone();
                s.save_customer_details(
                    &draft.customer,
                    &draft.phone,
                    &draft.customer_contact,
                    &draft.customer_email,
                    &draft.customer_address,
                )?;
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
        choose_file(&u.window, false, "", move |p| u2.import_order(&p));
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
            move |p| u2.backup_to(p),
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
    let navigation_toggle = navigation.clone();
    keys.connect_key_pressed(move |_, key, _, mods| {
        // Modal editors own their shortcuts; window commands must not replace
        // the underlying draft or navigate away while a dialog is open.
        if u.active_dialog.borrow().is_some() {
            return glib::Propagation::Proceed;
        }
        if key == gtk::gdk::Key::Escape && u.split.is_collapsed() && !u.split.shows_content() {
            u.split.set_show_content(true);
            navigation_toggle.grab_focus();
            return glib::Propagation::Stop;
        }
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
                    u.save_form_async_then(|ui| ui.refresh_review());
                }
                gtk::gdk::Key::k | gtk::gdk::Key::K => {
                    if u.stack.visible_child_name().as_deref() == Some("order") {
                        u.open_product_picker();
                    } else {
                        return glib::Propagation::Proceed;
                    }
                }
                gtk::gdk::Key::f | gtk::gdk::Key::F => {
                    match u.stack.visible_child_name().as_deref() {
                        Some("catalog") => {
                            u.search_catalog.grab_focus();
                        }
                        Some("customers") => {
                            u.search_customer.grab_focus();
                        }
                        Some("quotes") => {
                            u.search_quote.grab_focus();
                        }
                        Some("order") => u.open_customer_picker(),
                        _ => return glib::Propagation::Proceed,
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
                gtk::gdk::Key::_7 => "accounting",
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
        if u.persistence_busy.get() {
            u.close_after_save.set(true);
            u.message(Ok("پس از ذخیرهٔ امن، برنامه بسته می‌شود…"));
            return glib::Propagation::Stop;
        }
        match u.form_draft() {
            Ok((_, false)) => {
                u.close_after_save.set(true);
                u.save_form_async();
                return glib::Propagation::Stop;
            }
            Err(error) => {
                u.message(Err::<String, _>(error));
                return glib::Propagation::Stop;
            }
            _ => {}
        }
        if let Err(e) = u.save_form() {
            u.message(Err::<String, _>(e));
            return glib::Propagation::Stop;
        }
        if settings::has_unsaved(&u) {
            let dialog = adw::AlertDialog::new(
                Some("تنظیمات ذخیره‌نشده"),
                Some("پیش از بستن برنامه، تغییرات تنظیمات را ذخیره کنید یا به آخرین تنظیمات ذخیره‌شده بازگردید."),
            );
            dialog.add_response("cancel", "ادامهٔ ویرایش");
            dialog.add_response("discard", "بازگرداندن و بستن");
            dialog.add_response("save", "ذخیره و بستن");
            dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
            dialog.set_default_response(Some("cancel"));
            dialog.set_close_response("cancel");
            let weak = Rc::downgrade(&u);
            dialog.connect_response(None, move |_, response| {
                if let Some(ui) = weak.upgrade() {
                    match response {
                        "discard" => {
                            settings::discard_staged(&ui);
                            ui.window.close();
                        }
                        "save" => {
                            ui.stack.set_visible_child_name("settings");
                            ui.split.set_show_content(true);
                            if let Some(page) = ui.stack.child_by_name("settings") {
                                activate_named_button(&page, "save-settings");
                            }
                            if !settings::has_unsaved(&ui) {
                                ui.window.close();
                            }
                        }
                        _ => {}
                    }
                }
            });
            u.present_dialog(dialog.upcast_ref());
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    adw_window.set_content(Some(&root));
    window.present();
    ui.customer.grab_focus();
    accounting::schedule(&ui);
    core::start_core_smoke(&ui);
    smoke::schedule(&ui, &approve);
    responsiveness::schedule(&ui);
}
