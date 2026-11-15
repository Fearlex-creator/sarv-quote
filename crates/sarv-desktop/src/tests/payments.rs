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
    assert_eq!(cheque_action_label("spend"), Some("Endorse to supplier"));
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

    rt.state.payments.tab_idx = 1;
    rt.state.payments.selected_cheque = cheque_id;
    rt.frame(1.0 / 60.0);
    click(&mut rt, "btn-cheque-act-deposit");
    assert!(rt.state.confirm.is_some(), "deposit must open confirm");
    rt.frame(1.0 / 60.0);
    click(&mut rt, "btn-Deposit for collection");
    let (_, status) = cheque_entry(&rt, "CHQ-101");
    assert_eq!(status, "in_collection", "{}", blow(&rt, "deposit cheque"));
    assert_eq!(bank_balance(&rt), bank_before, "bank balance must not rise on deposit");

    rt.frame(1.0 / 60.0);
    click(&mut rt, "btn-cheque-act-clear");
    assert_eq!(rt.state.confirm.as_ref().unwrap().title, rt.state.fmt.t("Mark as cleared"));
    let chosen_key = rt.state.confirm.as_ref().unwrap().choices.last().unwrap().key.clone();
    rt.frame(1.0 / 60.0);
    click(&mut rt, &format!("btn-{chosen_key}"));
    let (_, status) = cheque_entry(&rt, "CHQ-101");
    assert_eq!(status, "cleared");
    assert_eq!(bank_balance(&rt), bank_before + 100000, "bank balance rises at clear");

    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
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
    rt.state.payments.tab_idx = 1;
    rt.state.payments.selected_cheque = cheque_id;
    rt.frame(1.0 / 60.0);
    click(&mut rt, "btn-cheque-act-deposit");
    rt.frame(1.0 / 60.0);
    click(&mut rt, "btn-Deposit for collection");
    assert_eq!(cheque_entry(&rt, "CHQ-202").1, "in_collection");
    rt.frame(1.0 / 60.0);
    click(&mut rt, "btn-cheque-act-bounce");
    rt.frame(1.0 / 60.0);
    click(&mut rt, "btn-Mark as bounced");
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

#[test]
fn test_payment_reversal_restores_the_open_invoice() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let party = rt
        .state
        .create_party(crate::ui::resources::ResourceKind::Customer, "Reverse Buyer", "", "", "")
        .unwrap_or_else(|e| panic!("{e}"));

    click(&mut rt, Page::Sales.key());
    click(&mut rt, "btn-New sales invoice");
    rt.state.sales.customer_id = party.clone();
    rt.state.sales.pay_method = 2;
    rt.state.sales.lines[0].item_id = "ITEM-A".into();
    rt.state.sales.lines[0].qty = "1".into();
    rt.state.sales.lines[0].price = "90000".into();
    type_into(&mut rt, "sale-qty-0", "1");
    type_into(&mut rt, "sale-price-0", "90000");
    click(&mut rt, "btn-Complete sale");
    assert!(!rt.state.sales.modal_open, "{}", blow(&rt, "credit sale"));
    assert_eq!(open_receivable_total(&mut rt, &party), 90000);

    click(&mut rt, Page::Payments.key());
    click(&mut rt, "btn-Receive money");
    rt.state.payments.party_id = party.clone();
    rt.state.payments.funds = 0;
    type_into(&mut rt, "pay-amount", "90000");
    click(&mut rt, "btn-Save");
    assert!(!rt.state.payments.modal_open, "{}", blow(&rt, "receive payment"));
    assert_eq!(open_receivable_total(&mut rt, &party), 0);

    let pay_id = data::s(&rt.state.books.list("payments")[0]["id"]);
    rt.state.payments.selected = pay_id.clone();
    rt.frame(1.0 / 60.0);
    click(&mut rt, "btn-Reverse payment");
    assert!(rt.state.confirm.is_some(), "reverse payment must ask for confirmation");
    rt.frame(1.0 / 60.0);
    click(&mut rt, "btn-Confirm reversal");
    assert!(rt.state.confirm.is_none(), "{}", blow(&rt, "confirm payment reversal"));
    assert_eq!(open_receivable_total(&mut rt, &party), 90000, "reversal restores open invoice");

    let rev_row = rt
        .state
        .payments
        .rows
        .iter()
        .find(|r| r.id == format!("payment-{pay_id}"))
        .expect("reversed payment row");
    assert_eq!(rev_row.cells[4], rt.state.fmt.t("Reversed"));
    assert_no_danger(&rt);

    rt.update(|s, _| s.toasts.clear());
    rt.state.payments.selected = pay_id;
    rt.frame(1.0 / 60.0);
    click(&mut rt, "btn-Reverse payment");
    assert!(rt.state.confirm.is_none(), "second reversal must not open confirm");
    assert_eq!(
        rt.state.toasts.iter().filter(|t| t.tone == crate::app::Tone::Danger).count(),
        1,
        "reversing twice must show one danger toast"
    );
    books_validate(&rt);
}

