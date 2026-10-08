//! Manual catalogue creation always writes a new validated source snapshot.
//! Existing catalogues and archived quote prices remain untouched.
use super::*;
use std::rc::Weak;

pub(super) const CSV_TEMPLATE: &str = "code,name,measure,price,currency,source_at,aliases\nSKU-001,کالای نمونه,عدد,1250,rial,2026-10-07T00:00:00Z,نام مستعار|نام دیگر\n";

fn amount_text(product: &Product) -> String {
    match product.source_unit {
        Unit::Rial => product.price_rial.to_string(),
        Unit::Toman if product.price_rial % 10 == 0 => (product.price_rial / 10).to_string(),
        Unit::Toman => format!("{}.{}", product.price_rial / 10, product.price_rial % 10),
    }
}

fn products_csv(products: &[Product]) -> anyhow::Result<String> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "code",
        "name",
        "measure",
        "price",
        "currency",
        "source_at",
        "aliases",
    ])?;
    for product in products {
        writer.write_record([
            product.code.as_str(),
            product.name.as_str(),
            product.measure.as_str(),
            &amount_text(product),
            if product.source_unit == Unit::Rial {
                "rial"
            } else {
                "toman"
            },
            product.source_at.as_str(),
            &product.aliases.join("|"),
        ])?;
    }
    Ok(String::from_utf8(writer.into_inner()?)?)
}

struct Composer {
    ui: Weak<Ui>,
    products: RefCell<Vec<Product>>,
    editing: Cell<Option<usize>>,
    page_start: Cell<usize>,
    search: gtk::SearchEntry,
    search_index: RefCell<SearchIndex>,
    previous: gtk::Button,
    next: gtk::Button,
    digits: DisplayDigits,
    date_display: DateDisplay,
    name: gtk::Entry,
    code: adw::EntryRow,
    description: adw::EntryRow,
    measure: adw::EntryRow,
    price: adw::EntryRow,
    currency: adw::ComboRow,
    source_at: adw::EntryRow,
    aliases: adw::EntryRow,
    save_product: gtk::Button,
    clear: gtk::Button,
    review: gtk::Button,
    list: gtk::ListBox,
    count: gtk::Label,
    error: gtk::Label,
    dialog: adw::Dialog,
}

impl Composer {
    fn set_busy(&self, busy: bool) {
        // Complete text focus-out before disabling its EntryRow. Otherwise
        // GTK's cursor timer can outlive the lost focus during validation.
        if busy {
            self.dialog.set_focus(None::<&gtk::Widget>);
        }
        for field in [
            &self.code,
            &self.description,
            &self.measure,
            &self.price,
            &self.source_at,
            &self.aliases,
        ] {
            field.set_sensitive(!busy);
        }
        self.name.set_sensitive(!busy);
        self.currency.set_sensitive(!busy);
        self.save_product.set_sensitive(!busy);
        self.clear.set_sensitive(!busy);
        self.review.set_sensitive(!busy);
        self.search.set_sensitive(!busy);
        self.list.set_sensitive(!busy);
        self.previous.set_sensitive(!busy);
        self.next.set_sensitive(!busy);
    }
    /// Prepare a large copied catalogue without normalizing every product in GTK.
    /// The worker owns only a disposable product snapshot; closing discards it.
    fn prepare_index(self: &Rc<Self>) {
        self.set_busy(true);
        self.count.set_text("در حال آماده‌سازی کالاهای فهرست…");
        let weak = Rc::downgrade(self);
        glib::timeout_add_local(std::time::Duration::from_millis(20), move || {
            let Some(state) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            let Some(ui) = state.ui.upgrade() else {
                return glib::ControlFlow::Break;
            };
            if !ui.window.is_visible() {
                return glib::ControlFlow::Break;
            }
            if ui.background_busy.get() {
                return glib::ControlFlow::Continue;
            }
            let products = state.products.borrow().clone();
            let weak = Rc::downgrade(&state);
            let started = ui.run_background(
                "آماده‌سازی کالاهای فهرست…",
                move || {
                    Ok(SearchIndex::new(
                        products.iter().map(super::managers::product_document),
                    ))
                },
                move |ui, result| {
                    let Some(state) = weak.upgrade() else {
                        ui.message(Ok("آماده‌سازی فهرست لغو شد؛ فهرست اصلی تغییر نکرد."));
                        return;
                    };
                    match result {
                        Ok(index) => {
                            *state.search_index.borrow_mut() = index;
                            state.set_busy(false);
                            state.render();
                            ui.message(Ok("کالاهای فهرست برای ویرایش آماده‌اند."));
                        }
                        Err(error) => {
                            state
                                .error
                                .set_text(&format!("{error:#}؛ فرم را ببندید و دوباره باز کنید."));
                            state.count.set_text("آماده‌سازی کالاها انجام نشد.");
                            ui.message(Err::<String, _>(error));
                        }
                    }
                },
            );
            if !started {
                state
                    .error
                    .set_text("آماده‌سازی کالاها شروع نشد؛ فرم را ببندید و دوباره باز کنید.");
                state.count.set_text("آماده‌سازی کالاها انجام نشد.");
            }
            glib::ControlFlow::Break
        });
    }

