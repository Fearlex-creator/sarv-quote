#!/usr/bin/env python3
"""Isolated AST predicate exercises: no observer imports, GTK, GUI, or build."""
import ast
import hashlib
import json
import re
from pathlib import Path
from collections import deque

ROOT = Path(__file__).resolve().parents[5]
source = ROOT / 'scripts/native-performance-M6.py'
tree = ast.parse(source.read_text())
wanted = {'normalize', 'current_node', 'CurrentPaths', 'walk', 'member'}
namespace = {'re': re, 'deque': deque, 'ISOLATION': dict.fromkeys(map(ord, '\u2066\u2067\u2068\u2069\u200e\u200f'), None), 'DIGITS': str.maketrans('۰۱۲۳۴۵۶۷۸۹٠١٢٣٤٥٦٧٨٩', '01234567890123456789'), 'QUANTITY_PREFIX': 'مقدار برگشت در واحد پایه • '}
namespace['ready'] = lambda access, row, root, focus=False: bool(row and not access.nodes[row['node']].get('defunct') and access.nodes[row['node']]['pid'] == access.pid)
namespace['legacy'] = type('Legacy', (), {'ready': staticmethod(namespace['ready'])})
namespace['time'] = type('Time', (), {'monotonic_ns': staticmethod(lambda: 1)})
for node in tree.body:
    if getattr(node, 'name', None) in wanted:
        exec(compile(ast.Module(body=[node], type_ignores=[]), str(source), 'exec'), namespace)
observer = next(node for node in tree.body if isinstance(node, ast.ClassDef) and node.name == 'Observer')
methods = [node for node in observer.body if isinstance(node, ast.FunctionDef) and node.name in {'source_kind', 'source_action_shape', 'source_pager_shape', 'reject_source', 'source_rows', 'source_result', 'qualify_editor', 'page_state', 'focused_control'}]
exec(compile(ast.fix_missing_locations(ast.Module(body=[ast.ClassDef(name='Predicates', bases=[], keywords=[], body=methods, decorator_list=[])], type_ignores=[])), str(source), 'exec'), namespace)

class Fake:
    def __init__(self):
        self.pid = 99
        self.nodes = {}
        self.calls = {}
        self.hook = None
        self.a = self
    def tick(self, kind, node):
        key = (kind, node)
        self.calls[key] = self.calls.get(key, 0) + 1
        if self.hook:
            self.hook(kind, node, self.calls[key])
    def add(self, node, parent, role, name='', text=''):
        self.nodes[node] = {'parent': parent, 'role': role, 'name': name, 'text': text, 'pid': self.pid, 'children': []}
        if parent in self.nodes:
            self.nodes[parent]['children'].append(node)
    def atspi_accessible_clear_cache(self, node):
        self.tick('clear', node)
    def atspi_accessible_get_state_set(self, node):
        return node
    def atspi_state_set_contains(self, node, state):
        return self.nodes[node].get('defunct', False)
    def atspi_accessible_get_process_id(self, node, _):
        return self.nodes[node]['pid']
    def atspi_accessible_get_parent(self, node, _):
        return self.nodes[node]['parent']
    def atspi_accessible_get_role_name(self, node, _):
        self.tick('role', node)
        return self.nodes[node]['role']
    def atspi_accessible_get_name(self, node, _):
        self.tick('name', node)
        return self.nodes[node]['name']
    def string(self, function, *args):
        return function(*args)
    def children(self, node):
        self.tick('children', node)
        return list(self.nodes[node]['children'])
    def focused(self, row):
        return self.nodes[row['node']].get('focused', False)
    def text(self, row):
        self.tick('text', row['node'])
        return self.nodes[row['node']]['text']

def common():
    f = Fake(); f.add(1, 0, 'application'); f.add(2, 1, 'frame'); f.add(3, 2, 'dialog')
    o = namespace['Predicates'](); o.report = {'rejected_source_observations': []}; o.source_paths = {}; o.editor_paths = {}
    return f, o

def source_case():
    f, o = common(); f.add(4, 3, 'panel'); f.add(5, 3, 'entry', 'Search', 'PERF-A')
    f.add(6, 4, 'label', '60 منبع قابل برگشت • صفحهٔ 1')
    for node in range(10, 35):
        f.add(node, 4, 'button', 'صورتحساب \u2068A060\u2069')
        f.add(1000+node,node,'label',f.nodes[node]['name'])
    f.add(7, 4, 'table'); f.add(8, 7, 'table cell', 'منابع بعد'); f.add(9,8,'button','منابع بعد'); f.add(90,9,'label','منابع بعد')
    o.source_paths[4] = namespace['CurrentPaths'](f, 3, [4, 5]); o.source_paths[4].bar_role = 'table'; o.source_paths[4].action_shapes = {(((), 'button'), ((0,), 'label'))}; f.calls = {}
    return f, o

