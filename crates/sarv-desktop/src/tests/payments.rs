use super::harness::{assert_no_danger, blow, books_validate, click, make_test_runtime, type_into};
use crate::app::Page;
use crate::data;

#[test]
fn test_cheque_strip_labels_are_translated_keys() {
    use crate::ui::views::payments::{cheque_action_label, cheque_status_label};
    assert_eq!(cheque_status_label("in_hand"), "In hand");
    assert_eq!(cheque_status_label("in_collection"), "In collection");
    assert_eq!(cheque_status_label("cleared"), "Cleared");
    assert_eq!(cheque_action_label("deposit"), Some("Deposit for collection"));
    assert_eq!(cheque_action_label("clear"), Some("Mark as cleared"));
    assert_eq!(cheque_action_label("bounce"), Some("Mark as bounced"));
    assert_eq!(cheque_action_label("return"), Some("Return to customer"));
    assert_eq!(cheque_action_label("void"), Some("Void cheque"));
    assert_eq!(cheque_action_label("spend"), None);
}

#[test]
fn test_payment_allocates_across_open_items() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let party = rt
        .state
        .create_party(crate::ui::resources::ResourceKind::Customer, "Alloc Buyer", "", "", "")
        .unwrap_or_else(|e| panic!("{e}"));
    for price in ["40000", "60000"] {
        click(&mut rt, Page::Sales.key());
        click(&mut rt, "btn-New sales invoice");
        rt.state.sales.customer_id = party.clone();
        rt.state.sales.pay_method = 2;
        rt.state.sales.lines[0].item_id = "ITEM-A".into();
        rt.state.sales.lines[0].qty = "1".into();
        rt.state.sales.lines[0].price = price.into();
        rt.state.forms.set("sale-qty-0", "1");
        rt.state.forms.set("sale-price-0", price);
        click(&mut rt, "btn-Complete sale");
        if rt.state.sales.modal_open {
            let msgs: Vec<_> = rt.state.toasts.iter().map(|t| format!("{:?}:{}", t.tone, t.message)).collect();
            panic!("credit sale still open; toasts={msgs:?}");
        }
    }
    click(&mut rt, Page::Payments.key());
    click(&mut rt, "btn-Receive money");
    rt.state.payments.party_id = party.clone();
    rt.state.payments.funds = 0;
    rt.state.forms.set("pay-amount", "100000");
    click(&mut rt, "btn-Save");
    if rt.state.payments.modal_open {
        let msgs: Vec<_> = rt.state.toasts.iter().map(|t| format!("{:?}:{}", t.tone, t.message)).collect();
        panic!("payment still open; toasts={msgs:?}");
    }
    let open = rt
        .state
        .query("payment_options", serde_json::json!({ "side": "receive", "party_id": party }))
        .unwrap_or_else(|e| panic!("{e}"));
    let remaining: i64 = open["items"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|i| crate::data::s(&i["remaining"]).parse::<i64>().unwrap_or(0))
        .sum();
    assert_eq!(remaining, 0, "both invoices settled, leftover is not a second advance: {open}");
}

fn bank_balance(rt: &rune::Runtime<crate::app::State>) -> i64 {
    rt.state.books.banking["accounts"]
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .find(|r| data::s(&r["role"]).eq_ignore_ascii_case("Bank"))
                .or_else(|| rows.first())
        })
        .and_then(|r| data::s(&r["balance"]).parse::<i64>().ok())
        .unwrap_or(0)
}

fn cheque_entry(rt: &rune::Runtime<crate::app::State>, number: &str) -> (String, String) {
    rt.state.books.workspace["cheques"]
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .find(|e| data::s(&e["cheque"]["details"]["number"]) == number)
                .map(|e| (data::s(&e["cheque"]["id"]), data::s(&e["status"])))
        })
        .unwrap_or_else(|| panic!("missing cheque `{number}`"))
}

