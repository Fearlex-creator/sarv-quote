from pathlib import Path
import sys,os,subprocess,tempfile,time,json,hashlib,importlib.util
root=Path('/workspace/sarv-quote');sys.path.insert(0,str(root/'scripts'))
spec=importlib.util.spec_from_file_location('native_qa',root/'scripts/qa-native.py');native=importlib.util.module_from_spec(spec);spec.loader.exec_module(native)
manifest=Path(sys.argv[1]).resolve();meta=json.loads(manifest.read_text());binary=Path(meta['binary']);out=Path(sys.argv[2]).resolve();assert not out.exists();out.mkdir(parents=True);profile=out/'profile';profile.mkdir();processes=[];logs=[]
def guard():
 h=hashlib.sha256();actual={}
 for name,expected in sorted(meta['frozen_source_provenance']['files'].items(),key=lambda item:item[0].encode()):
  b=(root/name).read_bytes();actual[name]=hashlib.sha256(b).hexdigest();assert actual[name]==expected,name;h.update(name.encode()+b'\0'+b+b'\0')
 assert h.hexdigest()==meta['production_source_sha256']
 assert hashlib.sha256(binary.read_bytes()).hexdigest()==meta['binary_sha256']
 return {'production_source_sha256':h.hexdigest(),'files':actual,'binary_sha256':meta['binary_sha256']}
def start(cmd,name,env=None,**kwargs):
 log=(out/(name+'.log')).open('w');logs.append(log);p=subprocess.Popen(cmd,env=env,stdout=log,stderr=log,start_new_session=True,**kwargs);processes.append(p);return p
before=guard();(out/'before.json').write_text(json.dumps(before,indent=2)+'\n');started=time.monotonic()
try:
 with tempfile.TemporaryFile() as fd:
  start(['Xvfb','-displayfd',str(fd.fileno()),'-screen','0','1450x1050x24','-nolisten','tcp','-xkbdir','/workspace/sarv-environment/sysroot/usr/share/X11/xkb'],'xvfb',pass_fds=(fd.fileno(),))
  def display():fd.seek(0);return fd.read().decode().strip()
  d=':'+native.wait_for(display,label='owned private X11')
 runtime=native.private_accessibility_environment(d,out)
 env=os.environ.copy();env.update(DISPLAY=d,GDK_BACKEND='x11',GSK_RENDERER='cairo',XDG_RUNTIME_DIR=str(runtime),SARV_TEST_INSTANCE='m5-quote-callback-qa',SARV_DATA_DIR=str(profile),SARV_QUOTE_CALLBACK_QA='1',G_DEBUG='fatal-criticals')
 for key in ['SARV_UI_SMOKE','SARV_CORE_SMOKE','SARV_PROCUREMENT_SMOKE','SARV_SALES_SMOKE','SARV_ACCOUNTING_SMOKE']:env.pop(key,None)
 start(['openbox','--sm-disable','--config-file','/workspace/sarv-environment/sysroot/etc/xdg/openbox/rc.xml'],'openbox',env)
 app=start([str(binary)],'application',env)
 def result():
  failure=profile/'quote-callback-failure.txt';assert not failure.exists(),failure.read_text() if failure.exists() else ''
  report=profile/'quote-callback-verification.json'
  if report.exists():
   j=json.loads(report.read_text());assert j['result']=='passed',j;return j
  assert app.poll() is None,'Application exited without callback proof'
 report=native.wait_for(result,seconds=180,label='held-write native quotation callback qualification')
 after=guard();assert before==after;(out/'after.json').write_text(json.dumps(after,indent=2)+'\n')
 (out/'manifest.json').write_text(json.dumps({'result':'passed','elapsed_seconds':time.monotonic()-started,'root_manifest':str(manifest),'root_manifest_sha256':hashlib.sha256(manifest.read_bytes()).hexdigest(),'before':before,'after':after,'report':report},indent=2)+'\n');print(json.dumps({'result':'passed','output':str(out),'checks':len(report.get('checks',[]))}))
finally:
 for p in reversed(processes):native.stop(p)
 for log in logs:log.close()
