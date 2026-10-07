//! Read-only, bounded rendering of a validated catalogue import.
//! Paging changes the visible widgets only; approval still accepts the complete
//! validated catalogue and no products or source prices are rewritten here.
use super::*;
use std::ops::Range;

const PAGE_SIZE: usize = 100;

fn page_range(total: usize, start: usize) -> Range<usize> {
    let start = start.min(total);
    start..start.saturating_add(PAGE_SIZE).min(total)
}

fn render_page(
    catalog: &Catalog,
    start: usize,
    list: &gtk::Box,
    count: &gtk::Label,
    previous: &gtk::Button,
    next: &gtk::Button,
) {
    let range = page_range(catalog.products.len(), start);
    clear(list);
    for p in &catalog.products[range.clone()] {
        list.append(&label(&format!(
            "{} • {} • {} / {}\nمنبع: {} • {}{}\nنام مستعار: {}",
            ltr(&p.code),
            p.name,
            Unit::Rial.display(p.price_rial),
            p.measure,
            ltr(&p.source_at),
            p.source_unit.label(),
            if stale(&p.source_at) {
                " • قیمت قدیمی"
            } else {
                ""
            },
            p.aliases.join("، ")
        )));
    }
    count.set_text(&format!(
        "نمایش {} تا {} از {} کالا • همهٔ کالاهای اعتبارسنجی‌شده در این پیش‌نمایش قابل بررسی‌اند",
        fa(if range.is_empty() { 0 } else { range.start + 1 }),
        fa(range.end),
        fa(catalog.products.len())
    ));
    previous.set_sensitive(range.start > 0);
    next.set_sensitive(range.end < catalog.products.len());
}

pub(super) fn preview(catalog: &Catalog) -> gtk::Box {
    let root = column();
    root.set_vexpand(true);
    let count = label("");
    count.set_selectable(false);
    count.add_css_class("caption");
    root.append(&count);
    let list = column();
    let scroller = scroll(&list);
    scroller.set_min_content_height(220);
    root.append(&scroller);
    let navigation = row();
    let previous = button("۱۰۰ کالای قبلی");
    previous.set_tooltip_text(Some("بررسی صفحهٔ پیشین قیمت‌ها؛ بدون تغییر دادهٔ واردشده"));
    let next = button("نمایش ۱۰۰ کالای بعدی");
    next.set_tooltip_text(Some(
        "بررسی صفحهٔ بعدی قیمت‌ها؛ تا آخرین کالای فایل اعتبارسنجی‌شده",
    ));
    navigation.append(&previous);
    navigation.append(&next);
    root.append(&navigation);

    let catalog = Rc::new(catalog.clone());
    let start = Rc::new(Cell::new(0usize));
    render_page(&catalog, start.get(), &list, &count, &previous, &next);

    // Weak widget references keep closed import dialogs reclaimable: neither
    // button's signal closure owns itself or the other navigation button.
    let previous_weak = previous.downgrade();
    let list_weak = list.downgrade();
    let count_weak = count.downgrade();
    let scroll_weak = scroller.downgrade();
    let data = catalog.clone();
    let offset = start.clone();
    next.connect_clicked(move |next| {
        if let (Some(list), Some(count), Some(previous)) = (
            list_weak.upgrade(),
            count_weak.upgrade(),
            previous_weak.upgrade(),
        ) {
            let next_start = offset.get().saturating_add(PAGE_SIZE);
            if next_start < data.products.len() {
                offset.set(next_start);
                render_page(&data, next_start, &list, &count, &previous, next);
                if let Some(scroll) = scroll_weak.upgrade() {
                    scroll.vadjustment().set_value(0.0);
                }
            }
        }
    });

    let next_weak = next.downgrade();
    let list_weak = list.downgrade();
    let count_weak = count.downgrade();
    let scroll_weak = scroller.downgrade();
    previous.connect_clicked(move |previous| {
        if let (Some(list), Some(count), Some(next)) = (
            list_weak.upgrade(),
            count_weak.upgrade(),
            next_weak.upgrade(),
        ) {
            let previous_start = start.get().saturating_sub(PAGE_SIZE);
            if previous_start != start.get() {
                start.set(previous_start);
                render_page(&catalog, previous_start, &list, &count, previous, &next);
                if let Some(scroll) = scroll_weak.upgrade() {
                    scroll.vadjustment().set_value(0.0);
                }
            }
        }
    });
    root
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
}
