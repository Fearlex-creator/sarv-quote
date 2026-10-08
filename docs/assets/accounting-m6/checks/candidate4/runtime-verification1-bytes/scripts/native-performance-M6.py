#!/usr/bin/env python3
"""Bounded M6 original-cost purchase-return observations on frozen synthetic copies.

Activate /workspace/sarv-environment/activate.sh, then use private dbus-run-session.
Fixture generation/build and all diagnostic qualification occur before quiet GO.
This observer never builds, changes source, flushes caches or opens live profiles.
"""
import argparse
import ctypes as C
import hashlib
import json
import math
import os
from pathlib import Path
import re
import sqlite3
import subprocess
import tempfile
import threading
import time
from collections import deque
from importlib.util import module_from_spec, spec_from_file_location

from qa_access import Access

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = Path(__file__).resolve()
SPEC = spec_from_file_location('m6_unchanged_infrastructure', ROOT / 'scripts/native-performance-M4.py')
legacy = module_from_spec(SPEC)
SPEC.loader.exec_module(legacy)
helper, native = legacy.helper, legacy.native
PROTOCOL = 'm6-scoped-v1'
QUANTITY_PREFIX = 'مقدار برگشت در واحد پایه • '
PREVIEW = 'بازبینی مقدار و اثر سند'
POST = 'تأیید و ثبت نسخهٔ بازبینی‌شده'
RETURN = 'برگشت خرید با بهای اصلی'
SEARCH = 'جست‌وجوی منبع، تأمین‌کننده، کالا یا انبار'
NEXT = 'ردیف‌های برگشت بعد'
PREVIOUS = 'ردیف‌های برگشت پیشین'
CONTROL_NAMES = (PREVIEW, POST, RETURN, SEARCH, NEXT, PREVIOUS)
ISOLATION = dict.fromkeys(map(ord, '\u2066\u2067\u2068\u2069\u200e\u200f'), None)
DIGITS = str.maketrans('۰۱۲۳۴۵۶۷۸۹٠١٢٣٤٥٦٧٨٩', '01234567890123456789')


def normalize(text):
    return text.translate(ISOLATION).translate(DIGITS)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')


def quantiles(values):
    if not values:
        return {'n': 0, 'status': 'unmeasured'}
    values = sorted(values)
    return {'n': len(values), **{key: values[math.ceil(len(values) * p) - 1]
            for key, p in [('p50', .5), ('p95', .95), ('p99', .99)]},
            'worst': values[-1], 'method': 'nearest rank'}


def payload(profile):
    with sqlite3.connect(f'file:{profile}/sarv.sqlite3?mode=ro', uri=True) as db:
        return db.execute('SELECT generation,payload FROM state WHERE id=1').fetchone()


def state(profile):
    return json.loads(payload(profile)[1])


def snapshot(manifest, verification):
    expected = manifest.get('frozen_source_provenance', manifest)
    paths = sorted([*ROOT.glob('src/**/*.rs'), ROOT / 'src/style.css', ROOT / 'Cargo.toml', ROOT / 'Cargo.lock'],
                   key=lambda path: path.relative_to(ROOT).as_posix().encode())
    digest, files = hashlib.sha256(), {}
    for path in paths:
        name = path.relative_to(ROOT).as_posix()
        data = path.read_bytes()
        digest.update(name.encode() + b'\0' + data + b'\0')
        files[name] = hashlib.sha256(data).hexdigest()
    assert files == expected['files'] and digest.hexdigest() == expected['sha256'], 'Frozen production source changed'
    observed = {name: sha(ROOT / name) for name in verification}
    assert observed == verification, 'Frozen verification source changed'
    return {'source_sha256': digest.hexdigest(), 'source_files': files, 'verification_files': observed,
            'observer_sha256': sha(SCRIPT)}


def source_guard(dataset):
    observed = {}
    for name, record in dataset['files'].items():
        path = Path(name)
        assert path.is_relative_to(ROOT / 'artifacts/performance-m6'), 'Dataset path outside synthetic allowlist'
        assert path.stat().st_size == record['bytes'] and sha(path) == record['sha256'], 'Prepared dataset changed'
        observed[name] = record['sha256']
    for key in ('seed_state', 'seed_backup'):
        assert sha(Path(dataset[key])) == dataset[key + '_sha256'], 'Protected original seed changed'
    return observed


def walk(access, root, maximum=3000):
    """Fresh bounded current subtree; timed search traverses only at most25 rows."""
    queue, seen, result = deque([root]), set(), []
    while queue:
        node = queue.popleft()
        if not node or node in seen:
            continue
        seen.add(node)
        assert len(seen) <= maximum, 'Declared bounded accessible subtree exceeded'
        access.a.atspi_accessible_clear_cache(node)
        role = access.string(access.a.atspi_accessible_get_role_name, node, None)
        name = access.string(access.a.atspi_accessible_get_name, node, None)
        result.append({'node': node, 'name': name, 'role': role})
        queue.extend(access.children(node))
    return result


def member(access, row, root):
    """Refresh each parent→child edge; retained stale SHOWING leaves cannot pass."""
    node, edges = row['node'], []
    for _ in range(40):
        access.a.atspi_accessible_clear_cache(node)
        states = access.a.atspi_accessible_get_state_set(node)
        if not states or access.a.atspi_state_set_contains(states, 6):  # DEFUNCT
            return False
        if node == root:
            parent = access.a.atspi_accessible_get_parent(node, None)
            return bool(parent and node in access.children(parent))
        parent = access.a.atspi_accessible_get_parent(node, None)
        if not parent:
            return False
        access.a.atspi_accessible_clear_cache(parent)
        if node not in access.children(parent):
            return False
        edges.append((node, parent))
        node = parent
    raise RuntimeError('Current accessible ancestry exceeded bound')


