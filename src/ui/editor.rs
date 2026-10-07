//! Native editable quotation table; approved prices remain read-only.
use super::*;

/// Exact immutable presentation data. Canonical state stays in Store; this
/// cache contains only disposable indices and the domain's validated preview.
pub(super) struct PreparedReview {
    generation: i64,
    valid_until: Option<chrono::DateTime<chrono::Utc>>,
    rows: Vec<ResolvedRow>,
    unresolved: Vec<usize>,
    preview: std::result::Result<Quote, String>,
}
struct ResolvedRow {
    count: usize,
    candidates: Vec<usize>,
    selected: Option<usize>,
}
impl PreparedReview {
    fn prepare(generation: i64, state: &State) -> Self {
        let mut rows = Vec::with_capacity(state.draft.lines.len());
        let mut unresolved = Vec::new();
        let now = chrono::Utc::now();
        let mut valid_until = None;
        if let Ok(catalog) = state.catalog() {
            let matcher = catalog.index();
            let positions: std::collections::HashMap<&str, usize> = catalog
                .products
                .iter()
                .enumerate()
                .map(|(i, p)| (p.code.as_str(), i))
                .collect();
            for (index, line) in state.draft.lines.iter().enumerate() {
                let count = matcher.match_count(&line.query);
                let candidates = matcher
                    .matches_limited(&line.query, 30)
                    .iter()
                    .map(|p| positions[p.code.as_str()])
                    .collect::<Vec<_>>();
                let selected = line
                    .chosen_code
                    .as_deref()
                    .and_then(|code| matcher.product_by_code(code))
                    .map(|p| positions[p.code.as_str()])
                    .or_else(|| (count == 1).then(|| candidates[0]));
                if !line
                    .chosen_code
                    .as_deref()
                    .is_some_and(|code| matcher.product_by_code(code).is_some())
                    && count != 1
                {
                    unresolved.push(index);
                }
                if let Some(product) = selected.and_then(|index| catalog.products.get(index))
                    && let Ok(source) = chrono::DateTime::parse_from_rfc3339(&product.source_at)
                {
                    // Cache validity ends precisely when future-source validation
                    // or seven-day stale-price policy can change without a write.
                    for boundary in [
                        source.with_timezone(&chrono::Utc),
                        source.with_timezone(&chrono::Utc) + chrono::Duration::days(7),
                    ] {
                        if boundary > now && valid_until.is_none_or(|prior| boundary < prior) {
                            valid_until = Some(boundary);
                        }
                    }
                }
                rows.push(ResolvedRow {
                    count,
                    candidates,
                    selected,
                });
            }
        }
        Self {
            generation,
            valid_until,
            rows,
            unresolved,
            preview: state.preview().map_err(|error| format!("{error:#}")),
        }
    }
    fn current(&self, generation: i64) -> bool {
        self.generation == generation
            && self
                .valid_until
                .is_none_or(|until| chrono::Utc::now() < until)
    }
}

pub(super) struct RowEditor {
    /// Position in the complete persisted draft, independent of visible page.
    pub index: usize,
    live_index: Rc<Cell<usize>>,
    cells: Option<ResolvedCells>,
    pub query: gtk::Entry,
    pub quantity: gtk::Entry,
    review: Option<gtk::Widget>,
    remove: gtk::Button,
}
struct ResolvedCells {
    number: gtk::Label,
    description: gtk::Label,
    measure: gtk::Label,
    price: gtk::Label,
    amount: gtk::Label,
    status: gtk::Label,
}

fn review_named<T: IsA<gtk::Widget> + glib::object::ObjectType + Clone + 'static>(
    root: &impl IsA<gtk::Widget>,
    name: &str,
) -> Option<T> {
    if root.widget_name() == name {
        return root.as_ref().clone().downcast::<T>().ok();
    }
    let mut child = root.first_child();
    while let Some(widget) = child {
        if let Some(found) = review_named(&widget, name) {
            return Some(found);
        }
        child = widget.next_sibling();
    }
    None
}

fn page_caption(state: &State, start: usize, page: usize, count: usize) -> String {
    let total = state.draft.lines.len();
    if total == 0 {
        "بدون ردیف".into()
    } else {
        format!(
            "ردیف‌های {} تا {} از {} • صفحهٔ {} از {}",
            state.settings.display_digits.format(start + 1),
            state
                .settings
                .display_digits
                .format((start + PAGE_SIZE).min(total)),
            state.settings.display_digits.format(total),
            state.settings.display_digits.format(page + 1),
            state.settings.display_digits.format(count)
        )
    }
}

const PAGE_SIZE: usize = 100;
const COLUMN_WIDTHS: [i32; 9] = [28, 130, 155, 58, 38, 93, 105, 135, 25];
const QUERY_HINT: &str =
    "کد دقیق یا نام مستعار تأییدشده • Enter برای بررسی • Ctrl+↑/↓ جابه‌جایی ردیف";
const QUANTITY_HINT: &str =
    "تعداد مثبت، حداکثر یک میلیون و سه رقم اعشار • Enter برای بررسی • Ctrl+↑/↓ جابه‌جایی ردیف";

#[derive(Clone, Copy)]
enum RowFocus {
    Query(i32, Option<(i32, i32)>),
    Quantity(i32, Option<(i32, i32)>),
    Review,
    Remove,
}

fn contains_focus(widget: &impl IsA<gtk::Widget>, focus: &gtk::Widget) -> bool {
    focus == widget.as_ref() || focus.is_ancestor(widget)
}

/// A rebuilt page acquires focus before GTK has allocated its new rows. The
/// viewport's normal focus handler runs then, without usable bounds. Retry
/// visibility after layout, without grabbing focus again or moving its caret.
fn ensure_focus_visible(widget: &impl IsA<gtk::Widget>) {
    let frames = Cell::new(0);
    widget.as_ref().add_tick_callback(move |widget, _| {
        let Some(window) = widget.root().and_downcast::<gtk::Window>() else {
            return glib::ControlFlow::Break;
        };
        let Some(focus) = gtk::prelude::GtkWindowExt::focus(&window) else {
            return glib::ControlFlow::Break;
        };
        // A later Tab/click owns the scroll position immediately. Obsolete
        // callbacks from a previous target must never pull the user back.
        if !contains_focus(widget, &focus) {
            return glib::ControlFlow::Break;
        }
        frames.set(frames.get() + 1);
        if frames.get() == 1 {
            return glib::ControlFlow::Continue;
        }
        let mut parent = widget.parent();
        while let Some(ancestor) = parent {
            parent = ancestor.parent();
            if let Some(scroller) = ancestor.downcast_ref::<gtk::ScrolledWindow>()
                && let Some(viewport) = scroller.child()
                && let Some(bounds) = widget.compute_bounds(&viewport)
            {
                for (adjustment, start, extent) in [
                    (scroller.vadjustment(), bounds.y(), bounds.height()),
                    (scroller.hadjustment(), bounds.x(), bounds.width()),
                ] {
                    let size = adjustment.page_size();
                    if size <= 0.0 {
                        continue;
                    }
                    let start = f64::from(start);
                    let end = start + f64::from(extent);
                    let delta = if start < 4.0 {
                        start - 4.0
                    } else if end > size - 4.0 {
                        end - size + 4.0
                    } else {
                        0.0
                    };
                    let value = (adjustment.value() + delta).clamp(
                        adjustment.lower(),
                        (adjustment.upper() - size).max(adjustment.lower()),
                    );
                    if (value - adjustment.value()).abs() > 0.5 {
                        adjustment.set_value(value);
                    }
                }
            }
        }
        // Inner viewport changes are allocated on the following frame. A few
        // bounded passes also reveal the table through the outer document.
        if frames.get() < 5 {
            glib::ControlFlow::Continue
        } else {
            glib::ControlFlow::Break
        }
    });
}

