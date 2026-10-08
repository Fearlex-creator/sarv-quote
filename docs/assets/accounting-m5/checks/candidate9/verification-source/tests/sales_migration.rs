use rusqlite::{Connection, params};
use sarv_quote::{
    accounting::MoneyUnit,
    domain::{CURRENT_SCHEMA, State},
    imports::hash,
    sales_draft::{RawSalesDraft, RawSalesLine},
    store::Store,
};
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
fn seed(path: &Path, state: &State) {
    let db = Connection::open(path.join("sarv.sqlite3")).unwrap();
    db.execute_batch("CREATE TABLE state(id INTEGER PRIMARY KEY CHECK(id=1),generation INTEGER NOT NULL,payload TEXT NOT NULL)").unwrap();
    db.execute(
        "INSERT INTO state VALUES(1,91,?1)",
        params![serde_json::to_string(state).unwrap()],
    )
    .unwrap();
}
fn row(path: &Path) -> (i64, String) {
    Connection::open(path.join("sarv.sqlite3"))
        .unwrap()
        .query_row("SELECT generation,payload FROM state", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap()
}
fn output(name: &str, bytes: &[u8]) {
    if let Ok(path) = std::env::var("SARV_M5_MIGRATION_ARTIFACT_DIR") {
        fs::create_dir_all(&path).unwrap();
        fs::write(Path::new(&path).join(name), bytes).unwrap();
    }
}
fn raw(s: &State) -> RawSalesDraft {
    let o = &s.accounting.core.openings[0].command;
    RawSalesDraft {
        id: "interrupt-raw-draft".into(),
        revision: 0,
        company_id: o.company_id.clone(),
        period_id: o.period_id.clone(),
        setup_id: o.setup_id.clone(),
        customer_id: "CU-A".into(),
        reference: "interrupt-order".into(),
        document_date: "2026-10-08".into(),
        financial_date: "2026-10-08".into(),
        date_display: sarv_quote::domain::DateDisplay::Iso,
        display_unit: MoneyUnit::Rial,
        document_discount: "1".into(),
        supplied_total: "199".into(),
        reason: "synthetic interrupted source".into(),
        quotation: None,
        lines: vec![RawSalesLine {
            id: "interrupt-line".into(),
            item_id: "ITEM-A".into(),
            warehouse_id: "W1".into(),
            unit_id: "PIECE".into(),
            quantity: "10".into(),
            unit_price: "20".into(),
            line_discount: "0".into(),
            price_reference: "synthetic".into(),
            price_date: "2026-10-08".into(),
            override_reason: String::new(),
        }],
    }
}
fn original() -> State {
    let v: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../docs/assets/accounting-m4/restore/po-receipt/procurement-v5.json"
    ))
    .unwrap();
    serde_json::from_value(v["state"].clone()).unwrap()
}
#[test]
fn actual_schema5_histories_preserve_exact_state_recovery_restore_and_one_upgrade() {
    for (label,bytes) in [
        ("core",include_bytes!("../docs/assets/accounting-m4/restore/core-preserved/core-v5.json").as_slice()),
        ("procurement",include_bytes!("../docs/assets/accounting-m4/restore/po-receipt/procurement-v5.json").as_slice()),
        ("manual-sales",include_bytes!("../docs/assets/accounting-m5/review/actual-m4-legacy/actual-m4-sales-prefix-v5.json").as_slice()),
        ("ordinary-revenue",include_bytes!("../docs/assets/accounting-m5/review/actual-m4-legacy/actual-m4-core-ordinary-revenue-v5.json").as_slice()),
    ] {
        let envelope:serde_json::Value=serde_json::from_slice(bytes).unwrap();let old:State=serde_json::from_value(envelope["state"].clone()).unwrap();assert_eq!(old.schema,5);assert_eq!(hash(&serde_json::to_vec(&old).unwrap()),envelope["checksum"].as_str().unwrap());old.validate().unwrap();
        let dir=tempfile::tempdir().unwrap();seed(dir.path(),&old);let st=Store::open(dir.path()).unwrap();let mut expected=old.clone();expected.schema=CURRENT_SCHEMA;assert_eq!(serde_json::to_vec(&st.state).unwrap(),serde_json::to_vec(&expected).unwrap());assert_eq!(st.local_generation(),92);
        let recovery=fs::read_dir(dir.path()).unwrap().map(|e|e.unwrap().path()).find(|p|p.file_name().unwrap().to_string_lossy().starts_with("before-schema6-")).unwrap();let recovery_bytes=fs::read(&recovery).unwrap();let re:serde_json::Value=serde_json::from_slice(&recovery_bytes).unwrap();assert_eq!(re["format"],"sarv-accounting-backup-v5");assert_eq!(re["state"],envelope["state"]);
        let backup=dir.path().join("current-v6.json");st.backup(&backup).unwrap();let current=fs::read(&backup).unwrap();let parsed:serde_json::Value=serde_json::from_slice(&current).unwrap();assert_eq!(parsed["format"],"sarv-accounting-backup-v6");
        let fresh=tempfile::tempdir().unwrap();let mut restored=Store::open(fresh.path()).unwrap();restored.restore(Store::restore_preview(&backup).unwrap()).unwrap();assert_eq!(serde_json::to_vec(&restored.state).unwrap(),serde_json::to_vec(&expected).unwrap());drop(st);let st=Store::open(dir.path()).unwrap();assert_eq!(st.local_generation(),92);assert_eq!(fs::read_dir(dir.path()).unwrap().filter(|e|e.as_ref().unwrap().file_name().to_string_lossy().starts_with("before-schema6-")).count(),1);
        output(&format!("{label}-original-v5.json"),bytes);output(&format!("{label}-before-schema6-v5.json"),&recovery_bytes);output(&format!("{label}-current-v6.json"),&current);
    }
}
#[test]
#[ignore = "requires retained immutable M4 executable in SARV_M4_COMPAT_BINARY"]
fn actual_m4_binary_refuses_schema6_without_changing_row() {
    let binary = std::env::var("SARV_M4_COMPAT_BINARY").unwrap();
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path(), &original());
    drop(Store::open(dir.path()).unwrap());
    let before = row(dir.path());
    let result = Command::new(&binary)
        .env("SARV_DATA_DIR", dir.path())
        .output()
        .unwrap();
    assert!(!result.status.success());
    let diagnostic = String::from_utf8_lossy(&result.stderr);
    assert!(diagnostic.contains("نسخه") || diagnostic.contains("schema"));
    assert_eq!(row(dir.path()), before);
    output("actual-m4-rejection.json",&serde_json::to_vec_pretty(&serde_json::json!({"result":"passed","old_binary":binary,"rejected_schema":6,"exit_code":result.status.code(),"stderr":diagnostic,"exact_row_unchanged":true})).unwrap());
}
#[test]
fn sales_process_child() {
    let Some(path) = std::env::var_os("SARV_M5_CHILD_PROFILE") else {
        return;
    };
    let path = Path::new(&path);
    let phase = std::env::var("SARV_M5_CHILD_PHASE").unwrap();
    let st = Store::open(path).unwrap();
    let (generation, mut state) = st.snapshot().unwrap();
    state
        .post_sales_from_draft("interrupt-raw-draft", 1, "interrupt-submit", "tester")
        .unwrap();
    let prepared = Store::prepare_candidate(generation, state).unwrap();
    if phase == "committed" {
        let _receipt = Store::commit_detached(path.to_path_buf(), prepared).unwrap();
        fs::write(path.join("child-phase"), "committed-before-adoption").unwrap();
    } else {
        fs::write(path.join("child-phase"), "prepared-before-commit").unwrap();
    }
    loop {
        std::thread::park();
    }
}
#[test]
fn killed_sales_before_commit_and_after_lost_adoption_recovers_old_or_complete_and_retries_once() {
    for phase in ["prepared", "committed"] {
        let dir = tempfile::tempdir().unwrap();
        seed(dir.path(), &original());
        let mut st = Store::open(dir.path()).unwrap();
        let d = raw(&st.state);
        st.change(|s| {
            s.save_sales_draft(d.clone(), None)?;
            Ok(())
        })
        .unwrap();
        let before = row(dir.path());
        drop(st);
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "sales_process_child", "--nocapture"])
            .env("SARV_M5_CHILD_PROFILE", dir.path())
            .env("SARV_M5_CHILD_PHASE", phase)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let started = Instant::now();
        while !dir.path().join("child-phase").exists() {
            assert!(started.elapsed() < Duration::from_secs(10));
            assert!(child.try_wait().unwrap().is_none());
            std::thread::sleep(Duration::from_millis(10));
        }
        child.kill().unwrap();
        assert!(!child.wait().unwrap().success());
        let durable = row(dir.path());
        if phase == "prepared" {
            assert_eq!(durable, before);
        } else {
            assert_eq!(durable.0, before.0 + 1);
        }
        let mut st = Store::open(dir.path()).unwrap();
        let result = st
            .change(|s| {
                s.post_sales_from_draft("interrupt-raw-draft", 1, "restart-new-key", "tester")
            })
            .unwrap();
        assert!(result.journal_id.is_none());
        assert_eq!(st.state.accounting.sales.records.len(), 1);
        assert!(st.state.sales_drafts.is_empty());
        st.state.validate().unwrap();
        let db = Connection::open(dir.path().join("sarv.sqlite3")).unwrap();
        assert_eq!(
            db.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
        output(&format!("interrupted-{phase}.json"),&serde_json::to_vec_pretty(&serde_json::json!({"result":"passed","software_phase":phase,"before_generation":before.0,"after_kill_generation":durable.0,"final_generation":st.local_generation(),"one_order":true,"journal_created":false,"integrity":"ok","scope":"Owned child SIGKILL before commit or after durable commit before adoption; not hardware power loss"})).unwrap());
    }
}