def ready(access, row, root, focus=False):
    return bool(row and member(access, row, root) and legacy.ready(access, row, focus))


def modal(access, anchor):
    node, fallback = anchor['node'], None
    for _ in range(40):
        access.a.atspi_accessible_clear_cache(node)
        role = access.string(access.a.atspi_accessible_get_role_name, node, None)
        if role in ('dialog', 'alert'):
            return node
        parent = access.a.atspi_accessible_get_parent(node, None)
        if not parent:
            break
        parent_role = access.string(access.a.atspi_accessible_get_role_name, parent, None)
        if parent_role in ('frame', 'window', 'application'):
            fallback = node
            break
        node = parent
    assert fallback, 'No owned modal/body ancestor for unique control'
    return fallback


def find(access, name, root=None, roles=None):
    rows = walk(access, root) if root else access.tree()
    return next((row for row in rows if normalize(row['name']) == normalize(name)
                 and (roles is None or row['role'] in roles)), None)


def observe(predicate, label, seconds=30):
    return legacy.observe(predicate, label, seconds)


def focus(access, row, x11):
    legacy.keyboard_focus(access, row, x11)
    observe(lambda: access.focused(row), 'actual native focus')


def press(access, name, x11, root=None):
    row = observe(lambda: find(access, name, root, ('button', 'push button')), name)
    focus(access, row, x11)
    x11.key('space')
    return row


def fill(access, name, text, x11, root=None):
    row = observe(lambda: find(access, name, root, ('entry', 'text')), name)
    focus(access, row, x11)
    x11.key('a', 'Control_L')
    if text:
        x11.type_ascii(text)
    else:
        x11.key('BackSpace')
    observe(lambda: access.text(row) == text, 'exact entered text')
    return row


def phase_records(profile):
    path = profile / 'procurement-performance-phases.jsonl'
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()] if path.exists() else []


def strict_phases(records):
    result = []
    for row in records:
        gaps = row['gaps_ms']
        actual = max(gaps) if gaps else None
        if actual is not None:
            assert abs(actual - row['max_gap_ms']) < 1e-6
            assert len(gaps) == row['ticks'] + 1 and row['interval_ms'] == 16
            assert abs(sum(gaps) - row['elapsed_ms']) < 1e-5
        expected = 'verified' if row['phase'] == 'preview' else 'worker_finished'
        measured = row['operation'] == 'return' and row['result'] == expected and row['ticks'] > 0 and actual is not None
        result.append({**row, 'max_gap_recomputed_ms': actual, 'terminal_gap_ms': gaps[-1] if gaps else None,
                       'budget_ms': 50, 'measurable_accepted': measured,
                       'strict_status': ('pass' if actual <= 50 else 'miss') if measured else 'unmeasured_or_rejected'})
    return result


def raw_controls(raw):
    accounting, balances = raw['accounting'], {}
    for journal in accounting['journals']:
        assert sum(line['debit'] for line in journal['lines']) == sum(line['credit'] for line in journal['lines'])
        for line in journal['lines']:
            identity = line['account']['id']
            balances[identity] = balances.get(identity, 0) + line['debit'] - line['credit']
    mappings = {row['role']: row['account_id'] for row in accounting['core']['setups'][0]['command']['mappings']}
    result = {role: balances.get(identity, 0) for role, identity in mappings.items()}
    result['TB_debit'] = sum(max(v, 0) for v in balances.values())
    result['TB_credit'] = sum(max(-v, 0) for v in balances.values())
    assert result['TB_debit'] == result['TB_credit']
    return result


def original_prefix(before, after):
    for key in before.keys() - {'accounting'}:
        assert before[key] == after[key], f'Original whole-State {key} changed'
    old, current = before['accounting'], after['accounting']
    for key in ('core', 'sales', 'companies', 'periods', 'accounts'):
        assert old[key] == current[key], f'Original {key} changed'
    for key in ('journals', 'requests', 'audit'):
        assert current[key][:len(old[key])] == old[key], f'Original {key} prefix changed'
    for key in ('records', 'requests'):
        assert current['procurement'][key][:len(old['procurement'][key])] == old['procurement'][key]