    fn form_pending(&self) -> bool {
        self.editing.get().is_some()
            || [
                &self.code,
                &self.description,
                &self.price,
                &self.source_at,
                &self.aliases,
            ]
            .iter()
            .any(|entry| !entry.text().trim().is_empty())
            || self.measure.text() != "عدد"
    }
    fn require_clear_form(&self) -> bool {
        if self.form_pending() {
            self.error
                .set_text("ابتدا کالا را ذخیره کنید یا «پاک‌کردن فرم» را بزنید.");
            false
        } else {
            true
        }
    }
    fn clear_form(&self) {
        self.editing.set(None);
        for field in [&self.code, &self.description, &self.price, &self.aliases] {
            field.set_text("");
        }
        self.measure.set_text("عدد");
        self.source_at.set_text("");
        self.save_product.set_label("افزودن کالا");
        self.error.set_text("");
    }

    fn product(&self) -> anyhow::Result<Product> {
        let mut writer = csv::Writer::from_writer(Vec::new());
        writer.write_record([
            "code",
            "name",
            "measure",
            "price",
            "currency",
            "source_at",
            "aliases",
        ])?;
        let source_input = self.source_at.text();
        let source_at = if chrono::NaiveDate::parse_from_str(&source_input, "%Y-%m-%d").is_ok() {
            format!("{}T00:00:00Z", source_input)
        } else {
            source_input.to_string()
        };
        writer.write_record([
            self.code.text().as_str(),
            self.description.text().as_str(),
            self.measure.text().as_str(),
            self.price.text().as_str(),
            if self.currency.selected() == 0 {
                "rial"
            } else {
                "toman"
            },
            source_at.as_str(),
            self.aliases.text().as_str(),
        ])?;
        let input = String::from_utf8(writer.into_inner()?)?;
        let catalog = imports::catalog_csv(&input, "ورودی دستی")?;
        Ok(catalog.products[0].clone())
    }

    fn save_product(self: &Rc<Self>) {
        let result = (|| {
            let product = self.product()?;
            let mut products = self.products.borrow().clone();
            if let Some(index) = self.editing.get() {
                products[index] = product;
            } else {
                products.push(product);
            }
            anyhow::ensure!(products.len() <= 50_000, "حداکثر ۵۰ هزار کالا");
            let code = normalize(&products[self.editing.get().unwrap_or(products.len() - 1)].code);
            anyhow::ensure!(
                products
                    .iter()
                    .filter(|product| normalize(&product.code) == code)
                    .count()
                    == 1,
                "کد کالا تکراری است"
            );
            let index = self
                .editing
                .get()
                .unwrap_or(products.len().saturating_sub(1));
            self.page_start.set(index / 100 * 100);
            *self.search_index.borrow_mut() =
                SearchIndex::new(products.iter().map(super::managers::product_document));
            *self.products.borrow_mut() = products;
            self.search.set_text("");
            Ok::<(), anyhow::Error>(())
        })();
        match result {
            Ok(()) => {
                self.clear_form();
                self.render();
                self.code.grab_focus();
            }
            Err(error) => self.error.set_text(&format!("{error:#}")),
        }
    }

