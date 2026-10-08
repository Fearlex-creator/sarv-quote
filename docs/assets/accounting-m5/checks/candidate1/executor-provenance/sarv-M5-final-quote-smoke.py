from pathlib import Path
import sys,os,subprocess,tempfile,time,json,hashlib
sys.path.insert(0,'/workspace/sarv-quote/scripts')
import importlib.util
s=importlib.util.spec_from_file_location('qa_native','/workspace/sarv-quote/scripts/qa-native.py');n=importlib.util.module_from_spec(s);s.loader.exec_module(n)
binary=Path(sys.argv[1]).resolve();expected=sys.argv[2];freeze=sys.argv[3]
r=Path('/workspace/sarv-quote');out=r/'artifacts/m5-preserved';out.mkdir(exist_ok=True);p=Path(tempfile.mkdtemp(prefix='quote-',dir=out));processes=[];logs=[]
def start(cmd,name,env=None,**kw):
 log=(p/(name+'.log')).open('w');logs.append(log);a=subprocess.Popen(cmd,env=env,stdout=log,stderr=log,start_new_session=True,**kw);processes.append(a);return a
try:
 with tempfile.TemporaryFile() as fd:
  x=start(['Xvfb','-displayfd',str(fd.fileno()),'-screen','0','1450x1050x24','-nolisten','tcp','-xkbdir','/workspace/sarv-environment/sysroot/usr/share/X11/xkb'],'xvfb',pass_fds=(fd.fileno(),))
  def disp():fd.seek(0);return fd.read().decode().strip()
  d=':'+n.wait_for(disp,label='private display')
 runtime=n.private_accessibility_environment(d,p)
 env=os.environ.copy();env.update(DISPLAY=d,GDK_BACKEND='x11',GSK_RENDERER='cairo',XDG_RUNTIME_DIR=str(runtime),SARV_TEST_INSTANCE='m5-preserved-quote',SARV_DATA_DIR=str(p),SARV_UI_SMOKE='full',G_DEBUG='fatal-criticals')
 start(['openbox','--sm-disable','--config-file','/workspace/sarv-environment/sysroot/etc/xdg/openbox/rc.xml'],'openbox',env)
 a=start([str(binary)],'quote',env)
 def verified():
  assert a.poll() is None,'Native process exited'
  f=p/'native-verification.json'
  if f.exists():
   j=json.loads(f.read_text());assert j['result']=='passed',j;return j
 j=n.wait_for(verified,seconds=150,label='full native quotation smoke')
 assert hashlib.sha256(binary.read_bytes()).hexdigest()==expected
 (r/('docs/assets/accounting-m5/checks/'+freeze+'-quote-source.json')).write_text(json.dumps({'profile':str(p),'binary':str(binary),'binary_sha256':expected,'native':j},indent=2))
 print(p,flush=True)
finally:
 for a in reversed(processes):n.stop(a)
 for l in logs:l.close()
