#!/usr/bin/env python3
"""Observe an immutable M4 binary using fresh synthetic profiles only.

Activate /workspace/sarv-environment/activate.sh and use dbus-run-session.
No builds, source edits, cache flushes or regular desktop profiles are performed.
Core observer dispatch/readiness matches the historical M3 harness.
The existing M2 scale harness is a separate, unchanged invocation.
Native procurement stage/full forms provide separate durable post and indexed-list observations.
"""
import argparse
import ctypes as C
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import tempfile
import threading
import time
from importlib.util import module_from_spec, spec_from_file_location

from qa_access import Access

ROOT = Path(__file__).resolve().parents[1]
SPEC = spec_from_file_location('m2_observer_helpers', ROOT / 'scripts/accounting-performance.py')
helper = module_from_spec(SPEC)
SPEC.loader.exec_module(helper)
native = helper.native
QUOTE_SOURCE = Path('/workspace/sarv-evidence/native-before-current')


def frozen_source_provenance():
    """Freeze manifest order; historical helper digest is retained separately."""
    paths = sorted([*ROOT.glob('src/**/*.rs'), ROOT / 'src/style.css',
                    ROOT / 'Cargo.toml', ROOT / 'Cargo.lock'],
                   key=lambda path: path.relative_to(ROOT).as_posix().encode('utf-8'))
    digest = hashlib.sha256()
    files = {}
    for path in paths:
        relative = path.relative_to(ROOT).as_posix()
        data = path.read_bytes()
        digest.update(relative.encode('utf-8') + b'\0' + data + b'\0')
        files[relative] = hashlib.sha256(data).hexdigest()
    return {'sha256': digest.hexdigest(), 'files': files,
            'algorithm': 'UTF-8 relative-path byte order; path, NUL, raw bytes, NUL'}


def observer_provenance():
    return {name: helper.sha(ROOT / 'scripts' / name) for name in (
        'native-performance-M4.py', 'native-performance-M2.py', 'accounting-performance.py',
        'qa-native.py', 'qa_access.py')}


def ready(access, row, focus=False):
    if not row:
        return False
    access.a.atspi_accessible_clear_cache(row['node'])
    states = access.a.atspi_accessible_get_state_set(row['node'])
    rect = access.rect(row)
    # AT-SPI sensitive=24, showing=25, visible=30, focused=12.
    return bool(rect and rect['width'] > 0 and rect['height'] > 0 and states
                and all(access.a.atspi_state_set_contains(states, s) for s in (24, 25, 30))
                and (not focus or access.focused(row)))


def sensitive(access, row):
    access.a.atspi_accessible_clear_cache(row['node'])
    states = access.a.atspi_accessible_get_state_set(row['node'])
    return bool(states and access.a.atspi_state_set_contains(states, 24))


def observe(predicate, label, seconds=15):
    """Bounded5ms observer poll; discovery is done before timed dispatch."""
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        result = predicate()
        if result:
            return result
        time.sleep(.005)
    raise RuntimeError(f'Timed out waiting for {label}')


def button(access, name, root=None):
    return helper.first_accessible(access, lambda row:
        access.string(access.a.atspi_accessible_get_name, row['node'], None) == name
        and access.string(access.a.atspi_accessible_get_role_name, row['node'], None) in ('button', 'push button'), root)


def keyboard_focus(access, row, x11):
    """Use real Tab when GTK rejects the component focus request."""
    if access.focus(row) and access.focused(row):
        return
    for _ in range(160):
        if access.focused(row):
            return
        x11.key('Tab')
        time.sleep(.01)
    raise RuntimeError('Native Tab did not focus the pre-discovered search control')


def text_control(access, name):
    return helper.first_accessible(access, lambda row:
        access.string(access.a.atspi_accessible_get_name, row['node'], None) == name
        and access.string(access.a.atspi_accessible_get_role_name, row['node'], None) in ('entry', 'text'))


def phases(profile):
    path = profile / 'core-performance-phases.jsonl'
    if not path.exists():
        return []
    return [json.loads(line) for line in path.read_text().splitlines() if line.endswith('}')]


def copy_database(source, destination):
    destination.mkdir()
    with sqlite3.connect(f'file:{source}/sarv.sqlite3?mode=ro', uri=True) as src:
        with sqlite3.connect(destination / 'sarv.sqlite3') as dst:
            src.backup(dst)


