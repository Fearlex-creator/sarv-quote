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

fn named<T: IsA<gtk::Widget> + glib::object::ObjectType + Clone + 'static>(
    root: &impl IsA<gtk::Widget>,
    name: &str,
) -> Result<T> {
    descendants::<T>(root)
        .into_iter()
        .find(|widget| widget.widget_name() == name)
        .with_context(|| format!("native scale widget missing: {name}"))
}

fn active_dialog(ui: &Ui) -> Result<adw::Dialog> {
    ui.active_dialog
        .borrow()
        .clone()
        .context("native scale dialog missing")
}

fn picker_results(ui: &Ui) -> Result<gtk::ListBox> {
    named(&active_dialog(ui)?, "product-picker-search-results")
}

fn picker_search(ui: &Ui) -> Result<gtk::SearchEntry> {
    named(&active_dialog(ui)?, "product-picker-search")
}

fn capture(window: &impl IsA<gtk::Window>, path: &Path) -> Result<()> {
    let started = Instant::now();
    let result = smoke::capture(window, path);
    if std::env::var("SARV_PERFORMANCE_TRACE").as_deref() == Ok("1") {
        eprintln!(
            "SARV_MAIN_THREAD probe_screenshot_{} {:.3}ms",
            path.file_name().unwrap_or_default().to_string_lossy(),
            started.elapsed().as_secs_f64() * 1000.0
        );
    }
    result
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
    inject_key("Tab", false)
}
fn inject_key(key: &str, control: bool) -> Result<std::process::Child> {
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
    let script = script.replace("b'Tab'", &format!("b{key:?}"));
    let script = if control {
        script.replace("t.XTestFakeKeyEvent(d,k,1,0);", "c=x.XKeysymToKeycode(d,x.XStringToKeysym(b'Control_L')); assert c\nt.XTestFakeKeyEvent(d,c,1,0); t.XTestFakeKeyEvent(d,k,1,0);")
            .replace("x.XFlush(d); x.XCloseDisplay(d)", "t.XTestFakeKeyEvent(d,c,0,0); x.XFlush(d); x.XCloseDisplay(d)")
    } else {
        script
    };
    Ok(std::process::Command::new("/usr/bin/python3")
        .args(["-c", &script])
        .spawn()?)
}

// Additive action clocks do not alter the original broad heartbeat sampler.
// Observer fixtures, screenshots and disk verification remain outside scopes.
struct ActionScope {
    id: usize,
    operation: &'static str,
    origin: &'static str,
    began: Instant,
    last: Instant,
    gaps: Vec<f64>,
    before: serde_json::Value,
    frames: Rc<Cell<usize>>,
    frame_guard: Rc<Cell<bool>>,
    first_ready_frame: Option<usize>,
    ready_observed_ms: Option<f64>,
    legacy_elapsed_ms: Option<f64>,
}

fn action_metadata(ui: &Ui) -> serde_json::Value {
    let rows = ui.row_editors.borrow();
    serde_json::json!({
        "generation": ui.store.borrow().local_generation(),
        "edit_revision": ui.edit_revision.get(), "page": ui.table_page.get() + 1,
        "rows": rows.len(), "first_id": rows.first().map(|row| row.index),
        "first_query": rows.first().map(|row| row.query.text().to_string()),
        "first_quantity": rows.first().map(|row| row.quantity.text().to_string()),
        "first_quantity_selection": rows.first().and_then(|row| row.quantity.selection_bounds()),
        "first_query_visible_and_focused": rows.first()
            .is_some_and(|row| entry_visible_and_focused(ui, &row.query)),
        "first_quantity_visible_and_focused": rows.first()
            .is_some_and(|row| entry_visible_and_focused(ui, &row.quantity)),
        "first_amount": rows.first().and_then(|row| named::<gtk::Label>(
            &ui.review, &format!("quote-row-amount-{}", row.index)).ok())
            .map(|label| label.text().to_string()),
        "last_id": rows.last().map(|row| row.index),
        "last_query": rows.last().map(|row| row.query.text().to_string()),
        "total": ui.total.text().to_string(),
        "focus_widget": gtk::prelude::GtkWindowExt::focus(&ui.window)
            .map(|focus| focus.widget_name().to_string()),
        "persistence_busy": ui.persistence_busy.get(),
        "review_pending": ui.review_requested.get().is_some(),
        "visual_pending": ui.visual_pending.get(),
        "active_dialog": ui.active_dialog.borrow().is_some(),
    })
}

fn entry_visible_and_focused(ui: &Ui, entry: &gtk::Entry) -> bool {
    if !entry.is_mapped() || !gtk::prelude::WidgetExt::is_visible(entry) || !entry.is_sensitive() {
        return false;
    }
    let Some(bounds) = entry.compute_bounds(&ui.window) else {
        return false;
    };
    if bounds.width() <= 0. || bounds.height() <= 0. {
        return false;
    }
    let intersects = |other: &gtk::graphene::Rect| {
        bounds.x() < other.x() + other.width()
            && bounds.x() + bounds.width() > other.x()
            && bounds.y() < other.y() + other.height()
            && bounds.y() + bounds.height() > other.y()
    };
    if !intersects(&gtk::graphene::Rect::new(
        0.,
        0.,
        ui.window.width() as f32,
        ui.window.height() as f32,
    )) {
        return false;
    }
    let mut ancestor = entry.parent();
    while let Some(widget) = ancestor {
        if widget.is::<gtk::ScrolledWindow>() {
            let Some(clip) = widget.compute_bounds(&ui.window) else {
                return false;
            };
            if !intersects(&clip) {
                return false;
            }
        }
        ancestor = widget.parent();
    }
    gtk::prelude::GtkWindowExt::focus(&ui.window).is_some_and(|focus| {
        focus == entry.clone().upcast::<gtk::Widget>() || focus.is_ancestor(entry)
    })
}

