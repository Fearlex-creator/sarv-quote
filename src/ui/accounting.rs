//! Native command-driven accounting workspace. No GTK callback mutates ledger rows.
use super::*;
use sarv_quote::accounting::*;
use sarv_quote::calendar::{Calendar, format_date, parse_date};
use std::rc::Weak;
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, mpsc},
    thread,
    time::Duration,
};

const ACTOR: &str = "local-single-user";
const PAGE_SIZE: usize = 100;

struct Page {
    ui: Weak<Ui>,
    tabs: gtk::Stack,
    company: gtk::DropDown,
    company_ids: RefCell<Vec<String>>,
    setup: gtk::Box,
    chart: gtk::Box,
    journals: gtk::Box,
    trial: gtk::Box,
    audit: gtk::Box,
    search: gtk::SearchEntry,
    chart_search: gtk::SearchEntry,
    audit_search: gtk::SearchEntry,
    chart_page: Cell<usize>,
    audit_page: Cell<usize>,
    trial_page: Cell<usize>,
    report_busy: Cell<bool>,
    report_revision: Cell<u64>,
    unit: gtk::DropDown,
    as_of: DateField,
    date_preference: Cell<DateDisplay>,
    journal_page: Cell<usize>,
    journal_account_filter: RefCell<Option<String>>,
    refreshing: Cell<bool>,
}
fn actions() -> gtk::FlowBox {
    let b = gtk::FlowBox::new();
    b.set_selection_mode(gtk::SelectionMode::None);
    b.set_min_children_per_line(1);
    b.set_max_children_per_line(4);
    b.set_column_spacing(8);
    b.set_row_spacing(8);
    b.set_halign(gtk::Align::Start);
    b
}
fn named_entry(caption: &str, name: &str, value: &str, ltr: bool) -> gtk::Entry {
    let e = entry(caption);
    e.set_widget_name(name);
    e.set_text(value);
    if ltr {
        e.set_direction(gtk::TextDirection::Ltr);
    }
    accessible_name(&e, caption);
    e
}
fn choice(caption: &str, options: &[String]) -> gtk::DropDown {
    let d = gtk::DropDown::from_strings(&options.iter().map(String::as_str).collect::<Vec<_>>());
    d.set_enable_search(true);
    accessible_name(&d, caption);
    d
}
fn selected(d: &gtk::DropDown, ids: &[String], title: &str) -> Result<String> {
    ids.get(d.selected() as usize)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("{title} را انتخاب کنید"))
}
fn isolate(value: &str) -> String {
    format!("\u{2068}{value}\u{2069}")
}
fn account_class(value: AccountClass) -> &'static str {
    match value {
        AccountClass::Asset => "دارایی",
        AccountClass::Liability => "بدهی",
        AccountClass::Equity => "حقوق مالکانه",
        AccountClass::Revenue => "درآمد",
        AccountClass::Expense => "هزینه",
    }
}
fn amount_unit(d: &gtk::DropDown) -> MoneyUnit {
    if d.selected() == 0 {
        MoneyUnit::Rial
    } else {
        MoneyUnit::Toman
    }
}
/// A financial date always names its input calendar. Switching calendars converts
/// the date; it cannot reinterpret a typed year or alter a stored journal.
#[derive(Clone)]
pub(super) struct DateField {
    pub(super) widget: gtk::Box,
    pub(super) entry: gtk::Entry,
    pub(super) calendar: gtk::DropDown,
    display_digits: DisplayDigits,
}
impl DateField {
    pub(super) fn set_canonical(&self, canonical: &str) -> Result<()> {
        let value = if canonical.is_empty() {
            String::new()
        } else {
            format_date(
                parse_date(canonical, Calendar::Gregorian)?,
                calendar_of(&self.calendar),
            )?
        };
        self.entry.set_text(&self.display_digits.format(value));
        Ok(())
    }
}
fn calendar_of(choice: &gtk::DropDown) -> Calendar {
    if choice.selected() == 1 {
        Calendar::Jalali
    } else {
        Calendar::Gregorian
    }
}
pub(super) fn date_field(ui: &Ui, caption: &str, name: &str, canonical: &str) -> DateField {
    let settings = &ui.store.borrow().state.settings;
    date_field_with_preferences(
        settings.date_display,
        settings.display_digits,
        caption,
        name,
        canonical,
    )
}
pub(super) fn date_field_with_preferences(
    dates: DateDisplay,
    display_digits: DisplayDigits,
    caption: &str,
    name: &str,
    canonical: &str,
) -> DateField {
    let calendar = gtk::DropDown::from_strings(&["میلادی • YYYY-MM-DD", "هجری شمسی • YYYY/MM/DD"]);
    calendar.set_widget_name(&format!("{name}-calendar"));
    accessible_name(&calendar, &format!("تقویم {caption}"));
    let selected = if dates == DateDisplay::Jalali {
        Calendar::Jalali
    } else {
        Calendar::Gregorian
    };
    calendar.set_selected(u32::from(selected == Calendar::Jalali));
    let value = parse_date(canonical, Calendar::Gregorian)
        .and_then(|d| format_date(d, selected))
        .unwrap_or_default();
    let text = named_entry(caption, name, &display_digits.format(value), true);
    text.set_placeholder_text(Some(if selected == Calendar::Jalali {
        "1405/01/01"
    } else {
        "2026-03-21"
    }));
    // Keep the selector, entry and equivalence stacked so narrow dialogs do not
    // obscure which calendar a field uses. Both controls remain in tab order.
    let widget = column();
    widget.append(&heading(caption));
    widget.append(&calendar);
    widget.append(&text);
    let feedback = label("");
    feedback.set_widget_name(&format!("{name}-equivalence"));
    feedback.add_css_class("caption");
    feedback.set_selectable(true);
    widget.append(&feedback);
    let current = Rc::new(Cell::new(selected));
    let switching = Rc::new(Cell::new(false));
    let entry = text.clone();
    let hint = feedback.clone();
    let prior = current.clone();
    let blocked = switching.clone();
    let title = caption.to_string();
    calendar.connect_selected_notify(move |choice| {
        if blocked.get() {
            return;
        }
        let next = calendar_of(choice);
        let previous = prior.get();
        if previous == next {
            return;
        }
        let result = if entry.text().trim().is_empty() {
            Ok(String::new())
        } else {
            parse_date(&entry.text(), previous).and_then(|date| format_date(date, next))
        };
        match result {
            Ok(value) => {
                prior.set(next);
                entry.set_placeholder_text(Some(if next == Calendar::Jalali {
                    "1405/01/01"
                } else {
                    "2026-03-21"
                }));
                entry.set_text(&display_digits.format(value));
                entry_validity(&entry, None, &title);
            }
            Err(error) => {
                blocked.set(true);
                choice.set_selected(u32::from(previous == Calendar::Jalali));
                blocked.set(false);
                let message = format!("{}؛ برای تغییر تقویم ابتدا تاریخ را اصلاح کنید", error);
                hint.set_text(&message);
                entry_validity(&entry, Some(&message), &title);
                entry.grab_focus();
            }
        }
    });
    let choice = calendar.downgrade();
    let hint = feedback.clone();
    let title = caption.to_string();
    let update = Rc::new(move |entry: &gtk::Entry| {
        let Some(choice) = choice.upgrade() else {
            return;
        };
        if entry.text().trim().is_empty() {
            hint.set_text("سال، ماه و روز را در تقویم انتخاب‌شده وارد کنید؛ ارقام فارسی، عربی و لاتین پذیرفته می‌شوند.");
            entry_validity(entry, None, &title);
            return;
        }
        match parse_date(&entry.text(), calendar_of(&choice)) {
            Ok(date) => {
                let gregorian = date.format("%Y-%m-%d").to_string();
                let jalali = format_date(date, Calendar::Jalali)
                    .unwrap_or_else(|_| "خارج از دامنهٔ پشتیبانی".into());
                hint.set_text(&format!(
                    "شمسی: {} • میلادی: {}",
                    isolate(&jalali),
                    isolate(&gregorian)
                ));
                entry_validity(entry, None, &title);
            }
            Err(error) => {
                hint.set_text(&error.to_string());
                entry_validity(entry, Some(&error.to_string()), &title);
            }
        }
    });
    update(&text);
    text.connect_changed(move |entry| update(entry));
    DateField {
        widget,
        entry: text,
        calendar,
        display_digits,
    }
}
fn clear_date_error_on_edit(field: &DateField, error: &gtk::Label) {
    let error = error.downgrade();
    field.entry.connect_changed(move |_| {
        if let Some(error) = error.upgrade() {
            error.set_visible(false);
        }
    });
}
pub(super) fn date_value(field: &DateField) -> Result<String> {
    let result = parse_date(&field.entry.text(), calendar_of(&field.calendar))
        .map(|date| date.format("%Y-%m-%d").to_string())
        .map_err(anyhow::Error::from);
    entry_validity(
        &field.entry,
        result.as_ref().err().map(|e| e.to_string()).as_deref(),
        "تاریخ مالی در تقویم انتخاب‌شده",
    );
    if result.is_err() {
        field.entry.grab_focus();
    }
    result
}
fn financial_date(ui: &Ui, canonical: &str) -> String {
    let settings = &ui.store.borrow().state.settings;
    let value = managers::display_date(canonical, settings.display_digits, settings.date_display);
    format!(
        "{} {}",
        isolate(&value),
        if settings.date_display == DateDisplay::Jalali {
            "شمسی"
        } else {
            "میلادی"
        }
    )
}
fn money_value(e: &gtk::Entry, unit: MoneyUnit) -> Result<Money> {
    let input = e.text();
    let result = Money::parse(
        if input.trim().is_empty() {
            "0"
        } else {
            input.trim()
        },
        unit,
    );
    entry_validity(
        e,
        result.as_ref().err().map(|e| e.to_string()).as_deref(),
        "مبلغ دقیق؛ کسر ریال و گردکردن پذیرفته نمی‌شود",
    );
    if result.is_err() {
        e.grab_focus();
    }
    result
}
fn form_dialog(
    ui: &Rc<Ui>,
    title: &str,
    name: &str,
) -> (adw::Dialog, gtk::Box, gtk::Label, gtk::Button) {
    let dialog = adw::Dialog::builder()
        .title(title)
        .content_width(680)
        .content_height(700)
        .build();
    dialog.set_widget_name(name);
    let content = column();
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(20);
    content.set_margin_end(20);
    let error = label("");
    error.set_widget_name("accounting-form-error");
    error.add_css_class("error");
    error.set_visible(false);
    content.append(&error);
    let scrolled = scroll(&content);
    let toolbar = dialog_toolbar(&scrolled);
    let save = button(title);
    save.set_widget_name(&format!("{name}-save"));
    save.add_css_class("suggested-action");
    let footer = row();
    footer.set_margin_top(12);
    footer.set_margin_bottom(12);
    footer.set_margin_start(20);
    footer.set_margin_end(20);
    let cancel = button("انصراف");
    let weak = dialog.downgrade();
    cancel.connect_clicked(move |_| {
        if let Some(d) = weak.upgrade() {
            d.close();
        }
    });
    footer.append(&save);
    footer.append(&cancel);
    toolbar.add_bottom_bar(&footer);
    dialog.set_child(Some(&toolbar));
    ui.present_dialog(&dialog);
    (dialog, content, error, save)
}
fn show_result<T>(
    result: Result<T>,
    dialog: &adw::Dialog,
    error: &gtk::Label,
    page: &Rc<Page>,
    message: &str,
) {
    match result {
        Ok(_) => {
            dialog.close();
            page.refresh();
            if let Some(ui) = page.ui.upgrade() {
                ui.message(Ok(message));
            }
        }
        Err(e) => {
            error.set_text(&format!("{e:#}"));
            error.set_visible(true);
        }
    }
}

