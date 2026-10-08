import os,sys,json,sqlite3,subprocess,time,tempfile,shutil,hashlib,importlib.util,ctypes as C
from pathlib import Path
ROOT=Path('/workspace/sarv-quote');sys.path.insert(0,str(ROOT/'scripts'))
spec=importlib.util.spec_from_file_location('native',ROOT/'scripts/qa-native.py');n=importlib.util.module_from_spec(spec);spec.loader.exec_module(n)
from qa_access import Access
out=ROOT/'artifacts/picker-async';out.mkdir(exist_ok=True);run=Path(tempfile.mkdtemp(prefix='run-',dir=out));profile=run/'profile';profile.mkdir();binary=Path(os.environ['SARV_QA_BINARY']);fixture=Path(os.environ['SARV_QUOTE_FIXTURE']);logs=[];procs=[];lock=None
with sqlite3.connect(f'file:{fixture}/sarv.sqlite3?mode=ro',uri=True)as s,sqlite3.connect(profile/'sarv.sqlite3')as d:s.backup(d)
with sqlite3.connect(profile/'sarv.sqlite3')as d:
 generation,payload=d.execute('select generation,payload from state').fetchone();state=json.loads(payload);state['draft']['lines']=[{'query':'QA-PICKER-TARGET','quantity_milli':1000,'chosen_code':None}];d.execute('update state set generation=?,payload=? where id=1',(generation+1,json.dumps(state,ensure_ascii=False,separators=(',',':'))))
report={'result':'failed','binary':str(binary),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'fixture':str(fixture),'run':str(run),'scope':'Synthetic one-line quote and50k catalog; real XTest picker activation/selection and owned SQLite writer lock. Not a performance benchmark.','checks':[]}
def start(cmd,name,env=None,**kw):
 f=(run/(name+'.log')).open('w');logs.append(f);p=subprocess.Popen(cmd,env=env,stdout=f,stderr=f,start_new_session=True,**kw);procs.append(p);return p
def check(name,ok,**details):report['checks'].append({'name':name,'passed':bool(ok),**details});assert ok,name
def raw():
 with sqlite3.connect(f'file:{profile}/sarv.sqlite3?mode=ro',uri=True)as d:return d.execute('select generation,payload from state where id=1').fetchone()
