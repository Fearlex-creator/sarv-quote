use free_accounting_core::Ledger;
use serde_json::{Value, json};

fn ledger() -> Ledger {
    Ledger::open(":memory:").unwrap()
}
fn account(l: &Ledger, currency: &str, opening: &str) -> String {
    l.mutate("account",json!({"name":format!("Checking {currency}"),"kind":"bank","currency":currency,"openingBalance":opening})).unwrap()["id"].as_str().unwrap().into()
}
fn expense(account: &str, amount: &str) -> Value {
    json!({"date":free_accounting_core::today(),"kind":"expense","amount":amount,"accountId":account,"payee":"Stationery","categoryId":"office"})
}
fn balance(l: &Ledger) -> String {
    l.read("workspace", json!({})).unwrap()["accounts"][0]["balanceMinor"]
        .as_str()
        .unwrap()
        .into()
}

#[test]
fn journal_is_balanced_and_opening_is_a_posting() {
    let l = ledger();
    let a = account(&l, "USD", "100.00");
    let tx = l.mutate("transaction", expense(&a, "0.29")).unwrap();
    assert_eq!(balance(&l), "9971");
    let detail = l.read("detail", tx).unwrap();
    let lines = detail["lines"].as_array().unwrap();
    let net: i64 = lines
        .iter()
        .map(|r| {
            r["debitMinor"].as_str().unwrap().parse::<i64>().unwrap()
                - r["creditMinor"].as_str().unwrap().parse::<i64>().unwrap()
        })
        .sum();
    assert_eq!(net, 0);
    assert_eq!(lines.len(), 2);
    let rows = l.read("ledger", json!({"currency":"USD"})).unwrap();
    assert_eq!(rows["rows"][0]["balanceMinor"], "9971");
    assert_eq!(rows["rows"][1]["kind"], "opening");
}

#[test]
fn correction_reverses_and_reposts_without_changing_history() {
    let l = ledger();
    let a = account(&l, "USD", "100");
    let old = l.mutate("transaction", expense(&a, "10")).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let mut corrected = expense(&a, "12");
    corrected["correctionId"] = json!(old);
    l.mutate("transaction", corrected).unwrap();
    assert_eq!(balance(&l), "8800");
    let detail = l.read("detail", json!({"id":old})).unwrap();
    assert_eq!(detail["amountMinor"], "1000");
    assert!(detail["reversedBy"].is_string());
    assert_eq!(detail["history"].as_array().unwrap().len(), 3);
    assert!(
        l.mutate(
            "reverse",
            json!({"id":old,"date":free_accounting_core::today(),"reason":"Again"})
        )
        .is_err()
    );
}

#[test]
fn splits_and_failed_mutations_are_atomic() {
    let l = ledger();
    let a = account(&l, "USD", "100");
    let mut input = expense(&a, "10");
    input["splits"] =
        json!([{"categoryId":"office","amount":"4"},{"categoryId":"food","amount":"5"}]);
    assert!(l.mutate("transaction", input.clone()).is_err());
    assert_eq!(balance(&l), "10000");
    input["splits"][1]["amount"] = json!("6");
    let tx = l.mutate("transaction", input).unwrap();
    let d = l.read("detail", tx).unwrap();
    assert_eq!(d["lines"].as_array().unwrap().len(), 3);
    assert_eq!(balance(&l), "9000");
}

#[test]
fn partial_payment_aging_and_settlement_use_journal_lines() {
    let l = ledger();
    let a = account(&l, "USD", "100");
    let c = l
        .mutate("contact", json!({"name":"Supplier","supplier":true}))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let mut input = expense(&a, "50");
    input["paidAmount"] = json!("20");
    input["contactId"] = json!(c);
    l.mutate("transaction", input).unwrap();
    let workspace = l.read("workspace", json!({})).unwrap();
    assert_eq!(
        workspace["contacts"][0]["balances"][0]["payableMinor"],
        "3000"
    );
    assert_eq!(workspace["dashboard"]["expenseMinor"], "5000");
    assert_eq!(balance(&l), "8000");
    l.mutate("settle",json!({"contactId":c,"accountId":a,"direction":"payable","amount":"10","date":free_accounting_core::today()})).unwrap();
    assert_eq!(balance(&l), "7000");
    let w = l.read("workspace", json!({})).unwrap();
    assert_eq!(w["contacts"][0]["balances"][0]["payableMinor"], "2000");
    assert_eq!(w["contacts"][0]["balances"][0]["payableAging"][0], "2000");
}

