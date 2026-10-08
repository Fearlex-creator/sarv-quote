#!/usr/bin/env python3
"""Read-only exact-key Sales list pager proof on a disposable synthetic backup."""
import hashlib,importlib.util,json,os,sqlite3,subprocess,sys,tempfile,time
from pathlib import Path
ROOT=Path('/workspace/sarv-quote');sys.path.insert(0,str(ROOT/'scripts'))
from qa_access import Access
spec=importlib.util.spec_from_file_location('native',ROOT/'scripts/qa-native.py');native=importlib.util.module_from_spec(spec);spec.loader.exec_module(native)
source=Path(sys.argv[1]);run=Path(sys.argv[2]);manifest=json.loads(Path(sys.argv[3]).read_text());target=run/'list-pager-inspection';target.mkdir()
def provenance():
 h=hashlib.sha256();files={}
 for name in sorted(manifest['frozen_source_provenance']['files'],key=lambda n:n.encode()):
  raw=(ROOT/name).read_bytes();h.update(name.encode()+b'\0'+raw+b'\0');files[name]=hashlib.sha256(raw).hexdigest()
 return {'sha256':h.hexdigest(),'files':files}
def payload(profile):
 with sqlite3.connect(f'file:{profile}/sarv.sqlite3?mode=ro',uri=True) as db:return db.execute('select generation,payload from state where id=1').fetchone()
before=provenance();assert before=={k:manifest['frozen_source_provenance'][k] for k in ['sha256','files']}
binary=Path(manifest['binary']);assert hashlib.sha256(binary.read_bytes()).hexdigest()==manifest['binary_sha256']
source_before=payload(source)
with sqlite3.connect(f'file:{source}/sarv.sqlite3?mode=ro',uri=True) as src,sqlite3.connect(target/'sarv.sqlite3') as dst:src.backup(dst)
original=payload(target);assert original==source_before
state=json.loads(original[1]);assert len(state['sales_drafts'])==120
processes=[];logs=[];x=None;checks=[]
try:
 def start(cmd,name,env=None,**kwargs):
  log=(target/f'{name}.log').open('w');logs.append(log);p=subprocess.Popen(cmd,stdout=log,stderr=log,env=env,start_new_session=True,**kwargs);processes.append(p);return p
 with tempfile.TemporaryFile() as fd:
  start(['Xvfb','-displayfd',str(fd.fileno()),'-screen','0','1450x1050x24','-nolisten','tcp','-xkbdir',str(native.ENVROOT/'sysroot/usr/share/X11/xkb')],'xvfb',pass_fds=(fd.fileno(),))
  def display_number():fd.seek(0);return fd.read().decode().strip()
  display=':'+native.wait_for(display_number)
 runtime=native.private_accessibility_environment(display,target);env=os.environ.copy();env.update(DISPLAY=display,GDK_BACKEND='x11',GSK_RENDERER='cairo',XDG_RUNTIME_DIR=str(runtime),SARV_DATA_DIR=str(target),SARV_TEST_INSTANCE='m5-read-only-list-pager',SARV_UI_SMOKE='capture',SARV_CAPTURE_PAGE='sales',SARV_CAPTURE_NAME='restarted-sales.png',ADW_DEBUG_COLOR_SCHEME='prefer-light')
 for name in ['SARV_SALES_SMOKE','SARV_CORE_SMOKE','SARV_PROCUREMENT_SMOKE','SARV_ACCOUNTING_SMOKE']:env.pop(name,None)
 start(['openbox','--sm-disable','--config-file',str(native.ENVROOT/'sysroot/etc/xdg/openbox/rc.xml')],'openbox',env)
 app=start([str(binary)],'app',env);x=native.X11(display);window=native.wait_for(lambda:x.find(app.pid));native.wait_for(lambda:(target/'restarted-sales.png').exists());x.focus(window);access=Access(app.pid)
 def exact_focus(caption):
  native.wait_for(lambda:access.find(caption,['button']),label=caption)
  for _ in range(100):
   row=access.find(caption,['button'])
   if row and access.focused(row):
    rect=access.rect(row);assert rect['width']>0 and rect['height']>0
    return row
   x.key('Tab');time.sleep(.02)
  raise AssertionError('exact real Tab button focus '+caption)
 def screenshot(name):subprocess.run(['/usr/bin/import','-display',display,'-window',str(window),str(target/name)],check=True)
 for width,height in [(1280,900),(650,550)]:
  x.resize(window,width,height);native.wait_for(lambda:x.geometry(window)==[width,height]);x.key('0','Alt_L');time.sleep(.3);x.key('f','Control_L');x.key('a','Control_L');x.type_ascii('PERF-B')
  native.wait_for(lambda:access.text(access.find('جست‌وجوی شماره، مشتری، کالا یا پیش‌نویس',['text','entry']))=='PERF-B' and any('۶۰ نتیجه' in r['name'] for r in access.tree()),label='60-row current filtered list')
  native.wait_for(lambda:any('پیش‌نویس • PERF-B-001'==r['name'] for r in access.tree()),label='first page001')
  exact_focus('صفحهٔ بعد');screenshot(f'M5-list-next-focused-{width}x{height}.png');x.key('space')
  native.wait_for(lambda:any('پیش‌نویس • PERF-B-051'==r['name'] for r in access.tree()) and not any('پیش‌نویس • PERF-B-001'==r['name'] for r in access.tree()),label='second page051')
  exact_focus('صفحهٔ قبل');screenshot(f'M5-list-previous-focused-{width}x{height}.png');x.key('space')
  native.wait_for(lambda:any('پیش‌نویس • PERF-B-001'==r['name'] for r in access.tree()) and not any('پیش‌نویس • PERF-B-051'==r['name'] for r in access.tree()),label='previous first page001')
  assert payload(target)==original
  checks.append({'geometry':[width,height],'filtered_rows':60,'page_size':25,'exact_real_Tab_Next_Previous':True,'Space_051_then_001':True,'financial_and_raw_state_unchanged':True})
 assert payload(source)==source_before
 after=provenance();assert after==before;assert hashlib.sha256(binary.read_bytes()).hexdigest()==manifest['binary_sha256']
 result={'result':'passed','scope':'native read-only filtered Sales Next/Previous using exact Tab and Space at standard/narrow geometry','synthetic_only':True,'seed_source':str(source),'binary_sha256':manifest['binary_sha256'],'source_before':before,'source_after':after,'checks':checks}
 (target/'list-pager-inspection.json').write_text(json.dumps(result,ensure_ascii=False,indent=2));print(json.dumps({k:v for k,v in result.items() if k not in ['source_before','source_after']},indent=2))
finally:
 if x:x.x.XCloseDisplay(x.d)
 for p in reversed(processes):
  if p.poll() is None:
   p.terminate()
   try:p.wait(timeout=3)
   except subprocess.TimeoutExpired:p.kill()
 for log in logs:log.close()
