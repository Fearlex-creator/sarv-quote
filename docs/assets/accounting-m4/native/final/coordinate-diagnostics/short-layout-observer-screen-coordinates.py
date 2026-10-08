import sys,os,subprocess,tempfile,time,sqlite3,json,hashlib,shutil
from pathlib import Path
sys.path.insert(0,'/workspace/sarv-quote/scripts')
import importlib.util
spec=importlib.util.spec_from_file_location('n','/workspace/sarv-quote/scripts/qa-native.py');n=importlib.util.module_from_spec(spec);spec.loader.exec_module(n)
from qa_access import Access
root=Path('/workspace/sarv-quote');out=root/'artifacts/procurement-short-native';out.mkdir(exist_ok=True);run=Path(tempfile.mkdtemp(prefix='run-',dir=out));profile=run/'profile';profile.mkdir()
source=Path(os.environ.get('SARV_QA_BINARY',str(root/'target/debug/sarv-quote')));binary=run/'sarv-quote';shutil.copy2(source,binary)
fixture=Path(os.environ['SARV_M4_NATIVE_FIXTURE'])
with sqlite3.connect(f'file:{fixture}/sarv.sqlite3?mode=ro',uri=True)as s,sqlite3.connect(profile/'sarv.sqlite3')as d:s.backup(d)
def accounting():
 with sqlite3.connect(profile/'sarv.sqlite3')as d:return json.loads(d.execute('select payload from state where id=1').fetchone()[0])['accounting']
before=accounting();procs=[];logs=[];report={'result':'failed','binary':str(source),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'checks':[],'key_events':[],'fixture_profile':str(fixture),'run':str(run),'scope':'Actual private XTest Tab navigation and auto-scroll in short native windows; synthetic10record5journal fixture copied read-only; no financial writes.'}
def start(cmd,name,env=None,**kw):
 log=(run/(name+'.log')).open('w');logs.append(log);p=subprocess.Popen(cmd,env=env,stdout=log,stderr=log,start_new_session=True,**kw);procs.append(p);return p
try:
 with tempfile.TemporaryFile()as fd:
  start(['Xvfb','-displayfd',str(fd.fileno()),'-screen','0','1450x1050x24','-nolisten','tcp','-xkbdir',str(n.ENVROOT/'sysroot/usr/share/X11/xkb')],'xvfb',pass_fds=(fd.fileno(),))
  def disp():fd.seek(0);return fd.read().decode().strip()
  display=':'+n.wait_for(disp)
 runtime=n.private_accessibility_environment(display,run);env=os.environ.copy();env.update(SARV_TEST_INSTANCE='M4-short-layout',SARV_DATA_DIR=str(profile),DISPLAY=display,GDK_BACKEND='x11',GSK_RENDERER='cairo',XDG_RUNTIME_DIR=str(runtime),G_DEBUG='fatal-criticals')
 for k in ['SARV_PROCUREMENT_SMOKE','SARV_CORE_SMOKE','SARV_UI_SMOKE']:env.pop(k,None)
 start(['openbox','--sm-disable','--config-file',str(n.ENVROOT/'sysroot/etc/xdg/openbox/rc.xml')],'openbox',env)
 for appearance in ['light','dark']:
  env['ADW_DEBUG_COLOR_SCHEME']='prefer-'+appearance;p=start([str(binary)],appearance,env);x=n.X11(display);w=n.wait_for(lambda:x.find(p.pid));a=Access(p.pid);x.focus(w)
  def focus(name):
   for i in range(140):
    target=a.find(name,['button','push button'])
    if target and a.focused(target):
     rect=a.rect(target);frame=next((a.rect(r)for r in a.tree()if r['role']=='frame'),None)
     assert rect and frame and rect['width']>0 and rect['height']>0,(name,rect,frame)
     assert rect['x']>=frame['x'] and rect['y']>=frame['y'] and rect['x']+rect['width']<=frame['x']+frame['width'] and rect['y']+rect['height']<=frame['y']+frame['height'],(name,rect,frame)
     report['key_events'].append({'target':name,'actual_tab_presses':i,'focused_control_rect':rect,'frame_rect':frame,'whole_control_inside_window':True})
     return
    if p.poll()is not None:raise Exception('Owned native process exited '+str(p.returncode))
    x.key('Tab');time.sleep(.02)
   raise Exception('Actual Tab focus '+name)
  for page,key,primary in [('procurement','8','سفارش خرید تازه'),('inventory','9','انتقال بین انبارها')]:
   x.key(key,'Alt_L');time.sleep(.35)
   for width,height in [(900,560),(650,550)]:
    x.resize(w,width,height);n.wait_for(lambda:x.geometry(w)==[width,height],label='short native exact geometry');time.sleep(.3)
    x.key('f','Control_L');n.wait_for(lambda:any(r['name']=='جست‌وجوی شماره، منبع، کالا یا طرف' and a.focused(r)for r in a.tree()),label='short CtrlF active search')
    focus(primary);time.sleep(.15)
    subprocess.run(['import','-display',display,'-window',str(w),str(run/f'{page}-{appearance}-{width}x{height}-focused-action.png')],check=True)
    focus('به‌روزرسانی پایان روز');time.sleep(.15)
    subprocess.run(['import','-display',display,'-window',str(w),str(run/f'{page}-{appearance}-{width}x{height}-focused-refresh.png')],check=True)
    report['checks'].append({'page':page,'appearance':appearance,'geometry':[width,height],'actual_primary_Tab_focus':True,'actual_footer_Tab_focus':True,'automatic_scroll_to_visible_focus':True})
  x.x.XCloseDisplay(x.d);n.stop(p)
 report['accounting_exactly_preserved']=accounting()==before;assert report['accounting_exactly_preserved'];report['result']='passed'
finally:
 for p in reversed(procs):
  if p.poll()is None:n.stop(p)
 for log in logs:log.close()
 report['gtk_diagnostics']={f.name:f.read_text()for f in run.glob('*.log')if f.name in ['light.log','dark.log']}
 if any(report['gtk_diagnostics'].values()):report['result']='failed';report['error']='Native app diagnostics must be empty, retained in report'
 (run/'short-native-layout.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n');print(json.dumps(report,ensure_ascii=False,indent=2),flush=True)

assert report['result']=='passed',report.get('error',report)
