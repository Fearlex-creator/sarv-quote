import hashlib,json,subprocess,shutil
from pathlib import Path
ROOT=Path('/workspace/sarv-quote');base=ROOT/'docs/assets/accounting-m6/checks/candidate8';manifest=base/'manifest.json';qualification=json.loads(manifest.read_text());out=base/'sales';assert not out.exists();out.mkdir()
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
helpers=[ROOT/'scripts'/n for n in ['sales-qa.py','qa-native.py','qa_access.py']];binary=Path(qualification['binary_path']);selfpath=Path(__file__)
def guard():
 for group in ['frozen_source_provenance','verification_source_provenance','compile_input_provenance']:
  for name,value in qualification[group]['files'].items():assert sha(ROOT/name)==value,name
 assert sha(binary)==qualification['binary_sha256']
 return {'manifest':sha(manifest),'binary':sha(binary),'helpers':{str(p.relative_to(ROOT)):sha(p) for p in helpers},'wrapper':sha(selfpath)}
before=guard();command='source /workspace/sarv-environment/activate.sh\nSARV_QA_BINARY=/workspace/sarv-evidence/bin/sarv-M6-candidate8 SARV_SALES_QA_FATAL_CRITICAL=1 SARV_SALES_SOURCE_MANIFEST=/workspace/sarv-quote/docs/assets/accounting-m6/checks/candidate8/manifest.json dbus-run-session -- python3 scripts/sales-qa.py'
with (out/'run.log').open('w') as f:r=subprocess.run(['bash','-c',command],cwd=ROOT,stdout=f,stderr=subprocess.STDOUT)
after=guard();assert before==after
lines=(out/'run.log').read_text().splitlines();summary=json.loads(next(line for line in reversed(lines) if line.startswith('{"result"')));raw=Path(summary['run']);native=json.loads((raw/'sales-independent-native.json').read_text());portable=ROOT/'docs/assets/accounting-m6/native/sales-candidate8';assert not portable.exists();portable.mkdir();files={}
for p in sorted(raw.rglob('*')):
 if p.is_file() and p.suffix in ['.json','.jsonl','.png','.log','.txt','.csv','.pdf']:
  relative=p.relative_to(raw);target=portable/relative;target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,target);assert sha(p)==sha(target);files[str(relative)]=sha(target)
shutil.copy2(selfpath,out/selfpath.name)
report={'passed':r.returncode==0 and native['result']=='passed','command':command,'exit_code':r.returncode,'summary':summary,'guard_before':before,'guard_after':after,'guard_stable':True,'portable_files':files,'raw_run':str(raw),'scope':'One fresh C8 affected Sales native suite with real keys/raw books/current load status, private synthetic profiles, fatal application criticals. Old M3 seed explicitly historical; no original user profile changes.'}
(out/'manifest.json').write_text(json.dumps(report,indent=2)+'\n');(portable/'portable-manifest.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(summary));assert report['passed']
