use sarv_quote::{store::Store,domain::State,export,imports};
use rusqlite::Connection;
fn main()->anyhow::Result<()> {
 let output=std::env::args().nth(1).expect("fresh rehearsal directory with legacy-v1.json");
 let root=std::path::Path::new(&output);
 let envelope:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("legacy-v1.json"))?)?;
 let old:State=serde_json::from_value(envelope["state"].clone())?;assert_eq!(old.schema,1);
 let quotes=serde_json::to_vec(&old.quotes)?;assert_eq!(old.quotes.len(),3);
 let profile=root.join("upgrade");std::fs::create_dir_all(&profile)?;
 let db=Connection::open(profile.join("sarv.sqlite3"))?;
 db.execute_batch("CREATE TABLE state(id INTEGER PRIMARY KEY CHECK(id=1),generation INTEGER NOT NULL,payload TEXT NOT NULL);")?;
 db.execute("INSERT INTO state VALUES(1,42,?1)",[serde_json::to_string(&old)?])?;drop(db);
 let upgraded=Store::open(&profile)?;assert_eq!(upgraded.state.schema,2);assert_eq!(upgraded.local_generation(),43);
 assert_eq!(serde_json::to_vec(&upgraded.state.quotes)?,quotes);
 let recovery=std::fs::read_dir(&profile)?.map(|e|e.unwrap().path()).find(|p|p.file_name().unwrap().to_string_lossy().starts_with("before-schema2-")).unwrap();
 std::fs::copy(&recovery,root.join("verified-before-schema2-v1.json"))?;
 assert_eq!(serde_json::to_vec(&Store::restore_preview(&recovery)?.quotes)?,quotes);
 upgraded.backup(&root.join("upgraded-v2.json"))?;
 let mut restored=Store::open(&root.join("restore-v2"))?;restored.restore(Store::restore_preview(&root.join("upgraded-v2.json"))?)?;
 assert_eq!(serde_json::to_vec(&restored.state)?,serde_json::to_vec(&upgraded.state)?);
 export::pdf(&upgraded.state.quotes[0],&root.join("migrated-quote.pdf"))?;
 drop(upgraded);assert_eq!(Store::open(&profile)?.local_generation(),43);
 let db=Connection::open(profile.join("sarv.sqlite3"))?;assert_eq!(db.query_row("pragma integrity_check",[],|r|r.get::<_,String>(0))?,"ok");
 let result=serde_json::json!({"result":"passed","baseline_source":"4aa82c2988fb2ea48a2fa2ae9824c06187f3d76a","quotes":3,"schema_before":1,"schema_after":2,"generation_before":42,"generation_after":43,"repeat_open_generation":43,"exact_quotes_sha256":imports::hash(&quotes),"fresh_restore_full_state_equal":true,"integrity":"ok","interrupted_schema_upgrade":"untested"});
 std::fs::write(root.join("migration-verification.json"),serde_json::to_vec_pretty(&result)?)?;println!("{result}");Ok(())
}
