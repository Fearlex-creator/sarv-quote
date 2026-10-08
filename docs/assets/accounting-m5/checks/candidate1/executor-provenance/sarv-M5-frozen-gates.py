import hashlib,json,os,subprocess,sys,time
from pathlib import Path
root=Path('/workspace/sarv-quote');m=Path(sys.argv[1]);out=Path(sys.argv[2]);assert not out.exists();out.mkdir(parents=True)
j=json.loads(m.read_text());expected=j['frozen_source_provenance'];results=[]
def guard():
 h=hashlib.sha256();files={}
 paths=sorted([*root.glob('src/**/*.rs'),root/'src/style.css',root/'Cargo.toml',root/'Cargo.lock'],key=lambda p:p.relative_to(root).as_posix().encode())
 for p in paths:
  name=p.relative_to(root).as_posix();b=p.read_bytes();h.update(name.encode()+b'\0'+b+b'\0');files[name]=hashlib.sha256(b).hexdigest()
 assert files==expected['files'] and h.hexdigest()==expected['sha256'],'Frozen production source changed'
 assert hashlib.sha256(Path(j['binary']).read_bytes()).hexdigest()==j['binary_sha256'],'Frozen binary changed'
def run(label,command):
 guard();t=time.monotonic()
 with (out/(label+'.log')).open('w') as log:
  r=subprocess.run(['bash','-c','source /workspace/sarv-environment/activate.sh\n'+command],cwd=root,stdout=log,stderr=subprocess.STDOUT)
 guard();results.append({'label':label,'command':command,'exit_code':r.returncode,'elapsed_seconds':time.monotonic()-t,'log_sha256':hashlib.sha256((out/(label+'.log')).read_bytes()).hexdigest()})
 (out/'manifest.json').write_text(json.dumps({'root_manifest':str(m),'root_manifest_sha256':hashlib.sha256(m.read_bytes()).hexdigest(),'source':expected['sha256'],'binary':j['binary_sha256'],'checks':results,'passed':all(x['exit_code']==0 for x in results)},indent=2)+'\n')
 assert r.returncode==0,label+' failed'
run('fmt','cargo fmt --all -- --check')
run('clippy','cargo clippy --offline --locked --all-targets -- -D warnings')
run('all-targets','cargo test --offline --locked --all-targets')
run('font-opt-in','cargo test --offline --locked --bin sarv-quote ui::background::tests::four_row_business_quote_with_native_fonts_fits_one_a4_page -- --ignored --exact')
run('actual-m3-refusal','SARV_M3_COMPAT_BINARY=/workspace/sarv-evidence/bin/sarv-M3-final cargo test --offline --locked --test procurement_migration actual_m3_binary_rejects_current_schema_without_modifying_committed_row -- --ignored --exact')
run('actual-m4-refusal','SARV_M4_COMPAT_BINARY=/workspace/sarv-evidence/bin/sarv-M4-final cargo test --offline --locked --test sales_migration actual_m4_binary_refuses_schema6_without_changing_row -- --ignored --exact')
run('gtk-lifecycle','python3 /tmp/sarv-M5-gtk-gate.py '+str(out/'gtk-lifecycle-owned.log'))
print(json.dumps({'passed':True,'checks':len(results),'output':str(out)}))
