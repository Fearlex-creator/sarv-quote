//! Opt-in native integration harness. Normal launches do not seed data or export files.
use super::*;
use anyhow::{Context, ensure};

fn descendants<T: IsA<gtk::Widget> + glib::object::ObjectType + Clone + 'static>(
    w: &impl IsA<gtk::Widget>,
) -> Vec<T> {
    let mut result = vec![];
    let widget = w.as_ref();
    if let Ok(t) = widget.clone().downcast::<T>() {
        result.push(t);
    }
    let mut child = widget.first_child();
    while let Some(c) = child {
        result.extend(descendants::<T>(&c));
        child = c.next_sibling();
    }
    result
}
fn find_button(w: &impl IsA<gtk::Widget>, caption: &str) -> Result<gtk::Button> {
    descendants::<gtk::Button>(w)
        .into_iter()
        .find(|b| b.label().as_deref() == Some(caption))
        .context("native button missing")
}
pub(super) fn capture(window: &impl IsA<gtk::Window>, path: &Path) -> Result<()> {
    ensure!(!path.exists(), "screenshot already exists");
    let w = window.as_ref();
    let paintable = gtk::WidgetPaintable::new(Some(w));
    let snapshot = gtk::Snapshot::new();
    paintable.snapshot(&snapshot, f64::from(w.width()), f64::from(w.height()));
    let node = snapshot.to_node().context("native snapshot empty")?;
    let renderer = w.renderer().context("native renderer missing")?;
    let texture = renderer.render_texture(&node, None);
    texture.save_to_png(path)?;
    Ok(())
}
fn directory(ui: &Ui) -> PathBuf {
    ui.store.borrow().directory.clone()
}
fn native_flow(ui: &Rc<Ui>, approve: &gtk::Button) -> Result<()> {
    let base = ui.store.borrow().state.quotes.len();
    ensure!(base == 0, "full smoke requires empty test data");
    approve.emit_clicked();
    ensure!(
        ui.store.borrow().state.quotes.is_empty(),
        "unresolved order approved"
    );
    let ambiguous = descendants::<gtk::DropDown>(&ui.review)
        .into_iter()
        .next()
        .context("ambiguous review missing")?;
    ambiguous.set_selected(2);
    wait_background(ui)?;
    let unmatched = descendants::<gtk::DropDown>(&ui.review)
        .into_iter()
        .next()
        .context("unmatched review missing")?;
    unmatched.set_selected(4);
    wait_background(ui)?;
    ui.stale.set_active(false);
    approve.emit_clicked();
    ensure!(
        ui.store.borrow().state.quotes.is_empty(),
        "stale order approved without acknowledgement"
    );
    ui.stale.set_active(true);
    ui.unit.set_selected(1);
    ui.save_form()?;
    ensure!(
        ui.store.borrow().state.draft.discount_rial == 10_000,
        "unit conversion changed discount"
    );
    ui.unit.set_selected(0);
    ui.save_form()?;
    ensure!(
        ui.store.borrow().state.draft.discount_rial == 10_000,
        "unit round trip changed discount"
    );
    approve.emit_clicked();
    approve.emit_clicked();
    let first = ui.store.borrow().state.quotes[0].clone();
    ensure!(
        ui.store.borrow().state.quotes.len() == 1 && first.total_rial == 3_995_000,
        "approval total / repeated click mismatch"
    );
    let original = ui.store.borrow().state.draft.clone();
    let qty_entry = ui.row_editors.borrow()[0].quantity.clone();
    qty_entry.set_text("-۱");
    ensure!(
        ui.status.text().contains("ذخیره‌نشده"),
        "pending input falsely marked saved"
    );
    ui.new_draft();
    ensure!(
        qty_entry.text() == "-۱" && ui.row_editors.borrow()[0].quantity.text() == "-۱",
        "New discarded invalid cell"
    );
    find_button(&ui.quote_list, "نسخهٔ جدید")?.emit_clicked();
    ensure!(
        ui.row_editors.borrow()[0].quantity.text() == "-۱",
        "Revision discarded invalid cell"
    );
    approve.emit_clicked();
    ensure!(
        ui.store.borrow().state.quotes.len() == 1,
        "invalid native cell approved"
    );
    qty_entry.set_text("۲");
    ui.commit_table();
    wait_background(ui)?;
    ui.delete_line(0);
    wait_background(ui)?;
    ui.row_editors.borrow()[0].quantity.set_text("۱۳");
    ui.commit_table();
    wait_background(ui)?;
    ui.new_item.set_text("P-101");
    ui.new_quantity.set_text("۱");
    ui.add_line();
    wait_background(ui)?;
    ui.undo_delete();
    wait_background(ui)?;
    ensure!(
        ui.store.borrow().state.draft.lines.len() == 5
            && ui.store.borrow().state.draft.lines[1].quantity_milli == 13000,
        "Undo discarded intervening edit/add"
    );
    ui.store.borrow_mut().change(|s| {
        s.draft = original.clone();
        Ok(())
    })?;
    ui.fill_form();
    ui.refresh_review();
    ui.row_editors.borrow()[0].query.set_text("P-103");
    approve.emit_clicked();
    ensure!(
        ui.store.borrow().state.quotes.len() == 1 && !ui.stale.is_active(),
        "changed product skipped review/stale gate"
    );
    ui.store.borrow_mut().change(|s| {
        s.draft = original;
        Ok(())
    })?;
    ui.fill_form();
    ui.refresh_review();
    let valid = csv_order(&ui.store.borrow().state.draft.lines);
    ui.order.buffer().set_text("item,quantity\nP-101,-1");
    approve.emit_clicked();
    ensure!(
        ui.store.borrow().state.quotes.len() == 1,
        "invalid input approved"
    );
    ui.order.buffer().set_text(&valid);
    ui.save_form()?;
    find_button(&ui.quote_list, "نسخهٔ جدید")?.emit_clicked();
    let mut lines = ui.store.borrow().state.draft.lines.clone();
    lines[0].quantity_milli = 3000;
    ui.order.buffer().set_text(&csv_order(&lines));
    ui.stale.set_active(true);
    ui.commit_table();
    wait_background(ui)?;
    approve.emit_clicked();
    approve.emit_clicked();
    let state = ui.store.borrow().state.clone();
    ensure!(
        state.quotes.len() == 2
            && state.quotes[0].total_rial == 3_995_000
            && state.quotes[1].version == 2
            && state.quotes[1].total_rial == 5_245_000,
        "revision mutated history / repeated click mismatch"
    );
    ui.search_quote.set_text("does-not-exist");
    ui.refresh_lists();
    ensure!(
        descendants::<adw::ActionRow>(&ui.quote_list).is_empty(),
        "quote search filter failed"
    );
    ui.search_quote.set_text("پارس");
    ui.search_customer.set_text("پارس");
    ui.refresh_lists();
    ensure!(
        ui.quote_list.first_child().is_some() && ui.customer_list.first_child().is_some(),
        "customer/quote search failed"
    );
    ui.search_quote.set_text("");
    ui.search_customer.set_text("");
    ui.refresh_lists();
    let dir = directory(ui);
    let q = &state.quotes[1];
    export::pdf(q, &dir.join("sample-quote.pdf"))?;
    export::csv(q, &dir.join("sample-quote.csv"))?;
    ui.store.borrow().backup(&dir.join("backup.json"))?;
    let restored = Store::restore_preview(&dir.join("backup.json"))?;
    ensure!(restored.quotes.len() == 2, "backup validation failed");
    ui.delete_line(0);
    wait_background(ui)?;
    ensure!(
        ui.deleted_lines.borrow().is_some() && ui.undo_button.is_sensitive(),
        "deleted-line restore setup failed"
    );
    ui.customer.set_text("تغییر موقت برای آزمون بازیابی");
    ui.restore(&dir.join("backup.json"));
    wait_background(ui)?;
    let restore_dialog = ui
        .active_dialog
        .borrow()
        .clone()
        .context("restore confirmation missing")?;
    find_button(&restore_dialog, "بازیابی و ذخیرهٔ نسخهٔ ایمنی")?.emit_clicked();
    wait_background(ui)?;
    ensure!(
        ui.store.borrow().state.draft.customer == "فروشگاه نمونهٔ پارس",
        "native restore did not replace draft"
    );
    ensure!(
        ui.deleted_lines.borrow().is_none() && !ui.undo_button.is_sensitive(),
        "restore retained the prior profile's undo buffer"
    );
    let restored_lines = ui.store.borrow().state.draft.lines.clone();
    ui.undo_delete();
    wait_background(ui)?;
    ensure!(
        serde_json::to_value(&ui.store.borrow().state.draft.lines)?
            == serde_json::to_value(&restored_lines)?,
        "undo after restore mixed rows from the prior profile"
    );
    ensure!(
        std::fs::read_dir(&dir)?.filter_map(|e| e.ok()).any(|e| e
            .file_name()
            .to_string_lossy()
            .starts_with("before-restore-")),
        "safety backup missing"
    );
    let restarted = Store::open(&dir)?;
    ensure!(
        restarted.state.quotes.len() == 2 && restarted.state.draft.lines[0].quantity_milli == 3000,
        "persisted widget state mismatch"
    );
    let report = serde_json::json!({"result":"passed","native_backend":std::env::var("GDK_BACKEND").unwrap_or_else(|_| "default".into()),"checks":["catalog preview and explicit confirm","Persian/Arabic input","ambiguous review dropdown","unmatched review dropdown","unresolved approval blocked","stale acknowledgement gate","toman/rial UI preserves discount","repeated approval clicks","invalid CSV approval blocked","native revision button","immutable prior quote","customer and quote search","PDF and CSV exports","backup validated","native restore confirmation","automatic safety backup","SQLite reopened","invalid table input preserved by New/Revision/Approve","pending edits labelled unsaved","delete-edit-add-undo preserves intervening changes","changed product requires visual review and stale re-ack","restore saves immediate widget edit before safety backup","restore clears prior-profile undo buffer"],"first_total_rial":first.total_rial,"second_total_rial":q.total_rial,"quotes":2,"data_directory":dir});
    sarv_quote::store::write_new(
        &dir.join("native-verification.json"),
        &serde_json::to_vec_pretty(&report)?,
    )?;
    ui.message(Ok(
        "آزمون رابط بومی موفق • ۲ نسخهٔ محفوظ • خروجی و پشتیبان ساخته شد",
    ));
    ui.stack.set_visible_child_name("quotes");
    Ok(())
}
fn settings_flow(ui: &Rc<Ui>) -> Result<()> {
    let page = ui
        .stack
        .child_by_name("settings")
        .context("settings page missing")?;
    let rows = descendants::<adw::EntryRow>(&page);
    for (title, value) in [
        ("نام کسب‌وکار", "پخش نمونهٔ سرو"),
        ("تلفن کسب‌وکار", "۰۲۱۰۰۰۰۰۰۰۰"),
        ("پیشوند شمارهٔ پیشنهاد", "DEMO-"),
        ("شمارهٔ بعدی (پس از نخستین تأیید افزایش می‌یابد)", "۴۲"),
    ] {
        rows.iter()
            .find(|r| r.title() == title)
            .context("settings entry missing")?
            .set_text(value);
    }
    for (title, value) in [
        ("نشانی", "تهران • نشانی نمایشی"),
        ("متن سربرگ", "سربرگ نمایشی • فقط پیشنهاد قیمت"),
        ("متن پایانی", "قیمت‌ها نیازمند تأیید منبع هستند."),
        (
            "شرایط پیش‌فرض پیشنهاد تازه",
            "اعتبار قراردادی چهارده روز؛ بدون رزرو موجودی.",
        ),
    ] {
        let holder = descendants::<gtk::Box>(&page)
            .into_iter()
            .find(|b| {
                b.first_child()
                    .and_downcast::<gtk::Label>()
                    .is_some_and(|l| l.text() == title)
            })
            .context("settings multiline missing")?;
        descendants::<gtk::TextView>(&holder)
            .first()
            .context("settings text missing")?
            .buffer()
            .set_text(value);
    }
    let combos = descendants::<adw::ComboRow>(&page);
    let currency = combos
        .iter()
        .find(|r| r.title() == "واحد مبلغ")
        .context("currency setting missing")?;
    currency.set_selected(1);
    let discount = rows
        .iter()
        .find(|r| r.title() == "تخفیف پیش‌فرض در واحد مبلغ انتخاب‌شده")
        .context("discount setting missing")?;
    discount.set_text("۱۰۰۰");
    currency.set_selected(0);
    ensure!(
        Unit::Toman.nonnegative(&discount.text())? == 1000,
        "settings currency changed canonical discount"
    );
    for (title, value) in [
        ("اعتبار پیش‌فرض (روز)", 14.0),
        ("نرخ افزودهٔ قراردادی (یک‌صدم درصد؛ ۱۰۰ = ۱٪)", 500.0),
    ] {
        descendants::<adw::SpinRow>(&page)
            .iter()
            .find(|r| r.title() == title)
            .context("spin setting missing")?
            .set_value(value);
    }
    for (title, n) in [
        ("ظاهر", 1),
        ("تراکم جدول‌ها", 1),
        ("نمایش ارقام", 1),
        ("نمایش تاریخ", 1),
    ] {
        combos
            .iter()
            .find(|r| r.title() == title)
            .context("combo setting missing")?
            .set_selected(n);
    }
    let save = find_button(&page, "ذخیرهٔ تنظیمات")?;
    save.emit_clicked();
    let settings = ui.store.borrow().state.settings.clone();
    ensure!(
        settings.business_name == "پخش نمونهٔ سرو"
            && settings.header.contains("سربرگ")
            && settings.default_terms.contains("چهارده")
            && settings.next_number == 42
            && settings.default_discount_rial == 1000
            && settings.tax_basis_points == 500
            && settings.default_validity_days == 14
            && settings.appearance == Appearance::Light
            && settings.density == Density::Compact
            && settings.display_digits == DisplayDigits::Latin
            && settings.date_display == DateDisplay::DayMonthYear,
        "settings native Save mismatch"
    );
    ensure!(
        !adw::StyleManager::default().is_dark() && ui.window.has_css_class("compact-density"),
        "appearance/density not applied"
    );
    ui.new_draft();
    ui.customer.set_text("یادداشت مشتری پیش از ورود اقلام");
    ui.notes.buffer().set_text("یادداشت ساختگی پیش از اقلام");
    ui.new_draft();
    ensure!(
        std::fs::read_dir(directory(ui))?
            .filter_map(|e| e.ok())
            .any(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("before-new-draft-")
                    && Store::restore_preview(&e.path()).is_ok_and(|s| {
                        s.draft.lines.is_empty()
                            && s.draft.customer == "یادداشت مشتری پیش از ورود اقلام"
                            && s.draft.notes == "یادداشت ساختگی پیش از اقلام"
                    })
            }),
        "metadata-only draft was not recoverable after New"
    );
    ui.customer.set_text("فروشگاه نمونهٔ پارس");
    for (code, qty) in [
        ("P-101", "۳"),
        ("P-102", "۱۲"),
        ("P-103", "۳"),
        ("P-104", "۱"),
    ] {
        ui.new_item.set_text(code);
        ui.new_quantity.set_text(qty);
        ui.add_line();
        wait_background(ui)?;
    }
    ui.stale.set_active(true);
    ui.approve();
    ui.approve();
    let q = ui
        .store
        .borrow()
        .state
        .quotes
        .last()
        .cloned()
        .context("settings quote missing")?;
    ensure!(
        q.number == "DEMO-42"
            && q.version == 1
            && q.total_rial == 5516700
            && q.tax_rial == 262700
            && q.issuer_name == settings.business_name
            && q.terms == settings.default_terms
            && q.display_digits == DisplayDigits::Latin
            && q.date_display == DateDisplay::DayMonthYear
            && chrono::NaiveDate::parse_from_str(&q.valid_until, "%Y-%m-%d")?
                - chrono::NaiveDate::parse_from_str(&q.quote_date, "%Y-%m-%d")?
                == chrono::Duration::days(14),
        "settings defaults/number/tax/snapshot mismatch"
    );
    ensure!(
        ui.store.borrow().state.quotes.len() == 3
            && ui.store.borrow().state.settings.next_number == 43,
        "settings repeated approval consumed number"
    );
    let dir = directory(ui);
    export::pdf(&q, &dir.join("business-quote.pdf"))?;
    export::csv(&q, &dir.join("business-quote.csv"))?;
    // Restore Persian display after proving Latin choice, preserving approved snapshot.
    combos
        .iter()
        .find(|r| r.title() == "نمایش ارقام")
        .context("digits setting missing")?
        .set_selected(0);
    save.emit_clicked();
    ui.refresh_review();
    ui.refresh_lists();
    // Native customer manager creates an actual saved customer without editing history.
    find_button(&ui.customer_list, "مشتری تازه")?.emit_clicked();
    let dialog = ui
        .active_dialog
        .borrow()
        .clone()
        .context("customer dialog missing")?;
    let inputs = descendants::<gtk::Entry>(&dialog);
    ensure!(inputs.len() >= 5, "customer dialog contact fields missing");
    let name_input = inputs
        .iter()
        .find(|entry| entry.placeholder_text().as_deref() == Some("نام شخص یا شرکت"))
        .context("customer name field missing")?;
    let phone_input = inputs
        .iter()
        .find(|entry| entry.placeholder_text().as_deref() == Some("تلفن (اختیاری)"))
        .context("customer phone field missing")?;
    name_input.set_text("مشتری نمایشی دوم");
    phone_input.set_text("۰۹۱۲۰۰۰۰۰۰۱");
    find_button(&dialog, "ذخیرهٔ مشتری")?.emit_clicked();
    ensure!(
        ui.store.borrow().state.customers.iter().any(|c| {
            c.name == "مشتری نمایشی دوم"
                && digits(&c.phone) == "09120000001"
                && c.contact_name.is_empty()
        }),
        "native customer creation failed"
    );
    ui.store
        .borrow()
        .backup(&dir.join("settings-backup.json"))?;
    let reopened = Store::open(&dir)?;
    ensure!(
        reopened.state.settings.business_name == settings.business_name
            && reopened.state.settings.display_digits == DisplayDigits::Persian
            && reopened.state.quotes.len() == 3
            && reopened.state.draft.lines.len() == 4,
        "settings/customer/quote restart mismatch"
    );
    let report = serde_json::json!({"result":"passed","checks":["all business fields/header/footer/terms saved by native controls","toman/rial setting preserves canonical discount","default currency/discount/expiry applied to new quote","configured basis-point tax uses exact integer rial","number consumed once and stable after repeated clicks","approved issuer/terms/digits snapshots","light theme applied by Adwaita","compact geometry applied","Latin digits applied then Persian restored","customer manager native dialog creates persisted customer","settings/quote/customer SQLite reopen","PDF and CSV settings export","settings backup validated","Gregorian date format saved and snapshotted in generated documents"],"quote_number":q.number,"quote_total_rial":q.total_rial,"tax_rial":q.tax_rial,"quotes":3});
    sarv_quote::store::write_new(
        &dir.join("settings-native-verification.json"),
        &serde_json::to_vec_pretty(&report)?,
    )?;
    sarv_quote::store::write_new(
        &dir.join("before-restart-state.json"),
        &serde_json::to_vec_pretty(&reopened.state)?,
    )?;
    Ok(())
}

