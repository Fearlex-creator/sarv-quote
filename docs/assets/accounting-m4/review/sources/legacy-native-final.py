#!/usr/bin/env python3
"""Rerun only the incorrectly selected legacy native final phase on a fresh profile."""
import hashlib,importlib.util,json,os,subprocess,tempfile,time
from pathlib import Path
spec=importlib.util.spec_from_file_location('review','/workspace/sarv-quote/artifacts/independent-m4/native-review.py');review=importlib.util.module_from_spec(spec);spec.loader.exec_module(review)
native=review.native;Access=review.Access;ROOT=review.ROOT
binary=Path(os.environ['SARV_QA_BINARY']);expected_sha=os.environ['SARV_QA_EXPECT_SHA'];expected_source='895386c1b3e66d58460c3e704487e8450833cb7ea581cc370c0fa9febf1ec8f2'
def source_hash():
 h=hashlib.sha256()
 for p in sorted([*ROOT.glob('src/**/*.rs'),ROOT/'src/style.css',ROOT/'Cargo.toml',ROOT/'Cargo.lock'],key=lambda p:str(p.relative_to(ROOT))):h.update(str(p.relative_to(ROOT)).encode()+b'\0'+p.read_bytes()+b'\0')
 return h.hexdigest()
assert hashlib.sha256(binary.read_bytes()).hexdigest()==expected_sha and source_hash()==expected_source
run=Path(tempfile.mkdtemp(prefix='legacy-corrected-',dir=ROOT/'artifacts/independent-m4/native'));processes=[];logs=[];x11=None
report={'result':'failed','binary_sha256':expected_sha,'production_source_sha256':expected_source,'run':str(run),'scope':'Fresh native chooser/restored exact archived-M3 manual-prefix history; corrected labelled-by reason selector; actual XTest Enter, typed reason, Tab, Escape; no production change','checks':[]}
def start(cmd,name,env=None,**kw):
 log=(run/(name+'.log')).open('w');logs.append(log);p=subprocess.Popen(cmd,env=env,stdout=log,stderr=log,start_new_session=True,**kw);processes.append(p);return p
