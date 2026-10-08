#!/usr/bin/env python3
"""Historical candidate3 focus diagnostic; no current source qualification."""
import ctypes as C,hashlib,importlib.util,json,os,sqlite3,subprocess,sys,tempfile,time
from pathlib import Path
ROOT=Path('/workspace/sarv-quote');sys.path.insert(0,str(ROOT/'scripts'))
from qa_access import Access
s=importlib.util.spec_from_file_location('native',ROOT/'scripts/qa-native.py');n=importlib.util.module_from_spec(s);s.loader.exec_module(n)
full_debug='--full-debug' in sys.argv
binary=ROOT/'target/debug/sarv-quote' if full_debug else Path('/workspace/sarv-evidence/bin/sarv-M5-candidate3');run=Path(tempfile.mkdtemp(prefix='c3-focus-',dir=ROOT/'artifacts/sales-native'));profile=run/'profile';profile.mkdir();source=ROOT/('artifacts/sales-native/run-pi197t86/m4-before' if full_debug else 'artifacts/sales-native/run-pi197t86/full');procs=[];logs=[];x=None
with sqlite3.connect(f'file:{source}/sarv.sqlite3?mode=ro',uri=True) as src,sqlite3.connect(profile/'sarv.sqlite3') as dst:src.backup(dst)
def payload(p):
 with sqlite3.connect(f'file:{p}/sarv.sqlite3?mode=ro',uri=True) as db:return db.execute('select generation,payload from state where id=1').fetchone()
if full_debug:
 (profile/'M5-synthetic-prices.csv').write_bytes((ROOT/'artifacts/sales-native/run-pi197t86/full/M5-synthetic-prices.csv').read_bytes())
original=payload(source);initial=payload(profile);result={'scope':'historical C3 read-only diagnostic, no current production claim','binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'steps':[]}
def start(cmd,name,env=None,**kw):
 f=(run/(name+'.log')).open('w');logs.append(f);p=subprocess.Popen(cmd,env=env,stdout=f,stderr=f,**kw);procs.append(p);return p
try:
 with tempfile.TemporaryFile() as fd:
  start(['Xvfb','-displayfd',str(fd.fileno()),'-screen','0','1450x1050x24','-nolisten','tcp','-xkbdir',str(n.ENVROOT/'sysroot/usr/share/X11/xkb')],'xvfb',pass_fds=(fd.fileno(),))
  def displaynum():fd.seek(0);return fd.read().decode().strip()
  display=':'+n.wait_for(displaynum)
 runtime=n.private_accessibility_environment(display,run);env=os.environ.copy();env.update(DISPLAY=display,GDK_BACKEND='x11',GSK_RENDERER='cairo',XDG_RUNTIME_DIR=str(runtime),SARV_TEST_INSTANCE='c3-focus-diagnostic',SARV_DATA_DIR=str(profile),SARV_UI_SMOKE='capture',SARV_CAPTURE_PAGE='sales',SARV_CAPTURE_NAME='ready.png',ADW_DEBUG_COLOR_SCHEME='prefer-light')
 for key in ['SARV_SALES_SMOKE','SARV_CORE_SMOKE','SARV_PROCUREMENT_SMOKE','SARV_ACCOUNTING_SMOKE']:env.pop(key,None)
 if full_debug:
  env.pop('SARV_UI_SMOKE',None);env.update(SARV_SALES_SMOKE='full',SARV_SALES_FOCUS_DIAGNOSTIC='1')
 start(['openbox','--sm-disable','--config-file',str(n.ENVROOT/'sysroot/etc/xdg/openbox/rc.xml')],'openbox',env);app=start([str(binary)],'app',env);x=n.X11(display);window=n.wait_for(lambda:x.find(app.pid));n.wait_for(lambda:(profile/('sales-native-verification.json' if full_debug else 'ready.png')).exists(),seconds=200);x.focus(window);a=Access(app.pid)
 x.x.XGetInputFocus.argtypes=[C.c_void_p,C.POINTER(C.c_ulong),C.POINTER(C.c_int)];x.x.XGetInputFocus.restype=C.c_int
 def flags(r):
  a.a.atspi_accessible_clear_cache(r['node']);ss=a.a.atspi_accessible_get_state_set(r['node']);return [i for i in [0,1,11,12,24,25,30] if ss and a.a.atspi_state_set_contains(ss,i)]
 def snapshot(label):
  fw=C.c_ulong();rev=C.c_int();x.x.XGetInputFocus(x.d,C.byref(fw),C.byref(rev));rows=a.tree();data={'step':label,'x_input_focus':fw.value,'owned_window':window,'client_windows':x.prop(x.root,'_NET_CLIENT_LIST'),'focused':[{'name':r['name'],'role':r['role'],'rect':a.rect(r),'flags':flags(r)} for r in rows if a.focused(r)],'top':[{'name':r['name'],'role':r['role'],'rect':a.rect(r),'flags':flags(r)} for r in rows if r['name'] in ['سفارش فروش تازه','از پیشنهاد تأییدشده','به‌روزرسانی']]};result['steps'].append(data);(run/'progress.json').write_text(json.dumps(result,ensure_ascii=False,indent=2))
 def tabs(label):
  found=False
  for i in range(40):
   snapshot(label+':'+str(i))
   if any(r['name']=='سفارش فروش تازه' and a.focused(r) for r in a.tree()):found=True;break
   x.key('Tab');time.sleep(.04)
  subprocess.run(['/usr/bin/import','-display',display,'-window',str(window),str(run/(label+'.png'))],check=True);return found
 snapshot('ready');result['fresh_top_tab']=tabs('fresh')
 x.key('0','Alt_L');time.sleep(.3);result['after_alt0_top_tab']=tabs('alt0')
 x.key('f','Control_L');time.sleep(.2);x.type_ascii('SO-M5');n.wait_for(lambda:a.text(a.find('جست‌وجوی شماره، مشتری، کالا یا پیش‌نویس',['text','entry']))=='SO-M5');snapshot('actual_ctrlf_typed');result['after_ctrlf_top_tab']=tabs('ctrlf')
 result['source_unchanged']=payload(source)==original;result['profile_unchanged']=payload(profile)==initial;result['full_debug']=full_debug
finally:
 (run/'focus-diagnostic.json').write_text(json.dumps(result,ensure_ascii=False,indent=2));print(json.dumps({'run':str(run),**{k:v for k,v in result.items() if k!='steps'}},ensure_ascii=False))
 if x:x.x.XCloseDisplay(x.d)
 for p in reversed(procs):
  if p.poll() is None:
   p.terminate()
   try:p.wait(timeout=3)
   except subprocess.TimeoutExpired:p.kill()
 for f in logs:f.close()
