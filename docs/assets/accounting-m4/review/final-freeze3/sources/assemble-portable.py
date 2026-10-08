#!/usr/bin/env python3
"""Copy the approved bounded reviewer evidence; never databases or executables.

Arguments: final-headless-output final-native-run headless-log native-log.
Final acceptance text/aggregate is written only after actual visual review.
"""
import hashlib,json,shutil,sys
from pathlib import Path
ROOT=Path('/workspace/sarv-quote');scratch=ROOT/'artifacts/independent-m4'
args=sys.argv[1:];assert len(args) in (4,5)
headless,native,hlog,nlog=map(Path,args[:4]);out=Path(args[4]) if len(args)==5 else ROOT/'docs/assets/accounting-m4/review'
assert not out.exists(), 'Keep existing portable outputs immutable; choose reviewed replacement explicitly.'
out.mkdir(parents=True)
def copy(p,relative):
 assert p.is_file() and p.suffix in {'.rs','.toml','.lock','.py','.json','.log','.md','.csv','.pdf','.png'}
 target=out/relative;target.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(p,target)
for project in ['probe','compat-probe']:
 for p in sorted((scratch/project).rglob('*')):
  if p.is_file() and (p.parent.name=='src' or p.name in ['Cargo.toml','Cargo.lock']):copy(p,Path('sources')/p.relative_to(scratch))
for name in ['literal-expected.json','frozen-run.py','native-review.py','fixture-from-backup.py','native-persisted-check.py','raw-state-check.py','raw-csv-check.py','pdf-bounds.py','assemble-portable.py']:
 copy(scratch/name,Path('sources')/name)
copy(scratch/'PORTABLE.md','README.md')
copy(hlog,'headless/runner.log');copy(nlog,'native/runner.log')
for p in sorted(headless.rglob('*')):
 if not p.is_file():continue
 rel=p.relative_to(headless)
 if p.suffix=='.log' and p.parent==headless:copy(p,Path('headless')/rel)
 elif p.suffix in {'.csv','.pdf'} and rel.parts[0] in ['export','dates']:copy(p,Path('headless')/rel)
 elif p.suffix=='.png' and rel.parts[:2]==('export','render'):copy(p,Path('headless')/rel)
 elif p.suffix=='.json' and (p.parent==headless or p.name.startswith('independent-') or p.name in ['accounting.json','state.json','backup.json','rounding-backup.json','free-zero-backup.json','tampered-recomputed-envelope.json','current-v5.json','m3-rerestored-v4.json'] or p.name.startswith('before-schema5-') or rel.parts[0]=='legacy-m3'):
  copy(p,Path('headless')/rel)
for p in sorted(native.glob('*')):
 if p.is_file() and p.suffix in {'.json','.png','.log'}:copy(p,Path('native')/p.name)
copy(scratch/'native-full-backup.json','native/native-full-backup.json')
old=list((ROOT/'artifacts/procurement-native/run-fpyy1yz3/full').glob('before-schema5-*.json'));assert len(old)==1
copy(old[0],'native/native-original-v4-recovery.json')
for p in ['native-persisted-final.json']:
 if (scratch/p).exists():copy(scratch/p,Path('native')/p)
for name in ['legacy-prefix-current-run-1.log','legacy-prefix-old-run-1.log']:
 copy(scratch/name,Path('before-fixes')/name)
copy(scratch/'legacy-prefix-current-1/independent-legacy-event.json','before-fixes/legacy-prefix-before-fix.json')
for name in ['independent-native.json','failure-tree.json','failure.png','stage.log']:
 copy(scratch/'native/run-1ek79fw1'/name,Path('before-fixes/native-freeze1')/name)
for name in ['bill-b-page1.png','control-final-page1.png']:
 copy(scratch/'export-1'/name,Path('before-fixes/export-initial')/name)
copy(scratch/'native-freeze1-run.log','before-fixes/native-freeze1/runner.log')
files=[{'path':str(p.relative_to(out)),'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in sorted(out.rglob('*')) if p.is_file()]
(out/'selected-file-manifest.json').write_text(json.dumps({'scope':'Selected independent portable reviewer evidence; final aggregate is separate. No binary, runtime cache, SQLite or temporary profile is copied.','files':files},indent=2))
print(json.dumps({'result':'copied','directory':str(out),'files':len(files),'bytes':sum(p['bytes'] for p in files)},indent=2))
