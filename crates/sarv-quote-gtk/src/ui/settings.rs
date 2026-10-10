use super::Ui;
use adw::prelude::*;
use anyhow::{Context, Result};
use sarv_quote::domain::*;
use std::rc::Rc;

fn rate_percent(basis_points: u32) -> String {
    format!("{}.{:02}", basis_points / 100, basis_points % 100)
}

/// Percent input is presentation only; the saved rate stays exact integer basis points.
fn parse_rate_percent(input: &str) -> Result<u32> {
    let input = digits(input.trim()).replace('٫', ".");
    anyhow::ensure!(
        !input.is_empty() && input.len() <= 16,
        "نرخ را بین صفر و صد درصد وارد کنید"
    );
    let (whole, fractional) = input.split_once('.').unwrap_or((&input, ""));
    anyhow::ensure!(
        !whole.is_empty()
            && whole.bytes().all(|c| c.is_ascii_digit())
            && fractional.len() <= 2
            && fractional.bytes().all(|c| c.is_ascii_digit())
            && (!input.contains('.') || !fractional.is_empty()),
        "نرخ باید عددی با حداکثر دو رقم اعشار باشد"
    );
    let whole: u32 = whole
        .parse()
        .context("نرخ را بین صفر و صد درصد وارد کنید")?;
    let fractional: u32 = if fractional.is_empty() {
        0
    } else {
        let value: u32 = fractional.parse()?;
        if fractional.len() == 1 {
            value * 10
        } else {
            value
        }
    };
    anyhow::ensure!(whole <= 100, "نرخ را بین صفر و صد درصد وارد کنید");
    let basis_points = whole * 100 + fractional;
    anyhow::ensure!(basis_points <= 10_000, "نرخ را بین صفر و صد درصد وارد کنید");
    Ok(basis_points)
}

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
        .subtitle("Ctrl+S برای ذخیرهٔ تغییرات")
        .build();
    let save = gtk::Button::with_label("ذخیرهٔ تنظیمات");
    save.set_widget_name("save-settings");
    save.add_css_class("suggested-action");
    save.set_valign(gtk::Align::Center);
    let discard = gtk::Button::with_label("لغو تغییرات");
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
        .title("اطلاعات کسب‌وکار در پیش‌فاکتور")
        .description(
            "برای چاپ پیش‌فاکتورهای جدید؛ اطلاعات شرکت حسابداری در بخش حسابداری تعریف می‌شود.",
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
        .title("سربرگ و شرایط پیش‌فاکتور")
        .build();
    let header = multiline(&document, "متن سربرگ", &settings.header);
    let footer = multiline(&document, "متن پایانی", &settings.footer);
    let terms = multiline(&document, "شرایط پیش‌فاکتور", &settings.default_terms);
    let defaults = adw::PreferencesGroup::builder()
        .title("پیش‌فرض‌های پیش‌فاکتور")
        .description(
            "برای پیش‌فاکتورهای جدید. ۱ تومان = ۱۰ ریال؛ تغییر واحد، مبلغ تخفیف را تبدیل می‌کند.",
        )
        .build();
    let currency = combo(
        "واحد مبلغ",
        &["تومان", "ریال"],
        u32::from(settings.default_currency == Unit::Rial),
    );
    let prefix = entry("پیشوند شماره", &settings.quote_prefix);
    prefix.set_direction(gtk::TextDirection::Ltr);
    let next = entry("شمارهٔ بعدی", &settings.next_number.to_string());
    next.set_direction(gtk::TextDirection::Ltr);
    let days = adw::SpinRow::with_range(0.0, 3650.0, 1.0);
    days.set_title("مدت اعتبار (روز)");
    days.set_value(f64::from(settings.default_validity_days));
    let discount = entry(
        "تخفیف پیش‌فرض",
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
    let tax = entry(
        "نرخ افزودهٔ پیش‌فاکتور (%)",
        &rate_percent(settings.tax_basis_points),
    );
    tax.set_widget_name("settings-quote-rate");
    tax.set_direction(gtk::TextDirection::Ltr);
    tax.set_tooltip_text(Some(
        "نرخ دلخواه، فقط برای پیش‌فاکتور؛ برای نمونه ۹٫۷۵ یعنی ۹٫۷۵٪.",
    ));
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
        .title("شماره‌گذاری پیش‌فاکتور")
        .description(
            "شماره پس از تأیید پیش‌فاکتور جدید افزایش می‌یابد؛ اصلاح نسخه همان شماره را نگه می‌دارد.",
        )
        .build();
    numbering.add(&prefix);
    numbering.add(&next);
    let number_preview = adw::ActionRow::builder()
        .title("شمارهٔ پیش‌فاکتور بعدی")
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
        "تم برنامه",
        &["سیستم", "روشن", "تیره"],
        match settings.appearance {
            Appearance::System => 0,
            Appearance::Light => 1,
            Appearance::Dark => 2,
        },
    );
    let density = combo(
        "تراکم جدول",
        &["معمولی", "فشرده"],
        u32::from(settings.density == Density::Compact),
    );
    let digit_choice = combo(
        "نمایش ارقام",
        &["فارسی", "لاتین"],
        u32::from(settings.display_digits == DisplayDigits::Latin),
    );
    let date_choice = combo(
        "قالب تاریخ",
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
    date_choice.set_subtitle("قالب نمایش و ورود تاریخ؛ تاریخ اسناد ثبت‌شده تغییر نمی‌کند.");
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
            ui.show_workspace_content();
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
    for row in [&name, &phone, &prefix, &next, &discount, &tax] {
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
    let dirty = mark_dirty.clone();
    days.connect_value_notify(move |_| dirty());
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
                        "شرایط پیش‌فرض پیش‌فاکتور تازه",
                    ][index],
                )?;
            }
            let unit = if currency.selected() == 0 {
                Unit::Toman
            } else {
                Unit::Rial
            };
            anyhow::ensure!(
                days.value().fract() == 0.0,
                "مدت اعتبار باید تعداد روز کامل باشد"
            );
            let tax_basis_points = parse_rate_percent(&tax.text()).inspect_err(|_| {
                tax.add_css_class("error");
                tax.grab_focus();
            })?;
            tax.remove_css_class("error");
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
                tax_basis_points,
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
            Ok("تنظیمات ذخیره شد.")
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
        provider.load_from_data(
            ".compact-density .table-row { min-height: 28px; padding-top: var(--sarv-space-1); padding-bottom: var(--sarv-space-1); } \
             .compact-density .table-cell { padding-top: var(--sarv-space-1); padding-bottom: var(--sarv-space-1); } \
             .compact-density .cell-editor { min-height: 24px; padding-top: var(--sarv-space-1); padding-bottom: var(--sarv-space-1); } \
             .compact-density .review-choice button { min-height: 24px; padding-top: var(--sarv-space-1); padding-bottom: var(--sarv-space-1); }",
        );
        gtk::style_context_add_provider_for_display(&gtk::prelude::WidgetExt::display(&ui.window), &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1);
        provider
    }); });
    let digits_started = std::time::Instant::now();
    apply_digits(&ui.window, settings.display_digits);
    super::editor::performance_trace("settings_global_digits", digits_started);
    super::editor::performance_trace("settings_global_apply", apply_started);
}

