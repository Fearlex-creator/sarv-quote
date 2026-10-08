#!/usr/bin/env python3
import json,sqlite3
from collections import defaultdict
from pathlib import Path
import sys
run=Path(sys.argv[1]);base=json.loads((run/'literal/baseline/state.json').read_text());observations=[]
for phase in ['prepared','committed','race-identical-keys','race-different-keys']:
    directory=run/'faults'/phase;s=json.loads((directory/'final-state.json').read_text());a=s['accounting'];before=base['accounting']
    assert a['journals'][:-1]==before['journals'] and a['sales']==before['sales'] and a['core']==before['core']
    assert a['procurement']['records'][:-1]==before['procurement']['records']
    records=[r for r in a['procurement']['records'] if 'Return' in r['data']];assert len(records)==1
    r=records[0];d=r['data']['Return'];c=r['command']['Return']
    assert d['return_id']=='fault-return' and d['credit_id']=='fault-credit' and d['credit_state']=='Posted' and d['return_state']=='Returned'
    assert d['total']==45 and d['lines'][0]['quantity_thousandths']==3000 and d['lines'][0]['value']==45
    assert len(r['movements'])==1;move=r['movements'][0]
    assert move['before']['quantity_thousandths']==25000 and move['before']['value']==292
    assert move['after']['quantity_thousandths']==22000 and move['after']['value']==247
    assert move['direction']=='Out' and move['value']==45 and move['quantity_thousandths']==3000
    j=a['journals'][-1];assert j['id']==r['journal_id'] and j['event']=='procurement.return.v1' and j['date']=='2026-04-01'
    assert sum(l['debit'] for l in j['lines'])==sum(l['credit'] for l in j['lines'])==45
    mappings={m['role']:m['account_id'] for m in a['core']['setups'][0]['command']['mappings']}
    assert {l['account']['id']:(l['debit'],l['credit']) for l in j['lines']}=={mappings['Payables']:(45,0),mappings['Inventory']:(0,45)}
    net=defaultdict(int)
    for journal in a['journals']:
        assert sum(l['debit'] for l in journal['lines'])==sum(l['credit'] for l in journal['lines'])
        for l in journal['lines']:net[l['account']['id']]+=l['debit']-l['credit']
    assert [net[mappings[k]] for k in ['Inventory','Receivables','Payables','Grni','Cogs','Sales','SalesReturns']]==[377,299,-284,-50,58,-199,100]
    assert sum(v for v in net.values() if v>0)==-sum(v for v in net.values() if v<0)==1585
    assert len([x for x in a['audit'] if x['entity_id']==j['id']])==1
    aliases=[q for q in a['procurement']['requests'] if q['record_id']==r['id']];assert aliases and len({q['business_sha256'] for q in aliases})==1
    for profile in [directory,directory/'fresh-restore']:
        with sqlite3.connect(f'file:{profile}/sarv.sqlite3?mode=ro',uri=True) as db:
            assert db.execute('PRAGMA integrity_check').fetchone()[0]=='ok'
            assert json.loads(db.execute('SELECT payload FROM state WHERE id=1').fetchone()[0])==s
    observations.append({'phase':phase,'unchanged_complete_m5_prefix':True,'one_coupled_return_credit_journal_movement_audit':True,'raw_inventory':377,'raw_ap':284,'raw_tb':1585,'full_sqlite_and_restored_state_equality':True})
(run/'independent-raw-faults.json').write_text(json.dumps({'status':'PASS','cases':observations,'sarv_reports_used':False},indent=2)+'\n')
print('PASS4 raw fault/race States, immutable prefixes, exact linked one effect, 8 SQLite profiles integrity/equality')
