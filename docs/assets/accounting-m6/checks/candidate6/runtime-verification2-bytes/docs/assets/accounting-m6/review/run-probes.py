#!/usr/bin/env python3
"""Run prebuilt independent probes; root grants build/benchmark coordination first."""
import argparse,hashlib,json,os,subprocess,time
from pathlib import Path
parser=argparse.ArgumentParser()
parser.add_argument('out',type=Path)
parser.add_argument('--bin-dir',type=Path,required=True)
parser.add_argument('--old-helper',type=Path,required=True)
parser.add_argument('--m5-state',type=Path,required=True)
a=parser.parse_args();a.out.mkdir(parents=True,exist_ok=False)
repo=Path(__file__).resolve().parents[4]
review=Path(__file__).resolve().parent
source=[repo/'Cargo.toml',repo/'Cargo.lock']+sorted((repo/'src').rglob('*.rs'))
observer=sorted(p for p in (review/'probe').rglob('*') if p.is_file())+[review/'literal-expected.json',Path(__file__).resolve()]
binaries=[a.bin_dir/name for name in ['sarv-independent-m6-probe','sarv-independent-m6-guards','sarv-independent-m6-store-faults','sarv-independent-m6-migration']]+[a.old_helper]
def hashes(paths):return {str(p.resolve()):hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}
before={'production':hashes(source),'observer':hashes(observer),'executables':hashes(binaries),'m5_input':hashes([a.m5_state])}
manifest={'before':before,'started_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'runs':[],'status':'RUNNING'}
(a.out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
env=os.environ.copy();env['SARV_REVIEW_M5_OLD_BINARY']=str(a.old_helper.resolve());env['SARV_REVIEW_M5_INPUT_STATE']=str(a.m5_state.resolve())
commands=[['sarv-independent-m6-probe',a.out/'literal'],['sarv-independent-m6-guards',a.out/'guards'],['sarv-independent-m6-store-faults',a.out/'literal/baseline/state.json',a.out/'faults'],['sarv-independent-m6-migration',a.out/'migration']]
for command in commands:
    cmd=[str((a.bin_dir/command[0]).resolve())]+[str(p.resolve()) for p in command[1:]]
    start=time.monotonic()
    with (a.out/(command[0]+'.log')).open('wb') as log:
        r=subprocess.run(cmd,env=env,stdout=log,stderr=subprocess.STDOUT)
    manifest['runs'].append({'command':cmd,'returncode':r.returncode,'elapsed_s':time.monotonic()-start})
    manifest['after_current']={'production':hashes(source),'observer':hashes(observer),'executables':hashes(binaries),'m5_input':hashes([a.m5_state])}
    stable=manifest['after_current']==before
    manifest['status']='RUNNING' if r.returncode==0 and stable else 'FAILED_OR_SOURCE_CHANGED'
    (a.out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
    if r.returncode!=0 or not stable:raise SystemExit(f'{command[0]} return{r.returncode}; source stable={stable}; diagnostic retained')
manifest['status']='PASS';manifest['finished_utc']=time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime())
(a.out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
print('PASS independent literal/guard/fault/migration probes; source/binary/input hashes unchanged')
