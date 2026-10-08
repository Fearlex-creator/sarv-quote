from pathlib import Path
import ast,json,hashlib,re,types
ROOT=Path('/workspace/sarv-quote');source=ROOT/'scripts/qa-native.py';before=hashlib.sha256(source.read_bytes()).hexdigest();tree=ast.parse(source.read_text());function=next(n for n in ast.walk(tree) if isinstance(n,ast.FunctionDef) and n.name=='current_focus');function.body=[ast.Global(names=['settled_focus']) if isinstance(n,ast.Nonlocal) else n for n in function.body];module=ast.fix_missing_locations(ast.Module(body=[function],type_ignores=[]));code=compile(module,str(source),'exec');name='تعداد ردیف ۱۰۰';original={'key':'Up','focus':'row100-quantity'}
class Probe:
 def __init__(self,case):self.case=case;self.a=self;self.clear=set();self.focus_calls=0
 def tree(self):return [{'node':1,'name':name,'role':'text'}]*(2 if self.case=='duplicate' else 1)
 def focused(self,row):self.focus_calls+=1;return self.case!='focus-lost' or self.focus_calls==1
 def atspi_accessible_clear_cache(self,node):self.clear.add(node)
 def atspi_accessible_get_state_set(self,node):return node
 def atspi_state_set_contains(self,node,state):return state==6 and ((self.case=='field-defunct' and node==1) or (self.case=='parent-defunct' and node==2))
 def atspi_accessible_get_parent(self,node,_):return {1:2,2:3}.get(node)
 def children(self,parent):
  assert parent in self.clear,'parent was not refreshed'
  self.clear.remove(parent)
  return [] if self.case=='retired' and parent==2 else {2:[1],3:[2]}.get(parent,[])
 def app(self):return 4 if self.case=='foreign-app' else 3
 def atspi_accessible_get_name(self,*args):raise AssertionError('string stub owns call')
 def string(self,*args):return 'different' if self.case=='renamed' else name
 def text(self,field):return '2.375'
results=[]
for case in ['live','retired','field-defunct','parent-defunct','focus-lost','renamed','duplicate','foreign-app']:
 probe=Probe(case);space={'settled_focus':None,'events':lambda profile:[],'profile':None,'count':0,'focus':'row99-quantity','re':re,'Access':lambda pid:probe,'app':types.SimpleNamespace(pid=123),'seen':[original]};exec(code,space);result=space['current_focus']();accepted=result is not None;assert accepted==(case=='live'),case
 if accepted:assert result==original and space['settled_focus']['original_key_event']==original and space['settled_focus']['requested_focus']=='row99-quantity'
 results.append({'case':case,'accepted':accepted,'correct':True,'observation':space['settled_focus']})
assert hashlib.sha256(source.read_bytes()).hexdigest()==before
out=ROOT/'docs/assets/accounting-m6/checks/candidate6/preserved-quote-observer-static';(out/'logic-check.json').write_text(json.dumps({'passed':True,'scope':'Isolated actual AST inner function with adversarial probe doubles; no native imports, GUI, keys or focus changes. This is static logic evidence, not native fallback exercise.','observer_sha256':before,'cases':results},ensure_ascii=False,indent=2)+'\n');print('PASS eight isolated current-focus cases; source stable')