#[test]
fn closed_period_and_currency_guards() {
    let l = ledger();
    let usd = account(&l, "USD", "100");
    let irr = account(&l, "IRR", "100000");
    assert!(l.mutate("transaction",json!({"date":free_accounting_core::today(),"kind":"transfer","amount":"10","accountId":usd,"toAccountId":irr,"payee":"Transfer"})).is_err());
    assert!(l.mutate("transaction", expense(&irr, "1.1")).is_err());
    let mut s = l.read("workspace", json!({})).unwrap()["settings"].clone();
    s["closedThrough"] = json!(free_accounting_core::today());
    l.mutate("settings", s.clone()).unwrap();
    assert!(l.mutate("transaction", expense(&usd, "10")).is_err());
    s["closedThrough"] = json!("");
    assert!(l.mutate("settings", s).is_err());
    assert!(
        l.mutate(
            "account",
            json!({"id":usd,"name":"Changed","kind":"bank","currency":"EUR"})
        )
        .is_err()
    );
}

#[test]
fn persistence_and_receipts() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ledger.sqlite");
    let tx_id;
    {
        let l = Ledger::open(&path).unwrap();
        let a = account(&l, "USD", "100");
        let mut input = expense(&a, "10");
        input["receipt"] = json!({"name":"receipt.txt","mime":"text/plain","bytes":[104,105]});
        tx_id = l.mutate("transaction", input).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();
    }
    let l = Ledger::open(&path).unwrap();
    assert_eq!(balance(&l), "9000");
    let d = l.read("detail", json!({"id":tx_id})).unwrap();
    let a = l
        .read("attachment", json!({"id":d["attachments"][0]["id"]}))
        .unwrap();
    assert_eq!(a["bytes"], json!([104, 105]));
    let backup = l.read("export", json!({"scope":"backup"})).unwrap();
    let content: Value = serde_json::from_str(backup["content"].as_str().unwrap()).unwrap();
    assert!(content["journal_lines"][0]["debit"].is_string());
    assert_eq!(content["attachments"].as_array().unwrap().len(), 1);
}

#[test]
fn duplicate_merge_and_bulk_categorization_leave_audit_history() {
    let l = ledger();
    let a = account(&l, "USD", "100");
    let t1 = l.mutate("transaction", expense(&a, "10")).unwrap()["id"].clone();
    let t2 = l.mutate("transaction", expense(&a, "10")).unwrap()["id"].clone();
    l.mutate(
        "merge",
        json!({"ids":[t1,t2],"date":free_accounting_core::today()}),
    )
    .unwrap();
    assert_eq!(balance(&l), "9000");
    l.mutate(
        "categorize",
        json!({"ids":[t1],"categoryId":"food","date":free_accounting_core::today()}),
    )
    .unwrap();
    assert_eq!(balance(&l), "9000");
    let w = l.read("workspace", json!({})).unwrap();
    assert!(
        w["audit"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["action"] == "duplicates.merged")
    );
}

