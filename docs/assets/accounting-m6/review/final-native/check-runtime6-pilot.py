"""Independent read-only pilot checks; no Sarv imports or GUI execution."""
import hashlib
import json
import math
from pathlib import Path
import re
import sqlite3

ROOT = Path('/workspace/sarv-quote')
RAW = ROOT/'artifacts/performance-m6/candidate6-runtime6-pilot1'
P = ROOT/'docs/assets/accounting-m6/performance/candidate6-runtime6-pilot1-passed'
OUT = ROOT/'docs/assets/accounting-m6/review/final-native'
D = json.loads((RAW/'observer/performance.json').read_text())
checks=[]
def check(name, ok, actual=None):
    checks.append({'name':name,'passed':bool(ok),'actual':actual})
    assert ok,name

def controls(state):
    a=state['accounting'];maps={m['account_id']:m['role'] for m in a['core']['setups'][0]['command']['mappings']}
    result=dict.fromkeys(maps.values(),0)
    for journal in a['journals']:
        assert sum(x['debit'] for x in journal['lines'])==sum(x['credit'] for x in journal['lines'])
        for line in journal['lines']:
            result[maps[line['account']['id']]]+=line['debit']-line['credit']
    result['TB_debit']=sum(max(x,0)for x in result.values())
    result['TB_credit']=sum(max(-x,0)for x in result.values())
    return result

base=dict(Cash=251,Bank=300,Receivables=299,ChequesReceivable=200,Inventory=422,
 SupplierAdvances=0,FixedAssets=0,Payables=-329,ChequesPayable=-100,Grni=-50,
 CustomerAdvances=0,Loans=0,OwnerCapital=-952,OwnerDrawings=0,RetainedEarnings=0,
 Sales=-199,SalesReturns=100,Cogs=58,OperatingExpense=0,BankFees=0,InventoryLoss=0,
 TB_debit=1630,TB_credit=1630)
check('closed guarded pilot, not quiet final',D['result']=='measured' and D['development_pilot'] and D['all_provenance_guards_stable'])
check('paired production/observer/binary guards exact',D['provenance_before']==D['provenance_after'])
for k in ('source_files','verification_files'):
    check('current '+k+' hash exact',all(hashlib.sha256((ROOT/f).read_bytes()).hexdigest()==h for f,h in D['provenance_after'][k].items()))
check('immutable binary hash exact',hashlib.sha256(Path(D['binary']).read_bytes()).hexdigest()==D['binary_sha256'])
check('protected dataset byte hashes exact',D['dataset_files_before']==D['dataset_files_after'] and all(hashlib.sha256(Path(f).read_bytes()).hexdigest()==h for f,h in D['dataset_files_after'].items()))

