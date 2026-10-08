mod opening;
use anyhow::{Result,ensure};
use sarv_quote::{accounting::*,core::*,sales::*};
use serde_json::{Value,json};
use std::{fs,path::PathBuf};
fn context(o:&OpeningCommand,s:&str)->Value {json!({"company_id":o.company_id,"period_id":o.period_id,"setup_id":o.setup_id,"source_id":s,"source_revision":1,"date":"2026-03-22","request_key":format!("residual-key-{s}"),"actor":"Independent reviewer","reason":"Literal residual fixture"})}
fn line(id:&str,q:i64,price:i64)->Value {json!({"id":id,"item_id":"item-a","warehouse_id":"w1","unit_id":"piece","quantity_thousandths":q,"unit_price":price,"line_discount":0,"price_reference":"Literal independent price","price_date":"2026-03-22","override_reason":null})}
fn order(o:&OpeningCommand,s:&str,lines:Vec<Value>,discount:i64,total:i64)->Result<SalesCommand>{Ok(serde_json::from_value(json!({"Order":{"context":context(o,s),"customer_id":"party-a","reference":s,"document_date":"2026-03-22","lines":lines,"document_discount":discount,"supplied_total":total,"quotation":null}}))?)}
fn main()->Result<()> {
 let out=PathBuf::from(std::env::args().nth(1).expect("private output"));fs::create_dir_all(&out)?;let mut rows=vec![];
 for (name,quantity,discount,part) in [("one-rial-three-pieces",3000,2,1000),("divisible-derived-gross",1500,1,500)] {
  let (mut state,original)=opening::fixture()?;let mut value=serde_json::to_value(original)?;
  for b in value["balances"].as_array_mut().unwrap(){let id=b["account_id"].as_str().unwrap().to_owned();if id==state.core.setups[0].account_id(CoreRole::Inventory)?{b["debit"]=1.into();}if id==state.core.setups[0].account_id(CoreRole::Grni)?{b["credit"]=0.into();}if id==state.core.setups[0].account_id(CoreRole::OwnerCapital)?{b["credit"]=703.into();}}
  value["grni"]=json!([]);value["stock"].as_array_mut().unwrap().truncate(1);value["stock"][0]["unit_id"]="piece".into();value["stock"][0]["quantity_thousandths"]=quantity.into();value["stock"][0]["value"]=1.into();value["items"][0]["indivisible"]=(part==1000).into();
  let o:OpeningCommand=serde_json::from_value(value)?;state.post_opening(o.clone())?;
  let id=state.post_sales_command(order(&o,name,vec![line("residual-order-line",quantity,1)],discount,1)?)?.record_id;
  let mut components=vec![];let mut invoice_ids=vec![];
  for n in 0..3 {
   let s=format!("fulfillment-{n}");let c:SalesCommand=serde_json::from_value(json!({"Fulfill":{"context":context(&o,&s),"order_id":id,"delivery_id":format!("delivery-{n}"),"invoice_id":format!("invoice-{n}"),"delivery_reference":format!("DEL-{n}"),"invoice_reference":format!("INV-{n}"),"document_date":"2026-03-22","due_date":"2026-04-01","lines":[{"id":format!("issued-{n}"),"order_line_id":"residual-order-line","warehouse_id":"w1","unit_id":"piece","quantity_thousandths":part}],"supplied_total":if n==1{1}else{0}}}))?;
   let result=state.post_sales_command(c)?;let SalesData::Fulfill(f)=&state.sales_document(&result.record_id)?.data else{unreachable!()};let l=&f.lines[0];let expected_discount=if discount==2{if n==1{0}else{1}}else{if n==1{1}else{0}};
   ensure!(l.net.rials()==if n==1{1}else{0}&&l.cost.rials()==if n==1{1}else{0}&&l.document_discount.rials()==expected_discount,"literal cumulative components");ensure!(l.gross==l.net.checked_add(l.document_discount)?,"derived original gross conservation");ensure!(result.journal_id.is_some()==(n==1),"zero-money/cost issue has no empty financial journal");components.push(json!({"net":l.net.rials(),"cost":l.cost.rials(),"discount":l.document_discount.rials(),"gross":l.gross.rials(),"journal":result.journal_id.is_some()}));invoice_ids.push(result.record_id);
  }
  ensure!(state.sales_order_remaining(&id)?[0].remaining_quantity_thousandths==0,"full original quantity issued");
  for n in 0..3 {
   let s=format!("return-{n}");let amount=if n==1{1}else{0};let c:SalesCommand=serde_json::from_value(json!({"Return":{"context":context(&o,&s),"fulfillment_id":invoice_ids[n],"return_id":format!("restock-{n}"),"credit_id":format!("credit-{n}"),"return_reference":format!("RET-{n}"),"credit_reference":format!("CR-{n}"),"document_date":"2026-03-22","lines":[{"id":format!("returned-{n}"),"fulfillment_line_id":format!("issued-{n}"),"quantity_thousandths":part,"supplied_credit":amount,"supplied_cost":amount}],"supplied_credit":amount,"supplied_cost":amount}}))?;let r=state.post_sales_command(c)?;ensure!(r.journal_id.is_some()==(n==1),"zero return has operational effect and no empty financial journal");
  }
  let report=state.sales_reconcile(&o.company_id,"2026-03-22")?;ensure!(report.reconciled&&report.stock.iter().map(|p|p.value.rials()).sum::<i64>()==1&&report.cogs==Money::ZERO&&report.sales.rials()==1&&report.sales_returns.rials()==1&&report.profit==Money::ZERO,"complete tiny sale/return controls");ensure!(state.sales_order_remaining(&id)?[0].remaining_quantity_thousandths==0,"returns never reopen order");state.validate()?;fs::write(out.join(format!("{name}-accounting.json")),serde_json::to_vec_pretty(&state)?)?;rows.push(json!({"fixture":name,"components":components,"full_return_restores_inventory1":true,"orders_remain_fulfilled":true}));
 }
 let(mut state,o)=opening::fixture()?;state.post_opening(o.clone())?;let id=state.post_sales_command(order(&o,"stable-tie",vec![line("tie-c",1000,100),line("tie-a",1000,100),line("tie-b",1000,100)],2,298)?)?.record_id;let SalesData::Order(data)=&state.sales_document(&id)?.data else{unreachable!()};ensure!(data.lines.iter().map(|l|l.document_discount.rials()).collect::<Vec<_>>()==vec![0,1,1],"stable ID tie allocation independent of input order");state.validate()?;rows.push(json!({"fixture":"equal weights in input order c,a,b","document_discounts":[0,1,1]}));fs::write(out.join("residual-observations.json"),serde_json::to_vec_pretty(&rows)?)?;println!("PASS independent one-rial/divisible residuals, nonfinancial zero issues/returns, original cost/full conservation, immutable-ID discount tie.");Ok(())
}
