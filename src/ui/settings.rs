use super::Ui;
use adw::prelude::*;
use anyhow::Result;
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
        .top_margin(8)
        .bottom_margin(8)
        .left_margin(12)
        .right_margin(12)
        .height_request(80)
        .build();
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
fn contents(text: &gtk::TextView) -> String {
    text.buffer()
        .text(&text.buffer().start_iter(), &text.buffer().end_iter(), true)
        .into()
}
/// Preferences are staged until Save succeeds in the same atomic store transaction.
pub(super) fn build(ui: &Rc<Ui>) -> gtk::Widget {
    let settings = ui.store.borrow().state.settings.clone();
    let page = adw::PreferencesPage::builder().title("تنظیمات").build();
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
    page.add(&document);
    let defaults = adw::PreferencesGroup::builder().title("پیش‌فرض پیشنهاد تازه")
        .description("واحد، اعتبار، تخفیف و نرخ افزوده فقط روی پیش‌نویس‌های تازه اعمال می‌شوند. تاریخ‌ها میلادی YYYY-MM-DD هستند.").build();
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
        &prefix.clone().upcast(),
        &next.clone().upcast(),
        &days.clone().upcast(),
        &discount.clone().upcast(),
        &tax.clone().upcast(),
    ] {
        defaults.add(row);
    }
    page.add(&defaults);
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
        &["میلادی YYYY-MM-DD", "میلادی DD/MM/YYYY"],
        u32::from(settings.date_display == DateDisplay::DayMonthYear),
    );
    appearance_group.add(&appearance);
    appearance_group.add(&density);
    appearance_group.add(&digit_choice);
    appearance_group.add(&date_choice);
    page.add(&appearance_group);
    let save_group = adw::PreferencesGroup::new();
    let save_row = adw::ActionRow::builder()
        .title("ذخیرهٔ تنظیمات")
        .subtitle("تغییرات تا فشردن ذخیره اعمال نمی‌شوند.")
        .build();
    let save = gtk::Button::with_label("ذخیرهٔ تنظیمات");
    save.set_widget_name("save-settings");
    save.add_css_class("suggested-action");
    save.set_valign(gtk::Align::Center);
    save_row.add_suffix(&save);
    save_group.add(&save_row);
    page.add(&save_group);
    let saved_next = std::cell::Cell::new(settings.next_number);
    let weak = Rc::downgrade(ui);
    save.connect_clicked(move |button| {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        button.set_sensitive(false);
        let result = (|| -> Result<&str> {
            let unit = if currency.selected() == 0 {
                Unit::Toman
            } else {
                Unit::Rial
            };
            anyhow::ensure!(
                days.value().fract() == 0.0 && tax.value().fract() == 0.0,
                "روز اعتبار و واحد نرخ باید عدد صحیح باشند"
            );
            let requested_next: u64 = digits(next.text().trim()).parse().inspect_err(|_| {
                next.add_css_class("error");
                next.grab_focus();
            })?;
            next.remove_css_class("error");
            let next_number = if requested_next == saved_next.get() {
                ui.store.borrow().state.settings.next_number
            } else {
                requested_next
            };
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
                date_display: if date_choice.selected() == 0 {
                    DateDisplay::Iso
                } else {
                    DateDisplay::DayMonthYear
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
            Ok("تنظیمات ذخیره شد؛ پیش‌فرض‌ها روی پیشنهاد تازه اعمال می‌شوند.")
        })();
        ui.message(result);
        button.set_sensitive(true);
    });
    page.upcast()
}

/// Apply real Adwaita appearance, table density and label digit preferences.
pub(super) fn apply(ui: &Rc<Ui>) {
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
    fn labels(widget: &gtk::Widget, choice: DisplayDigits) {
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
            labels(&current, choice);
        }
    }
    labels(ui.window.upcast_ref(), settings.display_digits);
}
