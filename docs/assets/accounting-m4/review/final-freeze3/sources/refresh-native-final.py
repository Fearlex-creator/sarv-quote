#!/usr/bin/env python3
"""Freeze3 targeted read-only refresh footer success/error/recovery observer."""
import hashlib,importlib.util,json,os,subprocess,tempfile,time,unicodedata
from pathlib import Path
spec=importlib.util.spec_from_file_location('review','/workspace/sarv-quote/artifacts/independent-m4/native-review.py');review=importlib.util.module_from_spec(spec);spec.loader.exec_module(review)
native=review.native;Access=review.Access;ROOT=review.ROOT
binary=Path(os.environ['SARV_QA_BINARY']);expected_sha=os.environ['SARV_QA_EXPECT_SHA'];expected_source='ec50e593693bff141271b490cf2cce74005c3b3117324d7285fa9e6f631b56b9'
def source_hash():
 h=hashlib.sha256()
 for p in sorted([*ROOT.glob('src/**/*.rs'),ROOT/'src/style.css',ROOT/'Cargo.toml',ROOT/'Cargo.lock'],key=lambda p:str(p.relative_to(ROOT))):h.update(str(p.relative_to(ROOT)).encode()+b'\0'+p.read_bytes()+b'\0')
 return h.hexdigest()
assert hashlib.sha256(binary.read_bytes()).hexdigest()==expected_sha and source_hash()==expected_source
run=Path(tempfile.mkdtemp(prefix='refresh-freeze3-',dir=ROOT/'artifacts/independent-m4/native'));processes=[];logs=[];x11=None
report={'result':'failed','binary_sha256':expected_sha,'production_source_sha256':expected_source,'run':str(run),'scope':'Only freeze2→3 affected native success/error footer completion paths, real XTest report date/Return, complete State unchanged; footer label distinguished from page status by ancestor scroll pane','checks':[]}
def start(cmd,name,env=None,**kw):
 log=(run/(name+'.log')).open('w');logs.append(log);p=subprocess.Popen(cmd,env=env,stdout=log,stderr=log,start_new_session=True,**kw);processes.append(p);return p
