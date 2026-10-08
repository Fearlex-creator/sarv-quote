#!/usr/bin/env python3
"""Read-only independent review suite pinned to complete production source digest."""
import hashlib,json,os,sqlite3,subprocess,sys
from pathlib import Path
root=Path('/workspace/sarv-quote');out=Path(sys.argv[1]).resolve();expected=sys.argv[2];out.mkdir(parents=True,exist_ok=False)
def source_hash():
 h=hashlib.sha256()
 for p in sorted([*root.glob('src/**/*.rs'),root/'src/style.css',root/'Cargo.toml',root/'Cargo.lock'],key=lambda p:str(p.relative_to(root))):
  h.update(str(p.relative_to(root)).encode()+b'\0'+p.read_bytes()+b'\0')
 return h.hexdigest()
def check_source():assert source_hash()==expected,(source_hash(),expected)
check_source();env=os.environ.copy();env['CARGO_TARGET_DIR']=str(root/'target')
def run(name,cmd):
 check_source()
 with (out/(name+'.log')).open('w') as log:subprocess.run([str(x) for x in cmd],cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
 check_source();print('PASS frozen '+name,flush=True)
manifest=root/'artifacts/independent-m4/probe/Cargo.toml';run('build',['cargo','build','--offline','--manifest-path',manifest,'--bins'])
for short in ['probe','atomic','integration','migration','dates','manual-revision']:
 name='sarv-independent-m4-'+short
 run(short,[root/'target/debug'/name,out/short])
with sqlite3.connect(f'file:{out}/probe/profile/sarv.sqlite3?mode=ro',uri=True) as db:payload=db.execute('select payload from state where id=1').fetchone()[0]
(out/'probe/state.json').write_text(payload)
run('raw-state',['/usr/bin/python3',root/'artifacts/independent-m4/raw-state-check.py',out/'probe/profile',out/'raw-state.json'])
run('export',[root/'target/debug/sarv-independent-m4-export',out/'probe/state.json',out/'export'])
run('raw-csv',['/usr/bin/python3',root/'artifacts/independent-m4/raw-csv-check.py',out/'export',out/'probe/state.json',out/'raw-csv.json'])
run('pdf-bounds',['/usr/bin/python3',root/'artifacts/independent-m4/pdf-bounds.py',out/'export',out/'pdf-bounds.json'])
run('old-m3-legacy-fixture',['cargo','run','--offline','--manifest-path',root/'artifacts/independent-m4/compat-probe/Cargo.toml','--bin','sarv-independent-m3-legacy-event','--',out/'legacy-m3'])
run('legacy-prefix-fixed',[root/'target/debug/sarv-independent-m4-legacy-event',out/'legacy-m3',out/'legacy-prefix-fixed','fixed'])
run('old-m3-recovery',['cargo','run','--offline','--manifest-path',root/'artifacts/independent-m4/compat-probe/Cargo.toml','--bin','sarv-independent-m3-recovery-compat','--',out/'migration',out/'old-m3-recovery'])
check_source();report={'result':'passed','production_source_sha256':expected,'scope':'Independent scratch suite compiled against frozen production; raw integer state/report reconstruction; actual processCAS/SIGKILL; prior M3 library original recovery verification. Native release checked separately.','checks':['probe43','atomic5','integration4','migration3','dates','manual-revision-boundary','raw-state','export','raw-csv','pdf-bounds9/14','old-m3-legacy-fixture','legacy-prefix-fixed2','old-m3-recovery3'],'output':str(out)};(out/'independent-frozen.json').write_text(json.dumps(report,indent=2));print(json.dumps(report),flush=True)
