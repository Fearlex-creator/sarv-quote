use anyhow::{Result,ensure};
use sarv_quote::{domain::State,sales::*};
use serde_json::{Value,json};
use std::{fs,path::PathBuf};
fn source(c:&SalesCommand)->String {c.context().source_id.clone()}
fn reject(name:&str,state:&State,command:&SalesCommand,edit:impl FnOnce(&mut Value))->Result<Value>{
    let mut value=serde_json::to_value(command)?;edit(&mut value);let changed:SalesCommand=serde_json::from_value(value)?;
    let mut candidate=state.accounting.clone();let error=candidate.post_sales_command(changed).expect_err(name);
    ensure!(candidate==state.accounting,"rejection must leave full accounting exactly unchanged: {name}");
    Ok(json!({"check":name,"result":"rejected_unchanged","error":error.to_string()}))
}
fn body(v:&mut Value)->&mut Value {v.as_object_mut().unwrap().values_mut().next().unwrap()}
fn main()->Result<()> {
    let input=PathBuf::from(std::env::args().nth(1).expect("actual fixture State"));let out=PathBuf::from(std::env::args().nth(2).expect("output directory"));fs::create_dir_all(&out)?;
    let state:State=serde_json::from_slice(&fs::read(input)?)?;state.validate()?;
    let order=&state.accounting.sales.records.iter().find(|r|source(&r.command)=="sales1499").unwrap().command;
    let fulfill=&state.accounting.sales.records.iter().find(|r|source(&r.command)=="fulfill633").unwrap().command;
    let returned=&state.accounting.sales.records.iter().find(|r|source(&r.command)=="return235").unwrap().command;
    let mut observations=vec![];
    observations.push(reject("same key changed actor",&state,order,|v|body(v)["context"]["actor"]="different actor".into())?);
    observations.push(reject("new key changed revision across all-revision identity",&state,order,|v|{let b=body(v);b["context"]["request_key"]="changed-revision-key".into();b["context"]["source_revision"]=2.into();})?);
    observations.push(reject("new key changed source-price provenance",&state,order,|v|{let b=body(v);b["context"]["request_key"]="changed-price-provenance".into();b["lines"][0]["price_reference"]="changed reference".into();})?);
    observations.push(reject("new key changed due date of fulfillment",&state,fulfill,|v|{let b=body(v);b["context"]["request_key"]="changed-due-key".into();b["due_date"]="2026-05-02".into();})?);
    observations.push(reject("new key changed original return cost",&state,returned,|v|{let b=body(v);b["context"]["request_key"]="changed-cost-key".into();b["lines"][0]["supplied_cost"]=13.into();b["supplied_cost"]=13.into();})?);
    for (kind,key) in [("procurement financial key", "independent-M5-receipt75"),("procurement nonfinancial key", "independent-M5-purchase180")] {
        observations.push(reject(kind,&state,order,|v|body(v)["context"]["request_key"]=key.into())?);
    }
    let mut fresh=serde_json::to_value(order)?;
    {let b=body(&mut fresh);b["context"]["source_id"]="new-valid-order".into();b["context"]["request_key"]="new-valid-order-key".into();b["context"]["date"]="2026-04-04".into();b["document_date"]="2026-04-04".into();b["reference"]="NEW-VALID-ORDER".into();b["lines"][0]["id"]="new-sale-line-a".into();b["lines"][1]["id"]="new-sale-line-b".into();}
    let fresh:SalesCommand=serde_json::from_value(fresh)?;
    observations.push(reject("opening financial request key collision",&state,&fresh,|v|body(v)["context"]["request_key"]=state.accounting.requests[0].command.request_key.clone().into())?);
    observations.push(reject("cross-company new order context",&state,&fresh,|v|body(v)["context"]["company_id"]="unrelated-company".into())?);
    observations.push(reject("new order financial date outside fiscal period",&state,&fresh,|v|{let b=body(v);b["context"]["date"]="2028-04-04".into();b["document_date"]="2028-04-04".into();})?);
    observations.push(reject("negative source quantity",&state,&fresh,|v|body(v)["lines"][0]["quantity_thousandths"]=(-1000).into())?);
    observations.push(reject("line discount cannot offset another line",&state,&fresh,|v|body(v)["lines"][0]["line_discount"]=1256.into())?);
    observations.push(reject("source price overflow limit",&state,&fresh,|v|body(v)["lines"][0]["unit_price"]=9_000_000_000_000_001_i64.into())?);
    observations.push(reject("unknown pack unit",&state,&fresh,|v|body(v)["lines"][0]["unit_id"]="unknown-pack".into())?);
    observations.push(reject("supplier-only/unavailable customer",&state,&fresh,|v|body(v)["customer_id"]="not-a-party".into())?);
    observations.push(reject("sales nested ID collides with procurement source line",&state,&fresh,|v|body(v)["lines"][0]["id"]="receipt75-line".into())?);
    observations.push(reject("independent total mismatch one rial",&state,&fresh,|v|body(v)["supplied_total"]=1500.into())?);
    let mut over=serde_json::to_value(fulfill)?;{let b=body(&mut over);b["context"]["source_id"]="independent-overfulfilled".into();b["context"]["request_key"]="independent-overfulfilled-key".into();b["context"]["date"]="2026-04-04".into();b["document_date"]="2026-04-04".into();b["delivery_id"]="over-delivery".into();b["invoice_id"]="over-invoice".into();b["delivery_reference"]="OVER-DEL".into();b["invoice_reference"]="OVER-INV".into();b["lines"].as_array_mut().unwrap().truncate(1);b["lines"][0]["id"]="over-fulfill-line".into();b["lines"][0]["quantity_thousandths"]=1000.into();}
    let mut backdated=over.clone();{let b=body(&mut backdated);b["context"]["date"]="2026-03-30".into();b["document_date"]="2026-03-30".into();}let backdated:SalesCommand=serde_json::from_value(backdated)?;let observation=reject("new stock issue before later company stock event",&state,&backdated,|_|{})?;ensure!(observation["error"].as_str().unwrap().contains("آخرین تاریخ موجودی"),"backdate rejection must be the shared stock-date guard");observations.push(observation);
    let over:SalesCommand=serde_json::from_value(over)?;observations.push(reject("full original order stays fulfilled after return",&state,&over,|_|{})?);
    let mut over=serde_json::to_value(returned)?;{let b=body(&mut over);b["context"]["source_id"]="independent-overreturned".into();b["context"]["request_key"]="independent-overreturned-key".into();b["context"]["date"]="2026-04-04".into();b["document_date"]="2026-04-04".into();b["return_id"]="over-return".into();b["credit_id"]="over-credit".into();b["return_reference"]="OVER-RET".into();b["credit_reference"]="OVER-CREDIT".into();b["lines"][0]["id"]="over-return-line".into();}
    let over:SalesCommand=serde_json::from_value(over)?;observations.push(reject("full returned invoice stays nonreturnable",&state,&over,|_|{})?);
    let period=state.accounting.periods[0].id.clone();let mut locked=state.clone();locked.accounting.set_period_lock(&period,true,"reviewer","Independent lock")?;
    observations.push(reject("new order during lock",&locked,&fresh,|_|{})?);
    for command in [order,fulfill,returned] {
        let before=locked.accounting.clone();let expected=locked.accounting.sales.records.iter().find(|r|r.command==*command).unwrap().id.clone();let result=locked.accounting.post_sales_command(command.clone())?;
        ensure!(result.record_id==expected&&locked.accounting==before,"exact successful locked retry leaves all state unchanged");
        let mut alias=serde_json::to_value(command)?;body(&mut alias)["context"]["request_key"]=format!("new-locked-alias-{}",command.context().source_id).into();
        let journals=locked.accounting.journals.clone();let audit=locked.accounting.audit.clone();let records=locked.accounting.sales.records.clone();let result=locked.accounting.post_sales_command(serde_json::from_value(alias)?)?;
        ensure!(result.record_id==expected&&locked.accounting.journals==journals&&locked.accounting.audit==audit&&locked.accounting.sales.records==records,"fresh-key alias after lock produces no second effect");
    }
    locked.validate()?;observations.push(json!({"check":"order/fulfillment/return exact and different-key retries after lock","result":"same_original_result_no_second_effect"}));
    let journal=state.accounting.sales.records.iter().find(|r|source(&r.command)=="fulfill633").unwrap().journal_id.clone().unwrap();let mut candidate=state.accounting.clone();
    ensure!(candidate.reverse(&journal,&period,"2026-04-04","bad-generic-reversal","reviewer","Must use supported original return").is_err()&&candidate==state.accounting,"typed generic reversal fails unchanged");
    observations.push(json!({"check":"typed generic reversal","result":"rejected_unchanged"}));
    let mut branch=state.clone();let mut branch_order=serde_json::to_value(&fresh)?;
    {let b=body(&mut branch_order);b["context"]["source_id"]="same-day-order".into();b["context"]["request_key"]="same-day-order-key".into();b["reference"]="SAME-DAY-SO".into();b["lines"].as_array_mut().unwrap().truncate(1);b["lines"][0]["id"]="same-day-order-line".into();b["lines"][0]["quantity_thousandths"]=4000.into();b["lines"][0]["unit_price"]=10.into();b["lines"][0]["line_discount"]=0.into();b["document_discount"]=0.into();b["supplied_total"]=40.into();}
    let branch_order_id=branch.accounting.post_sales_command(serde_json::from_value(branch_order)?)?.record_id;
    let mut branch_fulfill=serde_json::to_value(fulfill)?;
    {let b=body(&mut branch_fulfill);b["context"]["source_id"]="same-day-fulfill".into();b["context"]["request_key"]="same-day-fulfill-key".into();b["context"]["date"]="2026-04-04".into();b["document_date"]="2026-04-04".into();b["order_id"]=branch_order_id.into();b["delivery_id"]="same-day-delivery".into();b["invoice_id"]="same-day-invoice".into();b["delivery_reference"]="SAME-DAY-DEL".into();b["invoice_reference"]="SAME-DAY-INV".into();b["lines"].as_array_mut().unwrap().truncate(1);b["lines"][0]["id"]="same-day-fulfill-line".into();b["lines"][0]["order_line_id"]="same-day-order-line".into();b["supplied_total"]=20.into();}
    let branch_fulfill=serde_json::from_value(branch_fulfill)?;
    ensure!(branch.accounting.preview_sales(&branch_fulfill)?.cost.rials()==26,"same-day sale original cost26");
    let branch_fulfill_id=branch.accounting.post_sales_command(branch_fulfill)?.record_id;
    let procurement=&state.accounting.procurement.records;
    let mut tx=serde_json::to_value(&procurement.iter().find(|r|r.command.context().source_id=="transfer53").unwrap().command)?;
    {let b=body(&mut tx);b["context"]["source_id"]="same-day-transfer".into();b["context"]["request_key"]="same-day-transfer-key".into();b["context"]["date"]="2026-04-04".into();b["quantity_thousandths"]=1000.into();}
    let tx=serde_json::from_value(tx)?;ensure!(branch.accounting.preview_procurement(&tx)?.total.rials()==13,"same-day transfer after sale literalcost13");branch.accounting.post_procurement_command(tx)?;
    let mut buy=serde_json::to_value(&procurement.iter().find(|r|r.command.context().source_id=="purchase120").unwrap().command)?;
    {let b=body(&mut buy);b["context"]["source_id"]="same-day-purchase".into();b["context"]["request_key"]="same-day-purchase-key".into();b["context"]["date"]="2026-04-04".into();b["document_date"]="2026-04-04".into();b["reference"]="SAME-DAY-PO".into();b["lines"][0]["id"]="same-day-purchase-line".into();b["lines"][0]["quantity_thousandths"]=1000.into();b["lines"][0]["unit_price"]=120.into();b["supplied_total"]=120.into();}
    let purchase=branch.accounting.post_procurement_command(serde_json::from_value(buy)?)?.record_id;
    let mut receive=serde_json::to_value(&procurement.iter().find(|r|r.command.context().source_id=="receipt120").unwrap().command)?;
    {let b=body(&mut receive);b["context"]["source_id"]="same-day-receipt".into();b["context"]["request_key"]="same-day-receipt-key".into();b["context"]["date"]="2026-04-04".into();b["document_date"]="2026-04-04".into();b["order_id"]=purchase.into();b["reference"]="SAME-DAY-GR".into();b["lines"][0]["id"]="same-day-receipt-line".into();b["lines"][0]["order_line_id"]="same-day-purchase-line".into();b["lines"][0]["quantity_thousandths"]=1000.into();}
    branch.accounting.post_procurement_command(serde_json::from_value(receive)?)?;
    let mut ret=serde_json::to_value(returned)?;
    {let b=body(&mut ret);b["context"]["source_id"]="same-day-return".into();b["context"]["request_key"]="same-day-return-key".into();b["context"]["date"]="2026-04-04".into();b["document_date"]="2026-04-04".into();b["fulfillment_id"]=branch_fulfill_id.into();b["return_id"]="same-day-restock".into();b["credit_id"]="same-day-credit".into();b["return_reference"]="SAME-DAY-RET".into();b["credit_reference"]="SAME-DAY-CREDIT".into();b["lines"][0]["id"]="same-day-return-line".into();b["lines"][0]["fulfillment_line_id"]="same-day-fulfill-line".into();b["lines"][0]["quantity_thousandths"]=2000.into();b["lines"][0]["supplied_credit"]=20.into();b["lines"][0]["supplied_cost"]=26.into();b["supplied_credit"]=20.into();b["supplied_cost"]=26.into();}
    let ret=serde_json::from_value(ret)?;ensure!(branch.accounting.preview_sales(&ret)?.cost.rials()==26,"same-day return originalcost26, not current average34");branch.accounting.post_sales_command(ret)?;
    let report=branch.accounting.sales_reconcile(&state.accounting.companies[0].id,"2026-04-04")?;
    ensure!(report.reconciled&&report.stock.iter().find(|p|p.warehouse_id=="w1").map(|p|(p.quantity_thousandths,p.value.rials()))==Some((29000,485))&&report.stock.iter().find(|p|p.warehouse_id=="w2").map(|p|(p.quantity_thousandths,p.value.rials()))==Some((16000,185)),"same-day globally sequenced W1 29/485 W2 16/185");
    ensure!(report.total_debit.rials()==3190&&report.profit.rials()==816&&report.cogs.rials()==50,"same-day TB3190 and original profit816/COGS50");
    let prior=branch.accounting.sales_reconcile(&state.accounting.companies[0].id,"2026-04-03")?;ensure!(prior.total_debit.rials()==3050&&prior.stock.iter().map(|p|p.value.rials()).sum::<i64>()==550,"same-day append preserves prior cutoff");branch.validate()?;
    fs::write(out.join("same-day-state.json"),serde_json::to_vec_pretty(&branch)?)?;
    fs::write(out.join("same-day-report.json"),serde_json::to_vec_pretty(&report)?)?;
    observations.push(json!({"check":"same-day sale -> transfer -> receipt120 -> original-cost return26","result":"global_sequence_replay_exact","w1":[29000,485],"w2":[16000,185],"inventory":670,"grni":170,"ar":1066,"net_cogs":50,"profit":816,"trial_balance":3190,"prior_cutoff_exact":true}));
    for (name,edit) in [
        ("stock movement cost tamper",0u8),("duplicate sequence tamper",1),("component net tamper",2),("source allocation hash tamper",3),("sales request record ID tamper",4)] {
        let mut value=serde_json::to_value(&state)?;let sales=&mut value["accounting"]["sales"];
        match edit {
            0=>sales["records"][1]["movements"][0]["value"]=24.into(),
            1=>sales["records"][1]["sequence"]=sales["records"][0]["sequence"].clone(),
            2=>sales["records"][1]["data"]["Fulfill"]["lines"][0]["net"]=470.into(),
            3=>sales["records"][0]["command_sha256"]="0".repeat(64).into(),
            _=>sales["requests"][0]["record_id"]="not-a-record".into(),
        }
        let changed:State=serde_json::from_value(value)?;ensure!(changed.validate().is_err(),"tampered full history must reject: {name}");observations.push(json!({"check":name,"result":"restored_history_rejected"}));
    }
    let mut shuffled=state.clone();shuffled.accounting.sales.records.swap(1,2);ensure!(shuffled.validate().is_err(),"shuffled persisted record order must reject against actual audit sequence");observations.push(json!({"check":"shuffled persisted sales record order","result":"restored_history_rejected"}));
    fs::write(out.join("guards.json"),serde_json::to_vec_pretty(&observations)?)?;
    println!("PASS {} independent identity/retry/input/lock/reversal/tamper observations. No acceptance of untested native/fault cases.",observations.len());
    Ok(())
}