#[test]
fn test_received_cheque_can_be_endorsed_to_a_supplier() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let customer = rt
        .state
        .create_party(crate::ui::resources::ResourceKind::Customer, "Endorse Customer", "", "", "")
        .unwrap_or_else(|e| panic!("{e}"));
    let supplier = rt
        .state
        .create_party(crate::ui::resources::ResourceKind::Vendor, "Endorse Supplier", "", "", "")
        .unwrap_or_else(|e| panic!("{e}"));

    click(&mut rt, Page::Sales.key());
    click(&mut rt, "btn-New sales invoice");
    rt.state.sales.customer_id = customer.clone();
    rt.state.sales.pay_method = 2;
    rt.state.sales.lines[0].item_id = "ITEM-A".into();
    rt.state.sales.lines[0].qty = "1".into();
    rt.state.sales.lines[0].price = "120000".into();
    type_into(&mut rt, "sale-qty-0", "1");
    type_into(&mut rt, "sale-price-0", "120000");
    click(&mut rt, "btn-Complete sale");
    assert!(!rt.state.sales.modal_open, "{}", blow(&rt, "credit sale"));

    click(&mut rt, Page::Purchases.key());
    click(&mut rt, "btn-New purchase order");
    rt.state.purchases.supplier_id = supplier.clone();
    rt.state.purchases.receive_and_bill = true;
    rt.state.purchases.lines[0].item_id = "ITEM-A".into();
    rt.state.purchases.lines[0].qty = "1".into();
    rt.state.purchases.lines[0].price = "120000".into();
    type_into(&mut rt, "buy-qty-0", "1");
    type_into(&mut rt, "buy-price-0", "120000");
    click(&mut rt, "btn-Save");
    assert!(!rt.state.purchases.modal_open, "{}", blow(&rt, "purchase bill"));

    click(&mut rt, Page::Payments.key());
    click(&mut rt, "btn-Receive money");
    rt.state.payments.party_id = customer;
    rt.state.payments.funds = 2;
    rt.frame(1.0 / 60.0);
    type_into(&mut rt, "pay-amount", "120000");
    type_into(&mut rt, "pay-sayad", "1111222233334444");
    type_into(&mut rt, "pay-cheque-no", "CHQ-END-1");
    type_into(&mut rt, "pay-bank", "Melli");
    let due = rt.state.fmt.date_input(&data::today());
    type_into(&mut rt, "pay-due", &due);
    click(&mut rt, "btn-Save");
    assert!(!rt.state.payments.modal_open, "{}", blow(&rt, "receive cheque"));

    let (cheque_id, status) = cheque_entry(&rt, "CHQ-END-1");
    assert_eq!(status, "in_hand");

    rt.state.payments.tab_idx = 1;
    rt.state.payments.selected_cheque = cheque_id;
    rt.frame(1.0 / 60.0);
    click(&mut rt, "btn-Endorse to supplier");
    assert!(rt.state.payments.spend_open, "endorse dialog must open");

    rt.state.payments.spend_party_id = supplier.clone();
    let spend_date = rt.state.fmt.date_input(&data::today());
    type_into(&mut rt, "spend-date", &spend_date);
    click(&mut rt, "btn-Save");
    assert!(!rt.state.payments.spend_open, "{}", blow(&rt, "endorse save"));

    assert_eq!(cheque_entry(&rt, "CHQ-END-1").1, "spent");
    let open_ap = rt
        .state
        .query("payment_options", serde_json::json!({ "side": "make", "party_id": supplier }))
        .unwrap_or_else(|e| panic!("{e}"));
    let rem: i64 = open_ap["items"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|i| data::s(&i["remaining"]).parse::<i64>().unwrap_or(0))
        .sum();
    assert_eq!(rem, 0, "supplier bill must be settled by endorsed cheque");
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn test_cheque_register_sorts_by_due_date() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let party = rt
        .state
        .create_party(crate::ui::resources::ResourceKind::Customer, "Sort Cheque Buyer", "", "", "")
        .unwrap_or_else(|e| panic!("{e}"));

    click(&mut rt, Page::Sales.key());
    click(&mut rt, "btn-New sales invoice");
    rt.state.sales.customer_id = party.clone();
    rt.state.sales.pay_method = 2;
    rt.state.sales.lines[0].item_id = "ITEM-A".into();
    rt.state.sales.lines[0].qty = "1".into();
    rt.state.sales.lines[0].price = "90000".into();
    type_into(&mut rt, "sale-qty-0", "1");
    type_into(&mut rt, "sale-price-0", "90000");
    click(&mut rt, "btn-Complete sale");
    assert!(!rt.state.sales.modal_open, "{}", blow(&rt, "credit sale"));

    let base = chrono::NaiveDate::parse_from_str(&data::today(), "%Y-%m-%d").unwrap();
    let d_late = base.checked_add_days(chrono::Days::new(20)).unwrap().format("%Y-%m-%d").to_string();
    let d_soon = base.checked_add_days(chrono::Days::new(3)).unwrap().format("%Y-%m-%d").to_string();
    let d_mid = base.checked_add_days(chrono::Days::new(10)).unwrap().format("%Y-%m-%d").to_string();

    for (num, sayad, due_iso) in [
        ("CHQ-LATE", "2222333344445551", &d_late),
        ("CHQ-SOON", "2222333344445552", &d_soon),
        ("CHQ-MID", "2222333344445553", &d_mid),
    ] {
        click(&mut rt, Page::Payments.key());
        click(&mut rt, "btn-Receive money");
        rt.state.payments.party_id = party.clone();
        rt.state.payments.funds = 2;
        rt.frame(1.0 / 60.0);
        type_into(&mut rt, "pay-amount", "30000");
        type_into(&mut rt, "pay-sayad", sayad);
        type_into(&mut rt, "pay-cheque-no", num);
        type_into(&mut rt, "pay-bank", "Melli");
        let due_in = rt.state.fmt.date_input(due_iso);
        type_into(&mut rt, "pay-due", &due_in);
        click(&mut rt, "btn-Save");
        assert!(!rt.state.payments.modal_open, "{}", blow(&rt, num));
    }

    // Clear CHQ-SOON so it becomes closed and should sort after open cheques.
    let (soon_id, _) = cheque_entry(&rt, "CHQ-SOON");
    crate::ui::views::payments::open_cheque_confirm(&mut rt.state, &soon_id, "deposit");
    rt.frame(1.0 / 60.0);
    click(&mut rt, "btn-Deposit for collection");
    crate::ui::views::payments::open_cheque_confirm(&mut rt.state, &soon_id, "clear");
    let chosen_key = rt.state.confirm.as_ref().unwrap().choices.last().unwrap().key.clone();
    rt.frame(1.0 / 60.0);
    click(&mut rt, &format!("btn-{chosen_key}"));

    rt.state.payments.tab_idx = 1;
    rt.frame(1.0 / 60.0);
    let numbers: Vec<String> = rt.state.payments.cheques_rows.iter().map(|r| r.cells[0].clone()).collect();
    assert_eq!(numbers, vec!["CHQ-MID", "CHQ-LATE", "CHQ-SOON"]);
    assert_no_danger(&rt);
}
