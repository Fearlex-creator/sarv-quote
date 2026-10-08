"""Independently recompute retained C12 measurements; no apps or new timing."""
from pathlib import Path
import json, math, hashlib, re, sqlite3
root=Path('/workspace/sarv-quote')
base=root/'docs/assets/accounting-m5/performance/final-candidate12'
out=root/'docs/assets/accounting-m5/review/performance-candidate12'
out.mkdir(exist_ok=True)
inputs={}
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def load(p):
 inputs[str(p.relative_to(root))]=sha(p)
 return json.loads(p.read_text())
m=load(root/'docs/assets/accounting-m5/checks/candidate12/manifest.json')
r=load(base/'performance.json');s=load(base/'native-scale-verification.json');wrapper=load(base/'scale-performance.json');prov=load(base/'source-provenance.json')
inventory=load(base/'evidence-files.json')['files']
for name,v in inventory.items():
 p=base/name;assert p.stat().st_size==v['bytes'] and sha(p)==v['sha256'],name
assert {str(p.relative_to(base)) for p in base.rglob('*') if p.is_file() and p.name!='evidence-files.json'}==set(inventory)
assert not r['development_pilot'] and prov['authorized_final_measurement_epoch'] and not prov['functional_qualification_only']
assert r['result']=='measured' and r['frozen_source_provenance_before']==r['frozen_source_provenance_after'] and r['frozen_source_provenance_before']['files']==m['source_file_sha256']
assert r['binary_sha256']==r['binary_sha256_after']==m['binary_sha256'] and sha(Path(m['binary']))==m['binary_sha256']
assert r['observer_provenance_before']==r['observer_provenance_after']
assert prov['before']==prov['after'] and prov['before']['files']==m['source_file_sha256'] and prov['before']['binary_sha256']==m['binary_sha256']
assert len(prov['before']['files'])==36 and len(prov['before']['verification_file_sha256'])==53
assert prov['before']['verification_file_sha256']==load(base/'qualified-verification-before.json')==load(root/'docs/assets/accounting-m5/checks/candidate12/verification-source-before.json')
assert prov['before']['manifest_sha256']==sha(root/'docs/assets/accounting-m5/checks/candidate12/manifest.json')
assert prov['before']['verification_manifest_sha256']==sha(base/'qualified-verification-before.json')
assert prov['before']['wrapper_sha256']==sha(base/'sequential-provenance-driver.py')
assert prov['diagnostic_focus_receipts_enabled'] is False
assert len(prov['runs'])==2 and prov['driver_return_code']==0
assert prov['runs'][0]['script']=='native-performance-M5.py' and prov['runs'][1]['script']=='native-performance-M2.py'
assert prov['runs'][0]['end_utc']<=prov['runs'][1]['start_utc']
for run in prov['runs']:assert run['driver_return_code']==0 and run['before']==run['after']==prov['before']
assert prov['runs'][0]['command'][-4:]==['--samples','10','--search-samples','20']
assert prov['before']['observers']==r['observer_provenance_before']
for path,h in prov['before']['verification_file_sha256'].items():assert sha(root/'docs/assets/accounting-m5/checks/candidate12/verification-source'/path)==h
assert prov['before']['observers']['native-performance-M5.py']=='472e83fa8231a77a32293f4b00713552ec61bec9d802c16b3ff76781b2b2a40d'
assert prov['before']['observers']['native-performance-M2.py']=='579a255004d9a98ab94611e45873c13c65de9a0dcf609b355b24c3dd44e040c5'
verified=[]

def verify(label,values,reported):
 vals=sorted(values);assert len(vals)==reported['n'];actual={k:vals[math.ceil(q*len(vals))-1]for k,q in [('p50',.5),('p95',.95),('p99',.99)]};actual['worst']=vals[-1]
 for k,v in actual.items():assert math.isclose(v,reported[k],abs_tol=1e-9),(label,k,v,reported[k])
 verified.append({'name':label,'n':len(vals),**actual});return actual
for name,d in r['datasets'].items():
 assert len(d['samples'])==11
 for stat,field in [('repeated_process_startup_ms','startup_ms'),('steady_rss_kib','steady_rss_kib'),('kernel_peak_rss_kib','kernel_peak_rss_kib')]:verify(name+'/'+stat,[v[field]for v in d['samples'][1:]],d[stat])
 assert d['initialized_shape']==d['final_shape']
