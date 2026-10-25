use sarv_quote::{
    agent::{self, Access, AgentService, CONNECTION},
    desktop,
    store::Store,
};
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    path::Path,
    process::{Command, Stdio},
};

fn service(dir: &Path) -> AgentService {
    AgentService::start(dir, env!("CARGO_BIN_EXE_sarv-agent").into()).unwrap()
}
fn cli(dir: &Path, action: &str, payload: Value) -> Value {
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
    assert!(
        output.status.success(),
        "CLI {action}: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn post(dir: &Path, prefix: &str, mut payload: Value) -> Value {
    let preview = cli(dir, &format!("{prefix}_preview"), payload.clone());
    payload["expected_generation"] = preview["_generation"].clone();
    cli(dir, &format!("{prefix}_post"), payload)
}
fn context(company: &str, period: &str, key: &str) -> Value {
    json!({"company_id":company,"period_id":period,"source_id":key,"request_key":key,"date":"2026-10-10","reason":"Agent integration test","reference":key,"due_date":"2026-10-30"})
}
#[test]
fn access_is_off_by_default_readonly_cannot_write_and_agents_cannot_grant_access() {
    let dir = tempfile::tempdir().unwrap();
    let service = service(dir.path());
    assert_eq!(service.status().unwrap()["enabled"], false);
    assert!(!dir.path().join(CONNECTION).exists());
    assert!(agent::call(&dir.path().join(CONNECTION), "workspace", json!({})).is_err());
    service.configure(true, Access::ReadOnly).unwrap();
    let generation = Store::open(dir.path()).unwrap().local_generation();
    let connection = dir.path().join(CONNECTION);
    let workspace = agent::call(&connection, "workspace", json!({})).unwrap();
    let connection_value: Value = serde_json::from_slice(&fs::read(&connection).unwrap()).unwrap();
    let token = connection_value["token"].as_str().unwrap();
    assert!(!workspace.to_string().contains(token));
    assert!(!service.status().unwrap().to_string().contains(token));
    assert!(
        !agent::call(&connection, "tools", json!({}))
            .unwrap()
            .to_string()
            .contains(token)
    );
    assert!(
        agent::call(&connection, "setup_company", json!({}))
            .unwrap_err()
            .to_string()
            .contains("Read-only")
    );
    service.configure(true, Access::Full).unwrap();
    for action in [
        "agent_configure",
        "agent_status",
        "seed_fixture",
        "unknown",
        "../../books",
    ] {
        assert!(
            agent::call(&connection, action, json!({"enabled":true,"access":"full"}))
                .unwrap_err()
                .to_string()
                .contains("Unsupported")
        );
    }
    assert_eq!(
        generation,
        Store::open(dir.path()).unwrap().local_generation()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&connection).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}
#[test]
fn full_cli_goods_workflow_keeps_exact_values_original_costs_and_audit() {
    let dir = tempfile::tempdir().unwrap();
    let service = service(dir.path());
    service.configure(true, Access::Full).unwrap();
    let created = cli(
        dir.path(),
        "setup_company",
        json!({"name":"Agent books","fiscal_name":"2026","start_date":"2026-01-01","end_date":"2026-12-31"}),
    );
    let c = created["company_id"].as_str().unwrap();
    let p = created["period_id"].as_str().unwrap();
    cli(
        dir.path(),
        "initialize_zero",
        json!({"company_id":c,"source_id":"zero","request_key":"zero","confirmed":true}),
    );
    for (kind, id) in [
        ("customer", "customer"),
        ("vendor", "vendor"),
        ("unit", "unit"),
        ("warehouse", "main"),
        ("warehouse", "other"),
        ("item", "item"),
    ] {
        let mut payload = context(c, p, id);
        payload["kind"] = json!(kind);
        payload["id"] = json!(id);
        payload["name"] = json!(id);
        payload["unit_id"] = json!("unit");
        payload["sku"] = json!("sku");
        cli(dir.path(), "resource", payload);
    }
    let mut po = context(c, p, "purchase");
    po["supplier_id"] = json!("vendor");
    po["lines"] = json!([{"id":"po-line","item_id":"item","warehouse_id":"main","unit_id":"unit","quantity":"3","unit_price":"11","line_discount":"0"}]);
    let order = post(dir.path(), "purchase", po);
    let mut receipts = Vec::new();
    let mut bills = Vec::new();
    for (key, quantity) in [("first", "1"), ("remaining", "2")] {
        let mut receipt = context(c, p, &format!("receipt-{key}"));
        receipt["order_id"] = order["record_id"].clone();
        receipt["lines"] = json!([{"id":"po-line","quantity":quantity}]);
        let received = post(dir.path(), "receive", receipt);
        let mut bill = context(c, p, &format!("bill-{key}"));
        bill["receipt_id"] = received["record_id"].clone();
        bills.push(post(dir.path(), "bill", bill));
        receipts.push(received);
    }
    let workspace = cli(dir.path(), "workspace", json!({"company_id":c}));
    let draft = json!({"id":"draft","revision":"0","company_id":c,"period_id":p,"setup_id":workspace["setups"][0]["id"],"customer_id":"customer","reference":"sale","document_date":"2026-10-10","financial_date":"2026-10-10","date_display":"Iso","display_unit":"Rial","document_discount":"1","supplied_total":"0","reason":"Agent sale","quotation":null,"lines":[{"id":"sale-line","item_id":"item","warehouse_id":"main","unit_id":"unit","quantity":"2","unit_price":"20","line_discount":"0","price_reference":"Manual","price_date":"2026-10-10","override_reason":""}]});
    let mut draft = draft;
    let review = cli(dir.path(), "draft_preview", json!({"draft":draft}));
    assert_eq!(review["supplied_total"], "39");
    draft["supplied_total"] = review["supplied_total"].clone();
    let revision = cli(
        dir.path(),
        "draft_save",
        json!({"draft":draft,"expected_revision":null}),
    );
    let sales = cli(
        dir.path(),
        "draft_post",
        json!({"id":"draft","revision":revision,"request_key":"sale-post"}),
    );
    let mut delivery = context(c, p, "delivery");
    delivery["order_id"] = sales["record_id"].clone();
    let invoice = post(dir.path(), "fulfill", delivery);
    let mut returned = context(c, p, "customer-return");
    returned["record_id"] = invoice["record_id"].clone();
    let options = cli(
        dir.path(),
        "operation_options",
        json!({"company_id":c,"record_id":invoice["record_id"],"operation":"sale_return"}),
    );
    returned["lines"] = json!([{"id":options["lines"][0]["id"],"quantity":"1"}]);
    post(dir.path(), "sale_return", returned);
    let mut returned = context(c, p, "supplier-return");
    returned["record_id"] = bills[0]["record_id"].clone();
    post(dir.path(), "purchase_return", returned);
    let mut transfer = context(c, p, "transfer");
    transfer["item_id"] = json!("item");
    transfer["source_warehouse_id"] = json!("main");
    transfer["target_warehouse_id"] = json!("other");
    transfer["unit_id"] = json!("unit");
    transfer["quantity"] = json!("1");
    post(dir.path(), "transfer", transfer);
    let report = cli(
        dir.path(),
        "report",
        json!({"company_id":c,"from":"2026-01-01","to":"2026-10-10"}),
    );
    assert_eq!(report["position"]["receivables"], "19");
    assert_eq!(report["position"]["payables"], "22");
    assert_eq!(report["profit_loss"]["profit"], "8");
    assert_eq!(report["reconciliation"]["reconciled"], true);
    let stock = report["reconciliation"]["stock"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["warehouse_id"] == "other")
        .unwrap();
    assert_eq!(stock["quantity_thousandths"], "1000");
    assert_eq!(stock["value"], "11");
    let saved = Store::open(dir.path()).unwrap();
    assert!(
        saved
            .state
            .accounting
            .journals
            .iter()
            .all(|j| j.actor == "local-agent")
    );
    assert!(
        cli(
            dir.path(),
            "report_csv",
            json!({"company_id":c,"from":"2026-01-01","to":"2026-10-10"})
        )["content"]
            .as_str()
            .unwrap()
            .contains("Debit")
    );
    let _ = receipts;
}
#[test]
fn preview_generation_period_locks_and_idempotent_retries_protect_agent_posts() {
    let dir = tempfile::tempdir().unwrap();
    desktop::seed_fixture(dir.path()).unwrap();
    let service = service(dir.path());
    service.configure(true, Access::Full).unwrap();
    let s = Store::open(dir.path()).unwrap();
    let c = &s.state.accounting.companies[0].id;
    let p = &s.state.accounting.periods[0].id;
    let connection = dir.path().join(CONNECTION);
    let account = s
        .state
        .accounting
        .accounts
        .iter()
        .find(|a| a.postable && !a.control)
        .unwrap();
    let second = s
        .state
        .accounting
        .accounts
        .iter()
        .find(|a| a.postable && !a.control && a.id != account.id)
        .unwrap();
    let mut journal = context(c, p, "journal");
    journal["lines"] = json!([{"account_id":account.id,"debit":"9007199254740993","credit":"0"},{"account_id":second.id,"debit":"0","credit":"9007199254740993"}]);
    let before = s.local_generation();
    assert!(
        agent::call(&connection, "journal_post", journal.clone())
            .unwrap_err()
            .to_string()
            .contains("Preview first")
    );
    let preview = agent::call(&connection, "journal_preview", journal.clone()).unwrap();
    assert_eq!(preview["total_debit"], "9007199254740993");
    assert_eq!(Store::open(dir.path()).unwrap().local_generation(), before);
    journal["expected_generation"] = preview["_generation"].clone();
    service
        .ui_dispatch(
            "period_lock",
            json!({"company_id":c,"period_id":p,"locked":true,"reason":"Lock test"}),
        )
        .unwrap();
    assert!(
        agent::call(&connection, "journal_post", journal.clone())
            .unwrap_err()
            .to_string()
            .contains("Books changed")
    );
    assert!(agent::call(&connection, "journal_preview", journal.clone()).is_err());
    service
        .ui_dispatch(
            "period_lock",
            json!({"company_id":c,"period_id":p,"locked":false,"reason":"Reopen test"}),
        )
        .unwrap();
    let preview = agent::call(&connection, "journal_preview", journal.clone()).unwrap();
    journal["expected_generation"] = preview["_generation"].clone();
    let id = agent::call(&connection, "journal_post", journal.clone()).unwrap();
    assert_eq!(
        agent::call(&connection, "journal_post", journal.clone()).unwrap(),
        id
    );
    journal["reason"] = json!("Changed retry");
    assert!(agent::call(&connection, "journal_post", journal).is_err());
    let mut reverse = json!({"company_id":c,"period_id":p,"journal_id":id,"request_key":"reverse","date":"2026-10-10","reason":"Reverse test"});
    let preview = agent::call(&connection, "journal_reverse_preview", reverse.clone()).unwrap();
    reverse["expected_generation"] = preview["_generation"].clone();
    agent::call(&connection, "journal_reverse", reverse).unwrap();
}
#[test]
fn revocation_restart_and_second_instance_expire_existing_connections() {
    let dir = tempfile::tempdir().unwrap();
    let first = service(dir.path());
    first.configure(true, Access::Full).unwrap();
    let connection = dir.path().join(CONNECTION);
    let stale = dir.path().join("stale.json");
    fs::copy(&connection, &stale).unwrap();
    first.configure(false, Access::Full).unwrap();
    assert!(!connection.exists());
    assert!(agent::call(&stale, "workspace", json!({})).is_err());
    first.configure(true, Access::Full).unwrap();
    assert!(agent::call(&stale, "workspace", json!({})).is_err());
    fs::copy(&connection, &stale).unwrap();
    let second = service(dir.path());
    assert!(agent::call(&stale, "workspace", json!({})).is_err());
    assert!(agent::call(&connection, "workspace", json!({})).is_ok());
    drop(first);
    assert!(connection.exists());
    second.configure(true, Access::ReadOnly).unwrap();
    assert!(agent::call(&connection, "setup_company", json!({})).is_err());
    drop(second);
    assert!(!connection.exists());
    let restarted = service(dir.path());
    assert_eq!(restarted.status().unwrap()["access"], "read_only");
    restarted.configure(false, Access::ReadOnly).unwrap();
}
#[test]
fn transport_rejects_forged_tokens_remote_endpoints_and_corrupt_policy() {
    let dir = tempfile::tempdir().unwrap();
    let service = service(dir.path());
    service.configure(true, Access::Full).unwrap();
    let path = dir.path().join(CONNECTION);
    let original = fs::read(&path).unwrap();
    let mut connection: Value = serde_json::from_slice(&original).unwrap();
    let mut stream = TcpStream::connect(connection["address"].as_str().unwrap()).unwrap();
    writeln!(
        stream,
        "{}",
        json!({"token":"forged","action":"workspace","payload":{}})
    )
    .unwrap();
    let mut response = String::new();
    BufReader::new(stream).read_line(&mut response).unwrap();
    assert!(response.contains("unauthorized"));
    connection["address"] = json!("192.0.2.1:12345");
    fs::write(&path, serde_json::to_vec(&connection).unwrap()).unwrap();
    assert!(
        agent::call(&path, "workspace", json!({}))
            .unwrap_err()
            .to_string()
            .contains("loopback")
    );
    fs::write(&path, original).unwrap();
    fs::write(dir.path().join(".agent-access.json"), b"corrupt").unwrap();
    assert!(agent::call(&path, "workspace", json!({})).is_err());
}
#[test]
fn mcp_supports_initialization_discovery_calls_notifications_and_protocol_errors() {
    let dir = tempfile::tempdir().unwrap();
    let service = service(dir.path());
    service.configure(true, Access::ReadOnly).unwrap();
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"any-agent","version":"1"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"sarv_workspace","arguments":{}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"sarv_setup_company","arguments":{}}}),
        json!({"jsonrpc":"2.0","id":5,"method":"unsupported"}),
    ];
    let input = messages
        .iter()
        .map(|v| format!("{v}\n"))
        .collect::<String>()
        + "{malformed\n";
    let mut output = Vec::new();
    agent::mcp(
        &dir.path().join(CONNECTION),
        std::io::Cursor::new(input),
        &mut output,
    )
    .unwrap();
    let responses: Vec<Value> = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(responses.len(), 6);
    assert_eq!(responses[0]["result"]["serverInfo"]["name"], "sarv");
    assert_eq!(
        responses[1]["result"]["tools"].as_array().unwrap().len(),
        44
    );
    assert_eq!(responses[2]["result"]["isError"], false);
    assert_eq!(responses[3]["result"]["isError"], true);
    assert_eq!(responses[4]["error"]["code"], -32601);
    assert_eq!(responses[5]["error"]["code"], -32700);
    let tools = agent::tools();
    let tools = tools.as_array().unwrap();
    for action in [
        "sarv_receive_post",
        "sarv_sale_return_post",
        "sarv_purchase_return_post",
    ] {
        let schema = &tools.iter().find(|t| t["name"] == action).unwrap()["inputSchema"];
        assert!(
            schema["properties"]["lines"]["items"]["properties"]
                .get("id")
                .is_some()
        );
        assert!(
            schema["required"]
                .as_array()
                .unwrap()
                .contains(&json!("expected_generation"))
        );
    }
}
