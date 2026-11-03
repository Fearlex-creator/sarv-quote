//! Feedback loop for save cost: how long does one tiny edit take as data grows?
//! Run: cargo run --release -p sarv-quote --example write_latency -- 10000
use anyhow::Result;
use sarv_quote::{imports, store::Store};
use std::time::Instant;

fn main() -> Result<()> {
    let products: usize = std::env::args().nth(1).and_then(|v| v.parse().ok()).unwrap_or(10_000);
    let dir = tempfile::tempdir()?;
    let mut csv = String::from("code,name,measure,price,currency,source_at,aliases\n");
    let source = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
    for i in 0..products {
        csv.push_str(&format!("SYN-{i:05},Synthetic product {i},pcs,{},rial,{source},alias {i}\n", 100_000 + i));
    }
    let mut store = Store::open(dir.path())?;
    let t = Instant::now();
    let catalog = imports::catalog_csv(&csv, "probe")?;
    store.change(|s| imports::accept_catalog(s, catalog).map(|_| ()))?;
    println!("import+first save ({products} products): {:?}", t.elapsed());
    let t = Instant::now();
    for i in 0..20 {
        store.change(|s| {
            s.settings.business_name = format!("probe {i}");
            Ok(())
        })?;
    }
    println!("tiny edit save, mean of 20: {:?}", t.elapsed() / 20);
    let bytes = std::fs::metadata(dir.path().join("sarv.sqlite3"))?.len();
    println!("sqlite size: {} KiB", bytes / 1024);
    let t = Instant::now();
    drop(store);
    let _ = Store::open(dir.path())?;
    println!("cold open (parse+validate): {:?}", t.elapsed());
    Ok(())
}
