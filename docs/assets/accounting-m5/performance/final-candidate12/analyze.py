#!/usr/bin/env python3
"""Recompute immutable C12 observations; run with Python and matplotlib, no apps."""
from pathlib import Path
import json, math, re
ROOT=Path(__file__).resolve().parent
j=json.loads((ROOT/'performance.json').read_text());q=json.loads((ROOT/'native-scale-verification.json').read_text());scale=json.loads((ROOT/'scale-performance.json').read_text());provenance=json.loads((ROOT/'source-provenance.json').read_text());s=j['sales_measurement']
def stats(values):
 values=sorted(values);n=len(values)
 return {'n':n,**{k:values[math.ceil(n*p)-1] for k,p in [('p50',.5),('p95',.95),('p99',.99)]},'worst':values[-1],'method':'nearest rank'}
def budget(value,limit,unit='ms'):return {f'actual_{unit}':value,f'budget_{unit}':limit,'passes':value<=limit}
assert j['result']=='measured' and not j['development_pilot'] and q['result']=='passed'
assert all(j[k] for k in ('frozen_source_stable','binary_stable','observer_stable'))
assert all(provenance[k] for k in ('source_stable','binary_stable','observers_stable','full_verification_stable','wrapper_stable','all_snapshot_guards_stable'))
assert len(provenance['runs'])==2 and all(r['driver_return_code']==0 for r in provenance['runs'])
assert len(provenance['before']['verification_file_sha256'])==53 and len(provenance['before']['files'])==36
assert provenance['before']==provenance['after']
assert len(s['long_editor_pages'])==39 and len(s['filtered_page_samples'])==2
search={}
for name,rows,visible,focused in [('core',j['core_workflow']['search_samples'],'correct_visible_action_ms','focused_usable_action_ms'),('procurement',j['procurement_measurement']['searches']['document']['samples'],'correct_visible_ms','focused_usable_ms'),('posted_sales',s['posted_search']['samples'],'correct_visible_ms','focused_usable_ms'),('paged_sales',s['search_samples'],'correct_visible_ms','focused_usable_ms')]:
 assert len(rows)==20
 search[name]={'visible':stats([r[visible] for r in rows]),'focused':stats([r[focused] for r in rows])}
 search[name]['visible_budget']=budget(search[name]['visible']['p95'],200);search[name]['focused_budget']=budget(search[name]['focused']['p95'],200)
long={key:stats([r[field] for r in s['long_editor_pages']]) for key,field in [('visible','ready_ms'),('focused','focused_usable_ms')]}
long['focus_after_visible_delta']=stats([r['focused_usable_ms']-r['ready_ms'] for r in s['long_editor_pages']])
long['phase_limits']='Raw focus delta combines component focus attempt, real Tab fallback, polling, scrolling/layout and scheduler work; per-phase/Tab counts absent. Delta p95 is computed from paired deltas, not by subtracting marginal p95s. No pure product/observer attribution.'
long['visible_budget']=budget(long['visible']['p95'],150);long['focused_budget']=budget(long['focused']['p95'],150)
startup={}
for name,d in j['datasets'].items():
 assert len(d['samples'])==11
 rows=d['samples'][1:]
 startup[name]={'first_process_ms':d['samples'][0]['startup_ms'],'startup':stats([r['startup_ms'] for r in rows]),'rss_kib':stats([r['steady_rss_kib'] for r in rows]),'hwm_kib':stats([r['kernel_peak_rss_kib'] for r in rows])}
 startup[name]['startup_budget']=budget(startup[name]['startup']['p95'],3000);startup[name]['rss_budget']=budget(startup[name]['rss_kib']['p95']*1024,250000000,'bytes')
phases={'core':[dict(v,phase=k) for k,v in j['core_workflow']['strict_phase_heartbeat_budgets'].items()],'procurement':j['procurement_measurement']['strict_phase_heartbeat_checks'],'external_sales':s['external_stage']['strict_phase_checks'],**{f'sales_{k}':v['strict_phase_checks'] for k,v in s['workflows'].items()}}
phase_summary={name:{'measured_accepted':len([r for r in rows if r['measurable']]),'accepted_misses':[r for r in rows if r['measurable'] and not r['passes']],'excluded_or_unmeasured':[r for r in rows if not r['measurable']]} for name,rows in phases.items()}
actions=[]
for r in q['additive_action_phases']:
 assert r['result']=='accepted'
 assert r['ticks']>0 and r['gaps_ms'] and len(r['gaps_ms'])==r['ticks']
 actual=max([r['terminal_gap_ms'],*r['gaps_ms']]);assert abs(actual-r['max_gap_ms'])<1e-6
 assert (actual<=50)==(r['strict_status']=='pass')
 actions.append({k:r[k] for k in ('id','operation','origin','result','strict_status','elapsed_ms','ticks','max_gap_ms','terminal_gap_ms','ready_observed_ms','started_since_probe_ms','ended_since_probe_ms')})