    fn render(self: &Rc<Self>) {
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        let products = self.products.borrow();
        let matches = self
            .search_index
            .borrow()
            .search(&self.search.text(), products.len());
        let start = self
            .page_start
            .get()
            .min(matches.len().saturating_sub(1) / 100 * 100);
        self.page_start.set(start);
        let end = start.saturating_add(100).min(matches.len());
        self.previous.set_sensitive(start > 0);
        self.next.set_sensitive(end < matches.len());
        self.count.set_text(&format!(
            "{} کالا · {} نتیجه · نمایش {} تا {} · تا تأیید نهایی ذخیره نمی‌شود",
            self.digits.format(products.len()),
            self.digits.format(matches.len()),
            self.digits
                .format(if matches.is_empty() { 0 } else { start + 1 }),
            self.digits.format(end)
        ));
        if matches.is_empty() {
            let empty = adw::ActionRow::builder()
                .title(if products.is_empty() {
                    "هنوز کالایی افزوده نشده"
                } else {
                    "کالا پیدا نشد"
                })
                .subtitle(if products.is_empty() {
                    "مشخصات و قیمت منبع را وارد کنید و «افزودن کالا» را بزنید."
                } else {
                    "عبارت جست‌وجو را تغییر دهید؛ همهٔ کالاها در فهرست تازه محفوظ‌اند."
                })
                .build();
            self.list.append(&empty);
        }
        for &index in &matches[start..end] {
            let product = &products[index];
            let item = adw::ActionRow::builder()
                .title(format!("{} · {}", product.name, ltr(&product.code)))
                .subtitle(format!(
                    "{} / {} · {}",
                    self.digits
                        .format(product.source_unit.display(product.price_rial)),
                    product.measure,
                    ltr(&super::managers::display_date(
                        &product.source_at,
                        self.digits,
                        self.date_display
                    ))
                ))
                .title_lines(1)
                .subtitle_lines(1)
                .build();
            item.set_use_markup(false);
            let edit = button("ویرایش");
            edit.set_valign(gtk::Align::Center);
            accessible_name(
                &edit,
                &format!("ویرایش کالای {} در فهرست تازه", product.code),
            );
            item.add_suffix(&edit);
            item.set_activatable_widget(Some(&edit));
            let weak = Rc::downgrade(self);
            edit.connect_clicked(move |_| {
                if let Some(state) = weak.upgrade() {
                    if !state.require_clear_form() {
                        return;
                    }
                    let product = state.products.borrow()[index].clone();
                    state.editing.set(Some(index));
                    state.code.set_text(&product.code);
                    state.description.set_text(&product.name);
                    state.measure.set_text(&product.measure);
                    state.price.set_text(&amount_text(&product));
                    state
                        .currency
                        .set_selected(if product.source_unit == Unit::Rial {
                            0
                        } else {
                            1
                        });
                    state.source_at.set_text(&product.source_at);
                    state.aliases.set_text(&product.aliases.join("|"));
                    state.save_product.set_label("ذخیرهٔ ویرایش کالا");
                    state.error.set_text("");
                    state.code.grab_focus();
                }
            });
            let remove = gtk::Button::from_icon_name("edit-delete-symbolic");
            remove.set_valign(gtk::Align::Center);
            remove.set_tooltip_text(Some("حذف از فهرست تازه؛ منبع قبلی تغییر نمی‌کند"));
            accessible_name(&remove, &format!("حذف کالای {} از فهرست تازه", product.code));
            item.add_suffix(&remove);
            let weak = Rc::downgrade(self);
            remove.connect_clicked(move |_| {
                if let Some(state) = weak.upgrade() {
                    if !state.require_clear_form() {
                        return;
                    }
                    state.products.borrow_mut().remove(index);
                    *state.search_index.borrow_mut() = SearchIndex::new(
                        state
                            .products
                            .borrow()
                            .iter()
                            .map(super::managers::product_document),
                    );
                    state.clear_form();
                    state.render();
                }
            });
            self.list.append(&item);
        }
    }

