use super::Ui;
use adw::prelude::*;
use anyhow::{Context, Result};
use sarv_quote::domain::*;
use std::rc::Rc;

fn entry(title: &str, value: &str) -> adw::EntryRow {
    let row = adw::EntryRow::builder().title(title).text(value).build();
    row.set_direction(gtk::TextDirection::Rtl);
    row
}
fn combo(title: &str, choices: &[&str], selected: u32) -> adw::ComboRow {
    adw::ComboRow::builder()
        .title(title)
        .model(&gtk::StringList::new(choices))
        .selected(selected)
        .build()
}
fn multiline(group: &adw::PreferencesGroup, title: &str, value: &str) -> gtk::TextView {
    let text = gtk::TextView::builder()
        .wrap_mode(gtk::WrapMode::WordChar)
        .accepts_tab(false)
        .top_margin(8)
        .bottom_margin(8)
        .left_margin(12)
        .right_margin(12)
        .height_request(80)
        .build();
    text.set_direction(gtk::TextDirection::Rtl);
    text.set_accepts_tab(false);
    text.buffer().set_text(value);
    let frame = gtk::ScrolledWindow::builder()
        .child(&text)
        .min_content_height(80)
        .max_content_height(160)
        .propagate_natural_height(true)
        .build();
    frame.add_css_class("frame");
    let container = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let label = gtk::Label::new(Some(title));
    label.set_xalign(0.0);
    label.add_css_class("heading");
    text.update_property(&[gtk::accessible::Property::Label(title)]);
    container.set_margin_top(12);
    container.set_margin_bottom(12);
    container.append(&label);
    container.append(&frame);
    group.add(&container);
    text
}
fn validate_field(candidate: &Settings, widget: &impl IsA<gtk::Widget>) -> Result<()> {
    match candidate.validate() {
        Ok(()) => {
            widget.remove_css_class("error");
            Ok(())
        }
        Err(error) => {
            widget.add_css_class("error");
            widget.grab_focus();
            Err(error)
        }
    }
}
fn contents(text: &gtk::TextView) -> String {
    text.buffer()
        .text(&text.buffer().start_iter(), &text.buffer().end_iter(), true)
        .into()
}
/// Preferences are staged until Save succeeds in the same atomic store transaction.
pub(super) fn build(ui: &Rc<Ui>) -> gtk::Widget {
    let settings = ui.store.borrow().state.settings.clone();
    let page = adw::PreferencesPage::builder().title("تنظیمات").build();
    page.set_widget_name("settings-page");
    let save_group = adw::PreferencesGroup::new();
    let save_row = adw::ActionRow::builder()
        .title("تنظیمات برنامه")
        .subtitle("Ctrl+S برای ذخیره • تغییرات به اسناد قبلی منتقل نمی‌شوند.")
        .build();
    let save = gtk::Button::with_label("ذخیرهٔ تنظیمات");
    save.set_widget_name("save-settings");
    save.add_css_class("suggested-action");
    save.set_valign(gtk::Align::Center);
    let discard = gtk::Button::with_label("بازگرداندن");
    discard.set_widget_name("discard-settings");
    discard.set_tooltip_text(Some("بازگرداندن همهٔ فیلدها به آخرین تنظیمات ذخیره‌شده"));
    discard.set_sensitive(false);
    discard.set_valign(gtk::Align::Center);
    save_row.add_suffix(&discard);
    save_row.add_suffix(&save);
    let feedback = gtk::Label::new(Some("تنظیمات ذخیره‌شده"));
    feedback.set_widget_name("settings-feedback");
    feedback.set_xalign(0.0);
    feedback.set_wrap(true);
    feedback.set_margin_top(8);
    feedback.set_margin_bottom(4);
    feedback.add_css_class("caption");
    save_group.add(&save_row);
    save_group.add(&feedback);
    page.add(&save_group);
    let business = adw::PreferencesGroup::builder()
        .title("اطلاعات کسب‌وکار")
        .description(
            "این اطلاعات در پیشنهادهای تازه و خروجی آن‌ها درج می‌شود؛ اسناد قبلی تغییر نمی‌کنند.",
        )
        .build();
    let name = entry("نام کسب‌وکار", &settings.business_name);
    let phone = entry("تلفن کسب‌وکار", &settings.business_phone);
    phone.set_direction(gtk::TextDirection::Ltr);
    business.add(&name);
    business.add(&phone);
    let address = multiline(&business, "نشانی", &settings.business_address);
    page.add(&business);
    let document = adw::PreferencesGroup::builder()
        .title("سند و شرایط")
        .description("متن ساده؛ حداکثر ۴۰۰۰ نویسه برای هر بخش.")
        .build();
    let header = multiline(&document, "متن سربرگ", &settings.header);
    let footer = multiline(&document, "متن پایانی", &settings.footer);
    let terms = multiline(
        &document,
        "شرایط پیش‌فرض پیشنهاد تازه",
        &settings.default_terms,
    );
    let defaults = adw::PreferencesGroup::builder().title("پیش‌فرض پیشنهاد تازه")
        .description("۱ تومان = ۱۰ ریال؛ تغییر واحد، مبلغ دقیق را حفظ می‌کند. پیش‌فرض‌ها فقط روی پیشنهاد تازه اعمال می‌شوند.").build();
    let currency = combo(
        "واحد مبلغ",
        &["تومان", "ریال"],
        u32::from(settings.default_currency == Unit::Rial),
    );
    let prefix = entry("پیشوند شمارهٔ پیشنهاد", &settings.quote_prefix);
    prefix.set_direction(gtk::TextDirection::Ltr);
    let next = entry(
        "شمارهٔ بعدی (پس از نخستین تأیید افزایش می‌یابد)",
        &settings.next_number.to_string(),
    );
    next.set_direction(gtk::TextDirection::Ltr);
    let days = adw::SpinRow::with_range(0.0, 3650.0, 1.0);
    days.set_title("اعتبار پیش‌فرض (روز)");
    days.set_value(f64::from(settings.default_validity_days));
    let discount = entry(
        "تخفیف پیش‌فرض در واحد مبلغ انتخاب‌شده",
        &settings
            .default_currency
            .number(settings.default_discount_rial),
    );
    discount.set_direction(gtk::TextDirection::Ltr);
    let previous_unit = std::rc::Rc::new(std::cell::Cell::new(settings.default_currency));
    let converting = std::rc::Rc::new(std::cell::Cell::new(false));
    let weak_unit_ui = Rc::downgrade(ui);
    let discount_unit = discount.clone();
    let unit_feedback = feedback.clone();
    currency.connect_selected_notify(move |choice| {
        if converting.get() {
            return;
        }
        let unit = if choice.selected() == 0 {
            Unit::Toman
        } else {
            Unit::Rial
        };
        let previous = previous_unit.get();
        if unit == previous {
            return;
        }
        match previous.nonnegative(discount_unit.text().trim()) {
            Ok(rial) => {
                discount_unit.remove_css_class("error");
                discount_unit.set_text(&unit.number(rial));
                previous_unit.set(unit);
            }
            Err(error) => {
                discount_unit.add_css_class("error");
                discount_unit.grab_focus();
                converting.set(true);
                choice.set_selected(u32::from(previous == Unit::Rial));
                converting.set(false);
                unit_feedback.add_css_class("error");
                unit_feedback.set_text(&format!("{error:#}"));
                if let Some(ui) = weak_unit_ui.upgrade() {
                    ui.message(Err::<&str, _>(error));
                }
            }
        }
    });
    let tax = adw::SpinRow::with_range(0.0, 10_000.0, 1.0);
    tax.set_title("نرخ افزودهٔ قراردادی (یک‌صدم درصد؛ ۱۰۰ = ۱٪)");
    tax.set_subtitle(
        "نرخ را خودتان تعیین می‌کنید؛ برنامه نرخ قانونی یا انطباق مالیاتی را تأیید نمی‌کند.",
    );
    tax.set_value(f64::from(settings.tax_basis_points));
    for row in [
        &currency.clone().upcast::<gtk::Widget>(),
        &days.clone().upcast(),
        &discount.clone().upcast(),
        &tax.clone().upcast(),
    ] {
        defaults.add(row);
    }
    page.add(&defaults);
    let numbering = adw::PreferencesGroup::builder()
        .title("شماره‌گذاری")
        .description(
            "شماره فقط با تأیید پیشنهاد تازه مصرف می‌شود؛ اصلاح نسخه، شمارهٔ سند را نگه می‌دارد.",
        )
        .build();
    numbering.add(&prefix);
    numbering.add(&next);
    let number_preview = adw::ActionRow::builder()
        .title("شمارهٔ پیشنهاد تازه")
        .subtitle(format!("{}{}", settings.quote_prefix, settings.next_number))
        .use_markup(false)
        .build();
    number_preview.set_activatable(false);
    numbering.add(&number_preview);
    for row in [&prefix, &next] {
        let preview = number_preview.downgrade();
        let prefix = prefix.downgrade();
        let next = next.downgrade();
        row.connect_changed(move |_| {
            if let (Some(preview), Some(prefix), Some(next)) =
                (preview.upgrade(), prefix.upgrade(), next.upgrade())
            {
                preview.set_subtitle(&format!("{}{}", prefix.text(), next.text()));
            }
        });
    }
    page.add(&numbering);
    let appearance_group = adw::PreferencesGroup::builder().title("نمایش").build();
    let appearance = combo(
        "ظاهر",
        &["پیروی از سیستم", "روشن", "تیره"],
        match settings.appearance {
            Appearance::System => 0,
            Appearance::Light => 1,
            Appearance::Dark => 2,
        },
    );
    let density = combo(
        "تراکم جدول‌ها",
        &["راحت", "فشرده"],
        u32::from(settings.density == Density::Compact),
    );
    let digit_choice = combo(
        "نمایش ارقام",
        &["فارسی", "لاتین"],
        u32::from(settings.display_digits == DisplayDigits::Latin),
    );
    let date_choice = combo(
        "نمایش تاریخ",
        &[
            "میلادی YYYY-MM-DD",
            "میلادی DD/MM/YYYY",
            "هجری شمسی YYYY/MM/DD",
        ],
        match settings.date_display {
            DateDisplay::Iso => 0,
            DateDisplay::DayMonthYear => 1,
            DateDisplay::Jalali => 2,
        },
    );
    appearance_group.add(&appearance);
    appearance_group.add(&density);
    appearance_group.add(&digit_choice);
    date_choice.set_widget_name("settings-date-display");
    date_choice.set_subtitle(
        "تقویم نمایش و پیش‌فرض ورود تاریخ مالی؛ تاریخ واقعی سند و مرز دوره تغییر نمی‌کند.",
    );
    appearance_group.add(&date_choice);
    page.add(&appearance_group);
    page.add(&document);
    let local_data = adw::PreferencesGroup::builder()
        .title("داده‌ها و پشتیبان")
        .description("اطلاعات در همین رایانه ذخیره می‌شود؛ پیش از جابه‌جایی یک پشتیبان تهیه کنید.")
        .build();
    let location = adw::ActionRow::builder()
        .title("محل ذخیرهٔ داده‌ها")
        .subtitle(ui.store.borrow().directory.display().to_string())
        .use_markup(false)
        .subtitle_selectable(true)
        .subtitle_lines(3)
        .build();
    local_data.add(&location);
    let recovery = adw::ActionRow::builder()
        .title("پشتیبان و بازیابی")
        .subtitle("ساخت فایل پشتیبان یا بررسی فایل برای بازیابی")
        .build();
    let open_recovery = gtk::Button::with_label("باز کردن");
    open_recovery.set_widget_name("settings-open-backup");
    open_recovery.set_valign(gtk::Align::Center);
    recovery.add_suffix(&open_recovery);
    recovery.set_activatable_widget(Some(&open_recovery));
    let weak = Rc::downgrade(ui);
    open_recovery.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            ui.stack.set_visible_child_name("backup");
            ui.split.set_show_content(true);
        }
    });
    local_data.add(&recovery);
    page.add(&local_data);
    // Keep the same saved payload until the explicit atomic save succeeds.
    // Weak page references avoid retaining a replaced PreferencesPage.
    let weak_page = page.downgrade();
    let dirty_feedback = feedback.clone();
    let dirty_discard = discard.clone();
    let mark_dirty: Rc<dyn Fn()> = Rc::new(move || {
        if let Some(page) = weak_page.upgrade() {
            page.add_css_class("settings-dirty");
        }
        dirty_discard.set_sensitive(true);
        // Keep a validation error visible while the user corrects the fields.
        if !dirty_feedback.has_css_class("error") {
            dirty_feedback.set_text("تغییرات ذخیره نشده‌اند • Ctrl+S برای ذخیره");
        }
    });
    for row in [&name, &phone, &prefix, &next, &discount] {
        let dirty = mark_dirty.clone();
        row.connect_changed(move |row| {
            row.remove_css_class("error");
            dirty();
        });
    }
    for text in [&address, &header, &footer, &terms] {
        let dirty = mark_dirty.clone();
        let text = text.downgrade();
        if let Some(text_view) = text.upgrade() {
            text_view.buffer().connect_changed(move |_| {
                if let Some(text) = text.upgrade() {
                    text.remove_css_class("error");
                }
                dirty();
            });
        }
    }
    for row in [
        &currency,
        &appearance,
        &density,
        &digit_choice,
        &date_choice,
    ] {
        let dirty = mark_dirty.clone();
        row.connect_selected_notify(move |_| dirty());
    }
    for row in [&days, &tax] {
        let dirty = mark_dirty.clone();
        row.connect_value_notify(move |_| dirty());
    }
    let weak_discard = Rc::downgrade(ui);
    discard.connect_clicked(move |_| {
        if let Some(ui) = weak_discard.upgrade() {
            discard_staged(&ui);
            ui.message(Ok("فیلدهای تنظیمات به آخرین مقادیر ذخیره‌شده بازگشتند."));
        }
    });
    let saved_next = std::cell::Cell::new(settings.next_number);
    let weak = Rc::downgrade(ui);
    let weak_page = page.downgrade();
    save.connect_clicked(move |button| {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        button.set_sensitive(false);
        let result = (|| -> Result<&str> {
            // Validate fields using the domain rules before constructing the full
            // payload, so the first invalid field keeps its contents and focus.
            for (value, field) in [
                (name.text().trim().to_owned(), &name),
                (phone.text().trim().to_owned(), &phone),
                (prefix.text().to_string(), &prefix),
            ] {
                let mut candidate = Settings::default();
                if field == &name {
                    candidate.business_name = value;
                } else if field == &phone {
                    candidate.business_phone = value;
                } else {
                    candidate.quote_prefix = value;
                }
                validate_field(&candidate, field).with_context(|| field.title().to_string())?;
            }
            for (index, text) in [&address, &header, &footer, &terms].into_iter().enumerate() {
                let mut candidate = Settings::default();
                let value = contents(text);
                match index {
                    0 => candidate.business_address = value,
                    1 => candidate.header = value,
                    2 => candidate.footer = value,
                    _ => candidate.default_terms = value,
                }
                validate_field(&candidate, text).context(
                    [
                        "نشانی کسب‌وکار",
                        "متن سربرگ",
                        "متن پایانی",
                        "شرایط پیش‌فرض پیشنهاد تازه",
                    ][index],
                )?;
            }
            let unit = if currency.selected() == 0 {
                Unit::Toman
            } else {
                Unit::Rial
            };
            anyhow::ensure!(
                days.value().fract() == 0.0 && tax.value().fract() == 0.0,
                "روز اعتبار و واحد نرخ باید عدد صحیح باشند"
            );
            let requested_next: u64 = digits(next.text().trim())
                .parse()
                .context("شمارهٔ بعدی باید یک عدد صحیح مثبت باشد")
                .inspect_err(|_| {
                    next.add_css_class("error");
                    next.grab_focus();
                })?;
            next.remove_css_class("error");
            let next_number = if requested_next == saved_next.get() {
                ui.store.borrow().state.settings.next_number
            } else {
                requested_next
            };
            let candidate = Settings {
                next_number,
                ..Settings::default()
            };
            validate_field(&candidate, &next)?;
            let number = format!("{}{next_number}", prefix.text());
            if ui
                .store
                .borrow()
                .state
                .quotes
                .iter()
                .any(|quote| normalize(&quote.number) == normalize(&number))
            {
                next.add_css_class("error");
                next.grab_focus();
                anyhow::bail!(
                    "شمارهٔ {number} قبلاً استفاده شده است؛ شمارهٔ بعدی یا پیشوند را تغییر دهید"
                );
            }
            let settings = Settings {
                business_name: name.text().trim().into(),
                business_phone: phone.text().trim().into(),
                business_address: contents(&address),
                header: contents(&header),
                footer: contents(&footer),
                default_terms: contents(&terms),
                default_currency: unit,
                quote_prefix: prefix.text().into(),
                next_number,
                default_validity_days: days.value() as u32,
                default_discount_rial: unit.nonnegative(discount.text().trim()).inspect_err(
                    |_| {
                        discount.add_css_class("error");
                        discount.grab_focus();
                    },
                )?,
                tax_basis_points: tax.value() as u32,
                appearance: match appearance.selected() {
                    1 => Appearance::Light,
                    2 => Appearance::Dark,
                    _ => Appearance::System,
                },
                density: if density.selected() == 0 {
                    Density::Comfortable
                } else {
                    Density::Compact
                },
                date_display: match date_choice.selected() {
                    1 => DateDisplay::DayMonthYear,
                    2 => DateDisplay::Jalali,
                    _ => DateDisplay::Iso,
                },
                display_digits: if digit_choice.selected() == 0 {
                    DisplayDigits::Persian
                } else {
                    DisplayDigits::Latin
                },
            };
            settings.validate()?;
            discount.remove_css_class("error");
            ui.store.borrow_mut().change(|state| {
                state.settings = settings;
                Ok(())
            })?;
            saved_next.set(next_number);
            next.set_text(&next_number.to_string());
            apply(&ui);
            if let Some(page) = weak_page.upgrade() {
                page.remove_css_class("settings-dirty");
            }
            discard.set_sensitive(false);
            Ok("تنظیمات ذخیره شد؛ پیش‌فرض‌ها روی پیشنهاد تازه اعمال می‌شوند.")
        })();
        match &result {
            Ok(message) => {
                feedback.remove_css_class("error");
                feedback.set_text(message);
            }
            Err(error) => {
                feedback.add_css_class("error");
                feedback.set_text(&format!("ذخیره انجام نشد: {error:#}"));
            }
        }
        ui.message(result);
        button.set_sensitive(true);
    });
    page.upcast()
}

