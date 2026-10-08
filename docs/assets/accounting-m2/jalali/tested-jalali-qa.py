#!/usr/bin/env python3
"""Disposable Jalali1405 native workflow with independent canonical-date, money,
provenance, backup/restart and real XTest date-entry checks.

Source /workspace/sarv-environment/activate.sh then run:
  dbus-run-session -- /usr/bin/python3 scripts/jalali-qa.py
All databases, screenshots and PDFs stay in a new artifacts/jalali-native run.
"""
import csv
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import signal
import shutil
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
    output = ROOT / 'artifacts' / 'jalali-native'
    output.mkdir(parents=True, exist_ok=True)
    profile = Path(tempfile.mkdtemp(prefix='run-', dir=output))
    binary_source = ROOT / 'target/release/sarv-quote'
    binary = profile / 'sarv-quote-under-test'
    shutil.copy2(binary_source, binary)
    report = {'result': 'failed', 'profile': str(profile), 'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
              'backend': 'private X11 Xvfb/Openbox GTK4; signal financial workflow and actual XTest date input/navigation separately', 'checks': []}
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
                   SARV_DATA_DIR=str(profile), SARV_TEST_INSTANCE='jalali-qa', SARV_ACCOUNTING_SMOKE='jalali')
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
        check('Jalali presentation preference persisted', baseline['settings']['date_display'] == 'Jalali')
        period = ledger['periods'][0]
        check('fiscal1405 has independently expected canonical Gregorian bounds',
              period['start_date'] == '2026-03-21' and period['end_date'] == '2027-03-20')
        journals = ledger['journals']
        check('all financial dates stored canonical ISO Gregorian',
              [j['date'] for j in journals] == ['2026-03-21', '2026-04-21', '2026-04-22'])
        check('opening, manual and linked reversal retained', journals[0]['kind'] == 'Opening'
              and journals[1]['kind'] == 'Manual' and journals[2]['kind'] == {'Reversal': {'journal_id': journals[1]['id']}})
        check('exact native opening/manual money', [sum(line['debit'] for line in j['lines']) for j in journals] == [1251, 349, 349])
        check('native dates retain UTC provenance timestamps', all(j['recorded_at'].endswith('Z') or '+00:00' in j['recorded_at'] for j in journals))
        check('fresh restored profile matches complete saved aggregate and preferences', state(profile / 'restored-profile') == baseline)
        backup = json.loads((profile / 'accounting-backup.json').read_text())
        check('backup retains complete canonical ledger and presentation preference', backup['state'] == baseline)
        for filename in ['trial-before-reversal-rial.pdf', 'trial-final-toman.pdf']:
            check(f'{filename} exists', (profile / filename).stat().st_size > 500)
            subprocess.run(['pdftoppm', '-f', '1', '-singlefile', '-scale-to', '1200', '-png', str(profile / filename), str(profile / filename.removesuffix('.pdf'))], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        with (profile / 'trial-final-toman.csv').open(encoding='utf-8-sig', newline='') as file:
            exported = list(csv.DictReader(file))
        check('Jalali CSV discloses calendar and preserves canonical cutoff', len(exported) == 2 and all(
              row['as_of_gregorian'] == '2027-03-20' and row['display_calendar'] == 'jalali'
              and row['display_as_of'] == '1405/12/29' for row in exported))
        check('exact final exported trial balance1251 IRR', sum(int(row['debit_irr']) for row in exported) == 1251
              and sum(int(row['credit_irr']) for row in exported) == 1251)
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
        date_control = native.wait_for(lambda: access.find('تاریخ مالی', ['text']), label='date input accessible label')
        calendar_control = native.wait_for(lambda: access.find('هجری شمسی • YYYY/MM/DD', ['combo box']), label='date calendar accessible label')
        check('date input and explicit calendar both accessible', date_control is not None and calendar_control is not None)
        check('real native date field starts blank', access.text(date_control) == '')
        for _ in range(80):
            if access.focused(date_control): break
            x11.key('Tab'); time.sleep(.04)
        check('real Tab reaches financial date entry', access.focused(date_control))
        x11.type_text('۱۴۰۵/۱۲/۳۰'); time.sleep(.25)
        # Date validation precedes incomplete amount/account checks. Enter the
        # invalid date with real GTK Unicode keyboard input, then press save.
        access.click(native.wait_for(lambda: access.find('ثبت سند متوازن', ['button', 'push button']), label='journal save'))
        time.sleep(.3)
        check('nonleap Esfand30 rejected without losing Persian input', access.text(date_control) == '۱۴۰۵/۱۲/۳۰' and state(profile) == baseline, actual=access.text(date_control), state_equal=state(profile) == baseline)
        subprocess.run(['/usr/bin/import', '-display', display, '-window', str(window), str(profile / 'keyboard-invalid-jalali.png')], check=True)
        # Attempting a calendar switch while invalid must retain the input and
        # restore the prior calendar, rather than reinterpret its numeric year.
        x11.key('Tab', 'Shift_L'); time.sleep(.1); x11.key('space'); time.sleep(.15); x11.key('Home'); x11.key('Return'); time.sleep(.3)
        check('invalid date calendar switch retains input and books', access.text(date_control) == '۱۴۰۵/۱۲/۳۰' and state(profile) == baseline, actual=access.text(date_control), state_equal=state(profile) == baseline)
        x11.key('a', 'Control_L'); x11.type_ascii('1405/01/01'); time.sleep(.2)
        x11.key('Tab', 'Shift_L'); time.sleep(.1); x11.key('space'); time.sleep(.15); x11.key('Home'); x11.key('Return'); time.sleep(.3)
        check('valid Jalali-to-Gregorian switch preserves actual day', access.text(date_control).translate(str.maketrans('۰۱۲۳۴۵۶۷۸۹٠١٢٣٤٥٦٧٨٩', '01234567890123456789')) == '2026-03-21', actual=access.text(date_control))
        x11.key('space'); time.sleep(.15); x11.key('End'); x11.key('Return'); time.sleep(.3)
        check('Gregorian-to-Jalali switch roundtrips actual day', access.text(date_control).translate(str.maketrans('۰۱۲۳۴۵۶۷۸۹٠١٢٣٤٥٦٧٨٩', '01234567890123456789')) == '1405/01/01', actual=access.text(date_control))
        for width, height in [(1280, 900), (650, 760)]:
            x11.resize(window, width, height)
            native.wait_for(lambda: x11.geometry(window) == [width, height], label=f'{width}x{height} date dialog allocation')
            time.sleep(.3)
            subprocess.run(['/usr/bin/import', '-display', display, '-window', str(window), str(profile / f'keyboard-jalali-dialog-{width}x{height}.png')], check=True)
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
        history = native.wait_for(lambda: access.find('تاریخچه', ['page tab']), label='narrow native history tab')
        tabs = [native.wait_for(lambda title=title: access.find(title, ['page tab']), label=f'narrow tab {title}')
                for title in ['شرکت و دوره', 'کدینگ', 'اسناد', 'تراز آزمایشی', 'تاریخچه']]
        # All labels must have positive native extents within the same 650px
        # page; the span check catches horizontally clipped destinations.
        bounds = [access.rect(tab) for tab in tabs]
        check('all five native accounting tabs fit narrow layout', all(rect and rect['width'] > 0 and rect['height'] > 0 for rect in bounds)
              and max(rect['x'] + rect['width'] for rect in bounds) - min(rect['x'] for rect in bounds) <= 650, bounds=bounds)
        for _ in range(100):
            if access.focused(history): break
            x11.key('Tab'); time.sleep(.03)
        check('real Tab reaches history at650px', access.focused(history))
        x11.key('space'); time.sleep(.3)
        check('real Space activates narrow history', access.find('تاریخچهٔ محلی برنامه؛ در برابر مالک فایل‌ها مصون از دست‌کاری نیست. هویت فعلی: تک‌کاربر محلی.', ['label']) is not None and any(row['name'] == 'تاریخچه' and access.focused(row) for row in access.tree()))
        subprocess.run(['/usr/bin/import', '-display', display, '-window', str(window), str(profile / 'keyboard-accounting-history-650x760.png')], check=True)
        check('keyboard and narrow layouts preserve accounting state', state(profile) == baseline)
        # The same disposable profile also proves document and due dates use
        # the shared native calendar field without changing any accounting data.
        x11.resize(window, 1280, 900); time.sleep(.2)
        x11.key('1', 'Alt_L'); time.sleep(.3)
        details = native.wait_for(lambda: access.find('جزئیات پیشنهاد • تاریخ، اعتبار و مرجع'), label='quote details expander')
        check('native quote details expand', access.click(details)); time.sleep(.3)
        quote_date = native.wait_for(lambda: access.find('تاریخ پیشنهاد', ['text']), label='quote document date')
        validity = native.wait_for(lambda: access.find('اعتبار تا • اختیاری', ['text']), label='quote validity date')
        def tab_to(control, title):
            for _ in range(120):
                if access.focused(control): break
                x11.key('Tab'); time.sleep(.03)
            check(title, access.focused(control))
        before_quote = state(profile)
        tab_to(quote_date, 'real Tab reaches quote document date')
        x11.key('a', 'Control_L'); x11.type_text('۱۴۰۵/۱۲/۳۰'); x11.key('s', 'Control_L'); time.sleep(.7)
        check('invalid Jalali quote date keeps typed input and saved state', access.text(quote_date) == '۱۴۰۵/۱۲/۳۰' and state(profile) == before_quote)
        x11.key('a', 'Control_L'); x11.type_ascii('1405/01/01'); time.sleep(.2)
        tab_to(validity, 'real Tab reaches quote due date')
        x11.key('a', 'Control_L'); x11.type_text('١٤٠٥/١٢/٢٩'); x11.key('s', 'Control_L')
        quote_saved = native.wait_for(lambda: state(profile) if state(profile)['draft'].get('quote_date') == '2026-03-21'
                                     and state(profile)['draft'].get('valid_until') == '2027-03-20' else None, label='canonical quote dates durable')
        check('real CtrlS persists separate quote/due canonical ISO dates', quote_saved['draft']['quote_date'] == '2026-03-21' and quote_saved['draft']['valid_until'] == '2027-03-20')
        check('quote date entry retains exact ledger and prior UTC provenance', quote_saved['accounting'] == ledger)
        for width, height in [(1280, 900), (650, 760)]:
            x11.resize(window, width, height)
            native.wait_for(lambda: x11.geometry(window) == [width, height], label=f'{width}x{height} quote allocation')
            time.sleep(.25)
            subprocess.run(['/usr/bin/import', '-display', display, '-window', str(window), str(profile / f'keyboard-jalali-quote-{width}x{height}.png')], check=True)
        # Exercise the normal native backup chooser with a path inside this
        # disposable profile; never write into another user's data directory.
        x11.key('5', 'Alt_L'); time.sleep(.25)
        access.click(native.wait_for(lambda: access.find('ساخت پشتیبان JSON', ['button']), label='native backup action'))
        time.sleep(.6)
        quote_backup = profile / 'jalali-quote-backup.json'
        x11.key('l', 'Control_L'); time.sleep(.15); x11.type_ascii(str(quote_backup.with_suffix(''))); x11.key('Return'); time.sleep(.3)
        # A save chooser may first select its location and then need Save.
        if not quote_backup.exists():
            save_choice = access.find('Save', ['button']) or access.find('ذخیره', ['button'])
            if save_choice: access.click(save_choice)
        native.wait_for(quote_backup.exists, label='native quote backup written')
        backed_up = json.loads(quote_backup.read_text())['state']
        check('native backup preserves quote dates, complete ledger and presentation preference', backed_up == quote_saved)
        native.stop(app)
        env.update(SARV_CAPTURE_NAME='quote-date-restart.png')
        app = start([str(binary)], 'quote-restart', env)
        window = native.wait_for(lambda: x11.find(app.pid), label='quote restart window')
        native.wait_for(lambda: (profile / 'quote-date-restart.png').exists(), label='quote restart rendered')
        x11.focus(window)
        check('fresh native restart preserves canonical quote dates and complete state', state(profile) == quote_saved)
        report['result'] = 'passed' 
    except Exception as error:
        report['error'] = str(error)
        raise
    finally:
        if x11: x11.x.XCloseDisplay(x11.d)
        for proc in reversed(processes): native.stop(proc)
        for log in logs: log.close()
        (profile / 'jalali-independent-native.json').write_text(json.dumps(report, ensure_ascii=False, indent=2))
        print(json.dumps(report, ensure_ascii=False, indent=2))

if __name__ == '__main__': main()
