//! Canonical synthetic fixture preparation, never run inside measurement.
use anyhow::{Result, ensure};
use sarv_quote::{
    core::CoreRole,
    domain::State,
    procurement::{ProcurementCommand, ProcurementContext, ProcurementData},
    store::Store,
};
use serde_json::{Value, json};
use std::{fs, path::Path};

const DATE_RECEIVE: &str = "2026-04-01";
const DATE_BILL: &str = "2026-04-02";
const DATE_RETURN: &str = "2026-04-03";

fn value(state: &State) -> Result<Value> {
    Ok(serde_json::to_value(state)?)
}

fn context(base: &State, source: &str, date: &str) -> Result<ProcurementContext> {
    let opening = &base.accounting.core.openings[0].command;
    Ok(ProcurementContext {
        company_id: opening.company_id.clone(),
        period_id: opening.period_id.clone(),
        setup_id: opening.setup_id.clone(),
        source_id: source.into(),
        source_revision: 1,
        date: date.into(),
        request_key: format!("M6-PERF-REQUEST-{source}"),
        actor: "M6 synthetic performance fixture generator".into(),
        reason: "Deterministic canonical original-cost fixture; no live data".into(),
    })
}

fn command(v: Value) -> Result<ProcurementCommand> {
    Ok(serde_json::from_value(v)?)
}

fn preserve(base: &State, state: &State) -> Result<()> {
    let old = value(base)?;
    let current = value(state)?;
    for key in old
        .as_object()
        .unwrap()
        .keys()
        .filter(|k| *k != "accounting")
    {
        ensure!(
            old[key] == current[key],
            "nonaccounting State changed: {key}"
        );
    }
    for key in old["accounting"]
        .as_object()
        .unwrap()
        .keys()
        .filter(|k| !["procurement", "journals", "requests", "audit"].contains(&k.as_str()))
    {
        ensure!(
            old["accounting"][key] == current["accounting"][key],
            "old accounting field changed: {key}"
        );
    }
    for key in ["journals", "requests", "audit"] {
        let prefix = old["accounting"][key].as_array().unwrap();
        ensure!(
            current["accounting"][key]
                .as_array()
                .unwrap()
                .starts_with(prefix),
            "old {key} prefix changed"
        );
    }
    for key in ["records", "requests"] {
        let prefix = old["accounting"]["procurement"][key].as_array().unwrap();
        ensure!(
            current["accounting"]["procurement"][key]
                .as_array()
                .unwrap()
                .starts_with(prefix),
            "old procurement {key} prefix changed"
        );
    }
    Ok(())
}

fn literal(state: &State, n: i64, billed: bool, main_returned: bool) -> Result<Value> {
    let opening = &state.accounting.core.openings[0].command;
    let report = state
        .accounting
        .procurement_reconcile(&opening.company_id, DATE_RETURN)?;
    ensure!(
        report.reconciled && report.controls.len() == 12,
        "12 controls required"
    );
    ensure!(
        report.controls.iter().all(|c| c.difference.rials() == 0),
        "individual control difference"
    );
    let delta = 15 * n;
    let (inv, ap, grni, tb, q, v) = if main_returned {
        (317, 224, 50, 1525, 18000, 187)
    } else {
        (
            422 + delta,
            329 + if billed { delta } else { 0 },
            50 + if billed { 0 } else { delta },
            1630 + delta,
            25000 + 1000 * n,
            292 + delta,
        )
    };
    for (role, wanted) in [
        (CoreRole::Inventory, inv),
        (CoreRole::Payables, ap),
        (CoreRole::Grni, grni),
    ] {
        let control = report.controls.iter().find(|c| c.role == role).unwrap();
        ensure!(
            control.detail.rials() == wanted && control.ledger == control.detail,
            "literal {role:?}"
        );
    }
    let w1 = report
        .stock
        .iter()
        .find(|p| p.item_id == "ITEM-A" && p.warehouse_id == "W1")
        .unwrap();
    let w2 = report
        .stock
        .iter()
        .find(|p| p.item_id == "ITEM-A" && p.warehouse_id == "W2")
        .unwrap();
    ensure!(
        (w1.quantity_thousandths, w1.value.rials()) == (q, v),
        "literal W1"
    );
    ensure!(
        (w2.quantity_thousandths, w2.value.rials()) == (12000, 130),
        "literal W2"
    );
    ensure!(
        report.total_debit.rials() == tb && report.total_credit.rials() == tb,
        "literal TB"
    );
    let sales = state
        .accounting
        .sales_reconcile(&opening.company_id, DATE_RETURN)?;
    ensure!(
        sales.reconciled
            && sales.sales.rials() == 199
            && sales.sales_returns.rials() == 100
            && sales.cogs.rials() == 58
            && sales.profit.rials() == 41,
        "old Sales controls changed"
    );
    ensure!(
        sales
            .receivables
            .iter()
            .map(|r| r.remaining.rials())
            .sum::<i64>()
            == 299,
        "old AR changed"
    );
    Ok(
        json!({"inventory":inv,"ap":ap,"grni":grni,"tb_debit":tb,"tb_credit":tb,"w1_quantity_thousandths":q,"w1_value":v,"w2_quantity_thousandths":12000,"w2_value":130,"ar":299,"sales":199,"sales_returns":100,"cogs":58,"earnings":41,"reconciliation":report}),
    )
}

