#[allow(dead_code)] #[path="main.rs"] mod fixture;
use anyhow::{Result,ensure};
use sarv_quote::{procurement::ProcurementCommand,procurement_export as ex,domain::{DateDisplay,DisplayDigits},accounting::MoneyUnit};
use std::{fs,path::PathBuf};
fn document(c:ProcurementCommand,day:&str)->Result<ProcurementCommand>{let mut v=serde_json::to_value(c)?;v.as_object_mut().unwrap().iter_mut().next().unwrap().1["document_date"]=day.into();Ok(serde_json::from_value(v)?)}
fn main()->Result<()> {
 let out=PathBuf::from(std::env::args().nth(1).unwrap());fs::create_dir_all(&out)?;let(mut s,o)=fixture::base()?;
 let po=document(fixture::order(&o,"date-po","2026-03-21","piece",12000,15,180)?,"2026-03-20")?;let order=s.post_procurement_command(po)?.record_id;
 let rec=document(fixture::receive(&o,"date-receipt","2026-03-22",&order,"date-po-line","piece",5000,75)?,"2026-03-21")?;let rid=s.post_procurement_command(rec)?.record_id;
 let bill=document(fixture::bill(&o,"date-bill","2026-03-24","DATE-BILL",vec![("date-receipt-line",2000,30)],30)?,"2026-03-23")?;let bid=s.post_procurement_command(bill)?.record_id;s.validate()?;
 fixture::checked(&s,&o,"2026-03-21",(24000,240),(6000,60),149,50,1251)?;fixture::checked(&s,&o,"2026-03-22",(29000,315),(6000,60),149,125,1326)?;fixture::checked(&s,&o,"2026-03-24",(29000,315),(6000,60),179,95,1326)?;
 let recon=s.procurement_reconcile(&o.company_id,"2026-03-24")?;ensure!(recon.receipts.iter().find(|r|r.id=="date-receipt-line").unwrap().date=="2026-03-22"&&recon.receipts.iter().find(|r|r.id=="r1").unwrap().date=="2026-03-21","allocation uses distinct posting/opening availability");
 ex::document_csv(&s,&o.company_id,&rid,MoneyUnit::Rial,DateDisplay::Iso,DisplayDigits::Latin,&out.join("receipt.csv"))?;ex::document_csv(&s,&o.company_id,&bid,MoneyUnit::Rial,DateDisplay::Iso,DisplayDigits::Latin,&out.join("bill.csv"))?;ex::reconciliation_csv(&s,&o.company_id,"2026-03-24",MoneyUnit::Rial,DateDisplay::Iso,DisplayDigits::Latin,&out.join("control.csv"))?;fs::write(out.join("accounting.json"),serde_json::to_vec_pretty(&s)?)?;
 println!("PASS independently distinct PO paper20/post21, receipt paper21/post22, bill paper23/post24/dueMay1, originalR1 paperMar1/availableMar21; exact EOD posting cutoffs and source dates retained");Ok(())
}