fn wait_background(ui: &Ui) -> Result<()> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while ui.background_busy.get()
        || ui.persistence_busy.get()
        || ui.pending_save_action.borrow().is_some()
        || ui.review_requested.get().is_some()
    {
        anyhow::ensure!(
            std::time::Instant::now() < deadline,
            "background operation timed out"
        );
        glib::MainContext::default().iteration(true);
    }
    Ok(())
}

/// Exercise new native safety surfaces on the synthetic profile only.
fn polish_flow(ui: &Rc<Ui>) -> Result<()> {
    let dir = directory(ui);
    let original = ui.store.borrow().state.draft.clone();
    let quote_count = ui.store.borrow().state.quotes.len();
    let imported = dir.join("import-order.csv");
    sarv_quote::store::write_new(&imported, b"item,quantity\nP-101,5\n")?;
    ui.import_order(&imported);
    wait_background(ui)?;
    let confirmation = ui
        .active_dialog
        .borrow()
        .clone()
        .context("order import confirmation missing")?;
    let alert = confirmation
        .clone()
        .downcast::<adw::AlertDialog>()
        .map_err(|_| anyhow::anyhow!("import confirmation is not native alert"))?;
    alert.emit_by_name::<()>("response", &[&"cancel"]);
    confirmation.close();
    ensure!(
        serde_json::to_value(&ui.store.borrow().state.draft.lines)?
            == serde_json::to_value(&original.lines)?,
        "cancel changed order"
    );
    ui.row_editors.borrow()[0].quantity.set_text("-1");
    ui.import_order(&imported);
    wait_background(ui)?;
    let confirmation = ui
        .active_dialog
        .borrow()
        .clone()
        .context("import confirmation missing")?;
    confirmation.emit_by_name::<()>("response", &[&"replace"]);
    wait_background(ui)?;
    confirmation.close();
    ensure!(
        ui.row_editors.borrow()[0].quantity.text() == "-1"
            && serde_json::to_value(&ui.store.borrow().state.draft.lines)?
                == serde_json::to_value(&original.lines)?,
        "import discarded invalid current edit"
    );
    ui.row_editors.borrow()[0].quantity.set_text(&format!(
        "{}.{:03}",
        original.lines[0].quantity_milli / 1000,
        original.lines[0].quantity_milli % 1000
    ));
    ui.commit_table();
    wait_background(ui)?;
    let before = ui.store.borrow().state.draft.clone();
    ui.import_order(&imported);
    wait_background(ui)?;
    let confirmation = ui
        .active_dialog
        .borrow()
        .clone()
        .context("import confirmation missing")?;
    confirmation.emit_by_name::<()>("response", &[&"replace"]);
    wait_background(ui)?;
    confirmation.close();
    ensure!(
        ui.store.borrow().state.draft.lines.len() == 1
            && ui.store.borrow().state.draft.lines[0].quantity_milli == 5000
            && !ui.store.borrow().state.draft.stale_ack
            && ui.store.borrow().state.draft.customer == before.customer,
        "validated order import mismatch"
    );
    let recovery = std::fs::read_dir(&dir)?
        .filter_map(|entry| entry.ok())
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("before-order-import-")
        })
        .context("order import recovery backup missing")?;
    ensure!(
        serde_json::to_value(&Store::restore_preview(&recovery.path())?.draft.lines)?
            == serde_json::to_value(&before.lines)?,
        "order import backup did not retain previous rows"
    );
    ensure!(
        ui.store.borrow().state.quotes.len() == quote_count,
        "import changed immutable history"
    );
    ui.store.borrow_mut().change(|state| {
        state.draft = original;
        Ok(())
    })?;
    ui.fill_form();
    ui.refresh_review();
    ui.refresh_lists();
    for row in descendants::<adw::ExpanderRow>(&ui.catalog_list) {
        row.set_expanded(true);
    }
    let original = ui.store.borrow().state.draft.clone();
    find_button(&ui.catalog_list, "افزودن")?.emit_clicked();
    ensure!(
        ui.store.borrow().state.draft.lines.len() == original.lines.len() + 1
            && ui
                .store
                .borrow()
                .state
                .draft
                .lines
                .last()
                .is_some_and(|line| line.quantity_milli == 1000 && line.chosen_code.is_some())
            && !ui.store.borrow().state.draft.stale_ack,
        "product manager failed exact-source one-unit append"
    );
    ensure!(
        ui.store.borrow().state.quotes.len() == quote_count,
        "product manager changed quote history"
    );
    ui.store.borrow_mut().change(|state| {
        state.draft = original;
        Ok(())
    })?;
    ui.fill_form();
    ui.refresh_review();
    let page = ui
        .stack
        .child_by_name("settings")
        .context("settings missing")?;
    let business = descendants::<adw::EntryRow>(&page)
        .into_iter()
        .find(|entry| entry.title() == "نام کسب‌وکار")
        .context("business field missing")?;
    let saved_name = ui.store.borrow().state.settings.business_name.clone();
    business.set_text("تنظیمات موقت آزمون");
    ensure!(
        settings::has_unsaved(ui),
        "settings change not marked pending"
    );
    ui.window.close();
    let prompt = ui
        .active_dialog
        .borrow()
        .clone()
        .context("unsaved settings close prompt missing")?;
    prompt.emit_by_name::<()>("response", &[&"cancel"]);
    prompt.close();
    ensure!(
        ui.window.is_visible() && settings::has_unsaved(ui),
        "cancel close discarded settings"
    );
    find_button(&page, "بازگرداندن")?.emit_clicked();
    ensure!(
        !settings::has_unsaved(ui) && ui.store.borrow().state.settings.business_name == saved_name,
        "settings revert changed saved data"
    );
    let page = ui
        .stack
        .child_by_name("settings")
        .context("settings missing")?;
    let business = descendants::<adw::EntryRow>(&page)
        .into_iter()
        .find(|entry| entry.title() == "نام کسب‌وکار")
        .context("business field missing")?;
    business.set_text("bad\u{202e}name");
    find_button(&page, "ذخیرهٔ تنظیمات")?.emit_clicked();
    ensure!(
        settings::has_unsaved(ui)
            && ui.store.borrow().state.settings.business_name == saved_name
            && business.has_css_class("error"),
        "invalid settings saved or lacked field error"
    );
    settings::discard_staged(ui);
    let report = serde_json::json!({"result":"passed", "checks":[
        "order import native confirmation", "cancel import preserves rows", "invalid edit blocks replacement",
        "confirmed import retains customer and resets stale acknowledgment", "exact prior rows retained in recovery backup",
        "import preserves immutable quote history", "settings edits visibly pending", "settings close cancel retains edits and window",
        "settings revert preserves saved data", "invalid settings rejected with native field feedback", "product manager appends exact-source unit without changing history"]});
    sarv_quote::store::write_new(
        &dir.join("polish-native-verification.json"),
        &serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(())
}