c=r['core_workflow'];verify('core visible',[x['correct_visible_action_ms']for x in c['search_samples']],c['correct_visible_action_ms']);verify('core focused',[x['focused_usable_action_ms']for x in c['search_samples']],c['search_ms'])
for name,d in r['procurement_measurement']['searches'].items():
 for stat in ['correct_visible_ms','focused_usable_ms']:verify('proc '+name+'/'+stat,[x[stat]for x in d['samples']],d[stat])
a=r['sales_measurement'];post=a['posted_search']
for d,name in [(a,'120 draft'),(post,'6 posted')]:
 for stat in ['correct_visible_ms','focused_usable_ms']:verify(name+'/'+stat,[x[stat]for x in d['search_samples'if name=='120 draft'else'samples']],d[stat])
verify('long39 visible',[x['ready_ms']for x in a['long_editor_pages']],a['long_editor_page_ms']);verify('long39 focused',[x['focused_usable_ms']for x in a['long_editor_pages']],a['long_editor_focused_page_ms']);assert [x['page']for x in a['long_editor_pages']]==list(range(2,41));focusdelta=sorted(x['focused_usable_ms']-x['ready_ms']for x in a['long_editor_pages'])
for cache in [a['paged_cached_observation'],a['long_cached_observation'],post['cached_observation']]:
 for item in cache['identity_receipts']:
  paths=item['current_membership']['paths_exposed']
  if item['label']=='zero-result hide':assert paths['search']and paths['status']and not paths['title']and not paths['action']
  else:assert all(paths.values())and all(x['currently_exposed']for x in item['current_membership']['edges'])
assert s['result']=='passed'and len(s['additive_action_phases'])==22;phases=[]
for v in s['additive_action_phases']:
 assert v['result']=='accepted'and v['ready_frames']>=2 and v['gaps_ms'];mx=max(v['gaps_ms']+[v['terminal_gap_ms']]);assert math.isclose(mx,v['max_gap_ms'],abs_tol=1e-9);assert v['strict_status']==('pass'if mx<=50 else'miss');assert math.isclose(v['ended_since_probe_ms']-v['started_since_probe_ms'],v['elapsed_ms'],abs_tol=1e-6);phases.append({'id':v['id'],'operation':v['operation'],'max_gap_ms':mx,'strict_status':v['strict_status'],'elapsed_ms':v['elapsed_ms']})
pages=[v for v in phases if v['operation'].startswith('quote_page')];assert len(pages)==18 and sum(v['strict_status']=='miss'for v in pages)==3
unchanged=[x['duration_ms']for x in s['editor_page_clicks']if not x['draft_changed']];assert len(unchanged)==17 and math.isclose(max(unchanged),s['maximum_unchanged_editor_page_click_ms'],abs_tol=1e-8)

assert len(verified)==34 and len(s['checks'])==15 and all(v['result']=='accepted' for v in s['additive_action_phases'])
assert sum(v['strict_status']=='miss' for v in phases)==5
for phase in s['additive_action_phases']:
 assert phase['budget_ms']==50 and phase['ready_frames']>=2 and len(phase['gaps_ms'])==phase['ticks']
 assert math.isclose(sum(phase['gaps_ms'])+phase['terminal_gap_ms'],phase['elapsed_ms'],abs_tol=1e-6)
raw_phases=[]
raw_records=[]
for path in sorted((base/'repeated').rglob('*performance-phases.jsonl')):
 inputs[str(path.relative_to(root))]=sha(path)
 for line_number,line in enumerate(path.read_text().splitlines(),1):
  x=json.loads(line);g=x['gaps_ms'];maximum=max(g) if g else None
  raw_records.append(x)
  if g:assert math.isclose(maximum,x['max_gap_ms'],abs_tol=1e-9)
  assert len(g)==x['ticks']+1 and math.isclose(sum(g),x['elapsed_ms'],abs_tol=1e-6)
  raw_phases.append({'path':str(path.relative_to(base)),'line':line_number,'phase':x['phase'],'result':x['result'],'ticks':x['ticks'],'max_gap_ms':maximum,'terminal_gap_ms':g[-1] if g else None})
