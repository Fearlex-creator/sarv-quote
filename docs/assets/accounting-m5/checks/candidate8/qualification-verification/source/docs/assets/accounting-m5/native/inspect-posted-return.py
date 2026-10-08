#!/usr/bin/env python3
"""Read-only native restart/drilldown supplement for the frozen M5 keyboard return."""
import hashlib,importlib.util,json,os,sqlite3,subprocess,sys,tempfile,time
from pathlib import Path
ROOT=Path('/workspace/sarv-quote');sys.path.insert(0,str(ROOT/'scripts'))
from qa_access import Access
spec=importlib.util.spec_from_file_location('native',ROOT/'scripts/qa-native.py');native=importlib.util.module_from_spec(spec);spec.loader.exec_module(native)
run=Path(sys.argv[1]);manifest=json.loads(Path(sys.argv[2]).read_text());target=run/'posted-return-inspection';target.mkdir()
def provenance():
 h=hashlib.sha256();files={}
 for name in sorted(manifest['frozen_source_provenance']['files'],key=lambda n:n.encode()):
  raw=(ROOT/name).read_bytes();h.update(name.encode()+b'\0'+raw+b'\0');files[name]=hashlib.sha256(raw).hexdigest()
 return {'sha256':h.hexdigest(),'files':files}
before=provenance();assert before=={k:manifest['frozen_source_provenance'][k] for k in ['sha256','files']}
binary=Path(manifest['binary']);assert hashlib.sha256(binary.read_bytes()).hexdigest()==manifest['binary_sha256']
with sqlite3.connect(f'file:{run}/stage/sarv.sqlite3?mode=ro',uri=True) as src,sqlite3.connect(target/'sarv.sqlite3') as dst:src.backup(dst)
def state():
 with sqlite3.connect(f'file:{target}/sarv.sqlite3?mode=ro',uri=True) as d:return json.loads(d.execute('select payload from state where id=1').fetchone()[0])
original=state();processes=[];logs=[];x=None
try:
 def start(cmd,name,env=None,**kwargs):
  log=(target/f'{name}.log').open('w');logs.append(log);p=subprocess.Popen(cmd,stdout=log,stderr=log,env=env,start_new_session=True,**kwargs);processes.append(p);return p
 with tempfile.TemporaryFile() as fd:
  start(['Xvfb','-displayfd',str(fd.fileno()),'-screen','0','1450x1050x24','-nolisten','tcp','-xkbdir',str(native.ENVROOT/'sysroot/usr/share/X11/xkb')],'xvfb',pass_fds=(fd.fileno(),))
  def display_number():fd.seek(0);return fd.read().decode().strip()
  display=':'+native.wait_for(display_number)
 runtime=native.private_accessibility_environment(display,target);env=os.environ.copy();env.update(DISPLAY=display,GDK_BACKEND='x11',GSK_RENDERER='cairo',XDG_RUNTIME_DIR=str(runtime),SARV_DATA_DIR=str(target),SARV_TEST_INSTANCE='m5-read-only-return-inspection',SARV_UI_SMOKE='capture',SARV_CAPTURE_PAGE='sales',SARV_CAPTURE_NAME='restarted-sales.png',ADW_DEBUG_COLOR_SCHEME='prefer-dark')
 for name in ['SARV_SALES_SMOKE','SARV_CORE_SMOKE','SARV_PROCUREMENT_SMOKE','SARV_ACCOUNTING_SMOKE']:env.pop(name,None)
 start(['openbox','--sm-disable','--config-file',str(native.ENVROOT/'sysroot/etc/xdg/openbox/rc.xml')],'openbox',env)
 app=start([str(binary)],'app',env);x=native.X11(display);window=native.wait_for(lambda:x.find(app.pid));native.wait_for(lambda:(target/'restarted-sales.png').exists());x.focus(window);access=Access(app.pid)
 x.key('0','Alt_L');time.sleep(.3);x.key('f','Control_L');x.type_ascii('KBD-RET');time.sleep(.3)
 native.wait_for(lambda:access.text(access.find('جست‌وجوی شماره، مشتری، کالا یا پیش‌نویس',['text','entry']))=='KBD-RET')
 for _ in range(60):
  row=access.find('جزئیات و جریان کار',['button'])
  if row and access.focused(row):break
  x.key('Tab');time.sleep(.02)
 else:raise AssertionError('exact source button never focused')
 x.key('Return');native.wait_for(lambda:any('کالا به انبار اصلی بازگشت' in r['name'] for r in access.tree()));native.wait_for(lambda:any(r['name']=='منبع فروش و جریان کار آماده‌اند.' for r in access.tree()));time.sleep(.4)
 subprocess.run(['/usr/bin/import','-display',display,'-window',str(window),str(target/'M5-real-keyboard-completed-return-detail.png')],check=True)
 tree=[{k:v for k,v in r.items() if k!='node'} for r in access.tree()];(target/'completed-return-access-tree.json').write_text(json.dumps(tree,ensure_ascii=False,indent=2))
 assert state()==original
 returns=[r for r in original['accounting']['sales']['records'] if 'Return' in r['data']];assert len(returns)==1 and returns[0]['data']['Return']['total']==40 and returns[0]['data']['Return']['cost']==23
 after=provenance();assert after==before;assert hashlib.sha256(binary.read_bytes()).hexdigest()==manifest['binary_sha256']
 result={'result':'passed','scope':'fresh frozen release native read-only restart + real Alt0/CtrlF/Tab/Enter linked-return detail','binary_sha256':manifest['binary_sha256'],'source_before':before,'source_after':after,'financial_state_unchanged':True,'credit':40,'original_cost':23,'completed_idle_footer':True,'screenshot':'M5-real-keyboard-completed-return-detail.png'}
 (target/'completed-return-inspection.json').write_text(json.dumps(result,ensure_ascii=False,indent=2));print(json.dumps({k:v for k,v in result.items() if k not in ['source_before','source_after']},indent=2))
finally:
 if x:x.x.XCloseDisplay(x.d)
 for p in reversed(processes):
  if p.poll() is None:
   p.terminate()
   try:p.wait(timeout=3)
   except subprocess.TimeoutExpired:p.kill()
 for log in logs:log.close()
