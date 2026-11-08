//! Excel workbooks, the till, and the shop lock against a real SQLite profile.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use calamine::Reader;
use rust_xlsxwriter::Workbook;
use sarv_quote::agent::{Access, AgentService, CONNECTION};
use sarv_quote::desktop;
use serde_json::{Value, json};
use std::io::{Cursor, Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

fn call(dir: &Path, action: &str, payload: Value) -> Result<Value, String> {
    desktop::dispatch(dir, action, payload).map_err(|e| e.to_string())
}
fn must(dir: &Path, action: &str, payload: Value) -> Value {
    call(dir, action, payload).unwrap_or_else(|e| panic!("{action}: {e}"))
}
fn company(dir: &Path) -> (String, String) {
    let created = must(
        dir,
        "setup_company",
        json!({
            "name": "Sample shop",
            "fiscal_name": "1405",
            "start_date": "2026-03-21",
            "end_date": "2027-03-20",
            "actor": "owner"
        }),
    );
    let c = created["company_id"].as_str().unwrap().to_owned();
    let p = created["period_id"].as_str().unwrap().to_owned();
    must(
        dir,
        "initialize_zero",
        json!({"company_id": c, "source_id": "zero", "request_key": "zero", "confirmed": true}),
    );
    (c, p)
}
fn sheet(headers: &[&str], rows: &[Vec<String>], formula: bool) -> Vec<u8> {
    let mut book = Workbook::new();
    let data = book.add_worksheet();
    data.set_name("Data").unwrap();
    for (column, header) in headers.iter().enumerate() {
        data.write_string(0, column as u16, *header).unwrap();
    }
    for (row_index, row) in rows.iter().enumerate() {
        for (column, value) in row.iter().enumerate() {
            if formula && row_index == 0 && column == 0 {
                data.write_formula((row_index + 1) as u32, column as u16, value.as_str())
                    .unwrap();
            } else if value == "#NUMBER" {
                data.write_number((row_index + 1) as u32, column as u16, 123.0)
                    .unwrap();
            } else {
                data.write_string((row_index + 1) as u32, column as u16, value)
                    .unwrap();
            }
        }
    }
    book.save_to_buffer().unwrap()
}
fn products_file(rows: &[Vec<String>]) -> Vec<u8> {
    sheet(
        &[
            "sku",
            "name",
            "unit",
            "barcode",
            "category",
            "sale_price_rial",
            "purchase_price_rial",
            "minimum_stock",
        ],
        rows,
        false,
    )
}

#[test]
fn excel_roundtrip_opening_stock_replay_and_exact_rials() {
    let dir = tempfile::tempdir().unwrap();
    let (c, p) = company(dir.path());
    let bad = products_file(&[
        vec![
            "00123".into(),
            "Tea".into(),
            "Piece".into(),
            "6260000000012".into(),
            "Grocery".into(),
            "۲۵۰۰۰۰".into(),
            "180000".into(),
            "2".into(),
        ],
        vec![
            "".into(),
            "Broken".into(),
            "Piece".into(),
            "".into(),
            "".into(),
            "".into(),
            "".into(),
            "".into(),
        ],
    ]);
    let preview = must(
        dir.path(),
        "excel_preview",
        json!({"kind": "products", "content_base64": STANDARD.encode(&bad)}),
    );
    assert_eq!(preview["committed"], false);
    assert_eq!(preview["errors"].as_array().unwrap().len(), 1);
    assert!(call(
        dir.path(),
        "excel_commit",
        json!({
            "kind": "products",
            "content_base64": STANDARD.encode(&bad),
            "company_id": c,
            "period_id": p,
            "date": "2026-04-01",
            "request_key": "bad",
            "policy": "update",
            "confirm": true
        }),
    )
    .unwrap_err()
    .contains("row errors"));
    let workspace = must(dir.path(), "workspace", json!({"company_id": c}));
    assert!(workspace["resources"]["items"].as_array().unwrap().is_empty());

    let good = products_file(&[vec![
        "00123".into(),
        "Tea".into(),
        "Piece".into(),
        "6260000000012".into(),
        "Grocery".into(),
        "۲۵۰۰۰۰".into(),
        "180000".into(),
        "2".into(),
    ]]);
    let imported = must(
        dir.path(),
        "excel_commit",
        json!({
            "kind": "products",
            "content_base64": STANDARD.encode(&good),
            "company_id": c,
            "period_id": p,
            "date": "2026-04-01",
            "request_key": "products-1",
            "policy": "update",
            "confirm": true
        }),
    );
    assert_eq!(imported["created"], "1");
    assert_eq!(imported["currency"], "IRR");
    let again = must(
        dir.path(),
        "excel_commit",
        json!({
            "kind": "products",
            "content_base64": STANDARD.encode(&good),
            "company_id": c,
            "period_id": p,
            "date": "2026-04-01",
            "request_key": "products-1",
            "policy": "update",
            "confirm": true
        }),
    );
    assert_eq!(again["replayed"], true);
    let exported = must(
        dir.path(),
        "excel_export",
        json!({"kind": "products", "company_id": c}),
    );
    let bytes = STANDARD
        .decode(exported["content_base64"].as_str().unwrap())
        .unwrap();
    let mut book: calamine::Xlsx<_> =
        calamine::open_workbook_from_rs(Cursor::new(bytes)).unwrap();
    let range = book.worksheet_range("Data").unwrap();
    let rows: Vec<_> = range.rows().collect();
    assert_eq!(rows[1][0].to_string(), "00123");
    assert_eq!(rows[1][5].to_string(), "250000");

    let opening = sheet(
        &[
            "sku",
            "name",
            "unit",
            "warehouse",
            "quantity",
            "unit_cost_rial",
        ],
        &[vec![
            "00123".into(),
            "Tea".into(),
            "Piece".into(),
            "Main".into(),
            "2".into(),
            "180000".into(),
        ]],
        false,
    );
    let posted = must(
        dir.path(),
        "excel_commit",
        json!({
            "kind": "opening_stock",
            "content_base64": STANDARD.encode(&opening),
            "company_id": c,
            "period_id": p,
            "date": "2026-04-02",
            "request_key": "open-1",
            "policy": "skip",
            "confirm": true
        }),
    );
    assert_eq!(posted["effect"], "inventory_debited_grni_credited");
    assert_eq!(posted["supplier_bill"], false);
    must(
        dir.path(),
        "excel_commit",
        json!({
            "kind": "opening_stock",
            "content_base64": STANDARD.encode(&opening),
            "company_id": c,
            "period_id": p,
            "date": "2026-04-02",
            "request_key": "open-1",
            "policy": "skip",
            "confirm": true
        }),
    );
    let report = must(
        dir.path(),
        "report",
        json!({"company_id": c, "from": "2026-03-21", "to": "2026-04-02"}),
    );
    assert_eq!(
        report["trial"]["total_debit"],
        report["trial"]["total_credit"]
    );
    let stock = &report["reconciliation"]["stock"];
    assert_eq!(stock[0]["quantity_thousandths"], "2000");
    assert_eq!(stock[0]["value"], "360000");
    let trial = report["trial"]["rows"].as_array().unwrap();
    let inventory = trial.iter().find(|r| r["code"] == "1301").unwrap();
    let grni = trial.iter().find(|r| r["code"] == "2201").unwrap();
    assert_eq!(inventory["debit"], "360000");
    assert_eq!(grni["credit"], "360000");
}

#[test]
fn excel_rejects_formulas_macros_numeric_skus_and_oversized_files() {
    let dir = tempfile::tempdir().unwrap();
    let formula = sheet(
        &[
            "sku",
            "name",
            "unit",
            "barcode",
            "category",
            "sale_price_rial",
            "purchase_price_rial",
            "minimum_stock",
        ],
        &[vec![
            "=1+1".into(),
            "Tea".into(),
            "Piece".into(),
            "".into(),
            "".into(),
            "1".into(),
            "1".into(),
            "1".into(),
        ]],
        true,
    );
    let error = call(
        dir.path(),
        "excel_preview",
        json!({"kind": "products", "content_base64": STANDARD.encode(&formula)}),
    )
    .unwrap_err();
    assert!(error.contains("formula"), "{error}");

    let numeric = products_file(&[vec![
        "#NUMBER".into(),
        "Tea".into(),
        "Piece".into(),
        "".into(),
        "".into(),
        "10".into(),
        "10".into(),
        "1".into(),
    ]]);
    let preview = must(
        dir.path(),
        "excel_preview",
        json!({"kind": "products", "content_base64": STANDARD.encode(&numeric)}),
    );
    assert!(
        preview["errors"][0]["message"]
            .as_str()
            .unwrap()
            .contains("leading zeros")
    );

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut cursor);
        zip.start_file("xl/vbaProject.bin", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"not really a macro").unwrap();
        zip.finish().unwrap();
    }
    let macro_error = call(
        dir.path(),
        "excel_preview",
        json!({"kind": "products", "content_base64": STANDARD.encode(cursor.get_ref())}),
    )
    .unwrap_err();
    assert!(macro_error.contains("macro"), "{macro_error}");

    let mut huge = vec![b'P', b'K'];
    huge.resize(1_000_001, 0);
    let oversized = call(
        dir.path(),
        "excel_preview",
        json!({"kind": "products", "content_base64": STANDARD.encode(&huge)}),
    )
    .unwrap_err();
    assert!(oversized.contains("1 MB"), "{oversized}");

    let _guard = graph_env().lock().unwrap();
    let graph = must(dir.path(), "excel_graph_status", json!({}));
    assert_eq!(graph["configured"], false);
    assert_eq!(graph["status"], "not_configured");
    let sync = call(dir.path(), "excel_graph_sync", json!({})).unwrap_err();
    assert!(sync.contains("no background sync"), "{sync}");
}

