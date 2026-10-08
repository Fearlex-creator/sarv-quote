#!/usr/bin/env python3
"""Disposable G1 native workflow, then independent XTest keyboard/restart checks.

Source /workspace/sarv-environment/activate.sh then run:
  dbus-run-session -- /usr/bin/python3 scripts/accounting-qa.py
All databases, screenshots and PDFs stay in a new artifacts/accounting-native run.
"""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import signal
import sqlite3
import subprocess
import tempfile
import time
from qa_access import Access

spec = importlib.util.spec_from_file_location('native_qa', Path(__file__).with_name('qa-native.py'))
native = importlib.util.module_from_spec(spec)
spec.loader.exec_module(native)
ROOT, ENVROOT = native.ROOT, native.ENVROOT


def state(profile):
    with sqlite3.connect(f'file:{profile}/sarv.sqlite3?mode=ro', uri=True) as db:
        return json.loads(db.execute('SELECT payload FROM state WHERE id=1').fetchone()[0])


def main():
    output = ROOT / 'artifacts' / 'accounting-native'
    output.mkdir(parents=True, exist_ok=True)
    profile = Path(tempfile.mkdtemp(prefix='run-', dir=output))
    binary = ROOT / 'target/release/sarv-quote'
    report = {'result': 'failed', 'profile': str(profile), 'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
              'backend': 'private X11 Xvfb/Openbox GTK4; signal financial workflow and actual XTest navigation separately', 'checks': []}
    processes, logs = [], []
    x11 = None
    def start(command, name, env=None, **kwargs):
        log = (profile / f'{name}.log').open('w'); logs.append(log)
        proc = subprocess.Popen(command, env=env, stdout=log, stderr=log, start_new_session=True, **kwargs)
        processes.append(proc); return proc
    def check(name, condition, **details):
        report['checks'].append({'name': name, 'passed': bool(condition), **details})
        if not condition: raise AssertionError(name)
    try:
        with tempfile.TemporaryFile() as fd:
            start(['Xvfb', '-displayfd', str(fd.fileno()), '-screen', '0', '1450x1050x24', '-nolisten', 'tcp', '-xkbdir', str(ENVROOT / 'sysroot/usr/share/X11/xkb')], 'xvfb', pass_fds=(fd.fileno(),))
            def display_name():
                fd.seek(0); return fd.read().decode().strip()
            display = ':' + native.wait_for(display_name, label='private display')
        runtime = native.private_accessibility_environment(display, profile)
        env = os.environ.copy()
        env.update(DISPLAY=display, GDK_BACKEND='x11', GSK_RENDERER='cairo', XDG_RUNTIME_DIR=str(runtime),
                   SARV_DATA_DIR=str(profile), SARV_TEST_INSTANCE='accounting-qa', SARV_ACCOUNTING_SMOKE='full')
        env.pop('SARV_UI_SMOKE', None)
        start(['openbox', '--sm-disable', '--config-file', str(ENVROOT / 'sysroot/etc/xdg/openbox/rc.xml')], 'openbox', env)
        app = start([str(binary)], 'financial-native', env)
        app.wait(timeout=110)
        failure = profile / 'accounting-native-failure.txt'
        check('native financial form workflow passed', not failure.exists() and (profile / 'accounting-native-verification.json').exists(), failure=failure.read_text() if failure.exists() else None)
        check('native workflow normal exit', app.returncode == 0)
        check('fresh-profile restore and both database restarts match', json.loads((profile / 'accounting-recovery-verification.json').read_text())['result'] == 'passed')
        baseline = state(profile)
        ledger = baseline['accounting']
        check('independent counts after financial workflow', len(ledger['companies']) == 1 and len(ledger['accounts']) == 2 and len(ledger['journals']) == 3)
        for filename in ['trial-before-reversal-rial.pdf', 'trial-final-toman.pdf']:
            check(f'{filename} exists', (profile / filename).stat().st_size > 500)
            subprocess.run(['pdftoppm', '-f', '1', '-singlefile', '-scale-to', '1200', '-png', str(profile / filename), str(profile / filename.removesuffix('.pdf'))], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        env.pop('SARV_ACCOUNTING_SMOKE', None)
        env.update(SARV_UI_SMOKE='capture', SARV_CAPTURE_PAGE='accounting', SARV_CAPTURE_NAME='keyboard-initial.png')
        app = start([str(binary)], 'keyboard-native', env)
        x11 = native.X11(display)
        window = native.wait_for(lambda: x11.find(app.pid), label='owned accounting window')
        native.wait_for(lambda: (profile / 'keyboard-initial.png').exists(), label='accounting restart rendering')
        x11.focus(window)
        count = len(native.events(profile)); x11.key('7', 'Alt_L')
        seen = native.wait_for(lambda: native.events(profile)[count:] if len(native.events(profile)) > count else None, label='Alt7 received')
        check('real Alt7 opens accounting', seen[-1]['page'] == 'accounting')
        access = Access(app.pid)
        seen_names = set()
        for _ in range(42):
            x11.key('Tab'); time.sleep(.04)
            for row in access.tree():
                if access.focused(row): seen_names.add(row['name'])
        check('real Tab reaches company action', 'شرکت تازه' in seen_names, focused_names=sorted(seen_names))
        journals = native.wait_for(lambda: access.find('اسناد'), label='native journal tab')
        for _ in range(64):
            if any(row['name'] == 'اسناد' and access.focused(row) for row in access.tree()): break
            x11.key('Tab'); time.sleep(.04)
        check('real Tab reaches journal tab', any(row['name'] == 'اسناد' and access.focused(row) for row in access.tree()))
        x11.key('space'); time.sleep(.2)
        create = native.wait_for(lambda: access.find('سند دستی / افتتاحیهٔ دفتر کل'), label='native journal action')
        for _ in range(80):
            if any(row['name'] == 'سند دستی / افتتاحیهٔ دفتر کل' and access.focused(row) for row in access.tree()): break
            x11.key('Tab'); time.sleep(.04)
        check('real Tab reaches journal action', any(row['name'] == 'سند دستی / افتتاحیهٔ دفتر کل' and access.focused(row) for row in access.tree()))
        x11.key('Return'); time.sleep(.35)
        tree = access.tree()
        (profile / 'keyboard-dialog-tree.json').write_text(json.dumps([{k: v for k, v in row.items() if k != 'node'} for row in tree], ensure_ascii=False, indent=2))
        subprocess.run(['/usr/bin/import', '-display', display, '-window', str(window), str(profile / 'keyboard-journal-dialog.png')], check=True)
        check('real Enter opens journal dialog', any(row['name'] == 'ثبت سند متوازن' for row in tree))
        time.sleep(.4)
        # Escape must preserve ledger and dismiss staged unsaved journal.
        x11.key('Escape'); time.sleep(.35)
        check('real Escape closes unsaved journal', not any(row['name'] == 'ثبت سند متوازن' for row in access.tree()))
        check('Escape preserves complete saved state', state(profile) == baseline)
        for width, height in [(1280, 900), (650, 760)]:
            x11.resize(window, width, height)
            native.wait_for(lambda: x11.geometry(window) == [width, height], label=f'{width}x{height} accounting allocation')
            time.sleep(.4)
            subprocess.run(['/usr/bin/import', '-display', display, '-window', str(window), str(profile / f'keyboard-accounting-{width}x{height}.png')], check=True)
            check(f'accounting accepts {width}x{height}', True)
        check('keyboard and narrow layouts preserve accounting state', state(profile) == baseline)
        report['result'] = 'passed'
    except Exception as error:
        report['error'] = str(error)
        raise
    finally:
        if x11: x11.x.XCloseDisplay(x11.d)
        for proc in reversed(processes): native.stop(proc)
        for log in logs: log.close()
        (profile / 'accounting-independent-native.json').write_text(json.dumps(report, ensure_ascii=False, indent=2))
        print(json.dumps(report, ensure_ascii=False, indent=2))

if __name__ == '__main__': main()