def check(name,value,**detail):report['checks'].append({'name':name,'passed':bool(value),**detail});print('CHECK',name,bool(value),flush=True);assert value,name
try:
 with tempfile.TemporaryFile() as fd:
  start(['Xvfb','-displayfd',str(fd.fileno()),'-screen','0','1450x1050x24','-nolisten','tcp','-xkbdir',str(native.ENVROOT/'sysroot/usr/share/X11/xkb')],'xvfb',pass_fds=(fd.fileno(),))
  def get_display():fd.seek(0);return fd.read().decode().strip()
  display=':'+native.wait_for(get_display,label='private display')
 runtime=native.private_accessibility_environment(display,run);env=os.environ.copy();env.update(DISPLAY=display,GDK_BACKEND='x11',GSK_RENDERER='cairo',XDG_RUNTIME_DIR=str(runtime),SARV_TEST_INSTANCE='independent-legacy-corrected',ADW_DEBUG_COLOR_SCHEME='prefer-dark',G_DEBUG='fatal-criticals')
 for k in ['SARV_UI_SMOKE','SARV_ACCOUNTING_SMOKE','SARV_CORE_SMOKE','SARV_PROCUREMENT_SMOKE']:env.pop(k,None)
 start(['openbox','--sm-disable','--config-file',str(native.ENVROOT/'sysroot/etc/xdg/openbox/rc.xml')],'openbox',env)
 profile=run/'fresh';profile.mkdir();env.update(SARV_DATA_DIR=str(profile),SARV_UI_SMOKE='capture',SARV_CAPTURE_PAGE='backup',SARV_CAPTURE_NAME='fresh.png');app=start([str(binary)],'app',env);x11=native.X11(display);window=native.wait_for(lambda:x11.find(app.pid));native.wait_for(lambda:(profile/'fresh.png').exists());time.sleep(.3);access=Access(app.pid);x11.focus(window)
 def find(name,roles=None):return access.find(name,roles)
 def focus(name,roles=None):
  x11.focus(window);row=native.wait_for(lambda:find(name,roles),label=name)
  if access.focus(row):
   native.wait_for(lambda:access.focused(row),label='focus '+name);return row
  for _ in range(200):
   row=find(name,roles)
   if row and access.focused(row):return row
   x11.key('Tab');time.sleep(.015)
  raise AssertionError('actual Tab focus '+name)
 def press(name):focus(name,['button','push button']);x11.key('Return');time.sleep(.3)
 def shot(name):subprocess.run(['import','-display',display,'-window',str(window),str(run/name)],check=True)
 before=review.state(profile);backup=ROOT/'artifacts/independent-m4/final-freeze2/legacy-m3/old-m3-manual-prefix-v4.json';original=json.loads(backup.read_text());saved=original['state'];saved['schema']=5;report['original_backup']=str(backup);report['original_backup_sha256']=hashlib.sha256(backup.read_bytes()).hexdigest()
 x11.key('5','Alt_L');time.sleep(.3);press('انتخاب پشتیبان و بررسی بازیابی');native.wait_for(lambda:any(r['role']=='dialog' and r['name']=='انتخاب فایل' for r in access.tree()),label='native chooser');x11.key('l','Control_L');time.sleep(.2);x11.type_ascii(str(backup));time.sleep(.3);opener=native.wait_for(lambda:find('Open',['button','push button']),label='chooser Open');assert access.click(opener);native.wait_for(lambda:find('بازیابی و ذخیرهٔ نسخهٔ ایمنی',['button','push button']),label='verified restore preview');check('fresh native chooser verifies archivedM3 manual prefix backup without changing state',review.state(profile)==before);shot('legacy-preview.png');press('بازیابی و ذخیرهٔ نسخهٔ ایمنی');native.wait_for(lambda:review.state(profile)==saved,seconds=30,label='exact restored state');check('fresh actual native restore preserves exact old history except schema',review.state(profile)==saved)
 x11.key('7','Alt_L');time.sleep(.4);focus('اسناد',['page tab']);x11.key('space');time.sleep(.3);x11.key('f','Control_L');search=focus('جست‌وجوی اسناد حسابداری',['entry','text']);x11.key('a','Control_L');x11.type_ascii(saved['accounting']['journals'][0]['id']);native.wait_for(lambda:find('سند معکوس با دلیل',['button','push button']),label='eligible ordinary reversal')
 check('old arbitrary prefix exposes ordinary reversal and no procurement source',not find('منبع خرید و اثر موجودی',['button','push button']) and review.state(profile)==saved);press('سند معکوس با دلیل');reason=native.wait_for(lambda:find('دلیل ضروری',['entry','text']),label='actual labelled-by reason');check('real Enter opens ordinary reversal dialog with labelled-by reason',reason is not None and any(r['role']=='dialog' and r['name']=='ثبت سند معکوس' for r in access.tree()));focus('دلیل ضروری',['entry','text']);x11.type_ascii('independent cancellation focus proof');native.wait_for(lambda:access.text(find('دلیل ضروری',['entry','text']))=='independent cancellation focus proof',label='typed reason handled by GTK');check('typed reason retained without posted history mutation',access.text(reason)=='independent cancellation focus proof' and review.state(profile)==saved);shot('legacy-reversal-typed.png')
 reached=False
 for n in range(12):
  x11.key('Tab');time.sleep(.025);cancel=find('انصراف',['button','push button'])
  if cancel and access.focused(cancel):reached=True;break
 check('actual Tab reaches Cancel from reason field',reached,tabs=n+1);x11.key('Escape');native.wait_for(lambda:not any(r['role']=='dialog' and r['name']=='ثبت سند معکوس' for r in access.tree()),label='Escape closes reversal');check('real Escape cancellation preserves exact complete restored history',review.state(profile)==saved);shot('legacy-cancelled.png')
 native.stop(app);bad=[line for p in run.glob('*.log') if p.name=='app.log' for line in p.read_text(errors='replace').splitlines() if 'WARNING' in line or 'CRITICAL' in line];check('actual GTK app log contains no warnings or criticals',not bad,diagnostics=bad);check('production source and pinned executable unchanged',source_hash()==expected_source and hashlib.sha256(binary.read_bytes()).hexdigest()==expected_sha);report['result']='passed'
except Exception as e:
 report['error']=str(e)
 if 'access' in locals():(run/'failure-tree.json').write_text(json.dumps([{k:v for k,v in r.items() if k!='node'} for r in access.tree()],ensure_ascii=False,indent=2))
 raise
finally:
 if x11:x11.x.XCloseDisplay(x11.d)
 for p in reversed(processes):native.stop(p)
 for f in logs:f.close()
 (run/'legacy-corrected.json').write_text(json.dumps(report,ensure_ascii=False,indent=2));print(json.dumps(report,ensure_ascii=False,indent=2),flush=True)