#[test]
fn shop_lock_hides_secrets_and_cashier_cannot_post_a_journal() {
    let dir = tempfile::tempdir().unwrap();
    let (c, p) = company(dir.path());
    let password = "horse-battery-staple";
    let boot = must(
        dir.path(),
        "auth_bootstrap",
        json!({"name": "Sima", "password": password}),
    );
    let recovery = boot["recovery_code"].as_str().unwrap().to_owned();
    let owner = boot["session_token"].as_str().unwrap().to_owned();
    assert_eq!(boot["role"], "owner");
    let raw = std::fs::read(dir.path().join("sarv.sqlite3")).unwrap();
    let blob = String::from_utf8_lossy(&raw);
    assert!(!blob.contains(password));
    assert!(!blob.contains(&recovery));
    let workspace = must(
        dir.path(),
        "workspace",
        json!({"company_id": c, "session_token": owner}),
    );
    assert!(!workspace.to_string().contains(password));
    assert!(!workspace.to_string().contains(&recovery));

    let _cashier = must(
        dir.path(),
        "auth_create_user",
        json!({
            "session_token": owner,
            "name": "Niloofar",
            "password": "cashier-pass-1",
            "role": "cashier"
        }),
    );
    let signed = must(
        dir.path(),
        "auth_login",
        json!({"name": "Niloofar", "password": "cashier-pass-1"}),
    );
    let denied = call(
        dir.path(),
        "journal_post",
        json!({"session_token": signed["session_token"], "company_id": c, "period_id": p}),
    )
    .unwrap_err();
    assert!(denied.contains("role"), "{denied}");
    let excel_denied = call(
        dir.path(),
        "excel_commit",
        json!({"session_token": signed["session_token"], "confirm": true, "kind": "products"}),
    )
    .unwrap_err();
    assert!(excel_denied.contains("role"), "{excel_denied}");

    must(
        dir.path(),
        "auth_logout",
        json!({"session_token": owner}),
    );
    assert!(
        call(dir.path(), "workspace", json!({"company_id": c}))
            .unwrap_err()
            .contains("Sign in required")
    );
    let reset = must(
        dir.path(),
        "auth_reset",
        json!({"recovery_code": recovery, "password": "new-owner-pass-9"}),
    );
    assert_eq!(reset["reset"], true);
    assert!(
        call(
            dir.path(),
            "auth_login",
            json!({"name": "Sima", "password": password}),
        )
        .unwrap_err()
        .contains("wrong")
    );
    assert_eq!(
        must(
            dir.path(),
            "auth_login",
            json!({"name": "Sima", "password": "new-owner-pass-9"}),
        )["role"],
        "owner"
    );
}