pub(super) fn build(ui: &Rc<Ui>) -> gtk::Widget {
    let root = column();
    root.add_css_class("page");
    root.set_widget_name("accounting-page");
    root.append(&workspace::page_header(
        "دفتر حسابداری",
        Some("ثبت دستیِ دقیق و قابل‌ردیابی • ریال پایه • تک‌کاربر محلی"),
    ));
    let top = actions();
    let companies = choice("شرکت فعال", &[]);
    companies.set_widget_name("accounting-company");
    companies.set_hexpand(true);
    let add = button("شرکت تازه");
    add.set_widget_name("accounting-new-company");
    top.insert(&companies, -1);
    top.insert(&add, -1);
    root.append(&top);
    let tabs = gtk::Stack::new();
    tabs.set_vexpand(true);
    tabs.set_transition_type(gtk::StackTransitionType::Crossfade);
    // GTK StackSwitcher keeps a single wide row. A native wrapping tab list
    // keeps every destination visible at narrow widths and enlarged text.
    let switcher = gtk::FlowBox::builder()
        .accessible_role(gtk::AccessibleRole::TabList)
        .selection_mode(gtk::SelectionMode::None)
        .min_children_per_line(1)
        .max_children_per_line(5)
        .column_spacing(6)
        .row_spacing(6)
        .build();
    switcher.set_widget_name("accounting-tabs");
    accessible_name(&switcher, "بخش‌های دفتر حسابداری");
    for (name, title) in [
        ("setup", "شرکت و دوره"),
        ("chart", "کدینگ"),
        ("journals", "اسناد"),
        ("trial", "تراز آزمایشی"),
        ("audit", "تاریخچه"),
    ] {
        let tab = gtk::ToggleButton::builder()
            .label(title)
            .accessible_role(gtk::AccessibleRole::Tab)
            .build();
        tab.set_widget_name(&format!("accounting-tab-{name}"));
        accessible_name(&tab, title);
        let weak_stack = tabs.downgrade();
        tab.connect_clicked(move |tab| {
            if let Some(stack) = weak_stack.upgrade() {
                tab.set_active(true);
                stack.set_visible_child_name(name);
            }
        });
        tab.connect_toggled(|tab| {
            tab.update_state(&[gtk::accessible::State::Selected(Some(tab.is_active()))]);
        });
        let weak_tab = tab.downgrade();
        tabs.connect_visible_child_name_notify(move |stack| {
            if let Some(tab) = weak_tab.upgrade() {
                tab.set_active(stack.visible_child_name().as_deref() == Some(name));
            }
        });
        switcher.insert(&tab, -1);
        if let Some(child) = tab.parent() {
            child.set_focusable(false);
        }
    }
    root.append(&switcher);
    let setup = column();
    let chart = column();
    let journals = column();
    let trial = column();
    let audit = column();
    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some("شماره، منبع، رویداد، شرح یا حساب…"));
    search.set_widget_name("accounting-journal-search");
    let journal_shell = column();
    journal_shell.append(&search);
    journal_shell.append(&scroll(&journals));
    let chart_search = gtk::SearchEntry::new();
    chart_search.set_placeholder_text(Some("کد، نام یا نوع حساب…"));
    chart_search.set_widget_name("accounting-account-search");
    let chart_shell = column();
    chart_shell.append(&chart_search);
    chart_shell.append(&scroll(&chart));
    let audit_search = gtk::SearchEntry::new();
    audit_search.set_placeholder_text(Some("عملیات، دلیل، شناسه یا تاریخ…"));
    audit_search.set_widget_name("accounting-audit-search");
    let audit_shell = column();
    audit_shell.append(&audit_search);
    audit_shell.append(&scroll(&audit));
    let as_of = date_field(
        ui,
        "تا تاریخ مالی",
        "accounting-as-of",
        &chrono::Local::now().format("%Y-%m-%d").to_string(),
    );
    let unit = gtk::DropDown::from_strings(&["ریال", "تومان"]);
    unit.set_widget_name("accounting-report-unit");
    accessible_name(&unit, "واحد نمایش گزارش");
    let report = column();
    let filter = actions();
    filter.insert(&as_of.widget, -1);
    filter.insert(&field("واحد نمایش", &unit), -1);
    let run = button("نمایش تراز آزمایشی");
    run.set_widget_name("accounting-run-trial");
    run.set_valign(gtk::Align::Center);
    filter.insert(&run, -1);
    report.append(&filter);
    report.append(&scroll(&trial));
    for (name, title, widget) in [
        (
            "setup",
            "شرکت و دوره",
            scroll(&setup).upcast::<gtk::Widget>(),
        ),
        ("chart", "کدینگ", chart_shell.upcast()),
        ("journals", "اسناد", journal_shell.upcast()),
        ("trial", "تراز آزمایشی", report.upcast()),
        ("audit", "تاریخچه", audit_shell.upcast()),
    ] {
        tabs.add_titled(&widget, Some(name), title);
    }
    root.append(&tabs);
    let page = Rc::new(Page {
        ui: Rc::downgrade(ui),
        tabs,
        chart_search,
        audit_search,
        chart_page: Cell::new(0),
        audit_page: Cell::new(0),
        trial_page: Cell::new(0),
        report_busy: Cell::new(false),
        report_revision: Cell::new(0),
        company: companies,
        company_ids: RefCell::new(vec![]),
        setup,
        chart,
        journals,
        trial,
        audit,
        search,
        unit,
        as_of,
        date_preference: Cell::new(ui.store.borrow().state.settings.date_display),
        journal_page: Cell::new(0),
        journal_account_filter: RefCell::new(None),
        refreshing: Cell::new(false),
    });
    let weak = Rc::downgrade(&page);
    add.connect_clicked(move |_| {
        if let Some(p) = weak.upgrade() {
            p.company_dialog();
        }
    });
    let weak = Rc::downgrade(&page);
    page.company.connect_selected_notify(move |_| {
        if let Some(p) = weak.upgrade()
            && !p.refreshing.get()
        {
            p.journal_page.set(0);
            p.journal_account_filter.borrow_mut().take();
            p.refresh_content();
        }
    });
    let weak = Rc::downgrade(&page);
    page.search.connect_search_changed(move |_| {
        if let Some(p) = weak.upgrade() {
            p.journal_page.set(0);
            p.refresh_journals();
        }
    });
    let weak = Rc::downgrade(&page);
    page.chart_search.connect_search_changed(move |_| {
        if let Some(p) = weak.upgrade() {
            p.chart_page.set(0);
            p.refresh_chart();
        }
    });
    let weak = Rc::downgrade(&page);
    page.audit_search.connect_search_changed(move |_| {
        if let Some(p) = weak.upgrade() {
            p.audit_page.set(0);
            p.refresh_audit();
        }
    });
    let weak = Rc::downgrade(&page);
    run.connect_clicked(move |_| {
        if let Some(p) = weak.upgrade() {
            p.trial_page.set(0);
            p.refresh_trial();
        }
    });
    let weak = Rc::downgrade(&page);
    page.unit.connect_selected_notify(move |_| {
        if let Some(p) = weak.upgrade() {
            p.refresh_trial();
        }
    });
    // The root owns the controller; callbacks use Weak so closing the page releases it.
    let retained = page.clone();
    root.connect_map(move |_| retained.refresh());
    page.refresh();
    root.upcast()
}
impl Page {
    fn mutate(
        self: &Rc<Self>,
        dialog: &adw::Dialog,
        error: &gtk::Label,
        save: &gtk::Button,
        message: &'static str,
        command: impl FnOnce(&mut AccountingState) -> Result<()> + Send + 'static,
    ) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        if ui.persistence_busy.get() || ui.background_busy.get() {
            error.set_text("ذخیره یا خروجی دیگری در حال انجام است؛ ورودی شما حفظ شده، پس از پایان دوباره ثبت کنید");
            error.set_visible(true);
            return;
        }
        let directory = ui.store.borrow().directory.clone();
        let reserved = ui.store.borrow_mut().begin_detached();
        let (generation, mut candidate) = match reserved {
            Ok(v) => v,
            Err(e) => {
                error.set_text(&format!("{e:#}"));
                error.set_visible(true);
                return;
            }
        };
        ui.persistence_busy.set(true);
        save.set_sensitive(false);
        dialog.set_can_close(false);
        error.set_text("در حال بررسی و ذخیرهٔ اتمی…");
        error.set_visible(true);
        error.remove_css_class("error");
        let (sender, receiver) = mpsc::sync_channel(1);
        let spawned = thread::Builder::new()
            .name("sarv-accounting-commit".into())
            .spawn(move || {
                let result = catch_unwind(AssertUnwindSafe(|| {
                    command(&mut candidate.accounting)?;
                    let prepared = Store::prepare_candidate(generation, candidate)?;
                    Store::commit_detached(directory, prepared)
                }))
                .unwrap_or_else(|_| Err(anyhow::anyhow!("ثبت قطع شد؛ ورودی شما حفظ شده است")));
                let _ = sender.send(result);
            });
        if let Err(e) = spawned {
            ui.persistence_busy.set(false);
            save.set_sensitive(true);
            dialog.set_can_close(true);
            error.add_css_class("error");
            let result = ui.store.borrow_mut().finish_detached(Err(e.into()));
            show_result(result, dialog, error, self, message);
            return;
        }
        let page = self.clone();
        let dialog = dialog.clone();
        let error = error.clone();
        let save = save.clone();
        glib::timeout_add_local(Duration::from_millis(20), move || {
            let result = match receiver.try_recv() {
                Ok(result) => result,
                Err(mpsc::TryRecvError::Empty) => return glib::ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => {
                    Err(anyhow::anyhow!("ثبت قطع شد؛ دوباره تلاش کنید"))
                }
            };
            let result = ui.store.borrow_mut().finish_detached(result);
            ui.persistence_busy.set(false);
            save.set_sensitive(true);
            dialog.set_can_close(true);
            error.add_css_class("error");
            let succeeded = result.is_ok();
            show_result(result, &dialog, &error, &page, message);
            if succeeded && ui.close_after_save.get() {
                ui.save_form_async();
            }
            glib::ControlFlow::Break
        });
    }
    fn company_id(&self) -> Option<String> {
        self.company_ids
            .borrow()
            .get(self.company.selected() as usize)
            .cloned()
    }
    fn refresh(self: &Rc<Self>) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let preference = ui.store.borrow().state.settings.date_display;
        if self.date_preference.replace(preference) != preference {
            self.as_of
                .calendar
                .set_selected(u32::from(preference == DateDisplay::Jalali));
        }
        let previous = self.company_id();
        let companies = ui.store.borrow().state.accounting.companies.clone();
        self.refreshing.set(true);
        *self.company_ids.borrow_mut() = companies.iter().map(|c| c.id.clone()).collect();
        let names = if companies.is_empty() {
            vec!["شرکتی ساخته نشده است"]
        } else {
            companies
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>()
        };
        self.company.set_sensitive(!companies.is_empty());
        self.company.set_model(Some(&gtk::StringList::new(&names)));
        self.company.set_selected(
            previous
                .and_then(|id| companies.iter().position(|c| c.id == id))
                .unwrap_or(0) as u32,
        );
        self.refreshing.set(false);
        self.refresh_content();
    }
    fn refresh_content(self: &Rc<Self>) {
        for b in [
            &self.setup,
            &self.chart,
            &self.journals,
            &self.trial,
            &self.audit,
        ] {
            clear(b);
        }
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let Some(company_id) = self.company_id() else {
            self.setup.append(&empty_state("دفتر شرکت هنوز ساخته نشده", "شرکت تازه بسازید، مرزهای سال مالی و کدینگ خود را مشخص کنید، سپس سند متوازن ثبت کنید. پیشنهادهای قیمت به دفتر کل ارسال نمی‌شوند."));
            return;
        };
        let state = ui.store.borrow().state.accounting.clone();
        self.setup.append(&validation("این دفتر ثبت دستی دارد. کنترل دریافتنی، پرداختنی، موجودی و چک و ارسال مالیاتی هنوز متصل نیستند؛ ثبت مستقیم در حساب کنترل مسدود است. انتخاب کدینگ و رویه‌ها به بررسی حسابدار نیاز دارد.", StatusTone::Neutral));
        let new_period = button("تعریف دورهٔ مالی");
        new_period.set_widget_name("accounting-new-period");
        self.setup.append(&new_period);
        let weak = Rc::downgrade(self);
        new_period.connect_clicked(move |_| {
            if let Some(p) = weak.upgrade() {
                p.period_dialog();
            }
        });
        let periods = state
            .periods
            .iter()
            .filter(|p| p.company_id == company_id)
            .collect::<Vec<_>>();
        if periods.is_empty() {
            self.setup
                .append(&label("دوره‌ای تعریف نشده؛ شروع و پایان را صریح وارد کنید."));
        }
        for period in periods {
            let c = card();
            c.append(&heading(&period.name));
            c.append(&label(&format!(
                "{} تا {} • {}",
                financial_date(&ui, &period.start_date),
                financial_date(&ui, &period.end_date),
                if period.locked {
                    "قفل‌شده"
                } else {
                    "باز"
                }
            )));
            let action = button(if period.locked {
                "بازگشایی با دلیل"
            } else {
                "قفل دوره با دلیل"
            });
            action.set_widget_name(&format!("accounting-lock-{}", period.id));
            let id = period.id.clone();
            let locked = period.locked;
            let weak = Rc::downgrade(self);
            action.connect_clicked(move |_| {
                if let Some(p) = weak.upgrade() {
                    p.lock_dialog(id.clone(), !locked);
                }
            });
            c.append(&action);
            self.setup.append(&c);
        }
        self.refresh_chart();
        self.refresh_journals();
        self.refresh_trial();
        self.refresh_audit();
    }
    fn pager(self: &Rc<Self>, container: &gtk::Box, count: usize, kind: &'static str) -> usize {
        let cell = match kind {
            "chart" => &self.chart_page,
            "audit" => &self.audit_page,
            _ => &self.trial_page,
        };
        let pages = count.div_ceil(PAGE_SIZE).max(1);
        let current = cell.get().min(pages - 1);
        cell.set(current);
        let controls = actions();
        controls.insert(
            &label(&format!(
                "{} رکورد • صفحهٔ {} از {}",
                fa(count),
                fa(current + 1),
                fa(pages)
            )),
            -1,
        );
        for (caption, forward) in [("قبلی", false), ("بعدی", true)] {
            let b = button(caption);
            b.set_sensitive(if forward {
                current + 1 < pages
            } else {
                current > 0
            });
            let weak = Rc::downgrade(self);
            b.connect_clicked(move |_| {
                if let Some(p) = weak.upgrade() {
                    let cell = match kind {
                        "chart" => &p.chart_page,
                        "audit" => &p.audit_page,
                        _ => &p.trial_page,
                    };
                    cell.set(if forward {
                        cell.get() + 1
                    } else {
                        cell.get().saturating_sub(1)
                    });
                    match kind {
                        "chart" => p.refresh_chart(),
                        "audit" => p.refresh_audit(),
                        _ => p.refresh_trial(),
                    }
                }
            });
            controls.insert(&b, -1);
        }
        container.append(&controls);
        current * PAGE_SIZE
    }
    fn refresh_chart(self: &Rc<Self>) {
        clear(&self.chart);
        let (Some(ui), Some(company_id)) = (self.ui.upgrade(), self.company_id()) else {
            return;
        };
        let state = ui.store.borrow().state.accounting.clone();
        let add_account = button("حساب تازه");
        add_account.set_widget_name("accounting-new-account");
        self.chart.append(&add_account);
        let weak = Rc::downgrade(self);
        add_account.connect_clicked(move |_| {
            if let Some(p) = weak.upgrade() {
                p.account_dialog();
            }
        });
        let mut accounts = state
            .accounts
            .iter()
            .filter(|a| {
                a.company_id == company_id
                    && normalize(&format!(
                        "{} {} {}",
                        a.code,
                        a.name,
                        account_class(a.classification)
                    ))
                    .contains(&normalize(&self.chart_search.text()))
            })
            .collect::<Vec<_>>();
        accounts.sort_by(|a, b| a.code.cmp(&b.code));
        self.chart.append(&label("کدینگ بدون حساب پیش‌فرض است. حساب والد غیرقابل ثبت و حساب تفصیلی قابل ثبت را خودتان تعیین کنید؛ حساب استفاده‌شده فقط بایگانی می‌شود."));
        if accounts.is_empty() {
            self.chart.append(&empty_state(
                "کدینگ هنوز خالی است",
                "دست‌کم دو حساب قابل ثبت تعریف کنید. نوع و ماهیت حساب را حسابدار مشخص می‌کند.",
            ));
        }
        let start = self.pager(&self.chart, accounts.len(), "chart");
        for account in accounts.iter().skip(start).take(PAGE_SIZE) {
            let c = card();
            c.append(&heading(&format!(
                "\u{2068}{}\u{2069} — {}",
                account.code, account.name
            )));
            let parent = account
                .parent_id
                .as_ref()
                .and_then(|id| state.accounts.iter().find(|a| &a.id == id))
                .map(|a| format!(" • والد: {}", a.code))
                .unwrap_or_default();
            c.append(&label(&format!(
                "{} • ماهیت {} • {}{}{}",
                account_class(account.classification),
                if account.normal_balance == BalanceSide::Debit {
                    "بدهکار"
                } else {
                    "بستانکار"
                },
                if account.postable {
                    "قابل ثبت"
                } else {
                    "گروه؛ غیرقابل ثبت"
                },
                parent,
                if account.archived {
                    " • بایگانی"
                } else {
                    ""
                }
            )));
            if !account.archived {
                let archive = button("بایگانی با دلیل");
                let id = account.id.clone();
                let weak = Rc::downgrade(self);
                archive.connect_clicked(move |_| {
                    if let Some(p) = weak.upgrade() {
                        p.archive_dialog(id.clone());
                    }
                });
                c.append(&archive);
            }
            self.chart.append(&c);
        }
    }
    fn refresh_audit(self: &Rc<Self>) {
        clear(&self.audit);
        let (Some(ui), Some(company_id)) = (self.ui.upgrade(), self.company_id()) else {
            return;
        };
        let state = ui.store.borrow().state.accounting.clone();
        let events = state
            .audit
            .iter()
            .filter(|a| {
                a.company_id == company_id
                    && normalize(&format!(
                        "{} {} {} {}",
                        audit_operation(&a.operation),
                        a.reason,
                        a.entity_id,
                        a.recorded_at
                    ))
                    .contains(&normalize(&self.audit_search.text()))
            })
            .collect::<Vec<_>>();
        self.audit.append(&label("تاریخچهٔ محلی برنامه؛ در برابر مالک فایل‌ها مصون از دست‌کاری نیست. هویت فعلی: تک‌کاربر محلی."));
        if events.is_empty() {
            self.audit.append(&label("هنوز عملیاتی ثبت نشده است."));
        }
        let start = self.pager(&self.audit, events.len(), "audit");
        for event in events.iter().rev().skip(start).take(PAGE_SIZE) {
            let c = card();
            c.append(&label(&format!(
                "{} • {}",
                audit_operation(&event.operation),
                isolate(&event.recorded_at)
            )));
            c.append(&label(&format!("{} • {}", event.actor, event.reason)));
            let entity = label(&format!("رکورد مرتبط: \u{2068}{}\u{2069}", event.entity_id));
            entity.set_selectable(true);
            c.append(&entity);
            if let Some(journal) = state.journals.iter().find(|j| j.id == event.entity_id) {
                let source = journal.clone();
                let details = button("مشاهدهٔ سند مرتبط");
                let weak = Rc::downgrade(self);
                details.connect_clicked(move |_| {
                    if let Some(p) = weak.upgrade() {
                        p.journal_detail(source.clone());
                    }
                });
                c.append(&details);
            }
            self.audit.append(&c);
        }
    }
    fn company_dialog(self: &Rc<Self>) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let (dialog, form, error, save) =
            form_dialog(&ui, "ساخت شرکت", "accounting-company-dialog");
        let name = named_entry("نام شرکت", "accounting-company-name", "", false);
        form.append(&field("نام شرکت", &name));
        form.append(&label("دفتر این شرکت از شرکت‌های دیگر جداست. منطقهٔ زمانی این مرحله UTC است؛ تاریخ مالی با انتخاب صریح شما ثبت می‌شود."));
        let weak = Rc::downgrade(self);
        save.connect_clicked(move |save| {
            let Some(p) = weak.upgrade() else {
                return;
            };
            let name = name.text().to_string();
            p.mutate(
                &dialog,
                &error,
                save,
                "شرکت ذخیره شد",
                move |state| state.add_company(&name, ACTOR).map(|_| ()),
            );
        });
    }
    fn period_dialog(self: &Rc<Self>) {
        let (Some(ui), Some(company)) = (self.ui.upgrade(), self.company_id()) else {
            return;
        };
        let (dialog, form, error, save) =
            form_dialog(&ui, "تعریف دورهٔ مالی", "accounting-period-dialog");
        let name = named_entry("عنوان دوره", "accounting-period-name", "", false);
        let start = date_field(&ui, "شروع دورهٔ مالی", "accounting-period-start", "");
        let end = date_field(&ui, "پایان دورهٔ مالی", "accounting-period-end", "");
        clear_date_error_on_edit(&start, &error);
        clear_date_error_on_edit(&end, &error);
        form.append(&field("عنوان دوره", &name));
        form.append(&start.widget);
        form.append(&end.widget);
        form.append(&label("مرزهای سال مالی را از سیاست شرکت خود وارد کنید. دورهٔ هم‌پوشان پذیرفته نمی‌شود؛ قفل دوره با بستن حساب‌ها متفاوت است."));
        let weak = Rc::downgrade(self);
        save.connect_clicked(move |save| {
            let Some(p) = weak.upgrade() else {
                return;
            };
            let values: Result<(String, String)> =
                (|| Ok((date_value(&start)?, date_value(&end)?)))();
            match values {
                Ok((start, end)) => {
                    let name = name.text().to_string();
                    let company = company.clone();
                    p.mutate(
                        &dialog,
                        &error,
                        save,
                        "دورهٔ مالی ذخیره شد",
                        move |state| {
                            state
                                .add_period(&company, &name, &start, &end, ACTOR)
                                .map(|_| ())
                        },
                    );
                }
                Err(e) => {
                    error.set_text(&format!("{e:#}"));
                    error.set_visible(true);
                }
            }
        });
    }
    fn account_dialog(self: &Rc<Self>) {
        let (Some(ui), Some(company)) = (self.ui.upgrade(), self.company_id()) else {
            return;
        };
        let accounts = ui
            .store
            .borrow()
            .state
            .accounting
            .accounts
            .iter()
            .filter(|a| a.company_id == company && !a.archived && !a.postable)
            .cloned()
            .collect::<Vec<_>>();
        let (dialog, form, error, save) =
            form_dialog(&ui, "ساخت حساب", "accounting-account-dialog");
        let code = named_entry("کد حساب", "accounting-account-code", "", true);
        let name = named_entry("نام حساب", "accounting-account-name", "", false);
        let class =
            gtk::DropDown::from_strings(&["دارایی", "بدهی", "حقوق مالکانه", "درآمد", "هزینه"]);
        class.set_widget_name("accounting-account-class");
        class.set_selected(gtk::INVALID_LIST_POSITION);
        let side = gtk::DropDown::from_strings(&["بدهکار", "بستانکار"]);
        side.set_widget_name("accounting-account-side");
        side.set_selected(gtk::INVALID_LIST_POSITION);
        let parent_names = std::iter::once("بدون والد".into())
            .chain(accounts.iter().map(|a| format!("{} — {}", a.code, a.name)))
            .collect::<Vec<_>>();
        let parent = choice("حساب والد", &parent_names);
        let postable = gtk::CheckButton::with_label("قابل ثبت در سند (غیرفعال: گروه کدینگ)");
        postable.set_active(true);
        postable.set_widget_name("accounting-account-postable");
        for (caption, widget) in [
            ("کد حساب", code.clone().upcast::<gtk::Widget>()),
            ("نام حساب", name.clone().upcast()),
            ("نوع حساب", class.clone().upcast()),
            ("ماهیت", side.clone().upcast()),
            ("حساب والد", parent.clone().upcast()),
        ] {
            form.append(&field(caption, &widget));
        }
        form.append(&postable);
        form.append(&label("نوع و ماهیت مستقل و صریح‌اند. این مرحله فقط حساب عادی دارد؛ حساب کنترل زیرسیستم‌های آینده قابل ثبت دستی نیست."));
        let weak = Rc::downgrade(self);
        save.connect_clicked(move |save| {
            let Some(p) = weak.upgrade() else {
                return;
            };
            if class.selected() == gtk::INVALID_LIST_POSITION
                || side.selected() == gtk::INVALID_LIST_POSITION
            {
                error.set_text("نوع و ماهیت حساب را صریح انتخاب کنید؛ حساب پیش‌فرض تعیین نمی‌شود");
                error.set_visible(true);
                if class.selected() == gtk::INVALID_LIST_POSITION {
                    class.grab_focus();
                } else {
                    side.grab_focus();
                }
                return;
            }
            let classes = [
                AccountClass::Asset,
                AccountClass::Liability,
                AccountClass::Equity,
                AccountClass::Revenue,
                AccountClass::Expense,
            ];
            let account = NewAccount {
                company_id: company.clone(),
                parent_id: if parent.selected() == 0 {
                    None
                } else {
                    accounts
                        .get(parent.selected() as usize - 1)
                        .map(|a| a.id.clone())
                },
                code: code.text().to_string(),
                name: name.text().to_string(),
                classification: classes[class.selected() as usize],
                normal_balance: if side.selected() == 0 {
                    BalanceSide::Debit
                } else {
                    BalanceSide::Credit
                },
                postable: postable.is_active(),
                control: false,
            };
            p.mutate(
                &dialog,
                &error,
                save,
                "حساب ذخیره شد",
                move |state| state.add_account(account, ACTOR).map(|_| ()),
            );
        });
    }
    fn reason_dialog(
        self: &Rc<Self>,
        title: &str,
        explanation: &str,
        command: impl Fn(&mut AccountingState, &str) -> Result<()> + Send + Sync + 'static,
    ) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let (dialog, form, error, save) = form_dialog(&ui, title, "accounting-reason-dialog");
        form.append(&label(explanation));
        let reason = named_entry("دلیل ضروری", "accounting-reason", "", false);
        form.append(&field("دلیل ضروری", &reason));
        let command = Arc::new(command);
        let weak = Rc::downgrade(self);
        save.connect_clicked(move |save| {
            let Some(p) = weak.upgrade() else {
                return;
            };
            let reason = reason.text().to_string();
            let command = command.clone();
            p.mutate(
                &dialog,
                &error,
                save,
                "عملیات با دلیل در تاریخچه ثبت شد",
                move |state| command(state, &reason),
            );
        });
    }
    fn lock_dialog(self: &Rc<Self>, id: String, locked: bool) {
        self.reason_dialog(if locked { "قفل دوره" } else { "بازگشایی دوره" }, if locked { "تمام ثبت‌ها و ابطال‌ها در این دوره مسدود می‌شوند. این عملیات انتقال سود و بستن سال نیست." } else { "بازگشایی امکان ثبت بعدی را فراهم می‌کند. دلیل و هویت محلی در تاریخچه باقی می‌ماند؛ مجوز چندکاربره هنوز وجود ندارد." }, move |state, reason| state.set_period_lock(&id, locked, ACTOR, reason));
    }
    fn archive_dialog(self: &Rc<Self>, id: String) {
        self.reason_dialog(
            "بایگانی حساب",
            "حساب از انتخاب سند تازه کنار می‌رود؛ اسناد و تراز تاریخی آن حفظ می‌شوند.",
            move |state, reason| state.archive_account(&id, ACTOR, reason),
        );
    }
    fn refresh_journals(self: &Rc<Self>) {
        clear(&self.journals);
        let (Some(ui), Some(company)) = (self.ui.upgrade(), self.company_id()) else {
            return;
        };
        let state = ui.store.borrow().state.accounting.clone();
        let add = button("سند دستی / افتتاحیهٔ دفتر کل");
        add.set_widget_name("accounting-new-journal");
        add.add_css_class("suggested-action");
        self.journals.append(&add);
        let weak = Rc::downgrade(self);
        add.connect_clicked(move |_| {
            if let Some(p) = weak.upgrade() {
                p.journal_dialog();
            }
        });
        if let Some(id) = self.journal_account_filter.borrow().clone() {
            let account = state
                .accounts
                .iter()
                .find(|a| a.id == id)
                .map(|a| format!("{} — {}", a.code, a.name))
                .unwrap_or(id);
            let filters = actions();
            filters.insert(&label(&format!("فقط سندهای مرتبط با حساب: {account}")), -1);
            let reset = button("همهٔ حساب‌ها");
            let weak = Rc::downgrade(self);
            reset.connect_clicked(move |_| {
                if let Some(p) = weak.upgrade() {
                    p.journal_account_filter.borrow_mut().take();
                    p.journal_page.set(0);
                    p.refresh_journals();
                }
            });
            filters.insert(&reset, -1);
            self.journals.append(&filters);
        }
        let account_filter = self.journal_account_filter.borrow().clone();
        let needle = normalize(&self.search.text());
        let matches = state
            .journals
            .iter()
            .filter(|j| {
                j.company_id == company
                    && account_filter.as_ref().is_none_or(|account| {
                        j.lines.iter().any(|line| &line.account.id == account)
                    })
                    && (needle.is_empty()
                        || normalize(&format!(
                            "{} {} {} {} {} {} {}",
                            j.id,
                            j.date,
                            financial_date(&ui, &j.date),
                            j.source_id,
                            j.event,
                            j.reason,
                            j.lines
                                .iter()
                                .map(|l| format!(
                                    "{} {} {}",
                                    l.account.code, l.account.name, l.description
                                ))
                                .collect::<Vec<_>>()
                                .join(" ")
                        ))
                        .contains(&needle))
            })
            .rev()
            .collect::<Vec<_>>();
        if matches.is_empty() {
            self.journals.append(&empty_state("سندی پیدا نشد", "دوره و حساب‌های قابل ثبت بسازید و سند متوازن وارد کنید، یا عبارت جست‌وجو را تغییر دهید."));
            return;
        }
        let pages = matches.len().div_ceil(PAGE_SIZE);
        let current = self.journal_page.get().min(pages.saturating_sub(1));
        self.journal_page.set(current);
        let pager = actions();
        pager.insert(
            &label(&format!(
                "{} سند • صفحهٔ {} از {}",
                fa(matches.len()),
                fa(current + 1),
                fa(pages)
            )),
            -1,
        );
        for (caption, forwards) in [("قبلی", false), ("بعدی", true)] {
            let b = button(caption);
            b.set_sensitive(if forwards {
                current + 1 < pages
            } else {
                current > 0
            });
            let weak = Rc::downgrade(self);
            b.connect_clicked(move |_| {
                if let Some(p) = weak.upgrade() {
                    p.journal_page.set(if forwards {
                        p.journal_page.get() + 1
                    } else {
                        p.journal_page.get().saturating_sub(1)
                    });
                    p.refresh_journals();
                }
            });
            pager.insert(&b, -1);
        }
        self.journals.append(&pager);
        for journal in matches
            .into_iter()
            .skip(current * PAGE_SIZE)
            .take(PAGE_SIZE)
        {
            let c = card();
            let reversed = state.journals.iter().any(|j| matches!(&j.kind, JournalKind::Reversal {journal_id} if journal_id == &journal.id));
            let kind = match &journal.kind {
                JournalKind::Opening => "افتتاحیهٔ دفتر کل",
                JournalKind::Manual => "سند دستی",
                JournalKind::Reversal { .. } => "سند معکوس",
            };
            c.append(&heading(&format!(
                "{} • {} • {}",
                kind,
                financial_date(&ui, &journal.date),
                if reversed {
                    "ثبت‌شده؛ دارای سند معکوس"
                } else {
                    "ثبت‌شده"
                }
            )));
            let amount = journal
                .lines
                .iter()
                .try_fold(Money::ZERO, |total, line| total.checked_add(line.debit))
                .map(|v| v.display(MoneyUnit::Rial))
                .unwrap_or_else(|_| "خطای جمع".into());
            c.append(&label(&format!("شناسه: \u{2068}{}\u{2069}\nمنبع: \u{2068}{}\u{2069} • نسخه {} • رویداد: {}\n{} ریال • {} ردیف • {}",journal.id,journal.source_id,fa(journal.source_revision),journal.event,amount,fa(journal.lines.len()),journal.reason)));
            let buttons = actions();
            let details = button("ردیف‌ها و پیوندها");
            let snapshot = journal.clone();
            let weak = Rc::downgrade(self);
            details.connect_clicked(move |_| {
                if let Some(p) = weak.upgrade() {
                    p.journal_detail(snapshot.clone());
                }
            });
            buttons.insert(&details, -1);
            if !reversed && !matches!(journal.kind, JournalKind::Reversal { .. }) {
                let reverse = button("سند معکوس با دلیل");
                reverse.set_widget_name(&format!("accounting-reverse-{}", journal.id));
                let id = journal.id.clone();
                let weak = Rc::downgrade(self);
                reverse.connect_clicked(move |_| {
                    if let Some(p) = weak.upgrade() {
                        p.reversal_dialog(id.clone());
                    }
                });
                buttons.insert(&reverse, -1);
            }
            c.append(&buttons);
            self.journals.append(&c);
        }
    }
    fn journal_detail(self: &Rc<Self>, journal: Journal) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let (dialog, form, _, close) =
            form_dialog(&ui, "ردیف‌های سند ثبت‌شده", "accounting-journal-detail");
        close.set_label("بستن");
        let d = dialog.clone();
        close.connect_clicked(move |_| {
            d.close();
        });
        form.append(&label(&format!(
            "شناسه: {}\nتاریخ مالی: {}\nثبت در UTC: {}\n{} • {}",
            isolate(&journal.id),
            financial_date(&ui, &journal.date),
            isolate(&journal.recorded_at),
            journal.actor,
            journal.reason
        )));
        if let JournalKind::Reversal { journal_id } = &journal.kind {
            let source = label(&format!("معکوسِ سند: {journal_id}"));
            source.set_selectable(true);
            form.append(&source);
        }
        let journal = Rc::new(journal);
        let current = Rc::new(Cell::new(0usize));
        let pages = journal.lines.len().div_ceil(PAGE_SIZE).max(1);
        let list = column();
        let indicator = label("");
        let controls = actions();
        let previous = button("ردیف‌های قبلی");
        let next = button("ردیف‌های بعدی");
        controls.insert(&previous, -1);
        controls.insert(&indicator, -1);
        controls.insert(&next, -1);
        form.append(&controls);
        form.append(&list);
        let render: Rc<dyn Fn()> = {
            let journal = journal.clone();
            let current = current.clone();
            let list = list.clone();
            let indicator = indicator.clone();
            let previous = previous.downgrade();
            let next = next.downgrade();
            Rc::new(move || {
                clear(&list);
                let table = gtk::Grid::new();
                table.set_column_spacing(16);
                table.set_row_spacing(10);
                table.set_direction(gtk::TextDirection::Rtl);
                for (col, title) in ["حساب", "شرح", "بدهکار (ریال)", "بستانکار (ریال)"]
                    .iter()
                    .enumerate()
                {
                    table.attach(&heading(title), col as i32, 0, 1, 1);
                }
                for (index, line) in journal
                    .lines
                    .iter()
                    .skip(current.get() * PAGE_SIZE)
                    .take(PAGE_SIZE)
                    .enumerate()
                {
                    let r = index as i32 + 1;
                    table.attach(
                        &label(&format!(
                            "\u{2068}{}\u{2069} — {}",
                            line.account.code, line.account.name
                        )),
                        0,
                        r,
                        1,
                        1,
                    );
                    table.attach(&label(&line.description), 1, r, 1, 1);
                    table.attach(
                        &numeric_cell(&line.debit.display(MoneyUnit::Rial)),
                        2,
                        r,
                        1,
                        1,
                    );
                    table.attach(
                        &numeric_cell(&line.credit.display(MoneyUnit::Rial)),
                        3,
                        r,
                        1,
                        1,
                    );
                }
                let horizontal = gtk::ScrolledWindow::builder()
                    .child(&table)
                    .hscrollbar_policy(gtk::PolicyType::Automatic)
                    .vscrollbar_policy(gtk::PolicyType::Never)
                    .build();
                list.append(&horizontal);
                indicator.set_text(&format!(
                    "ردیف‌های {} تا {} از {} • ریال",
                    fa(current.get() * PAGE_SIZE + 1),
                    fa(((current.get() + 1) * PAGE_SIZE).min(journal.lines.len())),
                    fa(journal.lines.len())
                ));
                if let Some(b) = previous.upgrade() {
                    b.set_sensitive(current.get() > 0);
                }
                if let Some(b) = next.upgrade() {
                    b.set_sensitive(current.get() + 1 < pages);
                }
            })
        };
        render();
        for (button, forward) in [(previous, false), (next, true)] {
            let current = current.clone();
            let render = render.clone();
            button.connect_clicked(move |_| {
                current.set(if forward {
                    (current.get() + 1).min(pages - 1)
                } else {
                    current.get().saturating_sub(1)
                });
                render();
            });
        }
        form.append(&label("مقادیر و مشخصات حساب در این سند تصویری ثابت از زمان ثبت‌اند. ویرایش مستقیمِ سند ثبت‌شده مجاز نیست."));
    }
    fn refresh_trial(self: &Rc<Self>) {
        clear(&self.trial);
        let (Some(ui), Some(company)) = (self.ui.upgrade(), self.company_id()) else {
            return;
        };
        let revision = self.report_revision.get().wrapping_add(1);
        self.report_revision.set(revision);
        if self.report_busy.get() {
            self.trial.append(&label("در حال محاسبهٔ تراز…"));
            return;
        }
        let date = match date_value(&self.as_of) {
            Ok(date) => date,
            Err(e) => {
                self.trial
                    .append(&validation(&e.to_string(), StatusTone::Error));
                return;
            }
        };
        let snapshot = ui.store.borrow().state.accounting.clone();
        self.report_busy.set(true);
        self.trial
            .append(&label("در حال محاسبهٔ تراز؛ دفتر قابل استفاده است…"));
        let (sender, receiver) = mpsc::sync_channel(1);
        let spawned = thread::Builder::new()
            .name("sarv-accounting-report".into())
            .spawn(move || {
                let result =
                    catch_unwind(AssertUnwindSafe(|| snapshot.trial_balance(&company, &date)))
                        .unwrap_or_else(|_| {
                            Err(anyhow::anyhow!("محاسبهٔ تراز قطع شد؛ دوباره تلاش کنید"))
                        });
                let _ = sender.send(result);
            });
        if let Err(e) = spawned {
            self.report_busy.set(false);
            self.render_trial(Err(e.into()));
            return;
        }
        let weak = Rc::downgrade(self);
        glib::timeout_add_local(Duration::from_millis(20), move || {
            let Some(page) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            let result = match receiver.try_recv() {
                Ok(r) => r,
                Err(mpsc::TryRecvError::Empty) => return glib::ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => Err(anyhow::anyhow!("محاسبهٔ تراز قطع شد")),
            };
            page.report_busy.set(false);
            if page.report_revision.get() != revision {
                page.refresh_trial();
            } else {
                page.render_trial(result);
            }
            glib::ControlFlow::Break
        });
    }
    fn render_trial(self: &Rc<Self>, result: Result<TrialBalance>) {
        clear(&self.trial);
        let (Some(ui), Some(company)) = (self.ui.upgrade(), self.company_id()) else {
            return;
        };
        match result {
            Err(e) => self
                .trial
                .append(&validation(&e.to_string(), StatusTone::Error)),
            Ok(balance) => {
                let unit = amount_unit(&self.unit);
                let unit_name = if unit == MoneyUnit::Rial {
                    "ریال"
                } else {
                    "تومان"
                };
                let name = ui
                    .store
                    .borrow()
                    .state
                    .accounting
                    .companies
                    .iter()
                    .find(|c| c.id == company)
                    .map(|c| c.name.clone())
                    .unwrap_or_default();
                self.trial.append(&heading(&format!(
                    "{} • تا {} • {}",
                    name,
                    financial_date(&ui, &balance.as_of),
                    unit_name
                )));
                self.trial.append(&label("دفتر کل دستی • فقط اسناد ثبت‌شده تا تاریخ مالی انتخاب‌شده؛ شامل سند معکوس • ماندهٔ بدهکار/بستانکار هر حساب. سیاست شناسایی و زیرسیستم‌ها پیکربندی نشده‌اند."));
                let totals = card();
                totals.append(&label(&format!(
                    "جمع ماندهٔ بدهکار: {} {}\nجمع ماندهٔ بستانکار: {} {}",
                    balance.total_debit.display(unit),
                    unit_name,
                    balance.total_credit.display(unit),
                    unit_name
                )));
                totals.append(&status_label(
                    if balance.total_debit == balance.total_credit {
                        "مانده‌ها دقیقاً متوازن‌اند"
                    } else {
                        "اختلاف مانده؛ بررسی ضروری است"
                    },
                    if balance.total_debit == balance.total_credit {
                        StatusTone::Success
                    } else {
                        StatusTone::Error
                    },
                ));
                self.trial.append(&totals);
                let exports = actions();
                for extension in ["pdf", "csv"] {
                    let export = button(&format!("خروجی تراز {}", extension.to_uppercase()));
                    export.set_widget_name(&format!("accounting-trial-export-{extension}"));
                    let balance = balance.clone();
                    let company = name.clone();
                    let weak = Rc::downgrade(self);
                    export.connect_clicked(move |_| {
                        let Some(page) = weak.upgrade() else {
                            return;
                        };
                        let Some(ui) = page.ui.upgrade() else {
                            return;
                        };
                        if !ui.background_available() || ui.persistence_busy.get() {
                            return;
                        }
                        let balance = balance.clone();
                        let company = company.clone();
                        let weak = Rc::downgrade(&ui);
                        choose_file(
                            &ui.window,
                            true,
                            &format!("sarv-trial-{}.{}", balance.as_of, extension),
                            move |path| {
                                let Some(ui) = weak.upgrade() else {
                                    return;
                                };
                                let balance = balance.clone();
                                let company = company.clone();
                                let output = path.clone();
                                let dates = ui.store.borrow().state.settings.date_display;
                                ui.run_background(
                                    "در حال ساخت خروجی تراز…",
                                    move || {
                                        if extension == "pdf" {
                                            sarv_quote::accounting_export::trial_pdf_with_dates(
                                                &company, &balance, unit, dates, &output,
                                            )
                                        } else {
                                            sarv_quote::accounting_export::trial_csv_with_dates(
                                                &company, &balance, unit, dates, &output,
                                            )
                                        }
                                    },
                                    move |ui, result| {
                                        ui.message(result.map(|()| {
                                            format!("خروجی تراز ذخیره شد: {}", path.display())
                                        }))
                                    },
                                );
                            },
                        );
                    });
                    exports.insert(&export, -1);
                }
                self.trial.append(&exports);
                if balance.rows.is_empty() {
                    self.trial.append(&empty_state(
                        "بدون گردش تا این تاریخ",
                        "ثبت‌ها را در بخش اسناد بررسی کنید یا تاریخ گزارش را تغییر دهید.",
                    ));
                    return;
                }
                let table = gtk::Grid::new();
                table.set_column_spacing(20);
                table.set_row_spacing(10);
                for (col, title) in ["کد و نام حساب", "ماندهٔ بدهکار", "ماندهٔ بستانکار"]
                    .iter()
                    .enumerate()
                {
                    table.attach(&heading(title), col as i32, 0, 1, 1);
                }
                let start = self.pager(&self.trial, balance.rows.len(), "trial");
                for (index, r) in balance.rows.iter().skip(start).take(PAGE_SIZE).enumerate() {
                    let row = index as i32 + 1;
                    let drill = button(&format!("\u{2068}{}\u{2069} — {}", r.code, r.name));
                    let account_id = r.account_id.clone();
                    let weak = Rc::downgrade(self);
                    drill.connect_clicked(move |_| {
                        if let Some(p) = weak.upgrade() {
                            p.search.set_text("");
                            *p.journal_account_filter.borrow_mut() = Some(account_id.clone());
                            p.journal_page.set(0);
                            p.refresh_journals();
                            p.tabs.set_visible_child_name("journals");
                            if let Some(ui) = p.ui.upgrade() {
                                ui.message(Ok("اسناد با شناسهٔ دقیق حساب فیلتر شدند"));
                            }
                        }
                    });
                    table.attach(&drill, 0, row, 1, 1);
                    table.attach(&numeric_cell(&r.debit.display(unit)), 1, row, 1, 1);
                    table.attach(&numeric_cell(&r.credit.display(unit)), 2, row, 1, 1);
                }
                let horizontal = gtk::ScrolledWindow::builder()
                    .child(&table)
                    .hscrollbar_policy(gtk::PolicyType::Automatic)
                    .vscrollbar_policy(gtk::PolicyType::Never)
                    .build();
                self.trial.append(&horizontal);
            }
        }
    }
    fn reversal_dialog(self: &Rc<Self>, journal_id: String) {
        let (Some(ui), Some(company)) = (self.ui.upgrade(), self.company_id()) else {
            return;
        };
        let source_locked = {
            let store = ui.store.borrow();
            let accounting = &store.state.accounting;
            accounting
                .journals
                .iter()
                .find(|journal| journal.id == journal_id)
                .is_some_and(|journal| {
                    accounting
                        .periods
                        .iter()
                        .any(|period| period.id == journal.period_id && period.locked)
                })
        };
        if source_locked {
            ui.message(Err::<String, _>(anyhow::anyhow!(
                "دورهٔ سند اصلی قفل است؛ برای سند معکوس ابتدا دوره را با دلیل بازگشایی کنید"
            )));
            return;
        }
        let periods = ui
            .store
            .borrow()
            .state
            .accounting
            .periods
            .iter()
            .filter(|p| p.company_id == company && !p.locked)
            .cloned()
            .collect::<Vec<_>>();
        let (dialog, form, error, save) =
            form_dialog(&ui, "ثبت سند معکوس", "accounting-reversal-dialog");
        form.append(&label(&format!("سند اصلی {} حفظ می‌شود. همهٔ بدهکار و بستانکارها در سند تازه معکوس می‌شوند. دورهٔ اصلی و مقصد باید باز باشند.",journal_id)));
        let picker = choice(
            "دورهٔ مقصد",
            &periods
                .iter()
                .map(|p| {
                    format!(
                        "{} • {} تا {}",
                        p.name,
                        financial_date(&ui, &p.start_date),
                        financial_date(&ui, &p.end_date)
                    )
                })
                .collect::<Vec<_>>(),
        );
        picker.set_widget_name("accounting-reversal-period");
        let date = date_field(&ui, "تاریخ مالی", "accounting-reversal-date", "");
        clear_date_error_on_edit(&date, &error);
        let reason = named_entry("دلیل معکوس", "accounting-reversal-reason", "", false);
        form.append(&field("دورهٔ مقصد", &picker));
        form.append(&date.widget);
        form.append(&field("دلیل ضروری", &reason));
        let request_key = id();
        let ids = periods.iter().map(|p| p.id.clone()).collect::<Vec<_>>();
        let weak = Rc::downgrade(self);
        save.connect_clicked(move |save| {
            let Some(p) = weak.upgrade() else {
                return;
            };
            let values: Result<(String, String)> =
                (|| Ok((selected(&picker, &ids, "دوره")?, date_value(&date)?)))();
            match values {
                Ok((period, date)) => {
                    let journal_id = journal_id.clone();
                    let request_key = request_key.clone();
                    let reason = reason.text().to_string();
                    p.mutate(
                        &dialog,
                        &error,
                        save,
                        "سند معکوس ذخیره شد؛ اصل سند محفوظ است",
                        move |state| {
                            state
                                .reverse(&journal_id, &period, &date, &request_key, ACTOR, &reason)
                                .map(|_| ())
                        },
                    );
                }
                Err(e) => {
                    error.set_text(&format!("{e:#}"));
                    error.set_visible(true);
                }
            }
        });
    }
    fn journal_dialog(self: &Rc<Self>) {
        let (Some(ui), Some(company)) = (self.ui.upgrade(), self.company_id()) else {
            return;
        };
        let state = ui.store.borrow().state.accounting.clone();
        let periods = state
            .periods
            .iter()
            .filter(|p| p.company_id == company && !p.locked)
            .cloned()
            .collect::<Vec<_>>();
        let mut accounts = state
            .accounts
            .iter()
            .filter(|a| a.company_id == company && a.postable && !a.control && !a.archived)
            .cloned()
            .collect::<Vec<_>>();
        accounts.sort_by(|a, b| a.code.cmp(&b.code));
        if periods.is_empty() || accounts.len() < 2 {
            ui.message(Err::<String, _>(anyhow::anyhow!(
                "دست‌کم یک دورهٔ باز و دو حساب عادی قابل ثبت لازم است"
            )));
            return;
        }
        let (dialog, form, error, save) =
            form_dialog(&ui, "ثبت سند متوازن", "accounting-journal-dialog");
        form.append(&validation("ثبت، ماندهٔ دفتر کل را تغییر می‌دهد. سند ثبت‌شده ویرایش نمی‌شود؛ اصلاح با سند معکوس و دلیل است. افتتاحیه فقط یک‌بار برای شرکت، در اولین روز دوره و پیش از هر سند شرکت مجاز است؛ افتتاحیهٔ دوباره برای سال بعد وجود ندارد. افتتاحیهٔ زیرسیستم‌ها در این مرحله وجود ندارد.",StatusTone::Warning));
        let period = choice(
            "دورهٔ مالی",
            &periods
                .iter()
                .map(|p| {
                    format!(
                        "{} • {} تا {}",
                        p.name,
                        financial_date(&ui, &p.start_date),
                        financial_date(&ui, &p.end_date)
                    )
                })
                .collect::<Vec<_>>(),
        );
        period.set_widget_name("accounting-journal-period");
        let kind = gtk::DropDown::from_strings(&["سند دستی", "افتتاحیهٔ دفتر کل"]);
        kind.set_widget_name("accounting-journal-kind");
        let date = date_field(&ui, "تاریخ مالی", "accounting-journal-date", "");
        clear_date_error_on_edit(&date, &error);
        let unit = gtk::DropDown::from_strings(&["ریال", "تومان"]);
        unit.set_widget_name("accounting-journal-unit");
        let source = named_entry(
            "شناسهٔ منبع",
            "accounting-journal-source",
            &format!("manual-{}", id()),
            true,
        );
        let revision = named_entry("نسخهٔ منبع", "accounting-journal-revision", "1", true);
        let event = named_entry(
            "رویداد اقتصادی",
            "accounting-journal-event",
            "manual-entry",
            true,
        );
        let reason = named_entry("شرح و دلیل ثبت", "accounting-journal-reason", "", false);
        for (c, w) in [
            ("دورهٔ مالی", period.clone().upcast::<gtk::Widget>()),
            ("نوع سند", kind.clone().upcast()),
            ("", date.widget.clone().upcast()),
            ("واحد مبلغ‌های ورودی", unit.clone().upcast()),
            (
                "شناسهٔ منبع • برای جلوگیری از ثبت دوباره",
                source.clone().upcast(),
            ),
            ("نسخهٔ منبع", revision.clone().upcast()),
            ("رویداد اقتصادی", event.clone().upcast()),
            ("شرح و دلیل ثبت", reason.clone().upcast()),
        ] {
            if c.is_empty() {
                form.append(&w);
            } else {
                form.append(&field(c, &w));
            }
        }
        form.append(&label("یک تومان = ده ریال؛ تا یک رقم اعشار تومان دقیق است. تغییر واحد ورودی، عددهای تایپ‌شده را تبدیل نمی‌کند. در هر ردیف فقط یک طرف، با مبلغ مثبت، وارد کنید."));
        let line_box = column();
        form.append(&line_box);
        let rows = Rc::new(RefCell::new(Vec::<LineEditor>::new()));
        let account_ids = Rc::new(accounts.iter().map(|a| a.id.clone()).collect::<Vec<_>>());
        let account_names = Rc::new(
            accounts
                .iter()
                .map(|a| format!("{} — {}", a.code, a.name))
                .collect::<Vec<_>>(),
        );
        let total = label("جمع ردیف‌ها پس از بررسی محاسبه می‌شود.");
        total.set_widget_name("accounting-journal-totals");
        form.append(&total);
        let line_actions = actions();
        let add = button("افزودن ردیف");
        add.set_widget_name("accounting-add-line");
        let check = button("بررسی توازن و مبلغ‌ها");
        check.set_widget_name("accounting-check-balance");
        line_actions.insert(&add, -1);
        line_actions.insert(&check, -1);
        form.append(&line_actions);
        for _ in 0..2 {
            add_line(&line_box, &rows, &account_names);
        }
        let lb = line_box.clone();
        let rs = rows.clone();
        let names = account_names.clone();
        add.connect_clicked(move |_| {
            add_line(&lb, &rs, &names);
        });
        let rs = rows.clone();
        let u = unit.clone();
        let ids = account_ids.clone();
        let t = total.clone();
        let e = error.clone();
        check.connect_clicked(move |_| match collect_lines(&rs, &ids, amount_unit(&u)) {
            Ok((_, debit, credit)) => {
                t.set_text(&format!(
                    "جمع بدهکار: {} • جمع بستانکار: {} • {}",
                    debit.display(amount_unit(&u)),
                    credit.display(amount_unit(&u)),
                    if debit == credit {
                        "دقیقاً متوازن"
                    } else {
                        "نامتوازن؛ ثبت مجاز نیست"
                    }
                ));
                e.set_visible(false);
            }
            Err(err) => {
                e.set_text(&err.to_string());
                e.set_visible(true);
            }
        });
        let period_ids = periods.iter().map(|p| p.id.clone()).collect::<Vec<_>>();
        let request_key = id();
        let weak = Rc::downgrade(self);
        save.connect_clicked(move |save| {
            let Some(p) = weak.upgrade() else {
                return;
            };
            let result: Result<PostCommand> = (|| {
                let period_id = selected(&period, &period_ids, "دوره")?;
                let date = date_value(&date)?;
                let source_revision = digits(&revision.text())
                    .parse::<u64>()
                    .map_err(|_| anyhow::anyhow!("نسخهٔ منبع عدد صحیح نامنفی است"))?;
                let (lines, debit, credit) =
                    collect_lines(&rows, &account_ids, amount_unit(&unit))?;
                anyhow::ensure!(debit == credit, "جمع بدهکار و بستانکار دقیقاً برابر نیست");
                let command = PostCommand {
                    company_id: company.clone(),
                    period_id,
                    date,
                    source_id: source.text().to_string(),
                    source_revision,
                    event: event.text().to_string(),
                    request_key: request_key.clone(),
                    actor: ACTOR.into(),
                    reason: reason.text().to_string(),
                    kind: if kind.selected() == 1 {
                        JournalKind::Opening
                    } else {
                        JournalKind::Manual
                    },
                    lines,
                };
                Ok(command)
            })();
            match result {
                Ok(command) => p.mutate(
                    &dialog,
                    &error,
                    save,
                    "سند متوازن به‌صورت اتمی ذخیره شد",
                    move |state| state.post(command).map(|_| ()),
                ),
                Err(e) => {
                    error.set_text(&format!("{e:#}"));
                    error.set_visible(true);
                }
            }
        });
    }
}
#[derive(Clone)]
struct LineEditor {
    account: gtk::DropDown,
    debit: gtk::Entry,
    credit: gtk::Entry,
    description: gtk::Entry,
    active: Rc<Cell<bool>>,
}
fn add_line(container: &gtk::Box, rows: &Rc<RefCell<Vec<LineEditor>>>, names: &[String]) {
    let index = rows.borrow().len() + 1;
    let c = card();
    c.append(&heading(&format!("ردیف {}", fa(index))));
    let account = choice("حساب ردیف", names);
    account.set_widget_name(&format!("accounting-line-{index}-account"));
    // No account is silently assigned: selection is a deliberate user choice.
    account.set_selected(gtk::INVALID_LIST_POSITION);
    let debit = named_entry(
        "بدهکار",
        &format!("accounting-line-{index}-debit"),
        "",
        true,
    );
    let credit = named_entry(
        "بستانکار",
        &format!("accounting-line-{index}-credit"),
        "",
        true,
    );
    let description = named_entry(
        "شرح ردیف",
        &format!("accounting-line-{index}-description"),
        "",
        false,
    );
    c.append(&field("حساب • جست‌وجوی کد یا نام", &account));
    let amounts = actions();
    amounts.insert(&field("بدهکار", &debit), -1);
    amounts.insert(&field("بستانکار", &credit), -1);
    c.append(&amounts);
    c.append(&field("شرح ردیف", &description));
    let remove = button("حذف ردیف");
    let active = Rc::new(Cell::new(true));
    let a = active.clone();
    let weak = c.downgrade();
    remove.connect_clicked(move |_| {
        a.set(false);
        if let Some(c) = weak.upgrade() {
            c.set_visible(false);
        }
    });
    c.append(&remove);
    container.append(&c);
    rows.borrow_mut().push(LineEditor {
        account,
        debit,
        credit,
        description,
        active,
    });
}
fn collect_lines(
    rows: &RefCell<Vec<LineEditor>>,
    ids: &[String],
    unit: MoneyUnit,
) -> Result<(Vec<JournalLineInput>, Money, Money)> {
    let mut lines = vec![];
    let mut total_debit = Money::ZERO;
    let mut total_credit = Money::ZERO;
    for r in rows.borrow().iter().filter(|r| r.active.get()) {
        let account_id = selected(&r.account, ids, "حساب ردیف").inspect_err(|_| {
            r.account.grab_focus();
        })?;
        let debit = money_value(&r.debit, unit)?;
        let credit = money_value(&r.credit, unit)?;
        if !((debit.rials() > 0 && credit.rials() == 0)
            || (credit.rials() > 0 && debit.rials() == 0))
        {
            r.debit.grab_focus();
            anyhow::bail!("هر ردیف باید فقط یک مبلغ مثبت بدهکار یا بستانکار داشته باشد");
        }
        total_debit = total_debit.checked_add(debit)?;
        total_credit = total_credit.checked_add(credit)?;
        lines.push(JournalLineInput {
            account_id,
            debit,
            credit,
            description: r.description.text().to_string(),
        });
    }
    anyhow::ensure!(lines.len() >= 2, "دست‌کم دو ردیف معنادار لازم است");
    Ok((lines, total_debit, total_credit))
}

