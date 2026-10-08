#!/usr/bin/env python3
"""One untimed private dark200 appearance observation; one input/no post."""
import argparse
import faulthandler
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import time

faulthandler.enable(all_threads=True)
ROOT=Path(__file__).resolve().parents[5]
SELF=Path(__file__).resolve()
RECEIPT=ROOT/'docs/assets/accounting-m6/performance/candidate9-runtime3-coordinator-receipt.json'
RECEIPT_SHA='bca8475d98cb2401179fc684a0f4f25a8e0333a4c1465926b1991b47b0cea1a3'
OBSERVER_SHA='63edfdab8e16e96fb3451e14a54f0a8fab8417062d0d4302f2abbf292a446b6b'
sys.path.insert(0,str(ROOT/'scripts'))
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def load(name,path):
    spec=importlib.util.spec_from_file_location(name,path);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module
def save(path,value):path.write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n')
def snapshot(held,access,paths):
    rows={str(node):held.current_node(access,node) for node in paths.roles}
    children={}
    for parent in dict.fromkeys(paths.edges.values()):
        current=held.current_node(access,parent)
        if current is None:
            children[str(parent)]={'parent':None,'children':None};continue
        access.a.atspi_accessible_clear_cache(parent)
        nodes=tuple(access.children(parent))
        children[str(parent)]={'parent':current,'children':[{'node':node,'current':held.current_node(access,node)} for node in nodes]}
    return {'rows':rows,'children':children,'expected_edges':paths.edges,'qualified_roles':paths.roles,
            'qualified_stable_children':paths.stable_children,'held_path_snapshot_ready':paths.snapshot(access) is not None}

