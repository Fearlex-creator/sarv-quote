//! Desktop adapter over the original Sarv store. No second books or floating-point money.
use crate::{accounting::*, core::*, domain::{self, State, Unit}, procurement::*, sales::*, service::AppService};
use anyhow::{Result, ensure, bail};
use chrono::{Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all="camelCase")]
pub struct DesktopPreferences {
    pub completed: bool, pub active_company: String, pub profile: String, pub language: String,
    pub calendar: String, pub theme: String, pub audit: Vec<DesktopAudit>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all="camelCase")]
pub struct DesktopAudit { pub id: String, pub at: String, pub operation: String, pub detail: String }
impl DesktopPreferences {
    pub fn is_empty(&self)->bool { self == &Self::default() }
    pub fn validate(&self)->Result<()> {
        ensure!(["", "fa", "en"].contains(&self.language.as_str()), "زبان نامعتبر");
        ensure!(["", "Jalali", "Gregorian"].contains(&self.calendar.as_str()), "تقویم نامعتبر");
        ensure!(["", "light", "dark", "system"].contains(&self.theme.as_str()), "ظاهر نامعتبر");
        ensure!(self.audit.len() <= 100_000, "سابقه بسیار بزرگ است"); Ok(())
    }
    pub fn validate_transition(&self,next:&Self)->Result<()> {ensure!(next.audit.starts_with(&self.audit),"سابقه برنامه قابل حذف یا ویرایش نیست");Ok(())}
}
fn string<'a>(p:&'a Value,k:&str)->Result<&'a str> {p[k].as_str().ok_or_else(||anyhow::anyhow!("فیلد {k} لازم است"))}
fn optional<'a>(p:&'a Value,k:&str)->&'a str {p[k].as_str().unwrap_or("")}
fn money(p:&Value,k:&str)->Result<Money>{Money::parse(string(p,k)?,if optional(p,"unit")=="Rial" {MoneyUnit::Rial}else{MoneyUnit::Toman})}
fn calendar(s:&State)->crate::calendar::Calendar {if s.desktop.calendar=="Gregorian" {crate::calendar::Calendar::Gregorian}else{crate::calendar::Calendar::Jalali}}
fn date(s:&State,p:&Value,k:&str)->Result<String>{Ok(crate::calendar::parse_date(string(p,k)?,calendar(s))?.format("%Y-%m-%d").to_string())}
fn active(s:&State)->Result<String>{s.accounting.companies.iter().find(|c|c.id==s.desktop.active_company).or(s.accounting.companies.first()).map(|c|c.id.clone()).ok_or_else(||anyhow::anyhow!("ابتدا فضای کاری بسازید"))}
fn context(s:&State,p:&Value,key:&str)->Result<ProcurementContext>{
    let company=active(s)?;let d=date(s,p,"date")?;
    let setup=s.accounting.core.setups.iter().rev().find(|x|x.command.company_id==company).ok_or_else(||anyhow::anyhow!("تنظیمات حسابداری این شرکت کامل نیست"))?;
    let period=s.accounting.periods.iter().find(|x|x.company_id==company && d>=x.start_date && d<=x.end_date).ok_or_else(||anyhow::anyhow!("تاریخ خارج از سال مالی است"))?;
    Ok(ProcurementContext{company_id:company,period_id:period.id.clone(),setup_id:setup.id.clone(),source_id:key.into(),source_revision:1,date:d,request_key:key.into(),actor:"desktop-owner".into(),reason:optional(p,"memo").trim().to_string().replace('\0',"").pipe_nonempty()})
}
trait Nonempty {fn pipe_nonempty(self)->String;}
impl Nonempty for String {fn pipe_nonempty(self)->String{if self.is_empty(){"ثبت از میز کار سرو".into()}else{self}}}
fn audit(s:&mut State,op:&str,detail:&str){s.desktop.audit.push(DesktopAudit{id:domain::id(),at:domain::now(),operation:op.into(),detail:detail.into()});}
fn resources(s:&State)->Result<RegisterResources>{s.accounting.procurement_resources(&active(s)?)}
fn json_exact<T:Serialize>(v:&T)->Result<Value>{
    fn convert(v:&mut Value){match v {Value::Number(n) if n.is_i64() || n.is_u64()=>*v=Value::String(n.to_string()),Value::Array(a)=>a.iter_mut().for_each(convert),Value::Object(o)=>o.values_mut().for_each(convert),_=>{}}}
    let mut v=serde_json::to_value(v)?;convert(&mut v);Ok(v)
}
fn opening(s:&State,r:&CoreCompanyResult)->Result<OpeningCommand>{
    let setup=s.accounting.core.setups.iter().find(|x|x.id==r.setup_id).unwrap();
    let period=s.accounting.periods.iter().find(|x|x.id==r.period_id).unwrap();
    Ok(OpeningCommand{company_id:r.company_id.clone(),period_id:r.period_id.clone(),setup_id:r.setup_id.clone(),as_of:period.start_date.clone(),source_id:domain::id(),request_key:domain::id(),actor:"desktop-owner".into(),reason:"شروع فضای کاری؛ مانده‌های کنترلی صفر".into(),balances:CoreRole::ALL.into_iter().filter(|r|r.is_control()).map(|r|Ok(OpeningBalance{account_id:setup.account_id(r)?.into(),debit:Money::ZERO,credit:Money::ZERO,source_reference:format!("zero-{}",r.key())})).collect::<Result<_>>()?,parties:vec![],units:vec![],items:vec![],warehouses:vec![],cash:vec![],banks:vec![],open_items:vec![],advances:vec![],cheques:vec![],stock:vec![],grni:vec![],import_source:None})
}
fn workspace(s:&State,generation:i64)->Result<Value>{
    let today=local_today().format("%Y-%m-%d").to_string();
    let company=active(s).unwrap_or_default();
    let res=if company.is_empty(){None}else{s.accounting.procurement_resources(&company).ok()};
    let mut accounts=vec![];let(mut cash,mut income,mut expense)=(Money::ZERO,Money::ZERO,Money::ZERO);
    let mut monthly=std::collections::BTreeMap::<String,(Money,Money)>::new();
    for a in s.accounting.accounts.iter().filter(|a|a.company_id==company && a.postable){
        let(mut debit,mut credit)=(Money::ZERO,Money::ZERO);
        for j in s.accounting.journals.iter().filter(|j|j.company_id==company){for l in j.lines.iter().filter(|l|l.account.id==a.id){debit=debit.checked_add(l.debit)?;credit=credit.checked_add(l.credit)?;
            if matches!(a.classification,AccountClass::Revenue|AccountClass::Expense) {let m=monthly.entry(j.date[..7].into()).or_default();if a.classification==AccountClass::Revenue{m.0=m.0.checked_add(l.credit.checked_sub(l.debit)?)?;}else{m.1=m.1.checked_add(l.debit.checked_sub(l.credit)?)?;}}
        }}
        let balance=if a.normal_balance==BalanceSide::Debit {debit.checked_sub(credit)?}else{credit.checked_sub(debit)?};
        if a.classification==AccountClass::Revenue{income=income.checked_add(balance)?;}if a.classification==AccountClass::Expense{expense=expense.checked_add(balance)?;}
        let is_cash=s.accounting.core.setups.iter().filter(|x|x.command.company_id==company).flat_map(|x|&x.command.mappings).any(|m|m.account_id==a.id && matches!(m.role,CoreRole::Cash|CoreRole::Bank)) || s.desktop.audit.iter().any(|x|x.operation=="wallet" && x.detail==a.id);
        if is_cash{cash=cash.checked_add(balance)?;}
        accounts.push(json!({"id":a.id,"code":a.code,"name":a.name,"class":format!("{:?}",a.classification),"control":a.control,"archived":a.archived,"wallet":is_cash,"balance":balance.rials().to_string()}));
    }
    let plot_max=monthly.values().flat_map(|(i,e)|[i.rials().unsigned_abs(),e.rials().unsigned_abs()]).max().unwrap_or(1).max(1);
    let start=NaiveDate::from_ymd_opt(local_today().year(),3,21).unwrap();
    Ok(json!({"generation":generation,"ready":s.desktop.completed,"existing":!s.accounting.is_empty()||!s.catalogs.is_empty()||!s.quotes.is_empty(),"profile":if s.desktop.profile.is_empty(){"trading"}else{&s.desktop.profile},"language":if s.desktop.language.is_empty(){"fa"}else{&s.desktop.language},"calendar":if s.desktop.calendar.is_empty(){"Jalali"}else{&s.desktop.calendar},"theme":if s.desktop.theme.is_empty(){"system"}else{&s.desktop.theme},"unit":format!("{:?}",s.settings.default_currency),"companyId":company,"companies":s.accounting.companies,"name":s.accounting.companies.iter().find(|c|c.id==company).map(|c|c.name.as_str()).unwrap_or(&s.settings.business_name),"today":crate::calendar::format_date(local_today(),calendar(s))?,"fiscalStart":crate::calendar::format_date(start,calendar(s))?,"fiscalEnd":crate::calendar::format_date(NaiveDate::from_ymd_opt(start.year()+1,3,20).unwrap(),calendar(s))?,"accounts":accounts,"resources":res.as_ref().map(json_exact).transpose()?,"cash":cash.rials().to_string(),"income":income.rials().to_string(),"expense":expense.rials().to_string(),"profit":income.checked_sub(expense)?.rials().to_string(),"trend":monthly.iter().map(|(m,(i,e))|json!({"month":m,"income":i.rials().to_string(),"expense":e.rials().to_string(),"incomePlot":(i128::from(i.rials())*10000/i128::from(plot_max)) as i64,"expensePlot":(i128::from(e.rials())*10000/i128::from(plot_max)) as i64})).collect::<Vec<_>>(),"periods":s.accounting.periods.iter().filter(|p|p.company_id==company).collect::<Vec<_>>(),"settings":s.settings,"counts":{"products":res.as_ref().map(|r|r.items.len()).unwrap_or(0),"contacts":res.as_ref().map(|r|r.parties.len()).unwrap_or(s.customers.len()),"quotes":s.quotes.len(),"journals":s.accounting.journals.iter().filter(|j|j.company_id==company).count()},"asOf":today}))
}