assert len(actions)==22 and len(q['checks'])==15
raw_phase_recomputation=[]
for p in sorted((ROOT/'repeated').rglob('*performance-phases.jsonl')):
 for line_no,line in enumerate(p.read_text().splitlines(),1):
  r=json.loads(line);g=r['gaps_ms'];maximum=max(g) if g else None
  if maximum is not None:assert abs(maximum-r['max_gap_ms'])<1e-6
  raw_phase_recomputation.append({'path':str(p.relative_to(ROOT)),'line':line_no,'phase':r['phase'],'result':r['result'],'ticks':r['ticks'],'max_gap_ms_recomputed':maximum,'terminal_gap_in_array':True,'measurable_timer':r['ticks']>0,'passes_if_accepted':maximum<=50 if r['ticks']>0 and maximum is not None else None,'raw_gaps_ms':g})
spans={};active=None
for line in (ROOT/'scale/native.log').read_text().splitlines():
 if line.startswith('SARV_ACTION_START '):active=int(line.split(' ')[1]);spans[active]=[]
 elif active and line.startswith('SARV_MAIN_THREAD '):
  match=re.fullmatch(r'SARV_MAIN_THREAD (.+) ([0-9.]+)ms',line)
  if match:spans[active].append({'name':match[1],'ms':float(match[2])})
 elif line.startswith('SARV_ACTION_PHASE '):active=None
broad={name:budget(q[field],limit) for name,field,limit in [('global','maximum_heartbeat_gap_ms_including_setup_and_editor_paging',50),('PDF_busy','maximum_heartbeat_gap_ms_while_pdf_busy',50),('persistence_busy','maximum_heartbeat_gap_ms_while_save_busy',50),('unchanged_quote_pages','maximum_unchanged_editor_page_click_ms',150)]}
cache={name:meta for name,meta in [('posted',s['posted_search']['cached_observation']),('paged',s['paged_cached_observation']),('long',s['long_cached_observation'])]}
for name,meta in cache.items():
 for r in meta['identity_receipts']:
  paths=r['current_membership']['paths_exposed']
  if r['label']=='zero-result hide':assert paths['search'] and paths['status'] and not paths['title'] and not paths['action']
  else:assert all(paths.values())
