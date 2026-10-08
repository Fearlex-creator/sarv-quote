mod opening;
use anyhow::{Result, ensure};
use sarv_quote::{accounting::*, core::*, procurement::*, sales::*, store::Store};
use serde_json::{Value, json};
use std::{fs, path::PathBuf};

fn context(o:&OpeningCommand, source:&str, date:&str)->Value {
    json!({"company_id":o.company_id,"period_id":o.period_id,"setup_id":o.setup_id,"source_id":source,"source_revision":1,"date":date,"request_key":format!("independent-M5-{source}"),"actor":"Independent M5 reviewer","reason":"Literal independent fixture, no guessed balancing amount"})
}
fn buy(o:&OpeningCommand, source:&str, date:&str, quantity:i64, price:i64, total:i64)->Result<ProcurementCommand> {
    Ok(serde_json::from_value(json!({"Order":{"context":context(o,source,date),"supplier_id":"party-a","reference":format!("PO-{source}"),"document_date":date,"lines":[{"id":format!("{source}-line"),"item_id":"item-a","warehouse_id":"w1","unit_id":"piece","quantity_thousandths":quantity,"unit_price":price,"line_discount":0,"price_reference":"Independent supplier price","price_date":date,"override_reason":null}],"document_discount":0,"supplied_total":total}}))?)
}
fn receive(o:&OpeningCommand, source:&str, date:&str, order_id:&str, order_line:&str, quantity:i64,total:i64)->Result<ProcurementCommand> {
    Ok(serde_json::from_value(json!({"Receive":{"context":context(o,source,date),"order_id":order_id,"reference":format!("GR-{source}"),"document_date":date,"lines":[{"id":format!("{source}-line"),"order_line_id":order_line,"warehouse_id":"w1","unit_id":"piece","quantity_thousandths":quantity}],"supplied_total":total}}))?)
}
fn bill(o:&OpeningCommand, source:&str, date:&str, lines:Vec<(&str,i64,i64)>, total:i64)->Result<ProcurementCommand> {
    Ok(serde_json::from_value(json!({"Bill":{"context":context(o,source,date),"supplier_id":"party-a","reference":format!("BILL-{source}"),"document_date":date,"due_date":"2026-05-01","lines":lines.into_iter().enumerate().map(|(i,(line,q,v))|json!({"id":format!("{source}-line-{i}"),"receipt_line_id":line,"quantity_thousandths":q,"supplied_value":v})).collect::<Vec<_>>(),"supplied_total":total}}))?)
}
fn transfer(o:&OpeningCommand,source:&str,date:&str,q:i64)->Result<ProcurementCommand> {
    Ok(serde_json::from_value(json!({"Transfer":{"context":context(o,source,date),"item_id":"item-a","source_warehouse_id":"w1","target_warehouse_id":"w2","unit_id":"piece","quantity_thousandths":q}}))?)
}
fn fulfill(o:&OpeningCommand,source:&str,date:&str,order_id:&str,qa:i64,qb:i64,total:i64)->Result<SalesCommand> {
    Ok(serde_json::from_value(json!({"Fulfill":{"context":context(o,source,date),"order_id":order_id,"delivery_id":format!("{source}-delivery"),"invoice_id":format!("{source}-invoice"),"delivery_reference":format!("DEL-{source}"),"invoice_reference":format!("INV-{source}"),"document_date":date,"due_date":"2026-05-01","lines":[{"id":format!("{source}-a"),"order_line_id":"sale-line-a","warehouse_id":"w1","unit_id":"piece","quantity_thousandths":qa},{"id":format!("{source}-b"),"order_line_id":"sale-line-b","warehouse_id":"w2","unit_id":"piece","quantity_thousandths":qb}],"supplied_total":total}}))?)
}
fn restock(o:&OpeningCommand,source:&str,date:&str,fulfillment_id:&str,lines:Vec<(&str,i64,i64,i64)>,total:i64,cost:i64)->Result<SalesCommand> {
    Ok(serde_json::from_value(json!({"Return":{"context":context(o,source,date),"fulfillment_id":fulfillment_id,"return_id":format!("{source}-return"),"credit_id":format!("{source}-credit"),"return_reference":format!("RET-{source}"),"credit_reference":format!("CREDIT-{source}"),"document_date":date,"lines":lines.into_iter().enumerate().map(|(i,(line,q,v,c))|json!({"id":format!("{source}-line-{i}"),"fulfillment_line_id":line,"quantity_thousandths":q,"supplied_credit":v,"supplied_cost":c})).collect::<Vec<_>>(),"supplied_credit":total,"supplied_cost":cost}}))?)
}
fn role_value(s:&AccountingState,o:&OpeningCommand,date:&str,role:CoreRole)->Result<i64> {
    let tb=s.trial_balance(&o.company_id,date)?;let a=s.core.setups.iter().find(|x|x.id==o.setup_id).unwrap().account_id(role)?;
    Ok(tb.rows.iter().find(|r|r.account_id==a).map(|r|if role.normal_balance()==BalanceSide::Debit{r.debit.rials()-r.credit.rials()}else{r.credit.rials()-r.debit.rials()}).unwrap_or(0))
}
fn check(s:&AccountingState,o:&OpeningCommand,row:&Value)->Result<Value> {
    let date=row["date"].as_str().unwrap();let report=s.procurement_reconcile(&o.company_id,date)?;
    ensure!(report.reconciled&&report.controls.len()==12,"12 independently reconciled controls at {date}");
    for (role,key) in [(CoreRole::Inventory,"inventory_irr"),(CoreRole::Receivables,"ar_irr"),(CoreRole::Payables,"ap_irr"),(CoreRole::Grni,"grni_irr"),(CoreRole::Cogs,"net_cogs_irr")] {
        let expected=row[key].as_i64().unwrap();let c=report.controls.iter().find(|x|x.role==role).unwrap();
        ensure!(c.ledger.rials()==expected&&c.detail.rials()==expected&&c.difference==Money::ZERO,"control {role:?} at {date}: expected {expected}, actual {c:?}");
    }
    for warehouse in ["w1","w2"] {
        let p=report.stock.iter().find(|x|x.item_id=="item-a"&&x.warehouse_id==warehouse).unwrap();
        ensure!(p.quantity_thousandths==row[warehouse][0].as_i64().unwrap()&&p.value.rials()==row[warehouse][1].as_i64().unwrap(),"literal stock {warehouse} at {date}");
    }
    for (role,key) in [(CoreRole::Sales,"sales_irr"),(CoreRole::SalesReturns,"sales_returns_irr")] {ensure!(role_value(s,o,date,role)?==row[key].as_i64().unwrap(),"literal {role:?} at {date}");}
    ensure!(report.total_debit.rials()==row["tb_irr"].as_i64().unwrap()&&report.total_credit==report.total_debit,"literal trial balance at {date}");
    let sales=s.sales_reconcile(&o.company_id,date)?;
    ensure!(sales.reconciled&&sales.controls.len()==14&&sales.controls.iter().all(|x|x.difference==Money::ZERO),"all14 sales-aware controls at {date}");
    ensure!(sales.sales.rials()==row["sales_irr"].as_i64().unwrap()&&sales.sales_returns.rials()==row["sales_returns_irr"].as_i64().unwrap()&&sales.cogs.rials()==row["net_cogs_irr"].as_i64().unwrap()&&sales.profit.rials()==row["pnl_irr"].as_i64().unwrap(),"independent sales report values at {date}");
    ensure!(sales.receivables.iter().map(|x|x.remaining.rials()).sum::<i64>()==row["ar_irr"].as_i64().unwrap(),"complete original AR plus invoice/credit remainder at {date}");
    s.validate()?;
    Ok(json!({"date":date,"procurement":report,"sales":sales}))
}
fn main()->Result<()> {
    let out=PathBuf::from(std::env::args().nth(1).expect("private fresh output dir"));fs::create_dir_all(&out)?;
    let expected:Value=serde_json::from_str(include_str!("../../literal-expected.json"))?;
    let(mut s,o)=opening::fixture()?;s.post_opening(o.clone())?;
    let p=s.post_procurement_command(buy(&o,"purchase180","2026-03-22",12000,15,180)?)?.record_id;
    s.post_procurement_command(receive(&o,"receipt75","2026-03-23",&p,"purchase180-line",5000,75)?)?;
    s.post_procurement_command(receive(&o,"receipt105","2026-03-24",&p,"purchase180-line",7000,105)?)?;
    s.post_procurement_command(bill(&o,"bill30","2026-03-25",vec![("receipt75-line",2000,30)],30)?)?;
    s.post_procurement_command(bill(&o,"bill150","2026-03-26",vec![("receipt75-line",3000,45),("receipt105-line",7000,105)],150)?)?;
    s.post_procurement_command(transfer(&o,"transfer70","2026-03-27",6000)?)?;
    let order:SalesCommand=serde_json::from_value(json!({"Order":{"context":context(&o,"sales1499","2026-03-27"),"customer_id":"party-a","reference":"SO1499","document_date":"2026-03-27","lines":[{"id":"sale-line-a","item_id":"item-a","warehouse_id":"w1","unit_id":"piece","quantity_thousandths":5000,"unit_price":251,"line_discount":4,"price_reference":"Independent customer price A","price_date":"2026-03-27","override_reason":null},{"id":"sale-line-b","item_id":"item-a","warehouse_id":"w2","unit_id":"piece","quantity_thousandths":2000,"unit_price":175,"line_discount":1,"price_reference":"Independent customer price B","price_date":"2026-03-27","override_reason":null}],"document_discount":101,"supplied_total":1499,"quotation":null}}))?;
    let order_id=s.post_sales_command(order)?.record_id;
    let first=s.post_sales_command(fulfill(&o,"fulfill633","2026-03-28",&order_id,2000,1000,633)?)?.record_id;
    let later=s.post_procurement_command(buy(&o,"purchase120","2026-03-29",6000,20,120)?)?.record_id;
    s.post_procurement_command(receive(&o,"receipt120","2026-03-29",&later,"purchase120-line",6000,120)?)?;
    ensure!(s.preview_procurement(&transfer(&o,"transfer53","2026-03-30",4000)?)?.total.rials()==53,"independent post-sale interleaved transfer cost53");
    s.post_procurement_command(transfer(&o,"transfer53","2026-03-30",4000)?)?;
    s.post_sales_command(fulfill(&o,"fulfill866","2026-03-31",&order_id,3000,1000,866)?)?;
    s.post_sales_command(restock(&o,"return235","2026-04-01",&first,vec![("fulfill633-a",1000,235,12)],235,12)?)?;
    s.post_procurement_command(bill(&o,"bill120","2026-04-02",vec![("receipt120-line",6000,120)],120)?)?;
    s.post_sales_command(restock(&o,"return398","2026-04-03",&first,vec![("fulfill633-a",1000,234,11),("fulfill633-b",1000,164,11)],398,22)?)?;
    let mut reports=vec![];for row in expected["cutoffs"].as_array().unwrap(){reports.push(check(&s,&o,row)?);}
    let mut store=Store::open(&out.join("profile"))?;store.change(|x|{x.accounting=s.clone();Ok(())})?;store.backup(&out.join("backup.json"))?;
    let restored=Store::restore_preview(&out.join("backup.json"))?;let mut fresh=Store::open(&out.join("fresh-restore"))?;fresh.restore(restored)?;
    ensure!(serde_json::to_value(&fresh.state)?==serde_json::to_value(&store.state)?,"complete interleaved State fresh restore exact");
    fs::write(out.join("state.json"),serde_json::to_vec_pretty(&store.state)?)?;
    fs::write(out.join("cutoff-reports.json"),serde_json::to_vec_pretty(&reports)?)?;
    println!("PASS independent eight-cutoff interleaved purchase/sale/transfer/return fixture, exact controls and fresh restore. Final Inv550 AR1066 AP449 GRNI50 COGS50 P&L816 TB3050.");
    Ok(())
}
