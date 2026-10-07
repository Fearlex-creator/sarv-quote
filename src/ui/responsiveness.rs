//! Opt-in native scale probe. Synthetic writes require an explicitly selected
//! empty profile; normal application launches never run this code.
use super::*;
use anyhow::{Context, ensure};
use std::time::{Duration, Instant};

fn descendants<T: IsA<gtk::Widget> + glib::object::ObjectType + Clone + 'static>(
    widget: &impl IsA<gtk::Widget>,
) -> Vec<T> {
    let mut result = vec![];
    if let Ok(value) = widget.as_ref().clone().downcast::<T>() {
        result.push(value);
    }
    let mut child = widget.as_ref().first_child();
    while let Some(value) = child {
        result.extend(descendants::<T>(&value));
        child = value.next_sibling();
    }
    result
}

fn button_named(widget: &impl IsA<gtk::Widget>, name: &str) -> Result<gtk::Button> {
    descendants::<gtk::Button>(widget)
        .into_iter()
        .find(|button| button.label().as_deref() == Some(name))
        .context("native pagination button missing")
}

fn page_rows(widget: &impl IsA<gtk::Widget>) -> Vec<adw::ActionRow> {
    descendants::<adw::ActionRow>(widget)
}

fn persisted_generation(ui: &Ui) -> Result<i64> {
    let connection = rusqlite::Connection::open_with_flags(
        ui.store.borrow().directory.join("sarv.sqlite3"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    Ok(
        connection.query_row("SELECT generation FROM state WHERE id=1", [], |row| {
            row.get(0)
        })?,
    )
}

// Inject a genuine X11 Tab event without blocking GTK on a subprocess wait.
// Wayland and systems without XTest are reported as untested, never passed.
fn inject_tab() -> Result<std::process::Child> {
    ensure!(
        std::env::var("GDK_BACKEND").as_deref() == Ok("x11"),
        "X11 key injection unavailable"
    );
    let script = r#"import ctypes as C
x=C.CDLL('libX11.so.6'); t=C.CDLL('libXtst.so.6')
x.XOpenDisplay.argtypes=[C.c_char_p]; x.XOpenDisplay.restype=C.c_void_p
x.XStringToKeysym.argtypes=[C.c_char_p]; x.XStringToKeysym.restype=C.c_ulong
x.XKeysymToKeycode.argtypes=[C.c_void_p,C.c_ulong]; x.XKeysymToKeycode.restype=C.c_uint
x.XFlush.argtypes=[C.c_void_p]; x.XCloseDisplay.argtypes=[C.c_void_p]
t.XTestFakeKeyEvent.argtypes=[C.c_void_p,C.c_uint,C.c_int,C.c_ulong]
d=x.XOpenDisplay(None); assert d
k=x.XKeysymToKeycode(d,x.XStringToKeysym(b'Tab')); assert k
t.XTestFakeKeyEvent(d,k,1,0); t.XTestFakeKeyEvent(d,k,0,0)
x.XFlush(d); x.XCloseDisplay(d)
"#;
    Ok(std::process::Command::new("/usr/bin/python3")
        .args(["-c", script])
        .spawn()?)
}

struct Probe {
    began: Instant,
    stage: u8,
    ready_at: Instant,
    preview: Option<gtk::Box>,
    dialog: Option<adw::Dialog>,
    page: usize,
    max_rows: usize,
    checks: Vec<String>,
    durations: serde_json::Map<String, serde_json::Value>,
    heartbeat_last: Instant,
    heartbeat_count: usize,
    heartbeat_max_ms: f64,
    export_started: Option<Instant>,
    export_done: bool,
    export_error: Option<String>,
    export_ticks: usize,
    export_max_gap_ms: f64,
    large_gaps: Vec<serde_json::Value>,
    page_clicks: Vec<serde_json::Value>,
    keyboard: Option<std::process::Child>,
    finished: bool,
}

impl Probe {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            began: now,
            stage: 0,
            ready_at: now,
            preview: None,
            dialog: None,
            page: 0,
            max_rows: 0,
            checks: vec![],
            durations: serde_json::Map::new(),
            heartbeat_last: now,
            heartbeat_count: 0,
            heartbeat_max_ms: 0.,
            export_started: None,
            export_done: false,
            export_error: None,
            export_ticks: 0,
            export_max_gap_ms: 0.,
            large_gaps: vec![],
            page_clicks: vec![],
            keyboard: None,
            finished: false,
        }
    }
    fn advance(&mut self, stage: u8, delay_ms: u64) {
        self.stage = stage;
        self.ready_at = Instant::now() + Duration::from_millis(delay_ms);
    }
    fn elapsed(&mut self, name: &str, started: Instant) {
        self.durations.insert(
            name.into(),
            serde_json::json!(started.elapsed().as_secs_f64() * 1000.),
        );
    }
    fn finish(&mut self, ui: &Ui, error: Option<String>) {
        if self.finished {
            return;
        }
        self.finished = true;
        let passed = error.is_none();
        let report = serde_json::json!({
            "result": if passed {"passed"} else {"failed"},
            "synthetic_only": true,
            "products": 50_000, "quote_lines": 1_000,
            "catalog_preview_pages_visited": self.page + 1,
            "maximum_native_preview_rows": self.max_rows,
            "durations_ms": self.durations,
            "heartbeat_interval_ms": 20,
            "heartbeat_count": self.heartbeat_count,
            "maximum_heartbeat_gap_ms_including_setup_and_editor_paging": self.heartbeat_max_ms,
            "heartbeat_ticks_while_pdf_busy": self.export_ticks,
            "maximum_heartbeat_gap_ms_while_pdf_busy": self.export_max_gap_ms,
            "pdf_busy_responsiveness_threshold_ms": 250,
            "heartbeat_gaps_over_100_ms": self.large_gaps,
            "editor_page_clicks": self.page_clicks,
            "maximum_unchanged_editor_page_click_ms": self.page_clicks.iter()
                .filter(|click| click["draft_changed"] == false)
                .filter_map(|click| click["duration_ms"].as_f64())
                .fold(0.0, f64::max),
            "final_window_allocation": [ui.window.width(), ui.window.height()],
            "checks": self.checks,
            "failure": error,
            "limitations": ["Synthetic scale fixture, not production customer data", "SQLite state validation and persistence remain synchronous; measured setup/page-edit gaps are reported separately", "No Wayland, physical printer or target desktop packaging certification"]
        });
        let path = ui
            .store
            .borrow()
            .directory
            .join("native-responsiveness-verification.json");
        let result = serde_json::to_vec_pretty(&report)
            .map_err(anyhow::Error::from)
            .and_then(|bytes| sarv_quote::store::write_new(&path, &bytes));
        ui.message(result.map(|()| {
            if passed {
                "آزمون پاسخ‌گویی ساختگی موفق"
            } else {
                "آزمون پاسخ‌گویی ساختگی ناموفق؛ گزارش را بررسی کنید"
            }
        }));
    }
}

