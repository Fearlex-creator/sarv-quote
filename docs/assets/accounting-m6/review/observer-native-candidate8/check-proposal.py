#!/usr/bin/env python3
"""Independent actual-AST review; imports no observer/native modules."""
import ast
import hashlib
import json
import re
import sys
from collections import deque
from pathlib import Path
from types import SimpleNamespace

ROOT = Path(__file__).resolve().parents[5]
BASE = ROOT / 'docs/assets/accounting-m6/performance/candidate8-source-hierarchy-probe'
CANONICAL = '--canonical' in sys.argv
SOURCE = ROOT / 'scripts/native-performance-M6.py' if CANONICAL else BASE / 'observer-proposal.py'
EXPECTED_SHA = 'cb145aed5a13d289f8222083f3d23e67ef9c95759745c2d4aea9ecc391945408'
assert hashlib.sha256(SOURCE.read_bytes()).hexdigest() == EXPECTED_SHA
tree = ast.parse(SOURCE.read_text())
ns = {'re': re, 'deque': deque,
      'ISOLATION': dict.fromkeys(map(ord, '\u2066\u2067\u2068\u2069\u200e\u200f'), None),
      'DIGITS': str.maketrans('۰۱۲۳۴۵۶۷۸۹٠١٢٣٤٥٦٧٨٩', '01234567890123456789'),
      'time': SimpleNamespace(monotonic_ns=lambda: 1)}
for n in tree.body:
    if getattr(n, 'name', None) in {'normalize', 'current_node', 'CurrentPaths', 'member', 'ready'}:
        exec(compile(ast.Module(body=[n], type_ignores=[]), str(SOURCE), 'exec'), ns)
observer = next(n for n in tree.body if isinstance(n, ast.ClassDef) and n.name == 'Observer')
methods = [n for n in observer.body if getattr(n, 'name', None) in
           {'source_kind', 'source_action_shape', 'source_pager_shape', 'reject_source', 'source_rows', 'source_result'}]
cls = ast.ClassDef(name='Predicates', bases=[], keywords=[], body=methods, decorator_list=[])
exec(compile(ast.fix_missing_locations(ast.Module(body=[cls], type_ignores=[])), str(SOURCE), 'exec'), ns)

class Access:
    def __init__(self):
        self.pid = 123
        self.nodes = {}
        self.a = self
        self.calls = {}
        self.hook = lambda kind, node, count: None
    def tick(self, kind, node):
        key = (kind, node)
        self.calls[key] = self.calls.get(key, 0) + 1
        self.hook(kind, node, self.calls[key])
    def add(self, node, parent, role, name='', text=''):
        self.nodes[node] = dict(parent=parent, role=role, name=name, text=text,
                                pid=self.pid, children=[], defunct=False,
                                visible=True, sensitive=True, allocated=True, focused=True)
        if parent in self.nodes:
            self.nodes[parent]['children'].append(node)
    def children(self, node):
        self.tick('children', node)
        return list(self.nodes[node]['children'])
    def atspi_accessible_clear_cache(self, node): self.tick('clear', node)
    def atspi_accessible_get_state_set(self, node): return node
    def atspi_state_set_contains(self, node, state): return self.nodes[node]['defunct']
    def atspi_accessible_get_process_id(self, node, unused): return self.nodes[node]['pid']
    def atspi_accessible_get_parent(self, node, unused): return self.nodes[node]['parent']
    def atspi_accessible_get_role_name(self, node, unused):
        self.tick('role', node)
        return self.nodes[node]['role']
    def atspi_accessible_get_name(self, node, unused):
        self.tick('name', node)
        return self.nodes[node]['name']
    def string(self, function, *args): return function(*args)
    def text(self, row):
        self.tick('text', row['node'])
        return self.nodes[row['node']]['text']

def legacy_ready(access, row, focused=False):
    n = access.nodes[row['node']]
    return n['visible'] and n['sensitive'] and n['allocated'] and (not focused or n['focused'])
ns['legacy'] = SimpleNamespace(ready=legacy_ready)