pub(super) fn schedule(ui: &Rc<Ui>, approve: &gtk::Button) {
    let Ok(mode) = std::env::var("SARV_UI_SMOKE") else {
        return;
    };
    if std::env::var_os("SARV_FLOAT_TEST").is_some() {
        let class = if std::env::var_os("SARV_TEST_INSTANCE").is_some() {
            "ir.sarv.quote.verification"
        } else {
            "ir.sarv.quote.v1"
        };
        glib::timeout_add_local_once(std::time::Duration::from_secs(1), move || {
            let code = format!(
                "hl.dispatch(hl.dsp.window.float({{window=\"class:{class}\",action=\"on\"}})); hl.dispatch(hl.dsp.window.resize({{window=\"class:{class}\",x=1400,y=940}}))"
            );
            let _ = std::process::Command::new("hyprctl")
                .args(["eval", &code])
                .output();
        });
    }
    let event_keys = gtk::EventControllerKey::new();
    event_keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let weak = Rc::downgrade(ui);
    event_keys.connect_key_pressed(move |_, key, _, _| {
        if let Some(ui) = weak.upgrade() {
            let key = key.name().map(|s| s.to_string()).unwrap_or_default();
            glib::idle_add_local_once(move || {
                // A modal Dialog owns its focus independently of GtkWindow.
                // Report that focus rather than an underlying header control.
                let dialog = ui.visible_dialog();
                let dialog_focus = dialog.as_ref().and_then(|dialog| dialog.focus());
                let focus_scope = if dialog_focus.is_some() {
                    "dialog"
                } else if dialog.is_some() {
                    "window-fallback"
                } else {
                    "window"
                };
                let focus = dialog_focus.or_else(|| gtk::prelude::GtkWindowExt::focus(&ui.window));
                let contains = |widget: &gtk::Widget| focus.as_ref().is_some_and(|w| w == widget || w.is_ancestor(widget));
                let mut target = "other".to_string();
                for row in ui.row_editors.borrow().iter() {
                    if contains(row.query.upcast_ref()) {
                        target = format!("row{}-query", row.index);
                    } else if contains(row.quantity.upcast_ref()) {
                        target = format!("row{}-quantity", row.index);
                    }
                }
                for (name, field) in [("new-item", &ui.new_item), ("new-quantity", &ui.new_quantity), ("discount", &ui.discount), ("customer", &ui.customer), ("phone", &ui.phone)] {
                    if contains(field.upcast_ref()) { target = name.into(); }
                }
                for (name, search) in [("search-catalog", &ui.search_catalog), ("search-customer", &ui.search_customer), ("search-quote", &ui.search_quote)] {
                    if contains(search.upcast_ref()) { target = name.into(); }
                }
                let mut ancestor = focus.clone();
                let mut focus_label = None;
                while let Some(widget) = ancestor {
                    if let Some(row) = widget.downcast_ref::<adw::EntryRow>() {
                        focus_label = Some(row.title().to_string());
                        break;
                    }
                    if let Some(button) = widget.downcast_ref::<gtk::Button>() {
                        focus_label = button.label().map(|s| s.to_string());
                        if focus_label.is_some() { break; }
                    }
                    ancestor = widget.parent();
                }
                let rect_values = |bounds: gtk::graphene::Rect| [bounds.x(), bounds.y(), bounds.width(), bounds.height()];
                let focus_bounds = focus.as_ref().and_then(|widget| widget.compute_bounds(&ui.window)).map(rect_values);
                let focus_clip_bounds = focus.as_ref().and_then(|widget| widget.ancestor(gtk::ScrolledWindow::static_type()))
                    .and_then(|widget| widget.compute_bounds(&ui.window)).map(rect_values);
                let event = serde_json::json!({"key":key,"focus":target,"focus_label":focus_label,"focus_type":focus.as_ref().map(|widget|widget.type_().name().to_string()),"focus_name":focus.as_ref().map(|widget|widget.widget_name().to_string()),"focus_bounds":focus_bounds,"focus_clip_bounds":focus_clip_bounds,"window_size":[ui.window.width(),ui.window.height()],"page":ui.stack.visible_child_name().map(|s|s.to_string()),"quotes":ui.store.borrow().state.quotes.len(),"settings_dirty":settings::has_unsaved(&ui),"dialog_open":ui.active_dialog.borrow().is_some(),"focus_scope":focus_scope,"dialog_title":dialog.as_ref().map(|dialog|dialog.title().to_string())});
                use std::io::Write;
                if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(directory(&ui).join("keyboard-events.jsonl")) {
                    let _ = writeln!(file, "{event}");
                }
            });
        }
        glib::Propagation::Proceed
    });
    ui.window.add_controller(event_keys);
    if mode == "capture" {
        let ui = ui.clone();
        glib::timeout_add_local_once(std::time::Duration::from_secs(2), move || {
            let page = std::env::var("SARV_CAPTURE_PAGE").unwrap_or_else(|_| "quotes".into());
            ui.stack.set_visible_child_name(&page);
            if std::env::var_os("SARV_EXPAND").is_some() {
                for row in descendants::<adw::ExpanderRow>(&ui.window) {
                    row.set_expanded(true);
                }
            }
            if std::env::var_os("SARV_PREVIEW").is_some() {
                ui.preview_document();
            }
            if std::env::var_os("SARV_FOCUS_ROW").is_some()
                && let Some(row) = ui.row_editors.borrow().first()
            {
                row.query.grab_focus();
            }
            let u = ui.clone();
            let delay = std::env::var("SARV_CAPTURE_DELAY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1);
            glib::timeout_add_local_once(std::time::Duration::from_secs(delay), move || {
                u.message(
                    capture(
                        &u.window,
                        &directory(&u).join(
                            std::env::var("SARV_CAPTURE_NAME")
                                .unwrap_or_else(|_| "native-restart.png".into()),
                        ),
                    )
                    .map(|()| "بازیابی پس از اجرای مجدد تأیید شد"),
                );
            });
        });
        return;
    }
    let full = mode == "full";
    if full {
        ui.window.maximize();
    }
    let u = ui.clone();
    glib::timeout_add_local_once(std::time::Duration::from_secs(2), move || {
        if full {
            u.import_catalog(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("demo/prices.csv"));
        } else {
            let r = imports::catalog_csv(
                include_str!("../../demo/prices.csv"),
                "فهرست نمایشی مهر ۱۴۰۵",
            )
            .and_then(|c| {
                u.store
                    .borrow_mut()
                    .change(|s| imports::accept_catalog(s, c))
            });
            u.message(r.map(|_| "دادهٔ نمایشی"));
        }
    });
    let u = ui.clone();
    glib::timeout_add_local_once(std::time::Duration::from_secs(4), move || {
        let r = (|| -> Result<()> {
            if full {
                let dialog = u
                    .active_dialog
                    .borrow()
                    .clone()
                    .context("catalog preview missing")?;
                capture(&u.window, &directory(&u).join("native-catalog-preview.png"))?;
                find_button(&dialog, "منبع و قیمت‌ها را تأیید می‌کنم")?.emit_clicked();
                wait_background(&u)?;
            }
            u.fill_form();
            u.customer.set_text("فروشگاه نمونهٔ پارس");
            u.phone.set_text("۰۹۱۲۰۰۰۰۰۰۰");
            u.order
                .buffer()
                .set_text(include_str!("../../demo/order.csv"));
            u.discount.set_text("۱۰۰۰");
            u.stale.set_active(true);
            u.save_form()?;
            u.refresh_review();
            Ok(())
        })();
        u.message(r.map(|()| "حالت آزمون • فقط دادهٔ ساختگی"));
    });
    let u = ui.clone();
    glib::timeout_add_local_once(std::time::Duration::from_secs(6), move || {
        u.message(
            capture(&u.window, &directory(&u).join("native-review.png"))
                .map(|()| "رابط بررسی ثبت شد"),
        );
    });
    if full {
        let u = ui.clone();
        glib::timeout_add_local_once(std::time::Duration::from_secs(7), move || {
            if let Some(scroll) = u
                .review
                .ancestor(gtk::ScrolledWindow::static_type())
                .and_then(|w| w.downcast::<gtk::ScrolledWindow>().ok())
            {
                let a = scroll.vadjustment();
                a.set_value(a.upper() - a.page_size());
            }
            let u2 = u.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(300), move || {
                u2.message(
                    capture(&u2.window, &directory(&u2).join("native-human-review.png"))
                        .map(|()| "صف بررسی انسانی ثبت شد"),
                );
            });
        });
        let u = ui.clone();
        let approve = approve.clone();
        glib::timeout_add_local_once(std::time::Duration::from_secs(8), move || {
            let r = native_flow(&u, &approve)
                .and_then(|()| settings_flow(&u))
                .and_then(|()| polish_flow(&u));
            if let Err(e) = &r {
                let _ = sarv_quote::store::write_new(
                    &directory(&u).join("native-failure.txt"),
                    format!("{e:#}").as_bytes(),
                );
            }
            u.message(r.map(|()| "آزمون رابط بومی موفق"));
        });
        let u = ui.clone();
        glib::timeout_add_local_once(std::time::Duration::from_secs(10), move || {
            u.message(
                capture(&u.window, &directory(&u).join("native-archive.png"))
                    .map(|()| "بایگانی دو نسخه ثبت شد"),
            );
        });
        let u = ui.clone();
        glib::timeout_add_local_once(std::time::Duration::from_secs(12), move || {
            u.stack.set_visible_child_name("order");
            u.refresh_review();
            let u2 = u.clone();
            glib::timeout_add_local_once(std::time::Duration::from_secs(1), move || {
                u2.message(
                    capture(
                        &u2.window,
                        &directory(&u2).join("native-after-quotation.png"),
                    )
                    .map(|()| "پیشنهاد ثبت شد"),
                );
                u2.preview_document();
            });
        });
        let u = ui.clone();
        glib::timeout_add_local_once(std::time::Duration::from_secs(15), move || {
            let active = u.active_dialog.borrow().clone();
            if let Some(dialog) = active {
                u.message(
                    capture(&u.window, &directory(&u).join("native-print-preview.png"))
                        .map(|()| "پیش‌نمایش چاپ ثبت شد"),
                );
                dialog.close();
            }
        });
        let u = ui.clone();
        glib::timeout_add_local_once(std::time::Duration::from_secs(17), move || {
            u.stack.set_visible_child_name("quotes");
            for row in descendants::<adw::ExpanderRow>(&u.quote_list) {
                row.set_expanded(true);
            }
            if let Ok(button) = find_button(&u.quote_list, "پیش‌نمایش چاپ") {
                button.emit_clicked();
            }
        });
        let u = ui.clone();
        glib::timeout_add_local_once(std::time::Duration::from_secs(19), move || {
            let active = u.active_dialog.borrow().clone();
            if let Some(dialog) = active {
                if let Some(zoom) = descendants::<gtk::DropDown>(&dialog).first() {
                    zoom.set_selected(2);
                    let large = descendants::<gtk::Picture>(&dialog)
                        .iter()
                        .all(|p| p.width_request() == 875);
                    zoom.set_selected(0);
                    let small = descendants::<gtk::Picture>(&dialog)
                        .iter()
                        .all(|p| p.width_request() == 525);
                    zoom.set_selected(1);
                    let report = serde_json::json!({"result":if large&&small{"passed"}else{"failed"},"checks":["saved approved quotation opens native dialog","native preview zoom125 and75 changes picture geometry","100-percent preview restored"]});
                    let _ = sarv_quote::store::write_new(
                        &directory(&u).join("preview-native-verification.json"),
                        &serde_json::to_vec_pretty(&report).unwrap_or_default(),
                    );
                }
                let u2 = u.clone();
                glib::timeout_add_local_once(std::time::Duration::from_millis(400), move || {
                    u2.message(
                        capture(
                            &u2.window,
                            &directory(&u2).join("native-approved-preview.png"),
                        )
                        .map(|()| "پیش‌نمایش نسخهٔ تأییدشده ثبت شد"),
                    );
                    dialog.close();
                });
            }
        });
        for (time, page, name) in [
            (21, "catalog", "native-products.png"),
            (24, "customers", "native-customers.png"),
            (27, "settings", "native-settings.png"),
            (30, "quotes", "native-quotes.png"),
        ] {
            let u = ui.clone();
            glib::timeout_add_local_once(std::time::Duration::from_secs(time), move || {
                u.stack.set_visible_child_name(page);
                if page == "catalog" || page == "quotes" {
                    for row in descendants::<adw::ExpanderRow>(if page == "catalog" {
                        &u.catalog_list
                    } else {
                        &u.quote_list
                    }) {
                        row.set_expanded(true);
                    }
                }
                let u2 = u.clone();
                glib::timeout_add_local_once(std::time::Duration::from_millis(500), move || {
                    let _ = capture(&u2.window, &directory(&u2).join(name));
                });
            });
        }
    }
}