def core_shape(profile):
    state = json.loads(helper.payload(profile)[1])
    accounting = state.get('accounting', {})
    core = accounting.get('core', {})
    result = helper.shape(profile)
    procurement = accounting.get('procurement', {})
    result['procurement'] = {key: len(value) for key, value in procurement.items() if isinstance(value, list)}
    result['core'] = {key: len(value) for key, value in core.items() if isinstance(value, list)}
    if core.get('openings'):
        opening = core['openings'][0]
        command = opening['command']
        result['opening'] = {
            'counts': {key: len(command[key]) for key in ('balances', 'parties', 'units', 'items',
                        'warehouses', 'cash', 'banks', 'open_items', 'advances', 'cheques', 'stock', 'grni')},
            'packs': sum(len(item['packs']) for item in command['items']),
            'import_source': command.get('import_source'),
            'command_sha256': opening['command_sha256'],
            'source_id': command['source_id'], 'journal_id': opening['journal_id'],
            'as_of': command['as_of'],
            'ledger_debit_irr': sum(row['debit'] for row in command['balances']),
            'ledger_credit_irr': sum(row['credit'] for row in command['balances']),
            'stock_value_irr': sum(row['value'] for row in command['stock']),
            'grni_value_irr': sum(row['value'] for row in command['grni']),
        }
    return result



def procurement_phases(profile):
    path = profile / 'procurement-performance-phases.jsonl'
    return [json.loads(line) for line in path.read_text().splitlines() if line.endswith('}')] if path.exists() else []


def procurement_proof(profile, opening_baseline):
    accounting = json.loads(helper.payload(profile)[1])['accounting']
    records = accounting['procurement']['records']
    assert accounting['core'] == opening_baseline, 'Operational workflow changed immutable opening'
    kinds = {kind: [row for row in records if kind in row['data']]
             for kind in ('Resources', 'Order', 'Receive', 'Bill', 'Transfer')}
    assert len(records) == 10 and len(accounting['journals']) == 5
    assert len(kinds['Resources']) == 4 and len(kinds['Order']) == len(kinds['Transfer']) == 1
    assert kinds['Order'][0]['data']['Order']['total'] == 180 and kinds['Order'][0]['journal_id'] is None
    price_source = kinds['Order'][0]['command']['Order']['lines'][0]['price_reference']
    assert all(part in price_source for part in ('catalog=', 'SHA256=', 'SKU=SKU-A',
                                                 'قیمت‌ریال=15', '2026-03-22T09:00:00Z'))
    assert sorted(row['data']['Receive']['total'] for row in kinds['Receive']) == [75, 105]
    assert sorted(row['data']['Bill']['total'] for row in kinds['Bill']) == [30, 150]
    transfer = kinds['Transfer'][0]
    assert transfer['journal_id'] is None and transfer['data']['Transfer']['value'] == 70
    assert transfer['data']['Transfer']['quantity_thousandths'] == 6000 and len(transfer['movements']) == 2
    pools = {row['warehouse_id']: row['after'] for row in transfer['movements']}
    assert pools['W1']['quantity_thousandths'] == 30000 and pools['W1']['value'] == 350
    assert pools['W2']['quantity_thousandths'] == 12000 and pools['W2']['value'] == 130
    mappings = {row['role']: row['account_id'] for row in accounting['core']['setups'][0]['command']['mappings']}
    sums = {}
    for journal in accounting['journals']:
        for line in journal['lines']:
            account_id = line['account']['id']
            sums[account_id] = sums.get(account_id, 0) + line['debit'] - line['credit']
    expected = {'Cash': 251, 'Bank': 300, 'Receivables': 200, 'Inventory': 480,
                'ChequesReceivable': 200, 'Payables': -329, 'ChequesPayable': -100,
                'Grni': -50, 'OwnerCapital': -952}
    expected.update({role: 0 for role in ('SupplierAdvances', 'CustomerAdvances', 'Cogs', 'InventoryLoss',
                                        'Sales', 'SalesReturns', 'OperatingExpense', 'BankFees')})
    assert all(sums.get(mappings[role], 0) == value for role, value in expected.items())
    assert sum(value for value in sums.values() if value > 0) == 1431
    assert -sum(value for value in sums.values() if value < 0) == 1431
    return {'record_count': len(records), 'journal_count': len(accounting['journals']),
            'records_by_kind': {kind: len(rows) for kind, rows in kinds.items()},
            'movement_count': sum(len(row['movements']) for row in records),
            'frozen_price_source': price_source,
            'signed_controls_irr': expected, 'trial_debit_irr': 1431, 'trial_credit_irr': 1431,
            'transfer_pools': pools, 'immutable_opening_equal': True}