    fn review(self: &Rc<Self>) {
        if !self.require_clear_form() {
            return;
        }
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        if !ui.background_available() {
            return;
        }
        let products = self.products.borrow().clone();
        let name = self.name.text().to_string();
        self.set_busy(true);
        let weak = Rc::downgrade(self);
        if !ui.run_background(
            "در حال اعتبارسنجی فهرست تازه…",
            move || imports::catalog_csv(&products_csv(&products)?, &name),
            move |_, result| {
                let Some(state) = weak.upgrade() else {
                    return;
                };
                state.set_busy(false);
                state.render();
                match result {
                    Ok(catalog) => state.confirm(catalog),
                    Err(error) => state.error.set_text(&format!("{error:#}")),
                }
            },
        ) {
            self.set_busy(false);
            self.render();
        }
    }

    fn confirm(self: &Rc<Self>, catalog: Catalog) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let confirm = adw::Dialog::builder()
            .title("تأیید فهرست تازه")
            .content_width(760)
            .content_height(650)
            .build();
        let root = column();
        root.set_margin_start(16);
        root.set_margin_end(16);
        root.set_margin_top(16);
        root.set_margin_bottom(16);
        let explanation = label(
            "این فهرست به‌عنوان منبع تازه ذخیره و در پیش‌فاکتور انتخاب می‌شود. منبع قبلی و قیمت نسخه‌های تأییدشده محفوظ می‌مانند.",
        );
        explanation.add_css_class("caption");
        root.append(&explanation);
        root.append(&catalog_preview::preview(
            &catalog,
            self.digits,
            self.date_display,
        ));
        let confirmation_error = label("");
        confirmation_error.add_css_class("error");
        root.append(&confirmation_error);
        let accept = button("تأیید منبع و ذخیرهٔ فهرست");
        accept.add_css_class("suggested-action");
        root.append(&accept);
        let weak = Rc::downgrade(&ui);
        let state = Rc::downgrade(self);
        let confirmation = confirm.downgrade();
        accept.connect_clicked(move |accept| {
            if let (Some(ui), Some(state), Some(confirmation)) =
                (weak.upgrade(), state.upgrade(), confirmation.upgrade())
            {
                if !ui.background_available() {
                    return;
                }
                state.set_busy(true);
                accept.set_sensitive(false);
                let accept = accept.downgrade();
                let state = Rc::downgrade(&state);
                let confirmation = confirmation.downgrade();
                let error = confirmation_error.clone();
                ui.accept_catalog_async(catalog.clone(), move |_, ok| {
                    if let Some(accept) = accept.upgrade() {
                        accept.set_sensitive(true);
                    }
                    if let Some(state) = state.upgrade() {
                        state.set_busy(false);
                        state.render();
                        if ok {
                            state.dialog.close();
                        } else {
                            error.set_text(
                                "فهرست ذخیره نشد؛ پیام برنامه را بررسی و دوباره تلاش کنید.",
                            );
                        }
                    }
                    if ok && let Some(confirmation) = confirmation.upgrade() {
                        confirmation.close();
                    }
                });
            }
        });
        confirm.set_child(Some(&dialog_toolbar(&root)));
        ui.present_dialog(&confirm);
    }
}

