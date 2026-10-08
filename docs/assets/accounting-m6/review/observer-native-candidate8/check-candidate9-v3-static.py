#!/usr/bin/env python3
"""Bounded independent canonical AST adversarial review; no native imports."""
import ast
import hashlib
import json
import math
import signal
import tempfile
from pathlib import Path
from types import SimpleNamespace

ROOT=Path(__file__).resolve().parents[5]
OWN=Path(__file__).resolve().parent
BASE=ROOT/'docs/assets/accounting-m6/performance/runtime9-observer-guardian'
OBSERVER=ROOT/'scripts/native-performance-M6.py'
COORDINATOR=BASE.parent/'coordinator.py'
ANALYZER=BASE.parent/'analyze.py'
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def extract(p,names,ns):
    tree=ast.parse(p.read_text())
    for n in tree.body:
        if isinstance(n,ast.FunctionDef) and n.name in names:
            exec(compile(ast.Module(body=[n],type_ignores=[]),str(p),'exec'),ns)
    return tree

# Reuse this reviewer's independently constructed 88 subtree/owned-role/focus
# cases, extracting only their AST harness and reading current canonical bytes.
p=OWN/'check-proposal.py'
source=p.read_text().split('\nheld=ast.parse(',1)[0]
t=ast.parse(source)
for n in t.body:
    if isinstance(n,ast.Assign):
        names={x.id for x in n.targets if isinstance(x,ast.Name)}
        if 'EXPECTED_SHA' in names:n.value=ast.Constant(digest(OBSERVER))
        if 'SOURCE' in names:n.value=ast.Call(func=ast.Name(id='Path',ctx=ast.Load()),args=[ast.Constant(str(OBSERVER))],keywords=[])
ns={'__file__':str(p)}
exec(compile(ast.fix_missing_locations(t),str(p),'exec'),ns)
cases=list(ns['cases'])
def check(name,value):
    assert value,name
    cases.append({'case':name,'result':'PASS'})

phase_ns={'math':math}
tree=extract(OBSERVER,{'strict_phases'},phase_ns)
strict=phase_ns['strict_phases']
phase_base=dict(operation='return',phase='editor',result='preview_requested',ticks=1,interval_ms=16,gaps_ms=[16.,4.],elapsed_ms=20.,max_gap_ms=16.)
for phase,terminal in [('editor','preview_requested'),('preview','verified'),('post','worker_finished')]:
    for outcome in ('preview_requested','verified','worker_finished','closed','timeout','not_started','stale_or_closed','rejected'):
        answer=strict([{**phase_base,'phase':phase,'result':outcome}])[0]
        check(f'explicit {phase}/{outcome} phase acceptance',answer['measurable_accepted']==(outcome==terminal))
for gaps in ([16.,-1.],[16.,float('nan')],[16.,float('inf')],[16.,True],[],[16.],None):
    check('invalid full gap record '+repr(gaps),not strict([{**phase_base,'gaps_ms':gaps}])[0]['measurable_accepted'])
for row in ({**phase_base,'ticks':0,'gaps_ms':[20.],'max_gap_ms':20.},
            {**phase_base,'ticks':True},{**phase_base,'interval_ms':16.},
            {**phase_base,'operation':'bill'},{**phase_base,'phase':'unknown','result':'worker_finished'}):
    check('rejected/zero/foreign phase '+repr(row),not strict([row])[0]['measurable_accepted'])
terminal={**phase_base,'gaps_ms':[16.,51.],'elapsed_ms':67.,'max_gap_ms':51.}
answer=strict([terminal])[0]
check('terminal51 retained exact50ms miss',answer['measurable_accepted'] and answer['terminal_gap_ms']==51. and answer['budget_ms']==50 and answer['strict_status']=='miss')
for max_gap,wanted in [(50.,'pass'),(50.0001,'miss')]:
    row={**phase_base,'gaps_ms':[16.,max_gap],'elapsed_ms':16.+max_gap,'max_gap_ms':max_gap}
    check('unchanged exact50ms threshold '+str(max_gap),strict([row])[0]['strict_status']==wanted)

calls=[]
clock=SimpleNamespace(now=0.)
clock.monotonic=lambda:clock.now
clock.sleep=lambda n:setattr(clock,'now',clock.now+n)
guardian={'Path':Path,'json':json,'hashlib':hashlib,'time':clock,
          'os':SimpleNamespace(pidfd_open=lambda pid,flags:pid,close=lambda fd:None),
          'signal':SimpleNamespace(SIGTERM=signal.SIGTERM,pidfd_send_signal=lambda fd,sig:calls.append((fd,sig)))}