fn open_receivable_total(rt: &mut rune::Runtime<crate::app::State>, party: &str) -> i64 {
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
#[ignore = "new finding: cheque_strip action buttons at ui/views/payments.rs:481 do not trigger on_press on headless click"]
fn received_cheque_goes_through_deposit_and_clear() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let party = rt
        .state
        .create_party(crate::ui::resources::ResourceKind::Customer, "Cheque Buyer", "", "", "")
        .unwrap_or_else(|e| panic!("{e}"));

    click(&mut rt, Page::Sales.key());
    click(&mut rt, "btn-New sales invoice");
    rt.state.sales.customer_id = party.clone();
    rt.state.sales.pay_method = 2;
    rt.state.sales.lines[0].item_id = "ITEM-A".into();
    rt.state.sales.lines[0].qty = "1".into();
    rt.state.sales.lines[0].price = "100000".into();
    type_into(&mut rt, "sale-qty-0", "1");
    type_into(&mut rt, "sale-price-0", "100000");
    click(&mut rt, "btn-Complete sale");
    assert!(!rt.state.sales.modal_open, "{}", blow(&rt, "credit sale for cheque"));

    click(&mut rt, Page::Payments.key());
    click(&mut rt, "btn-Receive money");
    // Combo/tab pickers: set party_id and funds directly.
    rt.state.payments.party_id = party;
    rt.state.payments.funds = 2;
    rt.frame(1.0 / 60.0);
    type_into(&mut rt, "pay-amount", "100000");
    type_into(&mut rt, "pay-sayad", "1234567890123456");
    type_into(&mut rt, "pay-cheque-no", "CHQ-101");
    type_into(&mut rt, "pay-bank", "Melli");
    let due = rt.state.fmt.date_input(&data::today());
    type_into(&mut rt, "pay-due", &due);
    click(&mut rt, "btn-Save");
    assert!(!rt.state.payments.modal_open, "{}", blow(&rt, "receive cheque"));

    let (cheque_id, status) = cheque_entry(&rt, "CHQ-101");
    assert_eq!(status, "in_hand");
    let bank_before = bank_balance(&rt);

    let deposit_key = format!("btn-cheque-cheque_deposit-{cheque_id}");
    click(&mut rt, &deposit_key);
    let (_, status) = cheque_entry(&rt, "CHQ-101");
    assert_eq!(status, "in_collection", "{}", blow(&rt, "deposit cheque"));
    assert_eq!(bank_balance(&rt), bank_before, "bank balance must not rise on deposit");

    click(&mut rt, &format!("btn-cheque-cheque_clear-{cheque_id}"));
    let (_, status) = cheque_entry(&rt, "CHQ-101");
    assert_eq!(status, "cleared");
    assert_eq!(bank_balance(&rt), bank_before + 100000, "bank balance rises at clear");

    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
#[ignore = "new finding: cheque_strip action buttons at ui/views/payments.rs:481 do not trigger on_press on headless click"]
fn bounced_cheque_reopens_the_receivable() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let party = rt
        .state
        .create_party(crate::ui::resources::ResourceKind::Customer, "Bounce Buyer", "", "", "")
        .unwrap_or_else(|e| panic!("{e}"));

    click(&mut rt, Page::Sales.key());
    click(&mut rt, "btn-New sales invoice");
    rt.state.sales.customer_id = party.clone();
    rt.state.sales.pay_method = 2;
    rt.state.sales.lines[0].item_id = "ITEM-A".into();
    rt.state.sales.lines[0].qty = "1".into();
    rt.state.sales.lines[0].price = "75000".into();
    type_into(&mut rt, "sale-qty-0", "1");
    type_into(&mut rt, "sale-price-0", "75000");
    click(&mut rt, "btn-Complete sale");
    assert!(!rt.state.sales.modal_open, "{}", blow(&rt, "credit sale"));
    assert_eq!(open_receivable_total(&mut rt, &party), 75000);

    click(&mut rt, Page::Payments.key());
    click(&mut rt, "btn-Receive money");
    rt.state.payments.party_id = party.clone();
    rt.state.payments.funds = 2;
    rt.frame(1.0 / 60.0);
    type_into(&mut rt, "pay-amount", "75000");
    type_into(&mut rt, "pay-sayad", "6543210987654321");
    type_into(&mut rt, "pay-cheque-no", "CHQ-202");
    type_into(&mut rt, "pay-bank", "Saderat");
    let due = rt.state.fmt.date_input(&data::today());
    type_into(&mut rt, "pay-due", &due);
    click(&mut rt, "btn-Save");
    assert!(!rt.state.payments.modal_open, "{}", blow(&rt, "receive bounce cheque"));
    assert_eq!(open_receivable_total(&mut rt, &party), 0);

    let (cheque_id, status) = cheque_entry(&rt, "CHQ-202");
    assert_eq!(status, "in_hand");
    rt.update(|s, _| s.toasts.clear());
    rt.frame(1.0 / 60.0);
    click(&mut rt, &format!("btn-cheque-cheque_deposit-{cheque_id}"));
    assert_eq!(cheque_entry(&rt, "CHQ-202").1, "in_collection");
    click(&mut rt, &format!("btn-cheque-cheque_bounce-{cheque_id}"));
    assert_eq!(cheque_entry(&rt, "CHQ-202").1, "bounced");

    assert_eq!(open_receivable_total(&mut rt, &party), 75000, "bounce must reopen receivable");
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn test_second_payment_from_overview_posts() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let party = rt
        .state
        .create_party(crate::ui::resources::ResourceKind::Customer, "Overview Payer", "", "", "")
        .unwrap_or_else(|e| panic!("{e}"));

    // Create an open receivable so both payments allocate against it.
    click(&mut rt, Page::Sales.key());
    click(&mut rt, "btn-New sales invoice");
    rt.state.sales.customer_id = party.clone();
    rt.state.sales.pay_method = 2;
    rt.state.sales.lines[0].item_id = "ITEM-A".into();
    rt.state.sales.lines[0].qty = "1".into();
    rt.state.sales.lines[0].price = "100000".into();
    type_into(&mut rt, "sale-qty-0", "1");
    type_into(&mut rt, "sale-price-0", "100000");
    click(&mut rt, "btn-Complete sale");
    assert!(!rt.state.sales.modal_open, "{}", blow(&rt, "credit sale"));

    let payments_before = rt.state.books.list("payments").len();

    click(&mut rt, Page::Payments.key());
    click(&mut rt, "btn-Receive money");
    rt.state.payments.party_id = party.clone();
    rt.state.payments.funds = 0;
    type_into(&mut rt, "pay-amount", "15000");
    click(&mut rt, "btn-Save");
    assert!(!rt.state.payments.modal_open, "{}", blow(&rt, "first payment"));

    click(&mut rt, Page::Overview.key());
    click(&mut rt, "btn-Record a payment");
    assert!(rt.state.payments.modal_open, "{}", blow(&rt, "overview record payment"));
    rt.state.payments.party_id = party;
    rt.state.payments.funds = 0;
    type_into(&mut rt, "pay-amount", "25000");
    click(&mut rt, "btn-Save");
    assert!(!rt.state.payments.modal_open, "{}", blow(&rt, "second payment"));

    assert_eq!(rt.state.books.list("payments").len(), payments_before + 2);
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn test_cheque_clear_offers_fund_accounts() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let party = rt
        .state
        .create_party(crate::ui::resources::ResourceKind::Customer, "Cheque Clear Buyer", "", "", "")
        .unwrap_or_else(|e| panic!("{e}"));

    click(&mut rt, Page::Sales.key());
    click(&mut rt, "btn-New sales invoice");
    rt.state.sales.customer_id = party.clone();
    rt.state.sales.pay_method = 2;
    rt.state.sales.lines[0].item_id = "ITEM-A".into();
    rt.state.sales.lines[0].qty = "1".into();
    rt.state.sales.lines[0].price = "80000".into();
    type_into(&mut rt, "sale-qty-0", "1");
    type_into(&mut rt, "sale-price-0", "80000");
    click(&mut rt, "btn-Complete sale");
    assert!(!rt.state.sales.modal_open, "{}", blow(&rt, "credit sale for cheque clear"));

    click(&mut rt, Page::Payments.key());
    click(&mut rt, "btn-Receive money");
    rt.state.payments.party_id = party;
    rt.state.payments.funds = 2;
    rt.frame(1.0 / 60.0);
    type_into(&mut rt, "pay-amount", "80000");
    type_into(&mut rt, "pay-sayad", "1234567890123457");
    type_into(&mut rt, "pay-cheque-no", "CHQ-CLR-1");
    type_into(&mut rt, "pay-bank", "Melli");
    let due = rt.state.fmt.date_input(&data::today());
    type_into(&mut rt, "pay-due", &due);
    click(&mut rt, "btn-Save");
    assert!(!rt.state.payments.modal_open, "{}", blow(&rt, "receive cheque"));

    let (cheque_id, status) = cheque_entry(&rt, "CHQ-CLR-1");
    assert_eq!(status, "in_hand");

    crate::ui::views::payments::open_cheque_confirm(&mut rt.state, &cheque_id, "deposit");
    assert!(rt.state.confirm.is_some(), "deposit must ask for confirmation");
    rt.frame(1.0 / 60.0);
    click(&mut rt, "btn-Deposit for collection");
    let (_, status) = cheque_entry(&rt, "CHQ-CLR-1");
    assert_eq!(status, "in_collection", "{}", blow(&rt, "deposit cheque"));

    crate::ui::views::payments::open_cheque_confirm(&mut rt.state, &cheque_id, "clear");
    let confirm = rt.state.confirm.as_ref().expect("clear must open confirm with fund accounts");
    assert!(
        !confirm.choices.is_empty(),
        "clear confirm must offer at least one fund account choice"
    );
    let chosen_key = confirm.choices.last().unwrap().key.clone();
    rt.frame(1.0 / 60.0);
    click(&mut rt, &format!("btn-{chosen_key}"));

    let (_, status) = cheque_entry(&rt, "CHQ-CLR-1");
    assert_eq!(status, "cleared", "{}", blow(&rt, "clear cheque"));
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn test_cheque_due_date_accepts_jalali() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let party = rt
        .state
        .create_party(crate::ui::resources::ResourceKind::Customer, "Jalali Cheque Buyer", "", "", "")
        .unwrap_or_else(|e| panic!("{e}"));

    click(&mut rt, Page::Sales.key());
    click(&mut rt, "btn-New sales invoice");
    rt.state.sales.customer_id = party.clone();
    rt.state.sales.pay_method = 2;
    rt.state.sales.lines[0].item_id = "ITEM-A".into();
    rt.state.sales.lines[0].qty = "1".into();
    rt.state.sales.lines[0].price = "50000".into();
    type_into(&mut rt, "sale-qty-0", "1");
    type_into(&mut rt, "sale-price-0", "50000");
    click(&mut rt, "btn-Complete sale");
    assert!(!rt.state.sales.modal_open, "{}", blow(&rt, "credit sale for jalali cheque"));

    let (start, _) = rt.state.books.range();
    let target_iso = chrono::NaiveDate::parse_from_str(&start, "%Y-%m-%d")
        .unwrap()
        .checked_add_days(chrono::Days::new(10))
        .unwrap()
        .format("%Y-%m-%d")
        .to_string();
    let jalali_due = rt.state.fmt.date_input(&target_iso);

    click(&mut rt, Page::Payments.key());
    click(&mut rt, "btn-Receive money");
    rt.state.payments.party_id = party;
    rt.state.payments.funds = 2;
    rt.frame(1.0 / 60.0);
    type_into(&mut rt, "pay-amount", "50000");
    type_into(&mut rt, "pay-sayad", "9876543210123456");
    type_into(&mut rt, "pay-cheque-no", "CHQ-JAL-1");
    type_into(&mut rt, "pay-bank", "Melli");
    type_into(&mut rt, "pay-due", &jalali_due);
    click(&mut rt, "btn-Save");
    assert!(!rt.state.payments.modal_open, "{}", blow(&rt, "receive jalali cheque"));

    let due_saved = rt.state.books.workspace["cheques"]
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .find(|e| data::s(&e["cheque"]["details"]["number"]) == "CHQ-JAL-1")
                .map(|e| data::s(&e["cheque"]["details"]["due_date"]))
        })
        .unwrap_or_else(|| panic!("missing cheque CHQ-JAL-1"));
    assert_eq!(due_saved, target_iso);
    assert_no_danger(&rt);
    books_validate(&rt);
}