def child(args):
    held=load('held_topology_probe',ROOT/'scripts/native-performance-M6.py')
    receipt=json.loads(RECEIPT.read_text());dataset=json.loads((ROOT/receipt['dataset_manifest']['path']).read_text())
    args.binary=Path(receipt['binary']);args.samples=1;args.search_samples=2;args.pilot=True
    report={'result':'incomplete','diagnostic_only':True,'timed_budget_claim':False,'startup':{},'source_picker_search':{},'return_workflows':{},
            'capture_samples':[],'owned_process_records':[],'rejected_source_observations':[],'completed_checkpoint_boundaries':[],'x11_error_observations':[]}
    observer=held.Observer(args,report,dataset)
    try:
        observer.setup()
        observer.env['ADW_DEBUG_COLOR_SCHEME']='prefer-dark'
        report['appearance_only']=True
        report['requested_color_scheme']='prefer-dark'
        profile,recipe=observer.copied('return200-billed','invalid200-dark-private')
        baseline=held.payload(profile);ids=recipe['receipt_line_ids']
        app,window,access=observer.app(profile,'invalid200-dark')
        observer.purchases(access);quantity,editor,_=observer.open_source(access,recipe['source_reference'],ids[0]);observer.qualify_editor(access,editor,ids)
        paths=observer.editor_paths[editor]['paths'];size=observer.x11.geometry(window)
        report['initial_qualified_topology']=snapshot(held,access,paths)
        for page in range(2,9):observer.turn(access,editor,ids,page,'next')
        held.fill(access,held.QUANTITY_PREFIX+ids[-1],'2',observer.x11,editor);observer.editor_paths[editor]['expected'][ids[-1]]='2'
        for page in range(7,0,-1):observer.turn(access,editor,ids,page,'previous')
        report['before_preview_topology']=snapshot(held,access,paths)
        observer.checkpoint('targeted-before-invalid-preview')
        held.press(access,held.PREVIEW,observer.x11,editor)
        invalid=held.observe(lambda:held.find(access,held.QUANTITY_PREFIX+ids[-1],editor,('entry','text')),'actualL200 after invalidPreview')
        held.observe(lambda:held.ready(access,invalid,editor,True) and access.text(invalid)=='2','exactcurrentraw2/focus')
        observer.qualify_invalid_quantity_label(access,editor,ids,8)
        report['observations']=[]
        def capture_variation(variation):
            began=time.perf_counter()
            for delay in (.5,1.,2.):
                remaining=delay-(time.perf_counter()-began)
                if remaining>0:time.sleep(remaining)
                field=held.find(access,held.QUANTITY_PREFIX+ids[-1],editor,('entry','text'))
                assert field and held.ready(access,field,editor,True) and access.text(field)=='2'
                assert held.payload(profile)==baseline
                assert observer.page_state(access,editor,ids,8) is not None
                current=snapshot(held,access,paths)
                image=profile/f'{variation}-settled-min{delay:g}s.png';observer.capture(window,image)
                report['observations'].append({'variation':variation,'minimum_delay_s':delay,'completion_delay_s':time.perf_counter()-began,
                    'client_geometry':observer.x11.geometry(window),'receipt_line_id':ids[-1],'raw_text':'2','actual_focus':True,'whole_State_unchanged':True,
                    'no_keys_focus_or_scroll_after_dispatch_or_resize':True,'topology':current,'screenshot':str(image)})
                observer.checkpoint(f'targeted-{variation}-settled-{delay:g}s')
        capture_variation('dark-standard')
        observer.x11.resize(window,620,800);observer.x11.check();held.observe(lambda:observer.x11.geometry(window)==[620,800],'actualnarrow620x800')
        capture_variation('dark-narrow')
        observer.x11.resize(window,*size);observer.x11.check();held.observe(lambda:observer.x11.geometry(window)==size,'restoreexactoriginalsize')
        assert held.payload(profile)==baseline
        report.update(result='closed-dark200-appearance-observation-only',original_client_geometry=size,restored_client_geometry=observer.x11.geometry(window),
                      whole_State_unchanged=True,quantities_entered=1,full200_financial_workflow=False,post_attempted=False)
        observer.checkpoint('targeted-complete-before-cancel')
    except BaseException as error:
        report['error']=repr(error);raise
    finally:
        observer.close();save(args.output/'diagnostic.json',report)

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--output',type=Path,required=True);parser.add_argument('--child',action='store_true');args=parser.parse_args();args.output=args.output.resolve()
    if args.child:return child(args)
    assert args.output.is_relative_to(ROOT/'artifacts/performance-m6') and not args.output.exists();args.output.mkdir(parents=True)
    coord=load('external_guard',ROOT/'docs/assets/accounting-m6/performance/coordinator.py');receipt=json.loads(RECEIPT.read_text());ownsha=sha(SELF)
    assert sha(RECEIPT)==RECEIPT_SHA and sha(ROOT/'scripts/native-performance-M6.py')==OBSERVER_SHA
    result={'result':'incomplete','diagnostic_only':True,'sidecar_sha256':ownsha,'guard_before':coord.guard(receipt,RECEIPT,RECEIPT_SHA)}
    command=['dbus-run-session','--','/usr/bin/python3','-X','faulthandler',str(SELF),'--output',str(args.output),'--child']
    with (args.output/'native.log').open('x') as stream:
        process=subprocess.Popen(command,cwd=ROOT,stdout=stream,stderr=subprocess.STDOUT,start_new_session=True)
        identity=coord.process_identity(process.pid);result['wrapper_identity']=identity;save(args.output/'guardian.json',result)
        try:result['exit_code']=process.wait()
        finally:
            result['known_owned_cleanup']=coord.cleanup_owned(args.output/'owned-processes.jsonl',Path(receipt['binary']),identity['starttime_ticks'])
            result['guard_after']=coord.guard(receipt,RECEIPT,RECEIPT_SHA);result['all_guards_exact']=result['guard_before']==result['guard_after'] and sha(SELF)==ownsha
            result['result']='closed' if result.get('exit_code')==0 and result['all_guards_exact'] and result['known_owned_cleanup']['all_owned_inactive'] else 'failed-or-incomplete'
            save(args.output/'guardian.json',result)
    print(json.dumps({key:result[key] for key in ('result','exit_code','all_guards_exact')}),flush=True)
if __name__=='__main__':main()
