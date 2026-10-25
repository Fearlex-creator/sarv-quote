//! One-off verification of a real profile copy: open, validate, backup, restore.
//! Not part of the product; run with a copied profile directory.
use sarv_quote::store::Store;

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: profile_doctor <profile-dir>"))?;
    let dir = std::path::Path::new(&path);
    let store = Store::open(dir)?;
    let s = &store.state;
    println!("schema: {}", s.schema);
    println!("generation: {}", store.local_generation());
    println!("companies: {}", s.accounting.companies.len());
    for c in &s.accounting.companies {
        let journals = s
            .accounting
            .journals
            .iter()
            .filter(|j| j.company_id == c.id)
            .count();
        let sales = s
            .accounting
            .sales
            .records
            .iter()
            .filter(|r| r.command.context().company_id == c.id)
            .count();
        let purchases = s
            .accounting
            .procurement
            .records
            .iter()
            .filter(|r| r.command.context().company_id == c.id)
            .count();
        let payments = s
            .accounting
            .payments
            .records
            .iter()
            .filter(|r| r.command.context().company_id == c.id)
            .count();
        println!(
            "  {}: journals={journals} sales={sales} purchases={purchases} payments={payments}",
            c.name
        );
    }
    println!(
        "quotes={} drafts={} customers={}",
        s.quotes.len(),
        s.sales_drafts.len(),
        s.customers.len()
    );
    s.accounting.validate()?;
    println!("validate: OK (full audit replay reconstructed the books)");
    let backup = dir.join("doctor-backup.json");
    store.backup(&backup)?;
    let restored = Store::restore_preview(&backup)?;
    println!(
        "backup+restore-preview: OK ({} bytes)",
        std::fs::metadata(&backup)?.len()
    );
    std::fs::remove_file(&backup)?;
    let fresh = tempfile::tempdir()?;
    let mut target = Store::open(fresh.path())?;
    target.restore(restored)?;
    let reopened = Store::open(fresh.path())?;
    if serde_json::to_value(&reopened.state)? == serde_json::to_value(&store.state)? {
        println!("fresh restore + reopen: identical state");
    } else {
        anyhow::bail!("restored state differs from original");
    }
    println!("PROFILE HEALTHY");
    Ok(())
}
