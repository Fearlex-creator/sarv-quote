import hashlib,json,os,subprocess,sys,time
from pathlib import Path
root=Path('/workspace/sarv-quote');m=Path(sys.argv[1]);out=Path(sys.argv[2]);assert not out.exists();out.mkdir(parents=True)
j=json.loads(m.read_text());results=[]
def guard():
 e=j['frozen_source_provenance'];h=hashlib.sha256();files={}
 paths=sorted([*root.glob('src/**/*.rs'),root/'src/style.css',root/'Cargo.toml',root/'Cargo.lock'],key=lambda p:p.relative_to(root).as_posix().encode())
 for p in paths:
  name=p.relative_to(root).as_posix();b=p.read_bytes();h.update(name.encode()+b'\0'+b+b'\0');files[name]=hashlib.sha256(b).hexdigest()
 assert files==e['files'] and h.hexdigest()==e['sha256'];assert hashlib.sha256(Path(j['binary']).read_bytes()).hexdigest()==j['binary_sha256']
 assert hashlib.sha256((root/'target/release/sarv-quote').read_bytes()).hexdigest()==j['binary_sha256'],'Historical runners use release target; require identical frozen executable'
def run(label,cmd):
 guard();start=time.monotonic()
 environment=os.environ.copy();environment.update(SARV_QA_BINARY=j['binary'])
 with (out/(label+'.log')).open('w') as f:
  r=subprocess.run(['bash','-c','source /workspace/sarv-environment/activate.sh\n'+cmd],cwd=root,env=environment,stdout=f,stderr=subprocess.STDOUT)
 guard();results.append({'label':label,'command':cmd,'exit_code':r.returncode,'elapsed_seconds':time.monotonic()-start,'observer_sha256':{str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in root.glob('scripts/*.py')},'log_sha256':hashlib.sha256((out/(label+'.log')).read_bytes()).hexdigest()})
 (out/'manifest.json').write_text(json.dumps({'root_manifest':str(m),'binary_sha256':j['binary_sha256'],'production_source_sha256':j['production_source_sha256'],'checks':results,'passed':all(x['exit_code']==0 for x in results)},indent=2)+'\n')
 assert r.returncode==0,label+' failed; retained evidence; do not waive acceptance'
run('sidebar','dbus-run-session -- python3 scripts/native-sidebar-qa.py')
run('ledger','dbus-run-session -- python3 scripts/accounting-qa.py')
run('procurement','dbus-run-session -- python3 scripts/procurement-qa.py')
run('quote','dbus-run-session -- python3 /tmp/sarv-M5-final-quote-smoke.py '+j['binary']+' '+j['binary_sha256']+' candidate1-supplement-preserved')
run('held-save-close','SARV_CLOSE_TEST_BINARY='+j['binary']+' SARV_CLOSE_TEST_DIR='+str(out.resolve()/'held-save-close-profile')+' dbus-run-session -- python3 scripts/native-save-close.py')
print(json.dumps({'passed':True,'checks':len(results),'output':str(out)}))
