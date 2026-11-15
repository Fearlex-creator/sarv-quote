use super::harness::{assert_no_danger, blow, books_validate, click, count, make_test_runtime, type_into};
use crate::app::Page;

#[test]
fn test_owner_walks_profiles_documents_and_books() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;

    click(&mut rt, Page::Contacts.key());
    click(&mut rt, "contact-CU-A");
    click(&mut rt, "btn-Contact details");
    assert!(rt.find("contact-profile-dialog").is_some(), "{}", blow(&rt, "contact dialog"));
    rt.state.forms.set("profile-phone", "09120001122");
    click(&mut rt, "btn-Save");
    assert!(rt.state.contacts.open_id.is_empty(), "{}", blow(&rt, "contact save"));
    let detail = rt
        .state
        .query(
            "contact_detail",
            serde_json::json!({ "party_id": "CU-A", "as_of": crate::data::today() }),
        )
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(crate::data::s(&detail["profile"]["phone"]), "09120001122");

    click(&mut rt, Page::Inventory.key());
    click(&mut rt, "item-ITEM-A");
    click(&mut rt, "btn-Product details");
    assert!(rt.find("product-profile-dialog").is_some(), "{}", blow(&rt, "product dialog"));
    rt.state.forms.set("prod-barcode", "6260001112223");
    rt.state.forms.set("prod-price", "150000");
    click(&mut rt, "btn-Save");
    assert!(!rt.state.inventory.profile_open, "{}", blow(&rt, "product save"));
    assert_eq!(
        crate::data::s(&rt.state.books.workspace["product_profiles"]["ITEM-A"]["barcode"]),
        "6260001112223"
    );

    click(&mut rt, Page::Sales.key());
    click(&mut rt, "btn-New sales invoice");
    rt.state.sales.customer_id = "CU-A".into();
    rt.state.sales.pay_method = 2;
    rt.state.sales.lines[0].item_id = "ITEM-A".into();
    rt.state.sales.lines[0].qty = "1".into();
    rt.state.sales.lines[0].price = "150000".into();
    rt.state.forms.set("sale-qty-0", "1");
    rt.state.forms.set("sale-price-0", "150000");
    click(&mut rt, "btn-Issue quotation");
    assert!(!rt.state.sales.modal_open, "{}", blow(&rt, "quotation"));
    let draft_id = rt.state.books.workspace["drafts"]
        .as_array()
        .and_then(|rows| rows.iter().rev().find(|d| !d["quote_issue"].is_null()))
        .map(|d| crate::data::s(&d["id"]))
        .filter(|id| !id.is_empty())
        .unwrap_or_else(|| panic!("{}", blow(&rt, "draft id")));
    click(&mut rt, &format!("draft-{draft_id}"));
    click(&mut rt, "btn-Print / save PDF");
    let quote_pdf = rt.state.engine.dir.join("exports").join(format!("quotation-{draft_id}.pdf"));
    let quote_bytes = std::fs::read(&quote_pdf).unwrap_or_else(|e| panic!("{e}; {}", blow(&rt, "quotation pdf")));
    assert!(quote_bytes.starts_with(b"%PDF"), "quotation pdf");

    rt.state.sales.tab_idx = 0;
    rt.frame(1.0 / 60.0);
    click(&mut rt, "btn-New sales invoice");
    rt.state.sales.customer_id = "CU-A".into();
    rt.state.sales.pay_method = 2;
    rt.state.sales.lines[0].item_id = "ITEM-A".into();
    rt.state.sales.lines[0].qty = "1".into();
    rt.state.sales.lines[0].price = "150000".into();
    rt.state.forms.set("sale-qty-0", "1");
    rt.state.forms.set("sale-price-0", "150000");
    click(&mut rt, "btn-Complete sale");
    assert!(!rt.state.sales.modal_open, "{}", blow(&rt, "credit sale"));
    let invoice_id = rt
        .state
        .books
        .list("sales")
        .into_iter()
        .find(|r| r["command"].get("Fulfill").is_some())
        .map(|r| crate::data::s(&r["id"]))
        .unwrap_or_else(|| panic!("{}", blow(&rt, "invoice id")));
    click(&mut rt, &format!("sale-{invoice_id}"));
    click(&mut rt, "btn-Print / save PDF");
    let invoice_pdf = rt.state.engine.dir.join("exports").join(format!("invoice-{invoice_id}.pdf"));
    let invoice_bytes = std::fs::read(&invoice_pdf).unwrap_or_else(|e| panic!("{e}; {}", blow(&rt, "invoice pdf")));
    assert!(invoice_bytes.starts_with(b"%PDF"), "invoice pdf");
    click(&mut rt, "btn-Return");
    click(&mut rt, "btn-Review");
    click(&mut rt, "btn-Post return");
    let returned = rt
        .state
        .books
        .list("sales")
        .into_iter()
        .any(|r| r["command"].get("Return").is_some());
    assert!(returned, "{}", blow(&rt, "sale return"));

    let supplier = rt.state.books.workspace["resources"]["parties"]
        .as_array()
        .and_then(|rows| rows.iter().find(|p| p["supplier"] == true))
        .map(|p| crate::data::s(&p["id"]))
        .filter(|id| !id.is_empty());
    if let Some(supplier) = supplier {
        click(&mut rt, Page::Purchases.key());
        click(&mut rt, "btn-New purchase order");
        rt.state.purchases.supplier_id = supplier;
        rt.state.purchases.receive_and_bill = true;
        rt.state.purchases.lines[0].item_id = "ITEM-A".into();
        rt.state.purchases.lines[0].qty = "1".into();
        rt.state.purchases.lines[0].price = "80000".into();
        rt.state.forms.set("buy-qty-0", "1");
        rt.state.forms.set("buy-price-0", "80000");
        click(&mut rt, "btn-Save");
        assert!(!rt.state.purchases.modal_open, "{}", blow(&rt, "purchase"));
        let purchases = rt.state.books.list("purchases");
        let bill = purchases
            .iter()
            .find(|r| r["command"].get("Bill").is_some())
            .map(|r| crate::data::s(&r["id"]));
        let receipt = purchases
            .iter()
            .find(|r| r["command"].get("Receive").is_some())
            .map(|r| crate::data::s(&r["id"]));
        let (return_id, tab) = bill
            .filter(|id| !id.is_empty())
            .map(|id| (id, 2))
            .or_else(|| receipt.filter(|id| !id.is_empty()).map(|id| (id, 1)))
            .unwrap_or_else(|| panic!("{}", blow(&rt, "receipt")));
        click(&mut rt, &format!("segment{tab}"));
        for _ in 0..8 {
            rt.frame(1.0 / 60.0);
        }
        click(&mut rt, &format!("purchase-{return_id}"));
        click(&mut rt, "btn-Return");
        click(&mut rt, "btn-Review");
        click(&mut rt, "btn-Post return");
        let sent_back = rt
            .state
            .books
            .list("purchases")
            .into_iter()
            .any(|r| r["command"].get("Return").is_some());
        assert!(sent_back, "{}", blow(&rt, "purchase return"));
    }

    click(&mut rt, Page::Accounting.key());
    let accounts = crate::ui::pickers::account_options(&rt.state);
    assert!(accounts.len() >= 2, "chart of accounts");
    click(&mut rt, "btn-New journal");
    rt.state.accounting.set_line_account(0, accounts[0].0.clone());
    rt.state.accounting.set_line_account(1, accounts[1].0.clone());
    rt.state.forms.set("journal-debit-0", "25000");
    rt.state.forms.set("journal-credit-1", "25000");
    rt.state.forms.set("journal-reason", "Owner adjustment");
    click(&mut rt, "btn-Save");
    assert!(!rt.state.accounting.modal_open, "{}", blow(&rt, "journal"));
    let journal_id = rt
        .state
        .books
        .list("journals")
        .into_iter()
        .find(|j| crate::data::s(&j["reason"]) == "Owner adjustment")
        .map(|j| crate::data::s(&j["id"]))
        .unwrap_or_else(|| panic!("{}", blow(&rt, "journal id")));
    click(&mut rt, &format!("journal-{journal_id}"));
    click(&mut rt, "btn-Reverse");
    click(&mut rt, "btn-Reverse entry");
    let reversed = rt
        .state
        .books
        .list("journals")
        .into_iter()
        .any(|j| j["kind"].get("Reversal").is_some() || crate::data::s(&j["event"]) == "reversal");
    assert!(reversed, "{}", blow(&rt, "reversal"));

    click(&mut rt, "segment1");
    for _ in 0..8 {
        rt.frame(1.0 / 60.0);
    }
    let first_account = crate::data::s(&rt.state.books.workspace["accounts"][0]["id"]);
    click(&mut rt, &format!("acc-{first_account}"));
    click(&mut rt, "btn-Ledger");
    assert!(rt.state.accounting.ledger_open, "{}", blow(&rt, "ledger"));
    assert!(rt.find("ledger-dialog").is_some());
    click(&mut rt, "btn-Close");

    click(&mut rt, "segment2");
    for _ in 0..8 {
        rt.frame(1.0 / 60.0);
    }
    let period = rt.state.books.period_id.clone();
    click(&mut rt, &format!("period-{period}"));
    click(&mut rt, "btn-Lock fiscal period");
    click(&mut rt, "btn-Lock period");
    let locked = rt.state.books.workspace["periods"]
        .as_array()
        .and_then(|rows| rows.iter().find(|p| crate::data::s(&p["id"]) == period))
        .and_then(|p| p["locked"].as_bool())
        .unwrap_or(false);
    if locked {
        click(&mut rt, "btn-Reopen fiscal period");
        click(&mut rt, "btn-Reopen period");
        let open_again = rt.state.books.workspace["periods"]
            .as_array()
            .and_then(|rows| rows.iter().find(|p| crate::data::s(&p["id"]) == period))
            .and_then(|p| p["locked"].as_bool())
            .unwrap_or(true);
        assert!(!open_again, "{}", blow(&rt, "reopen period"));
    } else {
        let note = blow(&rt, "period stayed open");
        assert!(note.contains("Integrity") || note.contains("یکپارچ"), "{note}");
    }

    click(&mut rt, Page::Settings.key());
    click(&mut rt, "btn-Your business");
    assert!(rt.find("business-profile-dialog").is_some(), "{}", blow(&rt, "business dialog"));
    rt.state.forms.set("biz-prefix", "SARV");
    rt.state.forms.set("biz-phone", "02100000000");
    click(&mut rt, "btn-Save");
    assert!(!rt.state.settings.business_open, "{}", blow(&rt, "business save"));
    assert_eq!(
        crate::data::s(&rt.state.books.workspace["business_profile"]["invoice_prefix"]),
        "SARV"
    );

    click(&mut rt, "btn-Backup");
    click(&mut rt, "btn-Restore backup");
    let filename = rt.state.settings.backups["backups"]
        .as_array()
        .and_then(|rows| rows.first())
        .map(|row| crate::data::s(&row["filename"]))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| panic!("{}", blow(&rt, "backup list")));
    click(&mut rt, &format!("btn-backup-{filename}"));
    assert!(!rt.state.settings.restore_note.is_empty(), "{}", blow(&rt, "restore preview"));
    click(&mut rt, "btn-restore-confirm");
    assert!(!rt.state.settings.restore_open, "{}", blow(&rt, "restore"));
    assert!(!rt.state.books.company_name().is_empty(), "books still open after restore");
}