streams={'core':[dict(v,phase=k) for k,v in r['core_workflow']['strict_phase_heartbeat_budgets'].items()], 'procurement':r['procurement_measurement']['strict_phase_heartbeat_checks'], 'external_sales':a['external_stage']['strict_phase_checks'], **{'sales_'+k:v['strict_phase_checks'] for k,v in a['workflows'].items()}}
reported_raw={'core':r['core_workflow']['native_phase_records'], 'procurement':r['procurement_measurement']['external_order_phase_records']+r['procurement_measurement']['full_workflow_phase_records'], 'external_sales':a['external_stage']['native_phase_records'], **{'sales_'+k:v['native_phase_records'] for k,v in a['workflows'].items()}}
for name,rows in reported_raw.items():
 assert all(row in raw_records for row in rows),name
 if name=='core':
  for check in streams[name]:
   row=next(row for row in rows if row['phase']==check['phase'] and row['result']==check['result'])
   assert check['actual_ms']==row['max_gap_ms']
 else:
  assert len(rows)==len(streams[name])
  for row,check in zip(rows,streams[name]):
   assert check['actual_ms']==row['max_gap_ms'] and check['ticks']==row['ticks'] and check['result']==row['result']
   assert check['measurable']==(row['ticks']>0 and row['result'] in ('verified','worker_finished'))
accepted={}
for name,records in streams.items():
 for x in records:
  if x['measurable']:assert x['passes'] and x['actual_ms']<=x['budget_ms']==50
  else:assert not x['passes']
 accepted[name]={'accepted':sum(x['measurable'] for x in records),'excluded_or_unmeasured':sum(not x['measurable'] for x in records)}
assert {k:v['accepted'] for k,v in accepted.items()}=={'core':2,'procurement':22,'external_sales':4,'sales_full':14,'sales_draft':2}
startup_pass=all(v['p95']<=3000 for v in verified if v['name'].endswith('/repeated_process_startup_ms'))
rss_pass=all(v['p95']*1024<=250000000 for v in verified if v['name'].endswith('/steady_rss_kib'))
search_pass=all(v['p95']<=200 for v in verified if v['name'].startswith(('core ','proc ','120 draft','6 posted')))
long_pass=all(v['p95']<=150 for v in verified if v['name'].startswith('long39'))
assert startup_pass and rss_pass and search_pass and long_pass
broad={'global':s['maximum_heartbeat_gap_ms_including_setup_and_editor_paging'],'PDF_busy':s['maximum_heartbeat_gap_ms_while_pdf_busy'],'persistence_busy':s['maximum_heartbeat_gap_ms_while_save_busy'],'unchanged_quote_pages':s['maximum_unchanged_editor_page_click_ms']}
assert all(broad[k]>50 for k in ['global','PDF_busy','persistence_busy']) and broad['unchanged_quote_pages']<=150
warnings=[];logs=0
for path in [*(base/'repeated').rglob('*.log'),*(base/'scale').rglob('*.log')]:
 if any(word in path.name for word in ['observer','xvfb','openbox']):continue
 logs+=1
 for number,line in enumerate(path.read_text(errors='replace').splitlines(),1):
  if re.search(r'(Gtk|Adwaita|GLib|Gdk|Gsk)[-: ].*(WARNING|CRITICAL)',line,re.I):warnings.append({'path':str(path.relative_to(base)),'line':number,'text':line})
assert warnings==load(base/'application-warning-check.json')['entries']==[]
# Re-read actual ignored synthetic profiles; the portable receipt records hashes
# of their raw payloads, rather than substituting totals for complete data.
def state(path):
 db=root/path;c=sqlite3.connect(f'file:{db}?mode=ro',uri=True);assert c.execute('pragma integrity_check').fetchone()==('ok',)
 generation,payload=c.execute('select generation,payload from state where id=1').fetchone();c.close()
 inputs[path+':payload']=hashlib.sha256(payload.encode()).hexdigest();return generation,json.loads(payload)
_,full=state('artifacts/performance-m5/final-candidate12/sales-native-full/sarv.sqlite3')
balances={}
for journal in full['accounting']['journals']:
 assert sum(x['debit'] for x in journal['lines'])==sum(x['credit'] for x in journal['lines'])
 for line in journal['lines']:
  key=line['account']['id'];balances[key]=balances.get(key,0)+line['debit']-line['credit']
