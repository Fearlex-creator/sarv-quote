use sarv_quote::{accounting::*,store::Store};
use std::path::Path;
fn m(v:i64)->Money {Money::from_rials(v)}
fn main()->Result<(),Box<dyn std::error::Error>> {
 let destination=std::env::args().nth(1).expect("fresh artifact directory required");
 let base=Path::new(&destination);std::fs::create_dir_all(base)?;
 let profile=base.join("profile");let dir=profile.as_path();
 assert!(!dir.exists(),"Use a fresh synthetic probe directory");
 let mut store=Store::open(dir)?;
 let (c,p,a,e)=store.change(|s| {
  let c=s.accounting.add_company("شرکت مصنوعی بازبین","independent-review")?;
  let p=s.accounting.add_period(&c,"2026","2026-01-01","2026-12-31","independent-review")?;
  let a=s.accounting.add_account(NewAccount{company_id:c.clone(),parent_id:None,code:"1001".into(),name:"دارایی آزمایشی".into(),classification:AccountClass::Asset,normal_balance:BalanceSide::Debit,postable:true,control:false},"independent-review")?;
  let e=s.accounting.add_account(NewAccount{company_id:c.clone(),parent_id:None,code:"3001".into(),name:"سرمایه آزمایشی".into(),classification:AccountClass::Equity,normal_balance:BalanceSide::Credit,postable:true,control:false},"independent-review")?;
  Ok((c,p,a,e))
 })?;
 let cmd=|source:&str,key:&str,v:i64,kind:JournalKind,date:&str| PostCommand {company_id:c.clone(),period_id:p.clone(),date:date.into(),source_id:source.into(),source_revision:1,event:"manual-independent".into(),request_key:key.into(),actor:"independent-review".into(),reason:"synthetic technical review; not approved policy".into(),kind,lines:vec![JournalLineInput{account_id:a.clone(),debit:m(v),credit:m(0),description:"independent fixture".into()},JournalLineInput{account_id:e.clone(),debit:m(0),credit:m(v),description:"independent fixture".into()}]};
 let opening=cmd("opening","request-1",1251,JournalKind::Opening,"2026-01-01");
 let open_id=store.change(|s|s.accounting.post(opening.clone()))?;
 assert_eq!(store.state.accounting.trial_balance(&c,"2026-12-31")?.total_debit.rials(),1251);
 let mut retry=opening.clone();retry.request_key="different-retry".into();
 assert_eq!(store.change(|s|s.accounting.post(retry.clone()))?,open_id);
 assert_eq!(store.state.accounting.journals.len(),1);
 let before=store.state.clone();retry.lines[0].debit=m(1252);
 assert!(store.change(|s|s.accounting.post(retry)).is_err());assert_eq!(serde_json::to_value(&before)?,serde_json::to_value(&store.state)?);
 let mut unbalanced=cmd("bad","bad-key",349,JournalKind::Manual,"2026-03-01");unbalanced.lines[1].credit=m(348);
 assert!(store.change(|s|s.accounting.post(unbalanced)).is_err());
 let adjustment=cmd("adjustment","adjust-key",349,JournalKind::Manual,"2026-03-01");
 let adj_id=store.change(|s|s.accounting.post(adjustment.clone()))?;
 assert_eq!(store.state.accounting.trial_balance(&c,"2026-12-31")?.total_debit.rials(),1600);
 store.change(|s|s.accounting.archive_account(&a,"independent-review","archive referenced account"))?;
 let reverse=store.change(|s|s.accounting.reverse(&adj_id,&p,"2026-03-02","reverse-key","independent-review","exact linked correction"))?;
 assert_eq!(store.state.accounting.journals.len(),3);
 assert_eq!(store.state.accounting.trial_balance(&c,"2026-12-31")?.total_debit.rials(),1251);
 assert_eq!(store.state.accounting.trial_balance(&c,"2026-12-31")?.total_debit.display(MoneyUnit::Toman),"125.1");
 let mut tamper=store.state.clone();tamper.accounting.journals[0].lines[0].debit=m(1252);
 assert!(store.change(|s|{*s=tamper;Ok(())}).is_err());
 store.change(|s|s.accounting.set_period_lock(&p,true,"independent-review","lock test"))?;
 assert!(store.change(|s|s.accounting.post(cmd("locked","locked-key",1,JournalKind::Manual,"2026-04-01"))).is_err());
 let mut bypass=store.state.clone();bypass.accounting.periods[0].locked=false;
 assert!(store.change(|s|{*s=bypass;Ok(())}).is_err());
 assert_eq!(store.change(|s|s.accounting.post(opening.clone()))?,open_id);
 let next=store.change(|s|s.accounting.add_period(&c,"2027","2027-01-01","2027-12-31","independent-review"))?;
 let mut second_open=cmd("opening2","opening2-key",1251,JournalKind::Opening,"2027-01-01");second_open.period_id=next;
 assert!(store.change(|s|s.accounting.post(second_open)).is_err());
 store.backup(&base.join("backup.json"))?;
 let expected=store.state.clone();drop(store);
 let reopened=Store::open(dir)?;
 assert_eq!(serde_json::to_value(&expected)?,serde_json::to_value(&reopened.state)?);
 let restored=Store::restore_preview(&base.join("backup.json"))?;
 let mut fresh=Store::open(&base.join("restored"))?;fresh.restore(restored)?;
 assert_eq!(serde_json::to_value(&expected)?,serde_json::to_value(&fresh.state)?);
 for v in [i64::MIN,-1251,-1,0,1,1251,i64::MAX] {
   let value=m(v);assert_eq!(Money::parse(&value.display(MoneyUnit::Toman),MoneyUnit::Toman)?,value);
 }
 assert!(Money::parse("0.01",MoneyUnit::Toman).is_err());assert!(Money::parse("0.1",MoneyUnit::Rial).is_err());
 assert_eq!(Money::parse("۱۲۵.۱",MoneyUnit::Toman)?.rials(),1251);
 range_probe()?;
 println!("PASS independent expected 1251 -> 1600 -> 1251 IRR; archived reversal {reverse}; different-key retry journal count 1; conflicted/unbalanced/locked/direct-history/direct-unlock/second-opening rejected; restart+fresh restore exact; signed range toman roundtrip and Persian digits exact.");
 Ok(())
}

