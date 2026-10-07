//! Native editable quotation table; approved prices remain read-only.
use super::*;

pub(super) struct RowEditor {
    pub query: gtk::Entry,
    pub quantity: gtk::Entry,
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
        let matcher = c.index();
        let grid = gtk::Grid::new();
        grid.set_column_spacing(8);
        grid.set_row_spacing(1);
        grid.set_hexpand(true);
        let headers = [
            "ردیف",
            "کد / نام مستعار",
            "شرح کالا",
            "تعداد",
            "واحد",
            "قیمت واحد",
            "مبلغ ردیف",
            "بررسی",
            "",
        ];
        let widths = [28, 130, 155, 58, 38, 93, 105, 135, 25];
        for (i, h) in headers.iter().enumerate() {
            let l = table_cell(h, i == 2);
            l.add_css_class("table-header");
            l.set_wrap(false);
            cell(&grid, &l, i as i32, 0, widths[i]);
        }
        for (index, line) in s.draft.lines.iter().enumerate() {
            let r = index as i32 + 1;
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
            cell(&grid, &numeric_cell(&fa(index + 1)), 0, r, widths[0]);
            let query = entry("کد کالا");
            accessible_name(&query, &format!("کد یا نام مستعار ردیف {}", fa(index + 1)));
            query.set_text(&line.query);
            query.set_width_chars(9);
            query.set_max_width_chars(16);
            query.add_css_class("cell-editor");
            query.set_tooltip_text(Some("کد دقیق یا نام مستعار تأییدشده • Enter برای بررسی"));
            cell(&grid, &query, 1, r, widths[1]);
            let description = table_cell(
                selected
                    .map(|p| p.name.as_str())
                    .unwrap_or("انتخاب کالا لازم است"),
                true,
            );
            description.set_wrap(false);
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
            qty.set_text(&qty_display(line.quantity_milli));
            qty.set_width_chars(4);
            qty.set_max_width_chars(7);
            qty.set_hexpand(false);
            qty.set_direction(gtk::TextDirection::Ltr);
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
                .map(|p| dnumber(s.draft.display_unit, p.price_rial))
                .unwrap_or_else(|| "—".into());
            let amount = selected
                .and_then(|p| line_total(p.price_rial, line.quantity_milli).ok())
                .map(|n| dnumber(s.draft.display_unit, n))
                .unwrap_or_else(|| "—".into());
            let price_cell = numeric_cell(&price);
            let amount_cell = numeric_cell(&amount);
            cell(&grid, &price_cell, 5, r, widths[5]);
            cell(&grid, &amount_cell, 6, r, widths[6]);
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
            query.connect_changed(move |_| {
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
            qty.connect_changed(move |_| amount_cell.set_text("—"));
            for e in [&query, &qty] {
                let weak = Rc::downgrade(self);
                e.connect_changed(move |_| {
                    if let Some(ui) = weak.upgrade()
                        && !ui.loading.get()
                    {
                        ui.table_dirty.set(true);
                        ui.visual_pending.set(true);
                        ui.schedule();
                    }
                });
                let weak = Rc::downgrade(self);
                e.connect_activate(move |_| {
                    if let Some(ui) = weak.upgrade() {
                        ui.commit_table();
                    }
                });
            }
            self.row_editors.borrow_mut().push(RowEditor {
                query,
                quantity: qty,
            });
        }
        self.review.append(&grid);
        settings::apply(self);
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
            self.refresh_review();
            self.new_item.grab_focus();
        }
    }
    pub(super) fn delete_line(self: &Rc<Self>, index: usize) {
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
            self.refresh_review();
        }
    }
    pub(super) fn undo_delete(self: &Rc<Self>) {
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
