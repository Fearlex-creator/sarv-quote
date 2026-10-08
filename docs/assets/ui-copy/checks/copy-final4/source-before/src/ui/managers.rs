use super::*;
use std::collections::{HashMap, HashSet};

const PAGE_SIZE: usize = 100;

fn boxed_list() -> gtk::ListBox {
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    list.add_css_class("boxed-list");
    list.add_css_class("manager-list");
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
fn fields_match(values: &[&str], needle: &str) -> bool {
    let values = values
        .iter()
        .map(|value| normalize(value))
        .collect::<Vec<_>>();
    needle
        .split_whitespace()
        .all(|word| values.iter().any(|value| value.contains(word)))
}

pub(super) fn product_document(product: &Product) -> String {
    format!(
        "{}\n{}\n{}\n{}\n{}",
        product.code,
        product.name,
        product.measure,
        product.source_at,
        product.aliases.join("\n")
    )
}

struct CachedCatalogSearch {
    catalog_id: String,
    fingerprint: String,
    ordered_codes: Vec<String>,
    rows: usize,
    index: SearchIndex,
}
thread_local! {
    // Cache rows refer to source order, while the catalogue fingerprint is
    // order-independent. Match both identity and ordered codes after a restore.
    // Bound cached documents so search never constructs a widget per result.
    static CATALOG_SEARCH: RefCell<Vec<CachedCatalogSearch>> = const { RefCell::new(Vec::new()) };
}
fn catalog_page(
    catalog: &Catalog,
    needle: &str,
    start: usize,
    limit: usize,
) -> (Vec<usize>, usize) {
    if needle.is_empty() {
        return (
            (start..start.saturating_add(limit).min(catalog.products.len())).collect(),
            catalog.products.len(),
        );
    }
    CATALOG_SEARCH.with(|cache| {
        let mut cache = cache.borrow_mut();
        let position = if let Some(position) = cache.iter().position(|cached| {
            cached.catalog_id == catalog.id
                && cached.fingerprint == catalog.fingerprint
                && catalog
                    .products
                    .iter()
                    .map(|product| product.code.as_str())
                    .eq(cached.ordered_codes.iter().map(String::as_str))
        }) {
            position
        } else {
            while cache.iter().map(|cached| cached.rows).sum::<usize>() + catalog.products.len()
                > 100_000
                && !cache.is_empty()
            {
                cache.remove(0);
            }
            cache.push(CachedCatalogSearch {
                catalog_id: catalog.id.clone(),
                fingerprint: catalog.fingerprint.clone(),
                ordered_codes: catalog
                    .products
                    .iter()
                    .map(|product| product.code.clone())
                    .collect(),
                rows: catalog.products.len(),
                index: SearchIndex::new(catalog.products.iter().map(product_document)),
            });
            cache.len() - 1
        };
        cache[position].index.search_page(needle, start, limit)
    })
}

thread_local! {
    static CUSTOMER_EXPANDED: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

pub(super) fn display_date(
    value: &str,
    digits: DisplayDigits,
    date_display: DateDisplay,
) -> String {
    let date = value.get(..10).unwrap_or(value);
    digits.format(date_display.format(date))
}

fn wrapping_actions() -> gtk::FlowBox {
    let actions = gtk::FlowBox::new();
    actions.set_selection_mode(gtk::SelectionMode::None);
    actions.set_homogeneous(false);
    actions.set_min_children_per_line(1);
    actions.set_max_children_per_line(5);
    actions.set_column_spacing(8);
    actions.set_row_spacing(8);
    actions.set_halign(gtk::Align::Start);
    actions
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
    fn manager_date(&self, value: &str) -> String {
        let store = self.store.borrow();
        display_date(
            value,
            store.state.settings.display_digits,
            store.state.settings.date_display,
        )
    }
    pub(super) fn refresh_managers(self: &Rc<Self>) {
        // Preserve the archive filter when searches or customer edits refresh.
        let selected = self
            .quote_list
            .first_child()
            .and_then(|child| child.downcast::<gtk::DropDown>().ok())
            .map(|filter| filter.selected())
            .unwrap_or(0);
        let show_archived = self
            .customer_list
            .first_child()
            .and_then(|header| header.first_child())
            .and_then(|add| add.next_sibling())
            .and_then(|filter| filter.downcast::<gtk::CheckButton>().ok())
            .is_some_and(|filter| filter.is_active());
        self.search_customer
            .set_placeholder_text(Some("نام، مخاطب، تلفن یا ایمیل"));
        self.search_catalog
            .set_placeholder_text(Some("نام فهرست، کد، شرح یا نام مستعار کالا"));
        self.search_quote
            .set_placeholder_text(Some("شماره، مشتری، مرجع یا کالای پیش‌فاکتور"));
        clear(&self.catalog_list);
        clear(&self.customer_list);
        clear(&self.quote_list);
        self.catalog_list.set_spacing(12);
        self.customer_list.set_spacing(12);
        self.quote_list.set_spacing(12);
        self.refresh_customer_manager(show_archived);
        self.refresh_catalog_manager();
        let filter = gtk::DropDown::from_strings(&[
            "آخرین نسخه‌های تأییدشده",
            "نسخه‌های پیشین",
            "تمام نسخه‌های تأییدشده",
        ]);
        filter.set_selected(selected.min(2));
        filter.set_tooltip_text(Some("نسخه‌های ذخیره‌شده؛ بدون وضعیت ارسال یا پرداخت"));
        self.quote_list.append(&filter);
        self.append_current_draft();
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

    fn append_current_draft(self: &Rc<Self>) {
        let store = self.store.borrow();
        let draft = &store.state.draft;
        let number = store
            .state
            .quotes
            .iter()
            .rev()
            .find(|quote| quote.series == draft.series)
            .map(|quote| quote.number.as_str())
            .unwrap_or("");
        let customer_input = self.customer.text();
        let reference_input = self.reference.text();
        let needle = normalize(&self.search_quote.text());
        if !fields_match(
            &[
                "پیش‌نویس جاری",
                &draft.customer,
                &draft.phone,
                &draft.reference,
                number,
                &customer_input,
                &reference_input,
            ],
            &needle,
        ) {
            return;
        }
        let draft_row = action_row(
            "پیش‌نویس جاری",
            &format!(
                "{} · {} قلم · نسخهٔ تأییدشده نیست{}",
                if draft.customer.trim().is_empty() {
                    "مشتری ثبت نشده"
                } else {
                    &draft.customer
                },
                self.manager_number(draft.lines.len()),
                if draft.reference.is_empty() {
                    String::new()
                } else {
                    format!(" · مرجع: {}", draft.reference)
                }
            ),
        );
        let edit = compact_button("ادامهٔ ویرایش");
        edit.set_tooltip_text(Some(
            "بازگشت به همان فرم؛ نوشته‌های در انتظار ذخیره و ورودی‌های نامعتبر حفظ می‌شوند",
        ));
        draft_row.add_suffix(&edit);
        draft_row.set_activatable_widget(Some(&edit));
        let list = boxed_list();
        list.append(&draft_row);
        self.quote_list.append(&list);
        let weak = Rc::downgrade(self);
        edit.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.stack.set_visible_child_name("order");
            }
        });
    }

    fn refresh_customer_manager(self: &Rc<Self>, show_archived: bool) {
        let actions = row();
        actions.add_css_class("manager-toolbar");
        let add = compact_button("مشتری تازه");
        actions.append(&add);
        let archived = gtk::CheckButton::with_label("نمایش بایگانی");
        archived.set_active(show_archived);
        actions.append(&archived);
        self.customer_list.append(&actions);
        let weak = Rc::downgrade(self);
        archived.connect_toggled(move |filter| {
            if let Some(ui) = weak.upgrade() {
                let focused = filter.has_focus();
                ui.refresh_managers();
                if focused
                    && let Some(filter) = ui
                        .customer_list
                        .first_child()
                        .and_then(|header| header.first_child())
                        .and_then(|add| add.next_sibling())
                        .and_then(|filter| filter.downcast::<gtk::CheckButton>().ok())
                {
                    filter.grab_focus();
                }
            }
        });
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
                (show_archived || !customer.archived)
                    && fields_match(
                        &[
                            &customer.name,
                            &customer.phone,
                            &customer.contact_name,
                            &customer.email,
                            &customer.address,
                        ],
                        &needle,
                    )
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
            let item = adw::ExpanderRow::builder()
                .title(&customer.name)
                .subtitle(format!(
                    "{}{}{}{}",
                    if customer.phone.is_empty() {
                        "بدون تلفن".into()
                    } else {
                        ltr(&self.manager_number(&customer.phone))
                    },
                    if customer.contact_name.is_empty() {
                        String::new()
                    } else {
                        format!(" · {}", customer.contact_name)
                    },
                    if customer.email.is_empty() {
                        String::new()
                    } else {
                        format!(" · {}", ltr(&customer.email))
                    },
                    if customer.archived {
                        " · بایگانی"
                    } else {
                        ""
                    }
                ))
                .title_lines(1)
                .subtitle_lines(2)
                .build();
            item.set_use_markup(false);
            let use_customer = compact_button("استفاده");
            accessible_name(
                &use_customer,
                &format!("استفاده از مشتری {} در پیش‌فاکتور", customer.name),
            );
            use_customer.set_sensitive(!customer.archived);
            item.add_suffix(&use_customer);
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
                                .change(|state| state.select_customer(&customer_id))
                        });
                    let saved = result.is_ok();
                    ui.message(result.map(|()| "اطلاعات مشتری در پیش‌نویس ذخیره شد"));
                    if saved {
                        ui.fill_form();
                        ui.refresh_review();
                        ui.stack.set_visible_child_name("order");
                    }
                }
            });
            if !customer.address.is_empty() {
                item.add_row(&action_row("نشانی", &customer.address));
            }
            let actions = wrapping_actions();
            actions.set_margin_start(12);
            actions.set_margin_end(12);
            actions.set_margin_top(6);
            actions.set_margin_bottom(6);
            let edit = compact_button("ویرایش");
            accessible_name(&edit, &format!("ویرایش مشتری {}", customer.name));
            actions.insert(&edit, -1);
            let customer_id = customer.id.clone();
            let weak = Rc::downgrade(self);
            edit.connect_clicked(move |_| {
                if let Some(ui) = weak.upgrade() {
                    ui.customer_dialog(Some(&customer_id));
                }
            });
            let history = compact_button("پیش‌فاکتورهای مشتری");
            accessible_name(&history, &format!("پیش‌فاکتورهای مشتری {}", customer.name));
            actions.insert(&history, -1);
            let customer_snapshot = customer.clone();
            let weak = Rc::downgrade(self);
            history.connect_clicked(move |_| {
                if let Some(ui) = weak.upgrade() {
                    ui.customer_history(&customer_snapshot);
                }
            });
            let archive = compact_button(if customer.archived {
                "بازگردانی از بایگانی"
            } else {
                "بایگانی مشتری"
            });
            accessible_name(
                &archive,
                &format!(
                    "{} {}",
                    if customer.archived {
                        "بازگردانی از بایگانی مشتری"
                    } else {
                        "بایگانی مشتری"
                    },
                    customer.name
                ),
            );
            archive.set_tooltip_text(Some("سوابق تأییدشده و اطلاعات مشتری حذف نمی‌شوند"));
            actions.insert(&archive, -1);
            let customer_id = customer.id.clone();
            let archived = customer.archived;
            let weak = Rc::downgrade(self);
            archive.connect_clicked(move |_| {
                if let Some(ui) = weak.upgrade() {
                    let result = ui
                        .store
                        .borrow_mut()
                        .change(|state| state.set_customer_archived(&customer_id, !archived));
                    let ok = result.is_ok();
                    ui.message(result.map(|()| {
                        if archived {
                            "مشتری از بایگانی بازگردانده شد"
                        } else {
                            "مشتری بایگانی شد؛ سوابق محفوظ‌اند"
                        }
                    }));
                    if ok {
                        ui.refresh_managers();
                    }
                }
            });
            item.add_row(&actions);
            let opened =
                CUSTOMER_EXPANDED.with(|expanded| expanded.borrow().contains(&customer.id));
            item.set_expanded(opened || !self.search_customer.text().trim().is_empty());
            let customer_id = customer.id.clone();
            item.connect_expanded_notify(move |item| {
                CUSTOMER_EXPANDED.with(|expanded| {
                    if item.is_expanded() {
                        expanded.borrow_mut().insert(customer_id.clone());
                    } else {
                        expanded.borrow_mut().remove(&customer_id);
                    }
                });
            });
            list.append(&item);
        }
        drop(store);
        if start + PAGE_SIZE < ids.len() {
            let more = action_row(
                "نمایش مشتریان بیشتر",
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
            .content_height(600)
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
        let contact = entry("نام مخاطب (اختیاری)");
        let email = entry("name@example.com");
        email.set_direction(gtk::TextDirection::Ltr);
        email.set_input_purpose(gtk::InputPurpose::Email);
        let address = entry("نشانی (اختیاری)");
        if let Some(customer) = &existing {
            name.set_text(&customer.name);
            phone.set_text(&customer.phone);
            contact.set_text(&customer.contact_name);
            email.set_text(&customer.email);
            address.set_text(&customer.address);
        }
        root.append(&field("نام مشتری", &name));
        root.append(&field("نام مخاطب", &contact));
        root.append(&field("تلفن", &phone));
        root.append(&field("ایمیل", &email));
        root.append(&field("نشانی", &address));
        let hint = label("ویرایش اطلاعات، نسخه‌های تأییدشده را تغییر نمی‌دهد.");
        hint.add_css_class("caption");
        root.append(&hint);
        let error = label("");
        error.add_css_class("error");
        root.append(&error);
        let actions = row();
        actions.add_css_class("manager-toolbar");
        let save = compact_button("ذخیرهٔ مشتری");
        save.add_css_class("suggested-action");
        save.set_receives_default(true);
        dialog.set_default_widget(Some(&save));
        name.set_activates_default(true);
        phone.set_activates_default(true);
        contact.set_activates_default(true);
        email.set_activates_default(true);
        address.set_activates_default(true);
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
                        state.edit_customer_details(
                            &customer.id,
                            &name_input.text(),
                            &phone.text(),
                            &contact.text(),
                            &email.text(),
                            &address.text(),
                        )?;
                        Ok(customer.id.clone())
                    } else {
                        state.save_customer_details(
                            &name_input.text(),
                            &phone.text(),
                            &contact.text(),
                            &email.text(),
                            &address.text(),
                        )
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
        dialog.set_child(Some(&dialog_toolbar(&scroll(&root))));
        self.present_dialog(&dialog);
        name.grab_focus();
    }

    fn customer_history(self: &Rc<Self>, customer: &Customer) {
        let store = self.store.borrow();
        let mut legacy = 0usize;
        let ids = store
            .state
            .quotes
            .iter()
            .rev()
            .filter(|quote| {
                if quote.customer_id == customer.id {
                    true
                } else if quote.customer_id.is_empty()
                    && normalize(&quote.customer) == normalize(&customer.name)
                    && digits(&quote.phone) == digits(&customer.phone)
                {
                    legacy += 1;
                    true
                } else {
                    false
                }
            })
            .map(|quote| quote.id.clone())
            .collect::<Vec<_>>();
        drop(store);
        let dialog = adw::Dialog::builder()
            .title(format!("پیش‌فاکتورهای {}", customer.name))
            .content_width(760)
            .content_height(620)
            .build();
        let root = column();
        root.set_margin_top(16);
        root.set_margin_bottom(16);
        root.set_margin_start(16);
        root.set_margin_end(16);
        let count = label(&format!("{} نسخهٔ تأییدشده", self.manager_number(ids.len())));
        count.add_css_class("caption");
        root.append(&count);
        if legacy > 0 {
            let caveat =
                label("سوابق قدیمی بدون شناسهٔ مشتری، با نام و تلفن یکسان نمایش داده می‌شوند.");
            caveat.add_css_class("caption");
            root.append(&caveat);
        }
        if ids.is_empty() {
            root.append(&empty_state(
                "پیش‌فاکتوری ثبت نشده",
                "نسخه‌های تأییدشدهٔ این مشتری اینجا نمایش داده می‌شوند.",
            ));
        } else {
            let list = boxed_list();
            self.append_quote_page(&list, Rc::new(ids), 0);
            root.append(&list);
        }
        dialog.set_child(Some(&dialog_toolbar(&scroll(&root))));
        self.present_dialog(&dialog);
    }

    fn refresh_catalog_manager(self: &Rc<Self>) {
        let actions = row();
        actions.add_css_class("manager-toolbar");
        let create = compact_button("فهرست تازه");
        create.add_css_class("suggested-action");
        actions.append(&create);
        let template = compact_button("کپی قالب CSV");
        actions.append(&template);
        self.catalog_list.append(&actions);
        let weak = Rc::downgrade(self);
        create.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                catalog_editor::open(&ui, None);
            }
        });
        let weak = Rc::downgrade(self);
        template.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.window.clipboard().set_text(catalog_editor::CSV_TEMPLATE);
                ui.message(Ok("قالب CSV با یک ردیف نمونه در کلیپ‌بورد کپی شد"));
            }
        });
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
                    || catalog_page(catalog, &needle, 0, 1).1 > 0
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
                    ltr(&self.manager_date(&catalog.imported_at)),
                    if catalog.id == store.state.draft.catalog_id {
                        " · انتخاب‌شده"
                    } else {
                        ""
                    }
                ))
                .build();
            expander.set_use_markup(false);
            expander.set_title_lines(2);
            expander.set_subtitle_lines(2);
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
            let edit_actions = wrapping_actions();
            edit_actions.set_margin_start(12);
            edit_actions.set_margin_end(12);
            edit_actions.set_margin_top(6);
            edit_actions.set_margin_bottom(6);
            let edit_catalog = compact_button("ساخت نسخهٔ تازه");
            edit_catalog.set_tooltip_text(Some(
                "افزودن یا ویرایش کالا در منبع تازه؛ قیمت‌های این منبع و سوابق تغییر نمی‌کند",
            ));
            edit_actions.insert(&edit_catalog, -1);
            expander.add_row(&edit_actions);
            let snapshot = catalog.clone();
            let weak = Rc::downgrade(self);
            edit_catalog.connect_clicked(move |_| {
                if let Some(ui) = weak.upgrade() {
                    catalog_editor::open(&ui, Some(&snapshot));
                }
            });
            let product_list = boxed_list();
            let catalog_id = catalog.id.clone();
            let needle =
                if contains(&catalog.name, &needle) || contains(&catalog.imported_at, &needle) {
                    String::new()
                } else {
                    needle.clone()
                };
            let matching = catalog_page(catalog, &needle, 0, 0).1;
            let summary = action_row(
                &format!("{} کالای مطابق", self.manager_number(matching)),
                "منبع فقط خواندنی · ویرایش با «ساخت نسخهٔ تازه»",
            );
            expander.add_row(&summary);
            expander.add_row(&product_list);
            expander.set_expanded(false);
            if !needle.is_empty() {
                expander.set_expanded(true);
                self.append_product_page(&product_list, &catalog_id, &needle, 0);
            }
            let loaded = Cell::new(!needle.is_empty());
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
        let (indices, total) = catalog_page(catalog, needle, start, PAGE_SIZE);
        for index in indices {
            let product = &catalog.products[index];
            let item = action_row(
                &format!("{} · {}", product.name, ltr(&product.code)),
                &format!(
                    "منبع: {} · واحد منبع: {}{}",
                    ltr(&self.manager_date(&product.source_at)),
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
            let add = compact_button("افزودن");
            accessible_name(&add, &format!("افزودن {} به پیش‌فاکتور", product.code));
            add.set_sensitive(catalog.id == store.state.draft.catalog_id);
            add.set_tooltip_text(Some(if catalog.id == store.state.draft.catalog_id {
                "افزودن یک واحد از این کالا به پیش‌فاکتور، با همین قیمت منبع"
            } else {
                "ابتدا فهرست را انتخاب کنید؛ انتخاب فهرست نیازمند بررسی دوبارهٔ قیمت اقلام است"
            }));
            item.add_suffix(&add);
            item.set_activatable_widget(Some(&add));
            let catalog_id = catalog.id.clone();
            let code = product.code.clone();
            let weak = Rc::downgrade(self);
            add.connect_clicked(move |_| {
                if let Some(ui) = weak.upgrade() {
                    let result = ui
                        .save_form()
                        .and_then(|()| ui.preserve_draft("revision"))
                        .and_then(|()| {
                            ui.store.borrow_mut().change(|state| {
                                anyhow::ensure!(
                                    state.draft.catalog_id == catalog_id,
                                    "فهرست انتخاب‌شده تغییر کرده؛ کالا را دوباره بررسی کنید"
                                );
                                state.draft.lines.push(OrderLine {
                                    query: code.clone(),
                                    quantity_milli: 1000,
                                    chosen_code: Some(code.clone()),
                                });
                                state.draft.stale_ack = false;
                                Ok(())
                            })
                        });
                    let ok = result.is_ok();
                    ui.message(result.map(|()| "کالا با قیمت منبع به پیش‌فاکتور افزوده و ذخیره شد"));
                    if ok {
                        ui.fill_form();
                        ui.visual_pending.set(true);
                        ui.refresh_review();
                        ui.stack.set_visible_child_name("order");
                    }
                }
            });
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
                "نسخهٔ تأییدشده پیدا نشد",
                "عبارت یا فیلتر را تغییر دهید. پیش‌فاکتورها پس از تأیید نسخه اینجا ذخیره می‌شوند.",
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
                    "تأیید {}{}",
                    ltr(&self.manager_date(&quote.approved_at)),
                    if quote.reference.is_empty() {
                        String::new()
                    } else {
                        format!(" · مرجع: {}", quote.reference)
                    }
                ))
                .build();
            expander.set_use_markup(false);
            expander.set_title_lines(2);
            expander.set_subtitle_lines(2);
            expander.set_tooltip_text(Some(&format!(
                "شناسهٔ ذخیره‌شده: {}\nزمان دقیق تأیید: {}",
                quote.id, quote.approved_at
            )));
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
                        "تاریخ پیش‌فاکتور ثبت نشده".into()
                    } else {
                        format!(
                            "تاریخ پیش‌فاکتور {}",
                            ltr(&self.manager_date(&quote.quote_date))
                        )
                    },
                    if quote.valid_until.is_empty() {
                        String::new()
                    } else {
                        format!(
                            " · اعتبار تا {}",
                            ltr(&self.manager_date(&quote.valid_until))
                        )
                    }
                ),
            );
            expander.add_row(&details);
            let actions = wrapping_actions();
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
                ("کپی در پیش‌فاکتور تازه", "duplicate"),
            ] {
                let action = compact_button(caption);
                action.set_tooltip_text(Some(match kind {
                    "preview" => "بررسی سند همین نسخهٔ تأییدشده",
                    "pdf" => "خروجی PDF از همین نسخه و قیمت‌های بایگانی‌شده",
                    "csv" => "خروجی CSV از همین نسخه و قیمت‌های بایگانی‌شده",
                    "revise" => "ویرایش در همان شمارهٔ پیش‌فاکتور؛ نسخهٔ پیشین حفظ می‌شود",
                    _ => "پیش‌فاکتور تازه با شمارهٔ تازه؛ قیمت‌های این نسخه حفظ می‌شود",
                }));
                actions.insert(&action, -1);
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
                                        "پیش‌نویس پیش‌فاکتور تازه از نسخهٔ بایگانی ساخته شد"
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
            expander.set_expanded(false);
            list.append(&expander);
        }
        drop(store);
        if start + PAGE_SIZE < ids.len() {
            let more = action_row(
                "پیش‌فاکتورهای بیشتر",
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
        let title = format!(
            "پیش‌نمایش چاپ · نسخهٔ تأییدشدهٔ {}",
            self.manager_number(quote.version)
        );
        self.show_quote_preview(quote.clone(), &title);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restored_catalogue_row_order_cannot_reuse_wrong_search_indices() {
        let original = imports::catalog_csv(
            "code,name,measure,price,currency,source_at,aliases\nCODE-A,کاغذ,بسته,1251,rial,2026-10-07T00:00:00Z,نام اول\nCODE-B,جوهر,عدد,900,rial,2026-10-07T00:00:00Z,نام دوم",
            "منبع نمونه",
        ).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let mut store = Store::open(directory.path()).unwrap();
        store
            .change(|state| imports::accept_catalog(state, original.clone()))
            .unwrap();
        assert_eq!(
            catalog_page(store.state.catalog().unwrap(), "CODE-A", 0, 100).0,
            vec![0]
        );
        let mut candidate = store.state.clone();
        candidate.catalogs[0].products.reverse();
        assert_eq!(candidate.catalogs[0].id, original.id);
        assert_eq!(
            candidate.catalogs[0].fingerprint,
            imports::product_fingerprint(&candidate.catalogs[0].products).unwrap()
        );
        store.restore(candidate).unwrap();
        let before = serde_json::to_vec(&store.state).unwrap();
        let restored = store.state.catalog().unwrap();
        let (indices, total) = catalog_page(restored, "CODE-A", 0, 100);
        assert_eq!(total, 1);
        assert_eq!(indices, vec![1]);
        assert_eq!(restored.products[indices[0]].code, "CODE-A");
        assert_eq!(restored.products[indices[0]].price_rial, 1251);
        assert_eq!(catalog_page(restored, "CODE-B", 0, 100).0, vec![0]);
        assert_eq!(serde_json::to_vec(&store.state).unwrap(), before);
    }

    #[test]
    fn manager_dates_obey_preferences_without_exposing_storage_timestamp() {
        let source = "2026-10-07T03:18:53.969580+00:00";
        assert_eq!(
            display_date(source, DisplayDigits::Latin, DateDisplay::Iso),
            "2026-10-07"
        );
        assert_eq!(
            display_date(source, DisplayDigits::Persian, DateDisplay::DayMonthYear),
            "۰۷/۱۰/۲۰۲۶"
        );
    }
}
