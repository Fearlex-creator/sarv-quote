use super::*;
use std::collections::HashMap;

const PAGE_SIZE: usize = 100;

fn boxed_list() -> gtk::ListBox {
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    list.add_css_class("boxed-list");
    list
}
fn action_row(title: &str, subtitle: &str) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(title)
        .subtitle(subtitle)
        .title_lines(2)
        .subtitle_lines(2)
        .build();
    row.set_use_markup(false);
    row
}
fn compact_button(caption: &str) -> gtk::Button {
    let button = button(caption);
    button.set_valign(gtk::Align::Center);
    button
}
fn contains(value: &str, needle: &str) -> bool {
    normalize(value).contains(needle)
}
fn product_matches(product: &Product, needle: &str) -> bool {
    [
        &product.code,
        &product.name,
        &product.measure,
        &product.source_at,
    ]
    .iter()
    .any(|value| contains(value, needle))
        || product.aliases.iter().any(|alias| contains(alias, needle))
}

impl Ui {
    fn manager_number(&self, value: impl ToString) -> String {
        self.store
            .borrow()
            .state
            .settings
            .display_digits
            .format(value)
    }
    pub(super) fn refresh_managers(self: &Rc<Self>) {
        // Preserve the archive filter when searches or customer edits refresh.
        let selected = self
            .quote_list
            .first_child()
            .and_then(|child| child.downcast::<gtk::DropDown>().ok())
            .map(|filter| filter.selected())
            .unwrap_or(0);
        clear(&self.catalog_list);
        clear(&self.customer_list);
        clear(&self.quote_list);
        self.catalog_list.set_spacing(12);
        self.customer_list.set_spacing(12);
        self.quote_list.set_spacing(12);
        self.refresh_customer_manager();
        self.refresh_catalog_manager();
        let filter = gtk::DropDown::from_strings(&[
            "آخرین نسخه‌های تأییدشده",
            "نسخه‌های پیشین",
            "تمام نسخه‌های تأییدشده",
        ]);
        filter.set_selected(selected.min(2));
        filter.set_tooltip_text(Some("نسخه‌های ذخیره‌شده؛ بدون وضعیت ارسال یا پرداخت"));
        self.quote_list.append(&filter);
        let archive = column();
        self.quote_list.append(&archive);
        self.render_quote_manager(&archive, filter.selected());
        let weak = Rc::downgrade(self);
        filter.connect_selected_notify(move |filter| {
            if let Some(ui) = weak.upgrade() {
                ui.render_quote_manager(&archive, filter.selected());
            }
        });
    }

