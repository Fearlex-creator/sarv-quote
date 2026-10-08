use sarv_quote_legacy_verifier::{store::Store,export};
fn main() -> anyhow::Result<()> {
 let args: Vec<String> = std::env::args().collect();
 if args[1]=="prepare" {
  let mut value: serde_json::Value=serde_json::from_slice(&std::fs::read(&args[2])?)?;
  assert!(value.get("accounting").is_none(),"only an empty-accounting synthetic quote profile may be used");
  value["schema"]=serde_json::json!(1);
  let state=serde_json::from_value(value)?;
  sarv_quote_legacy_verifier::store::backup_state(&state,std::path::Path::new(&args[3]))?;
  println!("LEGACY_FIXTURE_CREATED");return Ok(());
 }
 let backup=std::path::Path::new(&args[1]);let directory=std::path::Path::new(&args[2]);
 let original=Store::restore_preview(backup)?;
 assert_eq!(original.schema,1);
 let quotes=serde_json::to_vec(&original.quotes)?;
 let mut store=Store::open(directory)?; store.restore(original)?;drop(store);
 let store=Store::open(directory)?;
 assert_eq!(store.state.schema,1);assert_eq!(serde_json::to_vec(&store.state.quotes)?,quotes);
 if let Some(q)=store.state.quotes.first(){export::pdf(q,&directory.join("legacy-quote.pdf"))?;}
 println!("LEGACY_RESTORE_OK schema=1 quotes={} exact_quotes_sha256={}",store.state.quotes.len(),sarv_quote_legacy_verifier::imports::hash(&quotes));
 Ok(())
}