def state():return json.loads(raw()[1])
try:
 with tempfile.TemporaryFile()as fd:
  start(['Xvfb','-displayfd',str(fd.fileno()),'-screen','0','1450x1050x24','-nolisten','tcp','-xkbdir',str(n.ENVROOT/'sysroot/usr/share/X11/xkb')],'xvfb',pass_fds=(fd.fileno(),))
  def displayread():fd.seek(0);return fd.read().decode().strip()
  display=':'+n.wait_for(displayread)
 runtime=n.private_accessibility_environment(display,run);env=os.environ.copy();env.update(DISPLAY=display,GDK_BACKEND='x11',GSK_RENDERER='cairo',XDG_RUNTIME_DIR=str(runtime),SARV_TEST_INSTANCE='M4-picker-async-debug',SARV_DATA_DIR=str(profile),SARV_UI_SMOKE='capture',SARV_CAPTURE_PAGE='order',SARV_CAPTURE_NAME='initial.png',G_DEBUG='fatal-criticals')
 for key in ['SARV_PROCUREMENT_SMOKE','SARV_CORE_SMOKE','LD_PRELOAD']:env.pop(key,None)
 start(['openbox','--sm-disable','--config-file',str(n.ENVROOT/'sysroot/etc/xdg/openbox/rc.xml')],'openbox',env);p=start([str(binary)],'app-native',env);x=n.X11(display)
 class XError(C.Structure):
  _fields_=[('type',C.c_int),('display',C.c_void_p),('resource',C.c_ulong),('serial',C.c_ulong),('code',C.c_ubyte),('request',C.c_ubyte),('minor',C.c_ubyte)]
 @C.CFUNCTYPE(C.c_int,C.c_void_p,C.POINTER(XError))
 def xerror(_,error):
  if error.contents.code!=3:raise AssertionError('Unexpected X error '+str(error.contents.code))
  return 0
 x.x.XSetErrorHandler.argtypes=[C.c_void_p];x.x.XSetErrorHandler(C.cast(xerror,C.c_void_p));w=n.wait_for(lambda:x.find(p.pid));n.wait_for(lambda:(profile/'initial.png').exists());a=Access(p.pid);x.focus(w);x.resize(w,1400,940);time.sleep(.3)
 def focus(name,roles=None):
  (run/'accessible-last.json').write_text(json.dumps([{k:v for k,v in row.items()if k!='node'}for row in a.tree()],ensure_ascii=False,indent=2))
  initial=a.find(name,roles)
  if initial:a.focus(initial)
  for _ in range(160):
   if name=='جست‌وجو و انتخاب انسانی کالای ردیف ۱':
    try:ev=n.events(profile)
    except json.JSONDecodeError:ev=[]
    if ev and ev[-1].get('focus_name')=='resolve-line-0':return {'native_focus_event':ev[-1]}
   r=a.find(name,roles)
   if r and a.focused(r):return r
   x.key('Tab');time.sleep(.015)
  raise Exception('Actual Tab focus failed: '+name)
 def press(name):focus(name,['button','push button']);x.key('Return');time.sleep(.05)
 def picker():return a.find('جست‌وجوی نام، کد یا نام مستعار کالا',['text','entry'])
 def waitpicker():return n.wait_for(lambda:picker()and a.focused(picker()),label='native indexed picker search focused')
 def choose():
  waitpicker();x.type_ascii('SCALE-00001');n.wait_for(lambda:any('SCALE-00001'in r['name']for r in a.tree()),label='actual indexed product result');time.sleep(.15);x.key('Return')
 def screenshot(name):subprocess.run(['import','-display',display,'-window',str(w),str(run/(name+'.png'))],check=True)
 before=raw();beforestate=state();press('جست‌وجو و انتخاب انسانی کالای ردیف ۱');waitpicker();check('async opening preserves saved state',raw()==before)
 lock=sqlite3.connect(profile/'sarv.sqlite3');lock.execute('BEGIN IMMEDIATE');choose()
 n.wait_for(lambda:any(r['name']=='در حال ذخیرهٔ انتخاب کالا و پیش‌نویس…'for r in a.tree()),label='accepted choice enters async transaction')
 x.key('Escape');time.sleep(.2);check('pending accepted selection refuses Escape and stays responsive',bool(picker()));check('no intermediate write under owned SQLite writer lock',raw()==before);screenshot('pending-choice-locked')
 lock.rollback();lock.close();lock=None
 n.wait_for(lambda:state()['draft']['lines'][0]['chosen_code']=='SCALE-00001',seconds=12,label='one candidate durable explicit choice');n.wait_for(lambda:not picker(),label='choice dialog closes after receipt');time.sleep(.3)
 after=raw();afterstate=state();check('single CAS generation for combined form and explicit choice',after[0]==before[0]+1,before_generation=before[0],after_generation=after[0]);check('original query and exact quantity preserved',afterstate['draft']['lines'][0]=={'query':'QA-PICKER-TARGET','quantity_milli':1000,'chosen_code':'SCALE-00001'});check('financial and approved history unchanged',afterstate.get('accounting')==beforestate.get('accounting')and afterstate['quotes']==beforestate['quotes']);screenshot('choice-complete')
 n.stop(p);env['SARV_CAPTURE_NAME']='restart.png';p=start([str(binary)],'restart-native',env);w=n.wait_for(lambda:x.find(p.pid));n.wait_for(lambda:(profile/'restart.png').exists());check('restart retains complete chosen source and financial history',raw()==after);n.stop(p);x.x.XCloseDisplay(x.d);report['result']='passed'
finally:
 if lock is not None:lock.rollback();lock.close()
 for p in reversed(procs):
  if p.poll()is None:n.stop(p)
 for f in logs:f.close()
 report['gtk_diagnostics']={f.name:f.read_text()for f in run.glob('*-native.log')}
 if any(report['gtk_diagnostics'].values()):report['result']='failed';report['error']='actual app diagnostics must be empty'
 if(profile/'keyboard-events.jsonl').exists():shutil.copy2(profile/'keyboard-events.jsonl',run/'keyboard-events.jsonl')
 (run/'picker-async.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n');print(json.dumps(report,ensure_ascii=False,indent=2),flush=True)
assert report['result']=='passed',report.get('error',report)
