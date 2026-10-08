#!/usr/bin/env python3
"""Independent stopped/read-only partial-epoch review; standard library only."""
import hashlib
import json
import math
import sqlite3
from pathlib import Path

ROOT=Path(__file__).resolve().parents[5]
BASE=ROOT/'docs/assets/accounting-m6/performance/candidate8-runtime2-pilot1-partial'
def read(path): return json.loads(path.read_text())
def digest(path): return hashlib.sha256(path.read_bytes()).hexdigest()
raw=read(BASE/'raw/observer/performance.partial.json')
coord=read(BASE/'raw/coordinator.json')
analysis=read(BASE/'raw/analysis.json')
dataset=read(ROOT/'docs/assets/accounting-m6/performance/dataset-candidate2/dataset-manifest.json')
checks=[]
def check(name,condition):
    assert condition,name
    checks.append(name)

inventory=read(BASE/'evidence-inventory.json')
check('20 portable evidence files inventoried',len(inventory['files'])==inventory['portable_file_count']==20)
for path,record in inventory['files'].items():
    content=(BASE/path).read_bytes()
    check('portable exact '+path,len(content)==record['bytes'] and hashlib.sha256(content).hexdigest()==record['sha256'])
portable=read(BASE/'portable-manifest.json')
origin=Path(portable['raw_root'])
for path,record in portable['raw_files'].items():
    check('raw preserved exact '+path,digest(BASE/'raw'/path)==digest(origin/path)==record['sha256'])
check('no full observer report',not (BASE/'raw/observer/performance.json').exists() and not (origin/'observer/performance.json').exists())
check('observer exit139 and exact frame unavailable',coord['observer_exit_code']==139 and analysis['exact_crash_frame']=='unavailable')
check('pilot invocation not final',coord['development_pilot'] and '--pilot' in coord['command'] and raw['development_pilot'])
before,after=coord['guard_before'],coord['guard_after']
check('coordinator before/after entire guards exact',before==after and coord['all_coordinator_guards_stable'])
for key,count in [('production_files',36),('verification_files',74),('dataset_files',38),('seed_files',2)]:
    check('guard count '+key,len(before[key])==count)
    for name,expected in before[key].items():
        p=Path(dataset[name]) if key=='seed_files' else (Path(name) if Path(name).is_absolute() else ROOT/name)
        check('current frozen hash '+name,digest(p)==(expected['sha256'] if isinstance(expected,dict) else expected))
