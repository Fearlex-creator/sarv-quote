use sarv_quote::{calendar::*,accounting::*,domain::*,imports::*,store::Store,export,accounting_export::*};
use chrono::NaiveDate;
use std::{path::Path,fs};
fn main()->Result<(),Box<dyn std::error::Error>>{
 let args:Vec<_>=std::env::args().collect();let root=Path::new(&args[1]);let expected=fs::read_to_string(&args[2])?;
 let mut count=0;let mut last:Option<NaiveDate>=None;
 for row in expected.lines(){let (iso,jalali)=row.split_once(',').unwrap();let d=NaiveDate::parse_from_str(iso,"%Y-%m-%d")?;
  assert_eq!(parse_date(jalali,Calendar::Jalali)?,d,"ICU mismatch {row}");assert_eq!(format_date(d,Calendar::Jalali)?,jalali);assert_eq!(to_gregorian(from_gregorian(d)?),d);
  if let Some(previous)=last {assert_eq!((d-previous).num_days(),1);}last=Some(d);count+=1;
 }
 assert_eq!(count,109573);
 for yr in 1200..=1499{let (start,end)=jalali_fiscal_year(yr)?;assert_eq!(from_gregorian(start)?.to_string(),format!("{yr}/01/01"));assert_eq!(from_gregorian(end)?.year(),yr);assert_eq!(from_gregorian(end)?.month(),12);assert_eq!(from_gregorian(end)?.day(),if is_jalali_leap_year(yr)?{30}else{29});}
 for s in ["1199/12/29","1500/01/01","1404/12/30","1405/12/30","1403/00/01","1403/13/01","1403/01/00","1403/01/32","1403/07/31","1403/1/01","1403/01/1","1403/01-01","1403/01/01junk"," 1403/01/01","1403/01/01 ","+1403/01/01"]{assert!(parse_date(s,Calendar::Jalali).is_err(),"accepted {s}");}
 for s in ["0000-01-01","2025-02-29","1900-02-29","2026-13-01","2026-02-00","2026-04-31"]{assert!(parse_date(s,Calendar::Gregorian).is_err());}
 assert_eq!(parse_date("۱۴۰۳/۱۲/۳۰",Calendar::Jalali)?,parse_date("١٤٠٣/١٢/٣٠",Calendar::Jalali)?);
 assert_eq!(parse_date("۱۴۰۳/۱۲/۳۰",Calendar::Jalali)?.to_string(),"2025-03-20");
 for iso in ["1821-03-20","2121-03-21"]{assert!(from_gregorian(NaiveDate::parse_from_str(iso,"%Y-%m-%d")?).is_err());}
 println!("PASS calendar: all {count} supported days agree with external ICU78.3, both conversions; 300 fiscal-year bounds, strict invalids/digits/endpoints");
 fs::create_dir_all(root)?;let profile=root.join("profile");assert!(!profile.exists(),"fresh profile required");let mut store=Store::open(&profile)?;
 let (c,p,a,e)=store.change(|s|{let c=s.accounting.add_company("شرکت بازبین M2 TEST-1403","independent-m2")?;let (start,end)=jalali_fiscal_year(1403)?;let p=s.accounting.add_period(&c,"سال ۱۴۰۳",&start.to_string(),&end.to_string(),"independent-m2")?;
 let a=s.accounting.add_account(NewAccount{company_id:c.clone(),parent_id:None,code:"M2-1001".into(),name:"دارایی آزمایشی".into(),classification:AccountClass::Asset,normal_balance:BalanceSide::Debit,postable:true,control:false},"independent-m2")?;
 let e=s.accounting.add_account(NewAccount{company_id:c.clone(),parent_id:None,code:"M2-3001".into(),name:"سرمایه آزمایشی".into(),classification:AccountClass::Equity,normal_balance:BalanceSide::Credit,postable:true,control:false},"independent-m2")?;Ok((c,p,a,e))})?;
 let cmd=|key:&str,j:&str,value:i64|PostCommand{company_id:c.clone(),period_id:p.clone(),date:parse_date(j,Calendar::Jalali).unwrap().to_string(),source_id:key.into(),source_revision:1,event:"manual".into(),request_key:key.into(),actor:"independent-m2".into(),reason:"synthetic calendar fixture, no accounting policy claim".into(),kind:JournalKind::Manual,lines:vec![JournalLineInput{account_id:a.clone(),debit:Money::from_rials(value),credit:Money::ZERO,description:"fixture".into()},JournalLineInput{account_id:e.clone(),debit:Money::ZERO,credit:Money::from_rials(value),description:"fixture".into()}]};
 let first=cmd("before-year-end","1403/12/29",1251);let fid=store.change(|s|s.accounting.post(first.clone()))?;
 let second=cmd("leap-end","1403/12/30",349);store.change(|s|s.accounting.post(second))?;
 assert_eq!(store.state.accounting.trial_balance(&c,"2025-03-19")?.total_debit.rials(),1251);assert_eq!(store.state.accounting.trial_balance(&c,"2025-03-20")?.total_debit.rials(),1600);
 let before=serde_json::to_value(&store.state)?;assert!(store.change(|s|s.accounting.post(cmd("next-year","1404/01/01",1))).is_err());assert_eq!(before,serde_json::to_value(&store.state)?);
 let (start1499,end1499)=jalali_fiscal_year(1499)?;assert_eq!(end1499.to_string(),"2121-03-20");let period1499=store.change(|s|s.accounting.add_period(&c,"1499",&start1499.to_string(),&end1499.to_string(),"independent-m2"))?;let mut lastcmd=cmd("1499-end","1499/12/29",1);lastcmd.period_id=period1499.clone();store.change(|s|s.accounting.post(lastcmd))?;let mut aftercmd=cmd("1500-first","1499/12/29",1);aftercmd.period_id=period1499;aftercmd.date="2121-03-21".into();let before1499=serde_json::to_value(&store.state)?;assert!(store.change(|s|s.accounting.post(aftercmd)).is_err());assert_eq!(before1499,serde_json::to_value(&store.state)?);
 let ledger=serde_json::to_value(&store.state.accounting)?;
 store.change(|s|{accept_catalog(s,catalog_csv("code,name,measure,price,currency,source_at,aliases\nM2-CODE,نمونه آمیخته,عدد,1251,rial,2024-01-01T00:00:00Z,\n","بازبین")?)?;s.draft.customer="مشتری بازبین".into();s.draft.lines=order_csv("item,quantity\nM2-CODE,1\n")?;s.draft.quote_date="2025-03-20".into();s.draft.valid_until="2025-03-21".into();s.draft.stale_ack=true;s.approve()?;Ok(())})?;
 let old=serde_json::to_value(&store.state.quotes)?;let old_csv=export::csv_bytes(&store.state.quotes[0])?;store.change(|s|{s.settings.date_display=DateDisplay::Jalali;Ok(())})?;assert_eq!(old,serde_json::to_value(&store.state.quotes)?);assert_eq!(old_csv,export::csv_bytes(&store.state.quotes[0])?);assert_eq!(ledger,serde_json::to_value(&store.state.accounting)?);
 assert_eq!(store.state.quotes[0].date_display,DateDisplay::Iso);assert_eq!(DateDisplay::Jalali.format("2025-03-20"),"1403/12/30");
 let mut retry=first;retry.request_key="after-pref-toggle".into();assert_eq!(store.change(|s|s.accounting.post(retry))?,fid);let after_retry=serde_json::to_value(&store.state.accounting)?;for key in ["companies","periods","accounts","journals","audit"]{assert_eq!(ledger[key],after_retry[key]);}assert_eq!(after_retry["requests"].as_array().unwrap().len(),ledger["requests"].as_array().unwrap().len()+1);
 let old_id=store.state.quotes[0].id.clone();store.change(|s|{s.revise(&old_id)?;assert_eq!(s.draft.date_display,DateDisplay::Iso);s.draft.notes="اصلاح بدون تغییر تقویم".into();s.draft.stale_ack=true;s.approve()?;s.new_draft()?;s.draft.customer="مشتری شمسی".into();s.draft.lines=order_csv("item,quantity\nM2-CODE,1\n")?;s.draft.quote_date="2025-03-20".into();s.draft.valid_until="2025-03-21".into();s.draft.stale_ack=true;assert_eq!(s.draft.date_display,DateDisplay::Jalali);s.approve()?;Ok(())})?;
 assert_eq!(store.state.quotes[0].date_display,DateDisplay::Iso);assert_eq!(store.state.quotes[1].date_display,DateDisplay::Iso);assert_eq!(store.state.quotes[2].date_display,DateDisplay::Jalali);assert_eq!(old_csv,export::csv_bytes(&store.state.quotes[0])?);let jq=&store.state.quotes[2];assert_eq!(jq.quote_date,"2025-03-20");assert_eq!(jq.valid_until,"2025-03-21");export::csv(jq,&root.join("new-jalali.csv"))?;export::pdf(jq,&root.join("new-jalali.pdf"))?;
 let balance=store.state.accounting.trial_balance(&c,"2025-03-20")?;trial_csv_with_dates("شرکت بازبین M2",&balance,MoneyUnit::Toman,DateDisplay::Jalali,&root.join("leap-trial.csv"))?;trial_pdf_with_dates("شرکت بازبین M2",&balance,MoneyUnit::Toman,DateDisplay::Jalali,&root.join("leap-trial.pdf"))?;
 store.backup(&root.join("backup.json"))?;let expected=serde_json::to_value(&store.state)?;drop(store);let reopened=Store::open(&profile)?;assert_eq!(expected,serde_json::to_value(&reopened.state)?);assert_eq!(reopened.state.settings.date_display,DateDisplay::Jalali);
 let mut restored=Store::open(&root.join("restored"))?;restored.restore(Store::restore_preview(&root.join("backup.json"))?)?;assert_eq!(expected,serde_json::to_value(&restored.state)?);
 export::csv(&reopened.state.quotes[0],&root.join("old-iso.csv"))?;export::pdf(&reopened.state.quotes[0],&root.join("old-iso.pdf"))?;
 println!("PASS leap-year raw periods/journals/cutoffs exact; next-year posting rejected unchanged; pref toggle/retry leave ledger and approved ISO quote/CSV exactly unchanged; restart+fresh-profile restore exact");Ok(())
}
