#!/usr/bin/env python3
"""Literal independent report oracle: canonical CSV rows and full source snapshots."""
import csv,json,sys,hashlib
from pathlib import Path
p=Path(sys.argv[1]);state=json.loads(Path(sys.argv[2]).read_text());a=state['accounting'];out=Path(sys.argv[3])
records={next(iter(r['command'].values()))['context']['source_id']:r for r in a['procurement']['records']}
def read(path):
 rows=list(csv.DictReader(path.open()));assert rows
 for r in rows:assert r['base_currency']=='IRR' and r['policy_version']=='core-policy-v1'
 return rows
timeline=[('2026-03-21',300,149,50,24000,240,6000,60,1251),('2026-03-22',375,149,125,29000,315,6000,60,1326),('2026-03-23',480,149,230,36000,420,6000,60,1431),('2026-03-24',480,179,200,36000,420,6000,60,1431),('2026-03-25',480,329,50,36000,420,6000,60,1431),('2026-03-26',480,329,50,30000,350,12000,130,1431),('2026-03-27',480,379,0,30000,350,12000,130,1431)]
for date,inv,ap,grni,q1,v1,q2,v2,tb in timeline:
 rows=read(p/f'control-{date}.csv');controls={r['role']:r for r in rows if r['row_kind']=='control'};expected={'cash':251,'bank':300,'receivables':200,'cheques_receivable':200,'inventory':inv,'supplier_advances':0,'payables':ap,'cheques_payable':100,'grni':grni,'customer_advances':0,'cogs':0,'inventory_loss':0};assert len(controls)==12
 for role,v in expected.items():r=controls[role];assert int(r['ledger_irr'])==int(r['detail_irr'])==v and int(r['difference_irr'])==0
 pools={r['warehouse_id']:(int(r['quantity_thousandths']),int(r['amount_irr'])) for r in rows if r['row_kind']=='stock_pool'};assert pools=={'w1':(q1,v1),'w2':(q2,v2)}
 stock=next(r for r in rows if r['row_kind']=='stock_total');assert (int(stock['quantity_thousandths']),int(stock['amount_irr']))==(q1+q2,inv)
 assert sum(int(r['amount_irr']) for r in rows if r['row_kind']=='payable_remaining')==ap
 assert sum(int(r['amount_irr']) for r in rows if r['row_kind']=='receipt_remaining')==grni
 for r in [x for x in rows if x['row_kind']=='receipt_remaining']:
  if r['row_id']=='r1':original='2026-03-01';available='2026-03-21'
  else:
   source=next(x for x in a['procurement']['records'] if 'Receive' in x['command'] and any(l['id']==r['row_id'] for l in x['data']['Receive']['lines']))['command']['Receive'];original=source['document_date'];available=source['context']['date']
  assert r['document_date']==original and r['availability_date']==available
 assert all(int(r['amount_irr'])==tb for r in rows if r['row_kind'] in ['trial_total_debit','trial_total_credit'])
 snap=json.loads(next(r for r in rows if r['row_kind']=='complete_snapshot')['snapshot_json']);assert snap['opening']==a['core']['openings'][0] and snap['records']==[r for r in a['procurement']['records'] if next(iter(r['command'].values()))['context']['date']<=date]
 assert all(r['basis']=='accrual_perpetual_end_of_day' and r['as_of_gregorian']==date for r in rows)
for name,record in records.items():
 rows=read(p/f'{name}.csv');snap=json.loads(next(r for r in rows if r['row_kind']=='complete_snapshot')['snapshot_json']);assert snap['document']==record
 assert all(r['basis']=='immutable_source_snapshot_at_recording' for r in rows)
 meta=next(r for r in rows if r['row_kind']=='document_metadata');assert meta['financial_effect_kind']==('journal' if record['journal_id'] else 'nonfinancial') and meta['financial_effect_id']==(record['journal_id'] or record['id'])
 kind=next(iter(record['command']));data=record['data'][kind]
 if kind in ['Order','Receive','Bill']:
  total=next(r for r in rows if r['row_kind']=='document_total');assert int(total['amount_irr'])==data['total']
 if kind=='Bill':
  matches=[r for r in rows if r['row_kind']=='bill_match'];assert [(r['source_line_id'],int(r['quantity_thousandths']),int(r['amount_irr'])) for r in matches]==[(l['receipt_line_id'],l['quantity_thousandths'],l['value']) for l in data['lines']]
  assert snap['opening_source']==a['core']['openings'][0]
 if kind=='Transfer':
  moves=[r for r in rows if r['row_kind']=='stock_movement'];assert len(moves)==2 and {r['direction'] for r in moves}=={'in','out'} and all(int(r['amount_irr'])==70 and int(r['quantity_thousandths'])==6000 for r in moves)
report={'result':'passed','oracle':'Independent literal timeline over canonical CSV plus exact immutable source snapshots; no Sarv helper','documents':len(records),'historical_eod_snapshots':len(timeline),'final_stock_quantity_thousandths':42000,'final_stock_irr':480,'final_ap_irr':379,'final_grni_irr':0,'csv_sha256':{x.name:hashlib.sha256(x.read_bytes()).hexdigest() for x in sorted(p.glob('*.csv'))}}
out.write_text(json.dumps(report,indent=2));print(json.dumps(report))
