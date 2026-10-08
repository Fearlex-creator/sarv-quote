use sarv_quote::{accounting::AccountingState, store::Store};
use std::path::Path;
fn main()->Result<(),Box<dyn std::error::Error>> {
 let source=Store::restore_preview(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/ledger-fixture.json")))?;
 assert_eq!(source.accounting.journals.len(),3);
 let destination=std::env::args().nth(1).expect("fresh synthetic destination required");
 let base=Path::new(&destination);assert!(!base.exists());
 let mut store=Store::open(base)?;store.restore(source.clone())?;
 store.state.accounting=AccountingState::default();
 let result=store.change(|s| {s.draft.customer="cached bypass synthetic".into();Ok(())});
 let persisted=Store::open(base)?.state.accounting.journals.len();
 println!("change result {:?}; persisted journal count {persisted}; expected rejection and 3 journals",result);
 assert!(result.is_err());assert_eq!(persisted,3);

 let prepared_directory=base.join("prepared");let mut prepared_store=Store::open(&prepared_directory)?;prepared_store.restore(source.clone())?;
 let generation=prepared_store.local_generation();
 prepared_store.state.accounting=AccountingState::default();
 let prepared=Store::prepare_candidate(generation,prepared_store.state.clone())?;
 let result=prepared_store.commit_prepared(prepared);
 let persisted=Store::open(&prepared_directory)?.state.accounting.journals.len();
 println!("prepared result {:?}; persisted journal count {persisted}; expected rejection and 3 journals",result);
 assert!(result.is_err());assert_eq!(persisted,3);
 let detached_directory=base.join("detached");let mut detached=Store::open(&detached_directory)?;detached.restore(source)?;
 let (generation,mut candidate)=detached.begin_detached()?;candidate.accounting=AccountingState::default();
 let result=Store::commit_detached(detached_directory.clone(),Store::prepare_candidate(generation,candidate)?);
 assert!(detached.finish_detached(result).is_err());
 assert_eq!(Store::open(&detached_directory)?.state.accounting.journals.len(),3);
 println!("PASS synchronous change, prepared and detached candidates cannot remove committed history; explicit verified restore remains permitted.");
 Ok(())
}