def rows(f, o):
    return o.source_rows(f, 4, 3, 'static-case')

def result(f, o):
    return o.source_result(f, {'node': 5}, 3, 4, 'PERF-A', 'A060')

cases = []
def check(name, predicate):
    assert predicate(), name
    cases.append({'case': name, 'result': 'pass'})

f,o=source_case();check('exact direct status/25 actions/reference/query',lambda: bool(result(f,o)))
f,o=source_case();f.nodes[10]['defunct']=True;check('retired direct action rejected + event retained',lambda: rows(f,o) is None and len(o.report['rejected_source_observations'])==1)
f,o=source_case();f.nodes[10]['pid']=100;check('foreign action rejected',lambda: rows(f,o) is None)
f,o=source_case();f.nodes[10]['parent']=3;check('wrong action parent rejected',lambda: rows(f,o) is None)
f,o=source_case();f.nodes[4]['defunct']=True;check('retired common rows path rejected',lambda: rows(f,o) is None)
f,o=source_case();f.nodes[2]['pid']=100;check('foreign modal attachment rejected',lambda: rows(f,o) is None)
f,o=source_case();f.hook=lambda kind,node,n: f.nodes[4]['children'].remove(10) if (kind,node,n)==('children',4,2) else None;check('child retired between before/after lists rejected',lambda: rows(f,o) is None)
f,o=source_case();f.hook=lambda kind,node,n: f.nodes[10].update(name='صورتحساب \u2068OLD\u2069') if (kind,node,n)==('name',10,2) else None;check('action name changes during snapshot rejected',lambda: rows(f,o) is None)
f,o=source_case();f.hook=lambda kind,node,n: f.nodes[5].update(text='PERF-B') if (kind,node,n)==('text',5,2) else None;check('query changes after row read rejected',lambda: result(f,o) is None)
f,o=source_case();f.nodes[10]['name']='صورتحساب A060';check('missing exact FSI reference rejected',lambda: result(f,o) is None)
f,o=source_case();f.nodes[8]['name']='صورتحساب \u2068A060\u2069';check('unknown nested source action rejected',lambda: rows(f,o) is None)

def editor_case():
    f,o=common();f.add(4,3,'panel');f.add(5,4,'label','ردیف‌های برگشت 1 تا 25 از 50 • صفحهٔ 1')
    identities=[f'L{i:03}' for i in range(1,51)]
    for i,node in enumerate(range(10,35)):
        f.add(node,4,'entry',namespace['QUANTITY_PREFIX']+identities[i])
    o.qualify_editor(f,3,identities); f.calls={}
    return f,o,identities

def page(f,o,ids,n=1):
    return o.page_state(f,3,ids,n)