pub(super) fn open(ui: &Rc<Ui>, existing: Option<&Catalog>) {
    let dialog = adw::Dialog::builder()
        .title(if existing.is_some() {
            "ساخت نسخهٔ تازهٔ فهرست"
        } else {
            "فهرست قیمت تازه"
        })
        .content_width(800)
        .content_height(800)
        .build();
    let root = column();
    root.set_margin_top(16);
    root.set_margin_bottom(16);
    root.set_margin_start(16);
    root.set_margin_end(16);
    let name = entry("نام فهرست قیمت");
    accessible_name(&name, "نام فهرست قیمت تازه");
    name.set_text(
        &existing
            .map(|catalog| format!("{} · نسخهٔ تازه", catalog.name))
            .unwrap_or_default(),
    );
    root.append(&field("نام فهرست", &name));
    let details = adw::PreferencesGroup::builder().title("مشخصات کالا").build();
    let source = adw::PreferencesGroup::builder()
        .title("قیمت و منبع")
        .build();
    let make_entry = |group: &adw::PreferencesGroup, title: &str| {
        let entry = adw::EntryRow::builder()
            .title(title)
            .activates_default(true)
            .build();
        group.add(&entry);
        entry
    };
    let code = make_entry(&details, "کد کالا");
    let description = make_entry(&details, "شرح کالا");
    let measure = make_entry(&details, "واحد شمارش");
    let price = make_entry(&source, "قیمت در واحد منبع · بدون جداکننده");
    let currency = adw::ComboRow::builder()
        .title("واحد قیمت منبع")
        .model(&gtk::StringList::new(&["ریال", "تومان"]))
        .build();
    source.add(&currency);
    let source_at = make_entry(&source, "تاریخ منبع · YYYY-MM-DD");
    source_at.set_tooltip_text(Some(
        "نمونه: 2026-10-07؛ تاریخ در ساعت ۰۰:۰۰ UTC ثبت می‌شود. زمان کامل منبع نیز پذیرفته می‌شود.",
    ));
    let aliases = make_entry(&details, "نام‌های مستعار تأییدشده · جداشده با |");
    details.set_size_request(300, -1);
    source.set_size_request(300, -1);
    let groups = gtk::FlowBox::new();
    groups.set_selection_mode(gtk::SelectionMode::None);
    groups.set_homogeneous(true);
    groups.set_min_children_per_line(1);
    groups.set_max_children_per_line(2);
    groups.set_column_spacing(12);
    groups.set_row_spacing(12);
    groups.insert(&details, -1);
    groups.insert(&source, -1);
    root.append(&groups);
    let error = label("");
    error.add_css_class("error");
    root.append(&error);
    let actions = row();
    let save_product = button("افزودن کالا");
    save_product.add_css_class("suggested-action");
    save_product.set_receives_default(true);
    dialog.set_default_widget(Some(&save_product));
    let clear = button("پاک‌کردن فرم");
    actions.append(&save_product);
    actions.append(&clear);
    root.append(&actions);
    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some("جست‌وجوی کالاهای فهرست تازه"));
    accessible_name(&search, "جست‌وجوی کالا در ویرایش فهرست تازه");
    root.append(&search);
    let count = label("");
    count.add_css_class("caption");
    root.append(&count);
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    list.add_css_class("boxed-list");
    list.add_css_class("manager-list");
    let product_scroll = scroll(&list);
    product_scroll.set_min_content_height(150);
    root.append(&product_scroll);
    let pages = row();
    let previous = button("کالاهای قبلی");
    let next = button("کالاهای بعدی");
    pages.append(&previous);
    pages.append(&next);
    root.append(&pages);
    let review = button("بررسی و تأیید فهرست تازه");
    root.append(&review);
    let large_copy = existing.is_some_and(|catalog| catalog.products.len() > 1000);
    let state = Rc::new(Composer {
        ui: Rc::downgrade(ui),
        products: RefCell::new(
            existing
                .map(|catalog| catalog.products.clone())
                .unwrap_or_default(),
        ),
        editing: Cell::new(None),
        page_start: Cell::new(0),
        search,
        search_index: RefCell::new(if large_copy {
            SearchIndex::new(Vec::<String>::new())
        } else {
            SearchIndex::new(
                existing
                    .map(|catalog| {
                        catalog
                            .products
                            .iter()
                            .map(super::managers::product_document)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default(),
            )
        }),
        previous,
        next,
        digits: ui.store.borrow().state.settings.display_digits,
        date_display: ui.store.borrow().state.settings.date_display,
        name,
        code,
        description,
        measure,
        price,
        currency,
        source_at,
        aliases,
        save_product,
        clear: clear.clone(),
        review: review.clone(),
        list,
        count,
        error,
        dialog: dialog.clone(),
    });
    state.clear_form();
    if large_copy {
        state.prepare_index();
    } else {
        state.render();
    }
    let weak = Rc::downgrade(&state);
    state.search.connect_search_changed(move |_| {
        if let Some(state) = weak.upgrade() {
            state.page_start.set(0);
            state.render();
        }
    });
    let weak = Rc::downgrade(&state);
    state.save_product.connect_clicked(move |_| {
        if let Some(state) = weak.upgrade() {
            state.save_product();
        }
    });
    let weak = Rc::downgrade(&state);
    state.previous.connect_clicked(move |_| {
        if let Some(state) = weak.upgrade() {
            if !state.require_clear_form() {
                return;
            }
            state
                .page_start
                .set(state.page_start.get().saturating_sub(100));
            state.render();
        }
    });
    let weak = Rc::downgrade(&state);
    state.next.connect_clicked(move |_| {
        if let Some(state) = weak.upgrade() {
            if !state.require_clear_form() {
                return;
            }
            state
                .page_start
                .set(state.page_start.get().saturating_add(100));
            state.render();
        }
    });
    let weak = Rc::downgrade(&state);
    clear.connect_clicked(move |_| {
        if let Some(state) = weak.upgrade() {
            state.clear_form();
            state.code.grab_focus();
        }
    });
    let weak = Rc::downgrade(&state);
    review.connect_clicked(move |_| {
        if let Some(state) = weak.upgrade() {
            state.review();
        }
    });
    // Keep state alive only while this dialog is open; closing clears the owned
    // widget tree and the associated closures in Ui::present_dialog.
    let lifetime = RefCell::new(Some(state));
    dialog.connect_closed(move |_| {
        lifetime.borrow_mut().take();
    });
    dialog.set_child(Some(&dialog_toolbar(&scroll(&root))));
    // Native header button FlowBox focus may consume Escape. The current
    // composer owns plain Escape; nested dialogs and open popovers retain it.
    let escape = gtk::EventControllerKey::new();
    escape.set_propagation_phase(gtk::PropagationPhase::Capture);
    let weak_dialog = dialog.downgrade();
    let weak_ui = Rc::downgrade(ui);
    escape.connect_key_pressed(move |_, key, _, modifiers| {
        let Some(dialog) = weak_dialog.upgrade() else {
            return glib::Propagation::Proceed;
        };
        let Some(ui) = weak_ui.upgrade() else {
            return glib::Propagation::Proceed;
        };
        let modified = gtk::gdk::ModifierType::SHIFT_MASK
            | gtk::gdk::ModifierType::CONTROL_MASK
            | gtk::gdk::ModifierType::ALT_MASK
            | gtk::gdk::ModifierType::SUPER_MASK
            | gtk::gdk::ModifierType::HYPER_MASK
            | gtk::gdk::ModifierType::META_MASK;
        if key != gtk::gdk::Key::Escape
            || modifiers.intersects(modified)
            || ui.visible_dialog().as_ref() != Some(&dialog)
            || !dialog.can_close()
            || dialog
                .focus()
                .is_some_and(|focus| focus.ancestor(gtk::Popover::static_type()).is_some())
        {
            return glib::Propagation::Proceed;
        }
        dialog.close();
        glib::Propagation::Stop
    });
    dialog.add_controller(escape);
    ui.present_dialog(&dialog);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_catalog_snapshot_roundtrips_exact_source_units_and_prices() {
        let original = imports::catalog_csv("code,name,measure,price,currency,source_at,aliases\nSKU-1,کاغذ,بسته,125.1,toman,2026-10-07T00:00:00Z,نام اول|نام دوم\nSKU-2,جوهر,عدد,900,rial,2026-10-07T00:00:00Z,", "اصل").unwrap();
        let before = serde_json::to_vec(&original).unwrap();
        let fresh =
            imports::catalog_csv(&products_csv(&original.products).unwrap(), "تازه").unwrap();
        assert_eq!(
            serde_json::to_vec(&fresh.products).unwrap(),
            serde_json::to_vec(&original.products).unwrap()
        );
        assert_eq!(fresh.products[0].price_rial, 1251);
        assert_eq!(amount_text(&fresh.products[0]), "125.1");
        assert_eq!(serde_json::to_vec(&original).unwrap(), before);
    }
}
