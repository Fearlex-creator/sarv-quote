#!/usr/bin/env python3
"""M6 actual native XTest purchase returns with independent literal SQLite oracles.

Run only after integration GO, inside an owned dbus-run-session. Requires an
actual synthetic M5 March31 seed and a frozen source/binary manifest. No product
smoke driver is used: selection, typing, preview and post use native keyboard.
"""
import ctypes as C
import csv
import hashlib
import importlib.util
import json
import os
from pathlib import Path
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
        assert db.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
        return json.loads(db.execute('SELECT payload FROM state WHERE id=1').fetchone()[0])


def controls(raw):
    accounting = raw['accounting']
    balances = {}
    for journal in accounting['journals']:
        assert sum(line['debit'] for line in journal['lines']) == sum(line['credit'] for line in journal['lines'])
        for line in journal['lines']:
            identity = line['account']['id']
            balances[identity] = balances.get(identity, 0) + line['debit'] - line['credit']
    mapping = {m['role']: m['account_id'] for m in accounting['core']['setups'][0]['command']['mappings']}
    actual = {role: balances.get(identity, 0) for role, identity in mapping.items()}
    actual['TB'] = sum(v for v in balances.values() if v > 0)
    assert actual['TB'] == -sum(v for v in balances.values() if v < 0)
    return actual


def pools(raw):
    accounting = raw['accounting']
    result = {}
    for opening in accounting['core']['openings']:
        for row in opening['command']['stock']:
            result[row['item_id'], row['warehouse_id']] = (row['quantity_thousandths'], row['value'])
    records = accounting['procurement']['records'] + accounting['sales']['records']
    for record in sorted(records, key=lambda r: r['sequence']):
        for movement in record['movements']:
            after = movement['after']
            result[movement['item_id'], movement['warehouse_id']] = (after['quantity_thousandths'], after['value'])
    return result