impl AppService {
    pub fn desktop_read(&self,action:&str,p:Value)->Result<Value>{self.with_store(|store|{let s=&store.state;match action {
        "workspace"=>workspace(s,store.local_generation()),
        "importPreview"=>{let candidate=import_candidate(&p)?;Ok(json!({"name":candidate.settings.business_name,"companies":candidate.accounting.companies,"products":candidate.catalogs.iter().map(|c|c.products.len()).sum::<usize>(),"journals":candidate.accounting.journals.len(),"quotes":candidate.quotes.len(),"fingerprint":import_hash(&p)?}))},
        "preview"=>{let mut copy=s.clone();execute(&mut copy,string(&p,"operation")?,&p["input"])?;Ok(json!({"journals":json_exact(&copy.accounting.journals[s.accounting.journals.len()..].to_vec())?,"sales":json_exact(&copy.accounting.sales.records[s.accounting.sales.records.len()..].to_vec())?,"purchases":json_exact(&copy.accounting.procurement.records[s.accounting.procurement.records.len()..].to_vec())?}))},
        "rows"=>rows(s,&p),
        "detail"=>detail(s,&p),
        "export"=>export(s,&p),
        _=>bail!("فرمان خواندن ناشناخته")
    }})}
    pub fn desktop_mutate(&self,action:&str,p:Value)->Result<Value>{self.with_store(|store|{
        if action=="import" {
            ensure!(import_hash(&p)?==string(&p,"fingerprint")?,"فایل پس از بازبینی تغییر کرده است");
            // Import is permitted only into an empty NEW store. Existing books cannot be replaced.
            ensure!(store.state.accounting.is_empty()&&store.state.catalogs.is_empty()&&store.state.quotes.is_empty(),"برای ورود داده یک فضای کاری خالی و مستقل انتخاب کنید؛ داده جاری جایگزین نمی‌شود");
            let mut candidate=import_candidate(&p)?;candidate.schema=domain::CURRENT_SCHEMA;candidate.desktop.completed=false;
            store.restore(candidate)?;
        }else {store.change(|s|{execute(s,action,&p)?;audit(s,action,optional(&p,"memo"));Ok(())})?;}
        workspace(&store.state,store.local_generation())
    })}
}
fn import_hash(p:&Value)->Result<String>{Ok(format!("{:x}",Sha256::digest(string(p,"content")?.as_bytes())))}
fn import_candidate(p:&Value)->Result<State>{let content=string(p,"content")?;ensure!(content.len()<=64*1024*1024,"فایل بیش از ۶۴ مگابایت است");let v:Value=serde_json::from_str(content)?;let s:State=serde_json::from_value(v.get("state").unwrap_or(&v).clone())?;s.validate()?;
 if v.get("state").is_some(){let expected=match s.schema{1=>"sarv-quote-backup-v1",2=>"sarv-accounting-backup-v2",3=>"sarv-accounting-backup-v3",4=>"sarv-accounting-backup-v4",5=>"sarv-accounting-backup-v5",6=>"sarv-accounting-backup-v6",_=>"sarv-accounting-backup-v7"};ensure!(v["format"].as_str()==Some(expected),"قالب پشتیبان نامعتبر");ensure!(v["checksum"].as_str()==Some(&crate::imports::hash(&serde_json::to_vec(&s)?)),"اثر انگشت پشتیبان تطبیق ندارد");}Ok(s)}