f,o,ids=editor_case();check('qualified 25 initial entries exact empty text',lambda: bool(page(f,o,ids)))
f,o,ids=editor_case();f.nodes[5]['name']='ردیف‌های برگشت 26 تا 50 از 50 • صفحهٔ 2'
for i,node in enumerate(range(10,35)): f.nodes[node]['name']=namespace['QUANTITY_PREFIX']+ids[25+i]
check('complete persistent page rebind accepted',lambda: bool(page(f,o,ids,2)))
f,o,ids=editor_case();f.nodes[10]['name']=namespace['QUANTITY_PREFIX']+ids[25];check('mixed persistent names rejected',lambda: page(f,o,ids) is None)
f,o,ids=editor_case();f.nodes[10]['text']='1';check('mixed stale retained text rejected',lambda: page(f,o,ids) is None)
f,o,ids=editor_case();f.nodes[4]['defunct']=True;check('editor ancestor DEFUNCT rejected',lambda: page(f,o,ids) is None)
f,o,ids=editor_case();f.nodes[10]['pid']=100;check('foreign persistent entry rejected',lambda: page(f,o,ids) is None)
f,o,ids=editor_case();f.nodes[10]['parent']=3;check('persistent entry current parent edge rejected',lambda: page(f,o,ids) is None)
f,o,ids=editor_case();f.nodes[4]['children'][1:3]=f.nodes[4]['children'][2:0:-1];check('stable slot ordering change rejected',lambda: page(f,o,ids) is None)
f,o,ids=editor_case();f.hook=lambda kind,node,n: f.nodes[5].update(name='ردیف‌های برگشت 26 تا 50 از 50 • صفحهٔ 2') if (kind,node,n)==('text',10,1) else None;check('marker changes during text read rejected',lambda: page(f,o,ids) is None)
f,o,ids=editor_case();f.hook=lambda kind,node,n: f.nodes[10].update(text='1') if (kind,node,n)==('text',10,2) else None;check('text changes on final read rejected',lambda: page(f,o,ids) is None)
f,o=source_case();f.nodes[6]['name']='60 منبع قابل برگشت • صفحهٔ 2';check('exact current page mismatch rejected',lambda: result(f,o) is None)
f,o=source_case();f.nodes[6]['role']='button';check('status role mismatch rejected',lambda: rows(f,o) is None)
f,o=source_case();f.nodes[7]['role']='entry';check('unexpected pager-bar role rejected',lambda: rows(f,o) is None)
f,o=source_case();f.add(91,7,'table cell','منابع بعد');f.add(92,91,'button','منابع بعد');f.add(93,92,'label','منابع بعد');check('duplicate pager order rejected',lambda: rows(f,o) is None)
f,o=source_case();original_ready=namespace['ready']
def mutating_ready(access,row,root,focus=False):
    if row.get('node')==10:
        access.nodes[6]['name']='60 منبع قابل برگشت • صفحهٔ 2'
    return original_ready(access,row,root,focus)