digitmap=str.maketrans('۰۱۲۳۴۵۶۷۸۹','0123456789')
isolation=dict.fromkeys(map(ord,'\u2066\u2067\u2068\u2069\u200e\u200f'),None)
for name,n,billed in [('return20-billed',20,True),('return20-unbilled',20,False),('return200-billed',200,True)]:
    w=D['return_workflows'][name];folder=RAW/'observer'/f'{name}-workflow'
    before=json.loads((folder/'before-state.json').read_text());after=json.loads((folder/'after-state.json').read_text())
    with sqlite3.connect(f'file:{folder}/sarv.sqlite3?mode=ro',uri=True)as conn:
        saved=json.loads(conn.execute('select payload from state where id=1').fetchone()[0])
    check(name+' stopped raw SQLite/JSON exact',saved==after)
    check(name+' restart closed',w['result']=='measured' and w['restart_complete_state_exact'])
    expected_before=dict(base,Inventory=422+15*n,TB_debit=1630+15*n,TB_credit=1630+15*n)
    expected_before['Payables'if billed else'Grni']-=15*n
    check(name+' independent all-role before literal',controls(before)==expected_before,controls(before))
    check(name+' independent all-role after literal',controls(after)==base,controls(after))
    for k in before:
        if k!='accounting':check(name+' unchanged whole-State '+k,before[k]==after[k])
    a,b=after['accounting'],before['accounting']
    for k in ('companies','periods','accounts','core','sales'):check(name+' unchanged accounting '+k,a[k]==b[k])
    for k in ('journals','requests','audit'):check(name+' immutable '+k+' prefix',a[k][:len(b[k])]==b[k])
    for k in ('records','requests'):check(name+' immutable procurement '+k+' prefix',a['procurement'][k][:len(b['procurement'][k])]==b['procurement'][k])
    records=a['procurement']['records'][len(b['procurement']['records']):]
    check(name+' exactly one new return',len(records)==1)
    record=records[0];data=record['data']['Return'];command=record['command']['Return']
    check(name+' correct return/credit identities',data['return_state']=='Returned' and bool(data['credit_id'])==billed and (data['credit_state']=='Posted'if billed else data['credit_state']is None) and bool(data['bill_id'])==billed)
    check(name+' exact original cost/count',len(data['lines'])==n and data['total']==15*n and len(record['movements'])==n and all(x['quantity_thousandths']==1000 and x['value']==15 and x['item_id']=='ITEM-A' and x['warehouse_id']=='W1'for x in data['lines']))
    receipts=[f'M6-PERF-{name}-GR-L{i:03}'for i in range(1,n+1)]
    bills=[f'M6-PERF-{name}-BILL-L{i:03}'for i in range(1,n+1)]
    check(name+' exact all source receipt/bill lines',[x['receipt_line_id']for x in data['lines']]==receipts and [x['bill_line_id']for x in data['lines']]==(bills if billed else[None]*n))
    check(name+' exact all real input observations',len(w['input_samples'])==n and [x['receipt_line_id']for x in w['input_samples']]==receipts and all(x['input_ready_ms']>=0 for x in w['input_samples']))
    check(name+' unique generated line/movement IDs',len(set(x['id']for x in data['lines']))==n and len(set(x['id']for x in record['movements']))==n)
    for i,m in enumerate(record['movements']):
        assert m['direction']=='Out'and m['quantity_thousandths']==1000 and m['value']==15 and m['source_line_id']==data['lines'][i]['id']
        assert m['before']['quantity_thousandths']==(25+n-i)*1000 and m['after']['quantity_thousandths']==(24+n-i)*1000
        assert m['before']['value']==292+15*(n-i)and m['after']['value']==292+15*(n-i-1)
    check(name+' every original pool transition exact',True)
    journal=next(j for j in a['journals']if j['id']==record['journal_id']);mapping={m['account_id']:m['role']for m in a['core']['setups'][0]['command']['mappings']}
    check(name+' exact source journal',[(mapping[x['account']['id']],x['debit'],x['credit'])for x in journal['lines']]==[('Payables'if billed else'Grni',15*n,0),('Inventory',0,15*n)])
    rows=[x for page in w['preview_pages']for x in page['current_rows']]
    check(name+' exact detail page markers/count',len(w['preview_pages'])==math.ceil(2*n/25) and len(rows)==2*n and all(page['page']==i and page['marker']==f'جزئیات برگشت {25*(i-1)+1} تا {min(25*i,2*n)} از {2*n} • صفحهٔ {i}'and page['attached_detail_labels']==len(page['current_rows'])for i,page in enumerate(w['preview_pages'],1)))
    for i,row in enumerate(rows[:n]):
        identifiers=re.findall('\u2068([^\u2069]+)\u2069',row)
        assert receipts[i]in identifiers and(not billed or bills[i]in identifiers)
        text=row.translate(isolation).translate(digitmap)
        assert '• مقدار پایه 1.000 • بهای اصلی 15 ریال • منبع 'in text
    for i,row in enumerate(rows[n:]):
        assert data['lines'][i]['id']in re.findall('\u2068([^\u2069]+)\u2069',row)
        text=row.translate(isolation).translate(digitmap)
        assert f'• مقدار قبل {25+n-i}.000 • مقدار بعد {24+n-i}.000 • ارزش قبل {292+15*(n-i)} ریال • ارزش بعد {292+15*(n-i-1)} ریال'in text
    check(name+' every source and movement detail exact',True)
    if n==200:
        check('200 exact next/previous pager samples',len(w['page_samples'])==14 and [x['page']for x in w['page_samples']if x['direction']=='next']==list(range(2,9))and[x['page']for x in w['page_samples']if x['direction']=='previous']==list(range(7,0,-1))and all(x['attached_quantity_entries']==25 and x['first_receipt_line_id']==receipts[25*(x['page']-1)]for x in w['page_samples']))
        check('200 hidden invalid input/focus retained and later corrected',w['hidden_invalid_last_row_retained_focused']and w['first_page_input_retained_after_hidden_error'])
    for phase in w['strict_phase_checks']:
        gaps=phase['gaps_ms'];check(name+' '+phase['phase']+' raw phase recomputation',gaps and len(gaps)==phase['ticks']+1 and abs(sum(gaps)-phase['elapsed_ms'])<1e-5 and max(gaps)==phase['max_gap_recomputed_ms']and phase['terminal_gap_ms']==gaps[-1]and phase['budget_ms']==50 and phase['measurable_accepted']and phase['strict_status']==('pass'if max(gaps)<=50 else'miss'))

def quantiles(values):
    v=sorted(values)
    return {'n':len(v),**{k:v[math.ceil(len(v)*q)-1]for k,q in [('p50',.5),('p95',.95),('p99',.99)]},'worst':v[-1],'method':'nearest rank'}
for name,s in D['source_picker_search'].items():
    check(name+' complete retained raw search arrays',s['result']=='measured'and len(s['samples'])==2 and len(s['filtered_pages'])==2 and len(s['identity_checks'])==2 and s['durable_state_unchanged'])
    check(name+' independent search quantiles',s['focused']==quantiles([x['focused_usable_ms']for x in s['samples']])and s['visible']==quantiles([x['correct_visible_ms']for x in s['samples']]))
