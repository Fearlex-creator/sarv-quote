//! Read-only, bounded rendering of a validated catalogue import.
//! Search and paging change visible widgets only; approval still accepts the
//! complete validated catalogue without rewriting products or source prices.
use super::*;
use std::ops::Range;

const PAGE_SIZE: usize = 100;

fn page_range(total: usize, start: usize) -> Range<usize> {
    let start = start.min(total);
    start..start.saturating_add(PAGE_SIZE).min(total)
}

#[cfg(test)]
fn matching_indices(catalog: &Catalog, needle: &str) -> Vec<usize> {
    SearchIndex::new(
        catalog
            .products
            .iter()
            .map(super::managers::product_document),
    )
    .search(needle, catalog.products.len())
}

struct Preview {
    catalog: Catalog,
    digits: DisplayDigits,
    date_display: DateDisplay,
    search_index: SearchIndex,
    matches: RefCell<Vec<usize>>,
    start: Cell<usize>,
    list: gtk::ListBox,
    count: gtk::Label,
    previous: gtk::Button,
    next: gtk::Button,
    scroller: gtk::ScrolledWindow,
}

impl Preview {
    fn render(&self) {
        let matches = self.matches.borrow();
        let range = page_range(matches.len(), self.start.get());
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        if matches.is_empty() {
            let empty = adw::ActionRow::builder()
                .title("کالا پیدا نشد")
                .subtitle("نام، کد یا نام مستعار را تغییر دهید. جست‌وجو فقط نمایش را محدود می‌کند.")
                .build();
            self.list.append(&empty);
        }
        for &index in &matches[range.clone()] {
            let product = &self.catalog.products[index];
            let source = format!(
                "منبع: {} · واحد منبع: {}{}",
                ltr(&super::managers::display_date(
                    &product.source_at,
                    self.digits,
                    self.date_display
                )),
                product.source_unit.label(),
                if stale(&product.source_at) {
                    " · قیمت قدیمی"
                } else {
                    ""
                }
            );
            let item = adw::ActionRow::builder()
                .title(format!("{} · {}", product.name, ltr(&product.code)))
                .subtitle(if product.aliases.is_empty() {
                    source
                } else {
                    format!("{}\nنام مستعار: {}", source, product.aliases.join("، "))
                })
                .title_lines(2)
                .subtitle_lines(3)
                .build();
            item.set_use_markup(false);
            let price = numeric_cell(&format!(
                "{} / {}",
                self.digits.format(Unit::Rial.display(product.price_rial)),
                product.measure
            ));
            price.set_valign(gtk::Align::Center);
            item.add_suffix(&price);
            item.set_tooltip_text(Some(&format!(
                "{}\n{}\n{}\n{} / {}\nنام مستعار: {}",
                product.name,
                product.code,
                product.source_at,
                self.digits.format(Unit::Rial.display(product.price_rial)),
                product.measure,
                product.aliases.join("، ")
            )));
            self.list.append(&item);
        }
        self.count.set_text(&format!(
            "نمایش {} تا {} از {} نتیجه · ورود با تأیید، شامل تمام {} کالای فایل است",
            self.digits
                .format(if range.is_empty() { 0 } else { range.start + 1 }),
            self.digits.format(range.end),
            self.digits.format(matches.len()),
            self.digits.format(self.catalog.products.len())
        ));
        self.previous.set_sensitive(range.start > 0);
        self.next.set_sensitive(range.end < matches.len());
        self.scroller.vadjustment().set_value(0.0);
    }
}

pub(super) fn preview(
    catalog: &Catalog,
    digits: DisplayDigits,
    date_display: DateDisplay,
) -> gtk::Box {
    let root = column();
    root.set_vexpand(true);
    let stale_count = catalog
        .products
        .iter()
        .filter(|product| stale(&product.source_at))
        .count();
    let source_units = catalog
        .products
        .iter()
        .fold((0usize, 0usize), |(rial, toman), product| {
            if product.source_unit == Unit::Rial {
                (rial + 1, toman)
            } else {
                (rial, toman + 1)
            }
        });
    let summary = label(&format!(
        "{} کالا · منبع ریالی: {} · منبع تومانی: {} · قیمت قدیمی: {}\nمبالغ زیر برای مقایسه به ریال نمایش داده می‌شوند؛ واحد و قیمت منبع محفوظ می‌ماند.",
        digits.format(catalog.products.len()),
        digits.format(source_units.0),
        digits.format(source_units.1),
        digits.format(stale_count)
    ));
    summary.add_css_class("caption");
    root.append(&summary);
    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some("جست‌وجوی نام، کد یا نام مستعار"));
    accessible_name(&search, "جست‌وجوی کالا در پیش‌نمایش ورود");
    root.append(&search);
    let count = label("");
    count.set_selectable(false);
    count.add_css_class("caption");
    root.append(&count);
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    list.add_css_class("boxed-list");
    list.add_css_class("manager-list");
    let scroller = scroll(&list);
    scroller.set_min_content_height(160);
    root.append(&scroller);
    let navigation = row();
    let previous = button("صفحهٔ قبل");
    previous.set_tooltip_text(Some("بررسی صفحهٔ پیشین قیمت‌ها؛ بدون تغییر دادهٔ واردشده"));
    let next = button("صفحهٔ بعد");
    next.set_tooltip_text(Some(
        "بررسی صفحهٔ بعدی قیمت‌ها؛ تا آخرین کالای فایل اعتبارسنجی‌شده",
    ));
    navigation.append(&previous);
    navigation.append(&next);
    root.append(&navigation);

    let preview = Rc::new(Preview {
        catalog: catalog.clone(),
        digits,
        date_display,
        search_index: SearchIndex::new(
            catalog
                .products
                .iter()
                .map(super::managers::product_document),
        ),
        matches: RefCell::new((0..catalog.products.len()).collect()),
        start: Cell::new(0),
        list,
        count,
        previous,
        next,
        scroller,
    });
    preview.render();
    // The root owns the state through this signal; navigation signals use weak
    // state references so closing the import dialog releases all widgets.
    let state = preview.clone();
    search.connect_search_changed(move |search| {
        *state.matches.borrow_mut() = state
            .search_index
            .search(&search.text(), state.catalog.products.len());
        state.start.set(0);
        state.render();
    });
    let weak = Rc::downgrade(&preview);
    preview.next.connect_clicked(move |_| {
        if let Some(state) = weak.upgrade() {
            let start = state.start.get().saturating_add(PAGE_SIZE);
            let total = state.matches.borrow().len();
            if start < total {
                state.start.set(start);
                state.render();
            }
        }
    });
    let weak = Rc::downgrade(&preview);
    preview.previous.connect_clicked(move |_| {
        if let Some(state) = weak.upgrade() {
            state.start.set(state.start.get().saturating_sub(PAGE_SIZE));
            state.render();
        }
    });
    root
}