records=full['accounting']['sales']['records'];mapping=records[0]['frozen_setup']['command']['mappings'];roles={x['role']:balances.get(x['account_id'],0) for x in mapping}
expected={'Cash':251,'Bank':300,'Receivables':299,'Inventory':422,'ChequesReceivable':200,'Payables':-329,'ChequesPayable':-100,'Grni':-50,'OwnerCapital':-952,'Sales':-199,'SalesReturns':100,'Cogs':58}
assert all(roles[k]==v for k,v in expected.items())
assert len(records)==6 and len(full['accounting']['journals'])==9 and sum(len(x['movements']) for x in records)==4
assert sum(x for x in balances.values() if x>0)==1630 and -sum(x for x in balances.values() if x<0)==1630
components=[{'kind':next(iter(x['data'])),'net':next(iter(x['data'].values()))['total'],'cost':next(iter(x['data'].values())).get('cost')} for x in records]
assert components==[{'kind':'Order','net':199,'cost':None},{'kind':'Fulfill','net':100,'cost':58},{'kind':'Return','net':40,'cost':23},{'kind':'Return','net':60,'cost':35},{'kind':'Fulfill','net':99,'cost':58},{'kind':'Order','net':20,'cost':None}]
for profile,shape,count in [('sales-paged-search',a['paged_search_shape'],120),('sales-long-editor',a['long_editor_shape'],1)]:
 generation,data=state('artifacts/performance-m5/final-candidate12/'+profile+'/sarv.sqlite3')
 assert generation==shape['generation'] and inputs['artifacts/performance-m5/final-candidate12/'+profile+'/sarv.sqlite3:payload']==shape['payload_sha256']
 assert len(data['sales_drafts'])==count
 if count==120:assert sum(d['reference'].startswith('PERF-A') for d in data['sales_drafts'])==60 and sum(d['reference'].startswith('PERF-B') for d in data['sales_drafts'])==60
 else:assert len(data['sales_drafts'][0]['lines'])==1000
active=None;spans={}
for line in (base/'scale/native.log').read_text().splitlines():
 if line.startswith('SARV_ACTION_START '):active=int(line.split()[1]);spans[active]=[]
 elif active and line.startswith('SARV_MAIN_THREAD '):
  match=re.fullmatch(r'SARV_MAIN_THREAD (.+) ([0-9.]+)ms',line)
  if match:spans[active].append({'name':match[1],'ms':float(match[2])})
 elif line.startswith('SARV_ACTION_PHASE '):active=None
result={'result':'PASS bounded local workflow checkpoint review; strict performance FAIL remains PARTIAL and full M5/G4 OPEN',
 'epoch':'one immutable C12 final M5 n10/n20 followed by one unchanged M2 invocation',
 'source_sha256':m['production_source_sha256'],'binary_sha256':m['binary_sha256'],'portable_files_hashed':len(inventory),
 'actual36source53verificationbinaryobserverwrapper_guards_equal':True,'nearest_rank_stats_verified':verified,
 'startup_and_initial_RSS_budgets_pass':True,'all_search_budgets_pass':True,'long_visible_and_focused_budgets_pass':True,
 'native_phase_acceptance':accepted,'raw_phase_gaps_terminal_sums_recomputed':raw_phases,
 'additive_intervals_terminal_maxima':phases,'additive_miss_count':5,'page_miss_count':3,'broad_budgets_actual_ms':broad,
 'cache_current_membership_verified':{'posted':21,'paged':23,'long':40},
 'actual_final_fixture':{'balanced_journals':9,'sales_records':6,'typed_sales_movements':4,'signed_role_balances':roles,'trial_balance':[1630,1630],'components':components,'paged_drafts':120,'long_lines':1000},
 'source_wall_spans_in_active_scopes':spans,'maximum_active_source_wall_ms':max(x['ms'] for v in spans.values() for x in v),
 'application_material_log_scan':{'logs':logs,'warning_critical_entries':warnings,'scope':'Retained application logs only; observer shutdown notices separate.'},
 'limits':['Current protocols remain m5-cached-v3/m5-action-v1.2; source/method/history differences prohibit controlled product-only speed causation.','No unchanged50ms threshold waiver:3 paging+choice+internalPDF and all3 broad timers still miss.','n39 pages are descriptive one-process observations; n10/n20 p99 equals worst; physical cold-cache/reference/fullscale/leak/WaylandWindows/print/accessibility/accountant approval untested.','Financial/data7 retains C5 execution on unchanged financial implementation; fresh C12 native fixture and complete qualification are separately verified.','Local workflow checkpoint approval does not accept full PERF/G4/M5, production use or statutory/tax capability.'],
 'inputs_sha256':inputs}
(out/'independent-analysis.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
print(json.dumps({'result':result['result'],'stats':len(verified),'files':len(inventory),'action_misses':[x for x in phases if x['strict_status']=='miss'],'broad':broad,'application_logs':logs,'active_source_wall_max':result['maximum_active_source_wall_ms']},indent=2))
