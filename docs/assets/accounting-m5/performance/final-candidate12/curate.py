from pathlib import Path
import shutil
root=Path('/workspace/sarv-quote');target=root/'docs/assets/accounting-m5/performance/final-candidate12';target.mkdir(exist_ok=False)
for src,name in [(root/'artifacts/performance-m5/final-candidate12','repeated'),(root/'artifacts/performance-m5/final-candidate12-scale','scale')]:
 for p in src.rglob('*'):
  if not p.is_file() or p.is_symlink():continue
  rel=p.relative_to(src)
  if any(x in ('runtime','xdg-data') for x in rel.parts):continue
  if p.suffix not in ('.json','.jsonl','.png','.log','.csv','.txt'):continue
  if p.suffix=='.csv' and p.stat().st_size>1_000_000:continue
  q=target/name/rel;q.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,q)
for src,out in [('artifacts/performance-m5/final-candidate12/performance.json','performance.json'),('artifacts/performance-m5/final-candidate12-scale/performance.json','scale-performance.json'),('artifacts/performance-m5/final-candidate12-scale/synthetic-profile/native-responsiveness-verification.json','native-scale-verification.json'),('artifacts/performance-m5/final-candidate12-source-provenance.json','source-provenance.json'),('docs/assets/accounting-m5/checks/candidate12/manifest.json','root-freeze-manifest.json'),('docs/assets/accounting-m5/checks/candidate12/verification-source-before.json','qualified-verification-before.json')]:
 shutil.copy2(root/src,target/out)
for src,name in [('/tmp/sarv-m5-final-candidate12.log','repeated-observer.log'),('/tmp/sarv-m5-final-candidate12-scale.log','scale-observer.log'),('/tmp/run-final-candidate12.py','sequential-provenance-driver.py'),('/tmp/curate-final-candidate12.py','curate.py')]:shutil.copy2(src,target/name)
print(target)
