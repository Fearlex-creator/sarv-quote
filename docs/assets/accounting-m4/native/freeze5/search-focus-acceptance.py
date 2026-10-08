import sys,os,subprocess,tempfile,time,sqlite3,json,hashlib,shutil
from pathlib import Path
sys.path.insert(0,'/workspace/sarv-quote/scripts')
import importlib.util
spec=importlib.util.spec_from_file_location('n','/workspace/sarv-quote/scripts/qa-native.py');n=importlib.util.module_from_spec(spec);spec.loader.exec_module(n)
from qa_access import Access
# JSONL is appended by the application. Retry the full snapshot if a write is in flight; never discard a row.
_original_events=n.events
def stable_events(profile):
 def snapshot():
  try:return (_original_events(profile),)
  except json.JSONDecodeError:return None
 return n.wait_for(snapshot,seconds=2,label='complete native key-event JSONL snapshot')[0]
n.events=stable_events
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
 report['scope']='Read-only native keyboard acceptance for filtered document lookup, default/backward navigation, exact source opening and toolbar reachability; synthetic data only; no performance-budget claim.';report['diagnostic_only']=True;report['route_cases']=[]
 def event(key,*mods):
  before=len(n.events(profile));x.key(key,*mods)
  return n.wait_for(lambda:next((e for e in reversed(n.events(profile)[before:])if e.get('key')in([key,'ISO_Left_Tab']if key=='Tab'else[key])),None),label='actual '+key+' event')
 def search_focus():
  x.key('f','Control_L');n.wait_for(lambda:any(r['name']=='جست‌وجوی شماره، منبع، کالا یا طرف'and a.focused(r)for r in a.tree()),label='current search focus')
 def query(text,count):
  search_focus();x.key('a','Control_L')
  if text:x.type_ascii(text)
  else:x.key('BackSpace')
  n.wait_for(lambda:any((' • '+str(count)+' نتیجه • ')in r['name']for r in a.tree()),label='settled exact document result count')
  time.sleep(.07)
 def passed(name,condition,**details):
  report['checks'].append({'name':name,'passed':bool(condition),**details});assert condition,name
 # Settled filtered queries must lead to the first usable source in one Tab.
 ids={r['command']['Receive']['reference']:r['id']for r in before['procurement']['records']if 'Receive'in r['command']}
 for width,height in [(1280,900),(650,550)]:
  x.resize(w,width,height);n.wait_for(lambda:x.geometry(w)==[width,height]);time.sleep(.3)
  for text in ['REC-A','REC-B']:
   query(text,1);e=event('Tab');passed(text+' one Tab '+str(width),e['page']=='procurement'and e['focus_name']=='procurement-document-detail',native_event=e)
   time.sleep(.25);settled=event('Shift_L');rect=settled['focus_bounds'];clip=settled['focus_clip_bounds'];passed(text+' revealed focus '+str(width),rect and clip and rect[0]>=clip[0]-.01 and rect[1]>=clip[1]-.01 and rect[0]+rect[2]<=clip[0]+clip[2]+.01 and rect[1]+rect[3]<=clip[1]+clip[3]+.01,native_event=settled)
   subprocess.run(['import','-display',display,'-window',str(w),str(run/('focused-'+text+'-'+str(width)+'.png'))],check=True)
 # Multi-match keeps original latest-first order and opens the exact first source.
 query('REC',2);e=event('Tab');passed('multi-match one Tab',e['focus_name']=='procurement-document-detail',native_event=e);event('Return')
 n.wait_for(lambda:a.find('جزئیات کامل منبع، شناسه‌ها و اثر ثبت‌شده',['button','push button']),label='actual readonly source loaded')
 for _ in range(60):
  d=a.find('جزئیات کامل منبع، شناسه‌ها و اثر ثبت‌شده',['button','push button'])
  if d and a.focused(d):break
  x.key('Tab');time.sleep(.03)
 assert d and a.focused(d);x.key('space')
 n.wait_for(lambda:any(ids['REC-B']in r['name']for r in a.tree()),label='exact latest REC-B canonical source')
 passed('Enter opens exact first source REC-B',True,canonical_source_id=ids['REC-B']);x.key('Escape');n.wait_for(lambda:not a.find('جزئیات کامل منبع، شناسه‌ها و اثر ثبت‌شده',['button','push button']),label='readonly dialog closed');time.sleep(.3)
 # The toolbar remains reachable through standard backward navigation.
 query('REC-A',1);event('Tab');backward=[]
 for _ in range(12):
  e=event('Tab','Shift_L');backward.append(e)
  if e.get('focus_name')=='GtkText':break
  if any(r['name']=='جست‌وجوی شماره، منبع، کالا یا طرف'and a.focused(r)for r in a.tree()):break
 needed={'procurement-new-order','procurement-new-bill','procurement-tab-documents','procurement-tab-controls','procurement-tab-masters'}
 passed('all toolbar controls reachable backwards',needed.issubset({e['focus_name']for e in backward}),route=backward)
 search_focus();e=event('Tab','Shift_L');semantic=[{k:v for k,v in r.items()if k!='node'}for r in a.tree()if a.focused(r)];passed('ShiftTab from search preserves previous display-unit route',e['focus_type']=='GtkToggleButton' and any(r['name']=='نمایش ریال' and r['role']=='toggle button'for r in semantic),native_event=e,focused_semantic_controls=semantic)
 for text,count,label in [('',10,'empty query default route'),('NO-MATCH',0,'no-result default route')]:
  query(text,count);e=event('Tab');passed(label,e['focus_name']=='procurement-tab-documents',native_event=e)
 query('REC-A',1);e=event('Tab','Control_L');passed('modified Tab does not jump to result',e['focus_name']!='procurement-document-detail',native_event=e)
 query('REC-A',1);search_focus();x.key('a','Control_L');x.type_ascii('NEW-STILL-DEBOUNCING-NO-MATCH');e=event('Tab');passed('stale query does not focus old result',e['focus_name']!='procurement-document-detail',native_event=e)
 # A master search keeps normal tab navigation, not the document shortcut.
 x.key('f','Control_L');tab=a.find('تعریف منابع',['page tab']);assert tab and a.click(tab);search_focus();x.key('a','Control_L');x.type_ascii('CU-A');n.wait_for(lambda:any(r['name'].startswith('1 منبع •')for r in a.tree()),label='settled master query');time.sleep(.07);e=event('Tab');passed('masters query keeps default route',e['focus_name']!='procurement-document-detail',native_event=e)
 x.x.XCloseDisplay(x.d);n.stop(p)
 report['accounting_exactly_preserved']=accounting()==before;assert report['accounting_exactly_preserved'];report['result']='passed'
finally:
 for p in reversed(procs):
  if p.poll()is None:n.stop(p)
 for log in logs:log.close()
 report['gtk_diagnostics']={f.name:f.read_text()for f in run.glob('*.log')if f.name in ['light.log','dark.log']}
 if any(report['gtk_diagnostics'].values()):report['result']='failed';report['error']='Native app diagnostics must be empty, retained in report'
 if (profile/'keyboard-events.jsonl').exists():shutil.copy2(profile/'keyboard-events.jsonl',run/'keyboard-events.jsonl')
 (run/'search-focus-acceptance.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n');print(json.dumps(report,ensure_ascii=False,indent=2),flush=True)

assert report['result']=='passed',report.get('error',report)
