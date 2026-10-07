//! Native editable quotation table; approved prices remain read-only.
use super::*;

pub(super) struct RowEditor {
    /// Position in the complete persisted draft, independent of visible page.
    pub index: usize,
    pub query: gtk::Entry,
    pub quantity: gtk::Entry,
    review: Option<gtk::DropDown>,
    remove: gtk::Button,
}
const PAGE_SIZE: usize = 100;
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
        match s.preview() {
            Ok(q) => {
                self.document_state
                    .set_text(&format!("پیش‌نویس {}", q.number));
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
        // Rebuilding after Enter, Save or source review must not send keyboard
        // users back to the window toolbar or lose their caret selection.
        let focused = gtk::prelude::GtkWindowExt::focus(&self.window).and_then(|focus| {
            self.row_editors
                .borrow()
                .iter()
                .find_map(|row| row.focused(&focus).map(|target| (row.index, target)))
        });
        self.visual_pending.set(false);
        clear(&self.review);
        self.row_editors.borrow_mut().clear();
        self.refresh_totals();
        let st = self.store.borrow();
        let s = &st.state;
        let Ok(c) = s.catalog() else {
            self.review.append(&empty_state(
                "فهرست قیمت لازم است",
                "فایل CSV را پیش‌نمایش و منبع آن را تأیید کنید؛ سپس سفارش مشتری را وارد کنید.",
            ));
            return;
        };
        let total_rows = s.draft.lines.len();
        let page_count = total_rows.max(1).div_ceil(PAGE_SIZE);
        let page = self.table_page.get().min(page_count - 1);
        self.table_page.set(page);
        let start = page * PAGE_SIZE;
        // Bound widget creation independently of the complete quote length.
        // Every total and export still uses the full canonical draft.
        let page_bar = row();
        page_bar.add_css_class("toolbar");
        page_bar.set_visible(page_count > 1);
        let count = label(&if total_rows == 0 {
            "بدون ردیف".to_string()
        } else {
            format!(
                "ردیف‌های {} تا {} از {} • صفحهٔ {} از {}",
                s.settings.display_digits.format(start + 1),
                s.settings
                    .display_digits
                    .format((start + PAGE_SIZE).min(total_rows)),
                s.settings.display_digits.format(total_rows),
                s.settings.display_digits.format(page + 1),
                s.settings.display_digits.format(page_count),
            )
        });
        count.add_css_class("dimmed");
        count.add_css_class("caption");
        count.set_hexpand(true);
        page_bar.append(&count);
        for (caption, target, enabled, name) in [
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
                    match ui.save_form() {
                        Ok(()) => {
                            ui.table_page.set(target);
                            ui.refresh_review();
                            if let Some(row) = ui.row_editors.borrow().first() {
                                row.query.grab_focus();
                                ensure_focus_visible(&row.query);
                            }
                        }
                        Err(error) => ui.message(Err::<String, _>(error)),
                    }
                }
            });
            page_bar.append(&control);
        }
        self.review.append(&page_bar);
        let matcher = c.index();
        let grid = gtk::Grid::new();
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
        let widths = [28, 130, 155, 58, 38, 93, 105, 135, 25];
        for (i, h) in headers.iter().enumerate() {
            let l = table_cell(h, i == 2);
            l.add_css_class("table-header");
            l.set_wrap(false);
            cell(&grid, &l, i as i32, 0, widths[i]);
        }
        for (index, line) in s.draft.lines.iter().enumerate().skip(start).take(PAGE_SIZE) {
            let r = (index - start) as i32 + 1;
            let matches = matcher.matches(&line.query);
            let selected = line
                .chosen_code
                .as_ref()
                .and_then(|code| matcher.product_by_code(code))
                .or_else(|| {
                    if matches.len() == 1 {
                        Some(matches[0])
                    } else {
                        None
                    }
                });
            cell(
                &grid,
                &numeric_cell(&s.settings.display_digits.format(index + 1)),
                0,
                r,
                widths[0],
            );
            let query = entry("کد کالا");
            accessible_name(&query, &format!("کد یا نام مستعار ردیف {}", fa(index + 1)));
            query.set_text(&line.query);
            query.set_width_chars(9);
            query.set_max_width_chars(16);
            query.add_css_class("cell-editor");
            entry_validity(&query, None, QUERY_HINT);
            cell(&grid, &query, 1, r, widths[1]);
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
            cell(&grid, &description, 2, r, widths[2]);
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
            cell(&grid, &qty, 3, r, widths[3]);
            cell(
                &grid,
                &table_cell(selected.map(|p| p.measure.as_str()).unwrap_or("—"), false),
                4,
                r,
                widths[4],
            );
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
            cell(&grid, &price_cell, 5, r, widths[5]);
            cell(&grid, &amount_cell, 6, r, widths[6]);
            let mut review_choice = None;
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
                cell(&grid, &status, 7, r, widths[7]);
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
                review_choice = Some(select.clone());
                cell(&grid, &select, 7, r, widths[7]);
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
            accessible_name(&remove, &format!("حذف ردیف {}", fa(index + 1)));
            remove.set_tooltip_text(Some("حذف ردیف • با امکان برگرداندن"));
            remove.add_css_class("row-remove");
            cell(&grid, &remove, 8, r, widths[8]);
            let weak = Rc::downgrade(self);
            remove.connect_clicked(move |_| {
                if let Some(ui) = weak.upgrade() {
                    ui.delete_line(index);
                }
            });
            let weak = Rc::downgrade(self);
            let description = description.clone();
            let amount_pending = amount_cell.clone();
            query.connect_changed(move |entry| {
                let error = text(&entry.text()).err().map(|error| error.to_string());
                entry_validity(entry, error.as_deref(), QUERY_HINT);
                if let Some(ui) = weak.upgrade()
                    && !ui.loading.get()
                {
                    ui.visual_pending.set(true);
                    ui.stale.set_active(false);
                    description.set_text("نیاز به بررسی کد جدید");
                    price_cell.set_text("—");
                    amount_pending.set_text("—");
                }
            });
            qty.connect_changed(move |entry| {
                let error = quantity(&entry.text()).err().map(|error| error.to_string());
                entry_validity(entry, error.as_deref(), QUANTITY_HINT);
                amount_cell.set_text("—");
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
                keys.connect_key_pressed(move |_, key, _, modifiers| {
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
            self.row_editors.borrow_mut().push(RowEditor {
                index,
                query,
                quantity: qty,
                review: review_choice,
                remove,
            });
        }
        self.review.append(&grid);
        settings::apply(self);
        if let Some((index, target)) = focused {
            let rows = self.row_editors.borrow();
            if let Some(row) = rows
                .iter()
                .find(|row| row.index == index)
                .or_else(|| rows.last())
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
            self.store.borrow_mut().change(|s| {
                s.draft.lines.push(OrderLine {
                    query,
                    quantity_milli: qty,
                    chosen_code: None,
                });
                s.draft.stale_ack = false;
                Ok(())
            })
        })();
        let ok = result.is_ok();
        self.message(result.map(|()| "ردیف اضافه شد"));
        if ok {
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
        let result = self
            .save_form()
            .and_then(|()| self.store.borrow().state.preview());
        match result {
            Err(error) => self.message(Err::<String, _>(error)),
            Ok(quote) => {
                let directory = self.store.borrow().directory.clone();
                self.run_background(
                    "در حال آماده‌سازی پیش‌نمایش چاپ؛ می‌توانید کار با برنامه را ادامه دهید",
                    move || super::background::PreviewPages::render(&quote, &directory),
                    |ui, result| match result {
                        Ok(pages) => {
                            ui.show_rendered_preview(pages, "پیش‌نمایش چاپ • پیش‌نویس تأیید نشده")
                        }
                        Err(error) => ui.message(Err::<String, _>(error)),
                    },
                );
            }
        }
    }
}
fn dnumber(unit: Unit, n: i64) -> String {
    unit.display(n)
        .trim_end_matches(unit.label())
        .trim()
        .to_string()
}