def literal_return(before, after, recipe, quantity, total):
    original_prefix(before, after)
    records = after['accounting']['procurement']['records'][len(before['accounting']['procurement']['records']):]
    assert len(records) == 1 and set(records[0]['data']) == {'Return'}
    record = records[0]
    data = record['data']['Return']
    billed = recipe['kind'] == 'billed'
    controls_before = raw_controls(before)
    count = recipe['line_count']
    assert controls_before['Inventory'] == 422 + 15 * count
    assert controls_before['Payables'] == -(329 + (15 * count if billed else 0))
    assert controls_before['Grni'] == -(50 + (0 if billed else 15 * count))
    assert controls_before['TB_debit'] == controls_before['TB_credit'] == 1630 + 15 * count
    assert data['total'] == total and data['return_state'] == 'Returned'
    assert bool(data['bill_id']) == billed and bool(data['credit_id']) == billed
    assert data['credit_state'] == ('Posted' if billed else None)
    assert (data['receipt_id'] is None) == billed
    assert data['return_id'] != data['credit_id']
    assert len(data['lines']) == recipe['line_count']
    assert [row['receipt_line_id'] for row in data['lines']] == recipe['receipt_line_ids']
    assert [row['bill_line_id'] for row in data['lines']] == (recipe['bill_line_ids'] if billed else [None] * recipe['line_count'])
    assert all(row['quantity_thousandths'] == quantity and row['value'] == 15 and row['warehouse_id'] == 'W1'
               and row['item_id'] == 'ITEM-A' for row in data['lines'])
    controls = raw_controls(after)
    expected = {'Inventory': 422, 'Payables': -329, 'Grni': -50, 'Receivables': 299,
                'Sales': -199, 'SalesReturns': 100, 'Cogs': 58, 'TB_debit': 1630, 'TB_credit': 1630}
    assert all(controls[key] == wanted for key, wanted in expected.items())
    movements = record['movements']
    assert len(movements) == recipe['line_count']
    for index, movement in enumerate(movements):
        assert movement['item_id'] == 'ITEM-A' and movement['warehouse_id'] == 'W1' and movement['direction'] == 'Out'
        assert movement['quantity_thousandths'] == 1000 and movement['value'] == 15
        assert movement['source_line_id'] == data['lines'][index]['id']
        assert movement['before']['quantity_thousandths'] == (25 + count - index) * 1000
        assert movement['before']['value'] == 292 + 15 * (count - index)
        assert movement['after']['quantity_thousandths'] == (24 + count - index) * 1000
        assert movement['after']['value'] == 292 + 15 * (count - index - 1)
    journals = after['accounting']['journals'][len(before['accounting']['journals']):]
    assert len(journals) == 1 and journals[0]['id'] == record['journal_id']
    assert journals[0]['event'] == 'procurement.return.v1' and journals[0]['date'] == recipe['return_date']
    mappings = {row['role']: row['account_id'] for row in before['accounting']['core']['setups'][0]['command']['mappings']}
    actual_gl = [(line['account']['id'], line['debit'], line['credit']) for line in journals[0]['lines']]
    assert actual_gl == [(mappings['Payables' if billed else 'Grni'], total, 0), (mappings['Inventory'], 0, total)]
    return {'controls_before': controls_before, 'controls_after': controls, 'record': record,
            'exact_new_journal': journals[0], 'all_lines_exact': True, 'original_whole_state_prefix_exact': True}