fn audit_operation(operation: &AuditOperation) -> &'static str {
    match operation {
        AuditOperation::CompanyCreated => "ساخت شرکت",
        AuditOperation::PeriodCreated => "تعریف دوره",
        AuditOperation::AccountCreated => "ساخت حساب",
        AuditOperation::AccountArchived => "بایگانی حساب",
        AuditOperation::JournalPosted => "ثبت سند",
        AuditOperation::PeriodLocked => "قفل دوره",
        AuditOperation::PeriodReopened => "بازگشایی دوره",
    }
}

// Opt-in native signal workflow on a fresh disposable synthetic profile only.
// This proves widgets are connected; external keyboard/AT-SPI checks are separate.
pub(super) fn schedule(ui: &Rc<Ui>) {
    let mode = std::env::var("SARV_ACCOUNTING_SMOKE").unwrap_or_default();
    if mode != "full" && mode != "jalali" {
        return;
    }
    let jalali = mode == "jalali";
    let directory = ui.store.borrow().directory.clone();
    let safe = directory.canonicalize().is_ok_and(|p| {
        p.starts_with("/tmp")
            || p.starts_with(Path::new(env!("CARGO_MANIFEST_DIR")).join("artifacts"))
    });
    if !safe || !ui.store.borrow().state.accounting.companies.is_empty() {
        ui.message(Err::<String, _>(anyhow::anyhow!(
            "accounting smoke requires a fresh disposable /tmp or artifacts profile"
        )));
        return;
    }
    let ui = ui.clone();
    let mut stage = 0usize;
    let started = std::time::Instant::now();
    glib::timeout_add_local(Duration::from_millis(400), move || {
        if started.elapsed() > Duration::from_secs(90) {
            let _ = std::fs::write(
                directory.join("accounting-native-failure.txt"),
                format!("timeout at stage {stage}: {}", ui.status.text()),
            );
            if let Some(app) = ui.window.application() {
                app.quit();
            }
            return glib::ControlFlow::Break;
        }
        if ui.persistence_busy.get() || ui.background_busy.get() {
            return glib::ControlFlow::Continue;
        }
        let step: Result<bool> = (|| {
            match stage {
                0 => {
                    if jalali {
                        ui.stack.set_visible_child_name("settings");
                        qa_widget::<adw::ComboRow>(&ui.window, "settings-date-display")?
                            .set_selected(2);
                        qa_widget::<gtk::Button>(&ui.window, "save-settings")?.emit_clicked();
                        anyhow::ensure!(
                            ui.store.borrow().state.settings.date_display == DateDisplay::Jalali,
                            "Jalali preference did not save via settings"
                        );
                    }
                    ui.stack.set_visible_child_name("accounting");
                    ui.split.set_show_content(true);
                }
                1 => {
                    smoke::capture(&ui.window, &directory.join("accounting-before.png"))?;
                    qa_widget::<gtk::Button>(&ui.window, "accounting-new-company")?.emit_clicked();
                    let d = qa_dialog(&ui)?;
                    qa_widget::<gtk::Entry>(&d, "accounting-company-name")?
                        .set_text("شرکت ساختگی سرو • آزمون دفتر");
                    qa_widget::<gtk::Button>(&d, "accounting-company-dialog-save")?.emit_clicked();
                }
                2 => {
                    anyhow::ensure!(
                        ui.store.borrow().state.accounting.companies.len() == 1,
                        "company UI did not commit"
                    );
                    qa_widget::<gtk::Button>(&ui.window, "accounting-new-period")?.emit_clicked();
                    let d = qa_dialog(&ui)?;
                    for (name, value) in [
                        (
                            "accounting-period-name",
                            if jalali {
                                "سال مالی ساختگی ۱۴۰۵"
                            } else {
                                "دورهٔ ساختگی ۲۰۲۶"
                            },
                        ),
                        (
                            "accounting-period-start",
                            if jalali {
                                "۱۴۰۵/۰۱/۰۱"
                            } else {
                                "2026-01-01"
                            },
                        ),
                        (
                            "accounting-period-end",
                            if jalali {
                                "۱۴۰۵/۱۲/۳۰"
                            } else {
                                "2026-12-31"
                            },
                        ),
                    ] {
                        qa_widget::<gtk::Entry>(&d, name)?.set_text(value);
                    }
                    let save = qa_widget::<gtk::Button>(&d, "accounting-period-dialog-save")?;
                    if jalali {
                        save.emit_clicked();
                        anyhow::ensure!(
                            ui.store.borrow().state.accounting.periods.is_empty()
                                && !ui.persistence_busy.get(),
                            "invalid Esfand30 saved fiscal year"
                        );
                        anyhow::ensure!(
                            qa_widget::<gtk::Entry>(&d, "accounting-period-end")?.text()
                                == "۱۴۰۵/۱۲/۳۰",
                            "invalid fiscal boundary input lost"
                        );
                        qa_widget::<gtk::Entry>(&d, "accounting-period-end")?
                            .set_text("١٤٠٥/١٢/٢٩");
                    }
                    save.emit_clicked();
                }
                3 | 4 => {
                    let expected = stage - 3;
                    anyhow::ensure!(
                        ui.store.borrow().state.accounting.accounts.len() == expected,
                        "account UI prior commit failed"
                    );
                    qa_widget::<gtk::Button>(&ui.window, "accounting-new-account")?.emit_clicked();
                    let d = qa_dialog(&ui)?;
                    let (code, name, class, side) = if stage == 3 {
                        ("1001", "دارایی ساختگی", 0, 0)
                    } else {
                        ("3001", "سرمایهٔ ساختگی", 2, 1)
                    };
                    qa_widget::<gtk::Entry>(&d, "accounting-account-code")?.set_text(code);
                    qa_widget::<gtk::Entry>(&d, "accounting-account-name")?.set_text(name);
                    qa_widget::<gtk::DropDown>(&d, "accounting-account-class")?.set_selected(class);
                    qa_widget::<gtk::DropDown>(&d, "accounting-account-side")?.set_selected(side);
                    qa_widget::<gtk::Button>(&d, "accounting-account-dialog-save")?.emit_clicked();
                }
                5 => {
                    anyhow::ensure!(
                        ui.store.borrow().state.accounting.accounts.len() == 2,
                        "chart UI did not commit"
                    );
                    qa_open_journal(
                        &ui,
                        true,
                        "1251",
                        "1250",
                        "SYN-OPEN",
                        if jalali {
                            "۱۴۰۵/۰۱/۰۱"
                        } else {
                            "2026-01-01"
                        },
                    )?;
                    let d = qa_dialog(&ui)?;
                    let save = qa_widget::<gtk::Button>(&d, "accounting-journal-dialog-save")?;
                    save.emit_clicked();
                    anyhow::ensure!(
                        ui.store.borrow().state.accounting.journals.is_empty()
                            && !ui.persistence_busy.get(),
                        "unbalanced GUI posted"
                    );
                    anyhow::ensure!(
                        qa_widget::<gtk::Label>(&d, "accounting-form-error")?
                            .text()
                            .contains("برابر"),
                        "unbalanced GUI feedback absent"
                    );
                    qa_widget::<gtk::Entry>(&d, "accounting-line-2-credit")?.set_text("۱۲۵۱");
                    save.emit_clicked();
                    save.emit_clicked();
                }
                6 => {
                    anyhow::ensure!(
                        ui.store.borrow().state.accounting.journals.len() == 1,
                        "opening duplicate or missing"
                    );
                    qa_open_journal(
                        &ui,
                        false,
                        "349",
                        "349",
                        "SYN-ADJUST",
                        if jalali {
                            "۱۴۰۵/۰۲/۰۱"
                        } else {
                            "2026-02-01"
                        },
                    )?;
                    qa_widget::<gtk::Button>(&qa_dialog(&ui)?, "accounting-journal-dialog-save")?
                        .emit_clicked();
                }
                7 => {
                    anyhow::ensure!(
                        ui.store.borrow().state.accounting.journals.len() == 2,
                        "manual UI did not commit"
                    );
                    if jalali {
                        qa_widget::<gtk::DropDown>(&ui.window, "accounting-as-of-calendar")?
                            .set_selected(1);
                    }
                    qa_widget::<gtk::Entry>(&ui.window, "accounting-as-of")?.set_text(if jalali {
                        "۱۴۰۵/۱۲/۲۹"
                    } else {
                        "2026-12-31"
                    });
                    qa_widget::<gtk::Button>(&ui.window, "accounting-run-trial")?.emit_clicked();
                    qa_accounting_tabs(&ui)?.set_visible_child_name("trial");
                    let state = ui.store.borrow().state.accounting.clone();
                    let balance = state.trial_balance(
                        &state.companies[0].id,
                        if jalali { "2027-03-20" } else { "2026-12-31" },
                    )?;
                    anyhow::ensure!(
                        balance.total_debit.rials() == 1600 && balance.total_credit.rials() == 1600,
                        "independent expected pre-reversal 1600 IRR mismatch"
                    );
                    let name = state.companies[0].name.clone();
                    let destination = directory.clone();
                    ui.run_background(
                        "ساخت نمونهٔ تراز آزمون…",
                        move || {
                            sarv_quote::accounting_export::trial_pdf_with_dates(
                                &name,
                                &balance,
                                MoneyUnit::Rial,
                                if jalali {
                                    DateDisplay::Jalali
                                } else {
                                    DateDisplay::Iso
                                },
                                &destination.join("trial-before-reversal-rial.pdf"),
                            )?;
                            sarv_quote::accounting_export::trial_csv_with_dates(
                                &name,
                                &balance,
                                MoneyUnit::Rial,
                                if jalali {
                                    DateDisplay::Jalali
                                } else {
                                    DateDisplay::Iso
                                },
                                &destination.join("trial-before-reversal-rial.csv"),
                            )
                        },
                        |ui, result| ui.message(result.map(|()| "نمونهٔ تراز آزمون آماده شد")),
                    );
                }
                8 => {
                    smoke::capture(
                        &ui.window,
                        &directory.join("accounting-trial-before-reversal.png"),
                    )?;
                    qa_accounting_tabs(&ui)?.set_visible_child_name("journals");
                    let journal_id = ui.store.borrow().state.accounting.journals[1].id.clone();
                    qa_widget::<gtk::Button>(
                        &ui.window,
                        &format!("accounting-reverse-{journal_id}"),
                    )?
                    .emit_clicked();
                    let d = qa_dialog(&ui)?;
                    qa_widget::<gtk::Entry>(&d, "accounting-reversal-date")?.set_text(if jalali {
                        "۱۴۰۵/۰۲/۰۲"
                    } else {
                        "2026-02-02"
                    });
                    qa_widget::<gtk::Entry>(&d, "accounting-reversal-reason")?
                        .set_text("اصلاح آزمایشی • حفظ اصل سند");
                    qa_widget::<gtk::Button>(&d, "accounting-reversal-dialog-save")?.emit_clicked();
                }
                9 => {
                    anyhow::ensure!(
                        ui.store.borrow().state.accounting.journals.len() == 3,
                        "reversal UI did not commit"
                    );
                    qa_accounting_tabs(&ui)?.set_visible_child_name("setup");
                    let period = ui.store.borrow().state.accounting.periods[0].id.clone();
                    qa_widget::<gtk::Button>(&ui.window, &format!("accounting-lock-{period}"))?
                        .emit_clicked();
                    let d = qa_dialog(&ui)?;
                    qa_widget::<gtk::Entry>(&d, "accounting-reason")?.set_text("قفل آزمایشی دوره");
                    qa_widget::<gtk::Button>(&d, "accounting-reason-dialog-save")?.emit_clicked();
                }
                10 => {
                    anyhow::ensure!(
                        ui.store.borrow().state.accounting.periods[0].locked,
                        "lock UI did not commit"
                    );
                    qa_accounting_tabs(&ui)?.set_visible_child_name("journals");
                    if jalali {
                        let source = ui.store.borrow().state.accounting.journals[0].id.clone();
                        qa_widget::<gtk::Button>(
                            &ui.window,
                            &format!("accounting-reverse-{source}"),
                        )?
                        .emit_clicked();
                        anyhow::ensure!(
                            ui.active_dialog.borrow().is_none()
                                && ui.status.text().contains("قفل")
                                && ui.store.borrow().state.accounting.journals.len() == 3,
                            "locked source reversal was not blocked"
                        );
                    }
                    qa_widget::<gtk::Button>(&ui.window, "accounting-new-journal")?.emit_clicked();
                    anyhow::ensure!(
                        ui.active_dialog.borrow().is_none() && ui.status.text().contains("باز"),
                        "locked period UI not blocked"
                    );
                    qa_accounting_tabs(&ui)?.set_visible_child_name("setup");
                    let period = ui.store.borrow().state.accounting.periods[0].id.clone();
                    qa_widget::<gtk::Button>(&ui.window, &format!("accounting-lock-{period}"))?
                        .emit_clicked();
                    let d = qa_dialog(&ui)?;
                    qa_widget::<gtk::Entry>(&d, "accounting-reason")?
                        .set_text("بازگشایی آزمایشی با دلیل");
                    qa_widget::<gtk::Button>(&d, "accounting-reason-dialog-save")?.emit_clicked();
                }
                11 => {
                    let state = ui.store.borrow().state.clone();
                    anyhow::ensure!(
                        !state.accounting.periods[0].locked,
                        "reopen UI did not commit"
                    );
                    if jalali {
                        let accounting = &state.accounting;
                        anyhow::ensure!(
                            accounting.periods[0].start_date == "2026-03-21"
                                && accounting.periods[0].end_date == "2027-03-20",
                            "Jalali fiscal1405 canonical bounds mismatch"
                        );
                        anyhow::ensure!(
                            accounting
                                .journals
                                .iter()
                                .map(|j| j.date.as_str())
                                .collect::<Vec<_>>()
                                == vec!["2026-03-21", "2026-04-21", "2026-04-22"],
                            "Jalali journal canonical financial dates mismatch"
                        );
                        let before =
                            accounting.trial_balance(&accounting.companies[0].id, "2026-04-20")?;
                        let manual =
                            accounting.trial_balance(&accounting.companies[0].id, "2026-04-21")?;
                        anyhow::ensure!(
                            before.total_debit.rials() == 1251
                                && manual.total_debit.rials() == 1600,
                            "Jalali cutoff excludes/includes exact manual effect incorrectly"
                        );
                    }
                    let balance = state.accounting.trial_balance(
                        &state.accounting.companies[0].id,
                        if jalali { "2027-03-20" } else { "2026-12-31" },
                    )?;
                    anyhow::ensure!(
                        balance.total_debit.rials() == 1251 && balance.total_credit.rials() == 1251,
                        "independent final expected 1251 IRR mismatch"
                    );
                    qa_widget::<gtk::DropDown>(&ui.window, "accounting-report-unit")?
                        .set_selected(1);
                    qa_accounting_tabs(&ui)?.set_visible_child_name("trial");
                    let destination = directory.clone();
                    let name = state.accounting.companies[0].name.clone();
                    ui.run_background("پشتیبان و بازیابی آزمون…",move||{
                        sarv_quote::accounting_export::trial_pdf_with_dates(&name,&balance,MoneyUnit::Toman,state.settings.date_display,&destination.join("trial-final-toman.pdf"))?;
                        sarv_quote::accounting_export::trial_csv_with_dates(&name,&balance,MoneyUnit::Toman,state.settings.date_display,&destination.join("trial-final-toman.csv"))?;
                        let backup=destination.join("accounting-backup.json");sarv_quote::store::backup_state(&state,&backup)?;
                        let restored=Store::restore_preview(&backup)?;anyhow::ensure!(restored.accounting==state.accounting,"backup ledger mismatch");
                        let target=destination.join("restored-profile");let mut restored_store=Store::open(&target)?;restored_store.restore(restored)?;drop(restored_store);
                        let reopened=Store::open(&target)?;anyhow::ensure!(reopened.state.accounting==state.accounting,"fresh-profile restart ledger mismatch");
                        let original=Store::open(&destination)?;anyhow::ensure!(original.state.accounting==state.accounting,"original SQLite restart ledger mismatch");
                        sarv_quote::store::write_new(&destination.join("accounting-recovery-verification.json"),&serde_json::to_vec_pretty(&serde_json::json!({"result":"passed","journals":3,"final_trial_debit_irr":1251,"final_trial_credit_irr":1251,"backup_restore":"full accounting aggregate equal","fresh_profile_restart":"equal","original_restart":"equal"}))?)
                    },|ui,result|ui.message(result.map(|()|"بازیابی و شروع مجدد آزمون برابر بود")));
                }
                12 => {
                    smoke::capture(&ui.window, &directory.join("accounting-after.png"))?;
                    ui.window.set_default_size(650, 760);
                }
                13 => {
                    smoke::capture(&ui.window, &directory.join("accounting-narrow.png"))?;
                    anyhow::ensure!(
                        directory
                            .join("accounting-recovery-verification.json")
                            .exists(),
                        "recovery worker failed"
                    );
                    let report = serde_json::json!({"result":"passed","calendar":if jalali {"jalali"} else {"gregorian"},"mode":"GTK real widgets and signals; keyboard not implied","checks":["company/period/user-defined chart via native forms","unbalanced input rejected and preserved","balanced opening 1251 IRR; duplicate click once","manual 349 IRR; expected cumulative 1600","linked reversal preserving original; expected 1251","lock blocks native posting and reopen reason audited","trial exact toman 125.1; PDF/CSV snapshots","original restart and fresh-profile backup restore retain accounting aggregate"],"screenshots":["accounting-before.png","accounting-trial-before-reversal.png","accounting-after.png","accounting-narrow.png"]});
                    sarv_quote::store::write_new(
                        &directory.join("accounting-native-verification.json"),
                        &serde_json::to_vec_pretty(&report)?,
                    )?;
                    if let Some(app) = ui.window.application() {
                        app.quit();
                    }
                    return Ok(true);
                }
                _ => {}
            }
            stage += 1;
            Ok(false)
        })();
        match step {
            Ok(true) => glib::ControlFlow::Break,
            Ok(false) => glib::ControlFlow::Continue,
            Err(e) => {
                let _ = std::fs::write(
                    directory.join("accounting-native-failure.txt"),
                    format!("stage {stage}: {e:#}\nstatus: {}", ui.status.text()),
                );
                if let Some(app) = ui.window.application() {
                    app.quit();
                }
                glib::ControlFlow::Break
            }
        }
    });
}
fn qa_widget<T: IsA<gtk::Widget> + glib::object::ObjectType + Clone + 'static>(
    root: &impl IsA<gtk::Widget>,
    name: &str,
) -> Result<T> {
    let root = root.as_ref();
    if root.widget_name() == name
        && let Ok(value) = root.clone().downcast::<T>()
    {
        return Ok(value);
    }
    let mut child = root.first_child();
    while let Some(w) = child {
        child = w.next_sibling();
        if let Ok(value) = qa_widget::<T>(&w, name) {
            return Ok(value);
        }
    }
    Err(anyhow::anyhow!("native widget {name} absent"))
}
fn qa_dialog(ui: &Ui) -> Result<adw::Dialog> {
    ui.active_dialog
        .borrow()
        .clone()
        .ok_or_else(|| anyhow::anyhow!("native dialog absent"))
}
fn qa_accounting_tabs(ui: &Ui) -> Result<gtk::Stack> {
    let page = ui
        .stack
        .child_by_name("accounting")
        .ok_or_else(|| anyhow::anyhow!("accounting page absent"))?;
    let mut child = page.first_child();
    while let Some(w) = child {
        child = w.next_sibling();
        if let Ok(stack) = w.downcast::<gtk::Stack>() {
            return Ok(stack);
        }
    }
    Err(anyhow::anyhow!("accounting tabs absent"))
}
fn qa_open_journal(
    ui: &Ui,
    opening: bool,
    debit: &str,
    credit: &str,
    source: &str,
    date: &str,
) -> Result<()> {
    qa_accounting_tabs(ui)?.set_visible_child_name("journals");
    qa_widget::<gtk::Button>(&ui.window, "accounting-new-journal")?.emit_clicked();
    let d = qa_dialog(ui)?;
    qa_widget::<gtk::DropDown>(&d, "accounting-journal-kind")?.set_selected(if opening {
        1
    } else {
        0
    });
    for (name, value) in [
        ("accounting-journal-date", date),
        ("accounting-journal-source", source),
        (
            "accounting-journal-reason",
            "ثبت ساختگی؛ هیچ دادهٔ واقعی نیست",
        ),
        ("accounting-line-1-debit", debit),
        ("accounting-line-2-credit", credit),
    ] {
        qa_widget::<gtk::Entry>(&d, name)?.set_text(value);
    }
    qa_widget::<gtk::DropDown>(&d, "accounting-line-1-account")?.set_selected(0);
    qa_widget::<gtk::DropDown>(&d, "accounting-line-2-account")?.set_selected(1);
    Ok(())
}
