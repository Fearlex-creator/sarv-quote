use super::*;
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::mpsc,
    thread,
    time::Duration,
};

/// Native document controls shared by draft and immutable revision previews.
/// Pages remain continuously scrollable; page buttons jump to the selected page.
pub(super) fn document_preview(paths: Vec<PathBuf>) -> adw::ToolbarView {
    let view = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    let zoom = gtk::DropDown::from_strings(&["۷۵٪", "۱۰۰٪", "۱۲۵٪"]);
    zoom.set_selected(1);
    zoom.set_widget_name("preview-zoom");
    zoom.set_tooltip_text(Some("بزرگ‌نمایی پیش‌نمایش"));
    zoom.update_property(&[gtk::accessible::Property::Label("بزرگ‌نمایی سند")]);
    header.pack_end(&zoom);
    let pager = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let previous = gtk::Button::with_label("قبلی");
    previous.set_widget_name("preview-previous-page");
    previous.set_tooltip_text(Some("صفحهٔ قبلی"));
    let next = gtk::Button::with_label("بعدی");
    next.set_widget_name("preview-next-page");
    next.set_tooltip_text(Some("صفحهٔ بعدی"));
    let indicator = gtk::Label::new(None);
    indicator.set_widget_name("preview-page-indicator");
    indicator.set_width_chars(12);
    pager.append(&previous);
    pager.append(&indicator);
    pager.append(&next);
    header.set_title_widget(Some(&pager));
    view.add_top_bar(&header);
    let pages = column();
    pages.set_margin_top(12);
    pages.set_margin_bottom(12);
    pages.set_spacing(16);
    let pictures: Vec<gtk::Picture> = paths
        .into_iter()
        .enumerate()
        .map(|(index, path)| {
            let picture = gtk::Picture::for_filename(path);
            picture.set_can_shrink(true);
            picture.set_size_request(700, 990);
            picture.set_valign(gtk::Align::Start);
            picture.set_vexpand(false);
            picture.update_property(&[gtk::accessible::Property::Label(&format!(
                "صفحهٔ {} سند",
                fa(index + 1)
            ))]);
            pages.append(&picture);
            picture
        })
        .collect();
    let count = pictures.len();
    let current = Rc::new(Cell::new(0usize));
    let weak_previous = previous.downgrade();
    let weak_next = next.downgrade();
    let weak_indicator = indicator.downgrade();
    let update: Rc<dyn Fn(usize)> = Rc::new(move |index| {
        if let Some(previous) = weak_previous.upgrade() {
            previous.set_sensitive(index > 0);
        }
        if let Some(next) = weak_next.upgrade() {
            next.set_sensitive(index + 1 < count);
        }
        if let Some(indicator) = weak_indicator.upgrade() {
            indicator.set_text(&format!(
                "صفحهٔ {} از {}",
                fa(if count == 0 { 0 } else { index + 1 }),
                fa(count)
            ));
        }
    });
    update(0);
    let scrolled = scroll(&pages);
    scrolled.set_hscrollbar_policy(gtk::PolicyType::Automatic);
    let adjustment = scrolled.vadjustment();
    for (button, forwards) in [(&previous, false), (&next, true)] {
        let current = current.clone();
        let update = update.clone();
        let pictures = pictures.clone();
        let adjustment = adjustment.downgrade();
        button.connect_clicked(move |_| {
            if pictures.is_empty() {
                return;
            }
            let index = if forwards {
                (current.get() + 1).min(pictures.len() - 1)
            } else {
                current.get().saturating_sub(1)
            };
            current.set(index);
            update(index);
            if let Some(adjustment) = adjustment.upgrade() {
                adjustment.set_value(f64::from(pictures[index].allocation().y()));
            }
        });
    }
    let scrolling_current = current.clone();
    let scrolling_update = update.clone();
    let scrolling_pictures = pictures.clone();
    adjustment.connect_value_changed(move |adjustment| {
        let index = scrolling_pictures
            .iter()
            .rposition(|picture| f64::from(picture.allocation().y()) <= adjustment.value() + 40.0)
            .unwrap_or(0);
        scrolling_current.set(index);
        scrolling_update(index);
    });
    zoom.connect_selected_notify(move |choice| {
        let width = match choice.selected() {
            0 => 525,
            2 => 875,
            _ => 700,
        };
        for picture in &pictures {
            picture.set_size_request(width, width * 842 / 595);
        }
    });
    view.set_content(Some(&scrolled));
    view
}