def main():
    seed_path = Path(os.environ['SARV_PURCHASE_RETURNS_SEED_PROFILE']).resolve()
    unbilled_seed = Path(os.environ['SARV_PURCHASE_RETURNS_UNBILLED_PROFILE']).resolve()
    assert (seed_path/'sarv.sqlite3').is_file() and (unbilled_seed/'sarv.sqlite3').is_file(), 'Both synthetic native fixture databases are required before running'
    seed_states = {str(p): hashlib.sha256(json.dumps(state(p),sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()).hexdigest() for p in [seed_path,unbilled_seed]}
    manifest_path = Path(os.environ['SARV_PURCHASE_RETURNS_SOURCE_MANIFEST']).resolve()
    manifest = json.loads(manifest_path.read_text())
    binary_source = Path(os.environ.get('SARV_QA_BINARY', str(ROOT / 'target/debug/sarv-quote'))).resolve()
    output = ROOT / 'artifacts/purchase-returns-native'
    output.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix='run-', dir=output))
    profile = run / 'main'
    profile.mkdir()
    with sqlite3.connect(f'file:{seed_path}/sarv.sqlite3?mode=ro', uri=True) as src, sqlite3.connect(profile/'sarv.sqlite3') as dst:
        src.backup(dst)
    binary = run/'sarv-quote'
    shutil.copy2(binary_source, binary)
    observer = Path(__file__)
    runtime_dependencies = [ROOT/'scripts/qa-native.py',ROOT/'scripts/qa_access.py',ROOT/'assets/fonts/Vazirmatn-Regular.ttf',ROOT/'assets/fonts/Vazirmatn-Bold.ttf',ROOT/'assets/fonts/OFL.txt']
    portable = ROOT / 'docs/assets/accounting-m6/native' / run.name
    report = {'result': 'failed', 'portable': str(portable), 'run': str(run), 'seed': str(seed_path), 'unbilled_seed':str(unbilled_seed), 'seed_state_sha256_before':seed_states, 'binary_source': str(binary_source),
              'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
              'observer_sha256': hashlib.sha256(observer.read_bytes()).hexdigest(),
              'scope': 'owned native GTK XTest keyboard, private X11/D-Bus, synthetic copy, raw SQLite literal arithmetic',
              'fatal_criticals': True, 'checks': [],
              'limits': ['physical printing', 'screen reader interaction', 'Wayland', 'hardware power loss', 'professional/statutory acceptance', 'full PUR/INV/treasury/PERF gates']}
    report['runtime_dependency_sha256'] = {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in runtime_dependencies}
    processes, logs = [], []
    x11 = None

    def check(name, condition, **details):
        entry = {'name': name, 'passed': bool(condition), **details}
        report['checks'].append(entry)
        with (run/'progress.jsonl').open('a') as out:
            out.write(json.dumps(entry, ensure_ascii=False)+'\n')
        if not condition:
            raise AssertionError(name)

    def freeze():
        provenance = manifest['frozen_source_provenance']
        files, digest = {}, hashlib.sha256()
        for name in sorted(provenance['files'], key=lambda n: n.encode()):
            raw = (ROOT/name).read_bytes()
            files[name] = hashlib.sha256(raw).hexdigest()
            digest.update(name.encode()+b'\0'+raw+b'\0')
        check('frozen source files/aggregate exact', files == provenance['files'] and digest.hexdigest() == provenance['sha256'])
        check('frozen native binary exact', hashlib.sha256(binary.read_bytes()).hexdigest() == manifest['binary_sha256'])
        check('observer unchanged', hashlib.sha256(observer.read_bytes()).hexdigest() == report['observer_sha256'])
        actual_dependencies = {str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in runtime_dependencies}
        check('imported native helpers and licensed font assets unchanged',actual_dependencies == report['runtime_dependency_sha256'],files=actual_dependencies)
        return {'files': files, 'sha256': digest.hexdigest()}

    def start(command, name, env=None, **kw):
        log = (run/f'{name}.log').open('w')
        logs.append(log)
        proc = subprocess.Popen(command, env=env, stdout=log, stderr=log, start_new_session=True, **kw)
        processes.append(proc)
        return proc

    try:
        report['source_before'] = freeze()
        shutil.copy2(manifest_path, run/'frozen-source-manifest.json')
        before = state(profile)
        (run/'M6-main-before-state.json').write_text(json.dumps(before,ensure_ascii=False,indent=2))
        baseline = controls(before)
        expected = {'Cash': 251, 'Bank': 300, 'Receivables': 299, 'Inventory': 422, 'ChequesReceivable': 200,
                    'Payables': -329, 'ChequesPayable': -100, 'Grni': -50, 'OwnerCapital': -952,
                    'Sales': -199, 'SalesReturns': 100, 'Cogs': 58, 'TB': 1630}
        check('independent literal M5 March31 seed', all(baseline[k] == v for k, v in expected.items()), actual=baseline)
        a = before['accounting']
        receipt = next(r for r in a['procurement']['records'] if 'Receive' in r['data'] and r['data']['Receive']['total'] == 105)
        receipt_line = next(l for l in receipt['data']['Receive']['lines'] if l['quantity_thousandths'] == 7000 and l['value'] == 105)
        bill = next(r for r in a['procurement']['records'] if 'Bill' in r['data'] and any(l['receipt_line_id'] == receipt_line['id'] for l in r['data']['Bill']['lines']))
        bill_line = next(l for l in bill['data']['Bill']['lines'] if l['receipt_line_id'] == receipt_line['id'])
        check('literal frozen R-B billed7/105 in bill150', bill['data']['Bill']['total'] == 150 and bill_line['value'] == 105 and bill_line['quantity_thousandths'] == 7000)
        source_reference = bill['command']['Bill']['reference']
        pool_key = (receipt_line['item_id'], receipt_line['warehouse_id'])
        original_pools = pools(before)
        check('literal original warehouse25/292 and other warehouse12/130', original_pools[pool_key] == (25000,292) and (12000,130) in original_pools.values())
        with tempfile.TemporaryFile() as fd:
            start(['Xvfb', '-displayfd', str(fd.fileno()), '-screen', '0', '1450x1050x24', '-nolisten', 'tcp', '-xkbdir', str(ENVROOT/'sysroot/usr/share/X11/xkb')], 'xvfb', pass_fds=(fd.fileno(),))
            def getdisplay():
                fd.seek(0)
                return fd.read().decode().strip()
            display = ':'+native.wait_for(getdisplay, label='owned X11')
        runtime = native.private_accessibility_environment(display, run)
        env = os.environ.copy()
        env.update(DISPLAY=display, GDK_BACKEND='x11', GSK_RENDERER='cairo', XDG_RUNTIME_DIR=str(runtime),
                   SARV_TEST_INSTANCE='purchase-returns-native-qa', SARV_DATA_DIR=str(profile),
                   ADW_DEBUG_COLOR_SCHEME='prefer-light', G_DEBUG='fatal-criticals')
        for name in list(env):
            if name.startswith('SARV_') and ('SMOKE' in name or 'CAPTURE' in name):
                env.pop(name)
        start(['openbox', '--sm-disable', '--config-file', str(ENVROOT/'sysroot/etc/xdg/openbox/rc.xml')], 'openbox', env)
        x11 = native.X11(display)
        report['x11_errors'] = []
        class XError(C.Structure):
            _fields_ = [('type',C.c_int),('display',C.c_void_p),('resource',C.c_ulong),('serial',C.c_ulong),('code',C.c_ubyte),('request',C.c_ubyte),('minor',C.c_ubyte)]
        @C.CFUNCTYPE(C.c_int,C.c_void_p,C.POINTER(XError))
        def stale_window_error(_, error):
            if error.contents.code != 3:
                report['x11_errors'].append({'code':error.contents.code,'request':error.contents.request})
            return 0
        x11.x.XSetErrorHandler.argtypes=[C.c_void_p]
        x11.x.XSetErrorHandler(C.cast(stale_window_error,C.c_void_p))
        app = start([str(binary)], 'M6-main', env)
        window = native.wait_for(lambda: x11.find(app.pid), label='native M6 window')
        x11.focus(window)
        access = Access(app.pid)

        def focus(name, roles=None):
            native.wait_for(lambda: access.find(name, roles), label=f'accessible {name}')
            for _ in range(180):
                target = access.find(name, roles)
                if target and access.focused(target):
                    return target
                x11.key('Tab')
                time.sleep(.018)
            raise AssertionError(f'real Tab focus {name}')

        def fill(name, text):
            focus(name, ['text','entry'])
            x11.key('a','Control_L')
            if text:
                x11.type_ascii(text)
            else:
                x11.key('BackSpace')
            time.sleep(.12)
            if not text:
                check('actual keyboard empty fill clears focused native text',access.text(access.find(name,['text','entry']))=='')

        def press(name):
            focus(name, ['button','push button'])
            x11.key('Return')
            time.sleep(.2)

        def shot(name):
            subprocess.run(['/usr/bin/import','-display',display,'-window',str(window),str(run/f'{name}.png')],check=True,timeout=10)

        def tree_has(text):
            return any(text in r['name'] for r in access.tree())

        def native_export(caption, destination, record, credit, extension):
            export_before=state(profile)
            press(caption)
            native.wait_for(lambda:tree_has('ذخیرهٔ اعتبار تأمین‌کننده') if credit else tree_has('ذخیرهٔ گزارش خرید و انبار'),label='actual native export chooser')
            x11.key('l','Control_L');x11.key('a','Control_L');x11.type_ascii(str(destination));x11.key('Return');time.sleep(.3)
            if not destination.exists():
                x11.key('Return')
            native.wait_for(lambda:destination.exists() and destination.stat().st_size>100,seconds=30,label=f'actual native {caption} saved')
            native.wait_for(lambda:tree_has('گزارش ذخیره شد'),label='export worker completion')
            check(f'{caption} actual chooser/save changes no complete State',state(profile)==export_before)
            posted=record['data']['Return'];identity=posted['credit_id'] if credit else posted['return_id']
            kind='supplier_credit' if credit else 'purchase_return'
            if extension=='csv':
                with destination.open(encoding='utf-8-sig',newline='') as f:rows=list(csv.DictReader(f))
                metadata=next(row for row in rows if row['row_kind']=='document_metadata')
                line=next(row for row in rows if row['row_kind']==('supplier_credit_line' if credit else 'purchase_return_line'))
                total=next(row for row in rows if row['row_kind']=='document_total')
                complete=json.loads(next(row['snapshot_json'] for row in rows if row['row_kind']=='complete_snapshot'))
                check(f'{caption} exact original source/value/separate document identity',metadata['document_kind']==kind and metadata['report_source_id']==identity and metadata['credit_id']==posted['credit_id'] and metadata['return_id']==posted['return_id'] and line['receipt_line_id']==receipt_line['id'] and line['bill_line_id']==bill_line['id'] and line['quantity_thousandths']=='3000' and line['amount_irr']=='45' and total['amount_irr']=='45' and complete['document']==record)
            else:
                extracted=subprocess.run(['pdftotext','-layout',str(destination),'-'],check=True,capture_output=True).stdout.decode()
                destination.with_suffix('.txt').write_text(extracted)
                heading='اعتبار تأمین‌کننده' if credit else 'برگشت فیزیکی خرید'
                # Poppler may omit shaping-only ZWNJ and emit PDF direction
                # controls around words. Normalize these only for the heading;
                # exact source code/ID characters remain strict ASCII checks.
                heading_text=''.join(c for c in extracted if not c.isspace() and c not in '\u200c\u202a\u202b\u202c\u2066\u2067\u2068\u2069')
                heading_expected=''.join(c for c in heading if not c.isspace() and c!='\u200c')
                check(f'{caption} actual PDF heading/source references/identity',heading_expected in heading_text and identity in extracted and 'PR-M6-NATIVE-1' in extracted and 'SC-M6-NATIVE-1' in extracted,heading_normalization='Whitespace/ZWNJ/PDF bidi controls only; reference/ID characters unchanged and checked verbatim')
                fonts=subprocess.run(['pdffonts',str(destination)],check=True,capture_output=True).stdout.decode();destination.with_name(destination.stem+'-fonts.txt').write_text(fonts)
                check(f'{caption} embedded licensed Vazirmatn', 'Vazirmatn' in fonts and 'yes' in fonts)
                subprocess.run(['pdftoppm','-f','1','-singlefile','-scale-to','1200','-png',str(destination),str(destination.with_suffix(''))],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)

        before_binary = Path(os.environ.get('SARV_QA_BEFORE_BINARY','/workspace/sarv-evidence/bin/sarv-M5-candidate12')).resolve()
        old_sha = hashlib.sha256(before_binary.read_bytes()).hexdigest()
        report['before_binary'] = {'path': str(before_binary), 'sha256': old_sha}
        old_profile = run/'actual-M5-before'
        old_profile.mkdir()
        with sqlite3.connect(f'file:{seed_path}/sarv.sqlite3?mode=ro',uri=True) as src, sqlite3.connect(old_profile/'sarv.sqlite3') as dst:
            src.backup(dst)
        old_env = env.copy();old_env.update(SARV_DATA_DIR=str(old_profile),SARV_TEST_INSTANCE='purchase-returns-M5-before')
        # Stop the new process so AT-SPI and XTest own exactly one financial app.
        native.stop(app)
        app = start([str(before_binary)],'M5-actual-before',old_env)
        window = native.wait_for(lambda:x11.find(app.pid),label='genuine old M5 native window');x11.focus(window);access=Access(app.pid)
        x11.key('8','Alt_L')
        native.wait_for(lambda:access.find('سفارش خرید جدید',['button','push button']),label='genuine M5 purchase workspace')
        x11.resize(window,1280,900);native.wait_for(lambda:x11.geometry(window)==[1280,900],label='M5 before geometry');time.sleep(.25)
        check('genuine old M5 lacks new purchase return action',access.find('برگشت از خرید',['button','push button']) is None)
        check('genuine M5 before view preserves source profile whole state',state(old_profile)==before)
        shot('M5-actual-before-purchases');native.stop(app)
        check('genuine old M5 binary unchanged',hashlib.sha256(before_binary.read_bytes()).hexdigest()==old_sha)
        app = start([str(binary)],'M6-main-after-before',env)
        window = native.wait_for(lambda:x11.find(app.pid),label='native M6 reopened after actual before');x11.focus(window);access=Access(app.pid)
        x11.key('8','Alt_L')
        native.wait_for(lambda: access.find('برگشت از خرید',['button','push button']),label='purchases action')

        def open_billed():
            press('برگشت از خرید')
            fill('جست‌وجوی منبع، تأمین‌کننده، کالا یا انبار',source_reference)
            name = native.wait_for(lambda: next((r['name'] for r in access.tree() if r['role'] in ['button','push button'] and source_reference in r['name'] and 'بستانکار مرتبط' in r['name']),None),label='indexed exact bill source')
            press(name)
            native.wait_for(lambda: access.find('مرجع بستانکار تأمین‌کننده',['text','entry']),label='billed return editor')
            actual_quantity_names=[r['name'] for r in access.tree() if r['role'] in ['text','entry'] and r['name'].startswith('مقدار برگشت در واحد پایه')]
            report.setdefault('actual_native_quantity_names',[]).append(actual_quantity_names)
            check('actual bound source quantity fields have unique AT-SPI names',len(actual_quantity_names)==len(set(actual_quantity_names)))
            return name

        quantity_name = 'مقدار برگشت در واحد پایه • '+receipt_line['id']
        source_name = open_billed()
        fill('شرح و دلیل ثبت','Independent native overreturn')
        fill('شماره یا مرجع سند','PR-M6-REJECTED')
        fill('مرجع بستانکار تأمین‌کننده','SC-M6-REJECTED')
        fill(quantity_name,'8')
        press('پیش‌نمایش ثبت')
        native.wait_for(lambda: tree_has('حداکثر مانده'),label='overreturn error')
        check('native overreturn retains safe quantity and source; no financial write',access.text(access.find(quantity_name,['text','entry'])) == '8' and state(profile)['accounting'] == a)
        shot('M6-overreturn-retained')
        x11.key('Escape')
        time.sleep(.25)
        check('Escape cancellation leaves aggregate unchanged',state(profile)['accounting'] == a)

        for index, qty, value, remaining, inv, ap, tb, w1 in [(1,'3',45,4,377,284,1585,(22000,247)),(2,'4',60,0,317,224,1525,(18000,187))]:
            source_name = open_billed()
            fill('تاریخ مؤثر مدنی',f'2026-04-0{index}')
            fill('تاریخ سند طرف',f'2026-04-0{index}')
            fill('شرح و دلیل ثبت',f'Independent original-cost return {index}')
            fill('شماره یا مرجع سند',f'PR-M6-NATIVE-{index}')
            fill('مرجع بستانکار تأمین‌کننده',f'SC-M6-NATIVE-{index}')
            fill(quantity_name,qty)
            # Date selector immediately precedes its entry; keyboard-only calendar conversion.
            focus('تاریخ مؤثر مدنی',['text','entry'])
            x11.key('Tab','Shift_L');x11.key('space');time.sleep(.12);x11.key('End');x11.key('Return');time.sleep(.2)
            translated = access.text(access.find('تاریخ مؤثر مدنی',['text','entry'])).translate(str.maketrans('۰۱۲۳۴۵۶۷۸۹','0123456789'))
            check(f'return{index} Jalali conversion retains canonical April date',translated == f'1405/01/{11+index:02d}')
            focus('مرجع بستانکار تأمین‌کننده',['text','entry']);x11.key('Tab');x11.key('space');time.sleep(.12);x11.key('End');x11.key('Return');time.sleep(.2)
            check(f'return{index} exact readonly toman cost',any('بهای اصلی برگشت:' in r['name'] and 'تومان' in r['name'] for r in access.tree()))
            check(f'return{index} original and remaining source amounts follow selected toman display',all('تومان' in r['name'] and 'ریال' not in r['name'] for r in access.tree() if 'اصل مقدار' in r['name'] and 'ماندهٔ بها' in r['name']))
            prepreview = state(profile)
            shot(f'M6-Jalali-toman-{index}')
            press('پیش‌نمایش ثبت')
            native.wait_for(lambda: tree_has('بستانکار مرتبط، مانده'),label='return accounting preview')
            check(f'preview{index} is read-only original cost',state(profile)['accounting'] == prepreview['accounting'])
            shot(f'M6-preview-{index}')
            press('ثبت نهایی')
            # Immediate second Enter exercises real repeated activation during BUSY.
            x11.key('Return')
            native.wait_for(lambda: len([r for r in state(profile)['accounting']['procurement']['records'] if 'Return' in r['data']]) == index,label='durable atomic native return')
            raw = state(profile)
            actual = controls(raw)
            wanted = dict(expected, Inventory=inv, Payables=-ap, TB=tb)
            check(f'literal native return{index} all controls and TB',all(actual[k] == v for k,v in wanted.items()),actual=actual)
            returns = [r for r in raw['accounting']['procurement']['records'] if 'Return' in r['data']]
            posted = returns[-1]['data']['Return']
            check(f'return{index} exact original source/cost/states/credit',posted['total'] == value and posted['bill_id'] == bill['id'] and posted['receipt_id'] is None and posted['return_state'] == 'Returned' and posted['credit_state'] == 'Posted' and posted['credit_id'] != posted['return_id'] and posted['lines'][0]['bill_line_id'] == bill_line['id'] and posted['lines'][0]['warehouse_id'] == pool_key[1])
            current_pools = pools(raw)
            check(f'return{index} original warehouse literal and others unchanged',current_pools[pool_key] == w1 and all(current_pools[k] == v for k,v in original_pools.items() if k != pool_key))
            check(f'return{index} preserves opening/sales/quotation and old procurement prefix',raw['accounting']['core'] == a['core'] and raw['accounting']['sales'] == a['sales'] and raw.get('quotes') == before.get('quotes') and raw['accounting']['procurement']['records'][:len(a['procurement']['records'])] == a['procurement']['records'])
            # Source search and actual drill-down link trail, with no extra post.
            fill('جست‌وجوی شماره، منبع، کالا یا طرف',f'PR-M6-NATIVE-{index}')
            press('جزئیات و چاپ')
            native.wait_for(lambda: tree_has('شناسهٔ برگشت'),label='return source drill-down')
            check(f'return{index} native linked states/source lineage',tree_has('بستانکار ثبت‌شده') and tree_has(f'SC-M6-NATIVE-{index}') and tree_has(receipt['command']['Receive']['reference']) and tree_has(bill_line['id']))
            shot(f'M6-posted-detail-{index}')
            if index==1:
                check('native billed return exposes separate supplier credit CSV/PDF actions',bool(access.find('خروجی اعتبار تأمین‌کننده CSV',['button','push button']) and access.find('خروجی اعتبار تأمین‌کننده PDF',['button','push button'])))
                export_before=state(profile)
                press('خروجی اعتبار تأمین‌کننده PDF')
                native.wait_for(lambda:tree_has('ذخیرهٔ اعتبار تأمین‌کننده'),label='actual credit export cancel chooser')
                x11.key('Escape');time.sleep(.3)
                check('native credit save cancellation preserves complete State',state(profile)==export_before)
                for credit_export in [False,True]:
                    for extension in ['csv','pdf']:
                        caption=('خروجی اعتبار تأمین‌کننده ' if credit_export else 'خروجی سند ')+extension.upper()
                        destination=run/(('M6-actual-supplier-credit' if credit_export else 'M6-actual-physical-return')+'.'+extension)
                        native_export(caption,destination,returns[-1],credit_export,extension)
                shot('M6-separate-credit-export-actions')
            original_link = native.wait_for(lambda:next((r['name'] for r in access.tree() if r['role'] in ['button','push button'] and 'فاکتور اصلی •' in r['name']),None),label='original bill drill-down button')
            press(original_link)
            native.wait_for(lambda:tree_has('فاکتور '+ '\u2068'+source_reference),label='original bill opened')
            check(f'return{index} reciprocal original bill return links',tree_has('برگشت و بستانکار مرتبط'))
            shot(f'M6-original-bill-links-{index}')
            x11.key('Escape');time.sleep(.2)
            x11.key('Escape')
            time.sleep(.2)
            fill('جست‌وجوی شماره، منبع، کالا یا طرف','')
        final = state(profile)
        (run/'M6-main-final-state.json').write_text(json.dumps(final,ensure_ascii=False,indent=2))
        report['final_controls'] = controls(final)
        native.stop(app)
        for appearance in ['light','dark']:
            env['ADW_DEBUG_COLOR_SCHEME'] = 'prefer-'+appearance
            app = start([str(binary)],f'M6-{appearance}',env)
            window = native.wait_for(lambda:x11.find(app.pid),label='restart native owned window')
            x11.focus(window)
            access = Access(app.pid)
            x11.key('8','Alt_L')
            native.wait_for(lambda:access.find('برگشت از خرید',['button','push button']),label='restart purchases')
            check(f'{appearance} restart retains canonical whole state',state(profile) == final)
            for width,height in [(1280,900),(650,550)]:
                x11.resize(window,width,height)
                native.wait_for(lambda:x11.geometry(window) == [width,height],label='native geometry')
                time.sleep(.25)
                shot(f'M6-{appearance}-{width}x{height}')
                target = focus('برگشت از خرید',['button','push button'])
                check(f'{appearance} {width}x{height} real Tab return action',access.focused(target))
                # Reopening and canceling proves fully-returned R-B has no editable quantity.
                open_billed()
                check(f'{appearance} {width}x{height} fully returned specific bill line absent',access.find(quantity_name,['text','entry']) is None)
                shot(f'M6-{appearance}-{width}x{height}-remaining-source')
                x11.key('Escape')
                time.sleep(.25)
            native.stop(app)
        check('restart/layout/cancel performs no extra economic effects',state(profile) == final)
        # Independent reviewer prepares the real separate April2 branch with
        # receipt6/90 followed by bill2/30, rather than editing fixture state.
        profile = run/'unbilled'
        profile.mkdir()
        with sqlite3.connect(f'file:{unbilled_seed}/sarv.sqlite3?mode=ro',uri=True) as src, sqlite3.connect(profile/'sarv.sqlite3') as dst:
            src.backup(dst)
        unbilled_before = state(profile)
        (run/'M6-unbilled-before-state.json').write_text(json.dumps(unbilled_before,ensure_ascii=False,indent=2))
        expected_unbilled = dict(expected,Inventory=512,Payables=-359,Grni=-110,TB=1720)
        actual = controls(unbilled_before)
        check('separate literal April2 unbilled interleave seed',all(actual[k] == v for k,v in expected_unbilled.items()),actual=actual)
        staged = next(r for r in unbilled_before['accounting']['procurement']['records'] if 'Receive' in r['data'] and r['data']['Receive']['total']==90)
        stage_line = staged['data']['Receive']['lines'][0]
        env.update(SARV_DATA_DIR=str(profile),ADW_DEBUG_COLOR_SCHEME='prefer-light')
        app=start([str(binary)],'M6-unbilled',env)
        window=native.wait_for(lambda:x11.find(app.pid),label='separate native unbilled window');x11.focus(window);access=Access(app.pid)
        x11.key('8','Alt_L')
        native.wait_for(lambda:access.find('برگشت از خرید',['button','push button']),label='unbilled purchases')
        press('برگشت از خرید')
        fill('جست‌وجوی منبع، تأمین‌کننده، کالا یا انبار',staged['command']['Receive']['reference'])
        staged_source = native.wait_for(lambda:next((r['name'] for r in access.tree() if r['role'] in ['button','push button'] and 'رسید بدون فاکتور خرید' in r['name'] and staged['command']['Receive']['reference'] in r['name']),None),label='unbilled operational receipt source')
        press(staged_source)
        native.wait_for(lambda:access.find('شرح و دلیل ثبت',['text','entry']),label='unbilled native editor')
        check('unbilled editor excludes supplier credit input',access.find('مرجع بستانکار تأمین‌کننده',['text','entry']) is None)
        fill('تاریخ مؤثر مدنی','2026-04-03');fill('تاریخ سند طرف','2026-04-03')
        fill('شرح و دلیل ثبت','Independent native unbilled original return');fill('شماره یا مرجع سند','PR-M6-NATIVE-UNBILLED')
        fill('مقدار برگشت در واحد پایه • '+stage_line['id'],'1')
        press('پیش‌نمایش ثبت')
        native.wait_for(lambda:tree_has('ماندهٔ کالای دریافت‌شدهٔ تطبیق‌نشده کاهش'),label='unbilled accounting preview')
        check('unbilled preview changes no financial aggregate',state(profile)['accounting']==unbilled_before['accounting']);shot('M6-unbilled-preview')
        press('ثبت نهایی')
        native.wait_for(lambda:any('Return' in r['data'] for r in state(profile)['accounting']['procurement']['records']),label='unbilled durable commit')
        raw=state(profile);actual=controls(raw);expected_unbilled.update(Inventory=497,Grni=-95,TB=1705)
        check('unbilled1/15 independent Inv497/AP359/GRNI95/TB1705 literal',all(actual[k]==v for k,v in expected_unbilled.items()),actual=actual)
        returned=next(r for r in raw['accounting']['procurement']['records'] if 'Return' in r['data'])['data']['Return']
        check('unbilled exact receipt correspondence without credit or bill identity',returned['total']==15 and returned['receipt_id']==staged['id'] and returned['bill_id'] is None and returned['credit_id'] is None and returned['credit_state'] is None and returned['lines'][0]['receipt_line_id']==stage_line['id'])
        check('unbilled original warehouse30/367 and immutable sales',pools(raw)[stage_line['item_id'],stage_line['warehouse_id']] == (30000,367) and raw['accounting']['sales']==unbilled_before['accounting']['sales'])
        (run/'M6-unbilled-final-state.json').write_text(json.dumps(raw,ensure_ascii=False,indent=2))
        shot('M6-unbilled-posted');native.stop(app)
        app=start([str(binary)],'M6-unbilled-restart',env);window=native.wait_for(lambda:x11.find(app.pid),label='unbilled restart');x11.focus(window);access=Access(app.pid);x11.key('8','Alt_L')
        native.wait_for(lambda:access.find('برگشت از خرید',['button','push button']),label='unbilled restart loaded')
        check('unbilled restart exact complete state',state(profile)==raw);shot('M6-unbilled-restart');native.stop(app)
        check('no unexpected X11 errors',not report['x11_errors'],errors=report['x11_errors'])
        seed_after={str(p):hashlib.sha256(json.dumps(state(p),sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()).hexdigest() for p in [seed_path,unbilled_seed]}
        check('Both protected synthetic fixture States remain exact',seed_after==seed_states,hashes=seed_after)
        report['source_after'] = freeze()
        for log in logs:
            log.flush()
        material = []
        for path in run.glob('M6-*.log'):
            for line in path.read_text(errors='replace').splitlines():
                if 'Gtk-CRITICAL' in line or 'did not receive a focus-out event' in line or ('AdwToolbarView' in line and 'exceeds' in line):
                    material.append({'log':path.name,'diagnostic':line})
        check('fatal-critical native processes have no material focus/minimum-height diagnostic',not material,diagnostics=material)
        report['result'] = 'passed'
    except Exception as error:
        report['error'] = str(error)
        if 'access' in locals():
            (run/'failed-access-tree.json').write_text(json.dumps([{k:v for k,v in r.items() if k != 'node'} for r in access.tree()],ensure_ascii=False,indent=2))
        if x11 and 'window' in locals():
            try:
                shot('M6-failure')
            except Exception:
                pass
        raise
    finally:
        (run/'purchase-returns-independent-native.json').write_text(json.dumps(report,ensure_ascii=False,indent=2))
        print(json.dumps({'result':report['result'],'run':str(run),'checks':len(report['checks']),'error':report.get('error')},ensure_ascii=False))
        if x11:
            x11.x.XCloseDisplay(x11.d)
        for process in reversed(processes):
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    process.kill()
        for log in logs:
            log.close()
        portable.mkdir(parents=True,exist_ok=True)
        # Portable evidence excludes executables, SQLite profiles and private
        # display/runtime sockets; the raw run stays under ignored artifacts.
        receipts = {}
        for file in sorted(run.iterdir()):
            if file.is_file() and file.suffix in ['.json','.jsonl','.png','.log','.txt','.csv','.pdf']:
                target = portable/file.name
                shutil.copy2(file,target)
                actual_hash = hashlib.sha256(target.read_bytes()).hexdigest()
                assert actual_hash == hashlib.sha256(file.read_bytes()).hexdigest()
                receipts[file.name] = actual_hash
        (portable/'portable-manifest.json').write_text(json.dumps({'raw_run':str(run),'portable':str(portable),'files':receipts,'result':report['result']},ensure_ascii=False,indent=2)+'\n')


if __name__ == '__main__':
    main()