impl RowEditor {
    fn focused(&self, focus: &gtk::Widget) -> Option<RowFocus> {
        if contains_focus(&self.query, focus) {
            Some(RowFocus::Query(
                self.query.position(),
                self.query.selection_bounds(),
            ))
        } else if contains_focus(&self.quantity, focus) {
            Some(RowFocus::Quantity(
                self.quantity.position(),
                self.quantity.selection_bounds(),
            ))
        } else if self
            .review
            .as_ref()
            .is_some_and(|w| contains_focus(w, focus))
        {
            Some(RowFocus::Review)
        } else if contains_focus(&self.remove, focus) {
            Some(RowFocus::Remove)
        } else {
            None
        }
    }

    fn restore_focus(&self, target: RowFocus) {
        let (entry, position, selection) = match target {
            RowFocus::Query(position, selection) => (&self.query, position, selection),
            RowFocus::Quantity(position, selection) => (&self.quantity, position, selection),
            RowFocus::Review => {
                if let Some(review) = &self.review {
                    review.grab_focus();
                    ensure_focus_visible(review);
                } else {
                    self.query.grab_focus();
                    ensure_focus_visible(&self.query);
                }
                return;
            }
            RowFocus::Remove => {
                self.remove.grab_focus();
                ensure_focus_visible(&self.remove);
                return;
            }
        };
        entry.grab_focus();
        ensure_focus_visible(entry);
        if let Some((start, end)) = selection {
            entry.select_region(start, end);
        } else {
            entry.set_position(position);
        }
    }
}
fn cell(grid: &gtk::Grid, widget: &impl IsA<gtk::Widget>, col: i32, row: i32, width: i32) {
    widget.set_width_request(width);
    widget.set_valign(gtk::Align::Center);
    grid.attach(widget, col, row, 1, 1);
}
impl Ui {
    pub(super) fn table_has_focus(&self) -> bool {
        gtk::prelude::GtkWindowExt::focus(&self.window).is_some_and(|w| w.is_ancestor(&self.review))
    }
    pub(super) fn refresh_totals(&self) {
        let st = self.store.borrow();
        let s = &st.state;
        let d = &s.draft;
        self.approve_control.add_css_class("suggested-action");
        self.preview_control.remove_css_class("suggested-action");
        accessible_name(
            &self.discount,
            &format!("تخفیف کل • {}", d.display_unit.label()),
        );
        self.document_state
            .set_text(if s.quotes.iter().any(|q| q.series == d.series) {
                "پیش‌نویس / دارای نسخهٔ محفوظ"
            } else {
                "پیش‌نویس"
            });
        if let Ok(c) = s.catalog() {
            self.source_hint.set_text(&format!(
                "{}  ·  {} کالا  ·  ورود {}  ·  هشدار قدمت پس از ۷ روز",
                c.name,
                fa(c.products.len()),
                ltr(&s
                    .settings
                    .date_display
                    .format(&c.imported_at[..10.min(c.imported_at.len())]))
            ));
        } else {
            self.source_hint
                .set_text("برای شروع، فهرست قیمت را در بخش «فهرست قیمت» وارد و تأیید کنید.");
        }
        let cache = self.review_cache.borrow();
        let preview = cache
            .as_ref()
            .filter(|cache| cache.current(st.local_generation()))
            .map(|cache| cache.preview.as_ref());
        let fallback = if preview.is_none() && s.catalog().is_ok_and(|c| c.products.len() <= 1000) {
            Some(s.preview().map_err(|error| error.to_string()))
        } else {
            None
        };
        let Some(preview) = preview.or_else(|| fallback.as_ref().map(Result::as_ref)) else {
            self.tax.set_visible(false);
            self.subtotal.set_text("جمع اقلام — در حال بررسی");
            self.total.set_text("جمع نهایی — در حال بررسی");
            return;
        };
        match preview {
            Ok(q) => {
                let approved = s
                    .quotes
                    .iter()
                    .filter(|saved| saved.series == d.series)
                    .max_by_key(|saved| saved.version)
                    .filter(|saved| quote_same_content(saved, q));
                if let Some(saved) = approved {
                    self.document_state.set_text(&format!(
                        "تأییدشده {} • نسخهٔ {}",
                        saved.number,
                        s.settings.display_digits.format(saved.version)
                    ));
                    self.approve_control.remove_css_class("suggested-action");
                    self.preview_control.add_css_class("suggested-action");
                } else {
                    self.document_state
                        .set_text(&format!("پیش‌نویس {}", q.number));
                }
                self.tax.set_visible(q.tax_basis_points != 0);
                self.tax.set_text(&format!(
                    "افزودهٔ تنظیم‌شده {} • {}",
                    s.settings.display_digits.format(format!(
                        "{}.{:02}٪",
                        q.tax_basis_points / 100,
                        q.tax_basis_points % 100
                    )),
                    s.settings
                        .display_digits
                        .format(q.display_unit.display(q.tax_rial))
                ));
                self.subtotal.set_text(&format!(
                    "جمع اقلام  {}",
                    s.settings.display_digits.format(
                        q.display_unit
                            .display(q.total_rial + q.discount_rial - q.tax_rial)
                    )
                ));
                self.total.set_text(&format!(
                    "جمع نهایی  {}",
                    s.settings
                        .display_digits
                        .format(q.display_unit.display(q.total_rial))
                ));
            }
            Err(_) => {
                self.tax.set_visible(false);
                self.subtotal.set_text("جمع اقلام —");
                self.total.set_text("جمع نهایی — نیاز به بررسی");
            }
        }
    }
    pub(super) fn refresh_review(self: &Rc<Self>) {
        let generation = self.store.borrow().local_generation();
        // A worker or detached GTK page owns this one bounded request. A
        // newer generation is prepared after its obsolete result returns.
        if self.review_requested.get().is_some() {
            return;
        }
        let cached = self
            .review_cache
            .borrow()
            .as_ref()
            .filter(|cache| cache.current(generation))
            .cloned();
        if let Some(cache) = cached {
            self.render_review(&cache);
            return;
        }
        // Avoid a catalogue index and full quote calculation in GTK callbacks.
        // The snapshot clone is bounded by the supported catalogue fixture;
        // validation and two index builds happen solely on this worker.
        let snapshot_started = std::time::Instant::now();
        let state = self.store.borrow().state.clone();
        performance_trace("review_snapshot_clone", snapshot_started);
        if state.catalog().is_ok_and(|c| c.products.len() > 1000) {
            self.review_requested.set(Some(generation));
            let serial = self.review_request_serial.get().wrapping_add(1);
            self.review_request_serial.set(serial);
            let revision = self.edit_revision.get();
            let (sender, receiver) = std::sync::mpsc::sync_channel(1);
            let spawned = std::thread::Builder::new()
                .name("sarv-review".into())
                .spawn(move || {
                    let prepared = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        PreparedReview::prepare(generation, &state)
                    }))
                    .map_err(|_| "بررسی اقلام قطع شد؛ دوباره تلاش کنید".to_string());
                    let _ = sender.send(prepared);
                });
            if let Err(error) = spawned {
                self.review_requested.set(None);
                self.message(Err::<String, _>(error.into()));
                return;
            }
            self.refresh_totals();
            let weak = Rc::downgrade(self);
            glib::timeout_add_local(std::time::Duration::from_millis(10), move || {
                let Some(ui) = weak.upgrade() else {
                    return glib::ControlFlow::Break;
                };
                if ui.review_request_serial.get() != serial {
                    return glib::ControlFlow::Break;
                }
                let prepared = match receiver.try_recv() {
                    Ok(prepared) => prepared,
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        return glib::ControlFlow::Continue;
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        Err("بررسی اقلام قطع شد؛ دوباره تلاش کنید".into())
                    }
                };
                ui.review_requested.set(None);
                match prepared {
                    Ok(prepared) if ui.store.borrow().local_generation() == generation => {
                        let prepared = Rc::new(prepared);
                        *ui.review_cache.borrow_mut() = Some(prepared.clone());
                        // Input arriving after dispatch owns the form. A late
                        // presentation receipt must never rebuild those rows.
                        if ui.edit_revision.get() == revision {
                            ui.render_review(&prepared);
                        }
                    }
                    Ok(_) => ui.refresh_review(),
                    Err(error) => ui.message(Err::<String, _>(anyhow::anyhow!(error))),
                }
                glib::ControlFlow::Break
            });
        } else {
            let prepared = Rc::new(PreparedReview::prepare(generation, &state));
            *self.review_cache.borrow_mut() = Some(prepared.clone());
            self.render_review(&prepared);
        }
    }
    fn render_review(self: &Rc<Self>, prepared: &PreparedReview) {
        let (rows, has_catalog) = {
            let store = self.store.borrow();
            (store.state.draft.lines.len(), store.state.catalog().is_ok())
        };
        let start = self
            .table_page
            .get()
            .min(rows.max(1).div_ceil(PAGE_SIZE) - 1)
            * PAGE_SIZE;
        let render_started = std::time::Instant::now();
        // Rebuilding after Enter, Save or source review must not send keyboard
        // users back to the window toolbar or lose their caret selection.
        let focused = gtk::prelude::GtkWindowExt::focus(&self.window).and_then(|focus| {
            self.row_editors
                .borrow()
                .iter()
                .find_map(|row| row.focused(&focus).map(|target| (row.index, target)))
        });
        if self.reuse_resolved_review(prepared) {
            self.visual_pending.set(false);
            self.refresh_totals();
            if let Some((index, target)) = focused {
                let rows = self.row_editors.borrow();
                if let Some(row) = rows
                    .iter()
                    .find(|row| row.index == index)
                    .or_else(|| rows.first())
                {
                    row.restore_focus(target);
                }
            }
            performance_trace("review_100_row_widget_reuse", render_started);
            if rows > 20 {
                self.review_frames_ready(prepared.generation, &self.review);
            }
            return;
        }
        if has_catalog && rows.saturating_sub(start).min(PAGE_SIZE) > 20 {
            if let Some(cache) = self.review_cache.borrow().as_ref().cloned() {
                self.build_review_in_batches(cache);
            }
            return;
        }
        // Release the old focused GtkText before unparenting its entry. GTK's
        // viewport would otherwise scroll an unallocated, detached widget.
        if self.table_has_focus() {
            gtk::prelude::GtkWindowExt::set_focus(&self.window, None::<&gtk::Widget>);
        }
        self.visual_pending.set(false);
        clear(&self.review);
        self.row_editors.borrow_mut().clear();
        self.refresh_totals();
        let st = self.store.borrow();
        let s = &st.state;
        // Short quotes should read like a compact document rather than reserve
        // a mostly empty fixed-height table. Natural GTK measurement can still
        // grow for wrapped descriptions and the unresolved-review message.
        if let Some(scroller) = self
            .review
            .ancestor(gtk::ScrolledWindow::static_type())
            .and_downcast::<gtk::ScrolledWindow>()
        {
            let rows = s.draft.lines.len();
            let minimum = if rows <= 4 {
                52 + 40 * rows.max(1) as i32
            } else {
                240
            };
            scroller.set_min_content_height(minimum);
        }
        let Ok(_c) = s.catalog() else {
            self.review.append(&empty_state(
                "فهرست قیمت لازم است",
                "فایل CSV را پیش‌نمایش و منبع آن را تأیید کنید؛ سپس سفارش مشتری را وارد کنید.",
            ));
            return;
        };
        let (content, grid, start, end) = self.review_structure(s, prepared);
        let mut editors = Vec::with_capacity(end - start);
        self.append_review_rows(&grid, s, prepared, start, start..end, &mut editors);
        self.row_editors.borrow_mut().extend(editors);
        content.append(&grid);
        self.review.append(&content);
        performance_trace("review_100_row_widget_construction", render_started);
        let settings_started = std::time::Instant::now();
        settings::apply(self);
        performance_trace("review_settings_apply", settings_started);
        if let Some((index, target)) = focused {
            let rows = self.row_editors.borrow();
            if let Some(row) = rows
                .iter()
                .find(|row| row.index == index)
                .or_else(|| rows.first())
            {
                row.restore_focus(target);
            } else {
                self.new_item.grab_focus();
                ensure_focus_visible(&self.new_item);
            }
        }
        if s.draft.lines.is_empty() {
            self.review.append(&empty_state(
                "اقلام سفارش را اضافه کنید",
                "کد کالا و تعداد را پایین جدول بنویسید یا فایل سفارش CSV وارد کنید.",
            ));
        }
    }
    fn review_structure(
        self: &Rc<Self>,
        s: &State,
        prepared: &PreparedReview,
    ) -> (gtk::Box, gtk::Grid, usize, usize) {
        let content = column();
        let total_rows = s.draft.lines.len();
        let page_count = total_rows.max(1).div_ceil(PAGE_SIZE);
        let page = self.table_page.get().min(page_count - 1);
        self.table_page.set(page);
        let start = page * PAGE_SIZE;
        // Bound widget creation independently of the complete quote length.
        // Every total and export still uses the full canonical draft.
        let page_bar = row();
        page_bar.set_widget_name("quote-page-bar");
        page_bar.add_css_class("toolbar");
        page_bar.set_visible(page_count > 1);
        let count = label(&page_caption(s, start, page, page_count));
        count.set_widget_name("quote-page-count");
        count.add_css_class("dimmed");
        count.add_css_class("caption");
        count.set_hexpand(true);
        page_bar.append(&count);
        for (caption, _target, enabled, name) in [
            (
                "قبلی",
                page.saturating_sub(1),
                page > 0,
                "quote-page-previous",
            ),
            ("بعدی", page + 1, page + 1 < page_count, "quote-page-next"),
        ] {
            let control = button(caption);
            control.set_widget_name(name);
            accessible_name(&control, &format!("صفحهٔ {caption} اقلام پیشنهاد"));
            control.set_sensitive(enabled);
            let weak = Rc::downgrade(self);
            control.connect_clicked(move |_| {
                if let Some(ui) = weak.upgrade() {
                    let target = if name == "quote-page-next" {
                        ui.table_page.get().saturating_add(1)
                    } else {
                        ui.table_page.get().saturating_sub(1)
                    };
                    ui.save_form_async_then(move |ui| {
                        ui.table_page.set(target);
                        ui.refresh_review();
                        if let Some(row) = ui.row_editors.borrow().first() {
                            row.query.grab_focus();
                            ensure_focus_visible(&row.query);
                        }
                    });
                }
            });
            page_bar.append(&control);
        }
        content.append(&page_bar);
        let unresolved = &prepared.unresolved;
        if let Some(first) = unresolved.first().copied() {
            let queue = row();
            let message = status_label(
                &format!(
                    "{} ردیف نیاز به انتخاب کالا دارد؛ تأیید نسخه تا رفع آن‌ها ممکن نیست",
                    s.settings.display_digits.format(unresolved.len())
                ),
                StatusTone::Warning,
            );
            message.set_hexpand(true);
            queue.append(&message);
            let review = button("بررسی اولین ردیف");
            review.set_widget_name("review-first-unresolved");
            let weak = Rc::downgrade(self);
            review.connect_clicked(move |_| {
                if let Some(ui) = weak.upgrade() {
                    if let Err(error) = ui.save_form() {
                        ui.message(Err::<String, _>(error));
                        return;
                    }
                    ui.table_page.set(first / PAGE_SIZE);
                    ui.refresh_review();
                    if let Some(row) = ui
                        .row_editors
                        .borrow()
                        .iter()
                        .find(|row| row.index == first)
                    {
                        row.restore_focus(RowFocus::Review);
                    }
                }
            });
            queue.append(&review);
            content.append(&queue);
        }
        let grid = gtk::Grid::new();
        grid.set_widget_name("quote-grid");
        grid.set_column_spacing(8);
        grid.set_row_spacing(1);
        grid.set_hexpand(true);
        let headers = [
            "ردیف".to_string(),
            "کد / نام مستعار".into(),
            "شرح کالا".into(),
            "تعداد".into(),
            "واحد".into(),
            format!("قیمت واحد\n{}", s.draft.display_unit.label()),
            format!("مبلغ ردیف\n{}", s.draft.display_unit.label()),
            "بررسی".into(),
            "".into(),
        ];
        let widths = COLUMN_WIDTHS;
        for (i, h) in headers.iter().enumerate() {
            let l = table_cell(h, i == 2);
            l.set_widget_name(&format!("quote-header-{i}"));
            l.add_css_class("table-header");
            l.set_wrap(false);
            cell(&grid, &l, i as i32, 0, widths[i]);
        }
        let end = (start + PAGE_SIZE).min(total_rows);
        (content, grid, start, end)
    }
    fn append_review_rows(
        self: &Rc<Self>,
        grid: &gtk::Grid,
        s: &State,
        prepared: &PreparedReview,
        start: usize,
        chunk: std::ops::Range<usize>,
        editors: &mut Vec<RowEditor>,
    ) {
        let c = s.catalog().expect("review was prepared for a catalogue");
        let widths = COLUMN_WIDTHS;
        for (index, line) in s
            .draft
            .lines
            .iter()
            .enumerate()
            .skip(chunk.start)
            .take(chunk.end - chunk.start)
        {
            let r = (index - start) as i32 + 1;
            let resolved = &prepared.rows[index];
            let match_count = resolved.count;
            let matches = resolved
                .candidates
                .iter()
                .map(|index| &c.products[*index])
                .collect::<Vec<_>>();
            let selected = resolved.selected.map(|index| &c.products[index]);
            let live_index = Rc::new(Cell::new(index));
            let number_cell = numeric_cell(&s.settings.display_digits.format(index + 1));
            cell(grid, &number_cell, 0, r, widths[0]);
            let query = entry("کد کالا");
            accessible_name(&query, &format!("کد یا نام مستعار ردیف {}", fa(index + 1)));
            query.set_text(&line.query);
            query.set_width_chars(9);
            query.set_max_width_chars(16);
            query.add_css_class("cell-editor");
            entry_validity(&query, None, QUERY_HINT);
            cell(grid, &query, 1, r, widths[1]);
            let description = table_cell(
                selected
                    .map(|p| p.name.as_str())
                    .unwrap_or("انتخاب کالا لازم است"),
                true,
            );
            description.set_wrap(true);
            description.set_wrap_mode(gtk::pango::WrapMode::WordChar);
            description.set_lines(2);
            description.set_ellipsize(gtk::pango::EllipsizeMode::End);
            description.set_max_width_chars(22);
            if let Some(p) = selected {
                description.set_tooltip_text(Some(&format!(
                    "{} • {}\nمنبع: {} • واحد منبع: {}",
                    p.code,
                    p.name,
                    p.source_at,
                    p.source_unit.label()
                )));
            }
            cell(grid, &description, 2, r, widths[2]);
            let qty = entry("تعداد");
            accessible_name(&qty, &format!("تعداد ردیف {}", fa(index + 1)));
            qty.set_text(
                &s.settings
                    .display_digits
                    .format(qty_display(line.quantity_milli)),
            );
            qty.set_width_chars(4);
            qty.set_max_width_chars(7);
            qty.set_hexpand(false);
            qty.set_direction(gtk::TextDirection::Ltr);
            qty.set_input_purpose(gtk::InputPurpose::Number);
            gtk::prelude::EntryExt::set_alignment(&qty, 1.0);
            entry_validity(&qty, None, QUANTITY_HINT);
            qty.add_css_class("cell-editor");
            cell(grid, &qty, 3, r, widths[3]);
            let measure_cell =
                table_cell(selected.map(|p| p.measure.as_str()).unwrap_or("—"), false);
            cell(grid, &measure_cell, 4, r, widths[4]);
            let price = selected
                .map(|p| {
                    s.settings
                        .display_digits
                        .format(dnumber(s.draft.display_unit, p.price_rial))
                })
                .unwrap_or_else(|| "—".into());
            let amount = selected
                .and_then(|p| line_total(p.price_rial, line.quantity_milli).ok())
                .map(|n| {
                    s.settings
                        .display_digits
                        .format(dnumber(s.draft.display_unit, n))
                })
                .unwrap_or_else(|| "—".into());
            let price_cell = numeric_cell(&price);
            let amount_cell = numeric_cell(&amount);
            amount_cell.set_widget_name(&format!("quote-row-amount-{index}"));
            cell(grid, &price_cell, 5, r, widths[5]);
            cell(grid, &amount_cell, 6, r, widths[6]);
            let mut review_choice = None;
            let mut status_cell = None;
            if let Some(p) = selected {
                let status = status_label(
                    if stale(&p.source_at) {
                        "قیمت قدیمی"
                    } else if line.chosen_code.is_some() {
                        "انتخاب انسانی"
                    } else {
                        "تطبیق دقیق"
                    },
                    if stale(&p.source_at) {
                        StatusTone::Warning
                    } else {
                        StatusTone::Success
                    },
                );
                status.set_wrap(false);
                status.set_tooltip_text(Some(&format!(
                    "قیمت {} • منبع {}",
                    s.draft.display_unit.display(p.price_rial),
                    p.source_at
                )));
                cell(grid, &status, 7, r, widths[7]);
                status_cell = Some(status);
            } else if match_count > 30 || (match_count == 0 && c.products.len() > 30) {
                let select = button(if match_count == 0 {
                    "یافت نشد؛ انتخاب…"
                } else {
                    "مبهم؛ جست‌وجو…"
                });
                select.set_widget_name(&format!("resolve-line-{index}"));
                accessible_name(
                    &select,
                    &format!("جست‌وجو و انتخاب انسانی کالای ردیف {}", fa(index + 1)),
                );
                select.set_tooltip_text(Some(
                    "درخواست مشتری حفظ می‌شود؛ از فهرست قیمت کالای درست را صریحاً انتخاب کنید",
                ));
                review_choice = Some(select.clone().upcast());
                cell(grid, &select, 7, r, widths[7]);
                let original_query = line.query.clone();
                let weak = Rc::downgrade(self);
                select.connect_clicked(move |_| {
                    if let Some(ui) = weak.upgrade() {
                        ui.open_line_product_picker(index, original_query.clone());
                    }
                });
            } else {
                let candidates = if matches.is_empty() {
                    c.products.iter().collect::<Vec<_>>()
                } else {
                    matches
                };
                let mut names = vec![if candidates.len() == c.products.len() {
                    "بدون تطبیق…".to_string()
                } else {
                    "مبهم؛ انتخاب…".to_string()
                }];
                names.extend(candidates.iter().map(|p| {
                    format!(
                        "{} • {} • {}",
                        p.code,
                        p.name,
                        s.draft.display_unit.display(p.price_rial)
                    )
                }));
                let codes = candidates
                    .iter()
                    .map(|p| p.code.clone())
                    .collect::<Vec<_>>();
                let refs = names.iter().map(String::as_str).collect::<Vec<_>>();
                let select = gtk::DropDown::from_strings(&refs);
                accessible_name(
                    &select,
                    &format!("انتخاب انسانی کالای ردیف {}", fa(index + 1)),
                );
                select.set_enable_search(true);
                select.add_css_class("review-choice");
                let factory = gtk::SignalListItemFactory::new();
                factory.connect_setup(|_, item| {
                    if let Some(item) = item.downcast_ref::<gtk::ListItem>() {
                        let l = gtk::Label::new(None);
                        l.set_ellipsize(gtk::pango::EllipsizeMode::End);
                        l.set_max_width_chars(16);
                        item.set_child(Some(&l));
                    }
                });
                factory.connect_bind(|_, item| {
                    if let Some(item) = item.downcast_ref::<gtk::ListItem>()
                        && let Some(obj) = item.item().and_downcast::<gtk::StringObject>()
                        && let Some(l) = item.child().and_downcast::<gtk::Label>()
                    {
                        l.set_text(&obj.string());
                    }
                });
                select.set_factory(Some(&factory));
                review_choice = Some(select.clone().upcast());
                cell(grid, &select, 7, r, widths[7]);
                let original_query = line.query.clone();
                let weak = Rc::downgrade(self);
                select.connect_selected_notify(move |w| {
                    let n = w.selected();
                    if n > 0
                        && n != gtk::INVALID_LIST_POSITION
                        && let Some(ui) = weak.upgrade()
                    {
                        let code = codes[(n - 1) as usize].clone();
                        let result = ui.save_form().and_then(|()| {
                            ui.store.borrow_mut().change(|s| {
                                let row = s.draft.lines.get_mut(index).ok_or_else(|| {
                                    anyhow::anyhow!("ردیف تغییر کرده؛ دوباره بررسی کنید")
                                })?;
                                anyhow::ensure!(
                                    row.query == original_query,
                                    "سفارش تغییر کرده؛ دوباره بررسی کنید"
                                );
                                row.chosen_code = Some(code);
                                s.draft.stale_ack = false;
                                Ok(())
                            })
                        });
                        let success = result.is_ok();
                        ui.message(
                            result.map(|()| "انتخاب انسانی ذخیره شد؛ منبع قیمت را بررسی کنید"),
                        );
                        if success {
                            ui.loading.set(true);
                            ui.stale.set_active(false);
                            ui.loading.set(false);
                            ui.refresh_review();
                        }
                    }
                });
            }
            let remove = button("×");
            remove.set_widget_name(&format!("quote-row-remove-{index}"));
            accessible_name(&remove, &format!("حذف ردیف {}", fa(index + 1)));
            remove.set_tooltip_text(Some("حذف ردیف • با امکان برگرداندن"));
            remove.add_css_class("row-remove");
            cell(grid, &remove, 8, r, widths[8]);
            let weak = Rc::downgrade(self);
            let remove_index = live_index.clone();
            remove.connect_clicked(move |_| {
                if let Some(ui) = weak.upgrade() {
                    ui.delete_line(remove_index.get());
                }
            });
            let weak = Rc::downgrade(self);
            let changing_description = description.clone();
            let changing_price = price_cell.clone();
            let amount_pending = amount_cell.clone();
            query.connect_changed(move |entry| {
                if weak.upgrade().is_some_and(|ui| ui.loading.get()) {
                    return;
                }
                let error = text(&entry.text()).err().map(|error| error.to_string());
                entry_validity(entry, error.as_deref(), QUERY_HINT);
                if let Some(ui) = weak.upgrade()
                    && !ui.loading.get()
                {
                    ui.visual_pending.set(true);
                    ui.stale.set_active(false);
                    changing_description.set_text("نیاز به بررسی کد جدید");
                    changing_price.set_text("—");
                    amount_pending.set_text("—");
                }
            });
            let changing_amount = amount_cell.clone();
            let quantity_loading = Rc::downgrade(self);
            qty.connect_changed(move |entry| {
                if quantity_loading
                    .upgrade()
                    .is_some_and(|ui| ui.loading.get())
                {
                    return;
                }
                let error = quantity(&entry.text()).err().map(|error| error.to_string());
                entry_validity(entry, error.as_deref(), QUANTITY_HINT);
                changing_amount.set_text("—");
            });
            for (is_quantity, e) in [(false, &query), (true, &qty)] {
                let weak = Rc::downgrade(self);
                e.connect_changed(move |_| {
                    if let Some(ui) = weak.upgrade()
                        && !ui.loading.get()
                    {
                        ui.table_dirty.set(true);
                        ui.visual_pending.set(true);
                        ui.subtotal.set_text("جمع اقلام — در حال ویرایش");
                        ui.total.set_text("جمع نهایی — در حال ویرایش");
                        ui.tax.set_visible(false);
                        ui.schedule();
                    }
                });
                let weak = Rc::downgrade(self);
                e.connect_activate(move |_| {
                    if let Some(ui) = weak.upgrade() {
                        ui.commit_table();
                    }
                });
                let keys = gtk::EventControllerKey::new();
                keys.set_propagation_phase(gtk::PropagationPhase::Capture);
                let weak = Rc::downgrade(self);
                let key_index = live_index.clone();
                keys.connect_key_pressed(move |_, key, _, modifiers| {
                    let index = key_index.get();
                    if !modifiers.contains(gtk::gdk::ModifierType::CONTROL_MASK) {
                        return glib::Propagation::Proceed;
                    }
                    let target = match key {
                        gtk::gdk::Key::Up => index.checked_sub(1),
                        gtk::gdk::Key::Down => index.checked_add(1),
                        _ => return glib::Propagation::Proceed,
                    };
                    if let Some(ui) = weak.upgrade() {
                        let total = ui.store.borrow().state.draft.lines.len();
                        if let Some(target) = target.filter(|target| *target < total) {
                            if target / PAGE_SIZE != ui.table_page.get() {
                                if let Err(error) = ui.save_form() {
                                    ui.message(Err::<String, _>(error));
                                    return glib::Propagation::Stop;
                                }
                                ui.table_page.set(target / PAGE_SIZE);
                                ui.refresh_review();
                            }
                            if let Some(row) = ui
                                .row_editors
                                .borrow()
                                .iter()
                                .find(|row| row.index == target)
                            {
                                let entry = if is_quantity {
                                    &row.quantity
                                } else {
                                    &row.query
                                };
                                entry.grab_focus();
                                entry.select_region(0, -1);
                                ensure_focus_visible(entry);
                            }
                        } else if key == gtk::gdk::Key::Down {
                            if is_quantity {
                                ui.new_quantity.grab_focus();
                                ensure_focus_visible(&ui.new_quantity);
                            } else {
                                ui.new_item.grab_focus();
                                ensure_focus_visible(&ui.new_item);
                            }
                        }
                    }
                    glib::Propagation::Stop
                });
                e.add_controller(keys);
            }
            editors.push(RowEditor {
                index,
                live_index,
                cells: status_cell.map(|status| ResolvedCells {
                    number: number_cell,
                    description,
                    measure: measure_cell,
                    price: price_cell,
                    amount: amount_cell,
                    status,
                }),
                query,
                quantity: qty,
                review: review_choice,
                remove,
            });
        }
    }
    fn build_review_in_batches(self: &Rc<Self>, prepared: Rc<PreparedReview>) {
        let generation = prepared.generation;
        let build_serial = self.review_request_serial.get().wrapping_add(1);
        self.review_request_serial.set(build_serial);
        let revision = self.edit_revision.get();
        let snapshot_started = std::time::Instant::now();
        let state = self.store.borrow().state.clone();
        performance_trace("review_batched_snapshot_clone", snapshot_started);
        let (content, grid, start, end) = self.review_structure(&state, &prepared);
        content.append(&grid);
        let mut editors = Vec::with_capacity(end - start);
        let mut next = start;
        let requested_page = self.table_page.get();
        self.review_requested.set(Some(generation));
        // The existing entries remain editable until the complete new page
        // can be installed atomically. Only their obsolete pager is disabled.
        for name in ["quote-page-previous", "quote-page-next"] {
            if let Some(control) = review_named::<gtk::Button>(&self.review, name) {
                control.set_sensitive(false);
            }
        }
        self.message(Ok("در حال آماده‌سازی ردیف‌ها…"));
        let weak = Rc::downgrade(self);
        glib::idle_add_local(move || {
            let Some(ui) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            if ui.store.borrow().local_generation() != generation
                || ui.edit_revision.get() != revision
                || ui.table_page.get() != requested_page
            {
                ui.review_requested.set(None);
                if let Some(first) = ui.row_editors.borrow().first() {
                    ui.table_page.set(first.index / PAGE_SIZE);
                }
                let count = ui
                    .store
                    .borrow()
                    .state
                    .draft
                    .lines
                    .len()
                    .max(1)
                    .div_ceil(PAGE_SIZE);
                for (name, enabled) in [
                    ("quote-page-previous", ui.table_page.get() > 0),
                    ("quote-page-next", ui.table_page.get() + 1 < count),
                ] {
                    if let Some(control) = review_named::<gtk::Button>(&ui.review, name) {
                        control.set_sensitive(enabled);
                    }
                }
                retire_review(vec![content.clone().upcast()], std::mem::take(&mut editors));
                if ui.edit_revision.get() == revision {
                    ui.refresh_review();
                }
                return glib::ControlFlow::Break;
            }
            let batch_started = std::time::Instant::now();
            let limit = (next + 20).min(end);
            ui.append_review_rows(&grid, &state, &prepared, start, next..limit, &mut editors);
            next = limit;
            performance_trace("review_construct_20_rows", batch_started);
            if next < end {
                return glib::ControlFlow::Continue;
            }
            let install_started = std::time::Instant::now();
            let focused = gtk::prelude::GtkWindowExt::focus(&ui.window).and_then(|focus| {
                ui.row_editors
                    .borrow()
                    .iter()
                    .find_map(|row| row.focused(&focus).map(|target| (row.index, target)))
            });
            if ui.table_has_focus() {
                gtk::prelude::GtkWindowExt::set_focus(&ui.window, None::<&gtk::Widget>);
            }
            let mut old = Vec::new();
            while let Some(child) = ui.review.first_child() {
                old.push(child.clone());
                ui.review.remove(&child);
            }
            let old_rows = ui.row_editors.replace(std::mem::take(&mut editors));
            ui.review.append(&content);
            if let Some(scroller) = ui
                .review
                .ancestor(gtk::ScrolledWindow::static_type())
                .and_downcast::<gtk::ScrolledWindow>()
            {
                scroller.set_min_content_height(240);
            }
            ui.visual_pending.set(false);
            ui.refresh_totals();
            settings::apply(&ui);
            if let Some((index, target)) = focused {
                let rows = ui.row_editors.borrow();
                if let Some(row) = rows
                    .iter()
                    .find(|row| row.index == index)
                    .or_else(|| rows.first())
                {
                    row.restore_focus(target);
                }
            }
            let frames = Cell::new(0usize);
            let weak_ready = Rc::downgrade(&ui);
            content.add_tick_callback(move |_, _| {
                frames.set(frames.get() + 1);
                if frames.get() < 2 {
                    return glib::ControlFlow::Continue;
                }
                if let Some(ui) = weak_ready.upgrade()
                    && ui.review_request_serial.get() == build_serial
                {
                    ui.review_requested.set(None);
                }
                glib::ControlFlow::Break
            });
            let weak_hidden = Rc::downgrade(&ui);
            glib::timeout_add_local(std::time::Duration::from_millis(20), move || {
                let Some(ui) = weak_hidden.upgrade() else {
                    return glib::ControlFlow::Break;
                };
                if ui.review_request_serial.get() != build_serial
                    || ui.review_requested.get().is_none()
                {
                    return glib::ControlFlow::Break;
                }
                // A hidden editor has no frame clock. Navigation elsewhere
                // must not leave a completed presentation request stuck.
                if !ui.window.is_visible()
                    || ui.stack.visible_child_name().as_deref() != Some("order")
                {
                    ui.review_requested.set(None);
                    return glib::ControlFlow::Break;
                }
                glib::ControlFlow::Continue
            });
            retire_review(old, old_rows);
            performance_trace("review_atomic_install", install_started);
            glib::ControlFlow::Break
        });
    }
    fn review_frames_ready(self: &Rc<Self>, generation: i64, widget: &impl IsA<gtk::Widget>) {
        let serial = self.review_request_serial.get().wrapping_add(1);
        self.review_request_serial.set(serial);
        self.review_requested.set(Some(generation));
        let frames = Cell::new(0usize);
        let weak = Rc::downgrade(self);
        widget.add_tick_callback(move |_, _| {
            frames.set(frames.get() + 1);
            if frames.get() < 2 {
                return glib::ControlFlow::Continue;
            }
            if let Some(ui) = weak.upgrade()
                && ui.review_request_serial.get() == serial
            {
                ui.review_requested.set(None);
            }
            glib::ControlFlow::Break
        });
        let weak = Rc::downgrade(self);
        glib::timeout_add_local(std::time::Duration::from_millis(20), move || {
            let Some(ui) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            if ui.review_request_serial.get() != serial || ui.review_requested.get().is_none() {
                return glib::ControlFlow::Break;
            }
            if !ui.window.is_visible() || ui.stack.visible_child_name().as_deref() != Some("order")
            {
                ui.review_requested.set(None);
                return glib::ControlFlow::Break;
            }
            glib::ControlFlow::Continue
        });
    }
    fn reuse_resolved_review(self: &Rc<Self>, prepared: &PreparedReview) -> bool {
        let store = self.store.borrow();
        let state = &store.state;
        let Ok(catalog) = state.catalog() else {
            return false;
        };
        let count = state.draft.lines.len().max(1).div_ceil(PAGE_SIZE);
        let page = self.table_page.get().min(count - 1);
        let start = page * PAGE_SIZE;
        let end = (start + PAGE_SIZE).min(state.draft.lines.len());
        let mut rows = self.row_editors.borrow_mut();
        if rows.is_empty()
            || rows.len() != end - start
            || !prepared.unresolved.is_empty()
            || rows.iter().any(|row| row.cells.is_none())
            || prepared.rows[start..end]
                .iter()
                .any(|row| row.selected.is_none())
        {
            return false;
        }
        self.table_page.set(page);
        if self.table_has_focus() {
            gtk::prelude::GtkWindowExt::set_focus(&self.window, None::<&gtk::Widget>);
        }
        // GTK edits on a mapped hundred-row grid trigger repeated viewport
        // and accessibility work. Hold the same page detached for this one
        // synchronous update; no intermediate frame or user event can occur.
        let held = self.review.first_child();
        if let Some(content) = &held {
            self.review.remove(content);
        }
        let loop_started = std::time::Instant::now();
        let loading = self.loading.replace(true);
        for (row, index) in rows.iter_mut().zip(start..end) {
            let line = &state.draft.lines[index];
            let product = &catalog.products[prepared.rows[index].selected.unwrap()];
            let cells = row.cells.as_ref().unwrap();
            row.index = index;
            row.live_index.set(index);
            row.remove
                .set_widget_name(&format!("quote-row-remove-{index}"));
            if row.query.text() != line.query {
                row.query.set_text(&line.query);
            }
            let quantity = state
                .settings
                .display_digits
                .format(qty_display(line.quantity_milli));
            if row.quantity.text() != quantity {
                row.quantity.set_text(&quantity);
            }
            if row.query.has_css_class("error") {
                entry_validity(&row.query, None, QUERY_HINT);
            }
            if row.quantity.has_css_class("error") {
                entry_validity(&row.quantity, None, QUANTITY_HINT);
            }
            accessible_name(
                &row.query,
                &format!("کد یا نام مستعار ردیف {}", fa(index + 1)),
            );
            accessible_name(&row.quantity, &format!("تعداد ردیف {}", fa(index + 1)));
            accessible_name(&row.remove, &format!("حذف ردیف {}", fa(index + 1)));
            cells
                .number
                .set_text(&state.settings.display_digits.format(index + 1));
            cells.description.set_text(&product.name);
            cells.description.set_tooltip_text(Some(&format!(
                "{} • {}\nمنبع: {} • واحد منبع: {}",
                product.code,
                product.name,
                product.source_at,
                product.source_unit.label()
            )));
            cells.measure.set_text(&product.measure);
            cells.price.set_text(
                &state
                    .settings
                    .display_digits
                    .format(dnumber(state.draft.display_unit, product.price_rial)),
            );
            let amount = line_total(product.price_rial, line.quantity_milli)
                .ok()
                .map(|amount| {
                    state
                        .settings
                        .display_digits
                        .format(dnumber(state.draft.display_unit, amount))
                })
                .unwrap_or_else(|| "—".into());
            cells.amount.set_text(&amount);
            cells
                .amount
                .set_widget_name(&format!("quote-row-amount-{index}"));
            let stale = stale(&product.source_at);
            cells.status.set_text(if stale {
                "قیمت قدیمی"
            } else if line.chosen_code.is_some() {
                "انتخاب انسانی"
            } else {
                "تطبیق دقیق"
            });
            cells
                .status
                .remove_css_class(if stale { "success" } else { "warning" });
            cells
                .status
                .add_css_class(if stale { "warning" } else { "success" });
            cells.status.set_tooltip_text(Some(&format!(
                "قیمت {} • منبع {}",
                state.draft.display_unit.display(product.price_rial),
                product.source_at
            )));
        }
        self.loading.set(loading);
        performance_trace("review_reuse_setters", loop_started);
        let chrome_started = std::time::Instant::now();
        drop(rows);
        if let Some(content) = &held {
            self.review.append(content);
        }
        if let Some(bar) = review_named::<gtk::Box>(&self.review, "quote-page-bar") {
            bar.set_visible(count > 1);
        }
        if let Some(label) = review_named::<gtk::Label>(&self.review, "quote-page-count") {
            label.set_text(&page_caption(state, start, page, count));
        }
        for (name, sensitive) in [
            ("quote-page-previous", page > 0),
            ("quote-page-next", page + 1 < count),
        ] {
            if let Some(control) = review_named::<gtk::Button>(&self.review, name) {
                control.set_sensitive(sensitive);
            }
        }
        for (column, text) in [
            (
                5,
                format!("قیمت واحد\n{}", state.draft.display_unit.label()),
            ),
            (
                6,
                format!("مبلغ ردیف\n{}", state.draft.display_unit.label()),
            ),
        ] {
            if let Some(label) =
                review_named::<gtk::Label>(&self.review, &format!("quote-header-{column}"))
            {
                label.set_text(&text);
            }
        }
        performance_trace("review_reuse_chrome", chrome_started);
        let settings_started = std::time::Instant::now();
        settings::apply(self);
        performance_trace("review_reuse_settings", settings_started);
        true
    }
    pub(super) fn commit_table(self: &Rc<Self>) {
        let r = self.save_form();
        let ok = r.is_ok();
        self.message(r.map(|()| "پیش‌نویس ذخیره و بررسی شد"));
        if ok {
            self.refresh_review();
        }
    }
    pub(super) fn add_line(self: &Rc<Self>) {
        let result = (|| {
            self.save_form()?;
            let query = self.new_item.text().to_string();
            text(&query)?;
            let qty = quantity(&self.new_quantity.text())?;
            let chosen = self.new_product_choice.borrow().clone();
            self.store.borrow_mut().change(|s| {
                let chosen_code = chosen
                    .filter(|(catalog, code)| catalog == &s.draft.catalog_id && code == &query)
                    .map(|(_, code)| code);
                s.draft.lines.push(OrderLine {
                    query,
                    quantity_milli: qty,
                    chosen_code,
                });
                s.draft.stale_ack = false;
                Ok(())
            })
        })();
        let ok = result.is_ok();
        self.message(result.map(|()| "ردیف اضافه شد"));
        if ok {
            self.new_product_choice.borrow_mut().take();
            self.new_item.set_text("");
            self.fill_form();
            self.table_page.set(
                self.store
                    .borrow()
                    .state
                    .draft
                    .lines
                    .len()
                    .saturating_sub(1)
                    / PAGE_SIZE,
            );
            self.refresh_review();
            self.new_item.grab_focus();
            ensure_focus_visible(&self.new_item);
        }
    }
    pub(super) fn delete_line(self: &Rc<Self>, index: usize) {
        let page = self.table_page.get();
        let result = self.save_form().and_then(|()| {
            let removed = self
                .store
                .borrow()
                .state
                .draft
                .lines
                .get(index)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("ردیف موجود نیست"))?;
            self.store.borrow_mut().change(|s| {
                anyhow::ensure!(index < s.draft.lines.len(), "ردیف موجود نیست");
                s.draft.lines.remove(index);
                Ok(())
            })?;
            *self.deleted_lines.borrow_mut() = Some((index, removed));
            Ok(())
        });
        let ok = result.is_ok();
        self.message(result.map(|()| "ردیف حذف شد؛ امکان برگرداندن دارید"));
        if ok {
            self.undo_button.set_sensitive(true);
            self.fill_form();
            self.table_page.set(page);
            self.refresh_review();
        }
    }
    pub(super) fn undo_delete(self: &Rc<Self>) {
        let page = self.table_page.get();
        let result = self.save_form().and_then(|()| {
            let (index, removed) = self
                .deleted_lines
                .borrow()
                .clone()
                .ok_or_else(|| anyhow::anyhow!("حذفی برای برگرداندن نیست"))?;
            self.store.borrow_mut().change(|s| {
                s.draft
                    .lines
                    .insert(index.min(s.draft.lines.len()), removed);
                s.draft.stale_ack = false;
                Ok(())
            })
        });
        let ok = result.is_ok();
        self.message(result.map(|()| "ردیف‌های پیش از حذف برگردانده شدند"));
        if ok {
            self.deleted_lines.borrow_mut().take();
            self.undo_button.set_sensitive(false);
            self.fill_form();
            self.table_page.set(page);
            self.refresh_review();
        }
    }
    pub(super) fn preview_document(self: &Rc<Self>) {
        if !self.background_available() {
            return;
        }
        self.save_form_async_then(|ui| {
            let state = ui.store.borrow().state.clone();
            let parent = ui.store.borrow().directory.clone();
            ui.run_background(
                "آماده‌سازی پیش‌نمایش…",
                move || {
                    let quote = state.preview()?;
                    let approved = state
                        .quotes
                        .iter()
                        .filter(|saved| saved.series == quote.series)
                        .max_by_key(|saved| saved.version)
                        .filter(|saved| quote_same_content(saved, &quote))
                        .cloned();
                    let (snapshot, title) = if let Some(saved) = approved {
                        let title = format!(
                            "پیش‌نمایش و خروجی • تأییدشده {} • نسخهٔ {}",
                            saved.number,
                            fa(saved.version)
                        );
                        (saved, title)
                    } else {
                        (quote, "پیش‌نمایش و خروجی • پیش‌نویس تأیید نشده".into())
                    };
                    let pages = background::PreviewPages::render(&snapshot, &parent)?;
                    Ok((pages, snapshot, title))
                },
                |ui, result| match result {
                    Ok((pages, quote, title)) => {
                        ui.show_rendered_quote_preview(pages, quote, &title)
                    }
                    Err(error) => ui.message(Err::<String, _>(error)),
                },
            );
        });
    }
}
fn dnumber(unit: Unit, n: i64) -> String {
    unit.display(n)
        .trim_end_matches(unit.label())
        .trim()
        .to_string()
}

