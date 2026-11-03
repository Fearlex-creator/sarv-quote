//! Per-step cost of a save at a given size. cargo run --release -p sarv-quote --example write_breakdown -- 50000
use anyhow::Result;
use sarv_quote::{imports, store::Store};
use std::time::Instant;
fn main() -> Result<()> {
    let n: usize = std::env::args().nth(1).and_then(|v| v.parse().ok()).unwrap_or(50_000);
    let dir = tempfile::tempdir()?;
    let source = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
    let mut csv = String::from("code,name,measure,price,currency,source_at,aliases\n");
    for i in 0..n { csv.push_str(&format!("SYN-{i:05},Synthetic product {i},pcs,{},rial,{source},alias {i}\n", 100_000 + i)); }
    let mut store = Store::open(dir.path())?;
    let c = imports::catalog_csv(&csv, "probe")?;
    store.change(|s| imports::accept_catalog(s, c).map(|_| ()))?;
    let s = &store.state;
    macro_rules! t { ($l:expr, $e:expr) => {{ let t = Instant::now(); let r = $e; println!("{:<28}{:?}", $l, t.elapsed()); r }}; }
    let _c = t!("clone State", s.clone());
    let json = t!("serialize (compact)", serde_json::to_string(s)?);
    t!("serialize (pretty, discard)", serde_json::to_writer_pretty(std::io::sink(), s)?);
    t!("parse State", serde_json::from_str::<sarv_quote::domain::State>(&json)?);
    t!("State::validate", s.validate()?);
    t!("validate_transition(desktop)", s.desktop.validate_transition(&s.desktop)?);
    t!("validate_transition(accounting)", s.accounting.validate_transition(&s.accounting)?);
    println!("payload bytes: {}", json.len());
    Ok(())
}
