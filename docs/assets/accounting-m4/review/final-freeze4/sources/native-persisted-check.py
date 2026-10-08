#!/usr/bin/env python3
"""Independent literal native procurement arithmetic over raw persisted SQLite."""
import hashlib,json,sqlite3,sys
from pathlib import Path
p=Path(sys.argv[1]);out=Path(sys.argv[2])
with sqlite3.connect(f'file:{p}/sarv.sqlite3?mode=ro',uri=True) as db:
 generation,payload=db.execute('select generation,payload from state where id=1').fetchone();assert db.execute('pragma integrity_check').fetchone()[0]=='ok'
state=json.loads(payload);a=state['accounting'];assert state['schema']==5;opening=a['core']['openings'][0];old=opening['command'];records=a['procurement']['records'];assert len(records)==10 and len(a['journals'])==5
pools={'W1':[24000,240],'W2':[6000,60]};receipts={old['grni'][0]['id']:[5000,50,5000,50]};received_q=received_v=0;ap=149;grni=50;order_id=None;timeline=[];seen={'Resources':0,'Order':0,'Receive':0,'Bill':0,'Transfer':0}
mapping={r['role']:r['account_id'] for r in opening['frozen_setup']['command']['mappings']}
for r in records:
 kind,cmd=next(iter(r['command'].items()));data=r['data'][kind];date=cmd['context']['date'];seen[kind]+=1
 if kind=='Resources':assert r['journal_id'] is None and not r['movements']
 elif kind=='Order':
  assert data['total']==cmd['supplied_total']==180 and r['journal_id'] is None and not r['movements'];line=data['lines'][0];assert line['gross']==line['net']==180 and line['base_quantity_thousandths']==12000 and line['input']['unit_price']==15;order_id=r['id']
 elif kind=='Receive':
  assert cmd['order_id']==order_id;line=data['lines'][0];q=line['quantity_thousandths'];v=line['value'];assert (q,v) in [(5000,75),(7000,105)];received_q+=q;target=(180*received_q*2+12000)//24000;assert v==target-received_v;received_v=target;receipts[line['id']]=[q,v,q,v]
  warehouse=line['warehouse_id'];assert warehouse=='W1';before=pools[warehouse][:];pools[warehouse][0]+=q;pools[warehouse][1]+=v;grni+=v;assert len(r['movements'])==1 and r['journal_id']
  m=r['movements'][0];assert m['direction']=='In' and [m['before']['quantity_thousandths'],m['before']['value']]==before and [m['after']['quantity_thousandths'],m['after']['value']]==pools[warehouse]
 elif kind=='Bill':
  total=0
  for line in data['lines']:
   rem=receipts[line['receipt_line_id']];q=line['quantity_thousandths'];origq,origv=rem[2:];cumulative=origq-rem[0]+q;assert 0<cumulative<=origq;target=(origv*cumulative*2+origq)//(2*origq);increment=target-(origv-rem[1]);assert line['value']==increment;total+=increment;rem[0]-=q;rem[1]-=increment
  assert total==data['total']==cmd['supplied_total'] and total in [30,150];ap+=total;grni-=total;assert r['journal_id'] and not r['movements']
 elif kind=='Transfer':
  assert cmd['source_warehouse_id']=='W1' and cmd['target_warehouse_id']=='W2' and data['quantity_thousandths']==6000 and r['journal_id'] is None;cost=(pools['W1'][1]*6000*2+pools['W1'][0])//(2*pools['W1'][0]);assert cost==data['value']==70 and len(r['movements'])==2
  for warehouse,delta,direction in [('W1',-1,'Out'),('W2',1,'In')]:
   before=pools[warehouse][:];pools[warehouse][0]+=delta*6000;pools[warehouse][1]+=delta*70;m=next(m for m in r['movements'] if m['warehouse_id']==warehouse);assert m['direction']==direction and [m['before']['quantity_thousandths'],m['before']['value']]==before and [m['after']['quantity_thousandths'],m['after']['value']]==pools[warehouse]
 if kind in ['Receive','Bill','Transfer']:
  signed={}
  for j in a['journals']:
   if j['date']<=date:
    for l in j['lines']:signed[l['account']['id']]=signed.get(l['account']['id'],0)+l['debit']-l['credit']
  assert signed[mapping['Inventory']]==sum(v[1] for v in pools.values()) and -signed[mapping['Payables']]==ap and -signed[mapping['Grni']]==grni and sum(v[1] for v in receipts.values())==grni
  timeline.append({'reference':cmd.get('reference','transfer'),'posting_date':date,'inventory_irr':sum(v[1] for v in pools.values()),'AP_irr':ap,'GRNI_irr':grni})
assert seen=={'Resources':4,'Order':1,'Receive':2,'Bill':2,'Transfer':1} and pools=={'W1':[30000,350],'W2':[12000,130]} and ap==329 and grni==50
expected={'Cash':251,'Bank':300,'Receivables':200,'Inventory':480,'ChequesReceivable':200,'Payables':-329,'ChequesPayable':-100,'Grni':-50,'OwnerCapital':-952,'SupplierAdvances':0,'CustomerAdvances':0,'Cogs':0,'InventoryLoss':0}
assert all(signed.get(mapping[role],0)==value for role,value in expected.items());assert sum(v for v in signed.values() if v>0)==1431 and -sum(v for v in signed.values() if v<0)==1431
report={'result':'passed','oracle':'Independent literal integer pools/cumulative receipt and bill costing/source movements/GL over raw SQLite; no Sarv helper','generation':generation,'payload_sha256':hashlib.sha256(payload.encode()).hexdigest(),'records':len(records),'journals':len(a['journals']),'stock_pools':pools,'AP_irr':ap,'GRNI_irr':grni,'trial_debit_irr':1431,'trial_credit_irr':1431,'timeline':timeline};out.write_text(json.dumps(report,indent=2));print(json.dumps(report))