def fixture():
    a = Access()
    a.add(999, 0, 'application', 'foreign-detached')
    a.nodes[999]['pid'] = 999
    for node, parent, role in [(1, 0, 'application'), (2, 1, 'frame'), (3, 2, 'dialog'), (4, 3, 'panel')]:
        a.add(node, parent, role)
    a.add(5, 3, 'entry', 'Search', 'PERF-A')
    a.add(6, 4, 'label', '60 منبع قابل برگشت • صفحهٔ 1')
    for i in range(25):
        a.add(10+i, 4, 'button', f'صورتحساب \u2068A{60-i:03}\u2069')
        a.add(1000+i, 10+i, 'label', a.nodes[10+i]['name'])
    a.add(7, 4, 'table')
    a.add(8, 7, 'table cell', 'منابع بعد')
    a.add(9, 8, 'button', 'منابع بعد')
    a.add(90, 9, 'label', 'منابع بعد')
    o = ns['Predicates']()
    o.report = {'rejected_source_observations': []}
    o.source_paths = {4: ns['CurrentPaths'](a, 3, [4, 5])}
    o.source_paths[4].bar_role = 'table'
    o.source_paths[4].action_shapes = {(((), 'button'), ((0,), 'label'))}
    a.calls.clear()
    return a, o

def result(a, o, focused=False):
    return o.source_result(a, {'node': 5}, 3, 4, 'PERF-A', 'A060', focused=focused)

cases = []
def run(name, mutate=lambda a,o: None, accept=False, focused=False):
    a, o = fixture()
    mutate(a, o)
    answer = result(a, o, focused)
    assert bool(answer) == accept, (name, answer)
    cases.append({'case': name, 'result': 'pass', 'expected': 'accept' if accept else 'reject'})

run('exact distinct ordered source captions and FSI first reference', accept=True)
for node, label in [(3,'modal'),(4,'rows'),(6,'status'),(7,'bar'),(8,'cell'),(9,'pager'),(90,'pager label'),(10,'source'),(1000,'source label')]:
    for field, value in [('defunct', True), ('pid', 999), ('parent', 999)]:
        run(f'{label} {field} rejected', lambda a,o,n=node,f=field,v=value:a.nodes[n].update({f:v}))
for node, label in [(6,'status'),(7,'bar'),(8,'cell'),(9,'pager'),(90,'pager label'),(10,'source'),(1000,'source label')]:
    run(f'{label} unknown role rejected', lambda a,o,n=node:a.nodes[n].update(role='unknown'))
for node, label in [(6,'status'),(90,'pager label'),(1000,'source label')]:
    for role in ('button','label','panel'):
        run(f'hidden {role} under {label} rejected',lambda a,o,n=node,r=role:a.add(2000,n,r,'صورتحساب \u2068HIDDEN\u2069'))
for node, label in [(8,'cell'),(9,'pager'),(10,'source')]:
    run(f'extra same-named child under {label} rejected',lambda a,o,n=node:a.add(2000,n,'label',a.nodes[n]['name']))
for node in (8,90,1000):
    run(f'literal full name mismatch node {node} rejected',lambda a,o,n=node:a.nodes[n].update(name='different'))
run('third wrapper rejected',lambda a,o: (a.add(2000,7,'table cell','منابع پیشین'),a.add(2001,2000,'button','منابع پیشین'),a.add(2002,2001,'label','منابع پیشین'),a.add(2003,7,'table cell','منابع بعد')))
run('bare direct pager rejected',lambda a,o:a.nodes[7].update(children=[9]))
run('source button hidden in pager wrapper rejected',lambda a,o:(a.nodes[8].update(name=a.nodes[10]['name']),a.nodes[9].update(name=a.nodes[10]['name']),a.nodes[90].update(name=a.nodes[10]['name'])))
for field in ('visible','sensitive','allocated'):
    for node in (5,10):
        run(f'readiness {field} false node {node} rejected',lambda a,o,n=node,f=field:a.nodes[n].update({f:False}))
