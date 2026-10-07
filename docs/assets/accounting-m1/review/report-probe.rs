use sarv_quote::{accounting::{Money,MoneyUnit,TrialBalance,TrialBalanceRow},accounting_export::{trial_csv,trial_pdf}};
use std::path::Path;
fn main()->Result<(),Box<dyn std::error::Error>> {
 let output=std::env::args().nth(1).expect("fresh artifact directory");let dir=Path::new(&output);assert!(!dir.exists());std::fs::create_dir_all(dir)?;
 let row=|id:&str,code:String,name:String,debit:i64,credit:i64|TrialBalanceRow{account_id:id.into(),code,name,debit:Money::from_rials(debit),credit:Money::from_rials(credit)};
 let mut balance=TrialBalance{company_id:"SYNTHETIC-INDEPENDENT-REPORT".into(),as_of:"2026-10-07".into(),rows:vec![row("1",format!("{} LONG-END-01", "LONG-CODE-ABCD ".repeat(24)),"ROW-NAME-01 حساب دقیق".into(),1251,0),row("2","SHORT-END-02".into(),"ROW-NAME-02 سرمایه دقیق".into(),0,1251)],total_debit:Money::from_rials(1251),total_credit:Money::from_rials(1251)};
 trial_pdf("شرکت مصنوعی مستقل • REPORT-COMPANY",&balance,MoneyUnit::Toman,&dir.join("long-code-trial.pdf"))?;
 trial_csv("شرکت مصنوعی مستقل • REPORT-COMPANY",&balance,MoneyUnit::Toman,&dir.join("long-code-trial.csv"))?;
 balance.rows[0].code="X".repeat(1024);
 assert!(trial_pdf("شرکت مصنوعی مستقل",&balance,MoneyUnit::Rial,&dir.join("invalid-row.pdf")).is_err());assert!(!dir.join("invalid-row.pdf").exists());
 trial_csv("شرکت مصنوعی مستقل",&balance,MoneyUnit::Rial,&dir.join("fallback-long-row.csv"))?;
 balance.rows[0].code="SHORT".into();
 assert!(trial_pdf(&"W".repeat(1024),&balance,MoneyUnit::Rial,&dir.join("invalid-heading.pdf")).is_err());assert!(!dir.join("invalid-heading.pdf").exists());
 println!("PASS long-code report generated; exact1251 IRR/125.1 toman; unsupported row/header rejected without destination publication; CSV fallback complete. Actual PDF render/text inspection pending.");
 Ok(())
}
