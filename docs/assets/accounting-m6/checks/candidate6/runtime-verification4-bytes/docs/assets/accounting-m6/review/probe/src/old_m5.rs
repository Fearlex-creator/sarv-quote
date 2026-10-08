// Compile ONLY against archived immutable M5 source. It uses no M6 return API.
mod opening;
mod common;
use anyhow::{Result,ensure};
use sarv_quote::store::Store;
use std::{fs,path::PathBuf};
fn main()->Result<()> {
 let args=std::env::args().collect::<Vec<_>>();let path=PathBuf::from(&args[2]);
 if args[1]=="--open"{match Store::open(&path){Ok(_)=>anyhow::bail!("old M5 unexpectedly accepted profile"),Err(e)=>{eprintln!("Actual archived M5 refusal: {e:#}");std::process::exit(13)}}}
 ensure!(args[1]=="--seed","expected --seed or --open");
 let mut st=Store::open(&path)?;ensure!(st.state.schema==6,"genuine oldM5 State default schema6");
 if let Ok(input)=std::env::var("SARV_REVIEW_M5_INPUT_STATE") {
  let raw:serde_json::Value=serde_json::from_slice(&fs::read(input)?)?;
  let original:sarv_quote::domain::State=serde_json::from_value(raw.get("state").unwrap_or(&raw).clone())?;
  ensure!(original.schema==6&&!original.quotes.is_empty(),"actual native M5 schema6 quotation-bearing fixture");
  original.validate()?;st.change(|s|{*s=original;Ok(())})?;
 } else {let(a,_)=common::baseline()?;st.change(|s|{s.accounting=a;Ok(())})?;}
 let o=st.state.accounting.core.openings[0].command.clone();
 st.backup(&path.join("actual-old-M5-v6.json"))?;
 fs::write(path.join("actual-old-M5-state.json"),serde_json::to_vec_pretty(&st.state)?)?;
 let r=st.state.accounting.sales_reconcile(&o.company_id,"2026-03-31")?;ensure!(r.reconciled&&r.profit.rials()==41&&r.stock.iter().map(|r|r.value.rials()).sum::<i64>()==422,"genuine M5 baseline");println!("Actual archived M5 schema6 seed created; Inv422 profit41");Ok(())
}