impl Ui {
    pub(super) fn show_import_diagnostics(
        self: &Rc<Self>,
        title: &str,
        diagnostics: &[imports::CsvDiagnostic],
        truncated: bool,
    ) {
        let dialog = adw::Dialog::builder()
            .title(title)
            .content_width(680)
            .content_height(520)
            .build();
        let root = column();
        root.set_margin_top(16);
        root.set_margin_bottom(16);
        root.set_margin_start(16);
        root.set_margin_end(16);
        let digits = self.store.borrow().state.settings.display_digits;
        let summary = label(&format!(
            "{} خطا · چیزی وارد یا جایگزین نشده است",
            digits.format(diagnostics.len())
        ));
        summary.add_css_class("heading");
        root.append(&summary);
        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::None);
        list.add_css_class("boxed-list");
        list.add_css_class("manager-list");
        for diagnostic in diagnostics {
            let row = adw::ActionRow::builder()
                .title(format!(
                    "ردیف {} · {}",
                    digits.format(diagnostic.row),
                    diagnostic.column
                ))
                .subtitle(&diagnostic.reason)
                .title_lines(1)
                .subtitle_lines(3)
                .build();
            row.set_use_markup(false);
            row.set_tooltip_text(Some(&diagnostic.reason));
            list.append(&row);
        }
        root.append(&scroll(&list));
        if truncated {
            let limit =
                label("تعداد خطاها بیشتر است؛ پس از اصلاح موارد بالا فایل را دوباره بررسی کنید.");
            limit.add_css_class("caption");
            root.append(&limit);
        }
        let close = button("بستن و اصلاح فایل");
        root.append(&close);
        let weak = dialog.downgrade();
        close.connect_clicked(move |_| {
            if let Some(dialog) = weak.upgrade() {
                dialog.close();
            }
        });
        dialog.set_child(Some(&dialog_toolbar(&root)));
        self.present_dialog(&dialog);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_preview_reaches_every_product_without_more_than_100_rows() {
        for total in [0, 1, 99, 100, 101, 999, 10_000, 50_000] {
            let mut visited = Vec::new();
            let mut start = 0;
            loop {
                let range = page_range(total, start);
                assert!(range.len() <= PAGE_SIZE);
                let end = range.end;
                visited.extend(range);
                if end == total {
                    break;
                }
                start += PAGE_SIZE;
            }
            assert_eq!(visited, (0..total).collect::<Vec<_>>());
        }
    }

    #[test]
    fn catalogue_preview_handles_last_page_and_out_of_range_offsets() {
        assert_eq!(page_range(101, 100), 100..101);
        assert_eq!(page_range(101, 101), 101..101);
        assert_eq!(page_range(101, usize::MAX), 101..101);
        assert_eq!(page_range(0, usize::MAX), 0..0);
    }

    #[test]
    fn filtered_catalogue_review_matches_persian_aliases_without_mutating_prices() {
        let catalog = imports::catalog_csv(
            "code,name,measure,price,currency,source_at,aliases\nSKU-۱,کاغذ,بسته,۱۲۵,toman,2026-10-07T00:00:00Z,كاغذ آ۴\nSKU-2,جوهر,عدد,900,rial,2026-10-07T00:00:00Z,",
            "فهرست نمونه",
        ).unwrap();
        let before = serde_json::to_vec(&catalog).unwrap();
        assert_eq!(matching_indices(&catalog, "SKU-١"), vec![0]);
        assert_eq!(matching_indices(&catalog, "كاغذ آ٤"), vec![0]);
        assert_eq!(matching_indices(&catalog, "کاغذ SKU-۱"), vec![0]);
        assert!(matching_indices(&catalog, "کاغذ SKU-2").is_empty());
        assert_eq!(matching_indices(&catalog, ""), vec![0, 1]);
        assert_eq!(catalog.products[0].price_rial, 1250);
        assert_eq!(serde_json::to_vec(&catalog).unwrap(), before);
    }
}
