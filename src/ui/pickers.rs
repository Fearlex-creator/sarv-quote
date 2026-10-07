//! Deliberate, bounded native catalog/contact browsing in the quote workspace.
use super::*;
use std::{
    sync::{Arc, mpsc},
    time::Duration,
};

const RESULT_LIMIT: usize = 30;
struct ProductSnapshot {
    catalog: Catalog,
    index: SearchIndex,
}
type ProductReady = Box<dyn FnOnce(Arc<ProductSnapshot>)>;
#[derive(Default)]
struct ProductCache {
    ready: RefCell<Option<Arc<ProductSnapshot>>>,
    pending: RefCell<Option<(String, String)>>,
    callbacks: RefCell<Vec<ProductReady>>,
}
thread_local! {
    // One bounded disposable cache for the active source, never persisted.
    static PRODUCT_CACHE: Rc<ProductCache> = Rc::new(ProductCache::default());
}
fn cached_products(ui: &Rc<Ui>, callback: impl FnOnce(Arc<ProductSnapshot>) + 'static) {
    let cache = PRODUCT_CACHE.with(Clone::clone);
    let key = {
        let store = ui.store.borrow();
        let Ok(catalog) = store.state.catalog() else {
            ui.message(Err::<String, _>(anyhow::anyhow!(
                "ابتدا فهرست قیمت را وارد و تأیید کنید"
            )));
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
    let catalog = ui
        .store
        .borrow()
        .state
        .catalog()
        .expect("source checked")
        .clone();
    cache.callbacks.borrow_mut().clear();
    cache.callbacks.borrow_mut().push(Box::new(callback));
    *cache.pending.borrow_mut() = Some(key.clone());
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let index = catalog.search_index();
        let _ = sender.send(Arc::new(ProductSnapshot { catalog, index }));
    });
    let weak = Rc::downgrade(ui);
    glib::timeout_add_local(Duration::from_millis(15), move || {
        if cache.pending.borrow().as_ref() != Some(&key) {
            return glib::ControlFlow::Break;
        }
        match receiver.try_recv() {
            Ok(snapshot) => {
                *cache.ready.borrow_mut() = Some(snapshot.clone());
                cache.pending.borrow_mut().take();
                let callbacks = cache.callbacks.borrow_mut().drain(..).collect::<Vec<_>>();
                for callback in callbacks {
                    callback(snapshot.clone());
                }
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                cache.pending.borrow_mut().take();
                cache.callbacks.borrow_mut().clear();
                if let Some(ui) = weak.upgrade() {
                    ui.message(Err::<String, _>(anyhow::anyhow!(
                        "آماده‌سازی جست‌وجوی کالا کامل نشد؛ دوباره تلاش کنید"
                    )));
                }
                glib::ControlFlow::Break
            }
        }
    });
}
fn clear_list(list: &gtk::ListBox) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
}
fn result_list(name: &str) -> gtk::ListBox {
    let list = gtk::ListBox::new();
    list.set_widget_name(name);
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.set_activate_on_single_click(false);
    list.add_css_class("boxed-list");
    list
}
fn move_selection(list: &gtk::ListBox, down: bool) {
    let index = list.selected_row().map(|row| row.index()).unwrap_or(-1);
    let next = if down { index + 1 } else { (index - 1).max(0) };
    if let Some(row) = list.row_at_index(next) {
        list.select_row(Some(&row));
    }
}
fn result_row(title: &str, subtitle: &str) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(title)
        .subtitle(subtitle)
        .title_lines(2)
        .subtitle_lines(3)
        .activatable(true)
        .build();
    row.set_use_markup(false);
    row
}
fn product_details(product: &Product, unit: Unit, settings: &Settings) -> String {
    format!(
        "{} • {} • {}\nمنبع {} • واحد منبع {}",
        ltr(&product.code),
        product.measure,
        settings
            .display_digits
            .format(unit.display(product.price_rial)),
        ltr(&settings.date_display.format(&product.source_at)),
        product.source_unit.label()
    )
}
fn picker_shell(
    title: &str,
    placeholder: &str,
    name: &str,
) -> (adw::Dialog, gtk::SearchEntry, gtk::ListBox, gtk::Label) {
    let dialog = adw::Dialog::builder()
        .title(title)
        .content_width(620)
        .content_height(620)
        .build();
    let root = column();
    root.set_margin_top(12);
    root.set_margin_bottom(12);
    root.set_margin_start(12);
    root.set_margin_end(12);
    let search = gtk::SearchEntry::new();
    // Search is indexed and results are bounded to 30 rows. Keep a short
    // debounce for Persian typing without spending the lookup budget waiting.
    search.set_search_delay(80);
    search.set_widget_name(name);
    search.set_placeholder_text(Some(placeholder));
    accessible_name(&search, placeholder);
    root.append(&search);
    let hint = label("در حال آماده‌سازی جست‌وجو…");
    hint.add_css_class("dimmed");
    hint.add_css_class("caption");
    root.append(&hint);
    let list = result_list(&format!("{name}-results"));
    root.append(&scroll(&list));
    let help = label("↑/↓ انتخاب نتیجه • Enter انتخاب صریح • Escape بازگشت");
    help.add_css_class("caption");
    help.add_css_class("dimmed");
    root.append(&help);
    let actions = row();
    let cancel = button("بازگشت");
    let weak_dialog = dialog.downgrade();
    cancel.connect_clicked(move |_| {
        if let Some(dialog) = weak_dialog.upgrade() {
            dialog.close();
        }
    });
    let choose = button("انتخاب");
    choose.set_widget_name(&format!("{name}-choose"));
    choose.add_css_class("suggested-action");
    let selected = list.clone();
    choose.connect_clicked(move |_| {
        if let Some(row) = selected.selected_row() {
            selected.emit_by_name::<()>("row-activated", &[&row]);
        }
    });
    actions.append(&cancel);
    actions.append(&choose);
    root.append(&actions);
    dialog.set_child(Some(&dialog_toolbar(&root)));
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let list_keys = list.clone();
    let weak_dialog = dialog.downgrade();
    keys.connect_key_pressed(move |_, key, _, _| {
        match key {
            gtk::gdk::Key::Down => move_selection(&list_keys, true),
            gtk::gdk::Key::Up => move_selection(&list_keys, false),
            gtk::gdk::Key::Escape => {
                if let Some(dialog) = weak_dialog.upgrade() {
                    dialog.close();
                }
            }
            gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter => {
                if let Some(row) = list_keys.selected_row() {
                    list_keys.emit_by_name::<()>("row-activated", &[&row]);
                }
            }
            _ => return glib::Propagation::Proceed,
        }
        glib::Propagation::Stop
    });
    search.add_controller(keys);
    (dialog, search, list, hint)
}
impl Ui {
    fn choose_new_product(self: &Rc<Self>, snapshot: &ProductSnapshot, code: &str) -> bool {
        let same = self.store.borrow().state.catalog().is_ok_and(|catalog| {
            catalog.id == snapshot.catalog.id
                && catalog.fingerprint == snapshot.catalog.fingerprint
                && catalog.products.iter().any(|product| product.code == code)
        });
        if !same {
            self.message(Err::<String, _>(anyhow::anyhow!(
                "فهرست قیمت تغییر کرده؛ کالا را از منبع فعلی انتخاب کنید"
            )));
            return false;
        }
        let loading = self.loading.replace(true);
        self.new_item.set_text(code);
        self.loading.set(loading);
        *self.new_product_choice.borrow_mut() =
            Some((snapshot.catalog.id.clone(), code.to_string()));
        self.new_quantity.grab_focus();
        self.new_quantity.select_region(0, -1);
        true
    }
    pub(super) fn open_product_picker(self: &Rc<Self>) {
        self.product_picker(None);
    }
    pub(super) fn open_line_product_picker(self: &Rc<Self>, index: usize, original_query: String) {
        self.product_picker(Some((index, original_query)));
    }
    fn product_picker(self: &Rc<Self>, target: Option<(usize, String)>) {
        if let Err(error) = self.save_form() {
            self.message(Err::<String, _>(error));
            return;
        }
        let (dialog, search, list, hint) = picker_shell(
            "انتخاب کالا از فهرست قیمت",
            "جست‌وجوی نام، کد یا نام مستعار کالا",
            "product-picker-search",
        );
        let result_ids = Rc::new(RefCell::new(Vec::<usize>::new()));
        let loaded = Rc::new(RefCell::new(None::<Arc<ProductSnapshot>>));
        let chosen = Rc::new(Cell::new(false));
        let chosen_closed = chosen.clone();
        let selected_line = target.as_ref().map(|(index, _)| *index);
        let weak = Rc::downgrade(self);
        dialog.connect_closed(move |_| {
            if chosen_closed.get()
                && let Some(ui) = weak.upgrade()
            {
                if let Some(index) = selected_line {
                    if let Some(row) = ui
                        .row_editors
                        .borrow()
                        .iter()
                        .find(|row| row.index == index)
                    {
                        row.quantity.grab_focus();
                    }
                } else {
                    ui.new_quantity.grab_focus();
                    ui.new_quantity.select_region(0, -1);
                }
            }
        });
        let weak = Rc::downgrade(self);
        let weak_dialog = dialog.downgrade();
        let ids = result_ids.clone();
        let snapshot = loaded.clone();
        list.connect_row_activated(move |_, row| {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let snapshot = snapshot.borrow();
            let Some(snapshot) = snapshot.as_ref() else {
                return;
            };
            let Some(id) = ids.borrow().get(row.index() as usize).copied() else {
                return;
            };
            let code = snapshot.catalog.products[id].code.clone();
            let success = if let Some((index, original_query)) = &target {
                let result = ui.save_form().and_then(|()| {
                    ui.store.borrow_mut().change(|state| {
                        anyhow::ensure!(
                            state.draft.catalog_id == snapshot.catalog.id
                                && state.catalog()?.fingerprint == snapshot.catalog.fingerprint,
                            "فهرست قیمت تغییر کرده؛ دوباره انتخاب کنید"
                        );
                        let line =
                            state.draft.lines.get_mut(*index).ok_or_else(|| {
                                anyhow::anyhow!("ردیف تغییر کرده؛ دوباره بررسی کنید")
                            })?;
                        anyhow::ensure!(
                            &line.query == original_query,
                            "درخواست ردیف تغییر کرده؛ دوباره بررسی کنید"
                        );
                        line.chosen_code = Some(code.clone());
                        state.draft.stale_ack = false;
                        Ok(())
                    })
                });
                let success = result.is_ok();
                ui.message(result.map(|()| "کالای ردیف صریحاً انتخاب شد؛ منبع قیمت را بررسی کنید"));
                if success {
                    ui.refresh_review();
                }
                success
            } else {
                ui.choose_new_product(snapshot, &code)
            };
            if success {
                chosen.set(true);
                if let Some(dialog) = weak_dialog.upgrade() {
                    dialog.close();
                }
            }
        });
        let refresh: Rc<dyn Fn()> = {
            let weak = Rc::downgrade(self);
            let search = search.clone();
            let list = list.clone();
            let hint = hint.clone();
            let ids = result_ids.clone();
            let loaded = loaded.clone();
            Rc::new(move || {
                let Some(ui) = weak.upgrade() else {
                    return;
                };
                let snapshot = loaded.borrow();
                let Some(snapshot) = snapshot.as_ref() else {
                    return;
                };
                let (matches, total) = snapshot.index.search_page(&search.text(), 0, RESULT_LIMIT);
                clear_list(&list);
                *ids.borrow_mut() = matches.clone();
                let store = ui.store.borrow();
                for id in matches {
                    let product = &snapshot.catalog.products[id];
                    list.append(&result_row(
                        &product.name,
                        &product_details(
                            product,
                            store.state.draft.display_unit,
                            &store.state.settings,
                        ),
                    ));
                }
                hint.set_text(&format!(
                    "{} نتیجه • نمایش حداکثر {}؛ برای محدودکردن، نام یا کد را دقیق‌تر بنویسید",
                    store.state.settings.display_digits.format(total),
                    fa(RESULT_LIMIT)
                ));
                if let Some(row) = list.row_at_index(0) {
                    list.select_row(Some(&row));
                }
            })
        };
        let refresh_changed = refresh.clone();
        search.connect_search_changed(move |_| refresh_changed());
        self.present_dialog(&dialog);
        search.grab_focus();
        let weak_dialog = dialog.downgrade();
        cached_products(self, move |snapshot| {
            if weak_dialog
                .upgrade()
                .is_some_and(|dialog| dialog.is_visible())
            {
                *loaded.borrow_mut() = Some(snapshot);
                refresh();
            }
        });
    }
    pub(super) fn open_customer_picker(self: &Rc<Self>) {
        if let Err(error) = self.save_form() {
            self.message(Err::<String, _>(error));
            return;
        }
        let (dialog, search, list, hint) = picker_shell(
            "انتخاب مشتری",
            "جست‌وجوی مشتری، تلفن، رابط، ایمیل یا نشانی",
            "customer-picker-search",
        );
        let customers = Arc::new(self.store.borrow().state.customers.clone());
        let chosen = Rc::new(Cell::new(false));
        let chosen_closed = chosen.clone();
        let weak = Rc::downgrade(self);
        dialog.connect_closed(move |_| {
            if chosen_closed.get()
                && let Some(ui) = weak.upgrade()
            {
                ui.customer.grab_focus();
            }
        });
        let index = Rc::new(RefCell::new(None::<SearchIndex>));
        let records = customers.clone();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let index = SearchIndex::new(records.iter().map(|customer| {
                if customer.archived {
                    String::new()
                } else {
                    format!(
                        "{} {} {} {} {}",
                        customer.name,
                        customer.phone,
                        customer.contact_name,
                        customer.email,
                        customer.address
                    )
                }
            }));
            let _ = sender.send(index);
        });
        let ids = Rc::new(RefCell::new(Vec::<usize>::new()));
        let weak = Rc::downgrade(self);
        let selected = ids.clone();
        let records = customers.clone();
        let weak_dialog = dialog.downgrade();
        list.connect_row_activated(move |_, row| {
            if let Some(ui) = weak.upgrade()
                && let Some(index) = selected.borrow().get(row.index() as usize).copied()
            {
                let result = ui.save_form().and_then(|()| {
                    ui.store
                        .borrow_mut()
                        .change(|state| state.select_customer(&records[index].id))
                });
                let success = result.is_ok();
                ui.message(result.map(|()| "مشتری و اطلاعات تماس در پیش‌نویس انتخاب شدند"));
                if success {
                    chosen.set(true);
                    ui.fill_form();
                    ui.refresh_review();
                    if let Some(dialog) = weak_dialog.upgrade() {
                        dialog.close();
                    }
                }
            }
        });
        let refresh: Rc<dyn Fn()> = {
            let search = search.clone();
            let list = list.clone();
            let hint = hint.clone();
            let ids = ids.clone();
            let index = index.clone();
            Rc::new(move || {
                let index = index.borrow();
                let Some(index) = index.as_ref() else {
                    return;
                };
                let (matches, total) = index.search_page(&search.text(), 0, RESULT_LIMIT);
                clear_list(&list);
                *ids.borrow_mut() = matches.clone();
                for id in matches {
                    let customer = &customers[id];
                    let details = [
                        &customer.phone,
                        &customer.contact_name,
                        &customer.email,
                        &customer.address,
                    ]
                    .into_iter()
                    .filter(|value| !value.trim().is_empty())
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join(" • ");
                    list.append(&result_row(&customer.name, &details));
                }
                hint.set_text(&format!(
                    "{} مشتری فعال • حداکثر {} نتیجه در این جست‌وجو",
                    fa(total),
                    fa(RESULT_LIMIT)
                ));
                if let Some(row) = list.row_at_index(0) {
                    list.select_row(Some(&row));
                }
            })
        };
        let changed = refresh.clone();
        search.connect_search_changed(move |_| changed());
        refresh();
        self.present_dialog(&dialog);
        search.grab_focus();
        let weak_dialog = dialog.downgrade();
        glib::timeout_add_local(Duration::from_millis(15), move || {
            if !weak_dialog
                .upgrade()
                .is_some_and(|dialog| dialog.is_visible())
            {
                return glib::ControlFlow::Break;
            }
            match receiver.try_recv() {
                Ok(ready) => {
                    *index.borrow_mut() = Some(ready);
                    refresh();
                    glib::ControlFlow::Break
                }
                Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => {
                    hint.set_text("جست‌وجوی مشتری آماده نشد؛ دوباره تلاش کنید");
                    glib::ControlFlow::Break
                }
            }
        });
    }
    pub(super) fn wire_product_picker(self: &Rc<Self>) {
        let popup = gtk::Popover::new();
        popup.set_autohide(false);
        popup.set_has_arrow(false);
        popup.set_position(gtk::PositionType::Bottom);
        popup.set_parent(&self.new_item);
        popup.set_width_request(480);
        let root = column();
        let hint = label("نام یا کد را جست‌وجو و نتیجه را صریحاً انتخاب کنید");
        hint.add_css_class("caption");
        root.append(&hint);
        let list = result_list("product-autocomplete-results");
        let scrolled = scroll(&list);
        scrolled.set_min_content_height(120);
        scrolled.set_max_content_height(280);
        scrolled.set_propagate_natural_height(true);
        scrolled.set_vexpand(false);
        root.append(&scrolled);
        popup.set_child(Some(&root));
        let loaded = Rc::new(RefCell::new(None::<Arc<ProductSnapshot>>));
        let ids = Rc::new(RefCell::new(Vec::<usize>::new()));
        let generation = Rc::new(Cell::new(0));
        let weak = Rc::downgrade(self);
        let popup_selected = popup.clone();
        let selected_ids = ids.clone();
        let snapshot = loaded.clone();
        list.connect_row_activated(move |_, row| {
            let snapshot = snapshot.borrow().clone();
            let id = selected_ids.borrow().get(row.index() as usize).copied();
            if let Some(ui) = weak.upgrade()
                && let Some(snapshot) = snapshot.as_ref()
                && let Some(id) = id
                && ui.choose_new_product(snapshot, &snapshot.catalog.products[id].code)
            {
                popup_selected.popdown();
            }
        });
        let weak = Rc::downgrade(self);
        let popup_changed = popup.clone();
        let list_changed = list.clone();
        let hint_changed = hint.clone();
        let snapshots = loaded.clone();
        let found = ids.clone();
        let versions = generation.clone();
        self.new_item.connect_changed(move |entry| {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            versions.set(versions.get() + 1);
            if ui.loading.get() {
                popup_changed.popdown();
                return;
            }
            ui.new_product_choice.borrow_mut().take();
            if entry.text().trim().is_empty() {
                popup_changed.popdown();
                return;
            }
            let version = versions.get();
            let versions = versions.clone();
            let weak = Rc::downgrade(&ui);
            let popup = popup_changed.clone();
            let list = list_changed.clone();
            let hint = hint_changed.clone();
            let snapshots = snapshots.clone();
            let found = found.clone();
            cached_products(&ui, move |snapshot| {
                let Some(ui) = weak.upgrade() else {
                    return;
                };
                if version != versions.get() || ui.active_dialog.borrow().is_some() {
                    return;
                }
                let focus = gtk::prelude::GtkWindowExt::focus(&ui.window);
                if !focus.is_some_and(|focus| {
                    focus == ui.new_item.clone().upcast::<gtk::Widget>()
                        || focus.is_ancestor(&ui.new_item)
                }) {
                    return;
                }
                let (matches, total) =
                    snapshot
                        .index
                        .search_page(&ui.new_item.text(), 0, RESULT_LIMIT);
                clear_list(&list);
                *found.borrow_mut() = matches.clone();
                let store = ui.store.borrow();
                for id in matches {
                    let product = &snapshot.catalog.products[id];
                    list.append(&result_row(
                        &product.name,
                        &product_details(
                            product,
                            store.state.draft.display_unit,
                            &store.state.settings,
                        ),
                    ));
                }
                *snapshots.borrow_mut() = Some(snapshot);
                hint.set_text(&format!(
                    "{} نتیجه • ↑/↓ و Enter برای انتخاب • Escape بستن",
                    fa(total)
                ));
                if let Some(row) = list.row_at_index(0) {
                    list.select_row(Some(&row));
                }
                if total > 0 {
                    popup.popup();
                    // Ordinary GtkEntry::grab_focus may select the whole query
                    // after a popover focus transition. Keep typing and caret
                    // navigation intact while the suggestions remain visible.
                    ui.new_item.grab_focus_without_selecting();
                } else {
                    popup.popdown();
                }
            });
        });
        let keys = gtk::EventControllerKey::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        let popup_keys = popup.clone();
        let list_keys = list.clone();
        keys.connect_key_pressed(move |_, key, _, _| {
            if key == gtk::gdk::Key::Escape {
                generation.set(generation.get() + 1);
                popup_keys.popdown();
                return glib::Propagation::Stop;
            }
            if !popup_keys.is_visible() {
                return glib::Propagation::Proceed;
            }
            match key {
                gtk::gdk::Key::Down => move_selection(&list_keys, true),
                gtk::gdk::Key::Up => move_selection(&list_keys, false),
                gtk::gdk::Key::Escape => popup_keys.popdown(),
                gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter => {
                    if let Some(row) = list_keys.selected_row() {
                        list_keys.emit_by_name::<()>("row-activated", &[&row]);
                    }
                }
                gtk::gdk::Key::Tab => {
                    popup_keys.popdown();
                    return glib::Propagation::Proceed;
                }
                _ => return glib::Propagation::Proceed,
            }
            glib::Propagation::Stop
        });
        self.new_item.add_controller(keys);
        let popup_unrealize = popup.clone();
        self.new_item
            .connect_unrealize(move |_| popup_unrealize.unparent());
        let focus = gtk::EventControllerFocus::new();
        let weak = Rc::downgrade(self);
        focus.connect_leave(move |_| {
            let popup = popup.clone();
            let weak = weak.clone();
            glib::idle_add_local_once(move || {
                if let Some(ui) = weak.upgrade() {
                    let stays =
                        gtk::prelude::GtkWindowExt::focus(&ui.window).is_some_and(|focus| {
                            focus.is_ancestor(&popup) || focus.is_ancestor(&ui.new_item)
                        });
                    if !stays {
                        popup.popdown();
                    }
                }
            });
        });
        self.new_item.add_controller(focus);
    }
}
