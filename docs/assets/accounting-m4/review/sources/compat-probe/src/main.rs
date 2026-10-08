use anyhow::{Result,ensure};
use sarv_quote::{store::Store,export};
use std::{fs,path::PathBuf};
fn main()->Result<()> {
 let args:Vec<_>=std::env::args().collect();let sources=PathBuf::from(&args[1]);let out=PathBuf::from(&args[2]);fs::create_dir_all(&out)?;
 for name in ["actual-quotes-manual","actual-detailed-opening","actual-zero-initialization"] {
  let from=sources.join(name);let paths=fs::read_dir(&from)?.map(|p|p.unwrap().path()).filter(|p|p.file_name().unwrap().to_string_lossy().starts_with("before-schema5-")).collect::<Vec<_>>();ensure!(paths.len()==1,"one original actual v4 migration recovery");let path=&paths[0];
  let envelope:serde_json::Value=serde_json::from_slice(&fs::read(path)?)?;ensure!(envelope["format"]=="sarv-accounting-backup-v4","originalv4 format");let prior=Store::restore_preview(path)?;ensure!(prior.schema==4&&serde_json::to_value(&prior)?==envelope["state"],"old M3 Store verifies exact original fullv4 state");
  let csv=prior.quotes.iter().map(export::csv_bytes).collect::<Result<Vec<_>>>()?;let dir=out.join(name);let mut store=Store::open(&dir)?;store.restore(prior.clone())?;ensure!(serde_json::to_value(&store.state)?==envelope["state"],"oldM3freshrestore exact entire state");drop(store);let store=Store::open(&dir)?;ensure!(store.state.schema==4&&serde_json::to_value(&store.state)?==envelope["state"],"oldM3reopen retains schema4 exact history");ensure!(csv==store.state.quotes.iter().map(export::csv_bytes).collect::<Result<Vec<_>>>()?,"oldM3quoteCSVbyteexact");store.backup(&dir.join("m3-rerestored-v4.json"))?;
  let db=rusqlite::Connection::open(dir.join("sarv.sqlite3"))?;let integrity:String=db.query_row("PRAGMA integrity_check",[],|r|r.get(0))?;ensure!(integrity=="ok","oldM3restoredintegrity");
  let report=serde_json::json!({"result":"passed","old_implementation_commit":"bdc304cb139e3d258b107b9ce18c7f4682ec5a89","recovery":path,"restored_schema":4,"full_state_exact":true,"quotes":prior.quotes.len(),"journals":prior.accounting.journals.len(),"openings":prior.accounting.core.openings.len(),"zero_aliases":prior.accounting.core.zero_requests.len(),"quotation_csv_byte_exact":true,"fresh_old_store_restore_reopen":true,"sqlite_integrity":integrity});fs::write(dir.join("independent-old-m3-verification.json"),serde_json::to_vec_pretty(&report)?)?;
  println!("PASS actual preM4 recovery {name}: archived M3 library verifies/restores/reopens exact schema4 fullhistory and quotes byteexact");
 }
 Ok(())
}
