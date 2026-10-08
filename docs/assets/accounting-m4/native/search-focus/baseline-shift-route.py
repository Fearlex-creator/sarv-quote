import sys,os,subprocess,tempfile,time,sqlite3,json,hashlib,shutil
from pathlib import Path
sys.path.insert(0,'/workspace/sarv-quote/scripts')
import importlib.util
spec=importlib.util.spec_from_file_location('n','/workspace/sarv-quote/scripts/qa-native.py');n=importlib.util.module_from_spec(spec);spec.loader.exec_module(n)
from qa_access import Access
root=Path('/workspace/sarv-quote');out=root/'artifacts/procurement-search-focus';out.mkdir(exist_ok=True);run=Path(tempfile.mkdtemp(prefix='run-',dir=out));profile=run/'profile';profile.mkdir()
source=Path(os.environ.get('SARV_QA_BINARY',str(root/'target/debug/sarv-quote')));binary=run/'sarv-quote';shutil.copy2(source,binary)
fixture=Path(os.environ['SARV_M4_NATIVE_FIXTURE'])
with sqlite3.connect(f'file:{fixture}/sarv.sqlite3?mode=ro',uri=True)as s,sqlite3.connect(profile/'sarv.sqlite3')as d:s.backup(d)
def accounting():
 with sqlite3.connect(profile/'sarv.sqlite3')as d:return json.loads(d.execute('select payload from state where id=1').fetchone()[0])['accounting']
before=accounting();procs=[];logs=[];report={'result':'failed','binary':str(source),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'checks':[],'key_events':[],'refresh_completion_checks':[],'fixture_profile':str(fixture),'run':str(run),'scope':'Actual private XTest Tab navigation and auto-scroll in short native windows; synthetic10record5journal fixture copied read-only; no financial writes.'}
def start(cmd,name,env=None,**kw):
 log=(run/(name+'.log')).open('w');logs.append(log);p=subprocess.Popen(cmd,env=env,stdout=log,stderr=log,start_new_session=True,**kw);procs.append(p);return p
try:
 with tempfile.TemporaryFile()as fd:
  start(['Xvfb','-displayfd',str(fd.fileno()),'-screen','0','1450x1050x24','-nolisten','tcp','-xkbdir',str(n.ENVROOT/'sysroot/usr/share/X11/xkb')],'xvfb',pass_fds=(fd.fileno(),))
  def disp():fd.seek(0);return fd.read().decode().strip()
  display=':'+n.wait_for(disp)
 runtime=n.private_accessibility_environment(display,run);env=os.environ.copy();env.update(SARV_TEST_INSTANCE='M4-short-layout',SARV_DATA_DIR=str(profile),DISPLAY=display,GDK_BACKEND='x11',GSK_RENDERER='cairo',XDG_RUNTIME_DIR=str(runtime),G_DEBUG='fatal-criticals')
 for k in ['SARV_PROCUREMENT_SMOKE','SARV_CORE_SMOKE']:env.pop(k,None)
 env.update(SARV_UI_SMOKE='capture',SARV_CAPTURE_PAGE='procurement')
 start(['openbox','--sm-disable','--config-file',str(n.ENVROOT/'sysroot/etc/xdg/openbox/rc.xml')],'openbox',env)
 env['ADW_DEBUG_COLOR_SCHEME']='prefer-light';env['SARV_CAPTURE_NAME']='initial.png';p=start([str(binary)],'light',env);x=n.X11(display);w=n.wait_for(lambda:x.find(p.pid));n.wait_for(lambda:(profile/'initial.png').exists());a=Access(p.pid);x.focus(w);x.resize(w,1280,900);n.wait_for(lambda:x.geometry(w)==[1280,900]);time.sleep(.3)
 report['diagnostic_only']=True;report['route_cases']=[]
 for query in ['REC-A','REC-B']:
  x.key('f','Control_L');n.wait_for(lambda:any(r['name']=='جست‌وجوی شماره، منبع، کالا یا طرف'and a.focused(r)for r in a.tree()),label='search focus')
  x.key('a','Control_L');x.type_ascii(query)
  n.wait_for(lambda:any(query in r['name']for r in a.tree())and not any(('REC-B'if query=='REC-A'else'REC-A')in r['name']for r in a.tree()),label='correct single filtered source')
  target=a.find('جزئیات و خروجی',['button','push button']);assert target
  request=a.focus(target);focused=a.focused(target);route=[]
  for i in range(30):
   if a.focused(target):break
   before_events=len(n.events(profile));x.key('Tab');time.sleep(.01)
   event=n.wait_for(lambda:next((e for e in reversed(n.events(profile)[before_events:])if e.get('key')=='Tab'),None),label='actual Tab event')
   route.append(event)
  assert a.focused(target),'Actual Tab never reached matching detail button'
  report['route_cases'].append({'query':query,'atspi_focus_request_returned':request,'atspi_request_immediately_focused':focused,'real_Tab_count_to_result':len(route),'route':route})
  subprocess.run(['import','-display',display,'-window',str(w),str(run/('focused-'+query+'.png'))],check=True)
 x.resize(w,650,550);n.wait_for(lambda:x.geometry(w)==[650,550]);time.sleep(.3);x.key('f','Control_L');n.wait_for(lambda:any(r['name']=='جست‌وجوی شماره، منبع، کالا یا طرف'and a.focused(r)for r in a.tree()));before_events=len(n.events(profile));x.key('Tab','Shift_L');report['shift_tab_from_search_event']=n.wait_for(lambda:next((e for e in reversed(n.events(profile)[before_events:])if e.get('key')=='ISO_Left_Tab'),None));report['shift_tab_focused_semantic_controls']=[{k:v for k,v in r.items()if k!='node'}for r in a.tree()if a.focused(r)]
 x.x.XCloseDisplay(x.d);n.stop(p)
 report['accounting_exactly_preserved']=accounting()==before;assert report['accounting_exactly_preserved'];report['result']='passed'
finally:
 for p in reversed(procs):
  if p.poll()is None:n.stop(p)
 for log in logs:log.close()
 report['gtk_diagnostics']={f.name:f.read_text()for f in run.glob('*.log')if f.name in ['light.log','dark.log']}
 if any(report['gtk_diagnostics'].values()):report['result']='failed';report['error']='Native app diagnostics must be empty, retained in report'
 if (profile/'keyboard-events.jsonl').exists():shutil.copy2(profile/'keyboard-events.jsonl',run/'keyboard-events.jsonl')
 (run/'search-focus-route.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n');print(json.dumps(report,ensure_ascii=False,indent=2),flush=True)

assert report['result']=='passed',report.get('error',report)