fn save(base: &State, state: &State, target: &Path, recipe: Value, expected: Value) -> Result<()> {
    preserve(base, state)?;
    state.validate()?;
    fs::create_dir(target)?;
    let mut store = Store::open(&target.join("profile"))?;
    store.restore(state.clone())?;
    ensure!(
        value(&store.state)? == value(state)?,
        "persisted complete State exact"
    );
    store.backup(&target.join("backup.json"))?;
    fs::write(
        target.join("state.json"),
        serde_json::to_vec_pretty(&store.state)?,
    )?;
    fs::write(
        target.join("recipe.json"),
        serde_json::to_vec_pretty(
            &json!({"generator":"sarv-m6-performance-dataset-v1","seed":"M5-C12-authentic-quote-main199","synthetic_only":true,"recipe":recipe,"expected":expected,"old_prefix_preserved":true,"quotation_count":state.quotes.len()}),
        )?,
    )?;
    println!("READY {}", target.display());
    Ok(())
}

fn original_source(base: &State) -> Result<(String, String, String)> {
    let receipt = base
        .accounting
        .procurement
        .records
        .iter()
        .find(|r| matches!(&r.data, ProcurementData::Receive(d) if d.total.rials()==105))
        .unwrap();
    let ProcurementData::Receive(data) = &receipt.data else {
        unreachable!()
    };
    let line = data
        .lines
        .iter()
        .find(|l| l.quantity_thousandths == 7000 && l.value.rials() == 105)
        .unwrap();
    let bill = base.accounting.procurement.records.iter().find(|r| matches!(&r.data, ProcurementData::Bill(d) if d.total.rials()==150 && d.lines.iter().any(|l|l.receipt_line_id==line.id))).unwrap();
    let ProcurementData::Bill(data) = &bill.data else {
        unreachable!()
    };
    let billed_line = data
        .lines
        .iter()
        .find(|l| l.receipt_line_id == line.id)
        .unwrap();
    ensure!(
        billed_line.quantity_thousandths == 7000 && billed_line.value.rials() == 105,
        "actual original R-B7/105"
    );
    Ok((
        line.id.clone(),
        billed_line.id.clone(),
        data.supplier_id.clone(),
    ))
}