    fn refresh_customer_manager(self: &Rc<Self>) {
        let actions = row();
        let add = compact_button("مشتری تازه");
        actions.append(&add);
        self.customer_list.append(&actions);
        let weak = Rc::downgrade(self);
        add.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.customer_dialog(None);
            }
        });
        let needle = normalize(&self.search_customer.text());
        let matches = self
            .store
            .borrow()
            .state
            .customers
            .iter()
            .filter(|customer| {
                contains(&customer.name, &needle) || contains(&customer.phone, &needle)
            })
            .map(|customer| customer.id.clone())
            .collect::<Vec<_>>();
        let count = label(&format!("{} مشتری", self.manager_number(matches.len())));
        count.add_css_class("dimmed");
        self.customer_list.append(&count);
        if matches.is_empty() {
            self.customer_list.append(&empty_state(
                "مشتری پیدا نشد",
                "مشتری تازه ثبت کنید یا عبارت جست‌وجو را تغییر دهید.",
            ));
            return;
        }
        let list = boxed_list();
        self.customer_list.append(&list);
        self.append_customer_page(&list, Rc::new(matches), 0);
    }

    fn append_customer_page(
        self: &Rc<Self>,
        list: &gtk::ListBox,
        ids: Rc<Vec<String>>,
        start: usize,
    ) {
        let store = self.store.borrow();
        let customers = store
            .state
            .customers
            .iter()
            .map(|customer| (customer.id.as_str(), customer))
            .collect::<HashMap<_, _>>();
        for customer in ids
            .iter()
            .skip(start)
            .take(PAGE_SIZE)
            .filter_map(|id| customers.get(id.as_str()).copied())
        {
            let item = action_row(
                &customer.name,
                &format!(
                    "تلفن: {}",
                    if customer.phone.is_empty() {
                        "—".into()
                    } else {
                        ltr(&self.manager_number(&customer.phone))
                    }
                ),
            );
            let use_customer = compact_button("استفاده");
            let edit = compact_button("ویرایش");
            item.add_suffix(&use_customer);
            item.add_suffix(&edit);
            item.set_activatable_widget(Some(&use_customer));
            let customer_id = customer.id.clone();
            let weak = Rc::downgrade(self);
            use_customer.connect_clicked(move |_| {
                if let Some(ui) = weak.upgrade() {
                    let result = ui
                        .save_form()
                        .and_then(|()| ui.preserve_draft("revision"))
                        .and_then(|()| {
                            ui.store
                                .borrow_mut()
                                .change(|state| state.use_customer(&customer_id))
                        });
                    let saved = result.is_ok();
                    ui.message(result.map(|()| "مشتری در پیش‌نویس ذخیره شد"));
                    if saved {
                        ui.fill_form();
                        ui.refresh_review();
                        ui.stack.set_visible_child_name("order");
                    }
                }
            });
            let customer_id = customer.id.clone();
            let weak = Rc::downgrade(self);
            edit.connect_clicked(move |_| {
                if let Some(ui) = weak.upgrade() {
                    ui.customer_dialog(Some(&customer_id));
                }
            });
            list.append(&item);
        }
        drop(store);
        if start + PAGE_SIZE < ids.len() {
            let more = action_row(
                "نمایش مشتری‌های بیشتر",
                &format!(
                    "{} مشتری دیگر",
                    self.manager_number(ids.len() - start - PAGE_SIZE)
                ),
            );
            let load = compact_button("نمایش بیشتر");
            more.add_suffix(&load);
            more.set_activatable_widget(Some(&load));
            list.append(&more);
            let list = list.downgrade();
            let more = more.downgrade();
            let weak = Rc::downgrade(self);
            load.connect_clicked(move |_| {
                if let (Some(ui), Some(list), Some(more)) =
                    (weak.upgrade(), list.upgrade(), more.upgrade())
                {
                    list.remove(&more);
                    ui.append_customer_page(&list, ids.clone(), start + PAGE_SIZE);
                }
            });
        }
    }

    fn customer_dialog(self: &Rc<Self>, customer_id: Option<&str>) {
        let existing = customer_id.and_then(|id| {
            self.store
                .borrow()
                .state
                .customers
                .iter()
                .find(|customer| customer.id == id)
                .cloned()
        });
        let dialog = adw::Dialog::builder()
            .title(if existing.is_some() {
                "ویرایش مشتری"
            } else {
                "مشتری تازه"
            })
            .content_width(520)
            .content_height(330)
            .build();
        let root = column();
        for edge in [gtk::Orientation::Horizontal, gtk::Orientation::Vertical] {
            if edge == gtk::Orientation::Horizontal {
                root.set_margin_start(24);
                root.set_margin_end(24);
            } else {
                root.set_margin_top(24);
                root.set_margin_bottom(24);
            }
        }
        let name = entry("نام شخص یا شرکت");
        let phone = entry("تلفن (اختیاری)");
        phone.set_direction(gtk::TextDirection::Ltr);
        if let Some(customer) = &existing {
            name.set_text(&customer.name);
            phone.set_text(&customer.phone);
        }
        root.append(&field("نام مشتری", &name));
        root.append(&field("تلفن", &phone));
        root.append(&label(
            "ویرایش مشتری، نام و تلفن نسخه‌های تأییدشدهٔ پیشین را تغییر نمی‌دهد.",
        ));
        let error = label("");
        error.add_css_class("error");
        root.append(&error);
        let actions = row();
        let save = compact_button("ذخیرهٔ مشتری");
        save.add_css_class("suggested-action");
        let cancel = compact_button("انصراف");
        actions.append(&save);
        actions.append(&cancel);
        root.append(&actions);
        let d = dialog.clone();
        cancel.connect_clicked(move |_| {
            d.close();
        });
        let weak = Rc::downgrade(self);
        let d = dialog.clone();
        let name_input = name.clone();
        save.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                let result = ui.store.borrow_mut().change(|state| {
                    if let Some(customer) = &existing {
                        state.edit_customer(&customer.id, &name_input.text(), &phone.text())?;
                        Ok(customer.id.clone())
                    } else {
                        state.save_customer(&name_input.text(), &phone.text())
                    }
                });
                match result {
                    Ok(_) => {
                        d.close();
                        ui.refresh_managers();
                        ui.message(Ok("مشتری ذخیره شد"));
                    }
                    Err(err) => {
                        error.set_text(&format!("{err:#}"));
                        name_input.grab_focus();
                    }
                }
            }
        });
        dialog.set_child(Some(&dialog_toolbar(&root)));
        let weak = Rc::downgrade(self);
        dialog.connect_closed(move |dialog| {
            dialog.set_child(None::<&gtk::Widget>);
            if let Some(ui) = weak.upgrade() {
                let current = ui.active_dialog.borrow().as_ref() == Some(dialog);
                if current {
                    ui.active_dialog.borrow_mut().take();
                }
            }
        });
        *self.active_dialog.borrow_mut() = Some(dialog.clone());
        dialog.present(Some(&self.window));
        name.grab_focus();
    }

    fn refresh_catalog_manager(self: &Rc<Self>) {
        let needle = normalize(&self.search_catalog.text());
        let store = self.store.borrow();
        let matches = store
            .state
            .catalogs
            .iter()
            .rev()
            .filter(|catalog| {
                contains(&catalog.name, &needle)
                    || contains(&catalog.imported_at, &needle)
                    || catalog
                        .products
                        .iter()
                        .any(|product| product_matches(product, &needle))
            })
            .collect::<Vec<_>>();
        let count = label(&format!(
            "{} فهرست • قیمت‌ها فقط خواندنی و دارای منبع هستند",
            self.manager_number(matches.len())
        ));
        count.add_css_class("dimmed");
        self.catalog_list.append(&count);
        if matches.is_empty() {
            self.catalog_list.append(&empty_state(
                "فهرست قیمت پیدا نشد",
                "فهرست تأییدشده وارد کنید یا کد و نام مستعار را جست‌وجو کنید.",
            ));
            return;
        }
        let list = boxed_list();
        self.catalog_list.append(&list);
        for catalog in matches {
            let expander = adw::ExpanderRow::builder()
                .title(&catalog.name)
                .subtitle(format!(
                    "{} کالا · ورود {}{}",
                    self.manager_number(catalog.products.len()),
                    ltr(&catalog.imported_at),
                    if catalog.id == store.state.draft.catalog_id {
                        " · انتخاب‌شده"
                    } else {
                        ""
                    }
                ))
                .build();
            expander.set_use_markup(false);
            let select = compact_button("انتخاب فهرست");
            select.set_sensitive(catalog.id != store.state.draft.catalog_id);
            expander.add_suffix(&select);
            let catalog_id = catalog.id.clone();
            let weak = Rc::downgrade(self);
            select.connect_clicked(move |_| {
                if let Some(ui) = weak.upgrade() {
                    let result = ui
                        .save_form()
                        .and_then(|()| ui.preserve_draft("revision"))
                        .and_then(|()| {
                            ui.store
                                .borrow_mut()
                                .change(|state| state.select_catalog(&catalog_id))
                        });
                    let ok = result.is_ok();
                    ui.message(result.map(
                        |()| "فهرست در پیش‌نویس انتخاب شد؛ اقلام و قیمت‌ها را دوباره بررسی کنید",
                    ));
                    if ok {
                        ui.fill_form();
                        ui.visual_pending.set(true);
                        ui.refresh_review();
                        ui.refresh_managers();
                    }
                }
            });
            let product_list = boxed_list();
            expander.add_row(&product_list);
            let catalog_id = catalog.id.clone();
            let needle =
                if contains(&catalog.name, &needle) || contains(&catalog.imported_at, &needle) {
                    String::new()
                } else {
                    needle.clone()
                };
            let loaded = Cell::new(false);
            let weak = Rc::downgrade(self);
            expander.connect_expanded_notify(move |expander| {
                if expander.is_expanded()
                    && !loaded.replace(true)
                    && let Some(ui) = weak.upgrade()
                {
                    ui.append_product_page(&product_list, &catalog_id, &needle, 0);
                }
            });
            list.append(&expander);
        }
    }

    fn append_product_page(
        self: &Rc<Self>,
        list: &gtk::ListBox,
        catalog_id: &str,
        needle: &str,
        start: usize,
    ) {
        let store = self.store.borrow();
        let Some(catalog) = store
            .state
            .catalogs
            .iter()
            .find(|catalog| catalog.id == catalog_id)
        else {
            return;
        };
        let products = catalog
            .products
            .iter()
            .filter(|product| product_matches(product, needle))
            .collect::<Vec<_>>();
        for product in products.iter().skip(start).take(PAGE_SIZE) {
            let item = action_row(
                &format!("{} · {}", product.name, ltr(&product.code)),
                &format!(
                    "منبع: {} · واحد منبع: {}{}",
                    ltr(&product.source_at),
                    product.source_unit.label(),
                    if stale(&product.source_at) {
                        " · قیمت قدیمی"
                    } else {
                        ""
                    }
                ),
            );
            let price = numeric_cell(&format!(
                "{} / {}",
                self.manager_number(
                    store
                        .state
                        .settings
                        .default_currency
                        .display(product.price_rial)
                ),
                product.measure
            ));
            item.add_suffix(&price);
            let aliases = product.aliases.join("، ");
            item.set_tooltip_text(Some(&format!(
                "{}\nنام‌های مستعار تأییدشده: {}\n{}",
                self.manager_number(Unit::Rial.display(product.price_rial)),
                if aliases.is_empty() {
                    "ندارد"
                } else {
                    &aliases
                },
                product.source_at
            )));
            if !aliases.is_empty() {
                item.set_subtitle(&format!(
                    "{}\nنام مستعار: {}",
                    item.subtitle().unwrap_or_default(),
                    aliases
                ));
            }
            item.set_subtitle_lines(3);
            list.append(&item);
        }
        let total = products.len();
        drop(store);
        if start + PAGE_SIZE < total {
            let more = action_row(
                "کالاهای بیشتر",
                &format!(
                    "{} کالای دیگر",
                    self.manager_number(total - start - PAGE_SIZE)
                ),
            );
            let load = compact_button("نمایش بیشتر");
            more.add_suffix(&load);
            more.set_activatable_widget(Some(&load));
            list.append(&more);
            let list = list.downgrade();
            let more = more.downgrade();
            let catalog_id = catalog_id.to_string();
            let needle = needle.to_string();
            let weak = Rc::downgrade(self);
            load.connect_clicked(move |_| {
                if let (Some(ui), Some(list), Some(more)) =
                    (weak.upgrade(), list.upgrade(), more.upgrade())
                {
                    list.remove(&more);
                    ui.append_product_page(&list, &catalog_id, &needle, start + PAGE_SIZE);
                }
            });
        }
    }

    fn render_quote_manager(self: &Rc<Self>, container: &gtk::Box, mode: u32) {
        clear(container);
        let needle = normalize(&self.search_quote.text());
        let store = self.store.borrow();
        let mut latest = HashMap::new();
        for quote in &store.state.quotes {
            latest.insert(&quote.series, quote.version);
        }
        let ids = store
            .state
            .quotes
            .iter()
            .rev()
            .filter(|quote| {
                let current = latest.get(&quote.series) == Some(&quote.version);
                (mode == 2 || (mode == 0 && current) || (mode == 1 && !current))
                    && ([
                        &quote.customer,
                        &quote.phone,
                        &quote.id,
                        &quote.reference,
                        &quote.number,
                        &quote.catalog_name,
                        &quote.approved_at,
                    ]
                    .iter()
                    .any(|value| contains(value, &needle))
                        || quote.lines.iter().any(|line| {
                            contains(&line.code, &needle) || contains(&line.name, &needle)
                        }))
            })
            .map(|quote| quote.id.clone())
            .collect::<Vec<_>>();
        container.append(&label(&format!(
            "{} نسخهٔ تأییدشده · این بایگانی وضعیت ارسال یا پرداخت ثبت نمی‌کند",
            self.manager_number(ids.len())
        )));
        drop(store);
        if ids.is_empty() {
            container.append(&empty_state(
                "پیشنهاد پیدا نشد",
                "عبارت یا فیلتر را تغییر دهید. پیشنهادها پس از تأیید نسخه اینجا ذخیره می‌شوند.",
            ));
            return;
        }
        let list = boxed_list();
        container.append(&list);
        self.append_quote_page(&list, Rc::new(ids), 0);
    }

    fn append_quote_page(self: &Rc<Self>, list: &gtk::ListBox, ids: Rc<Vec<String>>, start: usize) {
        let store = self.store.borrow();
        let quotes = store
            .state
            .quotes
            .iter()
            .map(|quote| (quote.id.as_str(), quote))
            .collect::<HashMap<_, _>>();
        let mut latest = HashMap::new();
        for quote in &store.state.quotes {
            latest.insert(quote.series.as_str(), quote.version);
        }
        for quote in ids
            .iter()
            .skip(start)
            .take(PAGE_SIZE)
            .filter_map(|id| quotes.get(id.as_str()).copied())
        {
            let expander = adw::ExpanderRow::builder()
                .title(format!(
                    "{}{} · نسخهٔ {} · {}",
                    if quote.number.is_empty() {
                        String::new()
                    } else {
                        format!("{} · ", ltr(&quote.number))
                    },
                    quote.customer,
                    self.manager_number(quote.version),
                    self.manager_number(quote.display_unit.display(quote.total_rial))
                ))
                .subtitle(format!(
                    "{} · {}{}",
                    ltr(&quote.id),
                    ltr(&quote.approved_at),
                    if quote.reference.is_empty() {
                        String::new()
                    } else {
                        format!(" · مرجع: {}", quote.reference)
                    }
                ))
                .build();
            expander.set_use_markup(false);
            let current = latest.get(quote.series.as_str()) == Some(&quote.version);
            let state = status_label(
                if current {
                    "تأییدشده · آخرین نسخه"
                } else {
                    "تأییدشده · نسخهٔ پیشین"
                },
                StatusTone::Neutral,
            );
            expander.add_suffix(&state);
            let details = action_row(
                &quote.catalog_name,
                &format!(
                    "{} قلم · {}{}",
                    self.manager_number(quote.lines.len()),
                    if quote.quote_date.is_empty() {
                        "تاریخ پیشنهاد ثبت نشده".into()
                    } else {
                        format!(
                            "تاریخ پیشنهاد {}",
                            ltr(&self.manager_number(&quote.quote_date))
                        )
                    },
                    if quote.valid_until.is_empty() {
                        String::new()
                    } else {
                        format!(
                            " · اعتبار تا {}",
                            ltr(&self.manager_number(&quote.valid_until))
                        )
                    }
                ),
            );
            expander.add_row(&details);
            let actions = row();
            actions.set_margin_top(8);
            actions.set_margin_bottom(8);
            actions.set_margin_start(12);
            actions.set_margin_end(12);
            let quote = Rc::new(quote.clone());
            for (caption, kind) in [
                ("پیش‌نمایش چاپ", "preview"),
                ("PDF", "pdf"),
                ("CSV", "csv"),
                ("نسخهٔ جدید", "revise"),
                ("کپی در پیشنهاد تازه", "duplicate"),
            ] {
                let action = compact_button(caption);
                actions.append(&action);
                let quote = quote.clone();
                let weak = Rc::downgrade(self);
                action.connect_clicked(move |_| {
                    if let Some(ui) = weak.upgrade() {
                        match kind {
                            "preview" => ui.preview_saved_quote(&quote),
                            "revise" | "duplicate" => {
                                let result = ui
                                    .save_form()
                                    .and_then(|()| ui.preserve_draft("revision"))
                                    .and_then(|()| {
                                        ui.store.borrow_mut().change(|state| {
                                            if kind == "revise" {
                                                state.revise(&quote.id)
                                            } else {
                                                state.duplicate_quote(&quote.id)
                                            }
                                        })
                                    });
                                let saved = result.is_ok();
                                ui.message(result.map(|()| {
                                    if kind == "revise" {
                                        "پیش‌نویس نسخهٔ جدید ذخیره شد؛ نسخهٔ پیشین محفوظ است"
                                    } else {
                                        "پیش‌نویس پیشنهاد تازه از نسخهٔ بایگانی ساخته شد"
                                    }
                                }));
                                if saved {
                                    ui.deleted_lines.borrow_mut().take();
                                    ui.undo_button.set_sensitive(false);
                                    ui.fill_form();
                                    ui.visual_pending.set(true);
                                    ui.refresh_review();
                                    ui.stack.set_visible_child_name("order");
                                }
                            }
                            _ => {
                                if !ui.background_available() { return; }
                                let snapshot = quote.as_ref().clone();
                                let weak = Rc::downgrade(&ui);
                                choose_file(
                                    &ui.window,
                                    true,
                                    &format!("sarv-{}-v{}.{}", quote.id, quote.version, kind),
                                    move |path| {
                                        let Some(ui) = weak.upgrade() else { return; };
                                        if !ui.window.is_visible() { return; }
                                        let quote = snapshot.clone();
                                        let working = if kind == "pdf" { "در حال ساخت فایل PDF از نسخهٔ تأییدشده؛ می‌توانید کار با برنامه را ادامه دهید" }
                                            else { "در حال ساخت فایل CSV از نسخهٔ تأییدشده؛ می‌توانید کار با برنامه را ادامه دهید" };
                                        ui.run_background(
                                            working,
                                            move || {
                                                if kind == "pdf" { export::pdf(&quote, &path)?; }
                                                else { export::csv(&quote, &path)?; }
                                                Ok(path)
                                            },
                                            |ui, result| ui.message(result.map(|path| format!("فایل نسخهٔ تأییدشده ذخیره شد: {}", path.display()))),
                                        );
                                    },
                                );
                            }
                        }
                    }
                });
            }
            expander.add_row(&actions);
            list.append(&expander);
        }
        drop(store);
        if start + PAGE_SIZE < ids.len() {
            let more = action_row(
                "پیشنهادهای بیشتر",
                &format!(
                    "{} نسخهٔ دیگر",
                    self.manager_number(ids.len() - start - PAGE_SIZE)
                ),
            );
            let load = compact_button("نمایش بیشتر");
            more.add_suffix(&load);
            more.set_activatable_widget(Some(&load));
            list.append(&more);
            let list = list.downgrade();
            let more = more.downgrade();
            let weak = Rc::downgrade(self);
            load.connect_clicked(move |_| {
                if let (Some(ui), Some(list), Some(more)) =
                    (weak.upgrade(), list.upgrade(), more.upgrade())
                {
                    list.remove(&more);
                    ui.append_quote_page(&list, ids.clone(), start + PAGE_SIZE);
                }
            });
        }
    }

    fn preview_saved_quote(self: &Rc<Self>, quote: &Quote) {
        if !self.background_available() {
            return;
        }
        let snapshot = quote.clone();
        let directory = self.store.borrow().directory.clone();
        let title = format!(
            "پیش‌نمایش چاپ · نسخهٔ تأییدشدهٔ {}",
            self.manager_number(quote.version)
        );
        self.run_background(
            "در حال آماده‌سازی پیش‌نمایش نسخهٔ تأییدشده؛ می‌توانید کار با برنامه را ادامه دهید",
            move || super::background::PreviewPages::render(&snapshot, &directory),
            move |ui, result| match result {
                Ok(pages) => ui.show_rendered_preview(pages, &title),
                Err(error) => ui.message(Err::<String, _>(error)),
            },
        );
    }
}