#[test]
fn csv_import_rolls_back_the_entire_batch_on_invalid_row() {
    let l = ledger();
    let result = l.mutate(
        "import_contacts",
        json!({"csv":"name,email\nValid,a@example.com\nInvalid,no-email\n"}),
    );
    assert!(result.is_err());
    assert_eq!(
        l.read("workspace", json!({})).unwrap()["contacts"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
}

#[test]
fn demo_is_explicit_and_cannot_overwrite_data() {
    let l = ledger();
    assert_eq!(
        l.read("workspace", json!({})).unwrap()["transactionCount"],
        0
    );
    l.mutate("demo", json!({})).unwrap();
    let w = l.read("workspace", json!({})).unwrap();
    assert_eq!(w["accounts"].as_array().unwrap().len(), 3);
    assert_eq!(w["transactionCount"], 11);
    assert!(l.mutate("demo", json!({})).is_err());
}

#[test]
fn correcting_an_unpaid_transaction_reverses_contact_balances() {
    let l = ledger();
    let a = account(&l, "USD", "100");
    let contact = l
        .mutate("contact", json!({"name":"Vendor","supplier":true}))
        .unwrap()["id"]
        .clone();
    let mut original = expense(&a, "50");
    original["paidAmount"] = json!("0");
    original["contactId"] = contact.clone();
    let tx = l.mutate("transaction", original).unwrap()["id"].clone();
    let mut corrected = expense(&a, "40");
    corrected["paidAmount"] = json!("0");
    corrected["contactId"] = contact;
    corrected["correctionId"] = tx;
    l.mutate("transaction", corrected).unwrap();
    assert_eq!(
        l.read("workspace", json!({})).unwrap()["contacts"][0]["balances"][0]["payableMinor"],
        "4000"
    );
}

#[test]
fn pdf_is_a_real_document() {
    let l = ledger();
    let a = account(&l, "IRR", "100000");
    l.mutate("transaction", expense(&a, "12345")).unwrap();
    let result = l
        .read(
            "export",
            json!({"scope":"ledger_pdf","filters":{"currency":"IRR"}}),
        )
        .unwrap();
    let bytes: Vec<u8> = serde_json::from_value(result["bytes"].clone()).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
    assert!(bytes.len() > 1000);
}

#[test]
fn a_hundred_thousand_rows_are_paged_with_exact_running_balance() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("scale.sqlite");
    let l = Ledger::open(&path).unwrap();
    let a = account(&l, "USD", "100");
    drop(l);
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON; BEGIN IMMEDIATE;")
        .unwrap();
    conn.execute("INSERT INTO accounts(id,name,kind,currency,system,created_at) VALUES('scale-expense','Expense','expense','USD',1,'fixture')",[]).unwrap();
    conn.execute("WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<100000) INSERT INTO transactions(id,date,kind,payee,currency,amount_minor,paid_minor,input_json,recorded_at,seq) SELECT 'scale-'||x,?1,'expense','Scale row','USD',1,1,json_object('accountId',?2,'categoryId','office','date',?1,'kind','expense','amount','0.01','payee','Scale row'),'fixture',x+1 FROM n",rusqlite::params![free_accounting_core::today(),a]).unwrap();
    conn.execute("INSERT INTO journal_lines(transaction_id,account_id,debit,credit) SELECT id,?1,0,1 FROM transactions WHERE id LIKE 'scale-%'",[&a]).unwrap();
    conn.execute("INSERT INTO journal_lines(transaction_id,account_id,category_id,debit,credit) SELECT id,'scale-expense','office',1,0 FROM transactions WHERE id LIKE 'scale-%'",[]).unwrap();
    conn.execute_batch("UPDATE transactions SET state='posted' WHERE id LIKE 'scale-%'; COMMIT;")
        .unwrap();
    drop(conn);
    let l = Ledger::open(&path).unwrap();
    let start = std::time::Instant::now();
    let page = l
        .read("ledger", json!({"currency":"USD","limit":150}))
        .unwrap();
    println!("100k first page: {:?}", start.elapsed());
    assert!(
        start.elapsed().as_secs() < 5,
        "100k page exceeded five seconds"
    );
    assert_eq!(page["total"], 100001);
    assert_eq!(page["rows"].as_array().unwrap().len(), 150);
    assert_eq!(page["rows"][0]["balanceMinor"], "-90000");
    let page = l
        .read(
            "ledger",
            json!({"currency":"USD","offset":99900,"limit":150}),
        )
        .unwrap();
    assert_eq!(page["rows"].as_array().unwrap().len(), 101);
    assert!(page["nextOffset"].is_null());
}
