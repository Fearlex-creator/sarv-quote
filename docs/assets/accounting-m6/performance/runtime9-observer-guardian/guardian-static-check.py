#!/usr/bin/env python3
"""Actual AST-only phase and guardian adversarial exercises; no GTK or GUI."""
import ast
import copy
import hashlib
import json
import math
from pathlib import Path
import signal
import tempfile
import types

ROOT = Path(__file__).resolve().parents[5]
HERE = Path(__file__).resolve().parent
observer = ROOT / 'scripts/native-performance-M6.py'
coordinator = HERE.parent / 'coordinator.py'
cases = []
def check(name, condition):
    assert condition, name
    cases.append({'case': name, 'result': 'PASS'})
def functions(path, wanted, namespace):
    tree = ast.parse(path.read_text())
    for node in tree.body:
        if isinstance(node, ast.FunctionDef) and node.name in wanted:
            exec(compile(ast.Module(body=[node], type_ignores=[]), str(path), 'exec'), namespace)
    return tree
phase_ns = {'math': math}
tree = functions(observer, {'strict_phases'}, phase_ns)
strict = phase_ns['strict_phases']
base = {'phase': 'editor', 'operation': 'return', 'result': 'preview_requested', 'ticks': 2,
        'interval_ms': 16, 'gaps_ms': [16., 17., 3.], 'elapsed_ms': 36., 'max_gap_ms': 17.}
for phase, outcome in [('editor','preview_requested'), ('preview','verified'), ('post','worker_finished')]:
    row = {**base, 'phase': phase, 'result': outcome}
    check('exact positive complete '+phase+' accepted', strict([row])[0]['measurable_accepted'])
for field, value in [('phase','unknown'), ('result','closed'), ('result','timeout'), ('result','worker_finished'),
                     ('operation','purchase'), ('interval_ms',20), ('interval_ms',16.), ('ticks',True),
                     ('ticks',2.), ('ticks',-1), ('elapsed_ms',float('nan')), ('elapsed_ms',float('inf')),
                     ('elapsed_ms',-1), ('max_gap_ms',float('nan')), ('max_gap_ms',18.), ('elapsed_ms',35.),
                     ('gaps_ms',[]), ('gaps_ms',None), ('gaps_ms','not gaps'), ('gaps_ms',7),
                     ('gaps_ms',[16.,float('nan'),3.]), ('gaps_ms',[16.,float('inf'),3.]),
                     ('gaps_ms',[16.,-1.,3.]), ('gaps_ms',[True,17.,3.]), ('gaps_ms',[16.,20.])]:
    row = copy.deepcopy(base);row[field] = value
    check('reject '+field+'='+repr(value), not strict([row])[0]['measurable_accepted'])
for field in base:
    row=copy.deepcopy(base);del row[field]
    check('missing '+field+' not accepted', not strict([row])[0]['measurable_accepted'])
zero={**base,'ticks':0,'gaps_ms':[4.],'elapsed_ms':4.,'max_gap_ms':4.}
check('zero ticks with valid terminal still unmeasured',not strict([zero])[0]['measurable_accepted'])
miss={**base,'gaps_ms':[16.,17.,51.],'elapsed_ms':84.,'max_gap_ms':51.}
check('terminal51ms retained strict miss',strict([miss])[0]['strict_status']=='miss')
check('unknown phase worker_finished cannot inherit post acceptance',not strict([{**base,'phase':'unknown','result':'worker_finished'}])[0]['measurable_accepted'])

calls=[]
fake_time=types.SimpleNamespace(now=0.)
fake_time.monotonic=lambda:fake_time.now
fake_time.sleep=lambda value:setattr(fake_time,'now',fake_time.now+value)
namespace={'Path':Path,'json':json,'hashlib':hashlib,'signal':types.SimpleNamespace(SIGTERM=signal.SIGTERM,pidfd_send_signal=lambda fd,sig:calls.append((fd,sig))),
           'time':fake_time,'os':types.SimpleNamespace(pidfd_open=lambda pid,flags:pid,close=lambda fd:None)}
functions(coordinator, {'same_identity','public_identity','cleanup_owned'}, namespace)
binary=Path('/workspace/sarv-evidence/bin/sarv-M6-candidate9')
with tempfile.TemporaryDirectory() as directory:
    ledger=Path(directory)/'missing.jsonl'
    missing=namespace['cleanup_owned'](ledger,binary,190)
    check('missing journal fails closure without discovery or signals',missing['journal_missing'] and not missing['all_owned_inactive'] and bool(missing['errors']) and not calls)
    ledger.write_text('')
    empty=namespace['cleanup_owned'](ledger,binary,190)
    check('empty journal fails closure without discovery or signals',empty['journal_empty'] and not empty['all_owned_inactive'] and bool(empty['errors']) and not calls)