for name,s in D['startup'].items():
    check(name+' complete pilot startup raw arrays',s['result']=='measured'and len(s['samples'])==2 and all(x['rss_observer_samples']==len(x['rss_observer_raw_kib'])and x['rss_observer_samples']>0 for x in s['samples'])and s['repeated_startup_ms']==quantiles([s['samples'][1]['startup_ms']]))
check('two exact retired-window errors recorded, no unexpected',len(D['x11_error_observations'])==2 and all(e['retired_pid_window_rejected']and e['owned_private_display']and e['error_code']==3 and e['request_code']==20 and e['minor_code']==0 and e['scope']['property']=='_NET_WM_PID'and e['scope']['phase']=='owned-private-window-discovery'and e['resource_id']==e['scope']['window']for e in D['x11_error_observations']))
check('322 complete-boundary checkpoints retained',len(D['completed_checkpoint_boundaries'])==322)
M=json.loads((P/'portable-manifest.json').read_text())
check('portable raw inventory exact',all((P/f).stat().st_size==v['bytes']and hashlib.sha256((P/f).read_bytes()).hexdigest()==v['sha256']for f,v in M['copied_raw_files'].items()),len(M['copied_raw_files']))
inventory=json.loads((P/'evidence-inventory.json').read_text())
check('all80 portable inventory hashes exact',len(inventory['files'])==80 and all((P/f).stat().st_size==v['bytes']and hashlib.sha256((P/f).read_bytes()).hexdigest()==v['sha256']for f,v in inventory['files'].items()))
folders=[x for x in (P/'observer').iterdir()if x.is_dir()and(x/'stopped-state.json').exists()]
check('all11 stopped SQLite/portable States exact',len(folders)==11)
for folder in folders:
    with sqlite3.connect(f'file:{RAW}/observer/{folder.name}/sarv.sqlite3?mode=ro',uri=True)as conn:
        value=json.loads(conn.execute('select payload from state where id=1').fetchone()[0]);integrity=conn.execute('pragma integrity_check').fetchall()
    check(folder.name+' stopped State/integrity exact',value==json.loads((folder/'stopped-state.json').read_text())and integrity==[('ok',)])
inactive=[]
for record in D['owned_process_records']:
    path=Path(f'/proc/{record["pid"]}/stat')
    if not path.exists():inactive.append(record['pid']);continue
    text=path.read_text();parts=text[text.rfind(')')+2:].split()
    check('owned process inactive or different birth '+str(record['pid']),int(parts[19])!=record['starttime_ticks']or parts[0]=='Z')
    inactive.append(record['pid'])
check('all22 owned process records inactive',len(inactive)==22)
analysis=json.loads((P/'analysis.json').read_text());recomputed=[]
for name,s in D['source_picker_search'].items():
    for group,label,limit in [(s['samples'],'query',200),(s['filtered_pages'],'filtered-page',150)]:
        for field in ['correct_visible_ms','focused_usable_ms']:
            q=quantiles([x[field]for x in group]);recomputed.append({'scope':f'{name}:{label}:{field}','observed':q['p95'],'limit':limit,'quantiles':q,'status':'MISS'if q['p95']>limit else'PASS'})
s=D['return_workflows']['return200-billed']
for direction in ['next','previous']:
    for field in ['correct_visible_ms','focused_usable_ms']:
        q=quantiles([x[field]for x in s['page_samples']if x['direction']==direction]);recomputed.append({'scope':f'return200-billed:editor:{direction}:{field}','observed':q['p95'],'limit':150,'quantiles':q,'status':'MISS'if q['p95']>150 else'PASS'})
check('all12 descriptive strict misses independently reproduced',len(analysis['misses'])==12 and all(any(all(m[k]==x[k]for k in ['scope','observed','limit','quantiles','status'])for m in analysis['misses'])for x in recomputed)and all(x['status']=='MISS'for x in recomputed))
images={str(folder.name+'/'+name):hashlib.sha256((folder/name).read_bytes()).hexdigest()for folder in [(RAW/'observer/return200-billed-workflow')]for name in ['invalid-last-row-focused.png','last-preview-page.png','posted.png']}
result={'reviewer':'m6_final_native_review','all_checks_pass':True,'checks':checks,'raw_receipt_sha256':hashlib.sha256((RAW/'observer/performance.json').read_bytes()).hexdigest(),'personally_viewed_200_images':images,'independent_descriptive_misses':recomputed,'functional_scope':'Exact retained native20/20/200 inputs, details, post, source lineage, old prefixes, pager, restart and pilot startup arrays','performance_accepted':False,'invalid_field_settled_visibility_accepted':False,'invalid_visibility_limit':'L200 quantity text is hidden at fixed footer in immediate captured image; FOCUSED/text state does not establish settled pixels/clip','quiet_final':False}
(OUT/'runtime6-pilot-independent-result.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
print('PASS',len(checks),'independent assertions; performance and invalid-field settled visibility remain unaccepted')