/// Own only the directory successfully created by this request. Failed renders,
/// closed windows and closed dialogs all release the same temporary files.
pub(super) struct PreviewPages {
    directory: PathBuf,
    pub(super) paths: Vec<PathBuf>,
}
impl PreviewPages {
    pub(super) fn render(quote: &Quote, parent: &Path) -> Result<Self> {
        let directory = parent.join(format!("preview-{}", id()));
        std::fs::create_dir(&directory)?;
        let mut owned = Self {
            directory,
            paths: vec![],
        };
        owned.paths = export::png_pages(quote, &owned.directory)?;
        Ok(owned)
    }
}
impl Drop for PreviewPages {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

/// A source-specific explicit selection for an existing draft line.
/// The worker checks the exact source and target before its single atomic save.
pub(super) struct LineProductChoice {
    pub(super) catalog_id: String,
    pub(super) fingerprint: String,
    pub(super) product_index: usize,
    pub(super) code: String,
    pub(super) line_index: usize,
    pub(super) original_query: String,
}

pub(super) enum DraftSelection {
    Product(LineProductChoice),
    Customer(Customer),
}

impl Ui {
    /// Structural quotation changes start only after the latest form save.
    /// Freeze the quote workspace while its row indices change; closing waits
    /// for the definite receipt through the shared persistence gate.
    pub(super) fn mutate_quote_async<T: Send + 'static>(
        self: &Rc<Self>,
        work: impl FnOnce(&mut State) -> Result<T> + Send + 'static,
        complete: impl FnOnce(&Rc<Self>, Result<T>) + 'static,
    ) {
        if self.persistence_busy.get() {
            complete(
                self,
                Err(anyhow::anyhow!("ذخیره در حال انجام است؛ دوباره تلاش کنید")),
            );
            return;
        }
        let (_, unchanged) = match self.form_draft() {
            Ok(value) => value,
            Err(error) => {
                complete(self, Err(error));
                return;
            }
        };
        if !unchanged {
            complete(
                self,
                Err(anyhow::anyhow!("ورودی تغییر کرده؛ دوباره تلاش کنید")),
            );
            return;
        }
        let directory = self.store.borrow().directory.clone();
        let (generation, mut candidate) = match self.store.borrow_mut().begin_detached() {
            Ok(value) => value,
            Err(error) => {
                complete(self, Err(error));
                return;
            }
        };
        if let Some(pending) = self.pending.borrow_mut().take() {
            pending.remove();
        }
        let frozen = self.stack.child_by_name("order").map(|widget| {
            let sensitive = widget.is_sensitive();
            widget.set_sensitive(false);
            (widget, sensitive)
        });
        self.persistence_busy.set(true);
        self.message(Ok("در حال ذخیرهٔ تغییر ردیف‌ها…"));
        let (sender, receiver) = mpsc::sync_channel(1);
        let spawned = thread::Builder::new()
            .name("sarv-quote-mutation".into())
            .spawn(move || {
                let result = catch_unwind(AssertUnwindSafe(|| {
                    let value = work(&mut candidate)?;
                    let prepared = Store::prepare_candidate(generation, candidate)?;
                    Ok((Store::commit_detached(directory, prepared)?, value))
                }))
                .unwrap_or_else(|_| Err(anyhow::anyhow!("ذخیره قطع شد؛ ورودی شما حفظ شده است")));
                let _ = sender.send(result);
            });
        if let Err(error) = spawned {
            let result = self.store.borrow_mut().finish_detached(Err(error.into()));
            self.persistence_busy.set(false);
            self.close_after_save.set(false);
            if let Some((widget, sensitive)) = frozen {
                widget.set_sensitive(sensitive);
            }
            complete(
                self,
                result.and_then(|()| Err(anyhow::anyhow!("ذخیره انجام نشد"))),
            );
            return;
        }
        let ui = self.clone();
        let mut complete = Some(complete);
        glib::timeout_add_local(Duration::from_millis(20), move || {
            let result = match receiver.try_recv() {
                Ok(result) => result,
                Err(mpsc::TryRecvError::Empty) => return glib::ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => {
                    Err(anyhow::anyhow!("ذخیره قطع شد؛ دوباره تلاش کنید"))
                }
            };
            let (receipt, value) = match result {
                Ok((receipt, value)) => (Ok(receipt), Some(value)),
                Err(error) => (Err(error), None),
            };
            let result = ui.store.borrow_mut().finish_detached(receipt);
            ui.persistence_busy.set(false);
            if let Some((widget, sensitive)) = &frozen {
                widget.set_sensitive(*sensitive);
            }
            let succeeded = result.is_ok();
            let result = result
                .and_then(|()| value.ok_or_else(|| anyhow::anyhow!("نتیجهٔ ذخیره موجود نیست")));
            if !succeeded {
                ui.close_after_save.set(false);
                ui.pending_save_action.borrow_mut().take();
            }
            if let Some(complete) = complete.take() {
                complete(&ui, result);
            }
            if succeeded && !ui.persistence_busy.get() {
                if ui.pending_save_action.borrow().is_some() {
                    ui.save_form_async();
                } else if ui.close_after_save.replace(false) {
                    ui.window.close();
                }
            }
            glib::ControlFlow::Break
        });
    }

