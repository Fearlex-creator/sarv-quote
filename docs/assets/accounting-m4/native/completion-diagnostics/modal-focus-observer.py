import os,sys,json,sqlite3,subprocess,time,tempfile,shutil,hashlib,importlib.util
from pathlib import Path
ROOT=Path('/workspace/sarv-quote');sys.path.insert(0,str(ROOT/'scripts'))
spec=importlib.util.spec_from_file_location('native',ROOT/'scripts/qa-native.py');n=importlib.util.module_from_spec(spec);spec.loader.exec_module(n)
from qa_access import Access
out=ROOT/'artifacts/procurement-completion';out.mkdir(exist_ok=True);run=Path(tempfile.mkdtemp(prefix='run-',dir=out))
binary=Path(os.environ['SARV_QA_BINARY']);fixture=Path(os.environ['SARV_M4_NATIVE_FIXTURE']);seed_fixture=Path(os.environ['SARV_M3_NATIVE_FIXTURE']);shim=Path('/tmp/sarv-M4-native-worker-delay.so');flag=run/'hold-worker';trace=run/'worker-delay.log';procs=[];logs=[]
report={'result':'failed','binary':str(binary),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'run':str(run),'checks':[],'synthetic_delay':{'scope':'Source cancellation branch only. Actual pthread_setname_np forwards unchanged; holds owned sarv-export worker behind private flag, maximum5000ms. No GTK/main thread delay and no financial writes. Uninstrumented ready and rejected preview observations are separate.','c_sha256':hashlib.sha256(Path(__file__).with_name('worker-delay.c').read_bytes()).hexdigest(),'so_sha256':hashlib.sha256(shim.read_bytes()).hexdigest()},'focus_events':[]}
def start(cmd,name,env=None,**kw):
 f=(run/(name+'.log')).open('w');logs.append(f);p=subprocess.Popen(cmd,env=env,stdout=f,stderr=f,start_new_session=True,**kw);procs.append(p);return p
def copy_profile(source,target):
 target.mkdir()
 with sqlite3.connect(f'file:{source}/sarv.sqlite3?mode=ro',uri=True)as s,sqlite3.connect(target/'sarv.sqlite3')as d:s.backup(d)
def state(profile):
 with sqlite3.connect(f'file:{profile}/sarv.sqlite3?mode=ro',uri=True)as d:return d.execute('select payload,generation from state where id=1').fetchone()
def check(name,ok,**details):
 report['checks'].append({'name':name,'passed':bool(ok),**details});assert ok,name