record={'name':'new app','pid':100,'pgid':100,'starttime_ticks':200,'comm':'sarv-M6-candida','command':[str(binary)]}
def exercise(records, identities):
    calls.clear();fake_time.now=0.
    def identity(pid):
        values=identities.get(pid,[None])
        return values.pop(0) if len(values)>1 else values[0]
    namespace['process_identity']=identity
    with tempfile.TemporaryDirectory() as directory:
        ledger=Path(directory)/'owned.jsonl';ledger.write_text('\n'.join(json.dumps(row) if isinstance(row,dict) else row for row in records)+'\n')
        result=namespace['cleanup_owned'](ledger,binary,190)
    return result,list(calls)
current={**record,'state':'S'}
r,c=exercise([record],{100:[current,current,None]})
check('only exact owned identity receives SIGTERM',r['all_owned_inactive'] and c==[(100,signal.SIGTERM)])
for field,value in [('pgid',101),('starttime_ticks',201),('comm','other'),('command',['historical app'])]:
    r,c=exercise([record],{100:[{**current,field:value}]})
    check('changed '+field+' untouched/rejected',not c and not r['all_owned_inactive'])
    check('unexpected '+field+' arguments/name redacted', 'command' not in r['records'][0]['before'] and 'comm' not in r['records'][0]['before'])
r,c=exercise([record],{100:[None]})
check('absent owned identity inactive/no signal',r['all_owned_inactive'] and not c)
r,c=exercise([record],{100:[{**current,'state':'Z','command':[]}]})
check('zombie inactive/no signal',r['all_owned_inactive'] and not c)
r,c=exercise([record],{100:[current,current,current]})
check('remaining active owned identity fails closure',not r['all_owned_inactive'] and c==[(100,signal.SIGTERM)])
for changed in [{**record,'starttime_ticks':180},{**record,'pgid':101},{**record,'pid':True},
                {**record,'command':['/historical/binary']},{**record,'comm':''}]:
    r,c=exercise([changed],{})
    check('malformed/historical command or identity rejected '+str(changed),not r['all_owned_inactive'] and not c)
r,c=exercise([record,'{"interrupted":'],{100:[current,current,None]})
check('truncated final journal preserves valid prefix cleanup and rejects completeness',not r['all_owned_inactive'] and c==[(100,signal.SIGTERM)] and len(r['records'])==1)
r,c=exercise([record,record],{100:[None]})
check('duplicate identity rejected',not r['all_owned_inactive'] and not c)
for command in [['openbox','--sm-disable','--config-file','/workspace/sarv-environment/sysroot/etc/xdg/openbox/rc.xml'],
                ['Xvfb','-displayfd','3','-screen','0','1450x1050x24','-nolisten','tcp','-xkbdir','/workspace/sarv-environment/sysroot/usr/share/X11/xkb']]:
    r,c=exercise([{**record,'command':command}],{100:[None]})
    check('exact declared private helper allowlisted '+command[0],r['all_owned_inactive'] and not c)

old=ast.parse((HERE/'observer-v2.1-cb145.py').read_text())
def defs(tree):return {node.name:ast.dump(node,include_attributes=False) for node in ast.walk(tree) if isinstance(node,ast.FunctionDef)}
a,b=defs(old),defs(tree)
for name in ('literal_return','original_prefix','raw_controls','source_result','source_rows','source_action_shape','source_pager_shape',
             'qualify_editor','page_state','turn','user_wins_invalid_reveal','preview_pages'):
    check('unchanged strict membership/data '+name,a[name]==b[name])
imports=next(index for index,node in enumerate(tree.body) if isinstance(node,ast.ImportFrom) and node.module=='qa_access')
enable=next(index for index,node in enumerate(tree.body) if isinstance(node,ast.Expr) and ast.unparse(node)=='faulthandler.enable(all_threads=True)')
check('faulthandler enabled before native imports',enable<imports)
workflow=next(node for node in ast.walk(tree) if isinstance(node,ast.FunctionDef) and node.name=='workflow')
calls_text=[ast.unparse(node) for node in ast.walk(workflow) if isinstance(node,ast.Call)]
check('workflow durable pre-opening checkpoint present',any('before-first-return-app-opening' in text for text in calls_text))
receipt={'observer_sha256':hashlib.sha256(observer.read_bytes()).hexdigest(),'coordinator_sha256':hashlib.sha256(coordinator.read_bytes()).hexdigest(),
         'no_native_imports_or_gui':True,'cases':cases,'total':len(cases)}
(HERE/'guardian-static-check.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2)+'\n')
print(json.dumps({'pass':len(cases),'observer_sha256':receipt['observer_sha256'],'coordinator_sha256':receipt['coordinator_sha256']}))