    pub(super) fn choose_draft_selection_async(
        self: &Rc<Self>,
        selection: DraftSelection,
        complete: impl FnOnce(&Rc<Self>, Result<bool>) + 'static,
    ) {
        if self.persistence_busy.get() {
            complete(
                self,
                Err(anyhow::anyhow!(
                    "ذخیره در حال انجام است؛ پس از پایان دوباره انتخاب کنید"
                )),
            );
            return;
        }
        let draft = match self.form_draft() {
            Ok((draft, _)) => draft,
            Err(error) => {
                complete(self, Err(error));
                return;
            }
        };
        let directory = self.store.borrow().directory.clone();
        let detached = self.store.borrow_mut().begin_detached();
        let (generation, mut candidate) = match detached {
            Ok(value) => value,
            Err(error) => {
                complete(self, Err(error));
                return;
            }
        };
        let previous_customer = matches!(&selection, DraftSelection::Customer(_)).then(|| {
            (
                self.customer.text().to_string(),
                self.phone.text().to_string(),
            )
        });
        candidate.draft = draft;
        let revision = self.edit_revision.get();
        if let Some(pending) = self.pending.borrow_mut().take() {
            pending.remove();
        }
        self.persistence_busy.set(true);
        self.message(Ok(match &selection {
            DraftSelection::Product(_) => "در حال ذخیرهٔ انتخاب کالا و پیش‌نویس…",
            DraftSelection::Customer(_) => "در حال ذخیرهٔ انتخاب مشتری و پیش‌نویس…",
        }));
        let (sender, receiver) = mpsc::sync_channel(1);
        let spawned = thread::Builder::new()
            .name("sarv-persistence".into())
            .spawn(move || {
                let result = catch_unwind(AssertUnwindSafe(|| {
                    match selection {
                        DraftSelection::Product(choice) => {
                            anyhow::ensure!(
                                candidate.draft.catalog_id == choice.catalog_id,
                                "فهرست قیمت تغییر کرده؛ دوباره انتخاب کنید"
                            );
                            let catalog = candidate.catalog()?;
                            anyhow::ensure!(
                                catalog.id == choice.catalog_id
                                    && catalog.fingerprint == choice.fingerprint
                                    && catalog
                                        .products
                                        .get(choice.product_index)
                                        .is_some_and(|product| product.code == choice.code),
                                "فهرست قیمت تغییر کرده؛ دوباره انتخاب کنید"
                            );
                            let line = candidate
                                .draft
                                .lines
                                .get_mut(choice.line_index)
                                .ok_or_else(|| {
                                    anyhow::anyhow!("ردیف تغییر کرده؛ دوباره بررسی کنید")
                                })?;
                            anyhow::ensure!(
                                line.query == choice.original_query,
                                "درخواست ردیف تغییر کرده؛ دوباره بررسی کنید"
                            );
                            line.chosen_code = Some(choice.code);
                            candidate.draft.stale_ack = false;
                        }
                        DraftSelection::Customer(customer) => {
                            let current = candidate
                                .customers
                                .iter()
                                .find(|record| record.id == customer.id)
                                .ok_or_else(|| {
                                    anyhow::anyhow!("مشتری تغییر کرده؛ دوباره انتخاب کنید")
                                })?;
                            anyhow::ensure!(
                                !current.archived
                                    && serde_json::to_vec(current)?
                                        == serde_json::to_vec(&customer)?,
                                "مشتری تغییر کرده یا بایگانی شده؛ دوباره انتخاب کنید"
                            );
                            candidate.select_customer(&customer.id)?;
                        }
                    }
                    let prepared = Store::prepare_candidate(generation, candidate)?;
                    Store::commit_detached(directory, prepared)
                }))
                .unwrap_or_else(|_| {
                    Err(anyhow::anyhow!(
                        "ذخیرهٔ انتخاب قطع شد؛ ورودی شما حفظ شده است"
                    ))
                });
                let _ = sender.send(result);
            });
        if let Err(error) = spawned {
            let result = self.store.borrow_mut().finish_detached(Err(error.into()));
            self.persistence_busy.set(false);
            self.close_after_save.set(false);
            self.pending_save_action.borrow_mut().take();
            complete(self, result.map(|()| false));
            return;
        }
        let ui = self.clone();
        let mut complete = Some(complete);
        glib::timeout_add_local(Duration::from_millis(20), move || {
            let result = match receiver.try_recv() {
                Ok(result) => result,
                Err(mpsc::TryRecvError::Empty) => return glib::ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => {
                    Err(anyhow::anyhow!("ذخیرهٔ انتخاب قطع شد؛ دوباره تلاش کنید"))
                }
            };
            let result = ui.store.borrow_mut().finish_detached(result);
            ui.persistence_busy.set(false);
            let same_revision = ui.edit_revision.get() == revision;
            if result.is_ok() && same_revision {
                ui.finish_form_save();
                ui.refresh_totals();
            } else if result.is_err() {
                ui.pending_save_action.borrow_mut().take();
                ui.close_after_save.set(false);
            }
            let succeeded = result.is_ok();
            if succeeded
                && !same_revision
                && let Some((name, phone)) = &previous_customer
                && ui.customer.text().as_str() == name
                && ui.phone.text().as_str() == phone
            {
                // Unrelated newer edits must not undo the explicit customer.
                // Update only untouched identity widgets; changed text wins.
                let (customer, phone) = {
                    let store = ui.store.borrow();
                    (
                        store.state.draft.customer.clone(),
                        store.state.draft.phone.clone(),
                    )
                };
                let loading = ui.loading.replace(true);
                ui.customer.set_text(&customer);
                ui.phone.set_text(&phone);
                ui.loading.set(loading);
            }
            if let Some(complete) = complete.take() {
                complete(&ui, result.map(|()| same_revision));
            }
            if succeeded {
                if !same_revision {
                    // Never replace newer GTK inputs, including invalid text,
                    // with the old receipt. Validate and save them separately.
                    ui.save_form_async();
                } else {
                    ui.run_saved_action();
                    if ui.close_after_save.replace(false) {
                        ui.window.close();
                    }
                }
            }
            glib::ControlFlow::Break
        });
    }