assert len(cache['long']['identity_receipts'])==40
analysis={'measurement_complete':True,'performance_acceptance':'partial; strict gate FAILED and M5 performance remains open','final_C12_epoch_count':1,'unchanged_M2_invocations':1,'protocols':{'external':j['external_observer_protocol'],'compiled':q['additive_action_protocol']},'binary_sha256':j['binary_sha256'],'production_source_sha256':j['frozen_source_provenance_after']['sha256'],'external_observer_sha256':j['observer_provenance_after']['native-performance-M5.py'],'initial_build_observer_sha256':provenance['initial_C12_build_M5_observer_sha256'],'full53_and_wrapper_provenance_guards':True,'all_provenance_guards_passed':True,'startup':startup,'search':search,'long_navigation':long,'long_open_visible_ms':s['long_editor_open_to_first_page_ms'],'long_open_focused_ms':s['long_editor_open_to_focused_action_ms'],'native_timer_phases':phase_summary,'all_raw_command_phase_recomputation':raw_phase_recomputation,'legacy_broad_strict_budgets':broad,'additive_action_scopes':actions,'additive_miss_count':sum(r['strict_status']=='miss' for r in actions),'additive_page_miss_count':sum(r['strict_status']=='miss' for r in actions if 'quote_page_' in r['operation']),'source_trace_spans_correlated_with_active_scopes':spans,'source_span_limits':'Nested wall-clock source spans are not summed, not isolated CPU and not proof of which observer/product/scheduling work caused a timer gap. No fixture/capture/disk verification inside additive scopes; readiness/tracing remains.','cache_identity_receipt_counts':{k:len(v['identity_receipts']) for k,v in cache.items()},'read_only_Sales_heartbeat':'unmeasured; no phase acceptance inferred','full_quote_workflow_memory_kib':scale['application_memory_kib_after_full_workflow'],'C2_strict_results_retained_unchanged':{'paged_visible_p95_ms':331.616323,'paged_focused_p95_ms':345.277586,'long_visible_p95_ms':419.622452,'long_focused_p95_ms':690.369299,'global_ms':320.116878,'PDF_busy_ms':117.240539,'persistence_busy_ms':100.135268,'unchanged_quote_page_ms':138.524176},'historical_M4_misses_retained':{'global_ms':280.552,'unchanged_page_ms':161.263,'PDF_busy_ms':127.266,'persistence_busy_ms':98.725},'causal_product_speed_or_competitor_parity_claim':False,'limits':['C12 changes production; observation protocols remain C8 m5-cached-v3/m5-action-v1.2; differences from C2 are not identical controlled causal endpoints.','All misses retained; timers are responsiveness proxies, not continuous CPU traces.','n10/n20 p99 equals worst; n39 sequential pages are descriptive, not independent launches.','No dedicated4core8GBSSD/physical-cache-cold/fiveyear100kdocs1Mjournals10kparties/WindowsWayland/physicalprinting/powerloss/leak acceptance.']}
(ROOT/'analysis.json').write_text(json.dumps(analysis,ensure_ascii=False,indent=2)+'\n')
warnings=[]
for p in [* (ROOT/'repeated').rglob('*.log'),* (ROOT/'scale').rglob('*.log')]:
 if any(x in p.name for x in ('observer','xvfb','openbox')):continue
 for number,line in enumerate(p.read_text(errors='replace').splitlines(),1):
  if re.search(r'(Gtk|Adwaita|GLib|Gdk|Gsk)[-: ].*(WARNING|CRITICAL)',line,re.I):warnings.append({'path':str(p.relative_to(ROOT)),'line':number,'text':line})
(ROOT/'application-warning-check.json').write_text(json.dumps({'application_GTK_warning_critical_count':len(warnings),'entries':warnings,'scope':'Application log scan; AT-SPI shutdown notices retained separately in observer logs.'},indent=2)+'\n')
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
fig,axes=plt.subplots(3,1,figsize=(11,10),constrained_layout=True)
for name,data in [('Core',j['core_workflow']['search_samples']),('Procurement',j['procurement_measurement']['searches']['document']['samples']),('Posted Sales',s['posted_search']['samples']),('120 raw drafts',s['search_samples'])]:
 field='focused_usable_action_ms' if name=='Core' else 'focused_usable_ms'
 axes[0].plot(range(1,21),[r[field] for r in data],'.-',label=name)
axes[0].axhline(200,color='red',linestyle='--',label='200 ms budget');axes[0].set(title='C12 real-key focused lookup observations, n20 per fixture',xlabel='Alternating query sample',ylabel='Elapsed ms');axes[0].legend(ncols=3,fontsize=8)
axes[1].plot([r['page'] for r in s['long_editor_pages']],[r['ready_ms'] for r in s['long_editor_pages']],'.-',label='Current visible Edit')
axes[1].plot([r['page'] for r in s['long_editor_pages']],[r['focused_usable_ms'] for r in s['long_editor_pages']],'.-',label='Current focused Edit')
axes[1].axhline(150,color='red',linestyle='--',label='150 ms budget');axes[1].set(title='1,000-line Sales editor: 39 sequential loaded page transitions',xlabel='Current page',ylabel='Elapsed ms');axes[1].legend(fontsize=8)
axes[2].bar([r['id'] for r in actions],[r['max_gap_ms'] for r in actions],color=['#b33838' if r['strict_status']=='miss' else '#24856a' for r in actions]);axes[2].axhline(50,color='red',linestyle='--');axes[2].set(title='Independent20ms action scopes: terminal segment included',xlabel='Scope ID:1–2 Save;3–20 paging;21 choice;22 internal PDF',ylabel='Maximum timer gap ms',xticks=range(1,23))
fig.suptitle('One frozen C12 epoch; changed observer method; no causal speed comparison',fontsize=12)
fig.savefig(ROOT/'latency-samples.svg');fig.savefig(ROOT/'latency-samples.png',dpi=130);plt.close(fig)
print('analysis complete: strict gate FAIL; additive misses',analysis['additive_miss_count'])
