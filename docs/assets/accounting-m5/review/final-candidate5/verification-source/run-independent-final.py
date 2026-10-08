"""Run independent financial probes against a root-issued immutable source manifest.

Call from the repository root after sourcing the execution environment.
Uses fresh synthetic scratch profiles only. No native/performance acceptance.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import time

REVIEW = Path(__file__).resolve().parent
ROOT = REVIEW.parents[3]

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def source():
    digest = hashlib.sha256()
    files = {}
    paths = sorted([*ROOT.glob('src/**/*.rs'), ROOT/'src/style.css', ROOT/'Cargo.toml', ROOT/'Cargo.lock'], key=lambda p:p.relative_to(ROOT).as_posix().encode('utf-8'))
    for path in paths:
        name = path.relative_to(ROOT).as_posix()
        data = path.read_bytes()
        digest.update(name.encode('utf-8')+b'\0'+data+b'\0')
        files[name] = hashlib.sha256(data).hexdigest()
    return {'sha256':digest.hexdigest(),'files':files}

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source-manifest', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    assert not args.out.exists(), 'Use a fresh output directory'
    root_manifest = json.loads(args.source_manifest.read_text())
    expected = root_manifest.get('frozen_source_provenance', root_manifest)
    before = source()
    assert before['files'] == expected['files'] and before['sha256'] == expected['sha256'], 'Source differs from root freeze'
    args.out.mkdir(parents=True)
    manifest = {'phase':'root-frozen independent headless verification','started_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'root_manifest_sha256':sha(args.source_manifest),'source_before':before,'observer_sha256':sha(Path(__file__)),'probe_files_sha256':{p.relative_to(REVIEW).as_posix():sha(p) for p in sorted((REVIEW/'probe').rglob('*')) if p.is_file()},'independent_fixture_sha256':{p.relative_to(REVIEW).as_posix():sha(p) for p in [REVIEW/'literal-expected.json',REVIEW/'check-literal-fixture.py',REVIEW/'raw-literal-check.py']},'commands':[]}
    root_binary = Path(root_manifest['binary'])
    manifest['root_binary'] = str(root_binary)
    manifest['root_binary_sha256'] = sha(root_binary)
    assert manifest['root_binary_sha256'] == root_manifest['binary_sha256'], 'Root binary differs from freeze'
    # Preserve exact reviewer bytes before running; never reconstruct an old epoch.
    capture = args.out/'verification-source'
    for path in [Path(__file__), *[REVIEW/p for p in manifest['probe_files_sha256']], *[REVIEW/p for p in manifest['independent_fixture_sha256']]]:
        target = capture/path.relative_to(REVIEW)
        target.parent.mkdir(parents=True,exist_ok=True)
        shutil.copyfile(path,target)
    shutil.copyfile(args.source_manifest,args.out/'root-freeze-manifest.json')
    environment = dict(os.environ)
    environment['CARGO_TARGET_DIR'] = str(ROOT/'target')
    environment['SARV_REVIEW_M4_INPUT_ROOT'] = str(ROOT/'artifacts/independent-m5/actual-m4-legacy')
    environment['SARV_REVIEW_M4_OLD_BINARY'] = str(ROOT/'target/debug/sarv-independent-m4-sales-namespace-fixture')
    manifest['old_m4_binary_sha256'] = sha(Path(environment['SARV_REVIEW_M4_OLD_BINARY']))
    manifest['old_m4_backup_sha256'] = {p.relative_to(ROOT).as_posix():sha(p) for p in sorted((ROOT/'artifacts/independent-m5').glob('actual-m4-legacy*/*.json'))}
    def observer():
        observed = {'runner_sha256':sha(Path(__file__)),'root_manifest_sha256':sha(args.source_manifest),'root_binary_sha256':sha(root_binary),'probe_files_sha256':{p.relative_to(REVIEW).as_posix():sha(p) for p in sorted((REVIEW/'probe').rglob('*')) if p.is_file()},'independent_fixture_sha256':{p.relative_to(REVIEW).as_posix():sha(p) for p in [REVIEW/'literal-expected.json',REVIEW/'check-literal-fixture.py',REVIEW/'raw-literal-check.py']},'old_m4_binary_sha256':sha(Path(environment['SARV_REVIEW_M4_OLD_BINARY'])),'old_m4_backup_sha256':{p.relative_to(ROOT).as_posix():sha(p) for p in sorted((ROOT/'artifacts/independent-m5').glob('actual-m4-legacy*/*.json'))}}
        assert observed['runner_sha256'] == manifest['observer_sha256'], 'Observer changed'
        for key in observed.keys()-{'runner_sha256'}:
            assert observed[key] == manifest[key], key+' changed'
        return observed
    def run(label, command):
        observers_before = observer()
        current=source()
        assert current==before, 'Source changed before '+label
        with (args.out/(label+'.log')).open('wb') as log:
            status=subprocess.run(command,cwd=ROOT,env=environment,stdout=log,stderr=subprocess.STDOUT).returncode
        entry={'label':label,'argv':list(map(str,command)),'returncode':status,'log_sha256':sha(args.out/(label+'.log')),'source_before':current,'observer_before':observers_before}
        if '--bin' in command:
            binary=ROOT/'target/debug'/command[command.index('--bin')+1]
            if binary.exists():
                entry['binary_sha256']=sha(binary)
        manifest['commands'].append(entry)
        entry['observer_after'] = observer()
        entry['source_after'] = source()
        (args.out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
        assert status==0, label+' failed; diagnostic evidence retained'
        assert entry['source_after']==before, 'Source changed during '+label
    cargo=['cargo','run','--offline','--manifest-path',str(REVIEW/'probe/Cargo.toml')]
    run('interleaved',cargo+['--bin','sarv-independent-m5-probe','--',str(args.out/'interleaved')])
    state=args.out/'interleaved/state.json'
    run('raw-literal-check',['python3',str(REVIEW/'raw-literal-check.py'),str(state),str(args.out/'raw-literal-check.json')])
    run('guards',cargo+['--bin','sarv-independent-m5-guards','--',str(state),str(args.out/'guards')])
    run('quote-draft',cargo+['--bin','sarv-independent-m5-quote-draft','--',str(state),str(args.out/'quote-draft')])
    run('residuals',cargo+['--bin','sarv-independent-m5-residuals','--',str(args.out/'residuals')])
    run('financial-store-faults',cargo+['--bin','sarv-independent-m5-store-faults','--',str(state),str(args.out/'financial-store-faults')])
    # Migration probe expects actual old-M4 runtime inputs beside its private package.
    # These originate from the separately archived/tagged old binary, never regenerated by M5.
    run('migration',cargo+['--bin','sarv-independent-m5-migration','--',str(args.out/'migration')])
    manifest['source_after']=source()
    manifest['passed']=True
    manifest['ended_utc']=time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime())
    (args.out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
    print('PASS independent frozen headless fixture/controls/identity/quote/draft/residual/actual-M4 migration checks. Native/performance acceptance remains separate.')

if __name__=='__main__':
    main()