fn range_probe()->Result<(),Box<dyn std::error::Error>> {
 let mut s=AccountingState::default();let c=s.add_company("Range fixture","review")?;
 let p=s.add_period(&c,"2026","2026-01-01","2026-12-31","review")?;
 let a=s.add_account(NewAccount{company_id:c.clone(),parent_id:None,code:"1000".into(),name:"Asset".into(),classification:AccountClass::Asset,normal_balance:BalanceSide::Debit,postable:true,control:false},"review")?;
 let e=s.add_account(NewAccount{company_id:c.clone(),parent_id:None,code:"3000".into(),name:"Equity".into(),classification:AccountClass::Equity,normal_balance:BalanceSide::Credit,postable:true,control:false},"review")?;
 let command=|source:&str,date:&str,amount:i64,swap:bool| PostCommand{company_id:c.clone(),period_id:p.clone(),date:date.into(),source_id:source.into(),source_revision:1,event:"manual".into(),request_key:source.into(),actor:"review".into(),reason:"synthetic range".into(),kind:JournalKind::Manual,lines:vec![JournalLineInput{account_id:a.clone(),debit:m(if swap {0}else{amount}),credit:m(if swap {amount}else{0}),description:"".into()},JournalLineInput{account_id:e.clone(),debit:m(if swap {amount}else{0}),credit:m(if swap {0}else{amount}),description:"".into()}]};
 s.post(command("max","2026-03-01",i64::MAX,false))?;
 let before=s.clone();assert!(s.post(command("overflow","2026-03-02",1,false)).is_err());assert_eq!(s,before);
 s.post(command("reduce","2026-04-01",1,true))?;
 let before=s.clone();assert!(s.post(command("backdate-overflow","2026-02-01",1,false)).is_err());assert_eq!(s,before);
 assert_eq!(s.trial_balance(&c,"2026-12-31")?.total_debit.rials(),i64::MAX-1);
 println!("PASS independent cumulative MAX+1 and backdated overflowing earlier cutoff both rejected atomically; valid later reduction reports MAX-1.");
 Ok(())
}