class Observer:
    def __init__(self, args, report, dataset):
        self.args, self.report, self.dataset = args, report, dataset
        self.processes, self.logs = [], []
        self.display, self.env, self.x11 = None, None, None

    def start(self, command, name, env=None, **kwargs):
        stream = (self.args.output / f'{name}.log').open('x')
        self.logs.append(stream)
        process = subprocess.Popen(command, env=env, stdout=stream, stderr=stream,
                                   start_new_session=True, **kwargs)
        self.processes.append(process)
        return process

    def setup(self):
        with tempfile.TemporaryFile() as stream:
            self.start(['Xvfb', '-displayfd', str(stream.fileno()), '-screen', '0', '1450x1050x24',
                        '-nolisten', 'tcp', '-xkbdir', str(helper.ENVROOT / 'sysroot/usr/share/X11/xkb')],
                       'xvfb', pass_fds=(stream.fileno(),))
            def display_number():
                stream.seek(0)
                return stream.read().decode().strip()
            self.display = ':' + native.wait_for(display_number, label='private X11 display')
        runtime = native.private_accessibility_environment(self.display, self.args.output)
        self.env = {key: value for key, value in os.environ.items() if not key.startswith('SARV_')}
        self.env.update(DISPLAY=self.display, GDK_BACKEND='x11', GSK_RENDERER='cairo',
                        XDG_RUNTIME_DIR=str(runtime), SARV_TEST_INSTANCE='m6-performance-owned',
                        ADW_DEBUG_COLOR_SCHEME='prefer-light', G_DEBUG='fatal-criticals')
        self.start(['openbox', '--sm-disable', '--config-file',
                    str(helper.ENVROOT / 'sysroot/etc/xdg/openbox/rc.xml')], 'openbox', self.env)
        self.x11 = native.X11(self.display)
        Access(0)

    def capture(self, window, path):
        begin = time.perf_counter()
        subprocess.run(['/usr/bin/import', '-display', self.display, '-window', str(window), str(path)], check=True)
        self.report['capture_samples'].append({'path': str(path), 'elapsed_ms': (time.perf_counter() - begin) * 1000})

    def copied(self, name, suffix):
        record = self.dataset['datasets'][name]
        source, target = Path(record['profile']), self.args.output / suffix
        legacy.copy_database(source, target)
        return target, json.loads(Path(record['recipe']).read_text())['recipe']

    def app(self, profile, name, trace=False):
        env = dict(self.env, SARV_DATA_DIR=str(profile))
        if trace:
            env['SARV_PROCUREMENT_PERFORMANCE_TRACE'] = '1'
        app = self.start([str(self.args.binary)], name, env)
        window = native.wait_for(lambda: self.x11.find(app.pid), label='owned app window')
        self.x11.focus(window)
        access = Access(app.pid)
        native.wait_for(lambda: helper.find_name(access, 'ذخیرهٔ پیش‌نویس'), label='usable quotation shell')
        return app, window, access

    def purchases(self, access):
        self.x11.key('8', 'Alt_L')
        observe(lambda: find(access, RETURN, roles=('button', 'push button')), 'Purchases return action')

    def source_picker(self, access):
        press(access, RETURN, self.x11)
        search = observe(lambda: find(access, SEARCH, roles=('entry', 'text')), 'source Search')
        root = modal(access, search)
        status = observe(lambda: next((row for row in walk(access, root)
                         if re.fullmatch(r'\d+ منبع قابل برگشت • صفحهٔ \d+', normalize(row['name']))), None), 'source ready status')
        rows = access.a.atspi_accessible_get_parent(status['node'], None)
        return search, root, rows

    def source_rows(self, access, rows, root):
        result = walk(access, rows, 500)
        statuses = [row for row in result if re.fullmatch(r'\d+ منبع قابل برگشت • صفحهٔ \d+', normalize(row['name']))]
        actions = [row for row in result if row['role'] in ('button', 'push button')
                   and (normalize(row['name']).startswith('صورتحساب ') or normalize(row['name']).startswith('رسید صورتحساب‌نشده '))]
        assert len(actions) <= 25
        assert all(member(access, row, root) for row in [*statuses, *actions])
        return statuses, actions

    def source_result(self, access, search, root, rows, query, expected_ref, page=1):
        if access.text(search) != query or not ready(access, search, root):
            return None
        statuses, actions = self.source_rows(access, rows, root)
        if len(statuses) != 1 or normalize(statuses[0]['name']) != f'60 منبع قابل برگشت • صفحهٔ {page}':
            return None
        if len(actions) != min(25, 60 - 25 * (page - 1)):
            return None
        references = re.findall('\u2068([^\u2069]+)\u2069', actions[0]['name'])
        if expected_ref not in references or not ready(access, actions[0], root):
            return None
        return actions[0]

    def searches(self, name):
        profile, recipe = self.copied(name, name + '-search')
        result = {'result': 'incomplete', 'samples': [], 'filtered_pages': [], 'identity_checks': []}
        self.report['source_picker_search'][name] = result
        app, window, access = self.app(profile, name + '-search')
        baseline = payload(profile)
        self.purchases(access)
        search, root, rows = self.source_picker(access)
        fill(access, SEARCH, 'M6-NO-ELIGIBLE-SOURCE', self.x11, root)
        def zero():
            statuses, actions = self.source_rows(access, rows, root)
            return access.text(search) == 'M6-NO-ELIGIBLE-SOURCE' and len(statuses) == 1 and normalize(statuses[0]['name']) == '0 منبع قابل برگشت • صفحهٔ 1' and not actions
        observe(zero, 'exact zero-result source scope')
        result['zero_result_exact'] = True
        assert payload(profile) == baseline
        expected_groups = {row['query']: row['pages'] for row in recipe['expected_filter_first_references']}
        for index in range(self.args.search_samples):
            query = ('PERF-A', 'PERF-B')[index % 2]
            focus(access, search, self.x11)
            begin = time.perf_counter()
            self.x11.key('a', 'Control_L')
            self.x11.type_ascii(query)
            dispatch = (time.perf_counter() - begin) * 1000
            action = observe(lambda: self.source_result(access, search, root, rows, query, expected_groups[query][0]), 'exact first current source')
            visible = (time.perf_counter() - begin) * 1000
            focus(access, action, self.x11)
            observe(lambda: ready(access, action, root, True), 'current source action usable focus')
            result['samples'].append({'index': index, 'query': query, 'expected_reference': expected_groups[query][0],
                                      'key_dispatch_ms': dispatch, 'correct_visible_ms': visible,
                                      'focused_usable_ms': (time.perf_counter() - begin) * 1000})
            result['identity_checks'].append({'query': query, 'expected_reference': expected_groups[query][0], 'exact_count': 60, 'current_parent_child_membership': True, 'visible_sensitive_geometry': True, 'actual_focus': True})
            assert payload(profile) == baseline, 'Read-only source query changed durable state'
        query = 'PERF-B'
        for page in (2, 3):
            next_button = observe(lambda: find(access, 'منابع بعد', rows, ('button', 'push button')), 'source Next')
            focus(access, next_button, self.x11)
            begin = time.perf_counter()
            self.x11.key('space')
            action = observe(lambda: self.source_result(access, search, root, rows, query, expected_groups[query][page - 1], page), 'exact filtered source page')
            visible = (time.perf_counter() - begin) * 1000
            focus(access, action, self.x11)
            observe(lambda: ready(access, action, root, True), 'filtered source focus')
            result['filtered_pages'].append({'page': page, 'expected_reference': expected_groups[query][page - 1], 'correct_visible_ms': visible, 'focused_usable_ms': (time.perf_counter() - begin) * 1000})
            assert payload(profile) == baseline
        # Exact receipt correspondence corroborates visible source reference after selection.
        selected = next(row for row in recipe['sources_in_posting_order'] if row['reference'] == expected_groups[query][2])
        self.x11.key('space')
        quantity = observe(lambda: find(access, QUANTITY_PREFIX + selected['receipt_line_id'], roles=('entry', 'text')), 'selected source exact original receipt line')
        assert quantity and member(access, quantity, modal(access, quantity))
        result['selected_source_receipt_line_id'] = selected['receipt_line_id']
        self.capture(window, profile / 'selected-source-exact.png')
        self.x11.key('Escape')
        observe(lambda: find(access, RETURN, roles=('button', 'push button')), 'source selection cancel')
        assert payload(profile) == baseline
        result.update(result='measured', visible=quantiles([row['correct_visible_ms'] for row in result['samples']]),
                      focused=quantiles([row['focused_usable_ms'] for row in result['samples']]),
                      application_memory_kib=helper.rss(app.pid), durable_state_unchanged=True)
        native.stop(app)
        print(json.dumps({'source_picker': name, 'focused': result['focused']}, ensure_ascii=False), flush=True)

    def open_source(self, access, reference, receipt_line):
        search, root, rows = self.source_picker(access)
        fill(access, SEARCH, reference, self.x11, root)
        def chosen():
            statuses, actions = self.source_rows(access, rows, root)
            if len(statuses) != 1 or normalize(statuses[0]['name']) != '1 منبع قابل برگشت • صفحهٔ 1' or len(actions) != 1:
                return None
            refs = re.findall('\u2068([^\u2069]+)\u2069', actions[0]['name'])
            return actions[0] if reference in refs and ready(access, actions[0], root) else None
        action = observe(chosen, 'exact one original return source')
        focus(access, action, self.x11)
        begin = time.perf_counter()
        self.x11.key('space')
        quantity = observe(lambda: find(access, QUANTITY_PREFIX + receipt_line, roles=('entry', 'text')), 'native original-source editor')
        editor = modal(access, quantity)
        focus(access, quantity, self.x11)
        observe(lambda: ready(access, quantity, editor, True), 'first source quantity focus')
        return quantity, editor, (time.perf_counter() - begin) * 1000

    def page_state(self, access, editor, receipt_ids, page, expected_text=None):
        rows = walk(access, editor)
        marker_text = f'ردیف‌های برگشت {25 * (page - 1) + 1} تا {min(25 * page, len(receipt_ids))} از {len(receipt_ids)} • صفحهٔ {page}'
        markers = [row for row in rows if normalize(row['name']) == marker_text]
        quantities = [row for row in rows if row['role'] in ('entry', 'text') and row['name'].startswith(QUANTITY_PREFIX)]
        current = receipt_ids[25 * (page - 1):25 * page]
        if len(markers) != 1 or len(quantities) != len(current) or [row['name'] for row in quantities] != [QUANTITY_PREFIX + identity for identity in current]:
            return None
        if not all(member(access, row, editor) for row in [*markers, *quantities]):
            return None
        if expected_text is not None and any(access.text(row) != expected_text for row in quantities):
            return None
        return quantities

    def turn(self, access, editor, receipt_ids, target_page, direction, expected_text=None):
        pager = find(access, NEXT if direction == 'next' else PREVIOUS, editor, ('button', 'push button'))
        focus(access, pager, self.x11)
        begin = time.perf_counter()
        self.x11.key('space')
        quantities = observe(lambda: self.page_state(access, editor, receipt_ids, target_page, expected_text), 'exact bounded current page')
        first = quantities[0]
        if not legacy.ready(access, first):
            access.focus(first)
        observe(lambda: ready(access, first, editor), 'visible current quantity')
        visible = (time.perf_counter() - begin) * 1000
        focus(access, first, self.x11)
        observe(lambda: ready(access, first, editor, True), 'current quantity focused')
        return {'page': target_page, 'direction': direction, 'first_receipt_line_id': receipt_ids[25 * (target_page - 1)], 'attached_quantity_entries': len(quantities),
                'correct_visible_ms': visible, 'focused_usable_ms': (time.perf_counter() - begin) * 1000}

    def preview_pages(self, access, editor, recipe):
        total_lines = len(recipe['receipt_line_ids'])
        count = 2 * total_lines
        result = []
        for page in range(1, math.ceil(count / 25) + 1):
            if page > 1:
                pager = find(access, 'جزئیات برگشت بعد', editor, ('button', 'push button'))
                focus(access, pager, self.x11)
                begin = time.perf_counter()
                self.x11.key('space')
            else:
                begin = time.perf_counter()
            expected = f'جزئیات برگشت {25*(page-1)+1} تا {min(25*page,count)} از {count} • صفحهٔ {page}'
            def current_details():
                marker = find(access, expected, editor)
                if not marker or not member(access, marker, editor):
                    return None
                parent = access.a.atspi_accessible_get_parent(marker['node'], None)
                children = access.children(parent)
                index = children.index(marker['node'])
                if index + 1 >= len(children):
                    return None
                labels = [row for row in walk(access, children[index + 1], 500)
                          if row['role'] in ('label', 'static', 'text') and row['name']]
                start = 25 * (page - 1)
                if len(labels) != min(25, count - start):
                    return None
                for offset, row in enumerate(labels):
                    if not member(access, row, editor):
                        return None
                    absolute = start + offset
                    text = normalize(row['name']).replace('٬', '').replace(',', '')
                    if absolute < total_lines:
                        ids = re.findall('\u2068([^\u2069]+)\u2069', row['name'])
                        if recipe['receipt_line_ids'][absolute] not in ids:
                            return None
                        if recipe['kind'] == 'billed' and recipe['bill_line_ids'][absolute] not in ids:
                            return None
                        if not re.search(r'• مقدار پایه 1\.000 • بهای اصلی 15 ریال • منبع ', text):
                            return None
                    else:
                        movement = absolute - total_lines
                        match = re.search(r'• مقدار قبل (\d+\.\d{3}) • مقدار بعد (\d+\.\d{3}) • ارزش قبل (\d+) ریال • ارزش بعد (\d+) ریال$', text)
                        expected_values = (f'{25 + total_lines - movement}.000', f'{24 + total_lines - movement}.000',
                                           str(292 + 15 * (total_lines - movement)), str(292 + 15 * (total_lines - movement - 1)))
                        if not match or match.groups() != expected_values:
                            return None
                return labels
            labels = observe(current_details, 'exact current preview sources and original pool transitions')
            result.append({'page': page, 'marker': expected, 'attached_detail_labels': len(labels),
                           'current_rows': [row['name'] for row in labels],
                           'observed_ms': (time.perf_counter() - begin) * 1000,
                           'scope': 'current source IDs, quantity/value, exact movement pool transitions and membership; first page inspection only; no pixel/CPU claim'})
        return result

    def workflow(self, name):
        profile, recipe = self.copied(name, name + '-workflow')
        result = {'result': 'incomplete', 'page_samples': [], 'input_samples': [], 'native_phase_records': []}
        self.report['return_workflows'][name] = result
        app, window, access = self.app(profile, name + '-workflow', True)
        before, baseline = state(profile), payload(profile)
        self.purchases(access)
        receipt_ids = recipe['receipt_line_ids']
        quantity, editor, opening = self.open_source(access, recipe['source_reference'], receipt_ids[0])
        result['source_to_first_focused_quantity_ms'] = opening
        fill(access, 'تاریخ مؤثر مدنی', '2026-04-03', self.x11, editor)
        fill(access, 'تاریخ سند طرف', '2026-04-03', self.x11, editor)
        fill(access, 'شرح و دلیل ثبت', f'M6 performance exact original cost {name}', self.x11, editor)
        fill(access, 'شماره یا مرجع سند', f'M6-PERF-RETURN-{name}', self.x11, editor)
        if recipe['kind'] == 'billed':
            fill(access, 'مرجع بستانکار تأمین‌کننده', f'M6-PERF-CREDIT-{name}', self.x11, editor)
        pages = math.ceil(len(receipt_ids) / 25)
        for page in range(1, pages + 1):
            if page > 1:
                result['page_samples'].append(self.turn(access, editor, receipt_ids, page, 'next'))
            quantities = observe(lambda: self.page_state(access, editor, receipt_ids, page), 'bounded editor entries')
            for row in quantities:
                focus(access, row, self.x11)
                begin = time.perf_counter()
                self.x11.key('a', 'Control_L')
                self.x11.type_ascii('1')
                observe(lambda: access.text(row) == '1' and ready(access, row, editor, True), 'exact current quantity input')
                result['input_samples'].append({'receipt_line_id': row['name'][len(QUANTITY_PREFIX):], 'input_ready_ms': (time.perf_counter() - begin) * 1000})
            assert payload(profile) == baseline
        # Deliberately invalid hidden final row: preview must reveal/focus that actual row.
        if pages > 1:
            last = find(access, QUANTITY_PREFIX + receipt_ids[-1], editor, ('entry', 'text'))
            fill(access, last['name'], '2', self.x11, editor)
            for page in range(pages - 1, 0, -1):
                result['page_samples'].append(self.turn(access, editor, receipt_ids, page, 'previous', '1'))
            press(access, PREVIEW, self.x11, editor)
            invalid = observe(lambda: find(access, QUANTITY_PREFIX + receipt_ids[-1], editor, ('entry', 'text')), 'offpage invalid source row revealed')
            observe(lambda: ready(access, invalid, editor, True) and access.text(invalid) == '2', 'actual invalid last row retained/focused')
            assert payload(profile) == baseline
            result['hidden_invalid_last_row_retained_focused'] = True
            self.capture(window, profile / 'invalid-last-row-focused.png')
            fill(access, invalid['name'], '1', self.x11, editor)
            # Verify every raw input retained after error and correction, separately from latency samples.
            for page in range(pages - 1, 0, -1):
                self.turn(access, editor, receipt_ids, page, 'previous', '1')
            result['first_page_input_retained_after_hidden_error'] = True
        result['all_entered_lines'] = len(result['input_samples'])
        result['entered_total_irr'] = 15 * len(receipt_ids)
        result['memory_kib_after_editor_input'] = helper.rss(app.pid)
        total = observe(lambda: next((row for row in walk(access, editor)
                         if normalize(row['name']).startswith('جمع بهای اصلی برگشت: ')), None), 'exact original return total')
        shown = normalize(total['name']).replace('٬', '').replace(',', '')
        assert shown == f"جمع بهای اصلی برگشت: {result['entered_total_irr']} ریال", 'Original total requires exact labeled IRR'
        assert payload(profile) == baseline
        phase_begin = len(phase_records(profile))
        post = find(access, POST, editor, ('button', 'push button'))
        assert not legacy.sensitive(access, post), 'Old verification unexpectedly retained'
        preview = find(access, PREVIEW, editor, ('button', 'push button'))
        focus(access, preview, self.x11)
        begin = time.perf_counter()
        self.x11.key('space')
        observe(lambda: legacy.sensitive(access, post), 'actual return preview enables post')
        result['preview_observed_ms'] = (time.perf_counter() - begin) * 1000
        assert payload(profile) == baseline
        result['preview_pages'] = self.preview_pages(access, editor, recipe)
        self.capture(window, profile / 'last-preview-page.png')
        result['memory_kib_after_preview'] = helper.rss(app.pid)
        focus(access, post, self.x11)
        begin = time.perf_counter()
        self.x11.key('space')
        expected_count = len(before['accounting']['procurement']['records']) + 1
        observe(lambda: len(state(profile)['accounting']['procurement']['records']) == expected_count, 'durable complete return')
        result['durable_post_observed_ms'] = (time.perf_counter() - begin) * 1000
        observe(lambda: any(row['phase'] == 'post' for row in phase_records(profile)[phase_begin:]), 'post phase receipt appended')
        observe(lambda: find(access, RETURN, roles=('button', 'push button')), 'usable Purchases after post')
        result['durable_and_visible_result_ms'] = (time.perf_counter() - begin) * 1000
        after = state(profile)
        result['raw_literal_proof'] = literal_return(before, after, recipe, 1000, result['entered_total_irr'])
        result['native_phase_records'] = phase_records(profile)
        result['strict_phase_checks'] = strict_phases(result['native_phase_records'])
        result['accepted_phase_coverage'] = {phase: sum(row['measurable_accepted'] for row in result['strict_phase_checks'] if row['phase'] == phase)
                                             for phase in ('preview', 'post')}
        result['memory_kib_after_post'] = helper.rss(app.pid)
        result['read_only_page_edit_continuous_heartbeat'] = 'unmeasured; loaded action latencies are not GTK heartbeat traces'
        write(profile / 'before-state.json', before)
        write(profile / 'after-state.json', after)
        self.capture(window, profile / 'posted.png')
        native.stop(app)
        app, window, access = self.app(profile, name + '-restart')
        self.purchases(access)
        assert state(profile) == after, 'Restart changed complete returned State'
        result.update(result='measured', restart_complete_state_exact=True,
                      page_by_direction={direction: {
                          'visible': quantiles([row['correct_visible_ms'] for row in result['page_samples'] if row['direction'] == direction]),
                          'focused': quantiles([row['focused_usable_ms'] for row in result['page_samples'] if row['direction'] == direction])}
                          for direction in ('next', 'previous')})
        native.stop(app)
        print(json.dumps({'workflow': name, 'pages': result['page_by_direction'], 'phases': result['strict_phase_checks']}, ensure_ascii=False), flush=True)

    def startups(self, name):
        profile, recipe = self.copied(name, name + '-startup')
        result = {'result': 'incomplete', 'samples': [], 'recipe': recipe}
        self.report['startup'][name] = result
        baseline = None
        for index in range(self.args.samples + 1):
            memory_samples, stopping = [], threading.Event()
            env = dict(self.env, SARV_DATA_DIR=str(profile))
            begin = time.perf_counter()
            app = self.start([str(self.args.binary)], f'{name}-startup-{index}', env)
            def monitor():
                while not stopping.is_set():
                    memory = helper.rss(app.pid)
                    if memory:
                        memory_samples.append(memory)
                    stopping.wait(.01)
            monitor_thread = threading.Thread(target=monitor, daemon=True)
            monitor_thread.start()
            try:
                window = native.wait_for(lambda: self.x11.find(app.pid), label='owned startup window')
                access = Access(app.pid)
                save = native.wait_for(lambda: helper.find_name(access, 'ذخیرهٔ پیش‌نویس'), label='startup usable Save')
                native.wait_for(lambda: legacy.ready(access, save), label='Save visible/sensitive/allocated')
                self.x11.focus(window)
                original_focus = native.wait_for(lambda: helper.find_focused(access), label='startup initial focus')
                self.x11.key('Tab')
                native.wait_for(lambda: helper.find_focused(access, original_focus['node']), label='real Tab focus response')
                pre_png = (time.perf_counter() - begin) * 1000
                screenshot = profile / f'startup-{index}.png'
                self.capture(window, screenshot)
                duration = (time.perf_counter() - begin) * 1000
                time.sleep(.5)
                memory = helper.rss(app.pid)
                result['samples'].append({'index': index, 'startup_ms': duration, 'interactive_before_png_ms': pre_png,
                                          'steady_rss_kib': memory['VmRSS'], 'kernel_peak_rss_kib': memory['VmHWM'],
                                          'rss_observer_samples': len(memory_samples), 'rss_observer_raw_kib': memory_samples,
                                          'screenshot': str(screenshot)})
                if index == 0:
                    baseline = payload(profile)
            finally:
                stopping.set()
                monitor_thread.join()
                native.stop(app)
            assert payload(profile) == baseline, 'Startup changed initialized durable state'
        rows = result['samples'][1:]
        result.update(result='measured', repeated_startup_ms=quantiles([row['startup_ms'] for row in rows]),
                      initial_rss_kib=quantiles([row['steady_rss_kib'] for row in rows]),
                      initial_hwm_kib=quantiles([row['kernel_peak_rss_kib'] for row in rows]))
        print(json.dumps({'startup': name, 'latency': result['repeated_startup_ms'], 'rss': result['initial_rss_kib']}), flush=True)

    def close(self):
        for process in reversed(self.processes):
            native.stop(process)
        for stream in self.logs:
            stream.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--expect-sha', required=True)
    parser.add_argument('--source-manifest', type=Path, required=True)
    parser.add_argument('--verification-manifest', type=Path, required=True)
    parser.add_argument('--dataset-manifest', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--samples', type=int, default=10)
    parser.add_argument('--search-samples', type=int, default=20)
    parser.add_argument('--pilot', action='store_true')
    args = parser.parse_args()
    args.binary, args.output = args.binary.resolve(), args.output.resolve()
    assert args.output.is_relative_to(ROOT / 'artifacts/performance-m6') and not args.output.exists()
    assert os.environ.get('DBUS_SESSION_BUS_ADDRESS', '').startswith('unix:path=/tmp/dbus-'), 'Use private dbus-run-session'
    assert not any(os.environ.get(name) for name in ('SARV_SALES_FOCUS_QA', 'SARV_SALES_FOCUS_DIAGNOSTIC')), 'Diagnostic focus receipts forbidden'
    if args.pilot:
        args.samples, args.search_samples = 1, 2
    else:
        assert args.samples == 10 and args.search_samples == 20, 'One declared n10/n20 epoch required'
    manifest = json.loads(args.source_manifest.read_text())
    verification = json.loads(args.verification_manifest.read_text())
    if 'files' in verification:
        verification = verification['files']
    assert SCRIPT.relative_to(ROOT).as_posix() in verification, 'Root must freeze this observer'
    dataset = json.loads(args.dataset_manifest.read_text())
    assert dataset['synthetic_only'] and len(dataset['startup_cohorts']) == 6
    before = snapshot(manifest, verification)
    before['binary_sha256'] = sha(args.binary)
    assert before['binary_sha256'] == args.expect_sha == manifest['binary_sha256']
    fixtures_before = source_guard(dataset)
    args.output.mkdir(parents=True)
    report = {'result': 'incomplete', 'synthetic_only': True, 'development_pilot': args.pilot,
              'protocol': PROTOCOL, 'binary': str(args.binary), 'binary_sha256': args.expect_sha,
              'utc': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()), 'provenance_before': before,
              'source_manifest_sha256': sha(args.source_manifest), 'verification_manifest_sha256': sha(args.verification_manifest),
              'dataset_manifest_sha256': sha(args.dataset_manifest), 'dataset_files_before': fixtures_before,
              'startup': {}, 'source_picker_search': {}, 'return_workflows': {}, 'capture_samples': [],
              'hardware': {'lscpu': helper.command('lscpu'), 'os': Path('/etc/os-release').read_text(),
                           'filesystem': helper.command('df', '-T', '/workspace'), 'meminfo': Path('/proc/meminfo').read_text(),
                           'cgroup_cpu_max': Path('/sys/fs/cgroup/cpu.max').read_text().strip(),
                           'cgroup_memory_max': Path('/sys/fs/cgroup/memory.max').read_text().strip(),
                           'loadavg_before': Path('/proc/loadavg').read_text().strip(),
                           'cgroup_cpu_stat_before': Path('/sys/fs/cgroup/cpu.stat').read_text(),
                           'screen': '1450x1050x24 private Xvfb/Openbox, X11/Cairo, scale1',
                           'rust': helper.command('rustc', '--version'), 'gtk_libadwaita': helper.command('pkg-config', '--modversion', 'gtk4', 'libadwaita-1')},
              'boundaries': {'startup': 'Popen to owned mapped window, usable Save, real Tab focus response and completed PNG; pre-PNG/capture separate. Initial stopped-backup launch n1 separately; repeated n10 new processes with warm/uncontrolled OS pages. No physical OS-cold claim.',
                             'source_picker': 'Actual Ctrl+A/ASCII query to exact first current reference/count/page/action, allocated visible sensitive then actual focus. Current bounded25-row subtree traversal, fresh parent-child membership, debounce/layout/AT-SPI5ms poll included. Initial picker/index discovery and zero warmup outside timer.',
                             'long_editor': 'Actual native pager dispatch through exact page marker/all current source quantity names/text and visible/sensitive allocated first quantity, separately actual focus. At most25 current entries. Main-thread read-only continuous heartbeat unmeasured.',
                             'preview_post': 'Actual real-key preview/post dispatch through usable Post or durable return and visible result. Existing native16ms dispatch-to-render/busy-release phases independently retain every tick/terminal gap; zero-tick/rejected/absent unmeasured. n1 per20billed/20unbilled/200billed workflow.',
                             'memory': 'Application PID only; /proc10ms startup sampling +500ms idle; later search/editor/preview/post RSS/HWM points separate. Helpers excluded.'},
              'historical_M5_strict_misses_retained': {'pages_ms': [51.488974,51.002871,51.904600], 'choice_ms':73.664108,'internal_PDF_ms':80.439368,'global_ms':289.749728,'PDF_busy_ms':107.151621,'persistence_busy_ms':96.586120},
              'limits': ['Shared virtual hardware: no dedicated4core8GBSSD, genuine physical-cache-cold or causal Rust-speed/competitor/production/statutory claim.',
                         'No full five-year100k-doc/million-journal/50k-item/10k-party reference profile;200line returns do not establish1000line or posted-ledger capacity.',
                         'n10/n20 p99 equals worst; sequential page samples and n1 commands/memory are descriptive, not independent tail estimates.',
                         'Read-only page/edit GTK heartbeat unmeasured; latency and accessibility readiness do not prove pixels or isolated CPU.',
                         'Typical20line INVOICE, annual reports/import, Windows/Wayland/physical print/screen readers/hardware loss/leaks/professional review remain unmeasured.']}
    observer = Observer(args, report, dataset)
    try:
        observer.setup()
        for name in ('source120-billed', 'source120-unbilled'):
            observer.searches(name)
        for name in ('return20-billed', 'return20-unbilled', 'return200-billed'):
            observer.workflow(name)
        for name in dataset['startup_cohorts']:
            observer.startups(name)
        report['result'] = 'measured'
    except Exception as error:
        report['error'] = repr(error)
        raise
    finally:
        observer.close()
        report['hardware']['loadavg_after'] = Path('/proc/loadavg').read_text().strip()
        report['hardware']['cgroup_cpu_stat_after'] = Path('/sys/fs/cgroup/cpu.stat').read_text()
        report['end_utc'] = time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())
        try:
            report['provenance_after'] = snapshot(manifest, verification)
            report['provenance_after']['binary_sha256'] = sha(args.binary)
            report['dataset_files_after'] = source_guard(dataset)
            report['manifest_sha256_after'] = {key: sha(path) for key, path in (
                ('source_manifest_sha256', args.source_manifest), ('verification_manifest_sha256', args.verification_manifest),
                ('dataset_manifest_sha256', args.dataset_manifest))}
            report['all_provenance_guards_stable'] = (before == report['provenance_after']
                and fixtures_before == report['dataset_files_after']
                and all(report[key] == value for key, value in report['manifest_sha256_after'].items()))
        except Exception as error:
            report['provenance_guard_error'] = repr(error)
            report['all_provenance_guards_stable'] = False
        if not report['all_provenance_guards_stable']:
            report['result'] = 'rejected_changed_provenance'
        write(args.output / 'performance.json', report)
        assert report['all_provenance_guards_stable'], 'Frozen measurement guard changed'


if __name__ == '__main__':
    main()
