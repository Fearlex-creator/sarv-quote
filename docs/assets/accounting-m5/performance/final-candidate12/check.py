from pathlib import Path
import hashlib,json,re
d=Path(__file__).resolve().parent;r=d.parents[4]
a=json.loads((d/'analysis.json').read_text());j=json.loads((d/'performance.json').read_text());p=json.loads((d/'source-provenance.json').read_text());q=json.loads((d/'native-scale-verification.json').read_text());inventory=json.loads((d/'evidence-files.json').read_text())['files'];checks=[]
for n,v in inventory.items():
 b=(d/n).read_bytes();assert len(b)==v['bytes'] and hashlib.sha256(b).hexdigest()==v['sha256']
checks.append(f"{len(inventory)} portable files exact bytes/SHA256")
actual={x.relative_to(d).as_posix() for x in d.rglob('*') if x.is_file() and x.name!='evidence-files.json'};assert actual==set(inventory)
assert not any(x.is_symlink() or x.suffix in ('.sqlite','.db') for x in d.rglob('*'))
checks.append('Portable inventory complete; no symlinks/SQLite/binary files')
assert j['result']=='measured' and not j['development_pilot'];assert q['result']=='passed' and len(q['checks'])==15
assert p['all_snapshot_guards_stable'] and len(p['runs'])==2 and all(x['driver_return_code']==0 for x in p['runs'])
assert p['before']==p['after'] and len(p['before']['files'])==36 and len(p['before']['verification_file_sha256'])==53
for run in p['runs']:assert run['before']==run['after']==p['before']
assert hashlib.sha256((d/'sequential-provenance-driver.py').read_bytes()).hexdigest()==p['before']['wrapper_sha256']
assert a['protocols']=={'external':'m5-cached-v3','compiled':'m5-action-v1.2'}
checks.append('One sequential n10/n20 epoch + unchanged M2; exact36/53/binary/wrapper guards')
assert len(a['startup'])==6 and all(v['startup']['n']==10 for v in a['startup'].values());assert all(v['visible']['n']==v['focused']['n']==20 for v in a['search'].values());assert a['long_navigation']['visible']['n']==a['long_navigation']['focused']['n']==39
assert len(q['additive_action_phases'])==22;assert a['additive_miss_count']==sum(v['strict_status']=='miss' for v in q['additive_action_phases'])
checks.append('Exact complete sample counts,22 scopes and recomputed strict misses')
for f in ('README.md','analyze.py'):
 assert 'candidate9-source-provenance' not in (d/f).read_text()
for target in re.findall(r'\]\(([^)]+)\)',(d/'README.md').read_text()):
 if '://' not in target:assert (d/target).exists(),target
checks.append('All report links resolve; raw historical C8 evidence remains separate')
report={'result':'passed','checks':checks,'limits':'Offline evidence/schema/hash review; does not infer unmeasured acceptance or erase performance misses.'}
print(json.dumps(report))
