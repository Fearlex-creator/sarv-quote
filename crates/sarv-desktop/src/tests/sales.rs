use super::harness::{assert_no_danger, blow, books_validate, click, make_test_runtime, type_into};
use crate::app::{Page, Tone};
use crate::data;

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

fn item_stock_total(rt: &mut rune::Runtime<crate::app::State>, item_id: &str) -> i64 {
    let detail = rt
        .state
        .query("product_detail", serde_json::json!({ "item_id": item_id, "as_of": data::today() }))
        .unwrap_or_else(|e| panic!("{e}"));
    data::s(&detail["stats"]["stock_quantity"]).parse::<i64>().unwrap_or(0)
}

fn customer_open_total(rt: &mut rune::Runtime<crate::app::State>, party: &str) -> i64 {
    let open = rt
        .state
        .query("payment_options", serde_json::json!({ "side": "receive", "party_id": party }))
        .unwrap_or_else(|e| panic!("{e}"));
    open["items"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|i| data::s(&i["remaining"]).parse::<i64>().unwrap_or(0))
        .sum()
}

#[test]
fn sale_return_restocks_and_credits_the_customer() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let party = rt
        .state
        .create_party(crate::ui::resources::ResourceKind::Customer, "Return Buyer", "", "", "")
        .unwrap_or_else(|e| panic!("{e}"));

    let stock_before = item_stock_total(&mut rt, "ITEM-A");
    click(&mut rt, Page::Sales.key());
    click(&mut rt, "btn-New sales invoice");
    // Combo/select pickers: set customer, item, and payment method directly.
    rt.state.sales.customer_id = party.clone();
    rt.state.sales.pay_method = 2;
    rt.state.sales.lines[0].item_id = "ITEM-A".into();
    rt.state.sales.lines[0].qty = "1".into();
    rt.state.sales.lines[0].price = "150000".into();
    type_into(&mut rt, "sale-qty-0", "1");
    type_into(&mut rt, "sale-price-0", "150000");
    click(&mut rt, "btn-Complete sale");
    assert!(!rt.state.sales.modal_open, "{}", blow(&rt, "credit sale"));

    assert_eq!(item_stock_total(&mut rt, "ITEM-A"), stock_before - 1000);
    assert_eq!(customer_open_total(&mut rt, &party), 150000);

    let invoice_id = rt
        .state
        .books
        .list("sales")
        .into_iter()
        .find(|r| r["command"].get("Fulfill").is_some())
        .map(|r| data::s(&r["id"]))
        .unwrap_or_else(|| panic!("{}", blow(&rt, "invoice id")));

    click(&mut rt, &format!("sale-{invoice_id}"));
    click(&mut rt, "btn-Return");
    click(&mut rt, "btn-Post return");

    assert_eq!(item_stock_total(&mut rt, "ITEM-A"), stock_before, "stock restored on return");
    assert_eq!(customer_open_total(&mut rt, &party), 0, "receivable credited on return");
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn opening_the_sale_dialog_twice_starts_clean() {
    let mut rt = make_test_runtime();
    click(&mut rt, Page::Sales.key());
    click(&mut rt, "btn-New sales invoice");
    let first_key = rt.state.sales.post_key.clone();
    assert!(!first_key.is_empty());

    rt.state.sales.customer_id = "CU-A".into();
    rt.state.sales.lines[0].item_id = "ITEM-A".into();
    type_into(&mut rt, "sale-qty-0", "3");
    type_into(&mut rt, "sale-price-0", "50000");
    click(&mut rt, "btn-Cancel");
    click(&mut rt, "btn-Discard");
    assert!(!rt.state.sales.modal_open, "{}", blow(&rt, "cancel sale dialog"));

    click(&mut rt, "btn-New sales invoice");
    assert!(rt.state.sales.modal_open);
    assert_ne!(rt.state.sales.post_key, first_key, "new dialog must mint a fresh post_key");
    assert!(rt.state.sales.customer_id.is_empty(), "customer_id must be cleared");
    assert!(rt.state.sales.lines[0].item_id.is_empty(), "line item_id must be cleared");
    assert_ne!(rt.state.forms.get("sale-qty-0"), "3", "leftover sale-qty-0 must be cleared");
    assert!(
        rt.state.forms.get("sale-price-0").is_empty(),
        "leftover sale-price-0 must be cleared"
    );
    assert_no_danger(&rt);
}

