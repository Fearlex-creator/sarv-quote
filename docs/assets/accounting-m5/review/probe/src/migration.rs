use anyhow::{Result, ensure};
use sarv_quote::{core::CoreRole, sales::*, store::Store};
use serde_json::{Value,json};
use std::{fs,path::{Path,PathBuf}};

fn copy_dir(from:&Path,to:&Path)->Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {let entry=entry?;let target=to.join(entry.file_name());if entry.file_type()?.is_dir(){copy_dir(&entry.path(),&target)?;}else{fs::copy(entry.path(),target)?;}}
    Ok(())
}
fn payload(profile:&Path)->Result<(i64,String)> {
    let connection=rusqlite::Connection::open(profile.join("sarv.sqlite3"))?;
    Ok(connection.query_row("SELECT generation,payload FROM state WHERE id=1",[],|r|Ok((r.get(0)?,r.get(1)?)))?)
}
fn main()->Result<()> {
    let out=PathBuf::from(std::env::args().nth(1).expect("fresh private output"));fs::create_dir_all(&out)?;
    let base=PathBuf::from(std::env::var("SARV_REVIEW_M4_INPUT_ROOT").expect("actual archived-M4 input root required"));
    copy_dir(&base.join("actual-m4-profile"),&out.join("actual-m4-profile"))?;
    let (generation,before)=payload(&out.join("actual-m4-profile"))?;let original:Value=serde_json::from_str(&before)?;
    ensure!(original["schema"]==5,"actual M4 database fixture schema5");
    let mut migrated=Store::open(&out.join("actual-m4-profile"))?;
    ensure!(migrated.state.schema==6&&migrated.local_generation()==generation+1,"actual schema5->6 one conditional generation");
    ensure!(serde_json::to_value(&migrated.state.accounting)?==original["accounting"],"legacy manual sales prefix full history exact after upgrade");
    let recovery=fs::read_dir(out.join("actual-m4-profile"))?.filter_map(|e|e.ok()).find(|e|e.file_name().to_string_lossy().starts_with("before-schema6-")).expect("original recovery before upgrade").path();
    let envelope:Value=serde_json::from_slice(&fs::read(&recovery)?)?;
    ensure!(envelope["state"]==original&&envelope["format"]=="sarv-accounting-backup-v5","original schema5 recovery exact/no normalization");
    let old_before=payload(&out.join("actual-m4-profile"))?;
    let old_binary=PathBuf::from(std::env::var("SARV_REVIEW_M4_OLD_BINARY").expect("actual archived-M4 writer binary required"));
    let old_result=std::process::Command::new(&old_binary).arg(&out).output()?;
    fs::write(out.join("actual-m4-older-writer.log"),&old_result.stderr)?;
    ensure!(!old_result.status.success()&&payload(&out.join("actual-m4-profile"))?==old_before,"actual archived-M4 writer rejects upgraded schema6 without payload/generation mutation");
    let first=migrated.state.accounting.journals[0].id.clone();let request=migrated.state.accounting.requests.iter().find(|r|r.journal_id==first).unwrap().command.clone();
    let before_accounting=migrated.state.accounting.clone();
    ensure!(migrated.change(|s|s.accounting.post(request.clone()))?==first&&migrated.state.accounting==before_accounting,"completed old manual sales retry after lock");
    let mut alias=request.clone();alias.request_key="legacy-M5-new-alias".into();
    ensure!(migrated.change(|s|s.accounting.post(alias))?==first,"new alias retained legacy manual sales identity");
    let mut forbidden=request;forbidden.source_id="new-reserved-manual".into();forbidden.request_key="new-reserved-manual".into();forbidden.event="sales.unsupported".into();
    let before=migrated.state.clone();ensure!(migrated.change(|s|s.accounting.post(forbidden)).is_err()&&serde_json::to_value(&before)?==serde_json::to_value(&migrated.state)?,"new generic sales prefix forbidden/unchanged");
    migrated.backup(&out.join("legacy-new-v6-backup.json"))?;let state=Store::restore_preview(&out.join("legacy-new-v6-backup.json"))?;
    let mut fresh=Store::open(&out.join("fresh-legacy-restore"))?;fresh.restore(state)?;
    ensure!(serde_json::to_value(&fresh.state)?==serde_json::to_value(&migrated.state)?,"legacy sales full State fresh restore exact");

    let revenue_path=base.with_file_name("actual-m4-legacy-revenue").join("actual-m4-core-ordinary-revenue-v5.json");
    let mut ordinary=Store::open(&out.join("ordinary-revenue-restored"))?;ordinary.restore(Store::restore_preview(&revenue_path)?)?;
    let company=ordinary.state.accounting.companies[0].id.clone();let period=ordinary.state.accounting.periods[0].id.clone();let setup=ordinary.state.accounting.core.setups[0].id.clone();
    let report=ordinary.state.accounting.sales_reconcile(&company,"2026-03-22")?;
    ensure!(report.reconciled,"retained M4 ordinary Sales17/Returns3 must reconcile with explicit provenance");
    let report_json=serde_json::to_value(&report)?;
    for(role,value)in[(CoreRole::Sales,17),(CoreRole::SalesReturns,3)] {
        let a=ordinary.state.accounting.core.setups[0].account_id(role)?;let tb=ordinary.state.accounting.trial_balance(&company,"2026-03-22")?;let row=tb.rows.iter().find(|x|x.account_id==a).unwrap();let net=if role==CoreRole::Sales{row.credit.rials()-row.debit.rials()}else{row.debit.rials()-row.credit.rials()};ensure!(net==value,"retained M4 ordinary mapped revenue literal");
    }
    let order:SalesCommand=serde_json::from_value(json!({"Order":{"context":{"company_id":company,"period_id":period,"setup_id":setup,"source_id":"ordinary-compatible-new-sale","source_revision":1,"date":"2026-03-23","request_key":"ordinary-compatible-key","actor":"reviewer","reason":"New supported goods order after retained ordinary adjustments"},"customer_id":"party-a","reference":"ORDINARY-COMPATIBLE-SO","document_date":"2026-03-23","lines":[{"id":"ordinary-compatible-line","item_id":"item-a","warehouse_id":"w1","unit_id":"piece","quantity_thousandths":1000,"unit_price":20,"line_discount":0,"price_reference":"new explicit source","price_date":"2026-03-23","override_reason":null}],"document_discount":0,"supplied_total":20,"quotation":null}}))?;
    ordinary.change(|s|s.accounting.post_sales_command(order))?;
    fs::write(out.join("ordinary-revenue-report.json"),serde_json::to_vec_pretty(&report_json)?)?;
    fs::write(out.join("legacy-migrated-state.json"),serde_json::to_vec_pretty(&migrated.state)?)?;
    println!("PASS actual M4 schema5 migration/recovery, reserved sales namespace compatibility/retries, exact fresh restore, retained ordinary Sales17/Returns3 reconciliation and new goods order.");
    Ok(())
}