def check(name,value,**detail):report['checks'].append({'name':name,'passed':bool(value),**detail});print('CHECK',name,bool(value),flush=True);assert value,name
try:
 with tempfile.TemporaryFile() as fd:
  start(['Xvfb','-displayfd',str(fd.fileno()),'-screen','0','1450x1050x24','-nolisten','tcp','-xkbdir',str(native.ENVROOT/'sysroot/usr/share/X11/xkb')],'xvfb',pass_fds=(fd.fileno(),))
  def get_display():fd.seek(0);return fd.read().decode().strip()
  display=':'+native.wait_for(get_display,label='private display')
 runtime=native.private_accessibility_environment(display,run);env=os.environ.copy();env.update(DISPLAY=display,GDK_BACKEND='x11',GSK_RENDERER='cairo',XDG_RUNTIME_DIR=str(runtime),SARV_TEST_INSTANCE='independent-refresh-freeze3',ADW_DEBUG_COLOR_SCHEME='prefer-light',G_DEBUG='fatal-criticals')
 for k in ['SARV_UI_SMOKE','SARV_ACCOUNTING_SMOKE','SARV_CORE_SMOKE','SARV_PROCUREMENT_SMOKE']:env.pop(k,None)
 start(['openbox','--sm-disable','--config-file',str(native.ENVROOT/'sysroot/etc/xdg/openbox/rc.xml')],'openbox',env)
 profile=run/'profile';source=Path(os.environ.get('SARV_M4_REFRESH_FIXTURE',str(ROOT/'artifacts/independent-m4/native/run-yrvj2t60/full')));review.copy_db(source,profile);saved=review.state(profile);env.update(SARV_DATA_DIR=str(profile),SARV_UI_SMOKE='capture',SARV_CAPTURE_PAGE='inventory',SARV_CAPTURE_NAME='fresh.png');app=start([str(binary)],'app',env);x11=native.X11(display);window=native.wait_for(lambda:x11.find(app.pid));native.wait_for(lambda:(profile/'fresh.png').exists());time.sleep(.4);access=Access(app.pid);x11.focus(window)
 def sensitive_showing(row):
  access.a.atspi_accessible_clear_cache(row['node']);s=access.a.atspi_accessible_get_state_set(row['node']);return bool(s and access.a.atspi_state_set_contains(s,24) and access.a.atspi_state_set_contains(s,25))
 def find(name,roles=None):return next((r for r in access.tree() if r['name']==name and (not roles or r['role'] in roles) and sensitive_showing(r)),None)
 def focus(name,roles=None):
  x11.focus(window);row=native.wait_for(lambda:find(name,roles),label=name)
  if access.focus(row):native.wait_for(lambda:access.focused(row),label='focus '+name);return row
  for _ in range(200):
   row=find(name,roles)
   if row and access.focused(row):return row
   x11.key('Tab');time.sleep(.015)
  raise AssertionError('actual Tab focus '+name)
 def press(name):focus(name,['button','push button']);x11.key('Return');time.sleep(.25)
 def fill(name,text):focus(name,['text','entry']);x11.key('a','Control_L');x11.type_ascii(text)
 def shot(name):subprocess.run(['import','-display',display,'-window',str(window),str(run/name)],check=True)
 def outside_scroll(row):
  node=access.a.atspi_accessible_get_parent(row['node'],None)
  for _ in range(60):
   if not node:return True
   if access.string(access.a.atspi_accessible_get_role_name,node,None)=='scroll pane':return False
   node=access.a.atspi_accessible_get_parent(node,None)
  raise AssertionError('unexpected ancestor cycle')
 def canonical(text):return ''.join(c for c in text.translate(str.maketrans('۰۱۲۳۴۵۶۷۸۹٠١٢٣٤٥٦٧٨٩','01234567890123456789')) if unicodedata.category(c)!='Cf')
 def footer(expected):
  return [r for r in access.tree() if r['name']==expected and r['role']=='label' and outside_scroll(r)]
 ready='منابع و گزارش پایان روز آماده است.';error='گزارش پیش از افتتاحیه پشتیبانی نمی‌شود'
 x11.key('9','Alt_L');time.sleep(.3);press('به‌روزرسانی پایان روز');native.wait_for(lambda:footer(ready),label='global footer completed explicitly requested refresh');check('explicit completed refresh replaces global loading footer',len(footer(ready))==1 and review.state(profile)==saved);shot('initial-ready-footer.png');x11.key('9','Alt_L');time.sleep(.3);fill('پایان روز گزارش','2026-03-20');press('به‌روزرسانی پایان روز');native.wait_for(lambda:footer(error),label='global footer reports preopening error');names=[r['name'] for r in access.tree()];check('preopening error replaces global loading footer and leaves exact State',len(footer(error))==1 and review.state(profile)==saved);check('error clears unavailable controls instead of showing stale zero or prior amounts',not any(' • دفتر ' in n and ' • تفصیل ' in n and ' • اختلاف ' in n for n in names));shot('preopening-error-footer.png')
 fill('پایان روز گزارش','2026-03-27');press('به‌روزرسانی پایان روز');native.wait_for(lambda:footer(ready),label='global footer recovered ready');check('corrected report refresh returns global footer to ready with exact State',len(footer(ready))==1 and review.state(profile)==saved);focus('کنترل و تفصیل',['page tab']);x11.key('space');time.sleep(.3);names=[canonical(r['name']) for r in access.tree()];check('corrected refresh restores literal inventory480 AP329 GRNI50 controls',any(canonical('موجودی کالا • دفتر 480 ریال • تفصیل 480 ریال • اختلاف 0 ریال') in n for n in names) and any(canonical('حساب‌های پرداختنی • دفتر 329 ریال • تفصیل 329 ریال • اختلاف 0 ریال') in n for n in names) and any(canonical('کالای دریافت‌شده، صورتحساب‌نشده • دفتر 50 ریال • تفصیل 50 ریال • اختلاف 0 ریال') in n for n in names));shot('recovered-ready-footer.png')
 native.stop(app);bad=[line for line in (run/'app.log').read_text(errors='replace').splitlines() if 'WARNING' in line or 'CRITICAL' in line];check('actual GTK log has no warnings or criticals',not bad,diagnostics=bad);check('production source and pinned executable unchanged',source_hash()==expected_source and hashlib.sha256(binary.read_bytes()).hexdigest()==expected_sha);report['result']='passed'
except Exception as e:
 report['error']=str(e)
 if 'access' in locals():
  (run/'failure-tree.json').write_text(json.dumps([{k:v for k,v in r.items() if k!='node'} for r in access.tree()],ensure_ascii=False,indent=2))
  try:shot('failure.png')
  except Exception:pass
 raise
finally:
 if x11:x11.x.XCloseDisplay(x11.d)
 for p in reversed(processes):native.stop(p)
 for f in logs:f.close()
 (run/'refresh-targeted.json').write_text(json.dumps(report,ensure_ascii=False,indent=2));print(json.dumps(report,ensure_ascii=False,indent=2),flush=True)
