import sys,os,json,time,tempfile,subprocess,hashlib
from pathlib import Path
from importlib.util import spec_from_file_location,module_from_spec
root=Path('/workspace/sarv-quote');sys.path.insert(0,str(root/'scripts'))
spec=spec_from_file_location('m5',root/'scripts/native-performance-M5.py');m=module_from_spec(spec);spec.loader.exec_module(m)
out=root/'artifacts/performance-m5/candidate7-cache-hide-diagnostic';out.mkdir(exist_ok=False)
processes=[];logs=[];report={'functional_diagnostic_only':True,'accepted_measurement':False,'observations':[],'observer_sha256':m.helper.sha(root/'scripts/native-performance-M5.py'),'source_before':m.frozen_source_provenance()}
binary=Path('/workspace/sarv-evidence/bin/sarv-M5-candidate7');report['binary_sha256']=m.helper.sha(binary)
def start(cmd,name,env=None,**kw):
 f=(out/(name+'.log')).open('w');logs.append(f);p=subprocess.Popen(cmd,env=env,stdout=f,stderr=f,start_new_session=True,**kw);processes.append(p);return p
try:
 with tempfile.TemporaryFile() as fd:
  start(['Xvfb','-displayfd',str(fd.fileno()),'-screen','0','1450x1050x24','-nolisten','tcp','-xkbdir',str(m.helper.ENVROOT/'sysroot/usr/share/X11/xkb')],'xvfb',pass_fds=(fd.fileno(),))
  def num():fd.seek(0);return fd.read().decode().strip()
  display=':'+m.native.wait_for(num,label='display')
 runtime=m.native.private_accessibility_environment(display,out)
 env={k:v for k,v in os.environ.items() if not k.startswith('SARV_')};env.update(DISPLAY=display,GDK_BACKEND='x11',GSK_RENDERER='cairo',XDG_RUNTIME_DIR=str(runtime),SARV_TEST_INSTANCE='m5-cache-hide-diagnostic')
 start(['openbox','--sm-disable','--config-file',str(m.helper.ENVROOT/'sysroot/etc/xdg/openbox/rc.xml')],'openbox',env)
 profile=out/'profile';m.copy_database(root/'artifacts/performance-m5/candidate7-functional-pilot/sales-native-full',profile);baseline=m.helper.payload(profile)
 env['SARV_DATA_DIR']=str(profile);app=start([str(binary)],'native',env);x11=m.native.X11(display)
 window=m.native.wait_for(lambda:x11.find(app.pid),label='owned window');x11.focus(window)
 access=m.Access(app.pid);page,search=m.sales_scope(access,x11);meta={};cache=m.discover_sales_page_cache(access,page,search,meta);report['cache_metadata']=meta
 def raw_node(node):
  access.a.atspi_accessible_clear_cache(node);state=access.a.atspi_accessible_get_state_set(node)
  return {'node':hex(node),'pid':access.a.atspi_accessible_get_process_id(node,None),'name':access.string(access.a.atspi_accessible_get_name,node,None),'role':access.string(access.a.atspi_accessible_get_role_name,node,None),'parent':hex(access.a.atspi_accessible_get_parent(node,None) or 0),'states':{name:bool(state and access.a.atspi_state_set_contains(state,s)) for name,s in [('defunct',6),('sensitive',24),('showing',25),('visible',30),('focused',12)]},'rect':access.rect({'node':node})}
 def capture(label,pump=True):
  if pump:access.pump()
  nodes={key:raw_node(row['node']) for key,row in cache.nodes.items()}
  ancestors=[]
  for node,parent in cache.edges.items():
   ancestors.append({'child':hex(node),'current_parent':raw_node(parent),'parent_children_contains_child':node in access.children(parent),'parent_children':[hex(c) for c in access.children(parent)]})
  rows=[];queue=[page];seen=set()
  while queue and len(seen)<1200:
   node=queue.pop(0)
   if node in seen:continue
   seen.add(node);r=raw_node(node)
   if r['role'] in ('label','heading','button','push button','entry','text'):rows.append(r)
   queue.extend(access.children(node))
  report['observations'].append({'label':label,'query':access.text(search),'cached':nodes,'parent_memberships':ancestors,'fresh_current_tree':rows,'zero_hidden_result':cache.zero_hidden()})
  (out/'diagnosis.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
 capture('initial six records')
 m.sales_zero_result(access,page,search,x11);capture('exact zero before explicit pump',False);capture('exact zero after explicit pump')
 time.sleep(.1);capture('exact zero after extra pump')
 for query in ['INV-M5-A','INV-M5-B']:
  m.sales_focus_search(access,search,x11);x11.key('a','Control_L');x11.type_ascii(query)
  m.observe(lambda:access.text(search)==query and m.helper.first_accessible(access,lambda row:m.normalize_digits(access.string(access.a.atspi_accessible_get_name,row['node'],None)).startswith('1 نتیجه •') and m.ready(access,row),page),query+' exact ready status')
  capture(query+' current query after fresh status')
 assert m.helper.payload(profile)==baseline
 report['durable_state_unchanged']=True
finally:
 report['source_after']=m.frozen_source_provenance();report['source_stable']=report['source_before']==report['source_after'];report['observer_sha256_after']=m.helper.sha(root/'scripts/native-performance-M5.py');report['binary_sha256_after']=m.helper.sha(binary)
 (out/'diagnosis.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
 for p in reversed(processes):m.native.stop(p)
 for f in logs:f.close()