receipt=read(ROOT/'docs/assets/accounting-m6/performance/candidate8-runtime2-coordinator-receipt.json')
check('receipt hash exact',before['receipt_sha256']==digest(ROOT/'docs/assets/accounting-m6/performance/candidate8-runtime2-coordinator-receipt.json')=='98ffd3f333f439ee4beb70ba82d28ae425848eef0e325cfb0cdd7acb4a88dd03')
check('binary remains exact',digest(Path(receipt['binary']))==before['binary_sha256']==receipt['binary_sha256'])
check('canonical observer exact portable source',digest(ROOT/'scripts/native-performance-M6.py')==digest(BASE/'observer-source.py')=='cb145aed5a13d289f8222083f3d23e67ef9c95759745c2d4aea9ecc391945408')
check('production provenance exact',before['production_sha256']==receipt['production_sha256']=='11b48de2e85b9465a22f4a618e4675801b1db08a5a2c35b35cb892e8ea381cf3')
scopes=[]
check('exact two completed source cohorts',set(raw['source_picker_search'])=={'source120-billed','source120-unbilled'})
for name,record in raw['source_picker_search'].items():
    recipe=read(Path(dataset['datasets'][name]['recipe']))['recipe']
    references={x['query']:x['pages'] for x in recipe['expected_filter_first_references']}
    check(name+' exact n2 queries and 2 sequential pages',len(record['samples'])==2 and [x['query'] for x in record['samples']]==['PERF-A','PERF-B'] and [x['page'] for x in record['filtered_pages']]==[2,3])
    check(name+' exact zero warmup / readonly state',record['result']=='measured' and record['zero_result_exact'] and record['durable_state_unchanged'])
    for row in record['samples']:
        check(name+' query reference '+row['query'],row['expected_reference']==references[row['query']][0])
    for row in record['identity_checks']:
        check(name+' identity/current/focus '+row['query'],row['exact_count']==60 and row['current_parent_child_membership'] and row['visible_sensitive_geometry'] and row['actual_focus'] and row['expected_reference']==references[row['query']][0])
    for row in record['filtered_pages']:
        check(name+' exact page reference '+str(row['page']),row['expected_reference']==references['PERF-B'][row['page']-1])
    selected=next(x for x in recipe['sources_in_posting_order'] if x['reference']==references['PERF-B'][2])
    check(name+' selected exact original receipt',record['selected_source_receipt_line_id']==selected['receipt_line_id'])
    for stream,rows,limit in [('query',record['samples'],200),('filtered-page',record['filtered_pages'],150)]:
        for metric in ('correct_visible_ms','focused_usable_ms'):
            values=sorted(x[metric] for x in rows)
            q={f'p{int(p*100)}':values[math.ceil(p*len(values))-1] for p in (.50,.95,.99)}
            scope=f'{name}:{stream}:{metric}'
            declared=next(x for x in analysis['checks'] if x['scope']==scope)
            check(scope+' unaltered nearest rank/miss',declared['limit']==limit and declared['observed']==q['p95'] and declared['quantiles']['n']==2 and all(declared['quantiles'][k]==v for k,v in q.items()) and declared['quantiles']['worst']==values[-1] and declared['status']=='MISS' and q['p95']>limit)
            scopes.append(dict(scope=scope,raw_values_ms=[x[metric] for x in rows],budget_ms=limit,p95_ms=q['p95'],status='MISS'))
check('eight diagnostics fail unchanged budgets',len(scopes)==len(analysis['checks'])==len(analysis['misses'])==8 and analysis['strict_measured_gate']=='FAILED')
check('missing current workflows/startups/phases explicit',raw['result']=='incomplete' and raw['return_workflows']=={} and raw['startup']=={} and analysis['all_native_phase_records_with_terminal_gaps']==[] and not analysis['epoch_complete_and_guarded'] and not analysis['complete200_functional_acceptance'] and not analysis['final_n10_n20_epoch_run'])
check('last checkpoint before any source opening workflow',raw['completed_checkpoint_boundaries'][-1]['event']=='source120-unbilled:completed-source-picker' and not any('source-opening' in x['event'] for x in raw['completed_checkpoint_boundaries']))
check('no Python traceback',b'Traceback (most recent call last)' not in (BASE/'raw/coordinator-observer.log').read_bytes())

states=[]
for profile in read(BASE/'stopped-profile-checks.json')['profiles']:
    original=Path(dataset['datasets'][profile['dataset']]['profile'])/'sarv.sqlite3'
    stopped=Path(profile['stopped_profile'])
    def stored(path):
        with sqlite3.connect(path.as_uri()+'?mode=ro',uri=True) as db:
            check(str(path)+' integrity',db.execute('PRAGMA integrity_check').fetchone()==('ok',))
            return db.execute('SELECT generation,payload FROM state WHERE id=1').fetchone()
    a,b=stored(original),stored(stopped)
    check(profile['dataset']+' exact generation and payload',a==b and a[0]==1)
    payload=b[1].encode() if isinstance(b[1],str) else b[1]
    check(profile['dataset']+' retained payload SHA',hashlib.sha256(payload).hexdigest()==profile['payload_sha256'])
    state=json.loads(payload)
    check(profile['dataset']+' portable full State exact',state==read(BASE/profile['portable_state']))
    # Derive balances directly from raw journals, independent of app calculators.
    bal={}
    for journal in state['accounting']['journals']:
        check(profile['dataset']+' balanced raw journal '+journal['id'],sum(x['debit'] for x in journal['lines'])==sum(x['credit'] for x in journal['lines']))
        for line in journal['lines']:
            key=line['account']['id'];bal[key]=bal.get(key,0)+line['debit']-line['credit']
    roles={x['role']:x['account_id'] for x in state['accounting']['core']['setups'][0]['command']['mappings']}
    totals={k:bal.get(v,0) for k,v in roles.items()}
    inv,ap,grni,tb={'source120-billed':(2222,-2129,-50,3430),'source120-unbilled':(2222,-329,-1850,3430),'return20-billed':(722,-629,-50,1930)}[profile['dataset']]
    check(profile['dataset']+' literal raw financial controls',totals['Inventory']==inv and totals['Payables']==ap and totals['Grni']==grni and totals['Receivables']==299 and totals['Sales']==-199 and totals['SalesReturns']==100 and totals['Cogs']==58 and sum(max(x,0) for x in bal.values())==sum(max(-x,0) for x in bal.values())==tb)
    states.append(dict(dataset=profile['dataset'],generation=b[0],payload_sha256=profile['payload_sha256'],whole_State_and_original_prefixes_exact=True,integrity='ok',Inventory=inv,AP_credit=-ap,GRNI_credit=-grni,TB_side=tb))

