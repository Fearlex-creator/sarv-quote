#!/usr/bin/env python3
"""Independently reconstruct actual-chooser posted sources without Sarv helpers."""
import csv,hashlib,json,sqlite3,sys
from pathlib import Path
run=Path(sys.argv[1]);results=[]
for variant in ['zero','nonzero']:
 p=run/variant
 with sqlite3.connect(f'file:{p}/sarv.sqlite3?mode=ro',uri=True) as db:
  a=json.loads(db.execute('select payload from state where id=1').fetchone()[0])['accounting'];assert db.execute('pragma integrity_check').fetchone()[0]=='ok'
 assert len(a['core']['openings'])==1
 o=a['core']['openings'][0];c=o['command'];src=c['import_source'];raw=(p/f'{variant}.csv').read_bytes();rows=list(csv.reader(raw.decode().splitlines()))
 assert src['sha256']==hashlib.sha256(raw).hexdigest()
 assert [r['original_values'] for r in src['rows']]==rows[1:]
 assert len(c['balances'])==13
 mapping={r['role']:r['account_id'] for r in a['core']['setups'][0]['command']['mappings']};amounts={b['account_id']:b['debit']-b['credit'] for b in c['balances']}
 debit=sum(b['debit'] for b in c['balances']);credit=sum(b['credit'] for b in c['balances'])
 if variant=='zero':
  assert debit==credit==0 and o['effect']=='ZeroBalanceInitialization' and a['journals']==[] and a['requests']==[] and len(a['core']['zero_requests'])==1
 else:
  expected={'Cash':251,'Bank':300,'Receivables':200,'Inventory':300,'ChequesReceivable':200,'Payables':-149,'ChequesPayable':-100,'Grni':-50,'OwnerCapital':-952}
  assert all(amounts[mapping[r]]==v for r,v in expected.items()) and debit==credit==1251 and o['effect']=='Journal' and len(a['journals'])==1 and len(a['journals'][0]['lines'])==9
  assert sum(r['remaining'] for r in c['open_items'] if r['side']=='Receivable')==200 and sum(r['remaining'] for r in c['open_items'] if r['side']=='Payable')==149
  assert [(r['quantity_thousandths'],r['value']) for r in c['stock']]==[(2000,240),(6000,60)] and c['items'][0]['packs'][0]['numerator']==12 and c['items'][0]['packs'][0]['denominator']==1
  assert c['grni'][0]['warehouse_id']=='w2' and c['grni'][0]['quantity_thousandths']==5000 and c['grni'][0]['value']==50
 results.append({'variant':variant,'debit_irr':debit,'credit_irr':credit,'balances':len(c['balances']),'source_rows':len(src['rows']),'original_raw_cells_exact':True,'source_sha256':src['sha256'],'financial_journals':len(a['journals']),'effect':o['effect']})
r={'result':'passed','checks':results,'oracle':'Python standard-library integer/raw-source reconstruction from actual native chooser persisted SQLite; no Sarv helpers'}
(run/'independent-native-persisted.json').write_text(json.dumps(r,indent=2));print(json.dumps(r))