pub(super) fn has_unsaved(ui: &Ui) -> bool {
    ui.stack
        .child_by_name("settings")
        .is_some_and(|page| page.has_css_class("settings-dirty"))
}

pub(super) fn discard_staged(ui: &Rc<Ui>) {
    let visible = ui.stack.visible_child_name();
    ui.refresh_settings();
    if let Some(visible) = visible {
        ui.stack.set_visible_child_name(&visible);
    }
}

/// Apply real Adwaita appearance, table density and label digit preferences.
pub(super) fn apply_digits(widget: &impl IsA<gtk::Widget>, choice: DisplayDigits) {
    let widget = widget.as_ref();
    if let Some(label) = widget.downcast_ref::<gtk::Label>() {
        let text = label.text();
        let formatted = choice.format(&text);
        if text.as_str() != formatted {
            label.set_text(&formatted);
        }
    }
    let mut child = widget.first_child();
    while let Some(current) = child {
        child = current.next_sibling();
        apply_digits(&current, choice);
    }
}

pub(super) fn apply_manager_digits(ui: &Rc<Ui>) {
    let choice = ui.store.borrow().state.settings.display_digits;
    for (operation, root) in [
        ("settings_catalog_digits", &ui.catalog_list),
        ("settings_customer_digits", &ui.customer_list),
        ("settings_quote_digits", &ui.quote_list),
    ] {
        let started = std::time::Instant::now();
        apply_digits(root, choice);
        super::editor::performance_trace(operation, started);
    }
}

