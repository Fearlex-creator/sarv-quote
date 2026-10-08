from pathlib import Path
import json,hashlib,subprocess,time,shutil
ROOT=Path('/workspace/sarv-quote');manifest=ROOT/'docs/assets/accounting-m6/checks/candidate6/manifest.json';expected=json.loads(manifest.read_text());out=ROOT/'docs/assets/accounting-m6/exports/candidate6-current';assert not out.exists();out.mkdir(parents=True)
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
wrapper=sha(Path(__file__))
def guard():
 assert sha(Path(__file__))==wrapper
 record={}
 for key in ['frozen_source_provenance','verification_source_provenance','compile_input_provenance']:
  files={name:sha(ROOT/name) for name in expected[key]['files']};assert files==expected[key]['files'],key;record[key]=files
 assert sha(Path(expected['binary_path']))==expected['binary_sha256']
 return record
before=guard();report={'source_manifest':str(manifest),'source_manifest_sha256':sha(manifest),'binary_sha256':expected['binary_sha256'],'guard_before':before,'wrapper_sha256':wrapper,'scope':'Focused four export tests at qualified current source with synthetic actual PDF/CSV samples; does not replace independent all-page visual review','passed':False};(out/'manifest.json').write_text(json.dumps(report,indent=2)+'\n')
command='source /workspace/sarv-environment/activate.sh\nSARV_PURCHASE_RETURN_EXPORT_ARTIFACT_DIR='+str(out)+' cargo test --offline --locked --test purchase_return_exports'
start=time.monotonic()
with (out/'test.log').open('w') as f:r=subprocess.run(['bash','-c',command],cwd=ROOT,stdout=f,stderr=subprocess.STDOUT)
report.update(command=command,exit_code=r.returncode,elapsed_seconds=time.monotonic()-start,guard_after=guard(),test_log_sha256=sha(out/'test.log'),passed=r.returncode==0)
shutil.copy2(__file__,out/Path(__file__).name)
report['artifact_hashes']={p.name:sha(p) for p in sorted(out.iterdir()) if p.is_file() and p.name!='manifest.json'}
(out/'manifest.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:v for k,v in report.items() if k not in ['guard_before','guard_after','artifact_hashes']}),flush=True);raise SystemExit(r.returncode)