fn retire_review(roots: Vec<gtk::Widget>, mut rows: Vec<RowEditor>) {
    let grids = roots
        .iter()
        .filter_map(|root| review_named::<gtk::Grid>(root, "quote-grid"))
        .collect::<Vec<_>>();
    let mut children = grids
        .iter()
        .flat_map(|grid| {
            let mut children = Vec::new();
            let mut child = grid.first_child();
            while let Some(widget) = child {
                child = widget.next_sibling();
                children.push((grid.clone(), widget));
            }
            children
        })
        .collect::<std::collections::VecDeque<_>>();
    glib::idle_add_local(move || {
        let began = std::time::Instant::now();
        for _ in 0..180 {
            if let Some((grid, child)) = children.pop_front() {
                grid.remove(&child);
            } else {
                break;
            }
        }
        let count = rows.len().min(20);
        rows.drain(..count);
        performance_trace("review_retire_20_rows", began);
        if children.is_empty() && rows.is_empty() {
            // Root/header references outlive the last detached row's disposal.
            let _ = &roots;
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
}

fn performance_trace(operation: &str, started: std::time::Instant) {
    if std::env::var("SARV_PERFORMANCE_TRACE").as_deref() == Ok("1") {
        eprintln!(
            "SARV_MAIN_THREAD {operation} {:.3}ms",
            started.elapsed().as_secs_f64() * 1000.0
        );
    }
}

#[cfg(test)]
mod presentation_tests {
    use super::*;

    fn fixture() -> State {
        let mut state = State::default();
        let source = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        let csv = format!(
            "code,name,measure,price,currency,source_at,aliases\nONE,اول,عدد,12345,rial,{source},مشترک\nTWO,دوم,عدد,23456,rial,{source},مشترک\n"
        );
        imports::accept_catalog(&mut state, imports::catalog_csv(&csv, "ساختگی").unwrap()).unwrap();
        state.draft.customer = "مشتری ساختگی".into();
        state.draft.lines = vec![
            OrderLine {
                query: "مشترک".into(),
                quantity_milli: 1000,
                chosen_code: Some("TWO".into()),
            },
            OrderLine {
                query: "ONE".into(),
                quantity_milli: 2000,
                chosen_code: None,
            },
        ];
        state
    }

    #[test]
    fn prepared_review_preserves_explicit_choice_and_exact_total() {
        let prepared = PreparedReview::prepare(17, &fixture());
        assert_eq!(prepared.rows[0].count, 2);
        assert_eq!(prepared.rows[0].selected, Some(1));
        assert!(prepared.unresolved.is_empty());
        let quote = prepared.preview.unwrap();
        assert_eq!(quote.lines[0].code, "TWO");
        assert_eq!(quote.total_rial, 48_146);
    }

    #[test]
    fn ambiguous_invalid_choice_and_older_generation_are_never_approved() {
        let mut state = fixture();
        state.draft.lines[0].chosen_code = Some("MISSING".into());
        let prepared = PreparedReview::prepare(17, &state);
        assert_eq!(prepared.unresolved, vec![0]);
        assert!(prepared.preview.is_err());
        assert!(!prepared.current(18));
        state.draft.lines[0].chosen_code = None;
        assert!(PreparedReview::prepare(18, &state).preview.is_err());
    }
}
