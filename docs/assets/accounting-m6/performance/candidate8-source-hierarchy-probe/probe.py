#!/usr/bin/env python3
"""One read-only actual hierarchy probe; no query/input/post or timed samples."""
from pathlib import Path
import argparse
import hashlib
import importlib.util
import json
import os
import time
from collections import deque

ROOT = Path(__file__).resolve().parents[5]
SELF = Path(__file__).resolve()
def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result
import sys
sys.path.insert(0, str(ROOT / 'scripts'))
held = module('m6_held_probe_observer', ROOT / 'scripts/native-performance-M6.py')
coordinator = module('m6_readonly_probe_guard', ROOT / 'docs/assets/accounting-m6/performance/coordinator.py')
RECEIPT = ROOT / 'docs/assets/accounting-m6/performance/candidate8-runtime1-coordinator-receipt.json'
RECEIPT_SHA = 'e1e3ca060d1b0b1b46de0161e6e85bcaf9b3ee7db4c108334ab4dede81287784'
OBSERVER_SHA = '930730bf94714ebb6370d4eba041afba286bce1ea11c3f3050214ff0dfc475e2'
def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()
def save(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')

def topology(access, rows, root):
    pending, seen, records = deque([rows]), set(), []
    while pending:
        node = pending.popleft()
        assert node and node not in seen and len(seen) < 200, 'Hierarchy bound/cycle'
        seen.add(node)
        access.a.atspi_accessible_clear_cache(node)
        states = access.a.atspi_accessible_get_state_set(node)
        parent = access.a.atspi_accessible_get_parent(node, None)
        access.a.atspi_accessible_clear_cache(parent)
        parent_children = access.children(parent)
        row = {'node': node, 'name': access.string(access.a.atspi_accessible_get_name, node, None),
               'role': access.string(access.a.atspi_accessible_get_role_name, node, None)}
        access.a.atspi_accessible_clear_cache(node)
        children = access.children(node)
        records.append({**row, 'node': hex(node), 'parent': hex(parent or 0),
                        'process_id': access.a.atspi_accessible_get_process_id(node, None),
                        'DEFUNCT': bool(not states or access.a.atspi_state_set_contains(states, 6)),
                        'children': [hex(child) for child in children],
                        'current_parent_children': [hex(child) for child in parent_children],
                        'current_parent_edge': node in parent_children,
                        'current_owned_modal_member': held.member(access, row, root)})
        pending.extend(children)
    assert all(not row['DEFUNCT'] and row['process_id'] == access.pid and row['current_parent_edge'] and row['current_owned_modal_member'] for row in records)
    return records

def main():
    parser = argparse.ArgumentParser();parser.add_argument('--output', type=Path, required=True);args=parser.parse_args()
    args.output=args.output.resolve();assert args.output.is_relative_to(ROOT/'artifacts/performance-m6') and not args.output.exists();args.output.mkdir(parents=True)
    own_sha=sha(SELF);assert sha(ROOT/'scripts/native-performance-M6.py')==OBSERVER_SHA
    receipt=json.loads(RECEIPT.read_text());before=coordinator.guard(receipt,RECEIPT,RECEIPT_SHA)
    dataset=json.loads((ROOT/receipt['dataset_manifest']['path']).read_text());args.binary=Path(receipt['binary'])
    report={'result':'incomplete','probe_only':True,'timed_budget_claim':False,'sidecar_sha256':own_sha,
            'held_observer_sha256':OBSERVER_SHA,'receipt_sha256':RECEIPT_SHA,'guard_before':before,
            'capture_samples':[],'owned_process_records':[],'x11_error_observations':[],
            'completed_checkpoint_boundaries':[],'rejected_source_observations':[],
            'utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime())}
    observer=held.Observer(args,report,dataset);profile=None;baseline=None
    try:
        observer.setup();profile,recipe=observer.copied('source120-billed','readonly-source-profile');baseline=held.payload(profile)
        app,window,access=observer.app(profile,'readonly-source-app');observer.purchases(access)
        held.press(access,held.RETURN,observer.x11)
        search=held.observe(lambda:held.find(access,held.SEARCH,roles=('entry','text')),'read-only source picker')
        root=held.modal(access,search)
        status=held.observe(lambda:next((row for row in held.walk(access,root) if held.Observer.source_kind(row)=='status'),None),'source index ready')
        rows=access.a.atspi_accessible_get_parent(status['node'],None)
        report.update(owned_pid=app.pid,root_node=hex(root),rows_node=hex(rows),query_before=access.text(search))
        report['hierarchy_before']=topology(access,rows,root)
        observer.capture(window,args.output/'actual-source-picker-hierarchy.png')
        report['hierarchy_after']=topology(access,rows,root)
        report['query_after']=access.text(search)
        assert report['hierarchy_before']==report['hierarchy_after'] and report['query_before']==report['query_after']==''
        assert held.payload(profile)==baseline
        report.update(result='qualified_actual_readonly_hierarchy',source_generation_payload_bytes_unchanged=True,
                      no_query_warmup_input_post_or_workflow=True,before_after_hierarchy_exact=True)
    except Exception as error:
        report['error']=repr(error);raise
    finally:
        observer.close()
        if profile is not None and baseline is not None:
            report['stopped_generation_payload_bytes_unchanged']=held.payload(profile)==baseline
            assert report['stopped_generation_payload_bytes_unchanged']
        report['guard_after']=coordinator.guard(receipt,RECEIPT,RECEIPT_SHA)
        report['all_guards_stable']=report['guard_before']==report['guard_after'] and sha(SELF)==own_sha and sha(ROOT/'scripts/native-performance-M6.py')==OBSERVER_SHA
        assert report['all_guards_stable']
        audit=[]
        for q in report['owned_process_records']:
            p=Path(f"/proc/{q['pid']}/stat");current=None
            if p.exists():
                text=p.read_text();parts=text[text.rfind(')')+2:].split();current={'state':parts[0],'pgid':int(parts[2]),'starttime_ticks':int(parts[19]),'same_identity':int(parts[2])==q['pgid'] and int(parts[19])==q['starttime_ticks']}
            assert current is None or not current['same_identity'] or current['state']=='Z'
            audit.append({'record':q,'current':current,'owned_identity_active':False})
        report['owned_process_closure']=audit;report['all_new_owned_processes_inactive']=True
        save(args.output/'hierarchy-probe.json',report)
        print(json.dumps({'result':report['result'],'all_guards_stable':report['all_guards_stable'],'all_new_owned_processes_inactive':True}))
if __name__=='__main__':main()