fn make_source(base: &State, name: &str, n: usize, billed: bool) -> Result<(State, Value)> {
    let mut state = base.clone();
    let po = format!("M6-PERF-{name}-PO");
    let gr = format!("M6-PERF-{name}-GR");
    let bill = format!("M6-PERF-{name}-BILL");
    let lines: Vec<_> = (1..=n).map(|i| json!({"id":format!("{po}-L{i:03}"),"item_id":"ITEM-A","warehouse_id":"W1","unit_id":"PIECE","quantity_thousandths":1000,"unit_price":15,"line_discount":0,"price_reference":"M6-PERF canonical supplier15IRR","price_date":DATE_RECEIVE,"override_reason":null})).collect();
    let posted_po = state.accounting.post_procurement_command(command(json!({"Order":{"context":context(base,&po,DATE_RECEIVE)?,"supplier_id":"SU-A","reference":po,"document_date":DATE_RECEIVE,"lines":lines,"document_discount":0,"supplied_total":15*n}}))?)?.record_id;
    let receipts: Vec<_> = (1..=n).map(|i| json!({"id":format!("{gr}-L{i:03}"),"order_line_id":format!("{po}-L{i:03}"),"warehouse_id":"W1","unit_id":"PIECE","quantity_thousandths":1000})).collect();
    let posted_gr = state.accounting.post_procurement_command(command(json!({"Receive":{"context":context(base,&gr,DATE_RECEIVE)?,"order_id":posted_po,"reference":gr,"document_date":DATE_RECEIVE,"lines":receipts,"supplied_total":15*n}}))?)?.record_id;
    let mut posted_bill = None;
    if billed {
        let bills: Vec<_> = (1..=n).map(|i| json!({"id":format!("{bill}-L{i:03}"),"receipt_line_id":format!("{gr}-L{i:03}"),"quantity_thousandths":1000,"supplied_value":15})).collect();
        posted_bill = Some(state.accounting.post_procurement_command(command(json!({"Bill":{"context":context(base,&bill,DATE_BILL)?,"supplier_id":"SU-A","reference":bill,"document_date":DATE_BILL,"due_date":"2026-05-01","lines":bills,"supplied_total":15*n}}))?)?.record_id);
    }
    let recipe = json!({"kind":if billed{"billed"}else{"unbilled"},"line_count":n,"quantity_per_line_thousandths":1000,"original_value_per_line_irr":15,"original_total_irr":15*n,"order_record_id":posted_po,"receipt_record_id":posted_gr,"bill_record_id":posted_bill,"source_reference":if billed{bill.clone()}else{gr.clone()},"receipt_line_ids":(1..=n).map(|i|format!("{gr}-L{i:03}")).collect::<Vec<_>>(),"bill_line_ids":if billed{(1..=n).map(|i|format!("{bill}-L{i:03}")).collect::<Vec<_>>()}else{vec![]},"return_date":DATE_RETURN,"full_return_expected":{"inventory":422,"ap":329,"grni":50,"tb_debit":1630,"tb_credit":1630,"w1_quantity_thousandths":25000,"w1_value":292}});
    Ok((state, recipe))
}