closure=read(BASE/'raw/owned-process-closure.json')
check('checkpoint owned records exact cleanup inventory',[x['record'] for x in closure['processes']]==raw['owned_process_records'])
for row in closure['processes']:
    record=row['record'];check('inactive cleanup identity '+record['name'],not row['owned_identity_active'])
    if row['before'] is not None:
        check('pre-signal exact identity '+record['name'],row['before']['same_identity'] and all(row['before'][k]==record[k] for k in ('pgid','starttime_ticks','command')) and row['before']['comm']==Path(record['command'][0]).name and row['action']=='SIGTERM exact owned identity')
        check('post-signal inactive exact identity '+record['name'],row['after']['same_identity'] and row['after']['state']=='Z' and all(row['after'][k]==record[k] for k in ('pgid','starttime_ticks')))
    else:check('no guessed absent PID kill '+record['name'],row['action']=='already inactive' and row['after'] is None)
    stat=Path(f"/proc/{record['pid']}/stat")
    if stat.exists():
        text=stat.read_text();suffix=text[text.rfind(')')+2:].split()
        check('current checkpointed PID inactive '+record['name'],suffix[0]=='Z' or int(suffix[19])!=record['starttime_ticks'])
active=[]
for path in Path('/proc').glob('[0-9]*/cmdline'):
    try:args=path.read_bytes().split(b'\0')
    except (FileNotFoundError,PermissionError,ProcessLookupError):continue
    if args and args[0]==receipt['binary'].encode():active.append(path.parent.name)
check('no active exact C8 binary command',active==[] and closure['no_active_exact_C8_binary_command'])
check('cleanup limitations accurately retained',closure['historical_processes_untouched'] and closure['no_environment_read_or_escalation'] and closure['signals_only_recorded_xvfb_openbox'] and bool(closure['lost_uncheckpointed_new_app_record']))

result=dict(review_result='PASS for retained evidence and claim scope',measurement_result='FAILED eight diagnostic scopes; epoch INCOMPLETE',reviewer='m6_observer_native_reviewer',checks_passed=len(checks),checks=checks,raw_scopes=scopes,stopped_states=states,source_cohorts_complete=2,source_queries_per_mode=2,filtered_pages_per_mode=2,coordinator_guards_stable=True,all_current_frozen_files_exact=True,owned_cleanup_precise=True,no_active_C8_command=True,observer_exit_code=139,exact_native_C_crash_frame='unavailable',observer_final_provenance_receipt='absent',native200_rendering_post_restart_startup_and_final_n10_n20='OPEN / unaccepted',no_product_defect_inferred=True,no_GUI_build_network_replay=True)
(Path(__file__).parent/'partial-pilot-review-result.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
print(json.dumps({k:v for k,v in result.items() if k not in ('checks','raw_scopes','stopped_states')},indent=2))