    pub(super) fn save_form_async_then(
        self: &Rc<Self>,
        complete: impl FnOnce(&Rc<Self>) + 'static,
    ) {
        if self.pending_save_action.borrow().is_some() {
            self.message(Ok("پس از ذخیره، جابه‌جایی انجام می‌شود…"));
            return;
        }
        *self.pending_save_action.borrow_mut() = Some(Box::new(complete));
        self.save_form_async();
    }

    fn run_saved_action(self: &Rc<Self>) -> bool {
        let action = self.pending_save_action.borrow_mut().take();
        if let Some(action) = action {
            action(self);
            true
        } else {
            false
        }
    }

    /// One durable save at a time. Edits made during persistence remain in GTK
    /// and are saved as a later revision; an old receipt never rewrites them.
    pub(super) fn save_form_async(self: &Rc<Self>) {
        if let Some(pending) = self.pending.borrow_mut().take() {
            pending.remove();
        }
        if self.persistence_busy.get() {
            // Other persistence workflows (including accounting) share this
            // gate. Keep one bounded retry rather than losing a draft's timer.
            let weak = Rc::downgrade(self);
            *self.pending.borrow_mut() = Some(glib::timeout_add_local_once(
                Duration::from_millis(100),
                move || {
                    if let Some(ui) = weak.upgrade() {
                        ui.pending.borrow_mut().take();
                        ui.save_form_async();
                    }
                },
            ));
            self.message(Ok(
                "ذخیره در حال انجام است؛ تغییرات تازه پس از آن ذخیره می‌شوند…",
            ));
            return;
        }
        let (draft, unchanged) = match self.form_draft() {
            Ok(value) => value,
            Err(error) => {
                self.pending_save_action.borrow_mut().take();
                self.close_after_save.set(false);
                self.message(Err::<String, _>(error));
                return;
            }
        };
        if unchanged {
            let result = self.store.borrow().verify_current();
            if result.is_ok() {
                self.finish_form_save();
                self.refresh_totals();
                self.message(Ok("پیش‌نویس ذخیره شد • آفلاین"));
                self.run_saved_action();
                if self.close_after_save.replace(false) {
                    self.window.close();
                }
            } else {
                self.pending_save_action.borrow_mut().take();
                self.close_after_save.set(false);
                self.message(result.map(|()| ""));
            }
            return;
        }
        let directory = self.store.borrow().directory.clone();
        let (generation, mut candidate) = match self.store.borrow_mut().begin_detached() {
            Ok(value) => value,
            Err(error) => {
                self.pending_save_action.borrow_mut().take();
                self.close_after_save.set(false);
                self.message(Err::<String, _>(error));
                return;
            }
        };
        candidate.draft = draft;
        let revision = self.edit_revision.get();
        self.persistence_busy.set(true);
        self.message(Ok("در حال ذخیرهٔ پیش‌نویس…"));
        let (sender, receiver) = mpsc::sync_channel(1);
        let spawned = thread::Builder::new()
            .name("sarv-persistence".into())
            .spawn(move || {
                let result = catch_unwind(AssertUnwindSafe(|| {
                    let prepared = Store::prepare_candidate(generation, candidate)?;
                    Store::commit_detached(directory, prepared)
                }))
                .unwrap_or_else(|_| Err(anyhow::anyhow!("ذخیره قطع شد؛ ورودی شما حفظ شده است")));
                let _ = sender.send(result);
            });
        if let Err(error) = spawned {
            self.persistence_busy.set(false);
            self.pending_save_action.borrow_mut().take();
            self.close_after_save.set(false);
            let result = self.store.borrow_mut().finish_detached(Err(error.into()));
            self.message(result.map(|()| ""));
            return;
        }
        // Keep the owner alive until its transaction has a definite outcome.
        // Close requests defer until this receipt and any newer edit are saved.
        let ui = self.clone();
        glib::timeout_add_local(Duration::from_millis(20), move || {
            let result = match receiver.try_recv() {
                Ok(result) => result,
                Err(mpsc::TryRecvError::Empty) => return glib::ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => {
                    Err(anyhow::anyhow!("ذخیره قطع شد؛ دوباره تلاش کنید"))
                }
            };
            let result = ui.store.borrow_mut().finish_detached(result);
            ui.persistence_busy.set(false);
            match result {
                Err(error) => {
                    ui.pending_save_action.borrow_mut().take();
                    ui.close_after_save.set(false);
                    ui.message(Err::<String, _>(error));
                }
                Ok(()) if ui.edit_revision.get() != revision => {
                    ui.message(Ok("نسخهٔ قبلی ذخیره شد؛ تغییرات تازه در حال ذخیره‌اند…"));
                    // Validate the latest widgets again. Invalid newer input
                    // remains visible and is never replaced by the old draft.
                    ui.save_form_async();
                }
                Ok(()) => {
                    ui.finish_form_save();
                    ui.refresh_totals();
                    if !ui.run_saved_action() && !ui.table_has_focus() {
                        ui.refresh_review();
                    }
                    ui.message(Ok("پیش‌نویس ذخیره شد • آفلاین"));
                    if ui.close_after_save.replace(false) {
                        ui.window.close();
                    }
                }
            }
            glib::ControlFlow::Break
        });
    }

