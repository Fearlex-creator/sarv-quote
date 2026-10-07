use super::*;
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::mpsc,
    thread,
    time::Duration,
};

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

impl Ui {
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

    pub(super) fn show_rendered_preview(self: &Rc<Self>, pages: PreviewPages, title: &str) {
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
        dialog.set_child(Some(&document_preview(pages.paths.clone())));
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
}
