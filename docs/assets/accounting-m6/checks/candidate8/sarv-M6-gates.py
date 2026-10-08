import hashlib,json,subprocess,time,sys,shutil,re
from pathlib import Path
ROOT=Path('/workspace/sarv-quote'); out=ROOT/sys.argv[1]; assert not out.exists();out.mkdir(parents=True)
def capture(kind):
 paths=([*ROOT.glob('src/**/*.rs'),ROOT/'src/style.css',ROOT/'Cargo.toml',ROOT/'Cargo.lock'] if kind=='production' else [*ROOT.glob('tests/*.rs')])
 files={}; digest=hashlib.sha256()
 for p in sorted(paths,key=lambda p:p.relative_to(ROOT).as_posix().encode()):
  name=p.relative_to(ROOT).as_posix(); raw=p.read_bytes();files[name]=hashlib.sha256(raw).hexdigest();digest.update(name.encode()+b'\0'+raw+b'\0'); dest=out/'source-before'/name;dest.parent.mkdir(parents=True,exist_ok=True)
  if not dest.exists():dest.write_bytes(raw)
 return {'sha256':digest.hexdigest(),'files':files,'file_count':len(files),'algorithm':'UTF-8 relative path byte order; path,NUL,raw bytes,NUL'}
def input_guards():
 paths=set()
 for source in [*ROOT.glob('src/**/*.rs'),*ROOT.glob('tests/*.rs')]:
  for name in re.findall(r'include_(?:bytes|str)!\s*\(\s*"([^"]+)"',source.read_text()):paths.add((source.parent/name).resolve())
 files={};digest=hashlib.sha256()
 for p in sorted(paths,key=lambda p:p.relative_to(ROOT).as_posix().encode()):
  name=p.relative_to(ROOT).as_posix();raw=p.read_bytes();files[name]=hashlib.sha256(raw).hexdigest();digest.update(name.encode()+b'\0'+raw+b'\0');dest=out/'source-before'/name;dest.parent.mkdir(parents=True,exist_ok=True)
  if not dest.exists():dest.write_bytes(raw)
 return {'files':files,'sha256':digest.hexdigest(),'file_count':len(files),'scope':'Static include_bytes/include_str compile inputs from production and Rust tests, including fonts and historical fixtures'}
inputs=input_guards();prod=capture('production');verify=capture('verification'); report={'frozen_source_provenance':prod,'verification_source_provenance':verify,'compile_input_provenance':inputs,'checks':[],'passed':False,'verification_scope':'Compiled Rust test sources; native/Python observers and their helpers are frozen separately at each actual execution and final source snapshot.'}
(out/'production-before.json').write_text(json.dumps(prod,indent=2)+'\n');(out/'verification-before.json').write_text(json.dumps(verify,indent=2)+'\n')
commands=[('fmt','cargo fmt --all -- --check'),('clippy','cargo clippy --offline --locked --all-targets -- -D warnings'),('all-targets','cargo test --offline --locked --all-targets'),('font-opt-in','cargo test --offline --locked --bin sarv-quote ui::background::tests::four_row_business_quote_with_native_fonts_fits_one_a4_page -- --ignored --exact'),('actual-M3-refusal','SARV_M3_COMPAT_BINARY=/workspace/sarv-evidence/bin/sarv-M3-final cargo test --offline --locked --test procurement_migration actual_m3_binary_rejects_current_schema_without_modifying_committed_row -- --ignored --exact'),('actual-M4-refusal','SARV_M4_COMPAT_BINARY=/workspace/sarv-evidence/bin/sarv-M4-final cargo test --offline --locked --test sales_migration actual_m4_binary_refuses_current_schema_without_changing_row -- --ignored --exact'),('actual-M5-refusal','SARV_M6_MIGRATION_ARTIFACT_DIR='+str(out/'migration')+' SARV_M5_COMPAT_BINARY=/workspace/sarv-evidence/bin/sarv-M5-candidate12 cargo test --offline --locked --test purchase_return_migration actual_m5_binary_refuses_schema7_without_modifying_committed_row -- --ignored --exact'),('gtk-lifecycle','python3 /tmp/sarv-M5-gtk-gate.py '+str(out/'gtk-owned.log')),('release','cargo build --release --offline --locked')]
for label,cmd in commands:
 started=time.monotonic(); log=out/(label+'.log')
 with log.open('w') as f:r=subprocess.run(['bash','-c','source /workspace/sarv-environment/activate.sh\n'+cmd],cwd=ROOT,stdout=f,stderr=subprocess.STDOUT)
 entry={'label':label,'command':cmd,'exit_code':r.returncode,'elapsed_seconds':time.monotonic()-started,'log_sha256':hashlib.sha256(log.read_bytes()).hexdigest()};report['checks'].append(entry);print(json.dumps(entry),flush=True);(out/'manifest.json').write_text(json.dumps(report,indent=2)+'\n')
 if r.returncode: print(log.read_text()[-5000:],flush=True);sys.exit(r.returncode)
assert capture('production')==prod and capture('verification')==verify and input_guards()==inputs,'Source changed during qualification'
binary=ROOT/'target/release/sarv-quote';dest=Path('/workspace/sarv-evidence/bin')/('sarv-M6-'+out.name);assert not dest.exists();shutil.copy2(binary,dest);report.update(passed=True,binary_path=str(dest),binary_sha256=hashlib.sha256(dest.read_bytes()).hexdigest(),binary_bytes=dest.stat().st_size)
ordinary=(out/'all-targets.log').read_text();summaries=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',ordinary);report['ordinary_test_results']={'passed':sum(int(p) for p,f,i in summaries),'failed':sum(int(f) for p,f,i in summaries),'ignored':sum(int(i) for p,f,i in summaries),'suite_summaries':len(summaries)}
report['source_stable']=True;(out/'manifest.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:v for k,v in report.items() if k not in ['frozen_source_provenance','verification_source_provenance','compile_input_provenance','checks']}),flush=True)