fn step(ui: &Rc<Ui>, probe: &Rc<RefCell<Probe>>) -> Result<()> {
    let mut p = probe.borrow_mut();
    let directory = ui.store.borrow().directory.clone();
    match p.stage {
        0 => {
            let begin = Instant::now();
            let source = chrono::Utc::now().to_rfc3339();
            let products = (1..=50_000)
                .map(|i| Product {
                    code: format!("SCALE-{i:05}"),
                    name: format!("کالای ساختگی پاسخ‌گویی {i}"),
                    measure: "عدد".into(),
                    price_rial: 100_000 + i,
                    source_unit: Unit::Rial,
                    source_at: source.clone(),
                    aliases: vec![],
                })
                .collect::<Vec<_>>();
            let catalog = Catalog {
                id: id(),
                name: "۵۰ هزار کالای ساختگی؛ آزمون پاسخ‌گویی".into(),
                imported_at: source,
                fingerprint: imports::product_fingerprint(&products)?,
                products,
            };
            ui.store
                .borrow_mut()
                .change(|state| imports::accept_catalog(state, catalog.clone()))?;
            p.elapsed("synthetic_catalog_validated_store_commit", begin);
            let begin = Instant::now();
            let preview = catalog_preview::preview(&catalog, DisplayDigits::Persian);
            let dialog = adw::Dialog::builder()
                .title("آزمون ۵۰ هزار کالا؛ دادهٔ ساختگی")
                .content_width(1000)
                .content_height(780)
                .build();
            dialog.set_child(Some(&dialog_toolbar(&preview)));
            ui.present_dialog(&dialog);
            p.elapsed("catalog_preview_widget_creation", begin);
            p.preview = Some(preview);
            p.dialog = Some(dialog);
            p.advance(1, 600);
        }
        1 => {
            let preview = p.preview.as_ref().context("preview missing")?;
            let rows = page_rows(preview);
            ensure!(
                rows.len() == 100 && rows[0].title().contains("SCALE-00001"),
                "first native page incorrect"
            );
            p.max_rows = rows.len();
            smoke::capture(
                &ui.window,
                &directory.join("native-scale-catalog-first.png"),
            )?;
            button_named(p.preview.as_ref().unwrap(), "صفحهٔ بعد")?.emit_clicked();
            p.page = 1;
            p.advance(2, 150);
        }
        2 => {
            let preview = p.preview.as_ref().context("preview missing")?;
            let rows = page_rows(preview);
            ensure!(
                rows.len() == 100 && rows[0].title().contains("SCALE-00101"),
                "next native page incorrect"
            );
            smoke::capture(&ui.window, &directory.join("native-scale-catalog-next.png"))?;
            p.checks.push(
                "Actual GTK import preview first and next page contain 100 expected products"
                    .into(),
            );
            p.advance(3, 20);
        }
        3 => {
            let preview = p.preview.as_ref().context("preview missing")?;
            let rows = page_rows(preview);
            ensure!(rows.len() <= 100, "native preview grew beyond 100 rows");
            p.max_rows = p.max_rows.max(rows.len());
            if p.page < 499 {
                button_named(p.preview.as_ref().unwrap(), "صفحهٔ بعد")?.emit_clicked();
                p.page += 1;
            } else {
                if let Some(scroller) =
                    descendants::<gtk::ScrolledWindow>(p.preview.as_ref().unwrap()).first()
                {
                    let adjustment = scroller.vadjustment();
                    adjustment.set_value(adjustment.upper() - adjustment.page_size());
                }
                p.advance(4, 300);
            }
        }
        4 => {
            let preview = p.preview.as_ref().context("preview missing")?;
            let rows = page_rows(preview);
            ensure!(
                rows.len() == 100 && rows[99].title().contains("SCALE-50000"),
                "last native page inaccessible"
            );
            ensure!(
                !button_named(preview, "صفحهٔ بعد")?.is_sensitive(),
                "last-page next remains enabled"
            );
            smoke::capture(&ui.window, &directory.join("native-scale-catalog-last.png"))?;
            p.checks.push("All 500 native pages reached by next button; final row is SCALE-50000; next disabled".into());
            let began = p.began;
            p.elapsed(
                "catalog_pagination_all_500_pages_including_initial_setup",
                began,
            );
            p.dialog.take().context("preview dialog missing")?.close();
            p.preview.take();
            let begin = Instant::now();
            ui.store.borrow_mut().change(|state| {
                state.draft.customer = "مشتری ساختگی پاسخ‌گویی".into();
                state.draft.lines = (1..=1000)
                    .map(|i| OrderLine {
                        query: format!("SCALE-{i:05}"),
                        quantity_milli: 1000,
                        chosen_code: None,
                    })
                    .collect();
                Ok(())
            })?;
            ui.fill_form();
            ui.refresh_review();
            ui.stack.set_visible_child_name("order");
            p.elapsed(
                "1000_line_store_commit_and_first_100_editor_widget_creation",
                begin,
            );
            p.advance(10, 1000);
        }
        10 => {
            ensure!(
                ui.store.borrow().state.draft.lines.len() == 1000,
                "canonical 1,000 lines lost"
            );
            ensure!(
                ui.row_editors.borrow().len() == 100,
                "first native editor page must contain exactly 100 rows"
            );
            if ui.table_page.get() == 0 {
                ui.row_editors.borrow()[0].quantity.set_text("2");
            }
            if ui.table_page.get() < 9 {
                let page = ui.table_page.get();
                let before = persisted_generation(ui)?;
                let started = Instant::now();
                button_named(&ui.review, "بعدی")?.emit_clicked();
                let elapsed = started.elapsed().as_secs_f64() * 1000.;
                let after = persisted_generation(ui)?;
                ensure!(
                    after == before + i64::from(page == 0),
                    "editor next page changed generation without edits or failed to persist first-page edit"
                );
                p.page_clicks
                    .push(serde_json::json!({"direction":"next", "from_page":page + 1,
                    "draft_changed":page == 0, "generation_before":before,
                    "generation_after":after, "duration_ms":elapsed}));
                ensure!(
                    ui.row_editors.borrow().len() <= 100,
                    "editor page exceeded 100 rows"
                );
                return Ok(());
            }
            ensure!(
                ui.row_editors
                    .borrow()
                    .last()
                    .is_some_and(|row| row.query.text() == "SCALE-01000"),
                "last native editor page inaccessible"
            );
            ensure!(
                !button_named(&ui.review, "بعدی")?.is_sensitive(),
                "editor final page next still enabled"
            );
            ui.row_editors
                .borrow()
                .last()
                .context("last row missing")?
                .quantity
                .set_text("3");
            ui.save_form()?;
            ui.refresh_review();
            p.advance(11, 300);
        }
        11 => {
            smoke::capture(
                &ui.window,
                &directory.join("native-scale-editor-last-page.png"),
            )?;
            p.advance(12, 20);
        }
        12 => {
            if ui.table_page.get() > 0 {
                let page = ui.table_page.get();
                let before = persisted_generation(ui)?;
                let started = Instant::now();
                button_named(&ui.review, "قبلی")?.emit_clicked();
                let elapsed = started.elapsed().as_secs_f64() * 1000.;
                let after = persisted_generation(ui)?;
                ensure!(
                    after == before,
                    "unchanged editor previous page persisted a new generation"
                );
                p.page_clicks.push(
                    serde_json::json!({"direction":"previous", "from_page":page + 1,
                    "draft_changed":false, "generation_before":before,
                    "generation_after":after, "duration_ms":elapsed}),
                );
                ensure!(
                    ui.row_editors.borrow().len() <= 100,
                    "editor page exceeded 100 rows"
                );
                return Ok(());
            }
            let store = ui.store.borrow();
            ensure!(
                store.state.draft.lines.len() == 1000,
                "off-page quote lines lost on paging save"
            );
            ensure!(
                store.state.draft.lines[0].quantity_milli == 2000
                    && store.state.draft.lines[999].quantity_milli == 3000,
                "first/last page edits not preserved across navigation"
            );
            let quote = store.state.preview()?;
            ensure!(
                quote.lines.len() == 1000 && quote.total_rial == 100_802_501,
                "full 1,000-line quotation total changed after page edits"
            );
            p.checks.push("Native editor reaches all ten pages with <=100 rows/page; first/last page quantity edits survive navigation and save; all1,000 canonical lines retained; exact100,802,501rial total".into());
            p.checks.push("All seventeen unchanged native next/previous clicks preserve persisted generation; edited first-page navigation commits exactly one generation".into());
            p.advance(5, 500);
        }
        5 => {
            ensure!(
                ui.row_editors.borrow().len() == 100,
                "100 bounded native editor rows missing"
            );
            smoke::capture(&ui.window, &directory.join("native-scale-editor-1000.png"))?;
            let quote = ui.store.borrow().state.preview()?;
            ensure!(
                quote.lines.len() == 1000 && quote.total_rial == 100_802_501,
                "PDF snapshot does not include all exact canonical amounts"
            );
            let path = directory.join("native-scale-1000-lines.pdf");
            let started = Instant::now();
            p.export_started = Some(started);
            p.heartbeat_last = started;
            drop(p);
            let complete_probe = probe.clone();
            ensure!(
                ui.run_background(
                    "آزمون ساختگی؛ PDF هزار ردیف در پس‌زمینه",
                    move || export::pdf(&quote, &path),
                    move |_, result| {
                        let mut p = complete_probe.borrow_mut();
                        p.export_done = true;
                        p.export_error = result.err().map(|error| format!("{error:#}"));
                        p.elapsed("1000_line_background_pdf_completion", started);
                    }
                ),
                "background worker rejected"
            );
            probe.borrow_mut().advance(6, 30);
        }
        6 => {
            ensure!(
                ui.background_busy.get(),
                "PDF finished before keyboard probe; busy keyboard test untested"
            );
            ui.row_editors.borrow()[0].query.grab_focus();
            p.keyboard = Some(inject_tab()?);
            p.advance(7, 150);
        }
        7 => {
            ensure!(
                ui.background_busy.get(),
                "PDF finished before keyboard verification; busy keyboard test untested"
            );
            let status = p
                .keyboard
                .as_mut()
                .context("key process missing")?
                .try_wait()?;
            ensure!(
                status.is_some_and(|status| status.success()),
                "XTest key subprocess did not complete successfully"
            );
            let focus = gtk::prelude::GtkWindowExt::focus(&ui.window)
                .context("native keyboard focus missing")?;
            let quantity = ui.row_editors.borrow()[0].quantity.clone();
            ensure!(
                focus == quantity.clone().upcast::<gtk::Widget>() || focus.is_ancestor(&quantity),
                "real Tab did not move native query focus to quantity during PDF"
            );
            p.checks.push("Genuine X11 XTest Tab moves first row query focus to quantity while PDF worker is busy".into());
            ui.stack.set_visible_child_name("settings");
            ensure!(
                ui.stack.visible_child_name().as_deref() == Some("settings"),
                "settings navigation blocked by worker"
            );
            p.checks
                .push("Native settings navigation completes while PDF worker is busy".into());
            p.advance(8, 200);
        }
        8 => {
            smoke::capture(
                &ui.window,
                &directory.join("native-scale-settings-while-pdf.png"),
            )?;
            ui.stack.set_visible_child_name("order");
            ui.window.set_default_size(1000, 780);
            p.advance(9, 300);
        }
        9 => {
            if !p.export_done {
                return Ok(());
            }
            smoke::capture(
                &ui.window,
                &directory.join("native-scale-editor-narrow.png"),
            )?;
            ensure!(
                p.export_error.is_none(),
                "PDF worker failed: {:?}",
                p.export_error
            );
            ensure!(
                p.export_ticks >= 3,
                "too few main-loop heartbeat ticks while PDF busy"
            );
            ensure!(
                p.export_max_gap_ms < 250.,
                "GTK busy heartbeat exceeded 250 ms: {:.1}",
                p.export_max_gap_ms
            );
            ensure!(
                std::fs::metadata(directory.join("native-scale-1000-lines.pdf"))?.len() > 1000,
                "PDF absent or empty"
            );
            p.checks.push("PDF export completed within 120-second total probe timeout; GTK heartbeat stayed below 250 ms while busy".into());
            p.finish(ui, None);
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn schedule(ui: &Rc<Ui>) {
    if std::env::var("SARV_RESPONSIVENESS").as_deref() != Ok("1") {
        return;
    }
    let guard = (|| -> Result<()> {
        let selected = std::env::var_os("SARV_DATA_DIR")
            .context("responsiveness requires explicit SARV_DATA_DIR")?;
        let store = ui.store.borrow();
        let connection = rusqlite::Connection::open_with_flags(
            store.directory.join("sarv.sqlite3"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let generation: i64 =
            connection.query_row("SELECT generation FROM state WHERE id=1", [], |row| {
                row.get(0)
            })?;
        ensure!(
            generation == 0,
            "responsiveness requires a fresh, never-modified profile"
        );
        ensure!(
            Path::new(&selected) == store.directory,
            "synthetic profile path mismatch"
        );
        let state = &store.state;
        ensure!(
            state.catalogs.is_empty()
                && state.customers.is_empty()
                && state.quotes.is_empty()
                && state.draft.lines.is_empty()
                && state.draft.customer.is_empty()
                && state.draft.phone.is_empty()
                && state.draft.catalog_id.is_empty()
                && state.draft.reference.is_empty()
                && state.draft.notes.is_empty()
                && state.draft.discount_rial == 0,
            "responsiveness refuses nonempty profile; existing data preserved"
        );
        ensure!(
            !store
                .directory
                .join("native-responsiveness-verification.json")
                .exists(),
            "responsiveness report already exists"
        );
        ensure!(
            std::env::var_os("SARV_SMOKE").is_none(),
            "responsiveness and smoke cannot share one profile"
        );
        Ok(())
    })();
    if let Err(error) = guard {
        ui.message(Err::<String, _>(error));
        return;
    }
    let probe = Rc::new(RefCell::new(Probe::new()));
    let heartbeat_probe = probe.clone();
    let weak = Rc::downgrade(ui);
    glib::timeout_add_local(Duration::from_millis(20), move || {
        let Some(ui) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        let mut p = heartbeat_probe.borrow_mut();
        if p.finished {
            return glib::ControlFlow::Break;
        }
        let now = Instant::now();
        let gap = now.duration_since(p.heartbeat_last).as_secs_f64() * 1000.;
        p.heartbeat_last = now;
        p.heartbeat_count += 1;
        p.heartbeat_max_ms = p.heartbeat_max_ms.max(gap);
        if gap > 100. {
            let stage = p.stage;
            let busy = ui.background_busy.get();
            p.large_gaps
                .push(serde_json::json!({"stage": stage, "gap_ms": gap, "pdf_busy": busy}));
        }
        if ui.background_busy.get() {
            p.export_ticks += 1;
            p.export_max_gap_ms = p.export_max_gap_ms.max(gap);
        }
        glib::ControlFlow::Continue
    });
    let weak = Rc::downgrade(ui);
    glib::timeout_add_local(Duration::from_millis(30), move || {
        let Some(ui) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        if probe.borrow().finished {
            return glib::ControlFlow::Break;
        }
        if probe.borrow().began.elapsed() > Duration::from_secs(120) {
            probe.borrow_mut().finish(
                &ui,
                Some("Native probe exceeded 120-second bounded timeout".into()),
            );
            return glib::ControlFlow::Break;
        }
        if Instant::now() < probe.borrow().ready_at {
            return glib::ControlFlow::Continue;
        }
        if let Err(error) = step(&ui, &probe) {
            probe.borrow_mut().finish(&ui, Some(format!("{error:#}")));
            return glib::ControlFlow::Break;
        }
        glib::ControlFlow::Continue
    });
}
