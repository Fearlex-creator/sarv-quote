use super::harness::{assert_no_danger, blow, books_validate, click, make_test_runtime, type_into};
use crate::app::Page;
use crate::data;

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
    assert!(rt.find("journal-account-0").is_some());
    assert!(rt.find("journal-account-1").is_some());
    click(&mut rt, "journal-account-0-trigger");
    for _ in 0..4 {
        rt.frame(1.0 / 60.0);
    }
    let (fx, fy) = rt.find("journal-account-0-trigger").unwrap();
    let (mx, my) = rt.find("journal-account-0-content").expect("account menu");
    assert!((fx - mx).abs() < 160.0, "account menu should cover the field: field {fx} menu {mx}");
    assert!(
        (fy - my).abs() > 12.0 && (fy - my).abs() < 280.0,
        "account menu should sit on the field: field {fy} menu {my}"
    );
    rt.state.accounting.modal_open = false;
    rt.frame(1.0 / 60.0);
}

fn item_stock_total(rt: &mut rune::Runtime<crate::app::State>, item_id: &str) -> i64 {
    let detail = rt
        .state
        .query("product_detail", serde_json::json!({ "item_id": item_id, "as_of": data::today() }))
        .unwrap_or_else(|e| panic!("{e}"));
    data::s(&detail["stats"]["stock_quantity"]).parse::<i64>().unwrap_or(0)
}

fn supplier_open_total(rt: &mut rune::Runtime<crate::app::State>, party: &str) -> i64 {
    let open = rt
        .state
        .query("payment_options", serde_json::json!({ "side": "make", "party_id": party }))
        .unwrap_or_else(|e| panic!("{e}"));
    open["items"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|i| data::s(&i["remaining"]).parse::<i64>().unwrap_or(0))
        .sum()
}