try:
 with tempfile.TemporaryFile()as fd:
  start(['Xvfb','-displayfd',str(fd.fileno()),'-screen','0','1450x1050x24','-nolisten','tcp','-xkbdir',str(n.ENVROOT/'sysroot/usr/share/X11/xkb')],'xvfb',pass_fds=(fd.fileno(),))
  def disp():fd.seek(0);return fd.read().decode().strip()
  display=':'+n.wait_for(disp)
 runtime=n.private_accessibility_environment(display,run);env=os.environ.copy();env.update(DISPLAY=display,GDK_BACKEND='x11',GSK_RENDERER='cairo',XDG_RUNTIME_DIR=str(runtime),G_DEBUG='fatal-criticals',ADW_DEBUG_COLOR_SCHEME='prefer-light',SARV_TEST_INSTANCE='M4-completion-native')
 for k in ['SARV_PROCUREMENT_SMOKE','SARV_UI_SMOKE','SARV_CORE_SMOKE','LD_PRELOAD']:env.pop(k,None)
 start(['openbox','--sm-disable','--config-file',str(n.ENVROOT/'sysroot/etc/xdg/openbox/rc.xml')],'openbox',env)
 x=n.X11(display)
 def attach(p,profile):
  global a,w
  w=n.wait_for(lambda:x.find(p.pid));a=Access(p.pid);x.focus(w);x.resize(w,1280,900);time.sleep(.3)
 def focus(name,roles=None):
  for _ in range(160):
   r=a.find(name,roles)
   if r and a.focused(r):return r
   x.key('Tab');time.sleep(.015)
  raise Exception('Actual Tab did not focus '+name)
 def press(name):focus(name,['button','push button']);x.key('Return');time.sleep(.05)
 def fill(name,text):focus(name,['text','entry']);x.key('a','Control_L');x.type_ascii(text)
 def label(name):return any(r['name']==name for r in a.tree())
 def capture(name):subprocess.run(['import','-display',display,'-window',str(w),str(run/(name+'.png'))],check=True)
 def query(text):
  x.key('f','Control_L');n.wait_for(lambda:any(r['name']=='جست‌وجوی شماره، منبع، کالا یا طرف'and a.focused(r)for r in a.tree()),label='current document search')
  x.key('a','Control_L');x.type_ascii(text);n.wait_for(lambda:any(' • 1 نتیجه • 'in r['name']for r in a.tree()),label='settled single source');time.sleep(.08);x.key('Tab');n.wait_for(lambda:a.find('جزئیات و خروجی',['button','push button'])and a.focused(a.find('جزئیات و خروجی',['button','push button'])),label='one actual Tab to detail')
 # Separate read-only actual modal focus probe. Native main-window capture may not receive modal keys.
 profile=run/'ready';copy_profile(fixture,profile);before=state(profile);env.update(SARV_DATA_DIR=str(profile),SARV_UI_SMOKE='capture',SARV_CAPTURE_PAGE='procurement',SARV_CAPTURE_NAME='initial.png')
 p=start([str(binary)],'ready-native',env);attach(p,profile);n.wait_for(lambda:(profile/'initial.png').exists());query('REC-A');x.key('Return')
 n.wait_for(lambda:label('منبع ثبت‌شده آماده است • فقط خواندنی'),label='readonly source loaded')
 target='جزئیات کامل منبع، شناسه‌ها و اثر ثبت‌شده';r=focus(target,['button','push button']);time.sleep(.25)
 focused=[{'name':v['name'],'role':v['role']}for v in a.tree()if a.focused(v)]
 check('actual Tab focuses the modal disclosure control',any(v['name']==target and v['role']in ['button','push button']for v in focused),actual_focused_semantic_controls=focused)
 prior=len(n.events(profile));x.key('Shift_L');time.sleep(.25);events=n.events(profile)[prior:]
 report['focus_events']=events;report['modal_numeric_geometry_measured']=False
 report['native_mainwindow_controller_receives_modal_modifier']=bool(events)
 report['method']='Actual XTest Tab until the disclosure itself is AT-SPI focused, then harmless Shift while focus remains. Native window events retained if delivered; absent events and null bounds cannot establish modal numeric geometry.'
 capture('source-actual-modal-disclosure-focus');x.key('space');time.sleep(.2);x.key('Escape');n.wait_for(lambda:not a.find(target,['button','push button']),label='actual Escape closes modal source');time.sleep(.3)
 check('actual modal focus/read-only expansion/cancel preserves full payload and generation',state(profile)==before)
 n.stop(p);x.x.XCloseDisplay(x.d);report['result']='passed'
finally:
 if flag.exists():flag.unlink()
 for p in reversed(procs):
  if p.poll()is None:n.stop(p)
 for f in logs:f.close()
 report['gtk_diagnostics']={f.name:f.read_text()for f in run.glob('*-native.log')}
 if any(report['gtk_diagnostics'].values()):report['result']='failed';report['error']='Actual native app diagnostics must all be empty'
 if trace.exists():report['synthetic_delay']['trace']=trace.read_text()
 for profile in ['ready','cancel','preview']:
  events=run/profile/'keyboard-events.jsonl'
  if events.exists():shutil.copy2(events,run/(profile+'-keyboard-events.jsonl'))
 (run/'modal-focus-observer.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n');print(json.dumps(report,ensure_ascii=False,indent=2),flush=True)
assert report['result']=='passed',report.get('error',report)
