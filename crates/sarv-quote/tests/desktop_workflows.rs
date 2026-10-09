use sarv_quote::{service::AppService,desktop::DesktopHost};
use serde_json::{Value,json};
fn new_service(dir:&std::path::Path)->(AppService,Value){let svc=AppService::open(dir).unwrap();let w=svc.desktop_mutate("onboard",json!({"path":"new","profile":"trading","name":"فروشگاه آزمایشی","start":"1405/01/01","end":"1405/12/29","language":"fa"})).unwrap();(svc,w)}
fn create(s:&AppService,action:&str,mut p:Value)->Value{p["date"]=json!("1405/01/01");s.desktop_mutate(action,p).unwrap()}
fn rows(s:&AppService,module:&str)->Value{s.desktop_read("rows",json!({"module":module})).unwrap()}
#[test]
fn original_store_restores_goods_and_exact_shared_ledger(){
 let dir=tempfile::tempdir().unwrap();let(s,w)=new_service(dir.path());assert_eq!(w["language"],"fa");assert_eq!(w["calendar"],"Jalali");assert_eq!(w["counts"]["journals"],0);
 let w=create(&s,"contact",json!({"name":"فروشنده","customer":true,"supplier":true,"phone":"۰۹۱۲۳۴۵۶۷۸۹"}));let party=w["resources"]["parties"][0]["id"].as_str().unwrap().to_string();let unit=w["resources"]["units"][0]["id"].as_str().unwrap();let warehouse=w["resources"]["warehouses"][0]["id"].as_str().unwrap().to_string();
 let w=create(&s,"product",json!({"name":"دفتر","code":"NOTE","unitId":unit,"price":"20.1","unit":"Toman"}));let item=w["resources"]["items"][0]["id"].as_str().unwrap().to_string();
 let buy=json!({"date":"1405/01/01","due":"1405/01/10","partyId":party,"reference":"BUY-1","requestKey":"buy-exact","lines":[{"itemId":item,"warehouseId":warehouse,"quantity":"10","price":"10.1","unit":"Toman"}]});
 let preview=s.desktop_read("preview",json!({"operation":"buy","input":buy})).unwrap();assert_eq!(preview["journals"].as_array().unwrap().len(),2);assert_eq!(rows(&s,"purchases")["total"],0);
 s.desktop_mutate("buy",buy.clone()).unwrap();s.desktop_mutate("buy",buy).unwrap();assert_eq!(rows(&s,"inventory")["rows"][0]["amount"],"1010");
 let sell=json!({"date":"1405/01/01","due":"1405/01/10","partyId":party,"reference":"SELL-1","requestKey":"sell-exact","lines":[{"itemId":item,"warehouseId":warehouse,"quantity":"3","price":"20.1","unit":"Toman"}]});s.desktop_mutate("sell",sell).unwrap();
 assert_eq!(rows(&s,"inventory")["rows"][0]["amount"],"707");let sales=rows(&s,"sales");let sale=sales["rows"].as_array().unwrap().iter().find(|r|r["kind"]=="sale").unwrap();let id=sale["id"].as_str().unwrap();let d=s.desktop_read("detail",json!({"kind":"sale","id":id})).unwrap();let line=d["returnable"][0]["id"].as_str().unwrap();
 create(&s,"salesReturn",json!({"id":id,"requestKey":"return-1","reference":"RET-1","lines":[{"lineId":line,"quantity":"1"}]}));assert_eq!(rows(&s,"inventory")["rows"][0]["amount"],"808");
 let contacts=rows(&s,"contacts");assert_eq!(contacts["rows"][0]["amount"],"402");assert_eq!(contacts["rows"][0]["payable"],"1010");
 s.with_store(|store|{store.state.validate()?;let t=store.state.accounting.trial_balance(&store.state.desktop.active_company,"2026-10-10")?;assert_eq!(t.total_debit,t.total_credit);Ok(())}).unwrap();
 drop(s);let reopened=AppService::open(dir.path()).unwrap();assert_eq!(rows(&reopened,"inventory")["rows"][0]["amount"],"808");assert!(dir.path().join("sarv.sqlite3").exists());assert!(!dir.path().join("accounting.sqlite3").exists());
}
#[test]
fn wallet_lock_reversal_and_audit_are_atomic(){let dir=tempfile::tempdir().unwrap();let(s,w)=new_service(dir.path());let account=w["accounts"].as_array().unwrap().iter().find(|a|a["wallet"]==true&&a["control"]==false).unwrap()["id"].as_str().unwrap();
 let p=json!({"date":"1405/01/01","kind":"income","accountId":account,"amount":"۱۲۳٫۴","unit":"Toman","requestKey":"income-1","memo":"حقوق"});s.desktop_mutate("quick",p).unwrap();assert_eq!(s.desktop_read("workspace",json!({})).unwrap()["cash"],"1234");let ledger=rows(&s,"ledger");let id=ledger["rows"][0]["id"].as_str().unwrap();
 create(&s,"reverse",json!({"id":id,"requestKey":"reverse-1","memo":"اصلاح مبلغ"}));assert_eq!(s.desktop_read("workspace",json!({})).unwrap()["cash"],"0");let period=w["periods"][0]["id"].as_str().unwrap();create(&s,"lock",json!({"id":period,"locked":true,"memo":"بستن دوره"}));let before=s.snapshot().unwrap().generation;assert!(s.desktop_mutate("quick",json!({"date":"1405/01/01","kind":"expense","accountId":account,"amount":"1","unit":"Rial","requestKey":"blocked"})).is_err());assert_eq!(s.snapshot().unwrap().generation,before);
}
#[test]
fn workspace_switch_keeps_original_books(){let dir=tempfile::tempdir().unwrap();let host=DesktopHost::open(dir.path()).unwrap();host.mutate("onboard",json!({"path":"new","profile":"individual","name":"Original","start":"1405/01/01","end":"1405/12/29"})).unwrap();let new=host.mutate("newWorkspace",json!({})).unwrap();assert_eq!(new["existing"],false);host.mutate("onboard",json!({"path":"new","profile":"trading","name":"Second","start":"1405/01/01","end":"1405/12/29"})).unwrap();assert_eq!(DesktopHost::open(dir.path()).unwrap().read("workspace",json!({})).unwrap()["name"],"Second");assert_eq!(host.mutate("originalWorkspace",json!({})).unwrap()["name"],"Original");}
