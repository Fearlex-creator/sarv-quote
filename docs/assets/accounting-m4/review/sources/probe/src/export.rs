use anyhow::{Result,ensure};
use sarv_quote::{domain::{State,DateDisplay,DisplayDigits},accounting::MoneyUnit,procurement_export as ex};
use std::{fs,path::PathBuf};
fn main()->Result<()> {
 let args:Vec<_>=std::env::args().collect();let state:State=serde_json::from_slice(&fs::read(&args[1])?)?;state.validate()?;let whole=serde_json::to_value(&state)?;let a=&state.accounting;let c=&a.core.openings[0].command.company_id;let out=PathBuf::from(&args[2]);fs::create_dir_all(&out)?;
 for r in &a.procurement.records {let name=&r.command.context().source_id;ex::document_csv(a,c,&r.id,MoneyUnit::Rial,DateDisplay::Iso,DisplayDigits::Latin,&out.join(format!("{name}.csv")))?;ex::document_pdf(a,c,&r.id,MoneyUnit::Toman,DateDisplay::Jalali,DisplayDigits::Persian,&out.join(format!("{name}.pdf")))?;}
 for date in ["2026-03-21","2026-03-22","2026-03-23","2026-03-24","2026-03-25","2026-03-26","2026-03-27"] {ex::reconciliation_csv(a,c,date,MoneyUnit::Rial,DateDisplay::Iso,DisplayDigits::Latin,&out.join(format!("control-{date}.csv")))?;}
 ex::reconciliation_pdf(a,c,"2026-03-27",MoneyUnit::Toman,DateDisplay::Jalali,DisplayDigits::Persian,&out.join("control-final.pdf"))?;
 ensure!(whole==serde_json::to_value(&state)?,"read-only exports");
 println!("PASS exported 8 immutable source CSV/PDF documents plus 7 exact EOD CSV snapshots/final PDF");
 let valid=a.procurement_reconcile(c,"9999-12-31")?;ensure!(valid.reconciled,"valid Gregorian boundary as-of");
 match ex::reconciliation_csv(a,c,"9999-12-31",MoneyUnit::Rial,DateDisplay::Jalali,DisplayDigits::Latin,&out.join("outside-jalali.csv")){Ok(())=>println!("PASS valid Gregorian beyond Jalali range renders fallback"),Err(e)=>println!("FINDING valid Gregorian9999 as-of rejects Jalali report preference: {e}")};
 Ok(())
}