pub(super) fn double_click(rt: &mut rune::Runtime<crate::app::State>, key: &str) {
    let target = key.to_string();
    rt.update(|_, tx| tx.scroll_into_view(target));
    rt.frame(1.0 / 60.0);
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("missing element `{key}`"));
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1.0 / 60.0);
}

#[test]
fn double_click_on_save_posts_once() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;

    // 1. Journal
    let accounts = crate::ui::pickers::account_options(&rt.state);
    assert!(accounts.len() >= 2);
    let journals_before = count(&rt, "journal_count");
    click(&mut rt, Page::Accounting.key());
    click(&mut rt, "btn-New journal");
    rt.state.accounting.set_line_account(0, accounts[0].0.clone());
    rt.state.accounting.set_line_account(1, accounts[1].0.clone());
    type_into(&mut rt, "journal-debit-0", "10000");
    type_into(&mut rt, "journal-credit-1", "10000");
    type_into(&mut rt, "journal-reason", "Double click journal");
    double_click(&mut rt, "btn-Save");
    assert!(!rt.state.accounting.modal_open, "{}", blow(&rt, "double click journal"));
    assert_eq!(count(&rt, "journal_count"), journals_before + 1, "journal must post once");

    // 2. Payment
    let payments_before = rt.state.books.list("payments").len();
    click(&mut rt, Page::Payments.key());
    click(&mut rt, "btn-Receive money");
    rt.state.payments.party_id = "CU-A".into();
    rt.state.payments.funds = 0;
    type_into(&mut rt, "pay-amount", "10000");
    double_click(&mut rt, "btn-Save");
    assert!(!rt.state.payments.modal_open, "{}", blow(&rt, "double click payment"));
    assert_eq!(rt.state.books.list("payments").len(), payments_before + 1, "payment must post once");

    // 3. Sale
    let invoices = |s: &crate::app::State| {
        s.books
            .list("sales")
            .into_iter()
            .filter(|r| r["command"].get("Fulfill").is_some())
            .count()
    };
    let sales_before = invoices(&rt.state);
    click(&mut rt, Page::Sales.key());
    click(&mut rt, "btn-New sales invoice");
    rt.state.sales.customer_id = "CU-A".into();
    rt.state.sales.pay_method = 2;
    rt.state.sales.lines[0].item_id = "ITEM-A".into();
    rt.state.sales.lines[0].qty = "1".into();
    rt.state.sales.lines[0].price = "20000".into();
    type_into(&mut rt, "sale-qty-0", "1");
    type_into(&mut rt, "sale-price-0", "20000");
    double_click(&mut rt, "btn-Complete sale");
    assert!(!rt.state.sales.modal_open, "{}", blow(&rt, "double click sale"));
    assert_eq!(invoices(&rt.state), sales_before + 1, "sale must post once");

    assert_no_danger(&rt);
    books_validate(&rt);
}