#[test]
fn till_close_records_expected_cash_without_a_journal() {
    let dir = tempfile::tempdir().unwrap();
    let (c, _) = company(dir.path());
    let before = must(dir.path(), "workspace", json!({"company_id": c}))["journal_count"]
        .as_str()
        .unwrap()
        .to_owned();
    let opened = must(
        dir.path(),
        "pos_shift_open",
        json!({
            "company_id": c,
            "request_key": "till-1",
            "opening_cash_rial": "150000",
            "cashier": "Niloofar",
            "date": "2026-04-03"
        }),
    );
    let shift_id = opened["shift"]["id"].as_str().unwrap();
    let closed = must(
        dir.path(),
        "pos_shift_close",
        json!({
            "company_id": c,
            "shift_id": shift_id,
            "counted_cash_rial": "150000",
            "date": "2026-04-03",
            "note": "Drawer counted"
        }),
    );
    assert_eq!(closed["ledger_posted"], false);
    assert_eq!(closed["shift"]["expected_cash_rial"], "150000");
    assert_eq!(closed["shift"]["variance_rial"], "0");
    let after = must(dir.path(), "workspace", json!({"company_id": c}));
    assert_eq!(after["journal_count"], before);
    assert!(after["retail"]["open_shift"].is_null());
}

#[test]
fn concurrent_last_unit_sells_once_and_return_restores_stock() {
    let dir = tempfile::tempdir().unwrap();
    let (c, p) = company(dir.path());
    let products = products_file(&[vec![
        "001".into(),
        "Tea".into(),
        "Piece".into(),
        "6261".into(),
        "Grocery".into(),
        "100000".into(),
        "60000".into(),
        "0".into(),
    ]]);
    must(
        dir.path(),
        "excel_commit",
        json!({
            "kind": "products",
            "content_base64": STANDARD.encode(&products),
            "company_id": c,
            "period_id": p,
            "date": "2026-04-01",
            "request_key": "sku",
            "policy": "skip",
            "confirm": true
        }),
    );
    let opening = sheet(
        &["sku", "name", "unit", "warehouse", "quantity", "unit_cost_rial"],
        &[vec![
            "001".into(),
            "Tea".into(),
            "Piece".into(),
            "Main".into(),
            "1".into(),
            "60000".into(),
        ]],
        false,
    );
    must(
        dir.path(),
        "excel_commit",
        json!({
            "kind": "opening_stock",
            "content_base64": STANDARD.encode(&opening),
            "company_id": c,
            "period_id": p,
            "date": "2026-04-02",
            "request_key": "open",
            "policy": "skip",
            "confirm": true
        }),
    );
    must(
        dir.path(),
        "resource",
        json!({
            "kind": "customer",
            "id": "walk-in",
            "name": "Walk-in",
            "company_id": c,
            "period_id": p,
            "source_id": "walk-in",
            "request_key": "walk-in",
            "date": "2026-04-03",
            "reason": "Counter customer"
        }),
    );
    let workspace = must(dir.path(), "workspace", json!({"company_id": c}));
    let item = workspace["resources"]["items"][0]["id"].as_str().unwrap();
    let unit = workspace["resources"]["items"][0]["base_unit_id"]
        .as_str()
        .unwrap();
    let warehouse = workspace["resources"]["warehouses"][0]["id"]
        .as_str()
        .unwrap();
    let setup = workspace["setups"][0]["id"].as_str().unwrap();
    let mut orders = Vec::new();
    for n in 0..2 {
        let id = format!("draft-{n}");
        let draft = json!({
            "id": id,
            "revision": "0",
            "company_id": c,
            "period_id": p,
            "setup_id": setup,
            "customer_id": "walk-in",
            "reference": format!("ORD-{n}"),
            "document_date": "2026-04-03",
            "financial_date": "2026-04-03",
            "date_display": "Iso",
            "display_unit": "Rial",
            "document_discount": "0",
            "supplied_total": "100000",
            "reason": "Last unit",
            "quotation": null,
            "lines": [{
                "id": format!("line-{n}"),
                "item_id": item,
                "warehouse_id": warehouse,
                "unit_id": unit,
                "quantity": "1",
                "unit_price": "100000",
                "line_discount": "0",
                "price_reference": "Till",
                "price_date": "2026-04-03",
                "override_reason": ""
            }]
        });
        must(
            dir.path(),
            "draft_save",
            json!({"draft": draft, "expected_revision": null}),
        );
        let posted = must(
            dir.path(),
            "draft_post",
            json!({"id": id, "revision": "1", "request_key": format!("post-{n}")}),
        );
        orders.push((
            posted["record_id"].as_str().unwrap().to_owned(),
            format!("line-{n}"),
        ));
    }
    let path = dir.path().to_path_buf();
    let mut handles = Vec::new();
    for (n, (order_id, line_id)) in orders.into_iter().enumerate() {
        let path = path.clone();
        let c = c.clone();
        let p = p.clone();
        handles.push(thread::spawn(move || {
            let key = format!("sale-{n}");
            let payload = json!({
                "company_id": c,
                "period_id": p,
                "source_id": key,
                "request_key": key,
                "date": "2026-04-03",
                "due_date": "2026-04-03",
                "reference": format!("INV-{n}"),
                "reason": "Last unit",
                "order_id": order_id,
                "lines": [{"id": line_id, "quantity": "1"}]
            });
            let result = call(&path, "fulfill_post", payload.clone());
            (payload, result)
        }));
    }
    let mut wins = Vec::new();
    let mut losses = 0;
    for handle in handles {
        match handle.join().unwrap() {
            (payload, Ok(value)) => wins.push((payload, value)),
            (_, Err(_)) => losses += 1,
        }
    }
    assert_eq!(wins.len(), 1, "one sale of the last unit");
    assert_eq!(losses, 1);
    let (winner_payload, winner) = &wins[0];
    let retry = must(dir.path(), "fulfill_post", winner_payload.clone());
    assert_eq!(retry["record_id"], winner["record_id"]);
    let sold = must(
        dir.path(),
        "report",
        json!({"company_id": c, "from": "2026-03-21", "to": "2026-04-03"}),
    );
    assert_eq!(sold["trial"]["total_debit"], sold["trial"]["total_credit"]);
    assert_eq!(
        sold["reconciliation"]["stock"][0]["quantity_thousandths"],
        "0"
    );
    let invoice_id = winner["record_id"].as_str().unwrap();
    let options = must(
        dir.path(),
        "operation_options",
        json!({"company_id": c, "record_id": invoice_id, "operation": "sale_return"}),
    );
    let returned = must(
        dir.path(),
        "sale_return_post",
        json!({
            "company_id": c,
            "period_id": p,
            "source_id": "return-1",
            "request_key": "return-1",
            "date": "2026-04-03",
            "reason": "Customer changed their mind",
            "reference": "RET-1",
            "record_id": invoice_id,
            "lines": [{"id": options["lines"][0]["id"], "quantity": "1"}]
        }),
    );
    let replay = must(
        dir.path(),
        "sale_return_post",
        json!({
            "company_id": c,
            "period_id": p,
            "source_id": "return-1",
            "request_key": "return-1",
            "date": "2026-04-03",
            "reason": "Customer changed their mind",
            "reference": "RET-1",
            "record_id": invoice_id,
            "lines": [{"id": options["lines"][0]["id"], "quantity": "1"}]
        }),
    );
    assert_eq!(replay["record_id"], returned["record_id"]);
    let restored = must(
        dir.path(),
        "report",
        json!({"company_id": c, "from": "2026-03-21", "to": "2026-04-03"}),
    );
    assert_eq!(
        restored["trial"]["total_debit"],
        restored["trial"]["total_credit"]
    );
    assert_eq!(
        restored["reconciliation"]["stock"][0]["quantity_thousandths"],
        "1000"
    );
}