fn execute(s:&mut State,action:&str,p:&Value)->Result<()> {match action {
    "onboard"=>{
        let path=string(p,"path")?;let profile=string(p,"profile")?;ensure!(["individual","service","trading"].contains(&profile),"نوع فضای کاری نامعتبر");
        if path=="new"{
            let r=s.accounting.setup_core_company(NewCoreCompany{name:string(p,"name")?.into(),fiscal_name:"سال مالی".into(),start_date:date(s,p,"start")?,end_date:date(s,p,"end")?,timezone:CompanyTimezone::AsiaTehran,chart:default_chart(),actor:"desktop-owner".into()})?;
            let o=opening(s,&r)?;s.accounting.post_opening(o)?;s.desktop.active_company=r.company_id;
            let company=active(s)?;let ctx=context(s,&json!({"date":string(p,"start")?,"memo":"واحد و انبار اولیه"}),&domain::id())?;
            s.accounting.post_procurement_command(ProcurementCommand::Resources(RegisterResources{context:ctx,parties:vec![],units:vec![CoreUnit{id:domain::id(),company_id:company.clone(),name:"عدد".into()}],items:vec![],warehouses:vec![CoreWarehouse{id:domain::id(),company_id:company,name:"انبار اصلی".into()}]}))?;
            add_wallet(s,&json!({"name":"کیف پول","code":format!("W{}",s.accounting.accounts.len()),"amount":"0","date":string(p,"start")?}))?;
        }else{ensure!(path=="existing","مسیر نامعتبر");let c=string(p,"companyId")?;ensure!(s.accounting.companies.iter().any(|x|x.id==c),"شرکت موجود را انتخاب کنید");s.desktop.active_company=c.into();}
        s.desktop.profile=profile.into();s.desktop.language=optional(p,"language").pipe_string("fa");s.desktop.completed=true;
    },
    "switch"=>{let c=string(p,"companyId")?;ensure!(s.accounting.companies.iter().any(|x|x.id==c),"فضای کاری نامعتبر");s.desktop.active_company=c.into();},
    "settings"=>{
        s.desktop.language=string(p,"language")?.into();s.desktop.calendar=string(p,"calendar")?.into();s.desktop.theme=string(p,"theme")?.into();s.desktop.profile=string(p,"profile")?.into();
        ensure!(["individual","service","trading"].contains(&s.desktop.profile.as_str()),"نوع فضای کاری نامعتبر");
        s.settings.default_currency=if string(p,"unit")?=="Rial"{Unit::Rial}else{Unit::Toman};
        s.settings.business_name=string(p,"name")?.into();s.settings.business_phone=optional(p,"phone").into();s.settings.business_address=optional(p,"address").into();s.settings.validate()?;
    },
    "contact"=>{let company=active(s)?;let name=string(p,"name")?;domain::text(name)?;let customer=p["customer"].as_bool().unwrap_or(true);let supplier=p["supplier"].as_bool().unwrap_or(false);ensure!(customer||supplier,"نقش مخاطب را انتخاب کنید");
        let id=s.save_customer_details(name,optional(p,"phone"),"",optional(p,"email"),optional(p,"address"))?;
        let ctx=context(s,p,&domain::id())?;s.accounting.post_procurement_command(ProcurementCommand::Resources(RegisterResources{context:ctx,parties:vec![CoreParty{id,company_id:company,name:name.into(),customer,supplier}],units:vec![],items:vec![],warehouses:vec![]}))?;
    },
    "product"=>{let company=active(s)?;let r=resources(s)?;let sku=string(p,"code")?;let name=string(p,"name")?;domain::text(sku)?;domain::text(name)?;
        ensure!(!r.items.iter().any(|i|domain::normalize(&i.sku)==domain::normalize(sku)),"کد کالا تکراری است");let unit_id=string(p,"unitId")?;
        let price=money(p,"price")?;ensure!(price.rials()>0,"قیمت باید مثبت باشد");
        s.accounting.post_procurement_command(ProcurementCommand::Resources(RegisterResources{context:context(s,p,&domain::id())?,parties:vec![],units:vec![],items:vec![CoreItem{id:domain::id(),company_id:company.clone(),sku:sku.into(),name:name.into(),base_unit_id:unit_id.into(),indivisible:p["indivisible"].as_bool().unwrap_or(false),packs:vec![]}],warehouses:vec![]}))?;
        // Price lists are versioned; historical catalogs and quotations are never edited.
        let mut products=s.catalogs.iter().rev().find(|c|c.name==format!("Desktop {company}")).map(|c|c.products.clone()).unwrap_or_default();
        products.push(domain::Product{code:sku.into(),name:name.into(),measure:r.units.iter().find(|u|u.id==unit_id).map(|u|u.name.clone()).unwrap_or_default(),price_rial:price.rials(),source_unit:s.settings.default_currency,source_at:domain::now(),aliases:vec![]});
        s.catalogs.push(domain::Catalog{id:domain::id(),name:format!("Desktop {company}"),imported_at:domain::now(),fingerprint:crate::imports::product_fingerprint(&products)?,products});
    },
    "warehouse"=>{let company=active(s)?;s.accounting.post_procurement_command(ProcurementCommand::Resources(RegisterResources{context:context(s,p,&domain::id())?,parties:vec![],units:vec![],items:vec![],warehouses:vec![CoreWarehouse{id:domain::id(),company_id:company,name:string(p,"name")?.into()}]}))?;},
    "buy"|"sell"=>trade(s,p,action=="sell")?,
    "purchaseReturn"|"salesReturn"=>return_document(s,p,action=="salesReturn")?,
    "transferStock"=>{let r=resources(s)?;let item=r.items.iter().find(|i|i.id==optional(p,"itemId")).ok_or_else(||anyhow::anyhow!("کالا را انتخاب کنید"))?;s.accounting.post_procurement_command(ProcurementCommand::Transfer(TransferStock{context:context(s,p,string(p,"requestKey")?)?,item_id:item.id.clone(),source_warehouse_id:string(p,"sourceId")?.into(),target_warehouse_id:string(p,"targetId")?.into(),unit_id:item.base_unit_id.clone(),quantity_thousandths:domain::quantity(string(p,"quantity")?)?}))?;},
    "quote"=>{let r=resources(s)?;s.new_draft()?;let party=r.parties.iter().find(|x|x.id==optional(p,"partyId")).ok_or_else(||anyhow::anyhow!("مشتری را انتخاب کنید"))?;
        let catalog=s.catalogs.iter().rev().find(|c|c.name==format!("Desktop {}",active(s).unwrap())).or(s.catalogs.last()).ok_or_else(||anyhow::anyhow!("ابتدا فهرست کالا بسازید"))?;
        s.draft.catalog_id=catalog.id.clone();s.draft.customer=party.name.clone();s.draft.phone=s.customers.iter().find(|c|c.id==party.id).map(|c|c.phone.clone()).unwrap_or_default();s.draft.quote_date=date(s,p,"date")?;s.draft.stale_ack=p["acknowledgeStale"].as_bool().unwrap_or(false);s.draft.reference=string(p,"reference")?.into();s.draft.notes=optional(p,"memo").into();
        s.draft.lines=p["lines"].as_array().ok_or_else(||anyhow::anyhow!("ردیف کالا لازم است"))?.iter().map(|l|{let i=r.items.iter().find(|i|i.id==optional(l,"itemId")).ok_or_else(||anyhow::anyhow!("کالا یافت نشد"))?;Ok(domain::OrderLine{query:i.sku.clone(),quantity_milli:domain::quantity(string(l,"quantity")?)?,chosen_code:Some(i.sku.clone())})}).collect::<Result<_>>()?;s.approve()?;
    },
    "wallet"=>{add_wallet(s,p)?;},
    "quick"=>{quick(s,p)?;},
    "reverse"=>{let journal=string(p,"id")?;s.accounting.reverse(journal,&context(s,p,string(p,"requestKey")?)?.period_id,&date(s,p,"date")?,string(p,"requestKey")?,"desktop-owner",string(p,"memo")?)?;},
    "archive"=>s.accounting.archive_account(string(p,"id")?,"desktop-owner",string(p,"memo")?)?,
    "lock"=>s.accounting.set_period_lock(string(p,"id")?,p["locked"].as_bool().unwrap_or(true),"desktop-owner",string(p,"memo")?)?,
    _=>bail!("فرمان ثبت ناشناخته: {action}")
}Ok(())}
trait DefaultString{fn pipe_string(self,fallback:&str)->String;}impl DefaultString for &str{fn pipe_string(self,f:&str)->String{if self.is_empty(){f.into()}else{self.into()}}}
fn add_wallet(s:&mut State,p:&Value)->Result<()> {
    let c=active(s)?;let id=s.accounting.add_account(NewAccount{company_id:c,parent_id:None,code:string(p,"code")?.into(),name:string(p,"name")?.into(),classification:AccountClass::Asset,normal_balance:BalanceSide::Debit,postable:true,control:false},"desktop-owner")?;audit(s,"wallet",&id);
    if money(p,"amount")?.rials()>0 {let mut input=p.clone();input["accountId"]=json!(id);input["kind"]=json!("opening");input["requestKey"]=json!(domain::id());quick(s,&input)?;}Ok(())
}
fn quick(s:&mut State,p:&Value)->Result<()> {
    let ctx=context(s,p,string(p,"requestKey")?)?;let setup=s.accounting.core.setups.iter().find(|x|x.id==ctx.setup_id).unwrap();let a=string(p,"accountId")?;let kind=string(p,"kind")?;let amount=money(p,"amount")?;ensure!(amount.rials()>0,"مبلغ باید مثبت باشد");
    ensure!(s.accounting.accounts.iter().any(|x|x.id==a&&x.company_id==ctx.company_id&&x.classification==AccountClass::Asset&&!x.control&&!x.archived),"کیف پول عادی را انتخاب کنید؛ حساب‌های کنترلی فقط از زیرسیستم ثبت می‌شوند");
    let counter=if kind=="transfer"{string(p,"targetId")?}else{setup.account_id(match kind{"expense"=>CoreRole::OperatingExpense,"income"=>CoreRole::Sales,"opening"=>CoreRole::OwnerCapital,_=>bail!("نوع تراکنش نامعتبر")})?};ensure!(a!=counter,"مبدأ و مقصد یکسان است");
    let incoming=matches!(kind,"income"|"opening");
    s.accounting.post(PostCommand{company_id:ctx.company_id,period_id:ctx.period_id,date:ctx.date,source_id:ctx.source_id,source_revision:1,event:format!("desktop.{kind}"),request_key:ctx.request_key,actor:ctx.actor,reason:ctx.reason.clone(),kind:JournalKind::Manual,lines:vec![JournalLineInput{account_id:a.into(),debit:if incoming{amount}else{Money::ZERO},credit:if incoming{Money::ZERO}else{amount},description:ctx.reason.clone()},JournalLineInput{account_id:counter.into(),debit:if incoming{Money::ZERO}else{amount},credit:if incoming{amount}else{Money::ZERO},description:ctx.reason}]})?;Ok(())
}
fn trade(s:&mut State,p:&Value,selling:bool)->Result<()> {
    let key=string(p,"requestKey")?;let ctx=context(s,p,key)?;
    // Replays return the original result, including multi-command receive/invoice chains.
    if s.desktop.audit.iter().any(|x|x.operation=="trade-request"&&x.detail==key){return Ok(());}
    let r=resources(s)?;let mut total=Money::ZERO;
    let lines=p["lines"].as_array().ok_or_else(||anyhow::anyhow!("ردیف کالا لازم است"))?.iter().map(|l|{let item=r.items.iter().find(|i|i.id==optional(l,"itemId")).ok_or_else(||anyhow::anyhow!("کالا را انتخاب کنید"))?;let price=money(l,"price")?;let q=domain::quantity(string(l,"quantity")?)?;let net=Money::from_rials(domain::line_total(price.rials(),q)?);total=total.checked_add(net)?;Ok(crate::procurement::OrderLine{id:domain::id(),item_id:item.id.clone(),warehouse_id:string(l,"warehouseId")?.into(),unit_id:item.base_unit_id.clone(),quantity_thousandths:q,unit_price:price,line_discount:Money::ZERO,price_reference:string(p,"reference")?.into(),price_date:ctx.date.clone(),override_reason:None})}).collect::<Result<Vec<_>>>()?;
    let reference=string(p,"reference")?;let party=string(p,"partyId")?;let due=date(s,p,"due")?;
    if selling {
        let order=s.accounting.post_sales_command(SalesCommand::Order(SalesOrder{context:ctx.clone(),customer_id:party.into(),reference:format!("{reference}-ORDER"),document_date:ctx.date.clone(),lines:lines.clone(),document_discount:Money::ZERO,supplied_total:total,quotation:None}))?;
        let mut fulfill_ctx=ctx.clone();fulfill_ctx.source_id=format!("{key}-fulfill");fulfill_ctx.request_key=fulfill_ctx.source_id.clone();
        s.accounting.post_sales_command(SalesCommand::Fulfill(FulfillOrder{context:fulfill_ctx,order_id:order.record_id,delivery_id:domain::id(),invoice_id:domain::id(),delivery_reference:format!("{reference}-DEL"),invoice_reference:reference.into(),document_date:ctx.date,due_date:due,lines:lines.iter().map(|l|FulfillLine{id:domain::id(),order_line_id:l.id.clone(),warehouse_id:l.warehouse_id.clone(),unit_id:l.unit_id.clone(),quantity_thousandths:l.quantity_thousandths}).collect(),supplied_total:total}))?;
    }else{
        let order=s.accounting.post_procurement_command(ProcurementCommand::Order(PurchaseOrder{context:ctx.clone(),supplier_id:party.into(),reference:format!("{reference}-ORDER"),document_date:ctx.date.clone(),lines:lines.clone(),document_discount:Money::ZERO,supplied_total:total}))?;
        let mut rc=ctx.clone();rc.source_id=format!("{key}-receive");rc.request_key=rc.source_id.clone();
        let receipt=s.accounting.post_procurement_command(ProcurementCommand::Receive(ReceiveOrder{context:rc,order_id:order.record_id,reference:format!("{reference}-RECEIPT"),document_date:ctx.date.clone(),lines:lines.iter().map(|l|ReceiveLine{id:domain::id(),order_line_id:l.id.clone(),warehouse_id:l.warehouse_id.clone(),unit_id:l.unit_id.clone(),quantity_thousandths:l.quantity_thousandths}).collect(),supplied_total:total}))?;
        let ProcurementData::Receive(received)=s.accounting.procurement_document(&receipt.record_id)?.data.clone() else {unreachable!()};
        let mut bc=ctx.clone();bc.source_id=format!("{key}-bill");bc.request_key=bc.source_id.clone();
        s.accounting.post_procurement_command(ProcurementCommand::Bill(MatchBill{context:bc,supplier_id:party.into(),reference:reference.into(),document_date:ctx.date,due_date:due,lines:received.lines.iter().map(|l|BillLine{id:domain::id(),receipt_line_id:l.id.clone(),quantity_thousandths:l.quantity_thousandths,supplied_value:l.value}).collect(),supplied_total:total}))?;
    }audit(s,"trade-request",key);Ok(())
}
fn return_document(s:&mut State,p:&Value,selling:bool)->Result<()> {
    let ctx=context(s,p,string(p,"requestKey")?)?;let source=string(p,"id")?;let reference=string(p,"reference")?;
    if selling{
        let rem=s.accounting.sales_invoice_returnable(source)?;let original=s.accounting.sales_document(source)?.data.clone();let SalesData::Fulfill(f)=original else{bail!("فاکتور فروش لازم است")};let(mut credit,mut cost)=(Money::ZERO,Money::ZERO);
        let lines=p["lines"].as_array().ok_or_else(||anyhow::anyhow!("ردیف برگشت لازم است"))?.iter().map(|l|{let r=rem.iter().find(|x|x.id==optional(l,"lineId")).ok_or_else(||anyhow::anyhow!("ردیف منبع نامعتبر"))?;let o=f.lines.iter().find(|x|x.id==r.id).unwrap();let q=domain::quantity(string(l,"quantity")?)?;
            let net=crate::sales::allocation(o.net,o.quantity_thousandths,r.returned_quantity_thousandths,o.net.checked_sub(r.net)?,q)?;let c=crate::sales::allocation(o.cost,o.quantity_thousandths,r.returned_quantity_thousandths,o.cost.checked_sub(r.cost)?,q)?;credit=credit.checked_add(net)?;cost=cost.checked_add(c)?;Ok(ReturnLine{id:domain::id(),fulfillment_line_id:r.id.clone(),quantity_thousandths:q,supplied_credit:net,supplied_cost:c})}).collect::<Result<_>>()?;
        s.accounting.post_sales_command(SalesCommand::Return(ReturnSale{context:ctx.clone(),fulfillment_id:source.into(),return_id:domain::id(),credit_id:domain::id(),return_reference:reference.into(),credit_reference:format!("{reference}-CREDIT"),document_date:ctx.date,lines,supplied_credit:credit,supplied_cost:cost}))?;
    }else{
        let rem=bill_returnable(s,source)?;let ProcurementData::Bill(b)=s.accounting.procurement_document(source)?.data.clone() else{bail!("فاکتور خرید لازم است")};let mut total=Money::ZERO;
        let lines=p["lines"].as_array().ok_or_else(||anyhow::anyhow!("ردیف برگشت لازم است"))?.iter().map(|l|{let r=rem.iter().find(|x|x.bill_line_id.as_deref()==Some(optional(l,"lineId"))).ok_or_else(||anyhow::anyhow!("ردیف منبع نامعتبر"))?;let q=domain::quantity(string(l,"quantity")?)?;let value=crate::procurement::allocation(r.original_value,r.original_quantity_thousandths,r.original_quantity_thousandths-r.remaining_quantity_thousandths,r.original_value.checked_sub(r.remaining_value)?,q)?;total=total.checked_add(value)?;Ok(PurchaseReturnLine{id:domain::id(),receipt_line_id:r.receipt_line_id.clone(),bill_line_id:r.bill_line_id.clone(),quantity_thousandths:q,supplied_value:value})}).collect::<Result<_>>()?;
        s.accounting.post_procurement_command(ProcurementCommand::Return(ReturnPurchase{context:ctx.clone(),supplier_id:b.supplier_id,return_id:domain::id(),return_reference:reference.into(),credit_id:Some(domain::id()),credit_reference:Some(format!("{reference}-CREDIT")),document_date:ctx.date,lines,supplied_total:total}))?;
    }Ok(())
}
fn rows(s:&State,p:&Value)->Result<Value>{
    let module=string(p,"module")?;let company=active(s)?;let search=domain::normalize(optional(p,"search"));let r=resources(s).ok();let mut out=Vec::<Value>::new();
    let party_name=|id:&str|r.as_ref().and_then(|r|r.parties.iter().find(|x|x.id==id)).map(|x|x.name.clone()).unwrap_or_else(||id.into());
    match module {
        "ledger"=>{let mut running=Money::ZERO;let account=optional(p,"accountId");let mut journals=s.accounting.journals.iter().filter(|j|j.company_id==company).collect::<Vec<_>>();journals.sort_by(|a,b|a.date.cmp(&b.date).then(a.recorded_at.cmp(&b.recorded_at)));
            for j in journals {let mut debit=Money::ZERO;let mut credit=Money::ZERO;for l in &j.lines {if !account.is_empty()&&l.account.id!=account{continue;}debit=debit.checked_add(l.debit)?;credit=credit.checked_add(l.credit)?;}
                if !account.is_empty() && !j.lines.iter().any(|l|l.account.id==account){continue;}
                if !account.is_empty(){running=running.checked_add(debit.checked_sub(credit)?)?;}
                out.push(json!({"id":j.id,"kind":"journal","date":j.date,"name":j.reason,"reference":j.event,"status":if matches!(j.kind,JournalKind::Reversal{..}){"reversal"}else{"posted"},"amount":debit.rials().to_string(),"balance":if account.is_empty(){Value::Null}else{json!(running.rials().to_string())},"detail":j.lines.iter().map(|l|l.account.name.as_str()).collect::<Vec<_>>().join(" · ")}));
            }out.reverse();},
        "products"=>{if let Some(r)=&r {for item in &r.items {let price=s.catalogs.iter().rev().flat_map(|c|&c.products).find(|x|x.code==item.sku).map(|x|x.price_rial).unwrap_or(0);out.push(json!({"id":item.id,"kind":"product","name":item.name,"reference":item.sku,"detail":r.units.iter().find(|u|u.id==item.base_unit_id).map(|u|u.name.as_str()).unwrap_or(""),"amount":price.to_string(),"status":"active"}));}}
            for c in &s.catalogs {for product in &c.products {if !out.iter().any(|x|x["reference"].as_str()==Some(&product.code)){out.push(json!({"id":format!("{}:{}",c.id,product.code),"kind":"catalogProduct","name":product.name,"reference":product.code,"detail":c.name,"amount":product.price_rial.to_string(),"status":"catalog"}));}}}},
        "quotes"=>{for q in &s.quotes{out.push(json!({"id":q.id,"kind":"quote","name":q.customer,"reference":q.number,"date":q.quote_date,"amount":q.total_rial.to_string(),"status":format!("revision {}",q.version),"detail":q.reference}));}},
        "purchases"|"purchaseReturns"=>{for x in s.accounting.procurement.records.iter().filter(|x|x.command.context().company_id==company){let (name,reference,total,kind)=match (&x.command,&x.data){(ProcurementCommand::Bill(c),ProcurementData::Bill(b)) if module=="purchases"=>(party_name(&b.supplier_id),c.reference.clone(),b.total,"purchase"),(ProcurementCommand::Order(c),ProcurementData::Order(b)) if module=="purchases"=>(party_name(&b.supplier.id),c.reference.clone(),b.total,"purchaseOrder"),(ProcurementCommand::Receive(c),ProcurementData::Receive(b)) if module=="purchases"=>(party_name(&b.supplier_id),c.reference.clone(),b.total,"receipt"),(ProcurementCommand::Return(c),ProcurementData::Return(b)) if module=="purchaseReturns"=>(party_name(&b.supplier_id),c.return_reference.clone(),b.total,"purchaseReturn"),_=>continue};out.push(json!({"id":x.id,"kind":kind,"name":name,"reference":reference,"date":x.command.context().date,"amount":total.rials().to_string(),"status":"posted","detail":x.journal_id}));}},
        "sales"|"salesReturns"=>{for x in s.accounting.sales.records.iter().filter(|x|x.command.context().company_id==company){let(name,reference,total,kind)=match(&x.command,&x.data){(SalesCommand::Fulfill(c),SalesData::Fulfill(b)) if module=="sales"=>(b.customer.name.clone(),c.invoice_reference.clone(),b.total,"sale"),(SalesCommand::Order(c),SalesData::Order(b)) if module=="sales"=>(b.customer.name.clone(),c.reference.clone(),b.total,"salesOrder"),(SalesCommand::Return(c),SalesData::Return(b)) if module=="salesReturns"=>(party_name(&b.customer_id),c.return_reference.clone(),b.total,"salesReturn"),_=>continue};out.push(json!({"id":x.id,"kind":kind,"name":name,"reference":reference,"date":x.command.context().date,"amount":total.rials().to_string(),"status":"posted","detail":x.journal_id}));}},
        "inventory"=>{let stock=s.accounting.procurement_reconcile(&company,&local_today().to_string())?.stock;let r=r.as_ref().unwrap();for pool in stock {out.push(json!({"id":format!("{}:{}",pool.item_id,pool.warehouse_id),"kind":"stock","name":r.items.iter().find(|x|x.id==pool.item_id).map(|x|x.name.as_str()).unwrap_or(""),"reference":r.warehouses.iter().find(|x|x.id==pool.warehouse_id).map(|x|x.name.as_str()).unwrap_or(""),"detail":domain::qty_display(pool.quantity_thousandths),"amount":pool.value.rials().to_string(),"status":if pool.quantity_thousandths==0{"empty"}else{"available"}}));}},
        "contacts"=>{let today=local_today();let date=today.to_string();let ar=s.accounting.sales_receivables(&company,&date).unwrap_or_default();let ap=s.accounting.procurement_reconcile(&company,&date).map(|x|x.payables).unwrap_or_default();if let Some(r)=&r{for party in &r.parties {let mut receivable=Money::ZERO;let mut payable=Money::ZERO;let mut aging=[Money::ZERO;4];for x in ar.iter().filter(|x|x.party_id==party.id){receivable=receivable.checked_add(x.remaining)?;let days=(today-NaiveDate::parse_from_str(&x.due_date,"%Y-%m-%d")?).num_days();let i=if days<=0{0}else if days<=30{1}else if days<=60{2}else{3};aging[i]=aging[i].checked_add(x.remaining)?;}for x in ap.iter().filter(|x|x.party_id==party.id){payable=payable.checked_add(x.remaining)?;}
            let contact=s.customers.iter().find(|x|x.id==party.id);out.push(json!({"id":party.id,"kind":"contact","name":party.name,"reference":contact.map(|c|c.phone.as_str()).unwrap_or(""),"detail":contact.map(|c|c.email.as_str()).unwrap_or(""),"status":if party.customer&&party.supplier{"both"}else if party.customer{"customer"}else{"supplier"},"amount":receivable.rials().to_string(),"payable":payable.rials().to_string(),"aging":aging.iter().map(|m|m.rials().to_string()).collect::<Vec<_>>()}));}}
            for c in &s.customers {if !out.iter().any(|x|x["id"].as_str()==Some(&c.id)){out.push(json!({"id":c.id,"kind":"legacyContact","name":c.name,"reference":c.phone,"detail":c.email,"status":"customer","amount":"0","payable":"0","aging":["0","0","0","0"]}));}}},
        "reports"=>{let trial=s.accounting.trial_balance(&company,&local_today().to_string())?;for x in trial.rows{out.push(json!({"id":x.account_id,"kind":"trial","reference":x.code,"name":x.name,"amount":x.debit.rials().to_string(),"balance":x.credit.rials().to_string(),"status":"reconciled"}));}},
        "audit"=>{for x in &s.accounting.audit {if x.company_id==company{out.push(json!({"id":x.id,"kind":"audit","name":format!("{:?}",x.operation),"reference":x.actor,"date":x.recorded_at,"detail":x.reason,"status":"recorded"}));}}for x in &s.desktop.audit{out.push(json!({"id":x.id,"kind":"audit","name":x.operation,"reference":"desktop-owner","date":x.at,"detail":x.detail,"status":"recorded"}));}out.reverse();},
        _=>bail!("صفحه ناشناخته")
    }
    out.retain(|x|search.is_empty()||domain::normalize(&x.to_string()).contains(&search));
    let total=out.len();let offset=p["offset"].as_u64().unwrap_or(0) as usize;let limit=p["limit"].as_u64().unwrap_or(150).min(500) as usize;Ok(json!({"total":total,"rows":out.into_iter().skip(offset).take(limit).collect::<Vec<_>>()}))
}
fn detail(s:&State,p:&Value)->Result<Value>{let id=string(p,"id")?;match string(p,"kind")?{
    "journal"=>{let j=s.accounting.journals.iter().find(|x|x.id==id&&x.company_id==active(s).unwrap_or_default()).ok_or_else(||anyhow::anyhow!("سند یافت نشد"))?;Ok(json!({"data":json_exact(j)?,"reversible":!s.accounting.has_typed_journal_source(id)&&!matches!(j.kind,JournalKind::Reversal{..}),"lines":j.lines.iter().map(|l|json!({"name":l.account.name,"debit":l.debit.rials().to_string(),"credit":l.credit.rials().to_string(),"description":l.description})).collect::<Vec<_>>()}))},
    "sale"=>Ok(json!({"data":json_exact(s.accounting.sales_document(id)?)?,"returnable":json_exact(&s.accounting.sales_invoice_returnable(id)?)?})),
    "purchase"=>Ok(json!({"data":json_exact(s.accounting.procurement_document(id)?)?,"returnable":json_exact(&bill_returnable(s,id)?)?})),
    "quote"=>Ok(json!({"data":json_exact(&s.quotes.iter().find(|q|q.id==id).ok_or_else(||anyhow::anyhow!("پیش‌فاکتور یافت نشد"))?)?})),
    "contact"|"legacyContact"=>Ok(json!({"data":s.customers.iter().find(|c|c.id==id),"history":s.accounting.sales.records.iter().filter(|r|match &r.data {SalesData::Order(x)=>x.customer.id==id,SalesData::Fulfill(x)=>x.customer.id==id,SalesData::Return(x)=>x.customer_id==id}).map(json_exact).collect::<Result<Vec<_>>>()?})),
    _=>Ok(json!({"data":p}))}}
