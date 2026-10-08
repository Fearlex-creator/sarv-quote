#!/usr/bin/env python3
"""Independent integer reconstruction of procurement persisted SQLite, no Sarv helper."""
import json,sqlite3,sys,hashlib
from pathlib import Path
profile=Path(sys.argv[1]);out=Path(sys.argv[2])
with sqlite3.connect(f'file:{profile}/sarv.sqlite3?mode=ro',uri=True) as d:
 generation,payload=d.execute('select generation,payload from state where id=1').fetchone();assert d.execute('pragma integrity_check').fetchone()[0]=='ok'
s=json.loads(payload);assert s['schema']==5;a=s['accounting'];opening=a['core']['openings'][0];command=opening['command'];records=a['procurement']['records'];mappings={m['role']:m['account_id'] for m in opening['frozen_setup']['command']['mappings']}
assert len(a['core']['openings'])==1
pools={'w1':[24000,240],'w2':[6000,60]};receipts={'r1':[5000,50,5000,50]};order=None;received_q=received_v=0;ap=149;grni=50;timelines=[];previous_sequence=0
for r in records:
 kind,input=next(iter(r['command'].items()));ctx=input['context'];assert r['sequence']>previous_sequence;previous_sequence=r['sequence'];data=r['data'][kind]
 if kind=='Order':
  assert data['total']==180 and data['lines'][0]['base_quantity_thousandths']==12000 and data['lines'][0]['gross']==180 and data['lines'][0]['net']==180
  order=r['id'];assert r['journal_id'] is None and r['movements']==[]
 elif kind=='Receive':
  assert input['order_id']==order;line=data['lines'][0];q=line['quantity_thousandths'];v=line['value'];assert (q,v) in [(5000,75),(7000,105)]
  received_q+=q;target=(180*received_q*2+12000)//(2*12000);assert v==target-received_v;received_v=target;receipts[line['id']]=[q,v,q,v];pools[line['warehouse_id']][0]+=q;pools[line['warehouse_id']][1]+=v;grni+=v;assert r['journal_id'] and len(r['movements'])==1
 elif kind=='Bill':
  v=0
  for line in data['lines']:
   rem=receipts[line['receipt_line_id']];q=line['quantity_thousandths'];original_q,original_v=rem[2:];cumulative=original_q-rem[0]+q;assert 0<cumulative<=original_q
   target=(original_v*cumulative*2+original_q)//(2*original_q);increment=target-(original_v-rem[1]);assert increment==line['value'];rem[0]-=q;rem[1]-=increment;v+=increment
  assert v==input['supplied_total']==data['total'];ap+=v;grni-=v;assert r['movements']==[] and r['journal_id']
 elif kind=='Transfer':
  assert r['journal_id'] is None and len(r['movements'])==2
  q=data['quantity_thousandths'];src=pools[input['source_warehouse_id']];dst=pools[input['target_warehouse_id']];cost=src[1] if q==src[0] else (src[1]*q*2+src[0])//(2*src[0]);assert cost==data['value']==70 and q==6000
  before_sum=sum(v[1] for v in pools.values());src[0]-=q;src[1]-=cost;dst[0]+=q;dst[1]+=cost;assert sum(v[1] for v in pools.values())==before_sum
 elif kind=='Resources':assert r['journal_id'] is None and r['movements']==[]
 else:raise AssertionError(kind)
 if kind in ['Receive','Bill','Transfer']:
  signed={}
  for j in a['journals']:
   if j['company_id']==ctx['company_id'] and j['date']<=ctx['date']:
    for l in j['lines']:signed[l['account']['id']]=signed.get(l['account']['id'],0)+l['debit']-l['credit']
  assert signed[mappings['Inventory']]==sum(v[1] for v in pools.values()) and -signed[mappings['Payables']]==ap and -signed.get(mappings['Grni'],0)==grni
  assert sum(v[1] for v in receipts.values())==grni
  timelines.append({'event':ctx['source_id'],'date':ctx['date'],'inventory_irr':sum(v[1] for v in pools.values()),'AP_irr':ap,'GRNI_irr':grni})
assert pools=={'w1':[30000,350],'w2':[12000,130]} and ap==379 and grni==0
assert len(records)==8 and len(a['journals'])==6
assert sum(v[0] for v in pools.values())==42000 and sum(v[1] for v in pools.values())==480
r={'result':'passed','schema':5,'generation':generation,'payload_sha256':hashlib.sha256(payload.encode()).hexdigest(),'oracle':'Python standard-library integers over persisted raw SQLite; explicit literal fixture, no Sarv helper','records':len(records),'journals':len(a['journals']),'stock_pools':pools,'AP_irr':ap,'GRNI_irr':grni,'timeline':timelines}
out.write_text(json.dumps(r,indent=2));print(json.dumps(r))
