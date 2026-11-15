use sarv_quote::{
    accounting::AccountClass,
    desktop::{dispatch, seed_fixture},
    store::Store,
};
use serde_json::{Value, json};

fn call(dir: &std::path::Path, action: &str, p: Value) -> Value {
    dispatch(dir, action, p).unwrap()
}

fn fixture() -> (tempfile::TempDir, String, String) {
    let d = tempfile::tempdir().unwrap();
    seed_fixture(d.path()).unwrap();
    let s = Store::open(d.path()).unwrap();
    let c = s.state.accounting.companies[0].id.clone();
    let p = s.state.accounting.periods[0].id.clone();
    (d, c, p)
}

#[test]
fn add_expense_account_post_journal_and_see_in_pnl() {
    let (d, c, p) = fixture();
    let ws = call(d.path(), "workspace", json!({ "company_id": c }));
    let generation = ws["_generation"].as_str().unwrap().to_string();

    let expense_parent = ws["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["classification"] == "Expense")
        .unwrap();
    let parent_id = expense_parent["id"].as_str().unwrap().to_string();

    let asset_acc = ws["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["classification"] == "Asset" && a["postable"] == true && a["control"] == false)
        .unwrap();
    let cash_id = asset_acc["id"].as_str().unwrap().to_string();

    let added = call(
        d.path(),
        "account_add",
        json!({
            "company_id": c,
            "parent_id": parent_id,
            "code": "5199",
            "name": "Rent",
            "classification": "Expense",
            "normal_balance": "Debit",
            "postable": true,
            "control": false,
            "expected_generation": generation,
        }),
    );
    let rent_id = added["account_id"].as_str().unwrap().to_string();

    let ws2 = call(d.path(), "workspace", json!({ "company_id": c }));
    let rent_acc = ws2["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == rent_id)
        .expect("Rent account should appear in workspace.accounts");
    assert_eq!(rent_acc["name"], "Rent");
    assert_eq!(rent_acc["code"], "5199");
    assert_eq!(rent_acc["parent_id"], parent_id);

    let mut journal_cmd = json!({
        "company_id": c,
        "period_id": p,
        "source_id": "rent-j1",
        "request_key": "rent-j1",
        "date": "2026-10-08",
        "reason": "Monthly office rent",
        "lines": [
            { "account_id": rent_id, "debit": "250000", "credit": "0", "description": "Rent" },
            { "account_id": cash_id, "debit": "0", "credit": "250000", "description": "Cash" }
        ]
    });
    let preview = call(d.path(), "journal_preview", journal_cmd.clone());
    journal_cmd["expected_generation"] = preview["_generation"].clone();
    call(d.path(), "journal_post", journal_cmd);

    let rep = call(
        d.path(),
        "report",
        json!({ "company_id": c, "from": "2026-01-01", "to": "2026-12-31" }),
    );
    let pnl_rows = rep["profit_loss"]["rows"].as_array().unwrap();
    let rent_row = pnl_rows
        .iter()
        .find(|r| r["account_id"] == rent_id)
        .expect("Rent account should appear in P&L report rows");
    assert_eq!(rent_row["name"], "Rent");
    assert_eq!(rent_row["amount"], "250000");

    let reopened = Store::open(d.path()).unwrap();
    reopened.state.validate().unwrap();
}

#[test]
fn account_add_rejects_duplicate_code_mismatched_parent_and_control_true() {
    let (d, c, _) = fixture();
    let ws = call(d.path(), "workspace", json!({ "company_id": c }));
    let generation = ws["_generation"].as_str().unwrap().to_string();

    let existing_code = ws["accounts"][0]["code"].as_str().unwrap().to_string();
    let dup = dispatch(
        d.path(),
        "account_add",
        json!({
            "company_id": c,
            "code": existing_code,
            "name": "Duplicate",
            "classification": "Expense",
            "normal_balance": "Debit",
            "expected_generation": generation,
        }),
    );
    assert!(dup.is_err(), "duplicate account code must be rejected");

    let asset_parent = ws["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["classification"] == "Asset")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let mismatch = dispatch(
        d.path(),
        "account_add",
        json!({
            "company_id": c,
            "parent_id": asset_parent,
            "code": "5201",
            "name": "Mismatched Expense",
            "classification": "Expense",
            "normal_balance": "Debit",
            "expected_generation": generation,
        }),
    );
    assert!(mismatch.is_err(), "mismatched parent classification must be rejected");

    let ctrl = dispatch(
        d.path(),
        "account_add",
        json!({
            "company_id": c,
            "code": "5202",
            "name": "Control Attempt",
            "classification": "Expense",
            "normal_balance": "Debit",
            "control": true,
            "expected_generation": generation,
        }),
    );
    assert!(ctrl.is_err(), "control: true must be rejected");
}

#[test]
fn archive_unused_role_mapped_and_balanced_accounts() {
    let (d, c, p) = fixture();
    let ws = call(d.path(), "workspace", json!({ "company_id": c }));
    let gen1 = ws["_generation"].as_str().unwrap().to_string();

    // 1. Add an unused account and archive it.
    let unused_id = call(
        d.path(),
        "account_add",
        json!({
            "company_id": c,
            "code": "5901",
            "name": "Unused Expense",
            "classification": "Expense",
            "normal_balance": "Debit",
            "expected_generation": gen1,
        }),
    )["account_id"]
        .as_str()
        .unwrap()
        .to_string();

    let gen2 = call(d.path(), "workspace", json!({ "company_id": c }))["_generation"]
        .as_str()
        .unwrap()
        .to_string();
    let arch = call(
        d.path(),
        "account_archive",
        json!({
            "company_id": c,
            "account_id": unused_id,
            "reason": "No longer needed",
            "expected_generation": gen2,
        }),
    );
    assert_eq!(arch["archived"], true);

    let ws_after = call(d.path(), "workspace", json!({ "company_id": c }));
    let archived_acc = ws_after["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == unused_id)
        .unwrap();
    assert_eq!(archived_acc["archived"], true);

    // 2. Archiving a role-mapped account must fail.
    let store = Store::open(d.path()).unwrap();
    let role_account_id = store.state.accounting.core.setups[0].command.mappings[0].account_id.clone();
    drop(store);
    let gen3 = ws_after["_generation"].as_str().unwrap().to_string();
    let role_err = dispatch(
        d.path(),
        "account_archive",
        json!({
            "company_id": c,
            "account_id": role_account_id,
            "reason": "Attempt role archive",
            "expected_generation": gen3,
        }),
    );
    assert!(role_err.is_err(), "role-mapped account must not be archivable");

    // 3. Add another account, post a journal to give it a non-zero balance, then archive it.
    // Observed engine behavior: archive_account permits archiving an account with a balance
    // (preserving historical journals and keeping the trial balance balanced).
    let used_id = call(
        d.path(),
        "account_add",
        json!({
            "company_id": c,
            "code": "5902",
            "name": "Travel",
            "classification": "Expense",
            "normal_balance": "Debit",
            "expected_generation": gen3,
        }),
    )["account_id"]
        .as_str()
        .unwrap()
        .to_string();
    let cash_id = ws_after["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| {
            a["classification"] == serde_json::to_value(AccountClass::Asset).unwrap() && a["postable"] == true && a["control"] == false
        })
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let mut j = json!({
        "company_id": c,
        "period_id": p,
        "source_id": "travel-j1",
        "request_key": "travel-j1",
        "date": "2026-10-09",
        "reason": "Travel expense",
        "lines": [
            { "account_id": used_id, "debit": "50000", "credit": "0", "description": "Travel" },
            { "account_id": cash_id, "debit": "0", "credit": "50000", "description": "Cash" }
        ]
    });
    let prev = call(d.path(), "journal_preview", j.clone());
    j["expected_generation"] = prev["_generation"].clone();
    call(d.path(), "journal_post", j);

    let gen4 = call(d.path(), "workspace", json!({ "company_id": c }))["_generation"]
        .as_str()
        .unwrap()
        .to_string();
    let arch_used = call(
        d.path(),
        "account_archive",
        json!({
            "company_id": c,
            "account_id": used_id,
            "reason": "Discontinued category",
            "expected_generation": gen4,
        }),
    );
    assert_eq!(arch_used["archived"], true);

    let rep = call(
        d.path(),
        "report",
        json!({ "company_id": c, "from": "2026-01-01", "to": "2026-12-31" }),
    );
    assert_eq!(rep["trial"]["total_debit"], rep["trial"]["total_credit"]);

    let reopened = Store::open(d.path()).unwrap();
    reopened.state.validate().unwrap();
}