fn export(s:&State,p:&Value)->Result<Value>{
    if optional(p,"format")=="json"{return Ok(json!({"name":"sarv-books.json","mime":"application/json","content":serde_json::to_string_pretty(&json!({"format":"sarv-accounting-backup-v7","checksum":crate::imports::hash(&serde_json::to_vec(s)?),"state":s}))?}));}
    let all=rows(s,&json!({"module":string(p,"module")?,"limit":500}))?;let mut writer=csv::Writer::from_writer(vec![]);writer.write_record(["id","date","name","reference","amount_rial","balance_rial","detail"])?;
    // Export every page, not merely the visible virtualized rows.
    let total=all["total"].as_u64().unwrap_or(0);for offset in (0..total).step_by(500){let page=rows(s,&json!({"module":string(p,"module")?,"offset":offset,"limit":500}))?;for row in page["rows"].as_array().unwrap(){writer.write_record(["id","date","name","reference","amount","balance","detail"].map(|k|crate::export::safe_cell(optional(row,k))))?;}}
    Ok(json!({"name":format!("sarv-{}.csv",string(p,"module")?),"mime":"text/csv;charset=utf-8","content":String::from_utf8(writer.into_inner()?)?}))
}
/// Local transport owner. Team transport may implement the same command contract later.
/// Workspace switches select independent stores; they never overwrite the old store.
pub struct DesktopHost { root: std::path::PathBuf, current: std::sync::Mutex<AppService> }
impl DesktopHost {
    pub fn open(root:impl AsRef<std::path::Path>)->Result<Self>{
        let root=root.as_ref().to_path_buf();let marker=root.join("desktop-workspace.json");
        let directory=if marker.exists(){let name:String=serde_json::from_slice(&std::fs::read(marker)?)?;ensure!(name.len()==36&&name.chars().all(|c|c.is_ascii_hexdigit()||c=='-'),"شناسه فضای کاری نامعتبر");root.join("workspaces").join(name)}else{root.clone()};
        Ok(Self{current:std::sync::Mutex::new(AppService::open(directory)?),root})
    }
    pub fn read(&self,action:&str,p:Value)->Result<Value>{let mut value=self.current.lock().map_err(|_|anyhow::anyhow!("قفل برنامه نامعتبر"))?.desktop_read(action,p)?;if action=="workspace"{self.enrich(&mut value)?;}Ok(value)}
    fn enrich(&self,value:&mut Value)->Result<()> {
        let marker=self.root.join("desktop-workspace.json");let selected=if marker.exists(){serde_json::from_slice::<String>(&std::fs::read(marker)?)?}else{"original".into()};
        let mut candidates=vec![("original".to_string(),self.root.clone())];let folder=self.root.join("workspaces");
        if folder.exists(){for entry in std::fs::read_dir(folder)?.take(100){let entry=entry?;let id=entry.file_name().to_string_lossy().to_string();if uuid::Uuid::parse_str(&id).is_ok(){candidates.push((id,entry.path()));}}}
        let mut choices=vec![];for(id,path)in candidates {let file=path.join("sarv.sqlite3");if !file.is_file(){continue;}let conn=rusqlite::Connection::open_with_flags(file,rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;let raw:String=conn.query_row("SELECT payload FROM state WHERE id=1",[],|r|r.get(0))?;let state:Value=serde_json::from_str(&raw)?;let name=state["accounting"]["companies"].as_array().and_then(|c|c.first()).and_then(|c|c["name"].as_str()).or(state["settings"]["business_name"].as_str()).filter(|n|!n.is_empty()).unwrap_or("فضای کاری جدید");choices.push(json!({"id":id,"name":name}));}
        value["workspaceChoices"]=json!(choices);value["workspaceId"]=json!(selected);Ok(())
    }
    pub fn mutate(&self,action:&str,p:Value)->Result<Value>{let mut current=self.current.lock().map_err(|_|anyhow::anyhow!("قفل برنامه نامعتبر"))?;
        if action=="newWorkspace"||action=="originalWorkspace"||action=="switchWorkspace"{
            let id=if action=="switchWorkspace"{string(&p,"id")?.into()}else if action=="originalWorkspace"{"original".into()}else{domain::id()};
            ensure!(id=="original"||uuid::Uuid::parse_str(&id).is_ok(),"شناسه فضای کاری نامعتبر");
            let directory=if id=="original"{self.root.clone()}else{self.root.join("workspaces").join(&id)};
            ensure!(action=="newWorkspace"||directory.join("sarv.sqlite3").is_file(),"فضای کاری یافت نشد");let service=AppService::open(directory)?;
            let mut value=service.desktop_read("workspace",json!({}))?;
            if id=="original"{if self.root.join("desktop-workspace.json").exists(){std::fs::remove_file(self.root.join("desktop-workspace.json"))?;}}else{std::fs::write(self.root.join("desktop-workspace.tmp"),serde_json::to_vec(&id)?)?;std::fs::rename(self.root.join("desktop-workspace.tmp"),self.root.join("desktop-workspace.json"))?;}
            *current=service;self.enrich(&mut value)?;Ok(value)
        }else{let mut value=current.desktop_mutate(action,p)?;self.enrich(&mut value)?;Ok(value)}
    }
}

fn bill_returnable(s:&State,id:&str)->Result<Vec<PurchaseReturnableLine>>{
    let ProcurementData::Bill(b)=&s.accounting.procurement_document(id)?.data else {bail!("فاکتور خرید لازم است")};
    b.lines.iter().map(|l|s.accounting.procurement_bill_returnable(&active(s)?,&l.id)).collect()
}

fn local_today()->NaiveDate{Utc::now().with_timezone(&chrono::FixedOffset::east_opt(12600).unwrap()).date_naive()}

// Full desktop and local agent transport share the original validated store.
#[path = "desktop_bridge.rs"]
mod bridge;
pub use bridge::{dispatch, request_id, seed_fixture, wire};
