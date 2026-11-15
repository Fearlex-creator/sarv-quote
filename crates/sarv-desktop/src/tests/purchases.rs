use super::harness::{click, make_test_runtime};
use crate::app::Page;

#[test]
fn test_purchase_and_journal_dialogs_headlessly() {
    let mut rt = make_test_runtime();
    click(&mut rt, Page::Purchases.key());
    click(&mut rt, "btn-New purchase order");
    assert!(rt.state.purchases.modal_open);
    assert!(rt.find("buy-supplier").is_some());
    assert!(rt.find("buy-item-0").is_some());
    rt.state.purchases.modal_open = false;
    click(&mut rt, "refresh-books");

    click(&mut rt, Page::Accounting.key());
    click(&mut rt, "btn-New journal");
    assert!(rt.state.accounting.modal_open);
    assert!(rt.find("journal-debit").is_some());
    assert!(rt.find("journal-credit").is_some());
    click(&mut rt, "journal-debit-trigger");
    for _ in 0..4 {
        rt.frame(1.0 / 60.0);
    }
    let (fx, fy) = rt.find("journal-debit-trigger").unwrap();
    let (mx, my) = rt.find("journal-debit-content").expect("account menu");
    assert!((fx - mx).abs() < 160.0, "account menu should cover the field: field {fx} menu {mx}");
    assert!(
        (fy - my).abs() > 12.0 && (fy - my).abs() < 280.0,
        "account menu should sit on the field: field {fy} menu {my}"
    );
    rt.state.accounting.modal_open = false;
    rt.frame(1.0 / 60.0);
}
