#!/usr/bin/env python3
"""One guarded disposable native full-smoke focus diagnosis; not release QA."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[4]
spec = importlib.util.spec_from_file_location('native_qa', ROOT/'scripts/qa-native.py')
native = importlib.util.module_from_spec(spec)
spec.loader.exec_module(native)


def state(path):
    with sqlite3.connect(f'file:{path}/sarv.sqlite3?mode=ro', uri=True) as database:
        return json.loads(database.execute('SELECT payload FROM state WHERE id=1').fetchone()[0])


def fingerprint(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, ensure_ascii=False,
                                     separators=(',', ':')).encode()).hexdigest()


def main():
    output = ROOT/'artifacts/sales-native'
    output.mkdir(exist_ok=True, parents=True)
    run = Path(tempfile.mkdtemp(prefix='focus-boundary-', dir=output))
    profile = run/'full'
    profile.mkdir()
    manifest_path = Path(os.environ['SARV_SALES_SOURCE_MANIFEST']).resolve()
    manifest = json.loads(manifest_path.read_text())
    source_binary = Path(os.environ['SARV_QA_BINARY']).resolve()
    binary = run/'sarv-quote'
    shutil.copy2(source_binary, binary)
    seed = Path(os.environ['SARV_SALES_SEED_PROFILE']).resolve()
    observer = Path(__file__).resolve()
    report = {'result': 'failed', 'run': str(run), 'scope': 'disposable native full-smoke focus diagnosis only; no external keyboard/release qualification',
              'source_manifest': str(manifest_path), 'binary_source': str(source_binary), 'seed_source': str(seed),
              'observer_sha256': hashlib.sha256(observer.read_bytes()).hexdigest(), 'checks': []}
    processes = []
    logs = []

    def check(name, passed):
        report['checks'].append({'name': name, 'passed': bool(passed)})
        if not passed:
            raise AssertionError(name)

    def frozen():
        hashes = {}
        aggregate = hashlib.sha256()
        for name in sorted(manifest['frozen_source_provenance']['files'], key=lambda value: value.encode()):
            raw = (ROOT/name).read_bytes()
            hashes[name] = hashlib.sha256(raw).hexdigest()
            aggregate.update(name.encode()+b'\0'+raw+b'\0')
        return {'files': hashes, 'sha256': aggregate.hexdigest()}

    def start(command, name, env=None, **kwargs):
        log = (run/f'{name}.log').open('w')
        logs.append(log)
        process = subprocess.Popen(command, env=env, stdout=log, stderr=log,
                                   start_new_session=True, **kwargs)
        processes.append(process)
        return process

    try:
        report['source_before'] = frozen()
        check('frozen production source exact before diagnosis', report['source_before']['files'] == manifest['frozen_source_provenance']['files'] and report['source_before']['sha256'] == manifest['frozen_source_provenance']['sha256'])
        report['binary_sha256'] = hashlib.sha256(binary.read_bytes()).hexdigest()
        check('frozen binary exact before diagnosis', report['binary_sha256'] == manifest['binary_sha256'])
        shutil.copy2(manifest_path, run/'frozen-source-manifest.json')
        shutil.copy2(observer, run/'observer.py')
        report['seed_database_sha256_before'] = hashlib.sha256((seed/'sarv.sqlite3').read_bytes()).hexdigest()
        before = state(seed)
        report['seed_state_sha256_before'] = fingerprint(before)
        check('authentic synthetic M4 seed without sales', len(before['accounting']['journals']) == 5
              and not before['accounting'].get('sales', {}).get('records')
              and any(company['name'] == 'F2-core-v1 • شرکت ساختگی' for company in before['accounting']['companies']))
        with sqlite3.connect(f'file:{seed}/sarv.sqlite3?mode=ro', uri=True) as source, sqlite3.connect(profile/'sarv.sqlite3') as target:
            source.backup(target)
        (profile/'M5-synthetic-prices.csv').write_text('code,name,measure,price,currency,source_at,aliases\nSKU-A,کالای الف فروش,عدد,20,rial,2026-03-22T09:00:00Z,نمونه فروش\n')
        with tempfile.TemporaryFile() as display_file:
            start(['Xvfb', '-displayfd', str(display_file.fileno()), '-screen', '0', '1450x1050x24', '-nolisten', 'tcp',
                   '-xkbdir', str(native.ENVROOT/'sysroot/usr/share/X11/xkb')], 'xvfb', pass_fds=(display_file.fileno(),))
            def read_display():
                display_file.seek(0)
                return display_file.read().decode().strip()
            display = ':'+native.wait_for(read_display, label='owned diagnostic X11')
        runtime = native.private_accessibility_environment(display, run)
        env = os.environ.copy()
        for key in ['SARV_UI_SMOKE', 'SARV_ACCOUNTING_SMOKE', 'SARV_CORE_SMOKE', 'SARV_PROCUREMENT_SMOKE', 'SARV_SALES_PERF']:
            env.pop(key, None)
        env.update(DISPLAY=display, GDK_BACKEND='x11', GSK_RENDERER='cairo', XDG_RUNTIME_DIR=str(runtime),
                   SARV_TEST_INSTANCE='sales-focus-boundary-qa', SARV_DATA_DIR=str(profile),
                   SARV_SALES_SMOKE='full', SARV_SALES_CAPTURE='1', SARV_SALES_FOCUS_DIAGNOSTIC='1',
                   ADW_DEBUG_COLOR_SCHEME='prefer-light')
        env['G_DEBUG'] = ','.join(filter(None, [env.get('G_DEBUG', ''), 'fatal-criticals']))
        start(['openbox', '--sm-disable', '--config-file', str(native.ENVROOT/'sysroot/etc/xdg/openbox/rc.xml')], 'openbox', env)
        app = start([str(binary)], 'M5-full', env)
        verification = profile/'sales-native-verification.json'
        failure = profile/'sales-native-failure.txt'
        native.wait_for(lambda: verification.exists() or failure.exists() or app.poll() is not None,
                        seconds=200, label='native diagnostic full-smoke completion')
        check('native diagnostic full-smoke reaches internal manifest', verification.exists() and not failure.exists())
        report['native_manifest'] = json.loads(verification.read_text())
        native.stop(app)
        for log in logs:
            log.flush()
        report['diagnostics'] = [line for line in (run/'M5-full.log').read_text(errors='replace').splitlines()
                                 if 'WARNING **' in line or 'CRITICAL **' in line]
        report['source_after'] = frozen()
        check('frozen production source unchanged after diagnosis', report['source_after'] == report['source_before'])
        check('frozen binary unchanged after diagnosis', hashlib.sha256(binary.read_bytes()).hexdigest() == report['binary_sha256'])
        report['seed_database_sha256_after'] = hashlib.sha256((seed/'sarv.sqlite3').read_bytes()).hexdigest()
        check('protected seed database bytes unchanged after diagnosis', report['seed_database_sha256_after'] == report['seed_database_sha256_before'])
        report['seed_state_sha256_after'] = fingerprint(state(seed))
        check('protected seed unchanged after diagnosis', report['seed_state_sha256_after'] == report['seed_state_sha256_before'])
        check('observer unchanged after diagnosis', hashlib.sha256(observer.read_bytes()).hexdigest() == report['observer_sha256'])
        report['result'] = 'diagnosis-complete'
        print(json.dumps(report, ensure_ascii=False, indent=2))
    except Exception as error:
        report['error'] = str(error)
        print(json.dumps(report, ensure_ascii=False, indent=2))
        raise
    finally:
        (run/'focus-boundary-diagnostic.json').write_text(json.dumps(report, ensure_ascii=False, indent=2))
        for process in reversed(processes):
            if process.poll() is None:
                native.stop(process)
        for log in logs:
            log.close()


if __name__ == '__main__':
    main()