run('focused source required and false rejected',lambda a,o:a.nodes[10].update(focused=False),focused=True)
run('coherent missing FSI rejected',lambda a,o:(a.nodes[10].update(name='صورتحساب A060'),a.nodes[1000].update(name='صورتحساب A060')))
run('wrong exact count rejected',lambda a,o:a.nodes[6].update(name='61 منبع قابل برگشت • صفحهٔ 1'))
run('wrong exact query rejected',lambda a,o:a.nodes[5].update(text='PERF-B'))
run('wrong exact page rejected',lambda a,o:a.nodes[6].update(name='60 منبع قابل برگشت • صفحهٔ 2'))
for node in (8,9,90,1000):
    for field,value in [('defunct',True),('pid',999),('parent',999),('role','unknown'),('name','retired')]:
        def mutation(a,o,n=node,f=field,v=value):
            def hook(kind, target, count):
                if (kind,target,count)==('clear',n,3):
                    a.nodes[n][f]=v
            a.hook=hook
        run(f'before/after {field} mutation node {node} rejected',mutation)
run('status gains hidden child at final child read',lambda a,o:setattr(a,'hook',lambda k,n,c:a.add(2000,6,'button','hidden') if (k,n,c)==('children',6,2) else None))
run('pager label gains hidden child at final read',lambda a,o:setattr(a,'hook',lambda k,n,c:a.add(2000,90,'button','hidden') if (k,n,c)==('children',90,2) else None))
run('source label gains hidden child at final read',lambda a,o:setattr(a,'hook',lambda k,n,c:a.add(2000,1000,'button','hidden') if (k,n,c)==('children',1000,2) else None))

# Replay the recorded genuine initial hierarchy through actual AST source_rows.
probe = json.loads((BASE/'result/hierarchy-probe.json').read_text())
assert probe['hierarchy_before'] == probe['hierarchy_after'] and len(probe['hierarchy_before']) == 56
a = Access()
a.pid = probe['owned_pid']
for r in probe['hierarchy_before']:
    n = int(r['node'],16)
    a.nodes[n] = dict(parent=int(r['parent'],16), role=r['role'], name=r['name'],
                     text='',pid=r['process_id'],defunct=r['DEFUNCT'],
                     children=[int(c,16) for c in r['children']])
rows = int(probe['rows_node'],16)
root = int(probe['root_node'],16)
# Actual common modal ancestry was qualified by the sidecar; this replay checks
# every recorded owned node/edge inside the 56-node subtree, never timings.
class RecordedPaths:
    bar_role='table'
    action_shapes={(((), 'button'), ((0,), 'label'))}
    def snapshot(self, access): return {'recorded_common_path': True}
    def coherent(self, before, after): return before == after
o=ns['Predicates']();o.report={'rejected_source_observations':[]};o.source_paths={rows:RecordedPaths()}
answer=o.source_rows(a,rows,root,'independent-actual-56-node-replay')
assert answer and len(answer[0])==1 and len(answer[1])==25
cases.append({'case':'actual 56-node recorded GTK subtree accepted through actual AST', 'result':'pass', 'expected':'accept'})

held=ast.parse((ROOT/'docs/assets/accounting-m6/checks/candidate8/runtime-verification1-bytes/scripts/native-performance-M6.py').read_text())
def functions(t):
    return {n.name:ast.dump(n,include_attributes=False) for n in ast.walk(t) if isinstance(n,ast.FunctionDef)}
old,new=functions(held),functions(tree)
changed=[name for name in old if old[name]!=new[name]]
assert set(changed)=={'source_action_shape','source_rows'}
assert set(new)-set(old)=={'source_pager_shape'}
receipt=dict(result='pass', reviewer='m6_observer_native_reviewer', observer_sha256=EXPECTED_SHA, canonical_installed_AST=CANONICAL,
             actual_ast=True,no_native_imports_gui_build_network=True,cases=cases,total=len(cases),
             changed_existing_functions=changed,new_function='source_pager_shape',
             preserved_query_count_page_FSI_readiness_focus=True,
             recorded_hierarchy_nodes=56, common_path_replay_limit='Sidecar-qualified common modal path; recorded subtree replay only',
             no_native_or_performance_acceptance_claim=True)
(Path(__file__).parent/('independent-canonical-check.json' if CANONICAL else 'independent-proposal-check.json')).write_text(json.dumps(receipt,ensure_ascii=False,indent=2)+'\n')
print(json.dumps({'result':'pass','cases':len(cases),'observer_sha256':EXPECTED_SHA}))