// The fixture has fixed prices100001..101000 and only first/last quantities
// change. Checking its rendered exact total avoids recomputing1,000 lines in
// a readiness poll. Full economic/on-disk verification remains outside scope.
fn action_review_ready(ui: &Ui, page: usize, quantity_focus: bool) -> Result<bool> {
    if ui.persistence_busy.get()
        || ui.review_requested.get().is_some()
        || ui.visual_pending.get()
        || ui.active_dialog.borrow().is_some()
        || ui.table_page.get() != page
        || ui.stack.visible_child_name().as_deref() != Some("order")
    {
        return Ok(false);
    }
    let store = ui.store.borrow();
    let state = &store.state;
    if state.draft.lines.len() != 1000 {
        return Ok(false);
    }
    let rows = ui.row_editors.borrow();
    let start = page * 100;
    if rows.len() != 100
        || rows.iter().enumerate().any(|(offset, row)| {
            row.index != start + offset
                || row.query.text().as_str() != state.draft.lines[start + offset].query
        })
    {
        return Ok(false);
    }
    let amount = line_total(
        100_001 + start as i64,
        state.draft.lines[start].quantity_milli,
    )?;
    let expected = state.settings.display_digits.format(
        state
            .draft
            .display_unit
            .display(amount)
            .trim_end_matches(state.draft.display_unit.label())
            .trim(),
    );
    if named::<gtk::Label>(&ui.review, &format!("quote-row-amount-{start}"))?.text() != expected {
        return Ok(false);
    }
    let total = 100_500_500 - 100_001 - 101_000
        + line_total(100_001, state.draft.lines[0].quantity_milli)?
        + line_total(101_000, state.draft.lines[999].quantity_milli)?;
    let expected_total = format!(
        "جمع نهایی  {}",
        state
            .settings
            .display_digits
            .format(state.draft.display_unit.display(total))
    );
    if ui.total.text() != expected_total {
        return Ok(false);
    }
    let entry = if quantity_focus {
        &rows[0].quantity
    } else {
        &rows[0].query
    };
    Ok(entry_visible_and_focused(ui, entry))
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
    catalog: Option<Catalog>,
    phase_started: Instant,
    phase_heartbeats: std::collections::BTreeMap<u8, (usize, f64)>,
    maximum_picker_rows: usize,
    maximum_composer_rows: usize,
    composer_list: Option<gtk::ListBox>,
    composer_generation: i64,
    save_lock: Option<rusqlite::Connection>,
    save_generation: i64,
    save_started: Option<Instant>,
    save_ticks: usize,
    save_max_gap_ms: f64,
    pending_editor_page: Option<(usize, i64, Instant, bool)>,
    action_scope: Option<ActionScope>,
    action_records: Vec<serde_json::Value>,
    unchanged_save_generation: Option<i64>,
    additive_pdf_done: bool,
    additive_pdf_error: Option<String>,
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
            catalog: None,
            phase_started: now,
            phase_heartbeats: std::collections::BTreeMap::new(),
            maximum_picker_rows: 0,
            maximum_composer_rows: 0,
            composer_list: None,
            composer_generation: 0,
            save_lock: None,
            save_generation: 0,
            save_started: None,
            save_ticks: 0,
            save_max_gap_ms: 0.,
            pending_editor_page: None,
            action_scope: None,
            action_records: vec![],
            unchanged_save_generation: None,
            additive_pdf_done: false,
            additive_pdf_error: None,
        }
    }
    fn start_action(
        &mut self,
        ui: &Ui,
        operation: &'static str,
        origin: &'static str,
    ) -> Result<()> {
        ensure!(
            self.action_scope.is_none(),
            "overlapping additive action scope"
        );
        let before = action_metadata(ui);
        let frames = Rc::new(Cell::new(0usize));
        let frame_guard = Rc::new(Cell::new(true));
        let tick_frames = frames.clone();
        let tick_guard = frame_guard.clone();
        ui.review.add_tick_callback(move |_, _| {
            if !tick_guard.get() {
                return glib::ControlFlow::Break;
            }
            tick_frames.set(tick_frames.get() + 1);
            glib::ControlFlow::Continue
        });
        // Start marker precedes the clock; subsequent existing main-thread
        // spans can be correlated with this action without hiding app work.
        eprintln!(
            "SARV_ACTION_START {} {} {}",
            self.action_records.len() + 1,
            operation,
            origin
        );
        let now = Instant::now();
        self.action_scope = Some(ActionScope {
            id: self.action_records.len() + 1,
            operation,
            origin,
            began: now,
            last: now,
            gaps: vec![],
            before,
            frames,
            frame_guard,
            first_ready_frame: None,
            ready_observed_ms: None,
            legacy_elapsed_ms: None,
        });
        Ok(())
    }
    fn mark_legacy_completion(&mut self, started: Instant) -> f64 {
        let scope = self.action_scope.as_mut().expect("additive action missing");
        *scope
            .legacy_elapsed_ms
            .get_or_insert_with(|| started.elapsed().as_secs_f64() * 1000.)
    }
    fn action_ready(&mut self, exact_ready: bool) -> bool {
        let Some(scope) = self.action_scope.as_mut() else {
            return false;
        };
        if !exact_ready {
            scope.first_ready_frame = None;
            scope.ready_observed_ms = None;
            return false;
        }
        let frame = scope.frames.get();
        scope
            .ready_observed_ms
            .get_or_insert_with(|| scope.began.elapsed().as_secs_f64() * 1000.);
        let first = *scope.first_ready_frame.get_or_insert(frame);
        frame >= first + 2
    }
    fn close_action(&mut self, ui: &Ui, result: &str) {
        let Some(scope) = self.action_scope.take() else {
            return;
        };
        let now = Instant::now();
        scope.frame_guard.set(false);
        let terminal = now.duration_since(scope.last).as_secs_f64() * 1000.;
        let maximum = scope.gaps.iter().copied().fold(terminal, f64::max);
        let status = if maximum > 50. {
            "miss"
        } else if scope.gaps.is_empty() {
            "unmeasured_zero_ticks"
        } else if result == "accepted" {
            "pass"
        } else {
            "incomplete"
        };
        let record = serde_json::json!({"id":scope.id,"operation":scope.operation,
            "origin":scope.origin,"result":result,"before":scope.before,
            "after":action_metadata(ui),"elapsed_ms":now.duration_since(scope.began).as_secs_f64()*1000.,
            "started_since_probe_ms":scope.began.duration_since(self.began).as_secs_f64()*1000.,
            "ended_since_probe_ms":now.duration_since(self.began).as_secs_f64()*1000.,
            "ready_observed_ms":scope.ready_observed_ms,
            "interval_ms":20,"ticks":scope.gaps.len(),"gaps_ms":scope.gaps,
            "terminal_gap_ms":terminal,"max_gap_ms":maximum,"budget_ms":50,
            "strict_status":status,"ready_frames":scope.frames.get(),
            "legacy_completion_ms":scope.legacy_elapsed_ms,
            "boundary":"before production dispatch through exact adopted generation, all100 current row IDs/queries, first-row amount, literal fixture total, allocated visible sensitive required focus and two subsequent frames; terminal gap included; not every-row amount or painted-pixel proof",
            "fixture_capture_disk_verification_inside_scope":false});
        eprintln!("SARV_ACTION_PHASE {}", record);
        self.action_records.push(record);
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
        self.close_action(ui, error.as_deref().unwrap_or("incomplete_at_finish"));
        self.finished = true;
        let passed = error.is_none();
        let report = serde_json::json!({
            "result": if passed {"passed"} else {"failed"},
            "synthetic_only": true,
            "additive_action_protocol": "m5-action-v1",
            "additive_action_phases": self.action_records,
            "additive_scope_limits": ["New scopes do not replace original broad strict budgets", "Two rendered readiness frames and observer polling are included", "PDF is a second separately declared internal-helper export, not native PDF file-picker action", "Original global includes extra PDF/probe work; changed observer protocol forbids identical-endpoint causal comparison", "Legacy page duration is latched on original busy-release/page/adopted-generation condition before stronger focus/frame wait; disk verification moves after scope closure", "Legacy choice duration is latched at original pending-review/busy-release boundary before stronger exact row/focus wait", "Changed Save recovery uses actual native Save button; original recovery workflow duration now includes stronger final readiness"],
            "products": 50_000, "quote_lines": 1_000,
            "catalog_preview_pages_visited": self.page + 1,
            "maximum_native_preview_rows": self.max_rows,
            "maximum_native_picker_rows": self.maximum_picker_rows,
            "maximum_native_composer_rows": self.maximum_composer_rows,
            "phase_heartbeats": self.phase_heartbeats.iter().map(|(stage, (ticks, max_gap))|
                (stage.to_string(), serde_json::json!({"ticks":ticks,"maximum_gap_ms":max_gap})))
                .collect::<serde_json::Map<String, serde_json::Value>>(),
            "durations_ms": self.durations,
            "heartbeat_interval_ms": 20,
            "heartbeat_count": self.heartbeat_count,
            "maximum_heartbeat_gap_ms_including_setup_and_editor_paging": self.heartbeat_max_ms,
            "heartbeat_ticks_while_pdf_busy": self.export_ticks,
            "maximum_heartbeat_gap_ms_while_pdf_busy": self.export_max_gap_ms,
            "pdf_busy_responsiveness_threshold_ms": 250,
            "lookup_readiness_observation": "30ms polling for exact native contents; no deliberate250ms wait",
            "heartbeat_ticks_while_save_busy": self.save_ticks,
            "maximum_heartbeat_gap_ms_while_save_busy": self.save_max_gap_ms,
            "heartbeat_gaps_over_100_ms": self.large_gaps,
            "editor_page_clicks": self.page_clicks,
            "maximum_unchanged_editor_page_click_ms": self.page_clicks.iter()
                .filter(|click| click["draft_changed"] == false)
                .filter_map(|click| click["duration_ms"].as_f64())
                .fold(0.0, f64::max),
            "final_window_allocation": [ui.window.width(), ui.window.height()],
            "checks": self.checks,
            "failure": error,
            "limitations": ["Synthetic scale fixture, not production customer data", "Explicit and automatic draft persistence validation, serialization and SQLite commit run on one bounded worker; GTK still clones the snapshot and some dependent document commands, catalog acceptance, replacements and row rebuilds remain synchronous. Global and per-phase gaps are reported rather than hidden; the 50ms overall budget is not accepted", "Injected lock is a responsiveness fixture, not hardware power-loss proof", "No Wayland, physical printer or target desktop packaging certification"]
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
    // A worker result is usable only after GTK has installed its exact page.
    // Keep the full action-to-ready duration, including worker/polling time.
    if ui.review_requested.get().is_some() {
        return Ok(());
    }
    let mut p = probe.borrow_mut();
    let directory = ui.store.borrow().directory.clone();
    match p.stage {
        0 => {
            let begin = Instant::now();
            let source = chrono::Utc::now().to_rfc3339();
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
            for index in 1..=50_000 {
                writer.write_record([
                    format!("SCALE-{index:05}"),
                    format!("کالای ساختگی پاسخ‌گویی {index}"),
                    "عدد".into(),
                    (100_000 + index).to_string(),
                    "rial".into(),
                    source.clone(),
                    String::new(),
                ])?;
            }
            let csv = String::from_utf8(writer.into_inner()?)?;
            p.elapsed("synthetic_50000_csv_fixture_creation", begin);
            p.phase_started = Instant::now();
            p.advance(40, 0);
            drop(p);
            let parsed = probe.clone();
            ensure!(
                ui.run_background(
                    "آزمون ساختگی؛ بررسی CSV پنجاه هزار کالا",
                    move || {
                        let started = Instant::now();
                        imports::catalog_csv(&csv, "۵۰ هزار کالای ساختگی؛ آزمون پاسخ‌گویی")
                            .map(|catalog| (catalog, started.elapsed().as_secs_f64() * 1000.))
                    },
                    move |ui, result| {
                        let mut p = parsed.borrow_mut();
                        match result {
                            Ok((catalog, worker_ms)) => {
                                p.catalog = Some(catalog);
                                p.durations.insert(
                                    "50000_csv_worker_parse_ms".into(),
                                    serde_json::json!(worker_ms),
                                );
                                let start = p.phase_started;
                                p.elapsed("50000_csv_parse_request_to_completion", start);
                            }
                            Err(error) => {
                                p.finish(ui, Some(format!("CSV parse failed: {error:#}")))
                            }
                        }
                    }
                ),
                "CSV parse worker rejected"
            );
        }
        40 => {
            let Some(catalog) = p.catalog.take() else {
                return Ok(());
            };
            p.phase_started = Instant::now();
            p.advance(41, 0);
            drop(p);
            let accepted = probe.clone();
            ui.accept_catalog_async(catalog, move |ui, success| {
                let mut p = accepted.borrow_mut();
                let started = p.phase_started;
                p.elapsed("50000_catalog_prepare_commit_and_native_refresh", started);
                if success {
                    p.checks.push("Actual50,000-row CSV parse and prepared catalogue acceptance complete via native background workflows".into());
                    p.advance(42, 300);
                } else {
                    p.finish(ui, Some("Prepared catalogue acceptance failed".into()));
                }
            });
        }
        41 => {}
        42 => {
            let catalog = ui.store.borrow().state.catalog()?.clone();
            ensure!(
                catalog.products.len() == 50_000,
                "validated CSV lost products"
            );
            ensure!(
                persisted_generation(ui)? == 1,
                "catalogue acceptance did not commit exactly once"
            );
            let begin = Instant::now();
            let preview =
                catalog_preview::preview(&catalog, DisplayDigits::Persian, DateDisplay::Iso);
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
            capture(
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
            capture(&ui.window, &directory.join("native-scale-catalog-next.png"))?;
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
            capture(&ui.window, &directory.join("native-scale-catalog-last.png"))?;
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
            p.elapsed("1000_line_store_commit_and_review_dispatch", begin);
            p.advance(60, 1000);
        }
        60 => {
            capture(&ui.window, &directory.join("native-save-before.png"))?;
            let connection = rusqlite::Connection::open(directory.join("sarv.sqlite3"))?;
            connection.execute_batch("BEGIN IMMEDIATE;")?;
            p.save_lock = Some(connection);
            p.save_generation = persisted_generation(ui)?;
            ui.customer.set_text("نسخهٔ نخست ذخیرهٔ ساختگی");
            p.save_started = Some(Instant::now());
            p.heartbeat_last = Instant::now();
            let dispatched = Instant::now();
            ui.save_form_async();
            p.elapsed(
                "50000_catalog_1000_line_save_dispatch_main_thread",
                dispatched,
            );
            ensure!(
                ui.persistence_busy.get(),
                "native save did not dispatch worker"
            );
            ui.save_form_async();
            p.advance(61, 150);
        }
        61 => {
            ensure!(
                ui.persistence_busy.get(),
                "SQLite lock did not hold save worker"
            );
            ensure!(
                ui.store.borrow_mut().change(|_| Ok(())).is_err(),
                "mutation allowed before persistence receipt"
            );
            ui.customer.set_text("نسخهٔ تازه در زمان ذخیرهٔ ساختگی");
            ui.row_editors.borrow()[0].quantity.set_text("نامعتبر");
            ui.stack.set_visible_child_name("settings");
            ensure!(
                ui.stack.visible_child_name().as_deref() == Some("settings"),
                "navigation blocked while save waits for SQLite"
            );
            ui.stack.set_visible_child_name("order");
            p.save_lock
                .take()
                .context("save lock missing")?
                .execute_batch("ROLLBACK;")?;
            p.advance(62, 100);
        }
        62 => {
            if ui.persistence_busy.get() {
                return Ok(());
            }
            ensure!(
                ui.row_editors.borrow()[0].quantity.text() == "نامعتبر",
                "older save receipt replaced invalid newer input"
            );
            ensure!(
                ui.customer.text() == "نسخهٔ تازه در زمان ذخیرهٔ ساختگی",
                "older save receipt replaced newer customer input"
            );
            ensure!(
                persisted_generation(ui)? == p.save_generation + 1,
                "double save duplicated commits or invalid input persisted"
            );
            let focus = gtk::prelude::GtkWindowExt::focus(&ui.window)
                .context("invalid field not focused")?;
            ensure!(
                focus.is_ancestor(&ui.row_editors.borrow()[0].quantity),
                "invalid quantity not focused after asynchronous receipt"
            );
            ui.row_editors.borrow()[0].quantity.set_text("1");
            p.start_action(
                ui,
                "quote_changed_save_recovery",
                "native_save_button_after_lock_released",
            )?;
            button_named(&ui.window, "ذخیرهٔ پیش‌نویس")?.emit_clicked();
            p.advance(63, 100);
        }
        63 => {
            let exact_ready = ui.store.borrow().local_generation() == p.save_generation + 2
                && ui.customer.text() == "نسخهٔ تازه در زمان ذخیرهٔ ساختگی"
                && ui.row_editors.borrow()[0].quantity.text() == "1"
                && action_review_ready(ui, 0, true)?;
            if !p.action_ready(exact_ready) {
                return Ok(());
            }
            p.close_action(ui, "accepted");
            ensure!(
                persisted_generation(ui)? == p.save_generation + 2,
                "latest valid edits not saved exactly once"
            );
            let reopened = Store::open(&directory)?;
            ensure!(
                reopened.state.draft.customer == "نسخهٔ تازه در زمان ذخیرهٔ ساختگی"
                    && reopened.state.draft.lines.len() == 1000
                    && reopened.state.draft.lines[0].quantity_milli == 1000,
                "worker save restart integrity failed"
            );
            ensure!(p.save_ticks >= 3, "save worker heartbeat not observed");
            let started = p.save_started.take().context("save timer missing")?;
            p.elapsed(
                "locked_worker_save_newer_invalid_edit_and_recovery",
                started,
            );
            p.checks.push("Native Save dispatches bounded worker; double save commits once; SQLite lock leaves GTK navigation live; old receipt preserves newer invalid input and focuses quantity; recovery saves latest edits; reopened profile retains all1,000 lines".into());
            // The save completion can rebuild rows in this same main-loop
            // iteration. Wait for GTK allocation before taking native pixels.
            p.advance(64, 200);
        }
        64 => {
            let rows = ui.row_editors.borrow();
            rows[0].quantity.grab_focus();
            rows[0].quantity.select_region(0, 1);
            drop(rows);
            p.unchanged_save_generation = Some(persisted_generation(ui)?);
            p.start_action(ui, "quote_unchanged_save_refresh", "native_save_button")?;
            button_named(&ui.window, "ذخیرهٔ پیش‌نویس")?.emit_clicked();
            p.advance(65, 200);
        }
        65 => {
            let exact_ready = ui.store.borrow().local_generation()
                == p.unchanged_save_generation
                    .context("unchanged save generation missing")?
                && ui.row_editors.borrow()[0].quantity.selection_bounds() == Some((0, 1))
                && action_review_ready(ui, 0, true)?;
            if !p.action_ready(exact_ready) {
                return Ok(());
            }
            p.close_action(ui, "accepted");
            ensure!(
                persisted_generation(ui)? == p.unchanged_save_generation.take().unwrap(),
                "explicit unchanged Save created an extra commit"
            );
            let amount = named::<gtk::Label>(&ui.review, "quote-row-amount-0")?;
            let store = ui.store.borrow();
            let expected = store.state.settings.display_digits.format(
                store
                    .state
                    .draft
                    .display_unit
                    .display(100_001)
                    .trim_end_matches(store.state.draft.display_unit.label())
                    .trim(),
            );
            ensure!(
                amount.text() == expected,
                "explicit Save left the edited row amount stale"
            );
            ensure!(
                !ui.visual_pending.get(),
                "explicit Save did not finish native review"
            );
            let quantity = ui.row_editors.borrow()[0].quantity.clone();
            let focus = gtk::prelude::GtkWindowExt::focus(&ui.window)
                .context("explicit Save lost native focus")?;
            ensure!(
                focus.is_ancestor(&quantity) && quantity.selection_bounds() == Some((0, 1)),
                "explicit Save lost quantity focus or selection"
            );
            drop(store);
            p.checks.push("Actual explicit Save refreshes exact100,001rial first-row amount, clears review pending, preserves native quantity focus and selection, and does not duplicate an unchanged commit".into());
            capture(&ui.window, &directory.join("native-save-after.png"))?;
            p.advance(10, 300);
        }
        10 => {
            if let Some((page, before, started, _)) = p.pending_editor_page {
                if ui.persistence_busy.get() {
                    return Ok(());
                }
                ensure!(
                    ui.table_page.get() == page + 1,
                    "deferred page navigation failed"
                );
                ensure!(
                    ui.store.borrow().local_generation() == before + i64::from(page == 0),
                    "page adopted generation mismatch"
                );
                let legacy_duration = p.mark_legacy_completion(started);
                let exact_ready = action_review_ready(ui, page + 1, false)?;
                if !p.action_ready(exact_ready) {
                    return Ok(());
                }
                p.close_action(ui, "accepted");
                ensure!(
                    ui.table_page.get() == page + 1,
                    "deferred page navigation failed"
                );
                let after = persisted_generation(ui)?;
                ensure!(
                    after == before + i64::from(page == 0),
                    "page save commit mismatch"
                );
                p.page_clicks
                    .push(serde_json::json!({"direction":"next", "from_page":page + 1,
                    "draft_changed":page == 0, "generation_before":before,
                    "generation_after":after, "duration_ms":legacy_duration}));
                p.pending_editor_page = None;
                return Ok(());
            }
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
                p.start_action(
                    ui,
                    if page == 0 {
                        "quote_page_changed_save"
                    } else {
                        "quote_page_unchanged_next"
                    },
                    "native_pager_button",
                )?;
                button_named(&ui.review, "بعدی")?.emit_clicked();
                p.pending_editor_page = Some((page, before, started, true));
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
            capture(
                &ui.window,
                &directory.join("native-scale-editor-last-page.png"),
            )?;
            ui.row_editors
                .borrow()
                .last()
                .context("last reused row missing")?
                .query
                .grab_focus();
            p.keyboard = Some(inject_key("Up", true)?);
            p.advance(111, 100);
        }
        111 => {
            let status = p
                .keyboard
                .as_mut()
                .context("Ctrl+Up subprocess missing")?
                .try_wait()?;
            if status.is_none() {
                return Ok(());
            }
            ensure!(
                status.is_some_and(|status| status.success()),
                "Ctrl+Up injection failed"
            );
            ensure!(
                ui.table_page.get() == 9,
                "reused row key callback used obsolete page index"
            );
            let rows = ui.row_editors.borrow();
            let query = &rows[98].query;
            let focus =
                gtk::prelude::GtkWindowExt::focus(&ui.window).context("Ctrl+Up lost focus")?;
            ensure!(
                focus.is_ancestor(query),
                "reused row Ctrl+Up did not focus global row999"
            );
            ensure!(
                query.selection_bounds() == Some((0, 11)),
                "reused row Ctrl+Up did not retain selected code"
            );
            p.checks.push("Genuine XTest Ctrl+Up on reused final-page row1000 focuses global row999, selects its exact code and keeps page10".into());
            drop(rows);
            named::<gtk::Button>(&ui.review, "quote-row-remove-999")?.emit_clicked();
            p.advance(112, 0);
        }
        112 => {
            ensure!(
                ui.store.borrow().state.draft.lines.len() == 999,
                "reused remove callback did not remove global row1000"
            );
            ensure!(
                ui.store
                    .borrow()
                    .state
                    .draft
                    .lines
                    .last()
                    .is_some_and(|line| line.query == "SCALE-00999"),
                "reused remove callback removed an obsolete page row"
            );
            ui.undo_button.emit_clicked();
            p.advance(113, 0);
        }
        113 => {
            let store = ui.store.borrow();
            ensure!(
                store.state.draft.lines.len() == 1000
                    && store.state.draft.lines[999].query == "SCALE-01000"
                    && store.state.draft.lines[999].quantity_milli == 3000,
                "reused final-page delete/undo changed exact line order or quantity"
            );
            ensure!(
                store.state.preview()?.total_rial == 100_802_501,
                "delete/undo changed economic total"
            );
            p.checks.push("Native reused final-page remove and Undo preserve global row1000 identity/order, quantity3 and exact100,802,501rial total".into());
            p.advance(12, 20);
        }
        12 => {
            if let Some((page, before, started, _)) = p.pending_editor_page {
                if ui.persistence_busy.get() {
                    return Ok(());
                }
                ensure!(
                    ui.table_page.get() + 1 == page,
                    "previous page navigation failed"
                );
                ensure!(
                    ui.store.borrow().local_generation() == before,
                    "unchanged page adopted generation mismatch"
                );
                let legacy_duration = p.mark_legacy_completion(started);
                let exact_ready = action_review_ready(ui, page - 1, false)?;
                if !p.action_ready(exact_ready) {
                    return Ok(());
                }
                p.close_action(ui, "accepted");
                ensure!(
                    ui.table_page.get() + 1 == page,
                    "previous page navigation failed"
                );
                let after = persisted_generation(ui)?;
                ensure!(
                    after == before,
                    "unchanged editor previous page persisted a new generation"
                );
                p.page_clicks.push(
                    serde_json::json!({"direction":"previous", "from_page":page + 1,
                    "draft_changed":false, "generation_before":before,
                    "generation_after":after, "duration_ms":legacy_duration}),
                );
                p.pending_editor_page = None;
                ensure!(
                    ui.row_editors.borrow().len() <= 100,
                    "editor page exceeded 100 rows"
                );
                return Ok(());
            }
            if ui.table_page.get() > 0 {
                let page = ui.table_page.get();
                let before = persisted_generation(ui)?;
                let started = Instant::now();
                p.start_action(ui, "quote_page_unchanged_previous", "native_pager_button")?;
                button_named(&ui.review, "قبلی")?.emit_clicked();
                p.pending_editor_page = Some((page, before, started, false));
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
            p.advance(20, 500);
        }
        20 => {
            p.phase_started = Instant::now();
            ui.open_product_picker();
            let started = p.phase_started;
            p.elapsed("50000_cold_picker_open_invocation", started);
            p.advance(21, 0);
        }
        21 => {
            if ui.persistence_busy.get() {
                return Ok(());
            }
            let list = picker_results(ui)?;
            if list.row_at_index(29).is_none() {
                return Ok(());
            }
            ensure!(
                list.row_at_index(30).is_none(),
                "indexed picker exceeded30 native rows"
            );
            p.maximum_picker_rows = 30;
            let started = p.phase_started;
            p.elapsed("50000_cold_picker_index_and_first_results_ready", started);
            p.advance(22, 200);
        }
        22 => {
            capture(&ui.window, &directory.join("native-scale-picker-cold.png"))?;
            p.phase_started = Instant::now();
            picker_search(ui)?.set_text("SCALE-50000");
            p.advance(23, 0);
        }
        23 => {
            let rows = page_rows(&picker_results(ui)?);
            if !(rows.len() == 1
                && rows[0]
                    .subtitle()
                    .is_some_and(|text| text.contains("SCALE-50000")))
            {
                return Ok(());
            }
            let started = p.phase_started;
            p.elapsed(
                "50000_indexed_exact_code_search_including_debounce",
                started,
            );
            capture(
                &ui.window,
                &directory.join("native-scale-picker-last-product.png"),
            )?;
            p.phase_started = Instant::now();
            picker_search(ui)?.set_text("کالای ساختگی");
            p.advance(24, 0);
        }
        24 => {
            let list = picker_results(ui)?;
            if page_rows(&list).len() != 30 {
                return Ok(());
            }
            ensure!(
                list.row_at_index(30).is_none(),
                "broad indexed picker created unbounded results"
            );
            ensure!(
                descendants::<gtk::Label>(&active_dialog(ui)?)
                    .iter()
                    .any(|label| label.text().contains(&fa(50_000))),
                "picker result count lost full50,000 products"
            );
            let started = p.phase_started;
            p.elapsed("50000_indexed_broad_search_including_debounce", started);
            active_dialog(ui)?.close();
            p.advance(25, 200);
        }
        25 => {
            p.phase_started = Instant::now();
            ui.open_product_picker();
            let started = p.phase_started;
            p.elapsed("50000_warm_cached_picker_open_dispatch", started);
            p.advance(125, 0);
        }
        125 => {
            if ui.persistence_busy.get() {
                return Ok(());
            }
            let list = picker_results(ui)?;
            if page_rows(&list).len() != 30 {
                return Ok(());
            }
            ensure!(
                list.row_at_index(30).is_none(),
                "warm picker cache exceeded30 rows"
            );
            let started = p.phase_started;
            p.elapsed("50000_warm_cached_picker_open_and_native_results", started);
            p.advance(26, 200);
        }
        26 => {
            capture(&ui.window, &directory.join("native-scale-picker-warm.png"))?;
            active_dialog(ui)?.close();
            let started = Instant::now();
            ui.store.borrow_mut().change(|state| {
                state.draft.lines[0].query = "SYNTHETIC-UNMATCHED-REQUEST".into();
                state.draft.lines[0].chosen_code = None;
                Ok(())
            })?;
            ui.fill_form();
            ui.refresh_review();
            p.phase_started = started;
            p.elapsed(
                "unknown_line_fixture_changed_save_and_review_dispatch",
                started,
            );
            ui.row_editors.borrow()[0]
                .quantity
                .set_text("worker-newer-invalid");
            p.advance(126, 0);
        }
        126 => {
            ensure!(
                ui.row_editors.borrow()[0].quantity.text() == "worker-newer-invalid",
                "review worker receipt overwrote newer invalid quantity"
            );
            ui.row_editors.borrow()[0].quantity.set_text("2");
            ui.table_dirty.set(false);
            ui.refresh_review();
            ensure!(
                ui.review_requested.get().is_some(),
                "bounded row construction was not dispatched"
            );
            ui.row_editors.borrow()[0]
                .quantity
                .set_text("chunk-newer-invalid");
            p.advance(127, 0);
        }
        127 => {
            ensure!(
                ui.row_editors.borrow()[0].quantity.text() == "chunk-newer-invalid",
                "bounded row installation overwrote newer invalid quantity"
            );
            ensure!(
                ui.store.borrow().state.draft.lines[0].query == "SYNTHETIC-UNMATCHED-REQUEST"
                    && ui.store.borrow().state.draft.lines[0].quantity_milli == 2000,
                "cancelled review changed canonical query or quantity"
            );
            ui.row_editors.borrow()[0].quantity.set_text("2");
            ui.table_dirty.set(false);
            ui.refresh_review();
            p.advance(128, 0);
        }
        128 => {
            p.checks.push("Review worker and detached20-row construction both preserve newer invalid typed quantity; aborted presentation leaves canonical query/quantity unchanged".into());
            let started = p.phase_started;
            p.elapsed(
                "unknown_line_request_to_ready_including_probe_poll",
                started,
            );
            ensure!(
                descendants::<gtk::DropDown>(&ui.review).is_empty(),
                "unknown50,000-product line created an eager giant dropdown"
            );
            named::<gtk::Button>(&ui.review, "resolve-line-0")?.emit_clicked();
            p.advance(27, 200);
        }
        27 => {
            if ui.persistence_busy.get() {
                return Ok(());
            }
            let list = picker_results(ui)?;
            if page_rows(&list).len() != 30 {
                return Ok(());
            }
            ensure!(
                list.row_at_index(30).is_none(),
                "unknown-line selector exceeded bounded picker"
            );
            p.phase_started = Instant::now();
            picker_search(ui)?.set_text("SCALE-00001");
            p.advance(28, 0);
        }
        28 => {
            let rows = page_rows(&picker_results(ui)?);
            if !(rows.len() == 1
                && rows[0]
                    .subtitle()
                    .is_some_and(|text| text.contains("SCALE-00001")))
            {
                return Ok(());
            }
            let started = p.phase_started;
            p.elapsed(
                "50000_unknown_line_indexed_search_including_debounce",
                started,
            );
            capture(
                &ui.window,
                &directory.join("native-scale-unknown-selector.png"),
            )?;
            let started = Instant::now();
            p.start_action(ui, "quote_product_choice", "native_product_choose_button")?;
            named::<gtk::Button>(&active_dialog(ui)?, "product-picker-search-choose")?
                .emit_clicked();
            p.elapsed("50000_unknown_line_choice_dispatch_main_thread", started);
            p.phase_started = started;
            p.advance(131, 0);
        }
        131 => {
            // Validate the actual committed choice, not merely an emitted click.
            // Keep the original search endpoint and report choice dispatch and
            // durable/native readiness separately without changing budgets.
            if ui.persistence_busy.get() {
                return Ok(());
            }
            let started = p.phase_started;
            let legacy_duration = p.mark_legacy_completion(started);
            p.durations.insert(
                "50000_unknown_line_choice_durable_and_native_review_ready".into(),
                serde_json::json!(legacy_duration),
            );
            let expected_generation = p
                .action_scope
                .as_ref()
                .context("choice action missing")?
                .before["generation"]
                .as_i64()
                .context("choice generation missing")?
                + 1;
            let exact_choice = {
                let store = ui.store.borrow();
                store.local_generation() == expected_generation
                    && store.state.draft.lines[0].query == "SYNTHETIC-UNMATCHED-REQUEST"
                    && store.state.draft.lines[0].chosen_code.as_deref() == Some("SCALE-00001")
            };
            let exact_ready = exact_choice && action_review_ready(ui, 0, false)?;
            if !p.action_ready(exact_ready) {
                return Ok(());
            }
            p.close_action(ui, "accepted");
            // The original callback/readiness duration is latched above;
            // exact post-render/focus completion is the additive scope only.
            {
                let store = ui.store.borrow();
                ensure!(
                    store.state.draft.lines[0].query == "SYNTHETIC-UNMATCHED-REQUEST"
                        && store.state.draft.lines[0].chosen_code.as_deref() == Some("SCALE-00001"),
                    "explicit unknown resolution replaced customer request or failed to persist choice"
                );
                ensure!(
                    store.state.preview()?.total_rial == 100_802_501,
                    "unknown selection changed exact quote amount"
                );
            }
            p.checks.push("Cold/warm50,000-product indexed picker keeps<=30 rows, finds exact last code, reports all broad matches; unknown-line native selector avoids giant dropdown and persists explicit choice while preserving request and exact price".into());
            let started = Instant::now();
            ui.store.borrow_mut().change(|state| {
                state.draft.lines[0].query = "SCALE-00001".into();
                state.draft.lines[0].chosen_code = None;
                Ok(())
            })?;
            ui.fill_form();
            ui.refresh_review();
            p.elapsed(
                "restore_known_line_fixture_changed_save_and_review_dispatch",
                started,
            );
            p.advance(30, 300);
        }
        30 => {
            let catalog = ui.store.borrow().state.catalog()?.clone();
            p.composer_generation = persisted_generation(ui)?;
            let started = Instant::now();
            catalog_editor::open(ui, Some(&catalog));
            p.elapsed("50000_native_composer_open_dispatch", started);
            p.phase_started = started;
            p.advance(130, 0);
        }
        130 => {
            let Some(list) = descendants::<gtk::ListBox>(&active_dialog(ui)?)
                .into_iter()
                .find(|list| list.row_at_index(99).is_some())
            else {
                return Ok(());
            };
            let started = p.phase_started;
            p.elapsed("50000_native_composer_copy_and_initial100_rows", started);
            ensure!(
                list.row_at_index(100).is_none(),
                "composer eagerly rendered more than100 products"
            );
            p.maximum_composer_rows = 100;
            p.composer_list = Some(list);
            p.advance(31, 200);
        }
        31 => {
            let list = p.composer_list.as_ref().context("composer list missing")?;
            ensure!(
                page_rows(list)[0].title().contains("SCALE-00001"),
                "composer first page incorrect"
            );
            capture(
                &ui.window,
                &directory.join("native-scale-composer-first.png"),
            )?;
            button_named(&active_dialog(ui)?, "کالاهای بعدی")?.emit_clicked();
            let rows = page_rows(p.composer_list.as_ref().unwrap());
            ensure!(
                rows.len() == 100 && rows[0].title().contains("SCALE-00101"),
                "composer next page lost bounded products"
            );
            let search = descendants::<gtk::SearchEntry>(&active_dialog(ui)?)
                .into_iter()
                .find(|search| {
                    search.placeholder_text().as_deref() == Some("جست‌وجوی کالاهای فهرست تازه")
                })
                .context("composer search missing")?;
            p.phase_started = Instant::now();
            search.set_text("SCALE-50000");
            p.advance(32, 0);
        }
        32 => {
            let rows = page_rows(p.composer_list.as_ref().context("composer list missing")?);
            if !(rows.len() == 1 && rows[0].title().contains("SCALE-50000")) {
                return Ok(());
            }
            ensure!(
                descendants::<gtk::Label>(&active_dialog(ui)?)
                    .iter()
                    .any(|label| label.text().contains(&fa(50_000))),
                "composer discarded off-page product count"
            );
            let started = p.phase_started;
            p.elapsed(
                "50000_composer_indexed_last_code_search_including_debounce",
                started,
            );
            capture(
                &ui.window,
                &directory.join("native-scale-composer-last-product.png"),
            )?;
            active_dialog(ui)?.close();
            p.composer_list.take();
            ensure!(
                persisted_generation(ui)? == p.composer_generation,
                "read-only composer copy changed persisted source"
            );
            ensure!(
                ui.store.borrow().state.catalogs.len() == 1
                    && ui.store.borrow().state.catalog()?.products.len() == 50_000,
                "composer mutated source catalog before confirmation"
            );
            p.checks.push("Native50,000-product composer copy keeps<=100 rows, next page and exact last-product search reachable, retains full product count and leaves persisted source/generation unchanged when closed".into());
            p.advance(5, 500);
        }
        5 => {
            ensure!(
                ui.row_editors.borrow().len() == 100,
                "100 bounded native editor rows missing"
            );
            capture(&ui.window, &directory.join("native-scale-editor-1000.png"))?;
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
            capture(
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
            capture(
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
            // The historical PDF busy window above is complete and keeps its
            // original screenshots/timer/reset. The additive export is distinct.
            p.advance(140, 0);
        }
        140 => {
            ui.row_editors.borrow()[0].quantity.grab_focus();
            p.start_action(
                ui,
                "quote_pdf_preview_and_export",
                "internal_probe_preview_then_background_pdf",
            )?;
            let quote = ui.store.borrow().state.preview()?;
            let path = directory.join("native-additive-1000-lines.pdf");
            drop(p);
            let completion = probe.clone();
            ensure!(
                ui.run_background(
                    "آزمون افزوده؛ PDF هزار ردیف",
                    move || export::pdf(&quote, &path),
                    move |_, result| {
                        let mut p = completion.borrow_mut();
                        p.additive_pdf_done = true;
                        p.additive_pdf_error = result.err().map(|error| format!("{error:#}"));
                    }
                ),
                "additive PDF worker rejected"
            );
            probe.borrow_mut().advance(141, 0);
        }
        141 => {
            if let Some(error) = &p.additive_pdf_error {
                anyhow::bail!("additive PDF failed: {error}");
            }
            let exact_ready = p.additive_pdf_done
                && !ui.background_busy.get()
                && action_review_ready(ui, 0, true)?;
            if !p.action_ready(exact_ready) {
                return Ok(());
            }
            p.close_action(ui, "accepted");
            ensure!(
                std::fs::metadata(directory.join("native-additive-1000-lines.pdf"))?.len() > 1000,
                "additive PDF absent or empty"
            );
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
        let stage = p.stage;
        let timing = p.phase_heartbeats.entry(stage).or_insert((0, 0.));
        timing.0 += 1;
        timing.1 = timing.1.max(gap);
        if ui.persistence_busy.get() {
            p.save_ticks += 1;
            p.save_max_gap_ms = p.save_max_gap_ms.max(gap);
        }
        let pdf_busy = p.export_started.is_some() && !p.export_done && ui.background_busy.get();
        if gap > 100. {
            p.large_gaps
                .push(serde_json::json!({"stage": stage, "gap_ms": gap,
                    "pdf_busy": pdf_busy, "worker_busy":ui.background_busy.get()}));
        }
        if pdf_busy {
            p.export_ticks += 1;
            p.export_max_gap_ms = p.export_max_gap_ms.max(gap);
        }
        glib::ControlFlow::Continue
    });
    // Separate20ms clock; never writes legacy heartbeat_last/max/busy fields.
    let action_probe = probe.clone();
    glib::timeout_add_local(Duration::from_millis(20), move || {
        let mut p = action_probe.borrow_mut();
        if p.finished {
            return glib::ControlFlow::Break;
        }
        if let Some(scope) = p.action_scope.as_mut() {
            let now = Instant::now();
            scope
                .gaps
                .push(now.duration_since(scope.last).as_secs_f64() * 1000.);
            scope.last = now;
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
            // A readiness poll can see fresh widgets before GTK has drawn
            // their first native frame. Retry this observer-only condition;
            // the global bounded deadline still applies.
            if error.to_string() == "native snapshot empty" {
                return glib::ControlFlow::Continue;
            }
            probe.borrow_mut().finish(&ui, Some(format!("{error:#}")));
            return glib::ControlFlow::Break;
        }
        glib::ControlFlow::Continue
    });
}