#[test]
fn workbook_limits_cover_declared_and_actual_expansion() {
    let dir = tempfile::tempdir().unwrap();
    let cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for index in 0..9 {
        zip.start_file(format!("pad{index}.bin"), options).unwrap();
        zip.write_all(&vec![b'A'; 1_000_000]).unwrap();
    }
    let bytes = zip.finish().unwrap().into_inner();
    assert!(bytes.len() < 1_000_000, "fixture must stay under the compressed cap");
    let error = call(
        dir.path(),
        "excel_preview",
        json!({"kind": "products", "content_base64": STANDARD.encode(&bytes)}),
    )
    .unwrap_err();
    assert!(error.contains("expands"), "{error}");

    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file("xl/externalLinks/link.xml", options).unwrap();
    zip.write_all(b"<link/>").unwrap();
    let bytes = zip.finish().unwrap().into_inner();
    let error = call(
        dir.path(),
        "excel_preview",
        json!({"kind": "products", "content_base64": STANDARD.encode(&bytes)}),
    )
    .unwrap_err();
    assert!(error.contains("external"), "{error}");
}

#[test]
fn legacy_agent_token_cannot_pass_a_shop_lock_until_the_owner_grants_a_role() {
    let dir = tempfile::tempdir().unwrap();
    let (c, _) = company(dir.path());
    let owner_password = "owner-passphrase-1";
    let cashier_password = "cashier-passphrase-1";
    let boot = must(
        dir.path(),
        "auth_bootstrap",
        json!({"name": "Owner", "password": owner_password}),
    );
    let owner_session = boot["session_token"].as_str().unwrap();
    assert!(!boot.to_string().contains(owner_password));
    let service = AgentService::start(dir.path(), env!("CARGO_BIN_EXE_sarv-agent").into()).unwrap();
    service.configure(true, Access::Full).unwrap();
    let denied = agent_call(dir.path(), "workspace", json!({"company_id": c})).unwrap_err();
    assert!(denied.contains("approve"), "{denied}");
    let configured = service
        .ui_dispatch(
            "agent_configure",
            json!({
                "session_token": owner_session,
                "enabled": true,
                "access": "full",
                "role": "accountant"
            }),
        )
        .unwrap();
    assert_eq!(configured["role"], "accountant");
    let workspace = agent_call(dir.path(), "workspace", json!({"company_id": c})).unwrap();
    assert!(workspace.get("error").is_none());
    let people = must(
        dir.path(),
        "auth_status",
        json!({"session_token": owner_session}),
    )["people"]
        .as_array()
        .unwrap()
        .len();
    assert!(
        agent_call(
            dir.path(),
            "auth_create_user",
            json!({"name": "Other", "password": cashier_password, "role": "cashier"}),
        )
        .is_err()
    );
    assert_eq!(
        must(
            dir.path(),
            "auth_status",
            json!({"session_token": owner_session}),
        )["people"]
            .as_array()
            .unwrap()
            .len(),
        people
    );
    must(
        dir.path(),
        "auth_create_user",
        json!({
            "session_token": owner_session,
            "name": "Till",
            "password": cashier_password,
            "role": "cashier"
        }),
    );
    let cashier = must(
        dir.path(),
        "auth_login",
        json!({"name": "Till", "password": cashier_password}),
    );
    let blocked = service.ui_dispatch(
        "agent_configure",
        json!({
            "session_token": cashier["session_token"],
            "enabled": true,
            "access": "full",
            "role": "accountant"
        }),
    );
    assert!(blocked.unwrap_err().to_string().contains("role"));
    service
        .ui_dispatch(
            "agent_configure",
            json!({"session_token": owner_session, "enabled": false, "access": "read_only"}),
        )
        .unwrap();
    let after = agent_call(dir.path(), "workspace", json!({"company_id": c}));
    assert!(after.is_err());
}

