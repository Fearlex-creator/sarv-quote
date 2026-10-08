#!/usr/bin/env python3
"""Independent raw performance-fixture qualification; no Sarv/report calls."""
import hashlib,json,sqlite3,sys
from collections import defaultdict
from pathlib import Path
root=Path(sys.argv[1]);out=Path(__file__).with_name('performance-dataset-inspection');out.mkdir(exist_ok=True)
def load(p):return json.loads(p.read_text())
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
original=load(root/'original-schema6-state.json');initialized=load(root/'initialized-schema7-state.json');expected=dict(original);expected['schema']=7;assert initialized==expected
actual_original=load(Path(__file__).with_name('diagnostics')/'old-m5-helper/actual-old-M5-state.json');assert original==actual_original and original['schema']==6 and len(original['quotes'])==1
manifest=load(root/'dataset-manifest.json');assert all(sha(Path(p))==d['sha256'] for p,d in manifest['files'].items())
def prefix(before,after):
    if isinstance(before,list):assert after[:len(before)]==before
    elif isinstance(before,dict):
        for k,v in before.items():prefix(v,after[k])
    else:assert after==before
literal={'main-seed':(25,292,422,329,50,1630),'main-posted':(18,187,317,224,50,1525),'return20-billed':(45,592,722,629,50,1930),'return20-unbilled':(45,592,722,329,350,1930),'return200-billed':(225,3292,3422,3329,50,4630),'source120-billed':(145,2092,2222,2129,50,3430),'source120-unbilled':(145,2092,2222,329,1850,3430)}
results=[]
for name,(q1,v1,inv,ap_expected,grni_expected,tb_expected) in literal.items():
    state=load(root/name/'state.json');recipe=load(root/name/'recipe.json')['recipe'];a=state['accounting'];assert state['schema']==7
    for key in ['catalogs','customers','quotes','draft','settings']:assert state[key]==original[key]
    prefix(original['accounting'],a);assert a['sales']==original['accounting']['sales'] and a['core']==original['accounting']['core']
    maps={m['role']:m['account_id'] for m in a['core']['setups'][0]['command']['mappings']};net=defaultdict(int)
    for j in a['journals']:
        assert sum(l['debit'] for l in j['lines'])==sum(l['credit'] for l in j['lines'])
        for l in j['lines']:net[l['account']['id']]+=l['debit']-l['credit']
    assert [net[maps[r]] for r in ['Inventory','Payables','Grni','Receivables','Sales','SalesReturns','Cogs']]==[inv,-ap_expected,-grni_expected,299,-199,100,58]
    assert sum(v for v in net.values() if v>0)==-sum(v for v in net.values() if v<0)==tb_expected
    o=a['core']['openings'][0]['command'];items={i['id']:i for i in o['items']};pools=defaultdict(lambda:[0,0])
    for x in o['stock']:
        item=items[x['item_id']];num,den=1,1
        if x['unit_id']!=item['base_unit_id']:
            r=next(r for r in item['packs'] if r['unit_id']==x['unit_id']);num,den=r['numerator'],r['denominator']
        assert x['quantity_thousandths']*num%den==0;pools[x['item_id'],x['warehouse_id']][0]+=x['quantity_thousandths']*num//den;pools[x['item_id'],x['warehouse_id']][1]+=x['value']
    records=a['procurement']['records']+a['sales']['records'];assert len({r['sequence'] for r in records})==len(records)
    for r in sorted(records,key=lambda r:r['sequence']):
        for m in r['movements']:
            key=m['item_id'],m['warehouse_id'];assert pools[key]==[m['before']['quantity_thousandths'],m['before']['value']]
            sign=1 if m['direction']=='In' else -1;pools[key]=[pools[key][0]+sign*m['quantity_thousandths'],pools[key][1]+sign*m['value']]
            assert pools[key]==[m['after']['quantity_thousandths'],m['after']['value']] and all(v>=0 for v in pools[key]) and (pools[key][0]!=0 or pools[key][1]==0)
    assert pools['ITEM-A','W1']==[q1*1000,v1] and pools['ITEM-A','W2']==[12000,130] and sum(v for q,v in pools.values())==inv
    receipts={};bill_lines={};payables={};grni=sum(x['value'] for x in o['grni']);ap=sum(x['remaining'] for x in o['open_items'] if x['side']=='Payable')
    for r in a['procurement']['records']:
        kind,d=next(iter(r['data'].items()))
        if kind=='Receive':
            for l in d['lines']:assert l['id'] not in receipts;receipts[l['id']]={'original_q':l['quantity_thousandths'],'original_v':l['value'],'q':l['quantity_thousandths'],'v':l['value'],'source':r['id'],'item':l['item_id'],'warehouse':l['warehouse_id']}
            grni+=d['total']
        elif kind=='Bill':
            payables[r['id']]=d['total'];ap+=d['total'];grni-=d['total']
            for l in d['lines']:
                if l['receipt_line_id'] in receipts:
                    source=receipts[l['receipt_line_id']];source['q']-=l['quantity_thousandths'];source['v']-=l['value'];assert source['q']>=0 and source['v']>=0
                bill_lines[l['id']]={'receipt':l['receipt_line_id'],'original_q':l['quantity_thousandths'],'original_v':l['value'],'q':l['quantity_thousandths'],'v':l['value'],'bill':r['id']}
        elif kind=='Return':
            assert d['return_id']!=d.get('credit_id') and d['return_state']=='Returned'
            for l in d['lines']:
                if l['bill_line_id'] is not None:
                    source=bill_lines[l['bill_line_id']];assert source['receipt']==l['receipt_line_id'] and source['bill']==d['bill_id'];source['q']-=l['quantity_thousandths'];source['v']-=l['value'];assert source['q']>=0 and source['v']>=0
                else:
                    source=receipts[l['receipt_line_id']];source['q']-=l['quantity_thousandths'];source['v']-=l['value'];assert source['q']>=0 and source['v']>=0
            if d.get('bill_id') is not None:payables[d['bill_id']]-=d['total'];ap-=d['total'];assert d['credit_state']=='Posted'
            else:grni-=d['total'];assert d.get('credit_id') is None
    assert ap==ap_expected and grni==grni_expected and all(v>=0 for v in payables.values())
    if name.startswith('return'):
        n=200 if name=='return200-billed' else 20;billed=name.endswith('-billed')
        assert recipe['line_count']==n and len(recipe['receipt_line_ids'])==n and recipe['original_total_irr']==n*15
        for rid in recipe['receipt_line_ids']:
            r=receipts[rid];assert (r['original_q'],r['original_v'],r['q'],r['v'])==(1000,15,0 if billed else 1000,0 if billed else 15) and r['source']==recipe['receipt_record_id']
        if billed:
            assert len(recipe['bill_line_ids'])==n
            for bid,rid in zip(recipe['bill_line_ids'],recipe['receipt_line_ids']):b=bill_lines[bid];assert (b['original_q'],b['original_v'],b['q'],b['v'])==(1000,15,1000,15) and b['receipt']==rid and b['bill']==recipe['bill_record_id']
        assert q1-n==25 and v1-15*n==292 and inv-15*n==422 and (ap_expected-15*n if billed else ap_expected)==329 and (grni_expected if billed else grni_expected-15*n)==50 and tb_expected-15*n==1630
    if name.startswith('source120'):
        sources=recipe['sources_in_posting_order'];assert len(sources)==120 and sum(s['query_group']=='PERF-A' for s in sources)==sum(s['query_group']=='PERF-B' for s in sources)==60
        assert recipe['page_size']==25 and recipe['filtered_pages']==3 and recipe['unfiltered_synthetic_pages']==5
        for s in sources:
            r=receipts[s['receipt_line_id']];assert (r['original_q'],r['original_v'])==(1000,15)
            if name.endswith('-billed'):
                b=bill_lines[s['bill_line_id']];assert b['receipt']==s['receipt_line_id'] and b['bill']==s['record_id'] and (b['q'],b['v'])==(1000,15) and (r['q'],r['v'])==(0,0)
            else:assert r['source']==s['record_id'] and (r['q'],r['v'])==(1000,15)
    with sqlite3.connect(f'file:{Path.cwd()}/artifacts/performance-m6/dataset-candidate2/{name}/profile/sarv.sqlite3?mode=ro',uri=True) as db:
        assert db.execute('PRAGMA integrity_check').fetchone()[0]=='ok' and json.loads(db.execute('SELECT payload FROM state WHERE id=1').fetchone()[0])==state
    results.append({'dataset':name,'whole_original_state_prefix':True,'raw_journal_stock_source_bounds':True,'inventory':inv,'ap':ap_expected,'grni':grni_expected,'tb':tb_expected,'w1_q':q1,'w1_v':v1,'state_sha256':sha(root/name/'state.json')})
(out/'result.json').write_text(json.dumps({'status':'PASS','sarv_reports_used':False,'datasets':results,'all38_manifest_hashes_match':True,'original_genuine6_and_only_schema7_migration':True},indent=2)+'\n');print('PASS7 actual raw datasets/full quotation+financial prefixes,38 hashes, literal stocks/journals/AP/GRNI/TB and original source remaining bounds')
