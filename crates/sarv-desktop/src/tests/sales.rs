use super::harness::{click, make_test_runtime};
use crate::app::{Page, Tone};

#[test]
fn test_sale_facture_pickers_headlessly() {
    let mut rt = make_test_runtime();
    click(&mut rt, Page::Sales.key());
    click(&mut rt, "btn-New sales invoice");
    assert!(rt.state.sales.modal_open);
    assert!(rt.find("sale-customer").is_some());
    assert!(rt.find("sale-item-0").is_some());
    assert!(rt.find("sale-pay-method").is_some());
    assert!(rt.find("sale-dialog").is_some());
    click(&mut rt, "sale-customer-trigger");
    for _ in 0..4 {
        rt.frame(1.0 / 60.0);
    }
    let (fx, fy) = rt.find("sale-customer-trigger").unwrap();
    let (mx, my) = rt.find("sale-customer-content").expect("customer menu");
    assert!(
        (fx - mx).abs() < 48.0,
        "customer menu should share the field's column: field {fx} menu {mx}"
    );
    assert!(
        (fy - my).abs() > 12.0 && (fy - my).abs() < 280.0,
        "customer menu should sit on the field: field {fy} menu {my}"
    );
    click(&mut rt, "sale-customer-trigger");
    // 10,000 toman must be stored as 100,000 rials, then delivered and settled.
    rt.state.fmt.unit = crate::i18n::Unit::Toman;
    rt.state.sales.customer_id = "CU-A".into();
    rt.state.sales.pay_method = 0;
    rt.state.sales.lines[0].item_id = "ITEM-A".into();
    rt.state.sales.lines[0].qty = "1".into();
    rt.state.sales.lines[0].price = "10000".into();
    rt.state.forms.set("sale-qty-0", "1");
    rt.state.forms.set("sale-price-0", "10000");
    let before = rt.state.books.list("sales").len();
    click(&mut rt, "btn-Complete sale");
    if rt.state.sales.modal_open {
        let msgs: Vec<_> = rt.state.toasts.iter().map(|t| format!("{:?}:{}", t.tone, t.message)).collect();
        panic!("sale still open; toasts={msgs:?}");
    }
    let sales = rt.state.books.list("sales");
    assert!(sales.len() > before, "fulfillment created a sales record");
    let dumped = serde_json::to_string(&sales[0]).unwrap_or_default();
    assert!(
        dumped.contains("100000") && !dumped.contains("1000000"),
        "toman price must not be multiplied twice: {dumped}"
    );
    assert!(
        rt.state.toasts.iter().any(|t| t.tone == Tone::Success),
        "posted only after delivery and payment"
    );
}

#[test]
fn test_display_quantity_is_not_thousandths() {
    assert_eq!(crate::ui::ops::qty_from_thousandths("1000"), "1");
    assert_eq!(crate::ui::ops::qty_from_thousandths("1500"), "1.5");
    assert_eq!(crate::ui::ops::typed_display_amount("۱۰٬۰۰۰"), "10000");
}

#[test]
fn test_rial_sale_is_not_scaled() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    click(&mut rt, Page::Sales.key());
    click(&mut rt, "btn-New sales invoice");
    rt.state.sales.customer_id = "CU-A".into();
    rt.state.sales.pay_method = 2;
    rt.state.sales.lines[0].item_id = "ITEM-A".into();
    rt.state.sales.lines[0].qty = "1".into();
    rt.state.sales.lines[0].price = "100000".into();
    rt.state.forms.set("sale-qty-0", "1");
    rt.state.forms.set("sale-price-0", "100000");
    click(&mut rt, "btn-Complete sale");
    if rt.state.sales.modal_open {
        let msgs: Vec<_> = rt.state.toasts.iter().map(|t| format!("{:?}:{}", t.tone, t.message)).collect();
        panic!("rial sale still open; toasts={msgs:?}");
    }
    let dumped = serde_json::to_string(&rt.state.books.list("sales")[0]).unwrap_or_default();
    assert!(dumped.contains("100000"), "rial amount stored once: {dumped}");
}