#[test]
fn test_walk_in_customer_is_reused_across_languages() {
    let mut rt = make_test_runtime();
    rt.state.set_lang(crate::i18n::Lang::Fa);
    let fa_id = rt.state.ensure_walk_in_customer().expect("walk-in in Fa");

    rt.state.set_lang(crate::i18n::Lang::En);
    let en_id = rt.state.ensure_walk_in_customer().expect("walk-in in En");

    assert_eq!(fa_id, en_id, "walk-in customer should be reused across languages");
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn test_new_sale_does_not_inherit_line_fields() {
    let mut rt = make_test_runtime();
    rt.state.forms.set("sale-qty-0", "7");
    crate::ui::views::sales::open_sale(&mut rt.state);
    assert!(rt.state.forms.get("sale-qty-0").is_empty());
}

#[test]
fn test_return_requires_confirmation() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let party = rt
        .state
        .create_party(crate::ui::resources::ResourceKind::Customer, "Confirm Return Buyer", "", "", "")
        .unwrap_or_else(|e| panic!("{e}"));

    click(&mut rt, Page::Sales.key());
    click(&mut rt, "btn-New sales invoice");
    rt.state.sales.customer_id = party;
    rt.state.sales.pay_method = 2;
    rt.state.sales.lines[0].item_id = "ITEM-A".into();
    rt.state.sales.lines[0].qty = "1".into();
    rt.state.sales.lines[0].price = "100000".into();
    type_into(&mut rt, "sale-qty-0", "1");
    type_into(&mut rt, "sale-price-0", "100000");
    click(&mut rt, "btn-Complete sale");
    assert!(!rt.state.sales.modal_open, "{}", blow(&rt, "sale before return"));

    let sales_before = rt.state.books.list("sales").len();
    let invoice_id = rt
        .state
        .books
        .list("sales")
        .into_iter()
        .find(|r| r["command"].get("Fulfill").is_some())
        .map(|r| data::s(&r["id"]))
        .unwrap_or_else(|| panic!("{}", blow(&rt, "invoice id")));

    click(&mut rt, &format!("sale-{invoice_id}"));
    click(&mut rt, "btn-Return");
    assert!(rt.state.confirm.is_some(), "return must open confirmation dialog");
    assert_eq!(rt.state.books.list("sales").len(), sales_before, "no return posted before confirm");

    click(&mut rt, "btn-Post return");
    assert!(rt.state.confirm.is_none(), "confirm must close after posting");
    assert_eq!(rt.state.books.list("sales").len(), sales_before + 1, "return posted after confirm");
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn test_dirty_sale_survives_cancel() {
    let mut rt = make_test_runtime();
    click(&mut rt, Page::Sales.key());
    click(&mut rt, "btn-New sales invoice");
    assert!(rt.state.sales.modal_open);

    rt.state.sales.lines[0].item_id = "ITEM-A".into();
    type_into(&mut rt, "sale-price-0", "50000");

    click(&mut rt, "btn-Cancel");
    assert!(rt.state.confirm.is_some(), "dirty sale must ask before discarding");
    assert!(rt.state.sales.modal_open, "sale dialog must remain open until Discard is clicked");

    click(&mut rt, "btn-Discard");
    assert!(rt.state.confirm.is_none());
    assert!(!rt.state.sales.modal_open, "sale dialog must close after Discard");
    assert_no_danger(&rt);
}

#[test]
fn test_backdated_sale_chain_uses_one_date() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;

    let (start, _) = rt.state.books.range();
    let backdate_iso = if start != data::today() {
        start
    } else {
        chrono::NaiveDate::parse_from_str(&start, "%Y-%m-%d")
            .unwrap()
            .checked_add_days(chrono::Days::new(1))
            .unwrap()
            .format("%Y-%m-%d")
            .to_string()
    };
    let typed_jalali = rt.state.fmt.date_input(&backdate_iso);

    let sales_before = rt.state.books.list("sales").len();
    let payments_before = rt.state.books.list("payments").len();

    click(&mut rt, Page::Sales.key());
    click(&mut rt, "btn-New sales invoice");
    rt.state.sales.customer_id = "CU-A".into();
    rt.state.sales.pay_method = 0;
    rt.state.sales.lines[0].item_id = "ITEM-A".into();
    rt.state.sales.lines[0].qty = "1".into();
    rt.state.sales.lines[0].price = "50000".into();
    type_into(&mut rt, "sale-qty-0", "1");
    type_into(&mut rt, "sale-price-0", "50000");
    type_into(&mut rt, "sale-date", &typed_jalali);
    click(&mut rt, "btn-Complete sale");
    assert!(!rt.state.sales.modal_open, "{}", blow(&rt, "backdated sale"));

    let sales = rt.state.books.list("sales");
    assert!(sales.len() >= sales_before + 2, "order and fulfill records created");
    for rec in &sales[..sales.len() - sales_before] {
        let cmd = &rec["command"];
        let doc_date = if cmd.get("Order").is_some() {
            data::s(&cmd["Order"]["document_date"])
        } else if cmd.get("Fulfill").is_some() {
            data::s(&cmd["Fulfill"]["date"])
        } else {
            String::new()
        };
        if !doc_date.is_empty() {
            assert_eq!(doc_date, backdate_iso, "sale chain record must use backdate: {rec}");
        }
    }

    let payments = rt.state.books.list("payments");
    assert_eq!(payments.len(), payments_before + 1, "settlement payment created");
    let newest_pay = payments
        .iter()
        .find(|p| data::s(&p["command"]["Payment"]["context"]["date"]) == backdate_iso);
    assert!(newest_pay.is_some(), "settlement payment must use {backdate_iso}: {payments:?}");
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn test_sales_table_sorts_by_amount() {
    use crate::ui::rows::SortKey;
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    click(&mut rt, Page::Sales.key());

    for price in ["10000", "50000"] {
        click(&mut rt, "btn-New sales invoice");
        rt.state.sales.customer_id = "CU-A".into();
        rt.state.sales.pay_method = 0;
        rt.state.sales.lines[0].item_id = "ITEM-A".into();
        rt.state.forms.set("sale-qty-0", "1");
        rt.state.forms.set("sale-price-0", price);
        click(&mut rt, "btn-Complete sale");
        assert!(!rt.state.sales.modal_open, "{}", blow(&rt, "sale post"));
    }

    assert!(rt.state.sales.rows.len() >= 2);
    crate::ui::views::sales::apply_sort(&mut rt.state, "amount", rune_kit::SortDir::Descending);
    rt.frame(1.0 / 60.0);
    let amounts: Vec<i128> = rt
        .state
        .sales
        .rows
        .iter()
        .filter_map(|r| match r.keys.get(3) {
            Some(SortKey::Num(n)) => Some(*n),
            _ => None,
        })
        .collect();
    assert!(amounts.windows(2).all(|w| w[0] >= w[1]), "expected descending amounts: {amounts:?}");

    crate::ui::views::sales::apply_sort(&mut rt.state, "amount", rune_kit::SortDir::Ascending);
    rt.frame(1.0 / 60.0);
    let amounts: Vec<i128> = rt
        .state
        .sales
        .rows
        .iter()
        .filter_map(|r| match r.keys.get(3) {
            Some(SortKey::Num(n)) => Some(*n),
            _ => None,
        })
        .collect();
    assert!(amounts.windows(2).all(|w| w[0] <= w[1]), "expected ascending amounts: {amounts:?}");
}
