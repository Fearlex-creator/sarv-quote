use super::harness::{click, make_test_runtime};
use crate::app::Page;

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
