#!/usr/bin/env python3
"""Native sidebar keyboard, adaptive, appearance and restart evidence.

source /workspace/sarv-environment/activate.sh
 dbus-run-session -- /usr/bin/python3 scripts/native-sidebar-qa.py
Only a fresh synthetic profile and its private X11/DBus session are used.
SARV_QA_BINARY can select a coordinated intermediate binary.
"""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import tempfile
import time
from qa_access import Access

spec = importlib.util.spec_from_file_location('native_qa', Path(__file__).with_name('qa-native.py'))
native = importlib.util.module_from_spec(spec)
spec.loader.exec_module(native)
ROOT, ENVROOT = native.ROOT, native.ENVROOT


def payload(profile):
    with sqlite3.connect(f'file:{profile}/sarv.sqlite3?mode=ro', uri=True) as db:
        return json.loads(db.execute('SELECT payload FROM state WHERE id=1').fetchone()[0])


def main():
    output = ROOT / 'artifacts/sidebar-native'
    output.mkdir(parents=True, exist_ok=True)
    profile = Path(tempfile.mkdtemp(prefix='run-', dir=output))
    binary = Path(os.environ.get('SARV_QA_BINARY', ROOT / 'target/release/sarv-quote'))
    report = {'result': 'failed', 'profile': str(profile), 'binary': str(binary),
              'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
              'backend': 'private X11/Xvfb/Openbox; real XTest keys; AT-SPI observation',
              'synthetic_only': True, 'checks': [], 'limits': ['Wayland and screen reader speech untested', 'High contrast and enlarged text untested']}
    processes, logs = [], []
    x = None
    def start(command, name, env=None, **kwargs):
        log = (profile / (name + '.log')).open('w'); logs.append(log)
        proc = subprocess.Popen(command, env=env, stdout=log, stderr=log, start_new_session=True, **kwargs)
        processes.append(proc)
        return proc
    def check(name, condition, **details):
        report['checks'].append({'name': name, 'passed': bool(condition), **details})
        if not condition:
            raise AssertionError(name)
    try:
        with tempfile.TemporaryFile() as fd:
            start(['Xvfb', '-displayfd', str(fd.fileno()), '-screen', '0', '1500x1100x24', '-nolisten', 'tcp', '-xkbdir', str(ENVROOT / 'sysroot/usr/share/X11/xkb')], 'xvfb', pass_fds=(fd.fileno(),))
            def display_name():
                fd.seek(0); return fd.read().decode().strip()
            display = ':' + native.wait_for(display_name, label='private display')
        runtime = native.private_accessibility_environment(display, profile)
        env = os.environ.copy()
        env.update(DISPLAY=display, GDK_BACKEND='x11', GSK_RENDERER='cairo',
                   XDG_RUNTIME_DIR=str(runtime), SARV_DATA_DIR=str(profile),
                   SARV_TEST_INSTANCE='sidebar-qa', SARV_UI_SMOKE='capture',
                   SARV_CAPTURE_PAGE='accounting', SARV_CAPTURE_NAME='initial.png')
        env.pop('SARV_ACCOUNTING_SMOKE', None)
        env.pop('SARV_RESPONSIVENESS_SMOKE', None)
        start(['openbox', '--sm-disable', '--config-file', str(ENVROOT / 'sysroot/etc/xdg/openbox/rc.xml')], 'openbox', env)
        app = start([str(binary)], 'native-sidebar', env)
        x = native.X11(display)
        window = native.wait_for(lambda: x.find(app.pid), label='owned native window')
        native.wait_for(lambda: (profile / 'initial.png').exists(), label='initial GTK rendering')
        x.focus(window)
        access = Access(app.pid)
        native.wait_for(lambda: access.tree(), label='owned accessibility tree')
        original = payload(profile)
        def key(name, *modifiers):
            x.key(name, *modifiers); time.sleep(.18)
        def page(number, expected):
            before = len(native.events(profile))
            key(str(number), 'Alt_L')
            seen = native.wait_for(lambda: next((event for event in native.events(profile)[before:] if event['page'] == expected), None), label=f'Alt+{number} opens {expected}')
            return seen
        def tab_to(name, limit=90, roles=None):
            for _ in range(limit):
                item = access.find(name, roles or (['list item'] if name == 'دفتر حسابداری' else None))
                if item and access.focused(item):
                    return item
                x.key('Tab'); time.sleep(.035)
            raise AssertionError(f'Tab never reached {name}')
        def capture(name):
            time.sleep(.3)
            subprocess.run(['/usr/bin/import', '-display', display, '-window', str(window), str(profile / name)], check=True)
        def resize(width, height):
            x.resize(window, width, height)
            native.wait_for(lambda: x.geometry(window) == [width, height], label='native resize')
            time.sleep(.35)
        resize(1280, 900)
        for number, name in [(1, 'order'), (2, 'catalog'), (3, 'customers'), (4, 'quotes'), (5, 'backup'), (6, 'settings'), (7, 'accounting')]:
            check(f'Alt+{number} opens {name}', page(number, name)['page'] == name)
        check('Navigation preserves entire saved payload', payload(profile) == original)
        names = [row['name'] for row in access.tree()]
        check('Native hierarchy exposes all three work groups', all(name in names for name in ['پیشنهادها', 'کالا و مشتری', 'حسابداری']))
        tab_to('دفتر حسابداری')
        capture('sidebar-standard-light-focus.png')
        key('Return')
        capture('sidebar-standard-light.png')
        resize(650, 760)
        page(7, 'accounting')
        capture('accounting-narrow-light.png')
        history = tab_to('تاریخچه', roles=['page tab'])
        focus_event = native.wait_for(lambda: next((event for event in reversed(native.events(profile)) if event.get('focus_name') == 'accounting-tab-audit'), None), label='native history focus geometry')
        rect = focus_event['focus_bounds']
        viewport = focus_event['window_size']
        check('Narrow history tab is fully inside native window', rect and rect[0] >= 0 and rect[0] + rect[2] <= viewport[0] and rect[1] >= 0 and rect[1] + rect[3] <= viewport[1], native_focus_bounds=rect, native_viewport=viewport)
        key('space')
        history = native.wait_for(lambda: access.find('تاریخچه', ['page tab']), label='history tab after keyboard activation')
        states = access.a.atspi_accessible_get_state_set(history['node'])
        check('Narrow keyboard activates history tab', states and access.a.atspi_state_set_contains(states, 23))
        capture('accounting-narrow-history-light.png')
        check('History navigation preserves saved payload', payload(profile) == original)
        tab_to('فهرست صفحه‌ها')
        key('Return')
        current = native.wait_for(lambda: access.find('دفتر حسابداری', ['list item']), label='collapsed menu destination')
        check('Opening narrow sidebar focuses current page', access.focused(current))
        capture('sidebar-narrow-light-focus.png')
        key('Escape')
        toggle = native.wait_for(lambda: access.find('فهرست صفحه‌ها'), label='menu toggle after Escape')
        check('Escape closes menu and restores toggle focus', access.focused(toggle))
        key('Return')
        key('Up')
        key('Return')
        check('Narrow arrow/Enter route opens catalog', native.events(profile)[-1]['page'] == 'catalog')
        check('Narrow keyboard route preserves saved payload', payload(profile) == original)
        resize(1280, 900)
        page(6, 'settings')
        tab_to('ظاهر', limit=130)
        key('Return'); key('Home'); key('Down'); key('Down'); key('Return')
        key('s', 'Control_L')
        native.wait_for(lambda: payload(profile)['settings']['appearance'] == 'Dark', label='appearance persisted')
        check('Native appearance selection saves dark', payload(profile)['settings']['appearance'] == 'Dark')
        page(7, 'accounting')
        capture('sidebar-standard-dark.png')
        resize(650, 760)
        tab_to('فهرست صفحه‌ها'); key('Return')
        capture('sidebar-narrow-dark-focus.png')
        key('Escape')
        page(7, 'accounting')
        capture('accounting-narrow-dark.png')
        before_restart = payload(profile)
        key('F4', 'Alt_L')
        native.wait_for(lambda: app.poll() is not None, label='normal window close')
        env['SARV_CAPTURE_NAME'] = 'restart.png'
        app = start([str(binary)], 'native-sidebar-restart', env)
        window = native.wait_for(lambda: x.find(app.pid), label='owned restarted window')
        native.wait_for(lambda: (profile / 'restart.png').exists(), label='restart rendering')
        check('Restart preserves full payload and appearance', payload(profile) == before_restart)
        check('Test binary remained immutable throughout run', hashlib.sha256(binary.read_bytes()).hexdigest() == report['binary_sha256'])
        report['result'] = 'passed'
    except Exception as error:
        report['error'] = repr(error)
        if x:
            try:
                capture('failure.png')
                (profile / 'failure-tree.json').write_text(json.dumps([{k:v for k,v in row.items() if k != 'node'} for row in access.tree()], ensure_ascii=False, indent=2))
            except Exception:
                pass
    finally:
        if x:
            x.x.XCloseDisplay(x.d)
        for proc in reversed(processes):
            native.stop(proc)
        for log in logs:
            log.close()
        (profile / 'sidebar-verification.json').write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
        print(json.dumps({'result': report['result'], 'checks': len(report['checks']), 'profile': str(profile), 'error': report.get('error')}, ensure_ascii=False))
    if report['result'] != 'passed':
        raise SystemExit(1)


if __name__ == '__main__':
    main()