namespace['ready']=mutating_ready
check('status rebind during readiness rejected',lambda: result(f,o) is None)
namespace['ready']=original_ready
f,o,ids=editor_case();f.nodes[10]['role']='label';check('persistent entry role change rejected',lambda: page(f,o,ids) is None)
f,o,ids=editor_case();f.nodes[5]['defunct']=True;check('persistent page marker DEFUNCT rejected',lambda: page(f,o,ids) is None)
f,o=common();f.add(10,3,'entry','actual-control');f.nodes[10]['focused']=True
check('unique actual owned focus identity and ancestry',lambda: o.focused_control(f,3)['name']=='actual-control')
f,o=common();f.add(10,3,'entry','actual-control');f.nodes[10].update(focused=True,pid=100)
check('foreign focused control rejected',lambda: o.focused_control(f,3) is None)
f,o=common();f.add(10,3,'entry','actual-control');f.nodes[10].update(focused=True,defunct=True)
check('DEFUNCT focused control rejected',lambda: o.focused_control(f,3) is None)
f,o=common();f.add(10,3,'entry','first');f.add(11,3,'entry','second');f.nodes[10]['focused']=f.nodes[11]['focused']=True
check('ambiguous actual focus rejected',lambda: o.focused_control(f,3) is None)
user_probe=next(n for n in observer.body if isinstance(n,ast.FunctionDef) and n.name=='user_wins_invalid_reveal')
probe_body=next(n for n in user_probe.body if isinstance(n,ast.Try)).body
keys=[i for i,n in enumerate(probe_body) if isinstance(n,ast.Expr) and isinstance(n.value,ast.Call) and isinstance(n.value.func,ast.Attribute) and n.value.func.attr=='key']
check('real Space and Tab calls immediately consecutive',lambda:len(keys)==2 and keys[1]==keys[0]+1 and [probe_body[i].value.args[0].value for i in keys]==['space','Tab'])
check('user-wins failure records and checkpoints then raises',lambda:any(isinstance(n,ast.Raise) for n in next(n for n in user_probe.body if isinstance(n,ast.Try)).handlers[0].body))
f,o=source_case();f.add(100,10,'button','صورتحساب \u2068EXTRA\u2069');check('extra nested source action under current direct action rejected',lambda:result(f,o) is None)
f,o=source_case();f.nodes[10]['children']=[];f.add(100,10,'label',f.nodes[10]['name']);o.source_paths[4].action_shapes.add((((),'button'),((0,),'label')))
check('qualified button label child accepted',lambda:bool(result(f,o)))
f,o=source_case();f.nodes[10]['children']=[];f.add(100,10,'label',f.nodes[10]['name']);o.source_paths[4].action_shapes.add((((),'button'),((0,),'label')));f.nodes[100]['parent']=4
check('button child wrong parent rejected',lambda:result(f,o) is None)
f,o=source_case();f.nodes[10]['children']=[];f.add(100,10,'label',f.nodes[10]['name']);o.source_paths[4].action_shapes.add((((),'button'),((0,),'label')));f.nodes[100]['defunct']=True
check('button child DEFUNCT rejected',lambda:result(f,o) is None)
f,o=source_case();f.nodes[10]['children']=[];f.add(100,10,'label',f.nodes[10]['name']);o.source_paths[4].action_shapes.add((((),'button'),((0,),'label')));f.hook=lambda kind,node,n:f.nodes[100].update(name='changed label') if (kind,node,n)==('name',100,2) else None
check('button child label changed during observation rejected',lambda:result(f,o) is None)
f,o=source_case();check('action child-shape qualification from discovery-only metadata',lambda:o.source_action_shape(f,{'node':10,'name':f.nodes[10]['name'],'role':'button'}) is not None)
f,o=source_case();f.nodes[8]['parent']=3;check('FlowBox wrapper wrong current parent rejected',lambda:result(f,o) is None)
f,o=source_case();f.nodes[8]['defunct']=True;check('FlowBox wrapper DEFUNCT rejected',lambda:result(f,o) is None)
f,o=source_case();f.nodes[8]['pid']=100;check('FlowBox wrapper foreign PID rejected',lambda:result(f,o) is None)
f,o=source_case();f.nodes[8]['role']='panel';check('unknown FlowBox wrapper role rejected',lambda:result(f,o) is None)
f,o=source_case();f.nodes[9]['parent']=7;check('pager button wrong cell parent rejected',lambda:result(f,o) is None)
f,o=source_case();f.add(91,8,'button','منابع بعد');check('extra button in wrapper rejected',lambda:result(f,o) is None)
f,o=source_case();f.nodes[8]['name']='منابع پیشین';check('wrapper/button literal name mismatch rejected',lambda:result(f,o) is None)
f,o=source_case();f.hook=lambda kind,node,n:f.nodes[8].update(name='retired') if (kind,node,n)==('name',8,2) else None;check('wrapper changes between snapshots rejected',lambda:result(f,o) is None)
f,o=source_case();f.nodes[8]['name']=f.nodes[9]['name']=f.nodes[90]['name']='منابع پیشین';check('same strict current wrapper shape accepts Previous',lambda:bool(result(f,o)))
f,o=source_case();f.nodes[7]['children']=[];check('zero-pager boundary current table accepted',lambda:bool(result(f,o)))
f,o=source_case();f.add(100,6,'button','صورتحساب \u2068EXTRA\u2069');check('hidden nested source action under status rejected',lambda:result(f,o) is None)
f,o=source_case();f.nodes[7]['name']='unexpected';check('nonempty FlowBox bar name rejected',lambda:result(f,o) is None)
f,o=source_case();f.nodes[1010]['name']='different';check('source button label full-name mismatch rejected',lambda:result(f,o) is None)
f,o=source_case();f.add(100,1010,'button','صورتحساب \u2068EXTRA\u2069');check('source caption label must be childless',lambda:result(f,o) is None)
f,o=source_case();f.add(100,90,'button','صورتحساب \u2068EXTRA\u2069');check('pager caption label must be childless',lambda:result(f,o) is None)
f,o=source_case();f.nodes[90]['name']='different';check('pager button label full-name mismatch rejected',lambda:result(f,o) is None)
f,o=source_case();f.nodes[10]['name']=f.nodes[1010]['name']='صورتحساب A060';check('coherent caption missing exact FSI reference rejected',lambda:result(f,o) is None)
old=ast.parse((ROOT/'docs/assets/accounting-m6/performance/runtime7-rpc-refinement/observer-v1.2.py').read_text())
def definitions(tree):
    return {n.name: ast.dump(n, include_attributes=False) for n in ast.walk(tree) if isinstance(n,ast.FunctionDef)}
a,b=definitions(old),definitions(tree)
for name in ('literal_return','strict_phases','original_prefix','raw_controls','preview_pages','startups'):
    check('unchanged '+name,lambda name=name:a[name]==b[name])
receipt={'observer_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'protocol':'m6-scoped-v2.1-proposal-only','no_native_imports_or_gui':True,'cases':cases,'total':len(cases)}
(Path(__file__).parent/'static-check.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2)+'\n')
print(json.dumps({'pass':len(cases),'observer_sha256':receipt['observer_sha256']}))
