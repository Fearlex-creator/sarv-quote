use anyhow::{Result, ensure};
use sarv_quote::{accounting::*, store::Store};
use std::{fs, path::PathBuf};
mod opening;

fn main() -> Result<()> {
    let out = PathBuf::from(std::env::args().nth(1).expect("private output directory"));
    fs::create_dir_all(&out)?;
    let mut store = Store::open(&out.join("actual-m4-profile"))?;
    let (company, period, asset, equity) = store.change(|state| {
        let a = &mut state.accounting;
        let c = a.add_company("Independent actual M4 sales-prefix synthetic fixture", "reviewer")?;
        let p = a.add_period(&c, "2026", "2026-01-01", "2026-12-31", "reviewer")?;
        let mut input = NewAccount { company_id:c.clone(), parent_id:None, code:"A".into(), name:"Ordinary asset".into(), classification:AccountClass::Asset, normal_balance:BalanceSide::Debit, postable:true, control:false };
        let asset = a.add_account(input.clone(), "reviewer")?;
        input.code="E".into(); input.name="Supplied capital".into(); input.classification=AccountClass::Equity; input.normal_balance=BalanceSide::Credit;
        let equity=a.add_account(input, "reviewer")?;
        Ok((c,p,asset,equity))
    })?;
    let command = PostCommand { company_id:company.clone(), period_id:period.clone(), date:"2026-03-22".into(), source_id:"actual-m4-legacy-sales-source".into(), source_revision:1, event:"sales.invoice.v1".into(), request_key:"actual-m4-sales-key".into(), actor:"reviewer".into(), reason:"Permitted ordinary manual source under tagged M4, not a typed goods sale".into(), kind:JournalKind::Manual, lines:vec![JournalLineInput { account_id:asset.clone(), debit:Money::from_rials(100), credit:Money::ZERO, description:"supplied asset".into() }, JournalLineInput { account_id:equity.clone(), debit:Money::ZERO, credit:Money::from_rials(100), description:"supplied capital".into() }] };
    let first=store.change(|s| s.accounting.post(command.clone()))?;
    let mut alias=command.clone(); alias.request_key="actual-m4-sales-alias".into();
    ensure!(store.change(|s| s.accounting.post(alias))? == first, "M4 alias one effect");
    store.state.validate()?;
    ensure!(store.state.schema==5 && store.state.accounting.procurement.is_empty(), "Actual M4 schema5 manual namespace");
    store.backup(&out.join("actual-m4-sales-prefix-v5.json"))?;
    let mut revision=command; revision.source_revision=2; revision.request_key="actual-m4-sales-revision2".into(); revision.lines[0].debit=Money::from_rials(200); revision.lines[1].credit=Money::from_rials(200);
    let second=store.change(|s| s.accounting.post(revision))?;
    ensure!(first!=second, "Existing M4 generic revision identity retained");
    store.backup(&out.join("actual-m4-sales-two-revisions-v5.json"))?;
    store.change(|s| s.accounting.reverse(&first, &period, "2026-03-23", "actual-m4-sales-reversal", "reviewer", "Linked exact ordinary M4 reversal"))?;
    store.change(|s| s.accounting.set_period_lock(&period, true, "reviewer", "Locked historical M4 fixture"))?;
    store.state.validate()?;
    store.backup(&out.join("actual-m4-sales-reversed-locked-v5.json"))?;
    ensure!(store.state.accounting.trial_balance(&company,"2026-03-23")?.total_debit.rials()==200, "100+200-100 literal trial balance");
    fs::write(out.join("actual-m4-state.json"),serde_json::to_vec_pretty(&store.state)?)?;
    println!("PASS actual tagged M4 schema5 sales.invoice.v1 manual100+alias; independent revision200; original linked reversal100; lock; three checksummed backups. No M5 acceptance claimed.");
    let (mut core, opening)=opening::fixture()?;
    core.post_opening(opening.clone())?;
    let setup=&core.core.setups[0];
    let asset=setup.account_id(sarv_quote::core::CoreRole::FixedAssets)?.to_string();
    let sales=setup.account_id(sarv_quote::core::CoreRole::Sales)?.to_string();
    let returns=setup.account_id(sarv_quote::core::CoreRole::SalesReturns)?.to_string();
    for (source, debit, credit, amount) in [("legacy-core-sales17",asset.clone(),sales,17),("legacy-core-returns3",returns,asset,3)] {
        core.post(PostCommand {company_id:opening.company_id.clone(),period_id:opening.period_id.clone(),date:"2026-03-22".into(),source_id:source.into(),source_revision:1,event:"manual".into(),request_key:source.into(),actor:"reviewer".into(),reason:"Explicit valid M4 mapped ordinary revenue adjustment; no invented invoice".into(),kind:JournalKind::Manual,lines:vec![JournalLineInput {account_id:debit,debit:Money::from_rials(amount),credit:Money::ZERO,description:source.into()},JournalLineInput {account_id:credit,debit:Money::ZERO,credit:Money::from_rials(amount),description:source.into()}]})?;
    }
    core.validate()?;
    ensure!(core.trial_balance(&opening.company_id,"2026-03-22")?.total_debit.rials()==1268,"literal ordinary Sales17/SalesReturns3 fixture TB1268");
    let mut adjusted=Store::open(&out.join("actual-m4-core-adjustment-profile"))?;
    adjusted.change(|s|{s.accounting=core;Ok(())})?;
    adjusted.backup(&out.join("actual-m4-core-ordinary-revenue-v5.json"))?;
    println!("PASS actual tagged M4 F2 core permits mapped ordinary Sales17 and SalesReturns3 with asset14, TB1268, no typed invoice. Future module-reconciliation compatibility input only.");
    Ok(())
}
