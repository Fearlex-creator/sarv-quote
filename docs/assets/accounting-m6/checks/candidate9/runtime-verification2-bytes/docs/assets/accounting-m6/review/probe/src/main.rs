mod opening;
mod common;
use anyhow::{Result,ensure};
use common::*;
use sarv_quote::{procurement::*,store::Store};
use serde_json::{Value,json};
use std::{fs,path::Path};
fn save(s:&sarv_quote::accounting::AccountingState,path:&Path,reports:Vec<Value>)->Result<()> {fs::create_dir_all(path)?;let mut st=Store::open(&path.join("profile"))?;st.change(|x|{x.accounting=s.clone();Ok(())})?;st.backup(&path.join("backup.json"))?;let mut fresh=Store::open(&path.join("fresh-restore"))?;fresh.restore(Store::restore_preview(&path.join("backup.json"))?)?;ensure!(serde_json::to_value(&fresh.state)?==serde_json::to_value(&st.state)?,"complete State fresh restore exact");fs::write(path.join("state.json"),serde_json::to_vec_pretty(&st.state)?)?;fs::write(path.join("cutoff-reports.json"),serde_json::to_vec_pretty(&reports)?)?;Ok(())}
fn main()->Result<()>{
 let out=std::path::PathBuf::from(std::env::args().nth(1).expect("fresh private output dir"));let expected:Value=serde_json::from_str(include_str!("../../literal-expected.json"))?;
 let(base,o)=baseline()?;
 save(&base,&out.join("baseline"),vec![check(&base,&o,&expected["billed_branch"][0])?])?;
 let mut main=base.clone();let first=returned(&o,"billed45","2026-04-01","receipt105-line",Some("bill150-line-1"),3000,45)?;
 let original_query=main.procurement_bill_returnable(&o.company_id,"bill150-line-1")?;ensure!(original_query.original_quantity_thousandths==7000&&original_query.original_value.rials()==105&&original_query.remaining_quantity_thousandths==7000&&original_query.remaining_value.rials()==105,"original specific bill-line source");
 let result=main.post_procurement_command(first.clone())?;ensure!(result.journal_id.is_some(),"valued billed return financial");
 let mut alias=serde_json::to_value(&first)?;alias["Return"]["context"]["request_key"]="independent-fresh-billed-alias".into();let before=main.journals.clone();let records=main.procurement.records.clone();let audits=main.audit.clone();let retry=main.post_procurement_command(serde_json::from_value(alias)?)?;ensure!(retry==result&&main.journals==before&&main.procurement.records==records&&main.audit==audits,"fresh-key alias exact one effect");
 let q=main.procurement_bill_returnable(&o.company_id,"bill150-line-1")?;ensure!(q.remaining_quantity_thousandths==4000&&q.remaining_value.rials()==60,"remaining original billed cursor");
 main.post_procurement_command(returned(&o,"billed60","2026-04-02","receipt105-line",Some("bill150-line-1"),4000,60)?)?;
 let q=main.procurement_bill_returnable(&o.company_id,"bill150-line-1")?;ensure!(q.remaining_quantity_thousandths==0&&q.remaining_value.rials()==0,"final billed cursor exact");
 let mut reports=vec![];for row in expected["billed_branch"].as_array().unwrap(){reports.push(check(&main,&o,row)?);}save(&main,&out.join("billed"),reports)?;
 let mut unbilled=base.clone();let p=unbilled.post_procurement_command(buy(&o,"new90","2026-04-01",6000,15,90)?)?.record_id;
 unbilled.post_procurement_command(receive(&o,"newreceipt90","2026-04-01",&p,"new90-line",6000,90)?)?;
 unbilled.post_procurement_command(bill(&o,"newbill30","2026-04-02",vec![("newreceipt90-line",2000,30)],30)?)?;
 save(&unbilled,&out.join("native-unbilled-stage"),vec![check(&unbilled,&o,&expected["unbilled_interleaved_branch"][2])?])?;
 unbilled.post_procurement_command(returned(&o,"unbilled15","2026-04-03","newreceipt90-line",None,1000,15)?)?;
 let q=unbilled.procurement_receipt_returnable(&o.company_id,"newreceipt90-line")?;ensure!(q.original_quantity_thousandths==6000&&q.original_value.rials()==90&&q.remaining_quantity_thousandths==3000&&q.remaining_value.rials()==45,"shared receipt cursor after partial bill+return");
 unbilled.post_procurement_command(bill(&o,"newbill45","2026-04-04",vec![("newreceipt90-line",3000,45)],45)?)?;
 for(id,date,line,q,v)in[("newcredit15a","2026-04-05","newbill30-line-0",1000,15),("newcredit15b","2026-04-06","newbill30-line-0",1000,15),("newcredit45","2026-04-07","newbill45-line-0",3000,45)]{unbilled.post_procurement_command(returned(&o,id,date,"newreceipt90-line",Some(line),q,v)?)?;}
 let q=unbilled.procurement_receipt_returnable(&o.company_id,"newreceipt90-line")?;ensure!(q.remaining_quantity_thousandths==0&&q.remaining_value.rials()==0,"billed returns never restore receipt eligibility");
 let unchanged=serde_json::to_value(&unbilled)?;ensure!(unbilled.post_procurement_command(bill(&o,"illegal-rebill","2026-04-08",vec![("newreceipt90-line",1000,15)],15)?).is_err()&&serde_json::to_value(&unbilled)?==unchanged,"cannot rebill returned original quantity");
 let mut reports=vec![];for row in expected["unbilled_interleaved_branch"].as_array().unwrap(){reports.push(check(&unbilled,&o,row)?);}save(&unbilled,&out.join("unbilled"),reports)?;
 fs::write(out.join("literal-result.json"),serde_json::to_vec_pretty(&json!({"literal_rows":11,"complete_branch_restores":2,"status":"PASS","product_behavior_tested":true}))?)?;
 println!("PASS independent11 literal cutoffs, shared receipt cursor, specific billed residuals, no rebill and full State restores");Ok(())
}
