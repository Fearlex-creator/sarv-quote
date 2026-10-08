from pathlib import Path
import hashlib,json,shutil,subprocess,sys,time
root=Path('/workspace/sarv-quote');epoch=sys.argv[1]
out=root/'docs/assets/accounting-m5/checks'/epoch
assert not out.exists(), 'New evidence epoch required; never overwrite previous run'
out.mkdir(parents=True)
def sources():
 paths=sorted([*root.glob('src/**/*.rs'),root/'src/style.css',root/'Cargo.toml',root/'Cargo.lock'],key=lambda p:p.relative_to(root).as_posix().encode())
 h=hashlib.sha256();files={}
 for p in paths:
  rel=p.relative_to(root).as_posix();b=p.read_bytes();h.update(rel.encode()+b'\0'+b+b'\0');files[rel]=hashlib.sha256(b).hexdigest()
 return {'sha256':h.hexdigest(),'files':files,'algorithm':'UTF-8 relative-path byte order; path, NUL, raw bytes, NUL'}
before=sources();t=time.monotonic()
with (out/'release-build.log').open('w') as log:
 subprocess.run(['bash','-c','source /workspace/sarv-environment/activate.sh\ncargo build --release --offline --locked'],cwd=root,stdout=log,stderr=subprocess.STDOUT,check=True)
after=sources();assert after==before, 'Production source changed during build; reject epoch'
dest=Path('/workspace/sarv-evidence/bin')/('sarv-M5-'+epoch)
assert not dest.exists(), 'Never overwrite immutable executable'
shutil.copy2(root/'target/release/sarv-quote',dest)
result={'milestone':'M5 goods sales/receivables/original-cost returns','epoch':epoch,'base_head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip(),'branch':subprocess.check_output(['git','branch','--show-current'],cwd=root,text=True).strip(),'binary':str(dest),'binary_sha256':hashlib.sha256(dest.read_bytes()).hexdigest(),'production_source_sha256':before['sha256'],'source_file_sha256':before['files'],'frozen_source_provenance':before,'build_elapsed_seconds':time.monotonic()-t,'source_stable_across_build':True,'acceptance':'pending; build/provenance only'}
(out/'manifest.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({k:v for k,v in result.items() if k not in ['source_file_sha256','frozen_source_provenance']},indent=2))