#[test]
fn graph_connector_uses_pkce_and_does_not_echo_tokens() {
    let _guard = graph_env().lock().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let seen = std::sync::Arc::new(Mutex::new(Vec::new()));
    let record = seen.clone();
    thread::spawn(move || {
        for incoming in listener.incoming().take(6) {
            let mut stream = incoming.unwrap();
            let mut buf = [0u8; 4096];
            let n = stream.read(&mut buf).unwrap_or(0);
            let text = String::from_utf8_lossy(&buf[..n]);
            let path = text.split_whitespace().nth(1).unwrap_or("");
            let authed = text.to_ascii_lowercase().contains("authorization:");
            record.lock().unwrap().push(format!("{path} {authed}"));
            let response = if path.starts_with("/token") || path.starts_with("/revoke") {
                let body = br#"{"access_token":"at-1","refresh_token":"rt-1","expires_in":3600,"token_type":"Bearer"}"#;
                let mut head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .into_bytes();
                head.extend_from_slice(body);
                head
            } else if path.contains("/content") {
                format!("HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:{port}/blob\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").into_bytes()
            } else {
                let file = products_file(&[vec![
                    "00123".into(),
                    "Tea".into(),
                    "Piece".into(),
                    "626".into(),
                    "Grocery".into(),
                    "1000".into(),
                    "500".into(),
                    "0".into(),
                ]]);
                let mut head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    file.len()
                )
                .into_bytes();
                head.extend(file);
                head
            };
            let _ = stream.write_all(&response);
        }
    });
    unsafe {
        std::env::set_var("SARV_GRAPH_CLIENT_ID", "client-1");
        std::env::set_var("SARV_GRAPH_TENANT", "common");
        std::env::set_var("SARV_GRAPH_REDIRECT_URI", "http://127.0.0.1/cb");
        std::env::set_var("SARV_GRAPH_AUTHORIZE_BASE", format!("http://127.0.0.1:{port}"));
        std::env::set_var("SARV_GRAPH_TOKEN_URL", format!("http://127.0.0.1:{port}/token"));
        std::env::set_var("SARV_GRAPH_API_BASE", format!("http://127.0.0.1:{port}/v1.0"));
    }
    let dir = tempfile::tempdir().unwrap();
    let (c, p) = company(dir.path());
    let absent = must(dir.path(), "excel_graph_status", json!({}));
    assert_eq!(absent["configured"], true);
    assert_eq!(absent["authenticated"], false);
    assert_eq!(absent["live_verified"], false);
    let begin = must(dir.path(), "excel_graph_begin", json!({}));
    let url = begin["authorize_url"].as_str().unwrap();
    assert!(url.contains("code_challenge="));
    assert!(url.contains("code_challenge_method=S256"));
    assert!(url.contains("Files.SelectedOperations.Selected"));
    assert!(!url.contains("Files.ReadWrite"));
    assert!(url.contains("127.0.0.1"));
    assert!(!url.contains("code_verifier"));
    let mismatch = call(
        dir.path(),
        "excel_graph_complete",
        json!({"redirect_url": format!("http://127.0.0.1/cb?code=one&state=nope")}),
    );
    assert!(mismatch.unwrap_err().contains("state"));
    let state = url.split("state=").nth(1).unwrap().split('&').next().unwrap();
    must(
        dir.path(),
        "excel_graph_complete",
        json!({"redirect_url": format!("http://127.0.0.1/cb?code=one&state={state}")}),
    );
    let signed = must(dir.path(), "excel_graph_status", json!({}));
    let rendered = signed.to_string();
    assert_eq!(signed["authenticated"], true);
    assert!(!rendered.contains("rt-1") && !rendered.contains("at-1"));
    let preview = must(
        dir.path(),
        "excel_graph_preview",
        json!({"kind": "products", "item_id": "ITEM1", "company_id": c, "period_id": p}),
    );
    assert_eq!(preview["accepted"], "1");
    assert_eq!(preview["live_verified"], false);
    let hits = seen.lock().unwrap().clone();
    assert!(hits.iter().any(|hit| hit.contains("/blob false")), "{hits:?}");
    must(dir.path(), "excel_graph_revoke", json!({}));
    let cleared = must(dir.path(), "excel_graph_status", json!({}));
    assert_eq!(cleared["authenticated"], false);
    unsafe {
        std::env::remove_var("SARV_GRAPH_CLIENT_ID");
        std::env::remove_var("SARV_GRAPH_TENANT");
        std::env::remove_var("SARV_GRAPH_REDIRECT_URI");
        std::env::remove_var("SARV_GRAPH_AUTHORIZE_BASE");
        std::env::remove_var("SARV_GRAPH_TOKEN_URL");
        std::env::remove_var("SARV_GRAPH_API_BASE");
    }
}