extract(COORDINATOR,{'same_identity','public_identity','cleanup_owned'},guardian)
binary=Path('/workspace/sarv-evidence/bin/sarv-M6-candidate9')
record=dict(name='owned',pid=500,pgid=500,starttime_ticks=1000,comm='sarv-M6-candida',command=[str(binary)])
current={**record,'state':'S'}
def cleanup(content,identities):
    calls.clear();clock.now=0.
    def identity(pid):
        values=identities.get(pid,[None])
        return values.pop(0) if len(values)>1 else values[0]
    guardian['process_identity']=identity
    with tempfile.TemporaryDirectory() as directory:
        p=Path(directory)/'owned.jsonl'
        if content is not None:p.write_text(content)
        answer=guardian['cleanup_owned'](p,binary,900)
    return answer,list(calls)
encoded=json.dumps(record)+'\n'
for value,name in [(None,'absent'),('','empty'),('\n','blank'),('   \n','whitespace'),('{"truncated":','truncated')]:
    answer,signals=cleanup(value,{})
    check(name+' ledger cannot claim closure',not answer['all_owned_inactive'] and not signals)
for key,value in [('pid',501),('pgid',501),('starttime_ticks',1001),('comm','unrelated SECRET'),('command',['unrelated','SECRET'])]:
    changed={**current,key:value}
    answer,signals=cleanup(encoded,{500:[changed]})
    check('different '+key+' identity untouched and rejected',not signals and not answer['all_owned_inactive'])
    check('different '+key+' identity evidence redacts unexpected text','SECRET' not in json.dumps(answer))
answer,signals=cleanup(encoded,{500:[current,{**current,'starttime_ticks':1001}]})
check('race after pidfd open cannot signal reused identity',not signals and not answer['all_owned_inactive'])
answer,signals=cleanup(encoded,{500:[current,current,None]})
check('exact owned process only SIGTERM',signals==[(500,signal.SIGTERM)] and answer['all_owned_inactive'])
answer,signals=cleanup(encoded,{500:[None]})
check('absent journaled identity needs no signal',not signals and answer['all_owned_inactive'])
answer,signals=cleanup(encoded,{500:[{**current,'state':'Z','command':[]}]})
check('journaled zombie identity inactive',not signals and answer['all_owned_inactive'])
answer,signals=cleanup(encoded+'{"truncated":',{500:[current,current,None]})
check('valid prefix cleaned but incomplete journal rejected',signals==[(500,signal.SIGTERM)] and not answer['all_owned_inactive'])
answer,signals=cleanup(encoded+encoded,{500:[None]})
check('duplicate ledger identity rejected',not signals and not answer['all_owned_inactive'])

def defs(tree):return {n.name:ast.dump(n,include_attributes=False) for n in ast.walk(tree) if isinstance(n,ast.FunctionDef)}
old=defs(ast.parse((BASE/'observer-v2.1-cb145.py').read_text()));new=defs(tree)
for name in ('literal_return','original_prefix','raw_controls','source_result','source_rows','source_action_shape','source_pager_shape','qualify_editor','page_state','turn','user_wins_invalid_reveal','preview_pages'):
    check('unchanged actual AST '+name,old[name]==new[name])
startup=next(n for n in ast.walk(ast.parse(OBSERVER.read_text())) if isinstance(n,ast.FunctionDef) and n.name=='startups')
for n in ast.walk(startup):
    if isinstance(n,ast.Dict):
        pairs=[(k,v) for k,v in zip(n.keys,n.values) if not(isinstance(k,ast.Constant) and k.value=='natural_client_geometry')]
        n.keys=[k for k,v in pairs];n.values=[v for k,v in pairs]
check('startup timing/memory logic unchanged except post-endpoint natural geometry field',old['startups']==ast.dump(startup,include_attributes=False))
workflow=next(n for n in ast.walk(tree) if isinstance(n,ast.FunctionDef) and n.name=='workflow')
text=ast.unparse(workflow)
check('exact all entered original IDs before Preview','assert entered == receipt_ids' in text)
check('exact one accepted positive editor trace','assert len(checks) == 1 and checks[0][\'measurable_accepted\']' in text)
check('preopening baseline checkpoint precedes new app',text.index('before-first-return-app-opening')<text.index('self.app('))
check('narrow geometry actual and natural restoration','self.x11.resize(window, 620, 800)' in text and 'self.x11.resize(window, *standard_geometry)' in text)
check('budget quantiles unmodified actual AST',old['quantiles']==new['quantiles'])
start=next(n for n in ast.walk(tree) if isinstance(n,ast.FunctionDef) and n.name=='start')
check('ledger flush then fsync before native discovery return',ast.unparse(start).index('ledger.flush()')<ast.unparse(start).index('os.fsync(')<ast.unparse(start).index('return process'))
result=dict(result='PASS bounded independent v3 canonical review',cases=cases,total=len(cases),observer_sha256=digest(OBSERVER),coordinator_sha256=digest(COORDINATOR),analyzer_sha256=digest(ANALYZER),no_native_import_GUI_build_network_or_process_signal=True,scope='AST and fake owned identities only; no current native200/performance acceptance')
(OWN/'candidate9-v3-independent-static-result.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
print(json.dumps({k:v for k,v in result.items() if k!='cases'},indent=2))
