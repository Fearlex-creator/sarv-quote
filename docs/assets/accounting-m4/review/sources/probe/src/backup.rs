use anyhow::{Result,ensure};
use sarv_quote::{domain::State,store::backup_state};
use std::path::Path;
fn main()->Result<()> {
 let args:Vec<_>=std::env::args().collect();let db=rusqlite::Connection::open_with_flags(Path::new(&args[1]).join("sarv.sqlite3"),rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;let payload:String=db.query_row("select payload from state where id=1",[],|r|r.get(0))?;let state:State=serde_json::from_str(&payload)?;state.validate()?;ensure!(state.schema==5&&state.accounting.procurement.records.len()==10&&state.accounting.journals.len()==5,"authentic native full10record/5J fixture");backup_state(&state,Path::new(&args[2]))?;println!("PASS read-only actual native full10record/5J snapshot fully validates; schema5 complete backup created without changing source profile");Ok(())
}