#[test]
fn graph_refuses_an_ungranted_workbook_without_echoing_tokens() {
    let _guard = graph_env().lock().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for incoming in listener.incoming().take(4) {
            let mut stream = incoming.unwrap();
            let mut buf = [0u8; 4096];
            let n = stream.read(&mut buf).unwrap_or(0);
            let text = String::from_utf8_lossy(&buf[..n]);
            let path = text.split_whitespace().nth(1).unwrap_or("").to_owned();
            let (status, extra, body): (u16, String, Vec<u8>) = if path.starts_with("/token") {
                (
                    200,
                    String::new(),
                    br#"{"access_token":"at-secret","refresh_token":"rt-secret","expires_in":3600}"#.to_vec(),
                )
            } else if path.contains("RATE") {
                (429, String::new(), br#"{"error":"access_token=at-secret"}"#.to_vec())
            } else if path.contains("META") {
                (
                    302,
                    "Location: http://169.254.169.254/latest\r\n".into(),
                    Vec::new(),
                )
            } else {
                (403, String::new(), br#"{"error":"access_token=at-secret"}"#.to_vec())
            };
            let head = format!(
                "HTTP/1.1 {status} x\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&body);
        }
    });
    unsafe {
        std::env::set_var("SARV_GRAPH_CLIENT_ID", "client-1");
        std::env::set_var("SARV_GRAPH_TENANT", "common");
        std::env::set_var("SARV_GRAPH_REDIRECT_URI", "http://127.0.0.1/cb");
        std::env::set_var("SARV_GRAPH_AUTHORIZE_BASE", format!("http://127.0.0.1:{port}"));
        std::env::set_var("SARV_GRAPH_TOKEN_URL", format!("http://127.0.0.1:{port}/token"));
        std::env::set_var("SARV_GRAPH_API_BASE", format!("http://127.0.0.1:{port}/v1.0"));
    }
    let dir = tempfile::tempdir().unwrap();
    let (c, p) = company(dir.path());
    let begin = must(dir.path(), "excel_graph_begin", json!({}));
    let url = begin["authorize_url"].as_str().unwrap();
    let state = url.split("state=").nth(1).unwrap().split('&').next().unwrap();
    must(
        dir.path(),
        "excel_graph_complete",
        json!({"redirect_url": format!("http://127.0.0.1/cb?code=one&state={state}")}),
    );
    for (item, needle) in [
        ("DENIED", "not granted"),
        ("RATE", "wait"),
        ("META", "Redirect"),
    ] {
        let error = call(
            dir.path(),
            "excel_graph_preview",
            json!({"kind": "products", "item_id": item, "company_id": c, "period_id": p}),
        )
        .unwrap_err();
        assert!(error.contains(needle), "{error}");
        assert!(!error.contains("at-secret"), "{error}");
    }
    unsafe {
        std::env::remove_var("SARV_GRAPH_CLIENT_ID");
        std::env::remove_var("SARV_GRAPH_TENANT");
        std::env::remove_var("SARV_GRAPH_REDIRECT_URI");
        std::env::remove_var("SARV_GRAPH_AUTHORIZE_BASE");
        std::env::remove_var("SARV_GRAPH_TOKEN_URL");
        std::env::remove_var("SARV_GRAPH_API_BASE");
    }
}

#[test]
fn restore_refuses_an_unlocked_profile_and_keeps_the_existing_lock() {
    let dir = tempfile::tempdir().unwrap();
    company(dir.path());
    let refused = call(
        dir.path(),
        "backup_restore",
        json!({"filename": "sarv-backup-missing.json", "confirmed": true}),
    )
    .unwrap_err();
    assert!(refused.contains("password"), "{refused}");
    let boot = must(
        dir.path(),
        "auth_bootstrap",
        json!({"name": "Owner", "password": "sample-shop-pass"}),
    );
    let token = boot["session_token"].as_str().unwrap().to_owned();
    let recovery = boot["recovery_code"].as_str().unwrap().to_owned();
    assert_ne!(recovery, "sample-shop-pass");
    let lock = sarv_quote::store::Store::open(dir.path())
        .unwrap()
        .security_payload()
        .unwrap();
    assert!(!lock.contains("sample-shop-pass"));
    let backup = must(dir.path(), "backup_create", json!({"session_token": token}));
    let filename = backup["filename"].as_str().unwrap().to_owned();
    let text = std::fs::read_to_string(dir.path().join(&filename)).unwrap();
    assert!(!text.contains("password_hash"));
    assert!(!text.contains(&recovery));
    assert!(!text.contains(&token));
    let other = tempfile::tempdir().unwrap();
    std::fs::copy(dir.path().join(&filename), other.path().join(&filename)).unwrap();
    let unlocked = call(
        other.path(),
        "backup_restore",
        json!({"filename": filename, "confirmed": true}),
    )
    .unwrap_err();
    assert!(unlocked.contains("password"), "{unlocked}");
    let other_boot = must(
        other.path(),
        "auth_bootstrap",
        json!({"name": "Other", "password": "other-shop-pass-1"}),
    );
    let other_token = other_boot["session_token"].as_str().unwrap().to_owned();
    let other_lock = sarv_quote::store::Store::open(other.path())
        .unwrap()
        .security_payload()
        .unwrap();
    assert_ne!(other_lock, lock);
    must(
        other.path(),
        "backup_restore",
        json!({"filename": filename, "confirmed": true, "session_token": other_token}),
    );
    assert_eq!(
        sarv_quote::store::Store::open(other.path())
            .unwrap()
            .security_payload()
            .unwrap(),
        other_lock
    );
    assert_eq!(
        sarv_quote::store::Store::open(dir.path())
            .unwrap()
            .security_payload()
            .unwrap(),
        lock
    );
    must(
        dir.path(),
        "auth_create_user",
        json!({"session_token": token, "name": "Till", "password": "cashier-pass-1", "role": "cashier"}),
    );
    let till = must(
        dir.path(),
        "auth_login",
        json!({"name": "Till", "password": "cashier-pass-1"}),
    );
    let denied = call(
        dir.path(),
        "backup_restore",
        json!({
            "filename": filename,
            "confirmed": true,
            "session_token": till["session_token"]
        }),
    )
    .unwrap_err();
    assert!(denied.contains("role"), "{denied}");
    let reset = must(
        dir.path(),
        "auth_reset",
        json!({"recovery_code": recovery, "password": "replaced-shop-pass"}),
    );
    assert_eq!(reset["reset"], true);
    assert!(
        call(
            dir.path(),
            "auth_login",
            json!({"name": "Owner", "password": "sample-shop-pass"}),
        )
        .is_err()
    );
    must(
        dir.path(),
        "auth_login",
        json!({"name": "Owner", "password": "replaced-shop-pass"}),
    );
}
fn graph_env() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn agent_call(dir: &Path, action: &str, payload: Value) -> Result<Value, String> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sarv-agent"))
        .args([
            "--connection",
            dir.join(CONNECTION).to_str().unwrap(),
            "call",
            action,
            "--stdin",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    serde_json::to_writer(child.stdin.take().unwrap(), &payload).unwrap();
    let output = child.wait_with_output().unwrap();
    let body = String::from_utf8_lossy(&output.stdout);
    if output.status.success() {
        Ok(serde_json::from_str(&body).unwrap_or(json!({})))
    } else {
        Err(format!("{body} {}", String::from_utf8_lossy(&output.stderr)))
    }
}
