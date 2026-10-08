#!/usr/bin/env python3
"""Independent native product workflow; disposable clone, actual XTest input.

Source activate.sh, then dbus-run-session -- /usr/bin/python3 scripts/product-qa.py
  --profile artifacts/cloud-onboarding/PASSING-PROFILE [--expect-sha SHA]
Only synthetic profiles with passing native verification are accepted. Every
recorded key is delivered to a private X server; AT-SPI selects native actions.
"""
import argparse
import csv
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


def state(profile):
    with sqlite3.connect(f'file:{profile}/sarv.sqlite3?mode=ro', uri=True) as connection:
        generation, payload = connection.execute('SELECT generation,payload FROM state WHERE id=1').fetchone()
    return generation, json.loads(payload)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--profile', type=Path, required=True)
    parser.add_argument('--expect-sha')
    parser.add_argument('--after-approval', action='store_true', help='Debug only: clone a prior owned product artifact and continue after approval')
    args = parser.parse_args()
    source = args.profile.resolve()
    if not source.is_relative_to(ROOT / 'artifacts'):
        raise ValueError('Only disposable synthetic artifacts are permitted')
    if args.after_approval:
        assert source.is_relative_to(ROOT / 'artifacts/product-native')
        prior = json.loads((source / 'product-verification.json').read_text())
        original = Path(prior['source'])
        assert original.is_relative_to(ROOT / 'artifacts')
        assert (original / 'native-verification.json').exists()
    else:
        assert json.loads((source / 'native-verification.json').read_text())['result'] == 'passed'
    binary = ROOT / 'target/release/sarv-quote'
    digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    if args.expect_sha and digest != args.expect_sha:
        raise ValueError(f'Release changed: {digest}')
    output = ROOT / 'artifacts/product-native'
    output.mkdir(exist_ok=True)
    profile = Path(tempfile.mkdtemp(prefix='run-', dir=output))
    with sqlite3.connect(f'file:{source}/sarv.sqlite3?mode=ro', uri=True) as src:
        with sqlite3.connect(profile / 'sarv.sqlite3') as dst:
            src.backup(dst)
    report = {'result': 'failed', 'binary_sha256': digest, 'source': str(source),
              'profile': str(profile), 'checks': [], 'resumed_debug': args.after_approval,
              'input': 'Private Xvfb XTest and owned-process native AT-SPI actions'}
    processes, logs, x = [], [], None
    def start(command, name, env=None, **kwargs):
        log = (profile / f'{name}.log').open('w')
        logs.append(log)
        proc = subprocess.Popen(command, env=env, stdout=log, stderr=log, start_new_session=True, **kwargs)
        processes.append(proc)
        return proc
    def check(name, condition, **details):
        report['checks'].append({'name': name, 'passed': bool(condition), **details})
        if not condition:
            raise AssertionError(name)
    try:
        with tempfile.TemporaryFile() as display_file:
            start(['Xvfb', '-displayfd', str(display_file.fileno()), '-screen', '0', '1450x1050x24',
                   '-nolisten', 'tcp', '-xkbdir', str(ENVROOT / 'sysroot/usr/share/X11/xkb')],
                  'xvfb', pass_fds=(display_file.fileno(),))
            def read_display():
                display_file.seek(0)
                return display_file.read().decode().strip()
            display = ':' + native.wait_for(read_display)
        runtime = native.private_accessibility_environment(display, profile)
        env = os.environ.copy()
        env.update(DISPLAY=display, GDK_BACKEND='x11', GSK_RENDERER='cairo',
                   SARV_DATA_DIR=str(profile), SARV_TEST_INSTANCE='qa-product-independent',
                   XDG_RUNTIME_DIR=str(runtime), SARV_UI_SMOKE='capture',
                   SARV_CAPTURE_PAGE='order', SARV_CAPTURE_NAME='initial.png')
        start(['openbox', '--sm-disable', '--config-file', str(ENVROOT / 'sysroot/etc/xdg/openbox/rc.xml')], 'openbox', env)
        x = native.X11(display)
        app = start([str(binary)], 'app', env)
        window = native.wait_for(lambda: x.find(app.pid))
        native.wait_for(lambda: (profile / 'initial.png').exists())
        x.focus(window)
        access = Access(app.pid)
        native.wait_for(access.tree)
        def key(name, *mods):
            page = {'1': 'order', '2': 'catalog', '3': 'customers', '4': 'quotes',
                    '5': 'backup', '6': 'settings'}.get(name) if 'Alt_L' in mods else None
            if page:
                # A successful dialog save updates state before its closing
                # animation releases window shortcuts. Wait for that boundary.
                time.sleep(.4)
            x.key(name, *mods)
            time.sleep(.12)
            if page:
                native.wait_for(lambda: native.events(profile)[-1].get('page') == page,
                                label=f'actual settled shortcut page {page}')
                time.sleep(.3)
        def tree():
            return access.tree()
        def control(name, roles=None):
            return native.wait_for(lambda: access.find(name, roles), label=f'native {name}')
        def has_text(part):
            return any(part in r['name'] or (r['role'] in ['label', 'static'] and part in access.text(r)) for r in tree())
        def action(name):
            def selected_action():
                exact = access.find(name, ['button', 'push button'])
                if exact:
                    return exact
                # GTK container label relations can override a button's own
                # accessible property. A filtered single result remains
                # unambiguous; never fall back among multiple records.
                candidates = [r for r in tree() if r['role'] in ['button', 'push button']
                              and r['name'] and name.startswith(r['name'] + ' ')]
                return candidates[0] if len(candidates) == 1 else None
            row = native.wait_for(selected_action, label=f'native action {name}')
            assert access.click(row), name
            time.sleep(.35)
        def tab_to(label=None, target=None, focus_type=None, maximum=100):
            backwards, previous, repeated = False, None, 0
            for _ in range(maximum):
                event = native.events(profile)[-1] if native.events(profile) else {}
                if (label and event.get('focus_label') == label) or (target and event.get('focus') == target) or (focus_type and event.get('focus_type') == focus_type):
                    return
                if label:
                    candidate = access.find(label, ['text', 'entry', 'combo box', 'check box'])
                    if candidate and access.focused(candidate):
                        return
                count = len(native.events(profile))
                key('Tab', *(['Shift_L'] if backwards else []))
                if len(native.events(profile)) > count:
                    current = native.events(profile)[-1]
                    signature = (current.get('focus'), current.get('focus_type'), str(current.get('focus_bounds')))
                    repeated = repeated + 1 if signature == previous else 0
                    previous = signature
                    # Native windows need not wrap at the last control. Search
                    # in the other direction after observing the end instead
                    # of claiming a trap from a forward-only harness.
                    if repeated >= 3 and not backwards:
                        backwards, previous, repeated = True, None, 0
            (profile / 'focus-failure-tree.json').write_text(json.dumps(tree(), ensure_ascii=False, indent=2))
            raise AssertionError(f'Native Tab cannot reach {label or target or focus_type}')
        def entry(label, value):
            tab_to(label=label)
            key('a', 'Control_L')
            x.type_text(value)
            time.sleep(.12)
        def capture(name):
            time.sleep(.7)
            expected = None
            for prefix, page in [('editor-', 'order'), ('customers-', 'customers'), ('quotes-', 'quotes'),
                                 ('backup-', 'backup'), ('settings-', 'settings')]:
                if name.startswith(prefix):
                    expected = page
            if name in ['catalog-light.png', 'catalog-dark.png']:
                expected = 'catalog'
            actual = native.events(profile)[-1].get('page') if native.events(profile) else None
            if expected:
                assert actual == expected, f'Capture {name} is {actual}, expected {expected}'
            report.setdefault('captures', {})[name] = {'page': actual, 'geometry': x.geometry(window)}
            subprocess.run(['/usr/bin/import', '-display', display, '-window', str(window), str(profile / name)], check=True)
            (profile / (name + '.a11y.json')).write_text(json.dumps(tree(), ensure_ascii=False, indent=2))
        if not args.after_approval:
            initial = state(profile)[1]
            key('3', 'Alt_L')
            action('مشتری تازه')
            fields = ['شرکت ساختگی مستقل کاغذ و نوشت افزار', 'مخاطب مستقل', '09120000009', 'independent@example.test', 'تهران خیابان ساختگی پلاک ۲']
            for i, value in enumerate(fields):
                if i:
                    key('Tab')
                x.type_text(value)
            capture('customer-dialog.png')
            key('Return')
            customer = native.wait_for(lambda: next((c for c in state(profile)[1]['customers'] if c['name'] == fields[0]), None))
            check('Native customer Enter preserves all five independently typed fields',
                  [customer['name'], customer.get('contact_name'), customer['phone'], customer.get('email'), customer.get('address')] == fields)
            check('Customer creation leaves approved history unchanged', state(profile)[1]['quotes'] == initial['quotes'])
            before = state(profile)
            action('مشتری تازه')
            duplicate = [fields[0], 'مخاطب متفاوت', fields[2], fields[3], fields[4]]
            for i, value in enumerate(duplicate):
                if i:
                    key('Tab')
                x.type_text(value)
            key('Return')
            check('Conflicting duplicate native customer submission remains unchanged', state(profile) == before)
            check('Conflicting duplicate shows explicit edit guidance', native.wait_for(lambda: has_text('جزئیات را از ویرایش مشتری تغییر دهید')))
            key('Escape')
            key('2', 'Alt_L')
            before = state(profile)
            invalid_csv = profile / 'independent-invalid.csv'
            invalid_csv.write_text('code,name,measure,price,currency,source_at,aliases\nGOOD,صحیح,عدد,125.1,toman,2020-01-01T00:00:00Z,\nBAD,نامعتبر,عدد,0,toman,2020-01-01T00:00:00Z,\nGOOD,تکرار,عدد,2,rial,2020-01-01T00:00:00Z,\n')
            action('ورود فهرست قیمت و پیش‌نمایش')
            native.wait_for(lambda: any(r['name'] == 'انتخاب فایل' for r in tree()), label='native import chooser')
            key('l', 'Control_L')
            key('a', 'Control_L')
            x.type_ascii(str(invalid_csv))
            key('Return')
            time.sleep(.35)
            if any(r['name'] == 'انتخاب فایل' for r in tree()):
                key('Return')
            native.wait_for(lambda: any(r['name'] == 'خطاهای فایل فهرست قیمت' for r in tree()), label='native bounded import diagnostics')
            capture('import-diagnostics.png')
            check('Native CSV import presents row/column diagnostics for both invalid records', has_text('price') and has_text('code') and has_text('کد تکراری'))
            check('Native rejected multi-error import preserves exact whole persisted state', state(profile) == before)
            key('Escape')
            action('کپی قالب CSV')
            action('فهرست تازه')
            # The dialog focuses its catalog-name entry. All remaining text fields
            # use native Tab traversal and actual Unicode XTest key input.
            key('v', 'Control_L')
            copied = access.text(control('نام فهرست', ['text']))
            check('In-app CSV template is actually copied and natively pasteable', 'code,name,measure,price,currency,source_at,aliases' in copied)
            key('a', 'Control_L')
            x.type_text('فهرست ساختگی مستقل')
            for label, value in [('کد کالا', 'QA-NATIVE-1'), ('شرح کالا', 'کالای ساختگی مستقل با شرح بلند'),
                                 ('واحد شمارش', 'عدد'), ('قیمت در واحد منبع · بدون جداکننده', '1251'),
                                 ('تاریخ منبع · YYYY-MM-DD', '2020-01-01'),
                                 ('نام‌های مستعار تأییدشده · جداشده با |', 'نام ساختگی|alias independent')]:
                entry(label, value)
            before = state(profile)
            action('افزودن کالا')
            check('Composer staging does not persist a partial catalog', state(profile) == before)
            capture('catalog-composer.png')
            for label, value in [('کد کالا', 'qa-native-1'), ('شرح کالا', 'ردیف تکراری'),
                                 ('قیمت در واحد منبع · بدون جداکننده', '1'), ('تاریخ منبع · YYYY-MM-DD', '2020-01-01')]:
                entry(label, value)
            action('افزودن کالا')
            check('Duplicate native product is rejected without partial persistence', state(profile) == before)
            check('Duplicate product displays a validation reason', native.wait_for(lambda: has_text('کد کالا تکراری') or has_text('کد تکراری')))
            action('پاک‌کردن فرم')
            action('بررسی و تأیید فهرست تازه')
            capture('catalog-confirm.png')
            action('تأیید منبع و ذخیرهٔ فهرست')
            catalog = native.wait_for(lambda: next((c for c in state(profile)[1]['catalogs'] if c['name'] == 'فهرست ساختگی مستقل'), None))
            product = catalog['products'][0]
            check('Native approved catalog persists exact source, aliases and rial amount',
                  product['code'] == 'QA-NATIVE-1' and product['price_rial'] == 1251 and product['source_unit'] == 'Rial'
                  and set(product['aliases']) == {'نام ساختگی', 'alias independent'} and product['source_at'] == '2020-01-01T00:00:00Z', product=product)
            key('1', 'Alt_L')
            key('n', 'Control_L')
            # New document may have a deliberate preservation dialog.
            for row in tree():
                if row['name'] == 'پیش‌فاکتور تازه' and row['role'] == 'button':
                    access.click(row)
                    time.sleep(.3)
                    break
            key('f', 'Control_L')
            x.type_text('مخاطب مستقل')
            time.sleep(.6)
            capture('customer-picker.png')
            key('Return')
            native.wait_for(lambda: state(profile)[1]['draft'].get('customer_id') == customer['id'])
            check('In-document customer picker loads identity/contact/email/address snapshot',
                  all(state(profile)[1]['draft'].get(k) == v for k, v in [('customer_contact', fields[1]), ('customer_email', fields[3]), ('customer_address', fields[4])]))
            before = state(profile)
            key('f', 'Control_L')
            x.type_text('NO MATCH QA')
            key('Escape')
            check('Customer picker Escape preserves exact saved draft', state(profile) == before)
            key('k', 'Control_L')
            x.type_text('alias independent')
            time.sleep(.6)
            capture('product-picker.png')
            key('Return')
            key('a', 'Control_L')
            x.type_ascii('2.375')
            key('Return')
            draft = native.wait_for(lambda: state(profile)[1]['draft'] if state(profile)[1]['draft']['lines'] else None)
            check('Product alias picker → quantity Enter adds exact explicit source',
                  draft['lines'] == [{'query': 'QA-NATIVE-1', 'quantity_milli': 2375, 'chosen_code': 'QA-NATIVE-1'}])
            control('منبع قیمت‌های قدیمی را بررسی و تأیید کرده‌ام', ['check box'])
            tab_to(focus_type='GtkCheckButton')
            key('space')
            key('s', 'Control_L')
            key('Return', 'Control_L')
            approved = native.wait_for(lambda: next((q for q in state(profile)[1]['quotes'] if q.get('customer_id') == customer['id']), None))
            net = 2971 - draft['discount_rial']
            tax = (net * draft['tax_basis_points'] + 5000) // 10000
            check('Native approval retains exact integer totals and contact snapshot',
                  approved['total_rial'] == net + tax and approved['tax_rial'] == tax
                  and approved['customer_address'] == fields[4] and approved['lines'][0]['price_rial'] == 1251,
                  expected_subtotal_rial=2971, expected_tax_rial=tax, expected_total_rial=net + tax)
            capture('editor-approved.png')
            action('پیش‌نمایش چاپ')
            native.wait_for(lambda: any('پیش‌نمایش' in r['name'] and r['role'] == 'dialog' for r in tree()), seconds=30)
            capture('document-preview.png')
            for extension in ['csv', 'pdf']:
                destination = profile / f'independent-preview.{extension}'
                action('خروجی ' + extension.upper())
                native.wait_for(lambda: any(r['name'] == 'ذخیرهٔ فایل با نام تازه' for r in tree()), label='native save chooser')
                key('l', 'Control_L')
                key('a', 'Control_L')
                x.type_ascii(str(destination))
                key('Return')
                time.sleep(.35)
                if any(r['name'] == 'ذخیرهٔ فایل با نام تازه' for r in tree()):
                    key('Return')
                native.wait_for(lambda: destination.exists() and destination.stat().st_size > 100, seconds=30,
                                label=f'native preview {extension} output')
                if extension == 'csv':
                    with destination.open(newline='') as exported:
                        rows = list(csv.DictReader(exported))
                    check('Native preview CSV saves the shown contact/address and exact total',
                          len(rows) == 1 and rows[0]['customer_contact'] == fields[1]
                          and rows[0]['customer_address'] == fields[4] and rows[0]['quote_total_rial'] == str(net + tax))
                else:
                    extracted = subprocess.run(['pdftotext', '-layout', str(destination), '-'], check=True, capture_output=True).stdout.decode()
                    (profile / 'independent-preview.txt').write_text(extracted)
                    check('Native preview PDF contains the selected code and contact email', 'QA-NATIVE-1' in extracted and fields[3] in extracted)
                check(f'Native preview {extension.upper()} export leaves approved history unchanged',
                      next(q for q in state(profile)[1]['quotes'] if q['id'] == approved['id']) == approved)
            key('Escape')
        else:
            fields = ['شرکت ساختگی مستقل کاغذ و نوشت افزار', 'مخاطب مستقل', '09120000009', 'independent@example.test', 'تهران خیابان ساختگی پلاک ۲']
            customer = next(c for c in state(profile)[1]['customers'] if c['name'] == fields[0])
            approved = next(q for q in reversed(state(profile)[1]['quotes']) if q.get('customer_id') == customer['id'])
        key('4', 'Alt_L')
        key('f', 'Control_L')
        x.type_ascii(approved['number'])
        time.sleep(.6)
        def latin_digits(value):
            return value.translate(str.maketrans('۰۱۲۳۴۵۶۷۸۹', '0123456789'))
        expander = native.wait_for(lambda: next((r for r in tree() if r['role'] == 'list item' and approved['number'] in latin_digits(r['name'])), None),
                                   label='approved offer native expander')
        bounds = access.rect(expander)
        assert bounds and bounds['width'] > 50 and bounds['height'] > 20, 'Native archive row exposes usable bounds'
        report['archive_expander_bounds'] = bounds
        report['window_origin'] = x.origin(window)
        # GTK on this backend supplies width/height but zero global positions
        # for these accessible rows. Do not click an invented screen location.
        if bounds['x'] > 0 and bounds['y'] > 0:
            x.pointer(bounds['x'] + bounds['width'] // 2, bounds['y'] + bounds['height'] // 2)
        x.focus(window)
        time.sleep(.25)
        if not access.find('نسخهٔ جدید', ['button']):
            for _ in range(25):
                key('Tab')
                if access.focused(expander):
                    key('Return')
                    break
        capture('archive-expanded-probe.png')
        action('نسخهٔ جدید')
        tab_to(target='row0-quantity')
        key('a', 'Control_L')
        x.type_ascii('3.125')
        key('Return')
        tab_to(focus_type='GtkCheckButton')
        key('space')
        key('s', 'Control_L')
        key('Return', 'Control_L')
        revision = native.wait_for(lambda: next((q for q in state(profile)[1]['quotes'] if q['series'] == approved['series'] and q['version'] == 2), None))
        check('Native revision preserves original offer and exact revised quantity',
              next(q for q in state(profile)[1]['quotes'] if q['id'] == approved['id']) == approved
              and revision['lines'][0]['quantity_milli'] == 3125 and revision['lines'][0]['total_rial'] == 3909)
        approved = revision
        key('3', 'Alt_L')
        key('f', 'Control_L')
        x.type_ascii(fields[3])
        time.sleep(.6)
        action('پیش‌فاکتورهای مشتری ' + fields[0])
        capture('customer-linked-history.png')
        check('Customer-linked native history exposes the approved quote number', native.wait_for(lambda: has_text(approved['number'])))
        key('Escape')
        immutable = state(profile)[1]['quotes']
        action('بایگانی مشتری ' + fields[0])
        native.wait_for(lambda: next(c for c in state(profile)[1]['customers'] if c['id'] == customer['id']).get('archived'))
        check('Native customer archive preserves approved offer snapshots', state(profile)[1]['quotes'] == immutable)
        control('نمایش بایگانی', ['check box'])
        tab_to(focus_type='GtkCheckButton')
        key('space')
        action('بازگردانی از بایگانی مشتری ' + fields[0])
        native.wait_for(lambda: not next(c for c in state(profile)[1]['customers'] if c['id'] == customer['id']).get('archived', False))
        check('Native customer unarchive restores identity without touching history', state(profile)[1]['quotes'] == immutable)
        action('ویرایش مشتری ' + fields[0])
        key('Tab')
        key('a', 'Control_L')
        x.type_text('مخاطب تازه مستقل')
        key('Return')
        native.wait_for(lambda: next(c for c in state(profile)[1]['customers'] if c['id'] == customer['id']).get('contact_name') == 'مخاطب تازه مستقل')
        check('Native customer contact edit preserves immutable quote contact/address', state(profile)[1]['quotes'] == immutable)
        for num, page in [(2, 'catalog'), (3, 'customers'), (4, 'quotes'), (5, 'backup'), (6, 'settings')]:
            key(str(num), 'Alt_L')
            capture(page + '-light.png')
        # AdwComboRow is an actual native combo. Open its accessible action,
        # select with real keys, save, then verify the persisted setting.
        for index, appearance in [(2, 'Dark'), (1, 'Light')]:
            control('تم برنامه', ['combo box'])
            tab_to(label='تم برنامه')
            key('Return')
            key('Home')
            for _ in range(index):
                key('Down')
            key('Return')
            key('s', 'Control_L')
            native.wait_for(lambda: state(profile)[1]['settings']['appearance'] == appearance,
                            label=f'native appearance saved {appearance}')
            check(f'Native appearance selection saves {appearance}', state(profile)[1]['settings']['appearance'] == appearance)
            if appearance == 'Dark':
                for num, page in [(1, 'editor'), (2, 'catalog'), (3, 'customers'), (4, 'quotes'), (5, 'backup'), (6, 'settings')]:
                    key(str(num), 'Alt_L')
                    capture(page + '-dark.png')
                key('1', 'Alt_L')
                action('پیش‌نمایش چاپ')
                native.wait_for(lambda: any('پیش‌نمایش' in r['name'] and r['role'] == 'dialog' for r in tree()), seconds=30)
                capture('document-preview-dark.png')
                key('Escape')
                key('6', 'Alt_L')
        key('1', 'Alt_L')
        x.resize(window, 900, 560)
        native.wait_for(lambda: x.geometry(window) == [900, 560])
        tab_to(target='new-item')
        capture('editor-900x560-input.png')
        tab_to(target='discount')
        # Show the actual final total below the focused discount field with
        # native wheel events; focus scrolling only guarantees the entry itself.
        origin = x.origin(window)
        for _ in range(3):
            x.pointer(origin[0] + 150, origin[1] + 480, button=5)
            time.sleep(.15)
        capture('editor-900x560-total.png')
        before = state(profile)
        key('F4', 'Alt_L')
        native.wait_for(lambda: app.poll() is not None)
        env['SARV_CAPTURE_NAME'] = 'restart.png'
        app = start([str(binary)], 'restart', env)
        window = native.wait_for(lambda: x.find(app.pid))
        native.wait_for(lambda: (profile / 'restart.png').exists())
        check('Fresh process preserves composed catalog, customer, exact draft and approved offer', state(profile) == before)
        check('Release hash is unchanged throughout independent run', hashlib.sha256(binary.read_bytes()).hexdigest() == digest)
        report['result'] = 'passed'
    except Exception as error:
        report['error'] = repr(error)
        if x:
            try:
                subprocess.run(['/usr/bin/import', '-display', display, '-window', str(window), str(profile / 'failure.png')], check=True)
                (profile / 'failure.a11y.json').write_text(json.dumps(tree(), ensure_ascii=False, indent=2))
            except Exception:
                pass
    finally:
        if x:
            x.x.XCloseDisplay(x.d)
        for proc in reversed(processes):
            native.stop(proc)
        for log in logs:
            log.close()
        (profile / 'product-verification.json').write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
        print(json.dumps({'result': report['result'], 'checks': len(report['checks']), 'profile': str(profile), 'error': report.get('error')}))
    return 0 if report['result'] == 'passed' else 1


if __name__ == '__main__':
    raise SystemExit(main())