pub(super) fn apply(ui: &Rc<Ui>) {
    let apply_started = std::time::Instant::now();
    let settings = ui.store.borrow().state.settings.clone();
    let scheme = match settings.appearance {
        Appearance::System => adw::ColorScheme::Default,
        Appearance::Light => adw::ColorScheme::ForceLight,
        Appearance::Dark => adw::ColorScheme::ForceDark,
    };
    adw::StyleManager::default().set_color_scheme(scheme);
    match settings.density {
        Density::Compact => ui.window.add_css_class("compact-density"),
        Density::Comfortable => ui.window.remove_css_class("compact-density"),
    }
    thread_local! { static PROVIDER: std::cell::OnceCell<gtk::CssProvider> = const { std::cell::OnceCell::new() }; }
    PROVIDER.with(|cell| { cell.get_or_init(|| {
        let provider = gtk::CssProvider::new();
        provider.load_from_data(".compact-density .table-row { min-height: 28px; padding-top: 2px; padding-bottom: 2px; } .compact-density .table-cell { padding-top: 3px; padding-bottom: 3px; } .compact-density .cell-editor { min-height: 24px; padding-top: 2px; padding-bottom: 2px; } .compact-density .review-choice button { min-height: 24px; padding-top: 2px; padding-bottom: 2px; }");
        gtk::style_context_add_provider_for_display(&gtk::prelude::WidgetExt::display(&ui.window), &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1);
        provider
    }); });
    let digits_started = std::time::Instant::now();
    apply_digits(&ui.window, settings.display_digits);
    super::editor::performance_trace("settings_global_digits", digits_started);
    super::editor::performance_trace("settings_global_apply", apply_started);
}
