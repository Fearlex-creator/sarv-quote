use anyhow::{Result,ensure};
use sarv_quote::{accounting::*,core::*,procurement::*,sales::*};
use serde_json::{Value,json};
pub fn cx(o:&OpeningCommand,id:&str,date:&str)->Value {json!({"company_id":o.company_id,"period_id":o.period_id,"setup_id":o.setup_id,"source_id":id,"source_revision":1,"date":date,"request_key":format!("independent-M6-{id}"),"actor":"Independent M6 reviewer","reason":"Synthetic literal fixture; original cost and no invented adjustment"})}
pub fn buy(o:&OpeningCommand,id:&str,date:&str,q:i64,price:i64,total:i64)->Result<ProcurementCommand> {Ok(serde_json::from_value(json!({"Order":{"context":cx(o,id,date),"supplier_id":"party-a","reference":format!("PO-{id}"),"document_date":date,"lines":[{"id":format!("{id}-line"),"item_id":"item-a","warehouse_id":"w1","unit_id":"piece","quantity_thousandths":q,"unit_price":price,"line_discount":0,"price_reference":"independent supplier price","price_date":date,"override_reason":null}],"document_discount":0,"supplied_total":total}}))?)}
pub fn receive(o:&OpeningCommand,id:&str,date:&str,order:&str,line:&str,q:i64,total:i64)->Result<ProcurementCommand> {Ok(serde_json::from_value(json!({"Receive":{"context":cx(o,id,date),"order_id":order,"reference":format!("GR-{id}"),"document_date":date,"lines":[{"id":format!("{id}-line"),"order_line_id":line,"warehouse_id":"w1","unit_id":"piece","quantity_thousandths":q}],"supplied_total":total}}))?)}
pub fn bill(o:&OpeningCommand,id:&str,date:&str,lines:Vec<(&str,i64,i64)>,total:i64)->Result<ProcurementCommand> {Ok(serde_json::from_value(json!({"Bill":{"context":cx(o,id,date),"supplier_id":"party-a","reference":format!("BILL-{id}"),"document_date":date,"due_date":"2026-05-01","lines":lines.into_iter().enumerate().map(|(i,(r,q,v))|json!({"id":format!("{id}-line-{i}"),"receipt_line_id":r,"quantity_thousandths":q,"supplied_value":v})).collect::<Vec<_>>(),"supplied_total":total}}))?)}
pub fn returned(o:&OpeningCommand,id:&str,date:&str,receipt:&str,bill_line:Option<&str>,q:i64,v:i64)->Result<ProcurementCommand> {Ok(serde_json::from_value(json!({"Return":{"context":cx(o,id,date),"supplier_id":"party-a","return_id":format!("{id}-return"),"return_reference":format!("RET-{id}"),"credit_id":bill_line.map(|_|format!("{id}-credit")),"credit_reference":bill_line.map(|_|format!("CREDIT-{id}")),"document_date":date,"lines":[{"id":format!("{id}-line"),"receipt_line_id":receipt,"bill_line_id":bill_line,"quantity_thousandths":q,"supplied_value":v}],"supplied_total":v}}))?)}
fn fulfill(o:&OpeningCommand,id:&str,date:&str,order:&str,total:i64)->Result<SalesCommand>{Ok(serde_json::from_value(json!({"Fulfill":{"context":cx(o,id,date),"order_id":order,"delivery_id":format!("{id}-delivery"),"invoice_id":format!("{id}-invoice"),"delivery_reference":format!("DEL-{id}"),"invoice_reference":format!("INV-{id}"),"document_date":date,"due_date":"2026-05-01","lines":[{"id":format!("{id}-line"),"order_line_id":"sale199-line","warehouse_id":"w1","unit_id":"piece","quantity_thousandths":5000}],"supplied_total":total}}))?)}
fn sale_return(o:&OpeningCommand,id:&str,date:&str,fulfillment:&str,q:i64,credit:i64,cost:i64)->Result<SalesCommand>{Ok(serde_json::from_value(json!({"Return":{"context":cx(o,id,date),"fulfillment_id":fulfillment,"return_id":format!("{id}-return"),"credit_id":format!("{id}-credit"),"return_reference":format!("RET-{id}"),"credit_reference":format!("CREDIT-{id}"),"document_date":date,"lines":[{"id":format!("{id}-line"),"fulfillment_line_id":"fulfill100-line","quantity_thousandths":q,"supplied_credit":credit,"supplied_cost":cost}],"supplied_credit":credit,"supplied_cost":cost}}))?)}
pub fn baseline()->Result<(AccountingState,OpeningCommand)>{
 let(mut s,o)=super::opening::fixture()?;s.post_opening(o.clone())?;
 let p=s.post_procurement_command(buy(&o,"purchase180","2026-03-22",12000,15,180)?)?.record_id;
 s.post_procurement_command(receive(&o,"receipt75","2026-03-23",&p,"purchase180-line",5000,75)?)?;
 s.post_procurement_command(receive(&o,"receipt105","2026-03-24",&p,"purchase180-line",7000,105)?)?;
 s.post_procurement_command(bill(&o,"bill30","2026-03-25",vec![("receipt75-line",2000,30)],30)?)?;
 s.post_procurement_command(bill(&o,"bill150","2026-03-26",vec![("receipt75-line",3000,45),("receipt105-line",7000,105)],150)?)?;
 s.post_procurement_command(serde_json::from_value(json!({"Transfer":{"context":cx(&o,"transfer70","2026-03-27"),"item_id":"item-a","source_warehouse_id":"w1","target_warehouse_id":"w2","unit_id":"piece","quantity_thousandths":6000}}))?)?;
 let sale:SalesCommand=serde_json::from_value(json!({"Order":{"context":cx(&o,"sale199","2026-03-27"),"customer_id":"party-a","reference":"SO199","document_date":"2026-03-27","lines":[{"id":"sale199-line","item_id":"item-a","warehouse_id":"w1","unit_id":"piece","quantity_thousandths":10000,"unit_price":20,"line_discount":0,"price_reference":"Independent customer20IRR","price_date":"2026-03-27","override_reason":null}],"document_discount":1,"supplied_total":199,"quotation":null}}))?;
 let order=s.post_sales_command(sale)?.record_id;
 let first=s.post_sales_command(fulfill(&o,"fulfill100","2026-03-28",&order,100)?)?.record_id;
 s.post_sales_command(sale_return(&o,"sales-return40","2026-03-29",&first,2000,40,23)?)?;
 s.post_sales_command(sale_return(&o,"sales-return60","2026-03-30",&first,3000,60,35)?)?;
 s.post_sales_command(fulfill(&o,"fulfill99","2026-03-31",&order,99)?)?;
 s.validate()?;Ok((s,o))
}
pub fn check(s:&AccountingState,o:&OpeningCommand,row:&Value)->Result<Value>{
 let date=row["date"].as_str().unwrap();let p=s.procurement_reconcile(&o.company_id,date)?;
 ensure!(p.reconciled&&p.controls.len()==12&&p.controls.iter().all(|c|c.difference==Money::ZERO),"12 procurement controls exact {date}");
 for(role,key)in[(CoreRole::Inventory,"inventory"),(CoreRole::Payables,"ap"),(CoreRole::Grni,"grni")] {let c=p.controls.iter().find(|c|c.role==role).unwrap();ensure!(c.detail.rials()==row[key].as_i64().unwrap()&&c.ledger==c.detail,"literal {role:?} {date}");}
 let w1=p.stock.iter().find(|p|p.item_id=="item-a"&&p.warehouse_id=="w1").unwrap();let w2=p.stock.iter().find(|p|p.item_id=="item-a"&&p.warehouse_id=="w2").unwrap();
 ensure!(w1.quantity_thousandths==row["w1_q"].as_i64().unwrap()*1000&&w1.value.rials()==row["w1_v"].as_i64().unwrap()&&w2.quantity_thousandths==12000&&w2.value.rials()==130,"literal pools {date}");
 ensure!(p.total_debit.rials()==row["tb"].as_i64().unwrap()&&p.total_credit==p.total_debit,"literal TB {date}");
 let sales=s.sales_reconcile(&o.company_id,date)?;ensure!(sales.reconciled&&sales.controls.len()==14&&sales.controls.iter().all(|c|c.difference==Money::ZERO)&&sales.sales.rials()==199&&sales.sales_returns.rials()==100&&sales.cogs.rials()==58&&sales.profit.rials()==41&&sales.receivables.iter().map(|r|r.remaining.rials()).sum::<i64>()==299,"unchanged sales controls {date}");
 s.validate()?;Ok(json!({"date":date,"procurement":p,"sales":sales}))
}

pub fn empty_pool()->Result<(AccountingState,OpeningCommand)>{
 let(mut s,mut o)=super::opening::fixture()?;
 for b in &mut o.balances{b.debit=Money::ZERO;b.credit=Money::ZERO;}
 o.cash.clear();o.banks.clear();o.open_items.clear();o.advances.clear();o.cheques.clear();o.stock.clear();o.grni.clear();
 s.post_opening(o.clone())?;s.validate()?;Ok((s,o))
}