#[cfg(test)]
mod rate_tests {
    use super::{parse_rate_percent, rate_percent};

    #[test]
    fn every_existing_rate_round_trips_without_rounding_or_float_money() {
        for basis_points in 0..=10_000 {
            assert_eq!(
                parse_rate_percent(&rate_percent(basis_points)).unwrap(),
                basis_points
            );
        }
    }

    #[test]
    fn percent_entry_accepts_persian_arabic_and_latin_digits_exactly() {
        for text in ["9.75", "۹٫۷۵", "٩٫٧٥", " ۹.۷۵ "] {
            assert_eq!(parse_rate_percent(text).unwrap(), 975);
        }
        assert_eq!(parse_rate_percent("0.01").unwrap(), 1);
        assert_eq!(parse_rate_percent("9.5").unwrap(), 950);
        assert_eq!(parse_rate_percent("100").unwrap(), 10_000);
        assert_eq!(parse_rate_percent("۰").unwrap(), 0);
    }

    #[test]
    fn invalid_or_over_precise_rates_are_rejected_instead_of_truncated() {
        for text in [
            "",
            "-1",
            "+1",
            "100.01",
            "101",
            "0.001",
            "9.750",
            "1e2",
            "NaN",
            "inf",
            "1,50",
            "1..0",
            "10%",
            "10.",
            "0x10",
            "9999999999999999",
            "9\u{202e}.75",
        ] {
            assert!(parse_rate_percent(text).is_err(), "accepted {text:?}");
        }
    }
}
