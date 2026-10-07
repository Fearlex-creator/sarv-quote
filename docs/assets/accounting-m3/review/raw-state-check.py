#!/usr/bin/env python3
"""Independent integer oracle over committed SQLite JSON, no Sarv import."""
import json,sqlite3,sys,hashlib
from pathlib import Path
profile=Path(sys.argv[1]);output=Path(sys.argv[2])
with sqlite3.connect(f'file:{profile}/sarv.sqlite3?mode=ro',uri=True) as db:
 generation,payload=db.execute('select generation,payload from state where id=1').fetchone()
 integrity=db.execute('pragma integrity_check').fetchone()[0]
s=json.loads(payload);a=s['accounting'];o=a['core']['openings'][0]['command'];j=a['journals'][0]
assert s['schema']==4 and integrity=='ok'
assert len(a['core']['openings'])==len(a['journals'])==1
mapping={x['role']:x['account_id'] for x in a['core']['setups'][0]['command']['mappings']}
expected={'Cash':251,'Bank':300,'Receivables':200,'ChequesReceivable':200,'Inventory':300,'SupplierAdvances':0,'Payables':-149,'ChequesPayable':-100,'Grni':-50,'CustomerAdvances':0,'OwnerCapital':-952,'Cogs':0,'InventoryLoss':0}
actual={role:sum(x['debit']-x['credit'] for x in j['lines'] if x['account']['id']==aid) for role,aid in mapping.items()}
assert all(actual[k]==v for k,v in expected.items()),(actual,expected)
assert sum(x['debit'] for x in j['lines'])==1251==sum(x['credit'] for x in j['lines'])
assert sum(x['remaining'] for x in o['open_items'] if x['side']=='Receivable')==200
assert sum(x['remaining'] for x in o['open_items'] if x['side']=='Payable')==149
assert {x['side'] for x in o['open_items'] if x['party_id']=='party-a'}=={'Receivable','Payable'}
assert len(o['advances'])==0
qty={};val={}
for x in o['stock']:
 item=next(i for i in o['items'] if i['id']==x['item_id'])
 q=x['quantity_thousandths']
 if x['unit_id']!=item['base_unit_id']:
  pack=next(p for p in item['packs'] if p['unit_id']==x['unit_id']);n=q*pack['numerator'];assert n%pack['denominator']==0;q=n//pack['denominator']
 key=x['warehouse_id'];qty[key]=qty.get(key,0)+q;val[key]=val.get(key,0)+x['value']
assert qty=={'w1':24000,'w2':6000} and val=={'w1':240,'w2':60}
assert sum(qty.values())==30000 and sum(val.values())==300
assert o['grni'][0]['warehouse_id']=='w2' and o['grni'][0]['quantity_thousandths']==5000 and o['grni'][0]['value']==50
assert len(o['cheques'])==2 and sum(x['amount'] for x in o['cheques'] if x['direction']=='Receivable')==200 and sum(x['amount'] for x in o['cheques'] if x['direction']=='Payable')==100
assert {x['settlement_reference'] for x in o['cheques']}.isdisjoint({x['document_reference'] for x in o['open_items']})
assert all(actual[k]==0 for k in ['Sales','SalesReturns','Cogs','OperatingExpense','BankFees','InventoryLoss'])
assert 1251-(149+100+50)==952==-actual['OwnerCapital']
report={'result':'passed','profile':str(profile),'schema':s['schema'],'generation':generation,'payload_sha256':hashlib.sha256(payload.encode()).hexdigest(),'sqlite_integrity':integrity,'debit_irr':1251,'credit_irr':1251,'signed_ledger_by_role':actual,'quantity_thousandths_by_warehouse':qty,'stock_value_irr_by_warehouse':val,'grni_is_subset':True,'net_assets_equity_irr':952,'pnl_irr':0,'source_id':o['source_id'],'opening_date':o['as_of'],'oracle':'Python standard-library integers over raw persisted state; literal expectations, no production helper'}
output.write_text(json.dumps(report,indent=2,ensure_ascii=False));print(json.dumps(report,ensure_ascii=False))