def measure_procurement(output, opening, binary, env, start, x11, capture, search_samples, pilot):
    """Native source hooks only; phase timers are independent of smoke sequencing."""
    csv = ('code,name,measure,price,currency,source_at,aliases\n'
           'SKU-A,کالای الف خرید,عدد,15,rial,2026-03-22T09:00:00Z,نمونه الف\n'
           'SKU-B,کالای ب خرید,عدد,20,rial,2026-03-22T09:00:00Z,نمونه ب\n')
    baseline_core = json.loads(helper.payload(opening)[1])['accounting']['core']
    result = {'result': 'incomplete', 'source': 'native forms; actual two-row catalogue import and explicit picker'}
    staged = output / 'procurement-stage'
    copy_database(opening, staged)
    (staged / 'M4-synthetic-prices.csv').write_text(csv)
    env.update(SARV_DATA_DIR=str(staged), SARV_PROCUREMENT_SMOKE='stage', SARV_PROCUREMENT_PERFORMANCE_TRACE='1')
    app = start([str(binary)], 'procurement-stage', env)
    marker = staged / 'procurement-stage-ready.json'
    failure = staged / 'procurement-native-failure.txt'
    native.wait_for(lambda: marker.exists() or failure.exists() or app.poll() is not None,
                    seconds=180, label='native staged PO ready')
    assert marker.exists() and not failure.exists(), 'Native PO staging failed'
    result['stage_manifest'] = json.loads(marker.read_text())
    access = Access(app.pid)
    window = native.wait_for(lambda: x11.find(app.pid), label='owned staged PO window')
    x11.focus(window)
    preview = native.wait_for(lambda: button(access, 'بازبینی مقدار و اثر سند'), label='PO preview')
    post = native.wait_for(lambda: button(access, 'تأیید و ثبت نسخهٔ بازبینی‌شده'), label='PO post')
    assert not sensitive(access, post), 'Staged PO unexpectedly already verified'
    baseline = helper.payload(staged)
    phase_start = len(procurement_phases(staged))
    keyboard_focus(access, preview, x11)
    begin = time.perf_counter()
    x11.key('space')
    observe(lambda: sensitive(access, post), label='new PO preview usable')
    result['order_preview_observed_ms'] = (time.perf_counter() - begin) * 1000
    assert helper.payload(staged) == baseline, 'PO preview wrote durable state'
    capture(window, staged / 'external-order-preview.png')
    keyboard_focus(access, post, x11)
    begin = time.perf_counter()
    x11.key('space')
    def order_committed():
        accounting = json.loads(helper.payload(staged)[1])['accounting']
        records = accounting['procurement']['records']
        return len(records) == 5 and len(accounting['journals']) == 1 and sum('Order' in row['data'] for row in records) == 1
    observe(order_committed, label='one durable nonfinancial PO')
    result['order_post_durable_observed_ms'] = (time.perf_counter() - begin) * 1000
    if not pilot:
        observe(lambda: any(row['phase'] == 'post' for row in procurement_phases(staged)[phase_start:]),
                label='external PO phase record', seconds=5)
    time.sleep(.5)
    result['external_order_phase_records'] = procurement_phases(staged)[phase_start:]
    staged_state = json.loads(helper.payload(staged)[1])['accounting']
    staged_order = [row for row in staged_state['procurement']['records'] if 'Order' in row['data']]
    assert len(staged_order) == 1 and staged_order[0]['journal_id'] is None
    assert staged_order[0]['data']['Order']['total'] == 180
    result['external_order_durable_proof'] = {
        'record_id': staged_order[0]['id'], 'command_sha256': staged_order[0]['command_sha256'],
        'total_irr': 180, 'journal_id': None, 'records': 5, 'journals': 1}
    result['memory_kib_after_external_order'] = helper.rss(app.pid)
    native.stop(app)
    posted = output / 'procurement-posted-workflow'
    copy_database(opening, posted)
    (posted / 'M4-synthetic-prices.csv').write_text(csv)
    env.update(SARV_DATA_DIR=str(posted), SARV_PROCUREMENT_SMOKE='full')
    app = start([str(binary)], 'procurement-full', env)
    marker = posted / 'procurement-native-verification.json'
    failure = posted / 'procurement-native-failure.txt'
    native.wait_for(lambda: marker.exists() or failure.exists() or app.poll() is not None,
                    seconds=180, label='literal full procurement posted')
    assert marker.exists() and not failure.exists(), 'Full native procurement failed'
    result['full_manifest'] = json.loads(marker.read_text())
    result['independent_raw_proof'] = procurement_proof(posted, baseline_core)
    result['catalog_csv_sha256'] = helper.sha(posted / 'M4-synthetic-prices.csv')
    result['catalog_csv_data_rows'] = 2
    result['full_workflow_phase_records'] = procurement_phases(posted)
    result['strict_phase_heartbeat_checks'] = [
        {'operation': row['operation'], 'phase': row['phase'], 'result': row['result'],
         'ticks': row['ticks'], 'actual_ms': row['max_gap_ms'], 'budget_ms': 50,
         'measurable': row['ticks'] > 0 and row['result'] in ('verified', 'worker_finished'),
         'passes': row['ticks'] > 0 and row['result'] in ('verified', 'worker_finished') and row['max_gap_ms'] <= 50}
        for row in result['external_order_phase_records'] + result['full_workflow_phase_records']]
    result['memory_kib_after_full_workflow'] = helper.rss(app.pid)
    baseline = helper.payload(posted)
    access = Access(app.pid)
    window = native.wait_for(lambda: x11.find(app.pid), label='owned posted procurement window')
    x11.focus(window)
    x11.key('8', 'Alt_L')
    # Whole-page scrolling may clip the heading after an earlier focused row.
    # The unique heading identifies Purchases; scoped Ctrl+F reveals its entry.
    page_heading = native.wait_for(lambda: helper.first_accessible(access, lambda row:
        access.string(access.a.atspi_accessible_get_name, row['node'], None)
        == 'خرید • سفارش، رسید جزئی و تطبیق صورتحساب'),
        label='unique Purchases page anchor')
    page_root = access.a.atspi_accessible_get_parent(page_heading['node'], None)
    assert page_root, 'Purchases page container absent'
    def visible_name(name, roles=None):
        return helper.first_accessible(access, lambda row:
            access.string(access.a.atspi_accessible_get_name, row['node'], None) == name
            and (not roles or access.string(access.a.atspi_accessible_get_role_name, row['node'], None) in roles)
            and ready(access, row), page_root)
    x11.key('f', 'Control_L')
    search = native.wait_for(lambda: visible_name('جست‌وجوی شماره، منبع، کالا یا طرف', ('entry', 'text')),
                            label='visible procurement search')
    def focus_search():
        x11.key('f', 'Control_L')
        native.wait_for(lambda: ready(access, search), label='current-page search showing and sensitive')
        keyboard_focus(access, search, x11)
        native.wait_for(lambda: ready(access, search, focus=True), label='current-page usable search focus')
    streams = [
        ('document', 'اسناد', [('REC-A', 'REC-A'), ('REC-B', 'REC-B')], True),
        ('party', 'تعریف منابع', [('CU-A', 'CU-A'), ('CU-B', 'CU-B')], False),
        ('item', 'تعریف منابع', [('SKU-A', 'ITEM-A'), ('SKU-B', 'ITEM-B')], False),
        ('mixed_master', 'تعریف منابع', [('CU-A', 'CU-A'), ('SKU-B', 'ITEM-B')], False),
    ]
    result['searches'] = {}
    for name, tab_name, pairs, actionable in streams:
        tab = native.wait_for(lambda: visible_name(tab_name, ('page tab',)), label=f'{name} tab')
        assert access.click(tab)
        focus_search()
        x11.key('a', 'Control_L')
        x11.type_ascii('NO-MATCH-PERFORMANCE')
        def result_count_label(row, count):
            caption = access.string(access.a.atspi_accessible_get_name, row['node'], None)
            return (f' • {count} نتیجه • ' in caption if actionable else
                    caption.startswith(f'{count} منبع •')) and ready(access, row)
        summary = native.wait_for(lambda: helper.first_accessible(access, lambda row:
            result_count_label(row, 0), page_root), label='zero-row search warmup')
        list_root = access.a.atspi_accessible_get_parent(summary['node'], None)
        samples = []
        for index in range(search_samples):
            query, expected = pairs[index % 2]
            focus_search()
            begin = time.perf_counter()
            x11.key('a', 'Control_L')
            x11.type_ascii(query)
            dispatch_ms = (time.perf_counter() - begin) * 1000
            def correct_result():
                if access.text(search) != query:
                    return None
                filtered = helper.first_accessible(access, lambda row:
                    result_count_label(row, 1), list_root)
                if not filtered:
                    return None
                target = helper.first_accessible(access, lambda row:
                    (expected in access.string(access.a.atspi_accessible_get_name, row['node'], None)
                     if actionable else access.string(access.a.atspi_accessible_get_name, row['node'], None).strip('\u2068\u2069') == expected)
                    and ready(access, row), list_root)
                if not target:
                    return None
                if not actionable:
                    return target if access.focused(search) else None
                parent = access.a.atspi_accessible_get_parent(target['node'], None)
                # ActionRow's title/subtitle labels may be nested; climb to its action.
                for _ in range(4):
                    action = button(access, 'جزئیات و خروجی', parent)
                    if action and ready(access, action):
                        return action
                    parent = access.a.atspi_accessible_get_parent(parent, None)
                return None
            target = observe(correct_result, label=f'correct current {name} result')
            visible_ms = (time.perf_counter() - begin) * 1000
            if actionable:
                keyboard_focus(access, target, x11)
                observe(lambda: ready(access, target, focus=True), label='document action native focus')
            samples.append({'index': index, 'query': query, 'expected': expected,
                            'key_dispatch_ms': dispatch_ms, 'correct_visible_ms': visible_ms,
                            'focused_usable_ms': (time.perf_counter() - begin) * 1000})
            assert helper.payload(posted) == baseline, 'Procurement search changed durable state'
        capture(window, posted / f'search-{name}.png')
        result['searches'][name] = {
            'samples': samples, 'correct_visible_ms': helper.stats([row['correct_visible_ms'] for row in samples]),
            'focused_usable_ms': helper.stats([row['focused_usable_ms'] for row in samples]),
            'boundary': 'Visible current document Detail action, then real Tab to focused action' if actionable else
                        'Visible current read-only master ID row while search remains focused; no selectable-row claim',
            'actionable_result': actionable}
    result['memory_kib_after_searches'] = helper.rss(app.pid)
    native.stop(app)
    env.pop('SARV_PROCUREMENT_SMOKE')
    env.pop('SARV_PROCUREMENT_PERFORMANCE_TRACE')
    result['result'] = 'measured'
    return result, posted


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--expect-sha', required=True)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--samples', type=int, default=10)
    parser.add_argument('--search-samples', type=int, default=20)
    parser.add_argument('--pilot', action='store_true', help='Development observer check only: n1 startup/n2 search, not acceptance evidence')
    args = parser.parse_args()
    if args.pilot:
        args.samples, args.search_samples = 1, 2
    else:
        assert args.samples >= 10 and args.search_samples >= 20
    binary = args.binary.resolve()
    assert helper.sha(binary) == args.expect_sha, 'Wrong immutable executable'
    assert os.environ.get('DBUS_SESSION_BUS_ADDRESS', '').startswith('unix:path=/tmp/dbus-'), 'Use private dbus-run-session'
    output = args.output.resolve()
    assert output.is_relative_to(ROOT / 'artifacts/performance-m4'), 'Output must be a fresh ignored performance-m4 directory'
    output.mkdir(parents=True, exist_ok=False)
    processes, logs = [], []
    report = {
        'result': 'incomplete', 'synthetic_only': True, 'development_pilot': args.pilot, 'binary': str(binary),
        'binary_sha256': args.expect_sha, 'utc': helper.command('date', '-u', '+%Y-%m-%dT%H:%M:%SZ'),
        'source_head': helper.command('git', '-C', str(ROOT), 'rev-parse', 'HEAD'),
        'source_status': helper.command('git', '-C', str(ROOT), 'status', '--short'),
        'source_code_sha256': helper.source_hash(),
        'frozen_source_provenance_before': frozen_source_provenance(),
        'observer_provenance_before': observer_provenance(),
        'source_hash_algorithm': 'Sorted pathlib.Path components; relative UTF-8 path, NUL, raw bytes, NUL',
        'matched_core_baseline': 'docs/assets/accounting-m3/performance/performance.json',
        'procurement_measurement': {'result': 'pending native observer contract'}, 'datasets': {},
        'hardware': {
            'lscpu': helper.command('lscpu'), 'os': Path('/etc/os-release').read_text(),
            'loadavg_before': Path('/proc/loadavg').read_text().strip(),
            'cgroup_cpu_stat_before': Path('/sys/fs/cgroup/cpu.stat').read_text(),
            'filesystem': helper.command('df', '-T', '/workspace'),
            'meminfo': Path('/proc/meminfo').read_text(),
            'cgroup_memory_max': Path('/sys/fs/cgroup/memory.max').read_text().strip(),
            'cgroup_cpu_max': Path('/sys/fs/cgroup/cpu.max').read_text().strip(),
            'screen': '1450x1050x24 private Xvfb/Openbox, X11, Cairo, scale 1',
            'rust': helper.command('rustc', '--version'),
            'gtk_libadwaita': helper.command('pkg-config', '--modversion', 'gtk4', 'libadwaita-1'),
        },
        'boundaries': {
            'startup': 'Popen to owned mapped window, usable visible editor Save, real XTest Tab focus roundtrip and completed PNG capture; includes AT-SPI polling and screenshot observer overhead.',
            'first_start': 'One new process on a fresh SQLite backup, initialization/migration may be included; OS cache uncontrolled.',
            'repeated_startup': 'n new processes on the same initialized backup; OS pages warm/uncontrolled, accessibility session initialized. Process-cold, not physical/cache-cold.',
            'core_search': 'Pre-discovery of search/list excluded. Real XTest Ctrl+A and ASCII keys to exact current ID label, then native Tab to focused, visible, sensitive Edit action. Includes debounce, rendering,5ms AT-SPI polling and result-focus navigation; no pure UI latency claim. Correct-visible-action boundary is recorded separately.',
            'preview': 'Real XTest Space on already-discovered focused preview button to enabled post button, after source reason mutation invalidates old verification. Includes accessibility polling; n=1.',
            'post': 'Real XTest Space on already-discovered focused post button to committed SQLite payload containing one opening and one journal; includes durable database observer polling; n=1.',
            'rss': 'Application-only /proc/PID/status KiB; startup sampled every10ms and steady after500ms idle. After-workflow separate point after posting. Helpers excluded.',
        },
        'limits': [
            'No OS cache flush, physical SSD evidence, dedicated reference hardware, Windows or Wayland claim.',
            'Virtual shared executor scheduling and font/library cache state are uncontrolled; hashing and backup warm file pages.',
            'Native forms fixture is distinct from the35-row CSV fixture. No CSV import claim when import_source is absent.',
            'n10/n20 nearest-rank p99 equals worst; n1 preview/post and workflow memory are not tail estimates.',
            'No five-year/100000-document/million-journal-line/10000-party or10000-row staged-import benchmark.',
        ],
    }

    def start(command, name, env=None, **kwargs):
        log = (output / f'{name}.log').open('w')
        logs.append(log)
        process = subprocess.Popen(command, env=env, stdout=log, stderr=log,
                                   start_new_session=True, **kwargs)
        processes.append(process)
        return process

    def capture(window, path):
        subprocess.run(['/usr/bin/import', '-display', display, '-window', str(window), str(path)], check=True)

    def editable(access, row, text):
        a = access.a
        a.atspi_accessible_get_editable_text_iface.argtypes = [C.c_void_p]
        a.atspi_accessible_get_editable_text_iface.restype = C.c_void_p
        a.atspi_editable_text_set_text_contents.argtypes = [C.c_void_p, C.c_char_p, C.c_void_p]
        a.atspi_editable_text_set_text_contents.restype = C.c_int
        iface = a.atspi_accessible_get_editable_text_iface(row['node'])
        assert iface and a.atspi_editable_text_set_text_contents(iface, text.encode(), None)

    try:
        with tempfile.TemporaryFile() as fd:
            start(['Xvfb', '-displayfd', str(fd.fileno()), '-screen', '0', '1450x1050x24',
                   '-nolisten', 'tcp', '-xkbdir', str(helper.ENVROOT / 'sysroot/usr/share/X11/xkb')],
                  'xvfb', pass_fds=(fd.fileno(),))
            def number():
                fd.seek(0)
                return fd.read().decode().strip()
            display = ':' + native.wait_for(number, label='private X11 display')
        runtime = native.private_accessibility_environment(display, output)
        env = {key: value for key, value in os.environ.items() if not key.startswith('SARV_')}
        env.update(DISPLAY=display, GDK_BACKEND='x11', GSK_RENDERER='cairo',
                   XDG_RUNTIME_DIR=str(runtime), SARV_TEST_INSTANCE='m4-performance-owned',
                   SARV_PERFORMANCE_TRACE='1', SARV_CORE_PERFORMANCE_TRACE='1')
        start(['openbox', '--sm-disable', '--config-file',
               str(helper.ENVROOT / 'sysroot/etc/xdg/openbox/rc.xml')], 'openbox', env)
        x11 = native.X11(display)
        Access(0)

        # This opt-in application probe uses native editable forms on a fresh profile.
        workflow = output / 'core-workflow'
        workflow.mkdir()
        env.update(SARV_DATA_DIR=str(workflow), SARV_CORE_SMOKE='stage')
        app = start([str(binary)], 'core-stage', env)
        marker = workflow / 'core-stage-ready.json'
        native.wait_for(lambda: marker.exists() or (workflow / 'core-native-failure.txt').exists(),
                        seconds=110, label='native staged core ready')
        assert marker.exists(), 'Native staging failed; inspect core-native-failure.txt'
        staged = json.loads(marker.read_text())
        assert staged['result'] == 'ready' and staged['preview_total_irr'] == 1251
        report['core_workflow'] = {'stage_manifest': staged}
        baseline = helper.payload(workflow)
        access = Access(app.pid)
        window = native.wait_for(lambda: x11.find(app.pid), label='owned staged window')
        x11.focus(window)
        search = native.wait_for(lambda: helper.find_name(access, 'جستجوی سطرهای جزئیات'), label='core search')
        (workflow / 'staged-accessibility.json').write_text(json.dumps(
            [{key: value for key, value in row.items() if key != 'node'} for row in access.tree()], ensure_ascii=False, indent=2) + '\n')
        native.wait_for(lambda: ready(access, search), label='allocated visible search after stage transition')
        list_heading = native.wait_for(lambda: helper.first_accessible(access, lambda row:
            'فقط پیش‌نویس موقت' in access.string(access.a.atspi_accessible_get_name, row['node'], None)), label='staged list root')
        list_root = access.a.atspi_accessible_get_parent(list_heading['node'], None)
        samples = []
        for index in range(args.search_samples):
            query = ('CU-A', 'CU-B')[index % 2]
            keyboard_focus(access, search, x11)
            native.wait_for(lambda: access.focused(search), label='search focused before timing')
            begin = time.perf_counter()
            x11.key('a', 'Control_L')
            x11.type_ascii(query)
            dispatch_ms = (time.perf_counter() - begin) * 1000
            def result():
                if access.text(search) != query:
                    return None
                filtered = helper.first_accessible(access, lambda candidate:
                    access.string(access.a.atspi_accessible_get_name, candidate['node'], None).startswith('1 سطر • '), list_root)
                if not filtered:
                    return None
                row = helper.first_accessible(access, lambda candidate:
                    access.string(access.a.atspi_accessible_get_name, candidate['node'], None).startswith(query + ' • '), list_root)
                if not row:
                    return None
                card = access.a.atspi_accessible_get_parent(row['node'], None)
                edit = button(access, 'ویرایش سطر', card)
                if edit and not ready(access, edit):
                    access.focus(edit)
                return edit if edit and ready(access, edit) else None
            edit = observe(result, label='correct staged row Edit action')
            observed_ms = (time.perf_counter() - begin) * 1000
            keyboard_focus(access, edit, x11)
            observe(lambda: ready(access, edit, focus=True), label='result edit focus usable')
            samples.append({'index': index, 'query': query, 'key_dispatch_ms': dispatch_ms,
                            'correct_visible_action_ms': observed_ms,
                            'focused_usable_action_ms': (time.perf_counter() - begin) * 1000})
            assert helper.payload(workflow) == baseline, 'Staged lookup wrote financial state'
        report['core_workflow']['search_samples'] = samples
        report['core_workflow']['search_ms'] = helper.stats([s['focused_usable_action_ms'] for s in samples])
        report['core_workflow']['correct_visible_action_ms'] = helper.stats([s['correct_visible_action_ms'] for s in samples])
        report['core_workflow']['search_memory_kib'] = helper.rss(app.pid)
        capture(window, workflow / 'staged-search.png')

        tab = native.wait_for(lambda: helper.find_name(access, 'بازبینی'), label='core Review tab')
        assert access.click(tab)
        preview = native.wait_for(lambda: button(access, 'بررسی تراز و تطبیق کنترل‌ها'), label='preview')
        post = native.wait_for(lambda: button(access, 'ثبت افتتاحیهٔ تطبیق‌شده'), label='post')
        reason = native.wait_for(lambda: text_control(access, 'توضیح منشأ مانده‌ها'), label='source reason')
        # A changed reason prevents an old successful preview from satisfying readiness.
        editable(access, reason, access.text(reason) + ' / synthetic performance observer')
        native.wait_for(lambda: not sensitive(access, post), label='old preview invalidated')
        keyboard_focus(access, preview, x11)
        native.wait_for(lambda: ready(access, preview, focus=True), label='preview focused')
        phase_baseline_count = len(phases(workflow))
        begin = time.perf_counter()
        x11.key('space')
        observe(lambda: sensitive(access, post), label='new preview verified')
        report['core_workflow']['preview_observed_ms'] = (time.perf_counter() - begin) * 1000
        assert helper.payload(workflow) == baseline, 'Preview mutated durable state'
        capture(window, workflow / 'preview-ready.png')
        keyboard_focus(access, post, x11)
        native.wait_for(lambda: ready(access, post, focus=True), label='post focused')
        begin = time.perf_counter()
        x11.key('space')
        def durable():
            state = json.loads(helper.payload(workflow)[1])
            accounting = state['accounting']
            return len(accounting['core']['openings']) == 1 and len(accounting['journals']) == 1
        observe(durable, label='one durable opening and journal')
        report['core_workflow']['post_durable_observed_ms'] = (time.perf_counter() - begin) * 1000
        time.sleep(.5)
        if not args.pilot:
            observe(lambda: any(record['phase'] == 'post' for record in phases(workflow)[phase_baseline_count:]),
                    label='final native phase metrics appended', seconds=5)
        report['core_workflow']['native_phase_records_before_observer'] = phases(workflow)[:phase_baseline_count]
        report['core_workflow']['native_phase_records'] = phases(workflow)[phase_baseline_count:]
        records = report['core_workflow']['native_phase_records']
        heartbeat = {}
        for name, expected_result in [('preview', 'verified'), ('post', 'worker_finished')]:
            matching = [record for record in records if record['phase'] == name]
            record = matching[-1] if matching else None
            measurable = bool(record and record['result'] == expected_result and record['ticks'] > 0)
            heartbeat[name] = {'budget_ms': 50, 'actual_ms': record['max_gap_ms'] if measurable else None,
                'passes': measurable and record['max_gap_ms'] <= 50,
                'measurable': measurable, 'result': record['result'] if record else 'absent'}
        report['core_workflow']['strict_phase_heartbeat_budgets'] = heartbeat
        report['core_workflow']['application_memory_kib_after_workflow'] = helper.rss(app.pid)
        report['core_workflow']['posted_shape'] = core_shape(workflow)
        posted = report['core_workflow']['posted_shape']['opening']
        assert posted['ledger_debit_irr'] == posted['ledger_credit_irr'] == 1251
        assert posted['stock_value_irr'] == 300 and posted['grni_value_irr'] == 50
        assert posted['import_source'] is None, 'Native forms provenance changed; update methodology'
        capture(window, workflow / 'after-post.png')
        native.stop(app)
        env.pop('SARV_CORE_SMOKE')
        env.pop('SARV_CORE_PERFORMANCE_TRACE')
        print(json.dumps({'core_search': report['core_workflow']['search_ms'],
                          'preview_ms': report['core_workflow']['preview_observed_ms'],
                          'post_ms': report['core_workflow']['post_durable_observed_ms']}, ensure_ascii=False), flush=True)

        procurement, procurement_profile = measure_procurement(output, workflow, binary, env, start, x11,
            capture, args.search_samples, args.pilot)
        report['procurement_measurement'] = procurement
        print(json.dumps({'procurement_searches': {name: row['correct_visible_ms'] for name, row in procurement['searches'].items()}}, ensure_ascii=False), flush=True)
        assert json.loads((QUOTE_SOURCE / 'native-responsiveness-verification.json').read_text())['synthetic_only']
        for name, source in [('core-small-posted', workflow), ('quote-scale', QUOTE_SOURCE), ('procurement-small-posted', procurement_profile)]:
            source_before = helper.payload(source)
            profile = output / name
            copy_database(source, profile)
            dataset = {'source': str(source), 'source_shape': core_shape(source), 'samples': []}
            report['datasets'][name] = dataset
            env['SARV_DATA_DIR'] = str(profile)
            for index in range(args.samples + 1):
                assert helper.sha(binary) == args.expect_sha, 'Executable changed'
                observed = []
                stop_sampler = threading.Event()
                begin = time.perf_counter()
                app = start([str(binary)], f'{name}-{index}', env)
                def monitor():
                    while not stop_sampler.is_set():
                        value = helper.rss(app.pid)
                        if value:
                            observed.append(value)
                        stop_sampler.wait(.01)
                sampler = threading.Thread(target=monitor, daemon=True)
                sampler.start()
                try:
                    window = native.wait_for(lambda: x11.find(app.pid), label='owned startup window')
                    access = Access(app.pid)
                    save = native.wait_for(lambda: helper.find_name(access, 'ذخیرهٔ پیش‌نویس'), label='editor Save')
                    native.wait_for(lambda: ready(access, save), label='visible usable editor Save')
                    x11.focus(window)
                    focused = native.wait_for(lambda: helper.find_focused(access), label='initial focus')
                    x11.key('Tab')
                    native.wait_for(lambda: helper.find_focused(access, focused['node']), label='real Tab focus change')
                    screenshot = profile / f'startup-{index}.png'
                    capture(window, screenshot)
                    duration = (time.perf_counter() - begin) * 1000
                    time.sleep(.5)
                    memory = helper.rss(app.pid)
                    dataset['samples'].append({'index': index, 'startup_ms': duration,
                        'steady_rss_kib': memory['VmRSS'], 'kernel_peak_rss_kib': memory['VmHWM'],
                        'rss_observer_samples': len(observed), 'screenshot': str(screenshot)})
                    if index == 0:
                        baseline = helper.payload(profile)
                        dataset['initialized_shape'] = core_shape(profile)
                finally:
                    stop_sampler.set()
                    sampler.join()
                    native.stop(app)
                assert helper.payload(profile) == baseline, 'Startup observer changed initialized state'
            measured = dataset['samples'][1:]
            dataset['repeated_process_startup_ms'] = helper.stats([s['startup_ms'] for s in measured])
            dataset['steady_rss_kib'] = helper.stats([s['steady_rss_kib'] for s in measured])
            dataset['kernel_peak_rss_kib'] = helper.stats([s['kernel_peak_rss_kib'] for s in measured])
            dataset['proposed_startup_budget'] = {'budget_ms': 3000,
                'p95_ms': dataset['repeated_process_startup_ms']['p95'],
                'passes': dataset['repeated_process_startup_ms']['p95'] <= 3000}
            dataset['proposed_initial_rss_budget'] = {'budget_bytes': 250000000,
                'p95_bytes': dataset['steady_rss_kib']['p95'] * 1024,
                'passes': dataset['steady_rss_kib']['p95'] * 1024 <= 250000000}
            dataset['final_shape'] = core_shape(profile)
            assert helper.payload(source) == source_before, 'Synthetic source changed'
            print(json.dumps({'dataset': name, 'startup': dataset['repeated_process_startup_ms'],
                              'rss': dataset['steady_rss_kib']}, ensure_ascii=False), flush=True)
        report['result'] = 'measured'
    except Exception as error:
        report['error'] = repr(error)
        raise
    finally:
        report['hardware']['loadavg_after'] = Path('/proc/loadavg').read_text().strip()
        report['hardware']['cgroup_cpu_stat_after'] = Path('/sys/fs/cgroup/cpu.stat').read_text()
        report['source_code_sha256_after'] = helper.source_hash()
        report['frozen_source_provenance_after'] = frozen_source_provenance()
        report['observer_provenance_after'] = observer_provenance()
        report['observer_stable'] = report['observer_provenance_after'] == report['observer_provenance_before']
        report['binary_sha256_after'] = helper.sha(binary)
        report['frozen_source_stable'] = report['frozen_source_provenance_after'] == report['frozen_source_provenance_before']
        report['binary_stable'] = report['binary_sha256_after'] == args.expect_sha
        if not report['frozen_source_stable'] or not report['binary_stable'] or not report['observer_stable']:
            report['result'] = 'rejected_changed_provenance'
        (output / 'performance.json').write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
        for process in reversed(processes):
            native.stop(process)
        for log in logs:
            log.close()
        assert report['frozen_source_stable'] and report['binary_stable'] and report['observer_stable'], 'Source/executable/observer changed during measurement'


if __name__ == '__main__':
    main()
