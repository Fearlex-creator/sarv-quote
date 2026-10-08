from pathlib import Path
import sys,json,sqlite3,hashlib,shutil
root=Path('/workspace/sarv-quote');epoch=sys.argv[1];source=Path(sys.argv[2]).resolve();out=root/'docs/assets/accounting-m5/checks'/epoch/'quote-callback';assert not out.exists();out.mkdir()
for name in ['before.json','after.json','manifest.json','application.log','xvfb.log','openbox.log','profile/quote-callback-verification.json']:
 dst=out/name;dst.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(source/name,dst)
with sqlite3.connect(f'file:{source}/profile/sarv.sqlite3?mode=ro',uri=True) as origin:
 before=origin.execute('select generation,payload from state where id=1').fetchone()
 with sqlite3.connect(out/'profile/sarv.sqlite3') as dst:
  origin.backup(dst);dst.execute('pragma wal_checkpoint(TRUNCATE)');assert dst.execute('pragma journal_mode=DELETE').fetchone()[0]=='delete';assert dst.execute('pragma integrity_check').fetchone()[0]=='ok';assert dst.execute('select generation,payload from state where id=1').fetchone()==before
 assert origin.execute('select generation,payload from state where id=1').fetchone()==before
(out/'profile/final-state.json').write_text(json.dumps({'synthetic_only':True,'generation':before[0],'state':json.loads(before[1])},ensure_ascii=False,indent=2)+'\n')
report=json.loads((out/'profile/quote-callback-verification.json').read_text());assert report['result']=='passed'
files={p.relative_to(out).as_posix():hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(out.rglob('*')) if p.is_file()}
(out/'delivery-manifest.json').write_text(json.dumps({'files':files,'sqlite_integrity':'ok','generation':before[0],'sqlite_export_method':'SQLite online backup including WAL from original stopped synthetic profile; original before/destination/after generation and payload exact; delivery copy consolidated to DELETE journal mode with unchanged logical state','checks':report['checks'],'performance_scope':'diagnostic qualification, not quiet final performance measurement','accounting_scope':'synthetic quotation only, no ledger effects'},indent=2)+'\n')
print(json.dumps({'result':'passed','files':len(files),'checks':len(report['checks']),'output':str(out)}))
