import hashlib,json,subprocess,shutil
from pathlib import Path
ROOT=Path('/workspace/sarv-quote');base=ROOT/'docs/assets/accounting-m6/checks/candidate8';manifest=base/'manifest.json';qualification=json.loads(manifest.read_text());out=base/'sidebar';assert not out.exists();out.mkdir()
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
helpers=[ROOT/'scripts'/n for n in ['native-sidebar-qa.py','qa-native.py','qa_access.py']]
binary=Path(qualification['binary_path']);selfpath=Path(__file__)
def guard():
 for group in ['frozen_source_provenance','verification_source_provenance','compile_input_provenance']:
  for name,value in qualification[group]['files'].items():assert sha(ROOT/name)==value,name
 assert sha(binary)==qualification['binary_sha256']
 return {'manifest':sha(manifest),'binary':sha(binary),'helpers':{str(p.relative_to(ROOT)):sha(p) for p in helpers},'wrapper':sha(selfpath)}
before=guard();command='source /workspace/sarv-environment/activate.sh\nSARV_QA_BINARY=/workspace/sarv-evidence/bin/sarv-M6-candidate8 dbus-run-session -- python3 scripts/native-sidebar-qa.py'
with (out/'run.log').open('w') as f:r=subprocess.run(['bash','-c',command],cwd=ROOT,stdout=f,stderr=subprocess.STDOUT)
after=guard();assert before==after
lines=(out/'run.log').read_text().splitlines();summary=json.loads(next(line for line in reversed(lines) if line.startswith('{"result"')));profile=Path(summary['profile']);native=json.loads((profile/'sidebar-verification.json').read_text());portable=ROOT/'docs/assets/accounting-m6/native/sidebar-candidate8';assert not portable.exists();portable.mkdir()
files={}
for p in sorted(profile.iterdir()):
 if p.is_file() and p.suffix in ['.json','.jsonl','.png','.log','.txt']:
  target=portable/p.name;shutil.copy2(p,target);assert sha(p)==sha(target);files[p.name]=sha(target)
shutil.copy2(selfpath,out/selfpath.name)
report={'passed':r.returncode==0 and native['result']=='passed','command':command,'exit_code':r.returncode,'summary':summary,'guard_before':before,'guard_after':after,'guard_stable':True,'portable_files':files,'raw_profile':str(profile),'scope':'One fresh C7 native shell run; actual focus/navigation/pointer/settings/restart and full synthetic State equality. Private X11; no Wayland/screen-reader claim.'}
(out/'manifest.json').write_text(json.dumps(report,indent=2)+'\n');(portable/'portable-manifest.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(summary));assert report['passed']
