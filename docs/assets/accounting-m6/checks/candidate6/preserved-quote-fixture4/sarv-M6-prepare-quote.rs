use sarv_quote::store::Store;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path=std::path::PathBuf::from(std::env::args().nth(1).expect("private synthetic profile"));
    let mut store=Store::open(&path)?;
    let quote=store.state.quotes.last().expect("authentic frozen quote").id.clone();
    store.change(|s|s.revise(&quote))?;
    let prepared=store.change(|s| { s.draft.stale_ack=true; s.approve() })?;
    let before=store.state.quotes.len();
    let mut retry=store.state.clone();
    assert_eq!(retry.approve()?,prepared);
    assert_eq!(retry.quotes.len(),before);
    assert!(store.state.draft.lines.len()>=2);
    println!("Prepared via actual Store revise/approve; draft rows={}, quotes={}, same-content retry exact",store.state.draft.lines.len(),before);
    Ok(())
}