#[test]
fn purchase_receive_and_bill_updates_stock_and_payables() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let supplier = rt
        .state
        .create_party(crate::ui::resources::ResourceKind::Vendor, "PO Supplier", "", "", "")
        .unwrap_or_else(|e| panic!("{e}"));

    let stock_before = item_stock_total(&mut rt, "ITEM-A");
    let payables_before = rt.state.books.position("payables").parse::<i64>().unwrap();

    click(&mut rt, Page::Purchases.key());
    click(&mut rt, "btn-New purchase order");
    // Combo/select pickers: set supplier, item, and receive_and_bill directly.
    rt.state.purchases.supplier_id = supplier.clone();
    rt.state.purchases.receive_and_bill = true;
    rt.state.purchases.lines[0].item_id = "ITEM-A".into();
    rt.state.purchases.lines[0].qty = "2".into();
    rt.state.purchases.lines[0].price = "80000".into();
    type_into(&mut rt, "buy-qty-0", "2");
    type_into(&mut rt, "buy-price-0", "80000");
    click(&mut rt, "btn-Save");
    assert!(!rt.state.purchases.modal_open, "{}", blow(&rt, "purchase save"));

    assert_eq!(item_stock_total(&mut rt, "ITEM-A"), stock_before + 2000);
    assert_eq!(
        rt.state.books.position("payables").parse::<i64>().unwrap(),
        payables_before + 160000
    );
    assert_eq!(supplier_open_total(&mut rt, &supplier), 160000);
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn purchase_return_reduces_stock_and_payables() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let supplier = rt
        .state
        .create_party(crate::ui::resources::ResourceKind::Vendor, "Return Supplier", "", "", "")
        .unwrap_or_else(|e| panic!("{e}"));

    let stock_before = item_stock_total(&mut rt, "ITEM-A");
    let payables_before = rt.state.books.position("payables").parse::<i64>().unwrap();

    click(&mut rt, Page::Purchases.key());
    click(&mut rt, "btn-New purchase order");
    rt.state.purchases.supplier_id = supplier.clone();
    rt.state.purchases.receive_and_bill = true;
    rt.state.purchases.lines[0].item_id = "ITEM-A".into();
    rt.state.purchases.lines[0].qty = "1".into();
    rt.state.purchases.lines[0].price = "80000".into();
    type_into(&mut rt, "buy-qty-0", "1");
    type_into(&mut rt, "buy-price-0", "80000");
    click(&mut rt, "btn-Save");
    assert!(!rt.state.purchases.modal_open, "{}", blow(&rt, "purchase save"));

    let purchases = rt.state.books.list("purchases");
    let bill = purchases
        .iter()
        .find(|r| r["command"].get("Bill").is_some())
        .map(|r| data::s(&r["id"]));
    let receipt = purchases
        .iter()
        .find(|r| r["command"].get("Receive").is_some())
        .map(|r| data::s(&r["id"]));
    let (return_id, tab) = bill
        .filter(|id| !id.is_empty())
        .map(|id| (id, 2))
        .or_else(|| receipt.filter(|id| !id.is_empty()).map(|id| (id, 1)))
        .unwrap_or_else(|| panic!("{}", blow(&rt, "receipt or bill")));

    click(&mut rt, &format!("segment{tab}"));
    for _ in 0..8 {
        rt.frame(1.0 / 60.0);
    }
    click(&mut rt, &format!("purchase-{return_id}"));
    click(&mut rt, "btn-Return");
    assert!(rt.state.operation.open, "{}", blow(&rt, "open return dialog"));
    click(&mut rt, "btn-Review");
    click(&mut rt, "btn-Post return");

    assert_eq!(item_stock_total(&mut rt, "ITEM-A"), stock_before, "stock reduced back");
    assert_eq!(
        rt.state.books.position("payables").parse::<i64>().unwrap(),
        payables_before,
        "payables reduced back"
    );
    assert_eq!(supplier_open_total(&mut rt, &supplier), 0);
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn test_open_purchase_order_can_be_received_later() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let supplier = rt
        .state
        .create_party(crate::ui::resources::ResourceKind::Vendor, "Later Supplier", "", "", "")
        .unwrap_or_else(|e| panic!("{e}"));

    let stock_before = item_stock_total(&mut rt, "ITEM-A");

    click(&mut rt, Page::Purchases.key());
    click(&mut rt, "btn-New purchase order");
    rt.state.purchases.supplier_id = supplier;
    rt.state.purchases.receive_and_bill = false;
    rt.state.purchases.lines[0].item_id = "ITEM-A".into();
    rt.state.purchases.lines[0].qty = "2".into();
    rt.state.purchases.lines[0].price = "80000".into();
    type_into(&mut rt, "buy-qty-0", "2");
    type_into(&mut rt, "buy-price-0", "80000");
    click(&mut rt, "btn-Save");
    assert!(!rt.state.purchases.modal_open, "{}", blow(&rt, "open PO save"));
    assert_eq!(
        item_stock_total(&mut rt, "ITEM-A"),
        stock_before,
        "open PO must not change stock yet"
    );

    let order_id = rt
        .state
        .books
        .list("purchases")
        .into_iter()
        .find(|r| r["command"].get("Order").is_some())
        .map(|r| data::s(&r["id"]))
        .unwrap_or_else(|| panic!("{}", blow(&rt, "order id")));

    click(&mut rt, &format!("purchase-{order_id}"));
    click(&mut rt, "btn-Receive goods");
    assert!(rt.state.operation.open, "{}", blow(&rt, "receive dialog"));
    click(&mut rt, "btn-Review");
    assert!(rt.state.confirm.is_some(), "{}", blow(&rt, "receive confirm"));
    click(&mut rt, "btn-Post");
    assert!(!rt.state.operation.open, "{}", blow(&rt, "receive post"));

    assert_eq!(item_stock_total(&mut rt, "ITEM-A"), stock_before + 2000);
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn test_partial_receive_leaves_a_remainder() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let supplier = rt
        .state
        .create_party(crate::ui::resources::ResourceKind::Vendor, "Partial Supplier", "", "", "")
        .unwrap_or_else(|e| panic!("{e}"));

    let stock_before = item_stock_total(&mut rt, "ITEM-A");

    click(&mut rt, Page::Purchases.key());
    click(&mut rt, "btn-New purchase order");
    rt.state.purchases.supplier_id = supplier;
    rt.state.purchases.receive_and_bill = false;
    rt.state.purchases.lines[0].item_id = "ITEM-A".into();
    rt.state.purchases.lines[0].qty = "3".into();
    rt.state.purchases.lines[0].price = "80000".into();
    type_into(&mut rt, "buy-qty-0", "3");
    type_into(&mut rt, "buy-price-0", "80000");
    click(&mut rt, "btn-Save");
    assert!(!rt.state.purchases.modal_open, "{}", blow(&rt, "partial PO save"));

    let order_id = rt
        .state
        .books
        .list("purchases")
        .into_iter()
        .find(|r| r["command"].get("Order").is_some())
        .map(|r| data::s(&r["id"]))
        .unwrap_or_else(|| panic!("{}", blow(&rt, "order id")));

    click(&mut rt, &format!("purchase-{order_id}"));
    click(&mut rt, "btn-Receive goods");
    assert!(rt.state.operation.open);
    assert_eq!(rt.state.forms.get("op-qty-0"), "3");
    type_into(&mut rt, "op-qty-0", "1");
    click(&mut rt, "btn-Review");
    click(&mut rt, "btn-Post");
    assert!(!rt.state.operation.open, "{}", blow(&rt, "partial receive post"));
    assert_eq!(item_stock_total(&mut rt, "ITEM-A"), stock_before + 1000);

    click(&mut rt, "btn-Receive goods");
    assert!(rt.state.operation.open, "{}", blow(&rt, "reopen receive"));
    assert_eq!(rt.state.forms.get("op-qty-0"), "2");
    assert_eq!(rt.state.operation.lines[0].remaining_thousandths, "2000");
    rt.state.operation.open = false;
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn test_quantity_above_remaining_is_rejected_before_preview() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let supplier = rt
        .state
        .create_party(crate::ui::resources::ResourceKind::Vendor, "Over Supplier", "", "", "")
        .unwrap_or_else(|e| panic!("{e}"));

    click(&mut rt, Page::Purchases.key());
    click(&mut rt, "btn-New purchase order");
    rt.state.purchases.supplier_id = supplier;
    rt.state.purchases.receive_and_bill = false;
    rt.state.purchases.lines[0].item_id = "ITEM-A".into();
    rt.state.purchases.lines[0].qty = "2".into();
    rt.state.purchases.lines[0].price = "80000".into();
    type_into(&mut rt, "buy-qty-0", "2");
    type_into(&mut rt, "buy-price-0", "80000");
    click(&mut rt, "btn-Save");
    assert!(!rt.state.purchases.modal_open);

    let order_id = rt
        .state
        .books
        .list("purchases")
        .into_iter()
        .find(|r| r["command"].get("Order").is_some())
        .map(|r| data::s(&r["id"]))
        .unwrap_or_else(|| panic!("{}", blow(&rt, "order id")));

    click(&mut rt, &format!("purchase-{order_id}"));
    click(&mut rt, "btn-Receive goods");
    assert!(rt.state.operation.open);
    type_into(&mut rt, "op-qty-0", "5");
    click(&mut rt, "btn-Review");
    assert!(rt.state.confirm.is_none(), "over-remaining quantity must not open confirm");
    assert!(
        rt.state.toasts.iter().any(|t| t.tone == crate::app::Tone::Danger),
        "expected danger toast for quantity exceeding remaining"
    );
    rt.state.operation.open = false;
}
