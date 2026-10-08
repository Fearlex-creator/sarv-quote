use anyhow::Result;
use sarv_quote::{accounting::*,core::*};
use serde_json::json;
pub(crate) fn fixture() -> Result<(AccountingState,OpeningCommand)> {
 let mut state=AccountingState::default();
 let made=state.setup_core_company(NewCoreCompany{name:"F2 independent reviewer".into(),fiscal_name:"1405".into(),start_date:"2026-03-21".into(),end_date:"2027-03-20".into(),timezone:CompanyTimezone::AsiaTehran,chart:default_chart(),actor:"Independent reviewer".into()})?;
 let c=&made.company_id;
 let mut balances=vec![];
 for (role,debit,credit) in [(CoreRole::Cash,251,0),(CoreRole::Bank,300,0),(CoreRole::Receivables,200,0),(CoreRole::Inventory,300,0),(CoreRole::ChequesReceivable,200,0),(CoreRole::Payables,0,149),(CoreRole::ChequesPayable,0,100),(CoreRole::Grni,0,50),(CoreRole::OwnerCapital,0,952),(CoreRole::SupplierAdvances,0,0),(CoreRole::CustomerAdvances,0,0),(CoreRole::Cogs,0,0),(CoreRole::InventoryLoss,0,0)] {
  let a=state.core.setups[0].account_id(role)?;
  balances.push(json!({"account_id":a,"debit":debit,"credit":credit,"source_reference":format!("literal-{}",role.key())}));
 }
 let raw=json!({"company_id":c,"period_id":made.period_id,"setup_id":made.setup_id,"as_of":"2026-03-21","source_id":"independent-F2-source","request_key":"arbitrary-review-key","actor":"Independent reviewer","reason":"Literal fixture, no balancing helper","import_source":null,"balances":balances,
 "parties":[{"id":"party-a","company_id":c,"name":"dual role A","customer":true,"supplier":true},{"id":"party-b","company_id":c,"name":"dual role B","customer":true,"supplier":true}],
 "units":[{"id":"piece","company_id":c,"name":"piece"},{"id":"carton","company_id":c,"name":"carton"}],
 "items":[{"id":"item-a","company_id":c,"sku":"A","name":"item A","base_unit_id":"piece","indivisible":true,"packs":[{"unit_id":"carton","numerator":12,"denominator":1}]}],
 "warehouses":[{"id":"w1","company_id":c,"name":"W1"},{"id":"w2","company_id":c,"name":"W2"}],
 "cash":[{"id":"c1","company_id":c,"name":"C1","balance":251,"source_reference":"count251"}],
 "banks":[{"id":"b1","company_id":c,"name":"B1","bank_reference":"bank-number-B1","balance":300,"statement_balance":300,"source_reference":"statement300"}],
 "open_items":[{"id":"ar-a","company_id":c,"party_id":"party-a","side":"Receivable","document_reference":"INV-A","document_date":"2026-02-01","due_date":"2026-04-01","remaining":125},{"id":"ar-b","company_id":c,"party_id":"party-b","side":"Receivable","document_reference":"INV-B","document_date":"2026-02-02","due_date":"2026-04-02","remaining":75},{"id":"ap-a","company_id":c,"party_id":"party-a","side":"Payable","document_reference":"BILL-A","document_date":"2026-02-03","due_date":"2026-04-03","remaining":100},{"id":"ap-b","company_id":c,"party_id":"party-b","side":"Payable","document_reference":"BILL-B","document_date":"2026-02-04","due_date":"2026-04-04","remaining":49}],
 "advances":[],
 "cheques":[{"id":"cr1","company_id":c,"party_id":"party-a","direction":"Receivable","state":"Held","bank_reference":"CR-bank","holder":"C1","cheque_reference":"CR1","settlement_reference":"SETTLED-OLD-INVOICE200","issue_date":"2026-02-10","due_date":"2026-05-10","amount":200},{"id":"cp1","company_id":c,"party_id":"party-a","direction":"Payable","state":"Issued","bank_reference":"CP-bank","holder":"supplier A","cheque_reference":"CP1","settlement_reference":"SETTLED-OLD-BILL100","issue_date":"2026-02-11","due_date":"2026-05-11","amount":100}],
 "stock":[{"id":"stock-w1","company_id":c,"item_id":"item-a","warehouse_id":"w1","unit_id":"carton","quantity_thousandths":2000,"value":240,"source_reference":"count-2-cartons"},{"id":"stock-w2","company_id":c,"item_id":"item-a","warehouse_id":"w2","unit_id":"piece","quantity_thousandths":6000,"value":60,"source_reference":"count6-includesR1"}],
 "grni":[{"id":"r1","company_id":c,"party_id":"party-a","item_id":"item-a","warehouse_id":"w2","unit_id":"piece","quantity_thousandths":5000,"value":50,"receipt_reference":"R1","received_date":"2026-03-01"}]});
 Ok((state,serde_json::from_value(raw)?))
}