    pub(super) fn background_available(&self) -> bool {
        if self.background_busy.get() {
            self.message(Ok(
                "آماده‌سازی فایل در حال انجام است؛ پس از پایان دوباره درخواست کنید",
            ));
            false
        } else {
            true
        }
    }

    /// GTK stays on its owning thread. The worker receives only immutable data
    /// and creates its own Cairo/Pango contexts; results return through mpsc.
    pub(super) fn run_background<T: Send + 'static>(
        self: &Rc<Self>,
        working: &str,
        work: impl FnOnce() -> Result<T> + Send + 'static,
        complete: impl FnOnce(&Rc<Self>, Result<T>) + 'static,
    ) -> bool {
        if !self.background_available() {
            return false;
        }
        self.background_busy.set(true);
        self.message(Ok(working));
        let (sender, receiver) = mpsc::channel();
        let spawned = thread::Builder::new()
            .name("sarv-export".into())
            .spawn(move || {
                let result = catch_unwind(AssertUnwindSafe(work)).unwrap_or_else(|_| {
                    Err(anyhow::anyhow!(
                        "آماده‌سازی فایل به پایان نرسید؛ دوباره تلاش کنید"
                    ))
                });
                // If the UI has closed, send returns the owned result and drops it.
                // PreviewPages consequently removes its temporary directory.
                let _ = sender.send(result);
            });
        if let Err(error) = spawned {
            self.background_busy.set(false);
            self.message(Err::<String, _>(error.into()));
            return false;
        }
        let weak = Rc::downgrade(self);
        let mut complete = Some(complete);
        glib::timeout_add_local(Duration::from_millis(50), move || {
            let Some(ui) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            if !ui.window.is_visible() {
                ui.background_busy.set(false);
                return glib::ControlFlow::Break;
            }
            match receiver.try_recv() {
                Ok(result) => {
                    ui.background_busy.set(false);
                    if let Some(complete) = complete.take() {
                        complete(&ui, result);
                    }
                    glib::ControlFlow::Break
                }
                Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => {
                    ui.background_busy.set(false);
                    if let Some(complete) = complete.take() {
                        complete(
                            &ui,
                            Err(anyhow::anyhow!("آماده‌سازی فایل قطع شد؛ دوباره تلاش کنید")),
                        );
                    }
                    glib::ControlFlow::Break
                }
            }
        });
        true
    }

    pub(super) fn show_rendered_quote_preview(
        self: &Rc<Self>,
        pages: PreviewPages,
        quote: Quote,
        title: &str,
    ) {
        self.present_rendered_document(pages, Some(quote), title);
    }

    pub(super) fn show_quote_preview(self: &Rc<Self>, quote: Quote, title: &str) {
        let snapshot = quote.clone();
        let parent = self.store.borrow().directory.clone();
        let title = title.to_owned();
        self.run_background(
            "آماده‌سازی پیش‌نمایش…",
            move || PreviewPages::render(&quote, &parent),
            move |ui, result| match result {
                Ok(pages) => ui.show_rendered_quote_preview(pages, snapshot, &title),
                Err(error) => ui.message(Err::<String, _>(error)),
            },
        );
    }

    fn present_rendered_document(
        self: &Rc<Self>,
        pages: PreviewPages,
        quote: Option<Quote>,
        title: &str,
    ) {
        // Finishing a long render must not replace a dialog the user opened
        // while navigating or editing. The abandoned pages are removed here.
        if self.active_dialog.borrow().is_some() {
            self.message(Ok(
                "پنجرهٔ فعلی باز است؛ پس از بستن آن پیش‌نمایش را دوباره درخواست کنید",
            ));
            return;
        }
        let dialog = adw::Dialog::builder()
            .title(title)
            .content_width(830)
            .content_height(850)
            .build();
        let toolbar = document_preview(pages.paths.clone());
        if let Some(quote) = quote {
            let actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            actions.set_halign(gtk::Align::Center);
            actions.set_margin_top(6);
            actions.set_margin_bottom(6);
            let snapshot = Rc::new(quote);
            for extension in ["pdf", "csv"] {
                let button =
                    gtk::Button::with_label(&format!("خروجی {}", extension.to_uppercase()));
                button.set_widget_name(&format!("preview-export-{extension}"));
                button.set_tooltip_text(Some("ذخیرهٔ همین نسخهٔ نمایش‌داده‌شده در فایل تازه"));
                let weak = Rc::downgrade(self);
                let snapshot = snapshot.clone();
                button.connect_clicked(move |_| {
                    let Some(ui) = weak.upgrade() else {
                        return;
                    };
                    if !ui.background_available() {
                        return;
                    }
                    let quote = snapshot.as_ref().clone();
                    let file_name = format!(
                        "sarv-quote-{}-v{}.{}",
                        if quote.id.is_empty() {
                            "draft"
                        } else {
                            &quote.id
                        },
                        quote.version,
                        extension
                    );
                    let weak = Rc::downgrade(&ui);
                    choose_file(&ui.window, true, &file_name, move |path| {
                        let Some(ui) = weak.upgrade() else {
                            return;
                        };
                        let quote = quote.clone();
                        let output = path.clone();
                        ui.run_background(
                            "آماده‌سازی خروجی نسخهٔ نمایش‌داده‌شده…",
                            move || {
                                if extension == "pdf" {
                                    export::pdf(&quote, &output)
                                } else {
                                    export::csv(&quote, &output)
                                }
                            },
                            move |ui, result| {
                                ui.message(result.map(|()| {
                                    format!(
                                        "خروجی {} ذخیره شد: {}",
                                        extension.to_uppercase(),
                                        path.display()
                                    )
                                }))
                            },
                        );
                    });
                });
                actions.append(&button);
            }
            toolbar.add_bottom_bar(&actions);
        }
        dialog.set_child(Some(&toolbar));
        let cleanup = RefCell::new(Some(pages));
        dialog.connect_closed(move |dialog| {
            dialog.set_child(None::<&gtk::Widget>);
            cleanup.borrow_mut().take();
        });
        self.message(Ok("پیش‌نمایش نسخهٔ درخواست‌شده آماده شد"));
        self.present_dialog(&dialog);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_quote() -> Quote {
        let mut state = State::default();
        let source = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        let prices = format!(
            "code,name,measure,price,currency,source_at,aliases\nSYN-1,کالای ساختگی,عدد,100000,rial,{source},نام ساختگی\n"
        );
        let catalog = imports::catalog_csv(&prices, "آزمایش ساختگی").unwrap();
        let code = catalog.products[0].code.clone();
        imports::accept_catalog(&mut state, catalog).unwrap();
        state.draft.customer = "مشتری ساختگی".into();
        state.draft.stale_ack = true;
        state.draft.lines = vec![OrderLine {
            query: code,
            quantity_milli: 1000,
            chosen_code: None,
        }];
        state.preview().unwrap()
    }

    #[test]
    fn worker_preview_owns_and_releases_temporary_files() {
        fn assert_send<T: Send>() {}
        assert_send::<PreviewPages>();
        let parent = tempfile::tempdir().unwrap();
        let worker_parent = parent.path().to_path_buf();
        let quote = sample_quote();
        let owned = thread::spawn(move || PreviewPages::render(&quote, &worker_parent))
            .join()
            .unwrap()
            .unwrap();
        assert!(!owned.paths.is_empty());
        assert!(owned.paths.iter().all(|path| path.is_file()));
        assert!(owned.directory.is_dir());
        drop(owned);
        assert_eq!(std::fs::read_dir(parent.path()).unwrap().count(), 0);
    }

    #[test]
    fn failed_worker_render_removes_only_its_owned_directory() {
        let parent = tempfile::tempdir().unwrap();
        let keep = parent.path().join("keep-existing.txt");
        std::fs::write(&keep, b"preserve").unwrap();
        let mut quote = sample_quote();
        quote.total_rial += 1;
        assert!(PreviewPages::render(&quote, parent.path()).is_err());
        assert_eq!(std::fs::read(&keep).unwrap(), b"preserve");
        assert_eq!(std::fs::read_dir(parent.path()).unwrap().count(), 1);
    }

    #[test]
    #[ignore = "registers application fonts; run alone before any concurrent font rendering"]
    fn four_row_business_quote_with_native_fonts_fits_one_a4_page() {
        let _fonts = workspace::install_fonts().unwrap();
        let mut quote = sample_quote();
        quote.lines = vec![quote.lines[0].clone(); 4];
        quote.issuer_name = "پخش نمونهٔ سرو".into();
        quote.issuer_phone = "02100000000".into();
        quote.issuer_address = "تهران • نشانی نمایشی".into();
        quote.header = "سربرگ نمایشی • فقط پیشنهاد قیمت".into();
        quote.terms = "اعتبار قراردادی چهارده روز؛ بدون رزرو موجودی.".into();
        quote.footer = "قیمت‌ها نیازمند تأیید منبع هستند.".into();
        quote.number = "DEMO-42".into();
        quote.id = id();
        quote.version = 1;
        quote.approved_at = now();
        quote.display_digits = DisplayDigits::Latin;
        quote.date_display = DateDisplay::DayMonthYear;
        quote.discount_rial = 1000;
        quote.tax_basis_points = 500;
        let net = quote.lines.iter().map(|line| line.total_rial).sum::<i64>() - quote.discount_rial;
        quote.tax_rial = tax_amount(net, quote.tax_basis_points).unwrap();
        quote.total_rial = net + quote.tax_rial;
        let directory = tempfile::tempdir().unwrap();
        let pages = export::png_pages(&quote, directory.path()).unwrap();
        assert_eq!(
            pages.len(),
            1,
            "a short four-row business quote must fit on one A4 page with the actual Persian font"
        );
        if let Ok(destination) = std::env::var("SARV_DOCUMENT_LAYOUT_ARTIFACT_DIR") {
            let destination = PathBuf::from(destination);
            std::fs::create_dir_all(&destination).unwrap();
            std::fs::copy(&pages[0], destination.join("four-row-business.png")).unwrap();
            export::pdf(&quote, &destination.join("four-row-business.pdf")).unwrap();
        }
    }
}