fn picker_sources(base: &State, billed: bool) -> Result<(State, Value)> {
    let name = if billed {
        "SOURCES-BILLED"
    } else {
        "SOURCES-UNBILLED"
    };
    let mut state = base.clone();
    let po = format!("M6-PERF-{name}-PO");
    let labels: Vec<_> = ['A', 'B']
        .into_iter()
        .flat_map(|group| (1..=60).map(move |i| format!("PERF-{group}-{i:03}")))
        .collect();
    let lines: Vec<_> = labels.iter().map(|label|json!({"id":format!("{po}-{label}"),"item_id":"ITEM-A","warehouse_id":"W1","unit_id":"PIECE","quantity_thousandths":1000,"unit_price":15,"line_discount":0,"price_reference":"M6-PERF canonical supplier15IRR","price_date":DATE_RECEIVE,"override_reason":null})).collect();
    let order = state.accounting.post_procurement_command(command(json!({"Order":{"context":context(base,&po,DATE_RECEIVE)?,"supplier_id":"SU-A","reference":po,"document_date":DATE_RECEIVE,"lines":lines,"document_discount":0,"supplied_total":1800}}))?)?.record_id;
    let mut sources = Vec::new();
    for label in &labels {
        let gr = format!("M6-PERF-{name}-{label}-GR");
        let receipt_line = format!("{gr}-L001");
        let receipt = state.accounting.post_procurement_command(command(json!({"Receive":{"context":context(base,&gr,DATE_RECEIVE)?,"order_id":order,"reference":gr,"document_date":DATE_RECEIVE,"lines":[{"id":receipt_line,"order_line_id":format!("{po}-{label}"),"warehouse_id":"W1","unit_id":"PIECE","quantity_thousandths":1000}],"supplied_total":15}}))?)?.record_id;
        let source = if billed {
            let bill = format!("M6-PERF-{name}-{label}-BILL");
            let bill_line = format!("{bill}-L001");
            let record = state.accounting.post_procurement_command(command(json!({"Bill":{"context":context(base,&bill,DATE_BILL)?,"supplier_id":"SU-A","reference":bill,"document_date":DATE_BILL,"due_date":"2026-05-01","lines":[{"id":bill_line,"receipt_line_id":receipt_line,"quantity_thousandths":1000,"supplied_value":15}],"supplied_total":15}}))?)?.record_id;
            json!({"query_group":format!("PERF-{}",&label[5..6]),"reference":bill,"record_id":record,"receipt_line_id":receipt_line,"bill_line_id":bill_line})
        } else {
            json!({"query_group":format!("PERF-{}",&label[5..6]),"reference":gr,"record_id":receipt,"receipt_line_id":receipt_line,"bill_line_id":null})
        };
        sources.push(source);
    }
    Ok((
        state,
        json!({"kind":if billed{"billed"}else{"unbilled"},"source_count":120,"group_count":60,"page_size":25,"filtered_pages":3,"unfiltered_synthetic_pages":5,"quantity_per_source_thousandths":1000,"value_per_source_irr":15,"sources_in_posting_order":sources,"expected_filter_first_references":(['A','B'].into_iter().map(|g|json!({"query":format!("PERF-{g}"),"pages": ([60,35,10].into_iter().map(|i|format!("M6-PERF-{name}-PERF-{g}-{i:03}-{}",if billed{"BILL"}else{"GR"})).collect::<Vec<_>>()) })).collect::<Vec<_>>())}),
    ))
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    ensure!(
        args.len() == 4,
        "Usage: helper SEED_STATE_JSON SEED_V6_BACKUP FRESH_OUTPUT"
    );
    let seed_path = Path::new(&args[1]);
    let backup_path = Path::new(&args[2]);
    let out = Path::new(&args[3]);
    ensure!(!out.exists(), "output must be fresh");
    let original: State = serde_json::from_slice(&fs::read(seed_path)?)?;
    original.validate()?;
    ensure!(
        original.schema == 6 && !original.quotes.is_empty(),
        "authentic quote-bearing M5 schema6 input required"
    );
    let backup: Value = serde_json::from_slice(&fs::read(backup_path)?)?;
    ensure!(
        backup["state"] == value(&original)?,
        "genuine backup and State mismatch"
    );
    let base = Store::restore_preview(backup_path)?;
    let mut expected = original.clone();
    expected.schema = 7;
    ensure!(
        value(&base)? == value(&expected)?,
        "upgrade must preserve entire original State except schema"
    );
    literal(&base, 0, true, false)?;
    fs::create_dir(out)?;
    fs::write(
        out.join("original-schema6-state.json"),
        serde_json::to_vec_pretty(&original)?,
    )?;
    fs::write(
        out.join("initialized-schema7-state.json"),
        serde_json::to_vec_pretty(&base)?,
    )?;
    save(
        &base,
        &base,
        &out.join("main-seed"),
        json!({"kind":"authentic-M5-March31-main199","quote_bearing":true}),
        literal(&base, 0, true, false)?,
    )?;
    let (receipt_line, bill_line, supplier) = original_source(&base)?;
    let mut posted = base.clone();
    for (i, q, v, date) in [(1, 3000, 45, DATE_RECEIVE), (2, 4000, 60, DATE_BILL)] {
        let id = format!("M6-PERF-MAIN-{i}");
        posted.accounting.post_procurement_command(command(json!({"Return":{"context":context(&base,&id,date)?,"supplier_id":supplier,"return_id":format!("{id}-RETURN"),"return_reference":format!("{id}-RETURN-REF"),"credit_id":format!("{id}-CREDIT"),"credit_reference":format!("{id}-CREDIT-REF"),"document_date":date,"lines":[{"id":format!("{id}-L001"),"receipt_line_id":receipt_line,"bill_line_id":bill_line,"quantity_thousandths":q,"supplied_value":v}],"supplied_total":v}}))?)?;
    }
    save(
        &base,
        &posted,
        &out.join("main-posted"),
        json!({"kind":"M6-billed-main45-plus60","receipt_line_id":receipt_line,"bill_line_id":bill_line}),
        literal(&posted, 0, true, true)?,
    )?;
    for (name, n, billed) in [
        ("return20-billed", 20, true),
        ("return20-unbilled", 20, false),
        ("return200-billed", 200, true),
    ] {
        let (state, recipe) = make_source(&base, name, n, billed)?;
        save(
            &base,
            &state,
            &out.join(name),
            recipe,
            literal(&state, n as i64, billed, false)?,
        )?;
    }
    for billed in [true, false] {
        let (state, recipe) = picker_sources(&base, billed)?;
        let name = if billed {
            "source120-billed"
        } else {
            "source120-unbilled"
        };
        save(
            &base,
            &state,
            &out.join(name),
            recipe,
            literal(&state, 120, billed, false)?,
        )?;
    }
    fs::write(
        out.join("preparation.json"),
        serde_json::to_vec_pretty(
            &json!({"result":"prepared-canonical-synthetic-only","generator":"sarv-m6-performance-dataset-v1","seed_state":seed_path,"seed_backup":backup_path,"final_measurement_run":false,"startup_cohorts":["main-seed","main-posted","source120-billed","source120-unbilled","return20-unbilled","return200-billed"],"workflow_only_without_startup_cohort":"return20-billed","root_compile_before_quiet_measurement":true}),
        )?,
    )?;
    Ok(())
}
