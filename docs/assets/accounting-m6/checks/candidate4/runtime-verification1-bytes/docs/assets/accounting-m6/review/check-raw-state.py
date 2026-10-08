#!/usr/bin/env python3
"""Read exported raw State. Independent integer balances; no Sarv imports/reports."""
import argparse,json,sqlite3
from collections import defaultdict
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('run',type=Path);a=p.parse_args()
expected=json.loads(Path(__file__).with_name('literal-expected.json').read_text())
c=expected['constants'];results=[]
for branch,name in [('billed','billed_branch'),('unbilled','unbilled_interleaved_branch')]:
    state=json.loads((a.run/'literal'/branch/'state.json').read_text());s=state['accounting'];o=s['core']['openings'][0]['command'];company=o['company_id']
    mappings={m['role']:m['account_id'] for m in s['core']['setups'][0]['command']['mappings']}
    items={i['id']:i for i in o['items']}
    for j in s['journals']:
        assert sum(x['debit'] for x in j['lines'])==sum(x['credit'] for x in j['lines'])
        assert len(j['lines'])>=2 and all((x['debit']>0) != (x['credit']>0) for x in j['lines'])
    for row in expected[name]:
        date=row['date'];net=defaultdict(int)
        for j in s['journals']:
            if j['company_id']==company and j['date']<=date:
                for l in j['lines']:net[l['account']['id']]+=l['debit']-l['credit']
        def role(r):return net[mappings[r]]
        assert role('Inventory')==row['inventory'] and -role('Payables')==row['ap'] and -role('Grni')==row['grni']
        assert role('Receivables')==299 and role('Cogs')==58 and -role('Sales')==199 and role('SalesReturns')==100
        assert sum(v for v in net.values() if v>0)==-sum(v for v in net.values() if v<0)==row['tb']
        pools=defaultdict(lambda:[0,0])
        for x in o['stock']:
            item=items[x['item_id']];num,den=1,1
            if x['unit_id']!=item['base_unit_id']:
                ratio=next(r for r in item['packs'] if r['unit_id']==x['unit_id']);num,den=ratio['numerator'],ratio['denominator']
            assert x['quantity_thousandths']*num%den==0
            pools[x['item_id'],x['warehouse_id']][0]+=x['quantity_thousandths']*num//den
            pools[x['item_id'],x['warehouse_id']][1]+=x['value']
        records=[r for mod in ['procurement','sales'] for r in s.get(mod,{}).get('records',[]) if next(iter(r['command'].values()))['context']['company_id']==company and next(iter(r['command'].values()))['context']['date']<=date]
        for r in sorted(records,key=lambda r:r['sequence']):
            for m in r['movements']:
                key=m['item_id'],m['warehouse_id'];before=pools[key]
                assert before==[m['before']['quantity_thousandths'],m['before']['value']]
                sign=1 if m['direction']=='In' else -1
                after=[before[0]+sign*m['quantity_thousandths'],before[1]+sign*m['value']]
                assert after==[m['after']['quantity_thousandths'],m['after']['value']]
                assert after[0]>=0 and after[1]>=0 and (after[0]!=0 or after[1]==0)
                pools[key]=after
        assert pools['item-a','w1']==[row['w1_q']*1000,row['w1_v']] and pools['item-a','w2']==[12000,130]
        assert sum(x[1] for x in pools.values())==row['inventory']
        ap=sum(x['remaining'] for x in o['open_items'] if x['side']=='Payable');grni=sum(x['value'] for x in o['grni']);ar=sum(x['remaining'] for x in o['open_items'] if x['side']=='Receivable');sales=returns=cogs=0
        for r in records:
            kind,d=next(iter(r['data'].items()))
            if r in s['procurement']['records']:
                if kind=='Receive':grni+=d['total']
                elif kind=='Bill':grni-=d['total'];ap+=d['total']
                elif kind=='Return':
                    assert d['return_id']!=d.get('credit_id')
                    if d.get('bill_id') is not None:
                        ap-=d['total'];assert d['credit_id'] and d['credit_state']=='Posted' and d['return_state']=='Returned'
                    else:grni-=d['total'];assert d.get('credit_id') is None and d.get('credit_state') is None
            elif kind=='Fulfill':ar+=d['total'];sales+=d['total'];cogs+=d['cost']
            elif kind=='Return':ar-=d['total'];returns+=d['total'];cogs-=d['cost']
        assert ap==row['ap'] and grni==row['grni'] and ar==299 and sales==199 and returns==100 and cogs==58 and sales-returns-cogs==41
        results.append({'branch':branch,'date':date,'inventory':role('Inventory'),'ap':ap,'grni':grni,'tb':row['tb'],'raw_movements_and_open_item_detail':True})
    profile=a.run/'literal'/branch/'profile'/'sarv.sqlite3'
    with sqlite3.connect(f'file:{profile}?mode=ro',uri=True) as db:
        assert db.execute('PRAGMA integrity_check').fetchone()[0]=='ok'
        assert json.loads(db.execute('SELECT payload FROM state WHERE id=1').fetchone()[0])==state
result={'status':'PASS','raw_literal_cutoffs':len(results),'sarv_reports_used_for_expected_values':False,'observations':results}
(a.run/'independent-raw-literals.json').write_text(json.dumps(result,indent=2)+'\n')
print('PASS11 raw literal cutoffs, exact journal/stock/source detail and live SQLite State equality')
