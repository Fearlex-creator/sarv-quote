#!/usr/bin/env python3
"""Bounded M6 original-cost purchase-return observations on frozen synthetic copies.

Activate /workspace/sarv-environment/activate.sh, then use private dbus-run-session.
Fixture generation/build and all diagnostic qualification occur before quiet GO.
This observer never builds, changes source, flushes caches or opens live profiles.
"""
import argparse
import ctypes as C
import faulthandler
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

# Enable before importing any native accessibility/X11 infrastructure. Fatal
# native signals can bypass Python finally; the external coordinator owns cleanup.
faulthandler.enable(all_threads=True)

from qa_access import Access

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = Path(__file__).resolve()
SPEC = spec_from_file_location('m6_unchanged_infrastructure', ROOT / 'scripts/native-performance-M4.py')
legacy = module_from_spec(SPEC)
SPEC.loader.exec_module(legacy)
helper, native = legacy.helper, legacy.native
PROTOCOL = 'm6-scoped-v3'
QUANTITY_PREFIX = 'مقدار برگشت در واحد پایه • '
PREVIEW = 'پیش‌نمایش ثبت'
POST = 'ثبت نهایی'
RETURN = 'برگشت از خرید'
SEARCH = 'جست‌وجوی منبع، تأمین‌کننده، کالا یا انبار'
NEXT = 'ردیف‌های برگشت بعد'
PREVIOUS = 'ردیف‌های برگشت پیشین'
INVALID_QUANTITY = 'مقدار برگشت باید مثبت و حداکثر ماندهٔ قابل برگشت باشد'
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



def current_node(access, node):
    """Fresh owned live node metadata; state/name/role/parent are never cached."""
    if not node:
        return None
    access.a.atspi_accessible_clear_cache(node)
    states = access.a.atspi_accessible_get_state_set(node)
    if not states or access.a.atspi_state_set_contains(states, 6):
        return None
    if access.a.atspi_accessible_get_process_id(node, None) != access.pid:
        return None
    return {'node': node, 'role': access.string(access.a.atspi_accessible_get_role_name, node, None),
            'name': access.string(access.a.atspi_accessible_get_name, node, None),
            'parent': access.a.atspi_accessible_get_parent(node, None)}


class CurrentPaths:
    """Qualified identity paths only; all state/edges are refreshed each poll.

    Shared parents are read once per snapshot, never shared between snapshots.
    The root's current attachment and owned PID are checked as well as leaves.
    """
    def __init__(self, access, root, nodes):
        self.root, self.edges, self.roles = root, {}, {}
        self.validation_prefixes = {}
        self.validation_sibling_roles = {}
        for leaf in [root, *nodes]:
            node = leaf
            for _ in range(40):
                row = current_node(access, node)
                assert row and row['parent'], 'Cannot qualify detached/unowned identity path'
                parent = row['parent']
                old = self.edges.setdefault(node, parent)
                assert old == parent, 'Identity path changed during qualification'
                self.roles[node] = row['role']
                if node == root:
                    parent_row = current_node(access, parent)
                    assert parent_row, 'Owned modal parent is not current'
                    self.roles[parent] = parent_row['role']
                    break
                node = parent
            else:
                raise RuntimeError('Qualified identity path exceeded bound')
        self.stable_children = None
        assert self.snapshot(access) is not None, 'Identity qualification failed current edges'

    def snapshot(self, access):
        rows = {node: current_node(access, node) for node in self.roles}
        if any(not row or row['role'] != self.roles[node] for node, row in rows.items()):
            return None
        children = {}
        for parent in dict.fromkeys(self.edges.values()):
            access.a.atspi_accessible_clear_cache(parent)
            children[parent] = tuple(access.children(parent))
        if self.stable_children is not None:
            if children.keys() != self.stable_children.keys():
                return None
            for parent, original in self.stable_children.items():
                current = children[parent]
                if current == original:
                    continue
                label = self.validation_prefixes.get(parent)
                if label is None or current != (label, *original):
                    return None
                row = current_node(access, label)
                if not row or row['parent'] != parent or row['role'] != 'label' or row['name'] != INVALID_QUANTITY:
                    return None
                access.a.atspi_accessible_clear_cache(label)
                if access.children(label) or current_node(access, label) != row:
                    return None
                expected_roles = self.validation_sibling_roles.get(parent, {})
                if len(original) != 16 or set(expected_roles) != set(original):
                    return None
                for sibling in original:
                    sibling_row = current_node(access, sibling)
                    if not sibling_row or sibling_row['parent'] != parent or sibling_row['role'] != expected_roles[sibling]:
                        return None
                    rows[sibling] = sibling_row
                rows[label] = row
        if any(rows[node]['parent'] != parent or node not in children[parent]
               for node, parent in self.edges.items()):
            return None
        return {'rows': rows, 'children': children}

    def coherent(self, before, after):
        # Labels can legitimately rebind between polls, never during an accepted poll.
        return bool(before and after and before == after)

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
    accepted = {'editor': 'preview_requested', 'preview': 'verified', 'post': 'worker_finished'}
    def finite(value):
        return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value) and value >= 0
    for row in records:
        gaps, ticks = row.get('gaps_ms'), row.get('ticks')
        valid_gaps = isinstance(gaps, list) and bool(gaps) and all(finite(value) for value in gaps)
        valid_ticks = isinstance(ticks, int) and not isinstance(ticks, bool) and ticks >= 0
        actual = max(gaps) if valid_gaps else None
        errors = []
        if not valid_gaps:
            errors.append('Missing, nonfinite or negative complete gaps')
        if not valid_ticks:
            errors.append('Ticks must be a nonnegative integer, never bool')
        if type(row.get('interval_ms')) is not int or row['interval_ms'] != 16:
            errors.append('Native interval must be integer16ms')
        if not finite(row.get('elapsed_ms')) or not finite(row.get('max_gap_ms')):
            errors.append('Elapsed/max must be finite and nonnegative')
        if valid_gaps and valid_ticks and len(gaps) != ticks + 1:
            errors.append('Every tick and terminal gap required')
        if valid_gaps and finite(row.get('max_gap_ms')) and abs(actual - row['max_gap_ms']) >= 1e-6:
            errors.append('Recorded max differs from complete gaps')
        if valid_gaps and finite(row.get('elapsed_ms')) and abs(sum(gaps) - row['elapsed_ms']) >= 1e-5:
            errors.append('Complete gap sum differs from elapsed')
        expected = accepted.get(row.get('phase'))
        measured = not errors and expected is not None and row.get('operation') == 'return' and row.get('result') == expected and ticks > 0
        result.append({**row, 'max_gap_recomputed_ms': actual, 'terminal_gap_ms': gaps[-1] if isinstance(gaps, list) and gaps else None,
                       'record_valid': not errors, 'validation_errors': errors,
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


class XErrorEvent(C.Structure):
    _fields_ = [('type', C.c_int), ('display', C.c_void_p), ('resourceid', C.c_ulong),
                ('serial', C.c_ulong), ('error_code', C.c_ubyte),
                ('request_code', C.c_ubyte), ('minor_code', C.c_ubyte)]


class ScopedX11(native.X11):
    """Keep C errors in Python; only retired PID-discovery windows are retried."""
    def __init__(self, display, report, owns_pid):
        super().__init__(display)
        self.report, self.owns_pid, self.scope = report, owns_pid, None
        self.pending_errors = []
        self.x.XSync.restype, self.x.XSync.argtypes = C.c_int, [C.c_void_p, C.c_int]
        self.x.XSetErrorHandler.restype, self.x.XSetErrorHandler.argtypes = C.c_void_p, [C.c_void_p]
        callback_type = C.CFUNCTYPE(C.c_int, C.c_void_p, C.POINTER(XErrorEvent))
        self.error_callback = callback_type(self.record_error)  # Retain callback for the connection lifetime.
        self.previous_handler = self.x.XSetErrorHandler(C.cast(self.error_callback, C.c_void_p))

    @staticmethod
    def retired_pid_window(error):
        scope = error.get('scope') or {}
        return (error['error_code'] == 3 and error['request_code'] == 20 and error['minor_code'] == 0
                and error.get('owned_private_display') is True
                and scope.get('phase') == 'owned-private-window-discovery'
                and scope.get('property') == '_NET_WM_PID'
                and error['resource_id'] == scope.get('window') and scope.get('owned_pid') is not None)

    def record_error(self, display, pointer):
        event = pointer.contents
        row = {'error_code': event.error_code, 'request_code': event.request_code,
               'minor_code': event.minor_code, 'resource_id': event.resourceid, 'serial': event.serial,
               'owned_private_display': display == self.d and event.display == self.d,
               'scope': dict(self.scope) if self.scope else None, 'monotonic_ns': time.monotonic_ns()}
        row['retired_pid_window_rejected'] = self.retired_pid_window(row)
        self.report['x11_error_observations'].append(row)
        self.pending_errors.append(row)
        return 0  # Never raise inside a ctypes C callback.

    def check(self):
        self.x.XSync(self.d, 0)
        pending, self.pending_errors = self.pending_errors, []
        unexpected = [row for row in pending if not row['retired_pid_window_rejected']]
        if unexpected:
            raise RuntimeError('Unexpected observer X11 error: ' + json.dumps(unexpected))
        return pending

    def prop(self, window, name):
        self.check()  # Drain earlier requests before assigning this request's scope.
        previous = self.scope
        if previous:
            self.scope = {**previous, 'property': name, 'window': window}
        try:
            result = super().prop(window, name)
            errors = self.check()
            return [] if errors else result
        finally:
            self.scope = previous

    def find(self, pid):
        assert self.owns_pid(pid), 'Private X11 discovery requires a newly owned application PID'
        self.check()
        self.scope = {'phase': 'owned-private-window-discovery', 'owned_pid': pid}
        try:
            return super().find(pid)
        finally:
            self.scope = None


class Observer:
    def __init__(self, args, report, dataset):
        self.args, self.report, self.dataset = args, report, dataset
        self.processes, self.logs = [], []
        self.display, self.env, self.x11 = None, None, None
        self.source_paths, self.editor_paths = {}, {}

    def checkpoint(self, event):
        """Atomic partial raw report, called only after a declared timing endpoint."""
        self.report['completed_checkpoint_boundaries'].append({'event': event, 'monotonic_ns': time.monotonic_ns()})
        temporary = self.args.output / '.performance.partial.tmp'
        write(temporary, self.report)
        temporary.replace(self.args.output / 'performance.partial.json')

    def start(self, command, name, env=None, **kwargs):
        stream = (self.args.output / f'{name}.log').open('x')
        self.logs.append(stream)
        process = subprocess.Popen(command, env=env, stdout=stream, stderr=stream,
                                   start_new_session=True, **kwargs)
        self.processes.append(process)
        proc_stat = Path(f'/proc/{process.pid}/stat').read_text()
        suffix = proc_stat[proc_stat.rfind(')') + 2:].split()
        record = {'name': name, 'pid': process.pid, 'pgid': int(suffix[2]),
                  'comm': proc_stat[proc_stat.find('(') + 1:proc_stat.rfind(')')],
                  'starttime_ticks': int(suffix[19]), 'command': command}
        self.report['owned_process_records'].append(record)
        # Write before any native discovery. This I/O remains in Popen startup
        # timing, and is never subtracted. The coordinator reads complete lines.
        with (self.args.output / 'owned-processes.jsonl').open('a') as ledger:
            ledger.write(json.dumps(record, ensure_ascii=False) + '\n')
            ledger.flush()
            os.fsync(ledger.fileno())
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
        self.x11 = ScopedX11(self.display, self.report, lambda pid: any(
            process.pid == pid and process.args == [str(self.args.binary)] for process in self.processes))
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
        # Qualify the actual hierarchy outside query/page dispatch intervals.
        discovered = walk(access, rows, 500)
        direct = tuple(access.children(rows))
        source_nodes = [row['node'] for row in discovered if self.source_kind(row)]
        assert source_nodes and all(node in direct for node in source_nodes), 'Source hierarchy is not direct-child qualified'
        paths = CurrentPaths(access, root, [rows, search['node']])
        bar = current_node(access, direct[-1])
        assert bar and bar['role'] not in ('entry', 'text', 'label', 'button', 'push button'), 'Unknown source pager-bar role'
        paths.bar_role = bar['role']
        qualified_shapes = [self.source_action_shape(access, row) for row in discovered if self.source_kind(row) == 'action']
        assert qualified_shapes and all(shape is not None for shape in qualified_shapes), 'Unknown nested source action hierarchy during qualification'
        paths.action_shapes = {tuple((item['path'], item['role']) for item in shape['records']) for shape in qualified_shapes}
        self.source_paths[rows] = paths
        current = observe(lambda: self.source_rows(access, rows, root, 'outside-timer-direct-hierarchy-qualification'), 'qualified direct source rows')
        self.report.setdefault('identity_path_qualifications', []).append({
            'scope': 'source', 'rows_node': hex(rows), 'root_node': hex(root),
            'direct_source_children': len(source_nodes), 'qualified_path_edges': len(paths.edges),
            'outside_dispatch_interval': True})
        return search, root, rows

    @staticmethod
    def source_kind(row):
        name = normalize(row['name'])
        if row['role'] == 'label' and re.fullmatch(r'\d+ منبع قابل برگشت • صفحهٔ \d+', name):
            return 'status'
        if row['role'] in ('button', 'push button') and (name.startswith('فاکتور ') or name.startswith('رسید بدون فاکتور خرید ')):
            return 'action'
        return None

    def source_action_shape(self, access, action):
        """Observed fixed button→same-name childless label; all edges current."""
        button = current_node(access, action['node'])
        if not button or button['role'] not in ('button', 'push button') or button['parent'] != action.get('parent', button['parent']):
            return None
        access.a.atspi_accessible_clear_cache(button['node'])
        children = tuple(access.children(button['node']))
        if len(children) != 1:
            return None
        label = current_node(access, children[0])
        if not label or label['role'] != 'label' or label['parent'] != button['node'] or label['name'] != button['name']:
            return None
        access.a.atspi_accessible_clear_cache(label['node'])
        label_children = tuple(access.children(label['node']))
        if label_children:
            return None
        return {'records': [{**button, 'path': ()}, {**label, 'path': (0,)}],
                'children': [(button['node'], children), (label['node'], label_children)]}

    def source_pager_shape(self, access, bar, paths):
        """Actual qualified GTK FlowBox table→cell→button factory, no shortcuts."""
        if bar['role'] != paths.bar_role or bar['role'] != 'table' or bar['name'] != '':
            return None
        access.a.atspi_accessible_clear_cache(bar['node'])
        cells = tuple(access.children(bar['node']))
        if len(cells) > 2:
            return None
        wrappers, cell_children, buttons, names = [], [], [], []
        for node in cells:
            cell = current_node(access, node)
            if not cell or cell['role'] != 'table cell' or cell['parent'] != bar['node']:
                return None
            access.a.atspi_accessible_clear_cache(node)
            children = tuple(access.children(node))
            if len(children) != 1:
                return None
            button = current_node(access, children[0])
            if not button or button['role'] not in ('button', 'push button') or button['parent'] != node:
                return None
            name = normalize(button['name'])
            if name not in ('منابع پیشین', 'منابع بعد') or cell['name'] != button['name']:
                return None
            shape = self.source_action_shape(access, button)
            if shape is None or tuple((item['path'], item['role']) for item in shape['records']) not in paths.action_shapes:
                return None
            wrappers.append(cell);cell_children.append((node, children));buttons.append(shape);names.append(name)
        if names not in ([], ['منابع پیشین'], ['منابع بعد'], ['منابع پیشین', 'منابع بعد']):
            return None
        return {'bar': bar, 'children': cells, 'wrappers': wrappers,
                'wrapper_children': cell_children, 'button_shapes': buttons, 'pager_names': names}

    def reject_source(self, event, rows, root, reason):
        self.report['rejected_source_observations'].append({
            'event': event, 'monotonic_ns': time.monotonic_ns(), 'reason': reason,
            'root_node': hex(root), 'rows_node': hex(rows)})
        return None

    def source_rows(self, access, rows, root, event, validate=None):
        paths = self.source_paths[rows]
        before = paths.snapshot(access)
        if not before:
            return self.reject_source(event, rows, root, 'Retired/unowned source common path before child read')
        access.a.atspi_accessible_clear_cache(rows)
        direct = tuple(access.children(rows))
        if any(not node for node in direct):
            return self.reject_source(event, rows, root, 'Missing current direct source child during hierarchy rebind')
        records = [current_node(access, node) for node in direct]
        if not before or any(not row or row['parent'] != rows for row in records):
            return self.reject_source(event, rows, root, 'Retired/unowned source child or common path')
        # Actual GTK rows are status, up-to25 direct actions, then one pager bar.
        # Unknown nesting cannot silently hide a source action from this inventory.
        if len(records) < 2 or self.source_kind(records[0]) != 'status' or any(self.source_kind(row) != 'action' for row in records[1:-1]) or self.source_kind(records[-1]):
            return self.reject_source(event, rows, root, 'Direct source hierarchy changed')
        access.a.atspi_accessible_clear_cache(records[0]['node'])
        if access.children(records[0]['node']):
            return self.reject_source(event, rows, root, 'Unknown nested children under status label')
        bar = records[-1]
        pager_shape = self.source_pager_shape(access, bar, paths)
        if pager_shape is None:
            return self.reject_source(event, rows, root, 'Unknown/retired FlowBox table-cell-button pager hierarchy')
        actions = records[1:-1]
        assert len(actions) <= 25
        shapes = [self.source_action_shape(access, row) for row in actions]
        if any(shape is None or tuple((item['path'], item['role']) for item in shape['records']) not in paths.action_shapes for shape in shapes):
            return self.reject_source(event, rows, root, 'Unknown/retired source-action child topology or nested source action')
        if validate is not None and not validate([records[0]], actions):
            return None
        # Re-read every name/role/parent and both child lists after readiness.
        final_records = [current_node(access, node) for node in direct]
        final_pager_shape = self.source_pager_shape(access, bar, paths)
        final_shapes = [self.source_action_shape(access, row) for row in actions]
        access.a.atspi_accessible_clear_cache(records[0]['node'])
        final_status_children = tuple(access.children(records[0]['node']))
        access.a.atspi_accessible_clear_cache(rows)
        final_direct = tuple(access.children(rows))
        if any(not node for node in final_direct):
            return self.reject_source(event, rows, root, 'Missing final direct source child during hierarchy rebind')
        after = paths.snapshot(access)
        if final_status_children or shapes != final_shapes or direct != final_direct or records != final_records or pager_shape != final_pager_shape or not paths.coherent(before, after):
            return self.reject_source(event, rows, root, 'Source/query/status/children changed during observation')
        actions = records[1:-1]
        assert len(actions) <= 25
        return [records[0]], actions

    def source_result(self, access, search, root, rows, query, expected_ref, page=1, focused=False):
        if access.text(search) != query or not ready(access, search, root):
            return None
        def exact(statuses, actions):
            if len(statuses) != 1 or normalize(statuses[0]['name']) != f'60 منبع قابل برگشت • صفحهٔ {page}':
                return False
            if len(actions) != min(25, 60 - 25 * (page - 1)):
                return False
            references = re.findall('\u2068([^\u2069]+)\u2069', actions[0]['name'])
            return expected_ref in references and ready(access, actions[0], root, focused) and access.text(search) == query
        current = self.source_rows(access, rows, root, f'query={query};page={page};reference={expected_ref};focused={focused}', exact)
        if current is None or access.text(search) != query:
            return None
        return current[1][0]

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
            current = self.source_rows(access, rows, root, 'exact-zero-result-warmup')
            if current is None:
                return False
            statuses, actions = current
            return access.text(search) == 'M6-NO-ELIGIBLE-SOURCE' and len(statuses) == 1 and normalize(statuses[0]['name']) == '0 منبع قابل برگشت • صفحهٔ 1' and not actions
        observe(zero, 'exact zero-result source scope')
        result['zero_result_exact'] = True
        assert payload(profile) == baseline
        self.checkpoint(name + ':exact-zero-result-warmup')
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
            observe(lambda: self.source_result(access, search, root, rows, query, expected_groups[query][0], focused=True), 'current exact source action usable focus')
            result['samples'].append({'index': index, 'query': query, 'expected_reference': expected_groups[query][0],
                                      'key_dispatch_ms': dispatch, 'correct_visible_ms': visible,
                                      'focused_usable_ms': (time.perf_counter() - begin) * 1000})
            result['identity_checks'].append({'query': query, 'expected_reference': expected_groups[query][0], 'exact_count': 60, 'current_parent_child_membership': True, 'visible_sensitive_geometry': True, 'actual_focus': True})
            self.checkpoint(f'{name}:completed-query:{index}')
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
            observe(lambda: self.source_result(access, search, root, rows, query, expected_groups[query][page - 1], page, True), 'current exact filtered source focus')
            result['filtered_pages'].append({'page': page, 'expected_reference': expected_groups[query][page - 1], 'correct_visible_ms': visible, 'focused_usable_ms': (time.perf_counter() - begin) * 1000})
            self.checkpoint(f'{name}:completed-filtered-page:{page}')
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
        self.checkpoint(name + ':completed-source-picker')
        native.stop(app)
        print(json.dumps({'source_picker': name, 'focused': result['focused']}, ensure_ascii=False), flush=True)

    def open_source(self, access, reference, receipt_line):
        search, root, rows = self.source_picker(access)
        fill(access, SEARCH, reference, self.x11, root)
        def chosen():
            current = self.source_rows(access, rows, root, f'one-original-source;reference={reference}')
            if current is None:
                return None
            statuses, actions = current
            if len(statuses) != 1 or normalize(statuses[0]['name']) != '1 منبع قابل برگشت • صفحهٔ 1' or len(actions) != 1:
                return None
            refs = re.findall('\u2068([^\u2069]+)\u2069', actions[0]['name'])
            if reference not in refs or not ready(access, actions[0], root) or access.text(search) != reference:
                return None
            final = self.source_rows(access, rows, root, 'final-one-original-source')
            return actions[0] if final == current and access.text(search) == reference else None
        action = observe(chosen, 'exact one original return source')
        focus(access, action, self.x11)
        begin = time.perf_counter()
        self.x11.key('space')
        quantity = observe(lambda: find(access, QUANTITY_PREFIX + receipt_line, roles=('entry', 'text')), 'native original-source editor')
        editor = modal(access, quantity)
        focus(access, quantity, self.x11)
        observe(lambda: ready(access, quantity, editor, True), 'first source quantity focus')
        return quantity, editor, (time.perf_counter() - begin) * 1000

    def qualify_editor(self, access, editor, receipt_ids):
        rows = walk(access, editor)
        quantities = [row for row in rows if row['role'] in ('entry', 'text') and row['name'].startswith(QUANTITY_PREFIX)]
        markers = [row for row in rows if re.fullmatch(r'ردیف‌های برگشت \d+ تا \d+ از \d+ • صفحهٔ \d+', normalize(row['name']))]
        assert len(markers) == 1 and len(quantities) == min(25, len(receipt_ids))
        assert [row['name'] for row in quantities] == [QUANTITY_PREFIX + identity for identity in receipt_ids[:25]]
        nodes = [row['node'] for row in quantities]
        paths = CurrentPaths(access, editor, [markers[0]['node'], *nodes])
        paths.stable_children = paths.snapshot(access)['children']
        validation_parent = paths.edges[markers[0]['node']]
        original = paths.stable_children[validation_parent]
        if len(original) == 16:
            siblings = [current_node(access, node) for node in original]
            assert all(row and row['parent'] == validation_parent for row in siblings), 'Current original16 sibling ownership/edges required'
            paths.validation_sibling_roles[validation_parent] = {row['node']: row['role'] for row in siblings}
        self.editor_paths[editor] = {'paths': paths, 'nodes': nodes, 'marker': markers[0]['node'],
                                    'expected': dict.fromkeys(receipt_ids, '')}
        assert self.page_state(access, editor, receipt_ids, 1) is not None, 'Initial editor binding is not coherent'
        self.report.setdefault('identity_path_qualifications', []).append({
            'scope': 'editor', 'root_node': hex(editor), 'stable_entry_nodes': [hex(node) for node in nodes],
            'qualified_path_edges': len(paths.edges), 'outside_dispatch_interval': True})

    def page_state(self, access, editor, receipt_ids, page, expected_text=None, require_ready=False, focused=False):
        cache = self.editor_paths[editor]
        paths, nodes, marker = cache['paths'], cache['nodes'], cache['marker']
        before = paths.snapshot(access)
        def rejected(reason, **diagnostic):
            self.report.setdefault('rejected_editor_observations', []).append({
                'event': f'page={page};focused={focused}', 'monotonic_ns': time.monotonic_ns(),
                'reason': reason, 'root_node': hex(editor), **diagnostic})
            return None
        if before is None:
            return rejected('Persistent identity path retired/reparented/unowned/DEFUNCT or ordered topology changed')
        current = receipt_ids[25 * (page - 1):25 * page]
        marker_text = f'ردیف‌های برگشت {25 * (page - 1) + 1} تا {min(25 * page, len(receipt_ids))} از {len(receipt_ids)} • صفحهٔ {page}'
        quantities = [before['rows'][node] for node in nodes]
        if normalize(before['rows'][marker]['name']) != marker_text or len(quantities) != len(current) or [row['name'] for row in quantities] != [QUANTITY_PREFIX + identity for identity in current]:
            return rejected('Page marker or ordered source identity rebinding incomplete', marker=before['rows'][marker]['name'], names=[row['name'] for row in quantities])
        texts = [access.text(row) for row in quantities]
        wanted = [cache['expected'][identity] for identity in current]
        if texts != wanted or (expected_text is not None and any(text != expected_text for text in texts)):
            return rejected('Current raw text differs from retained per-source input', observed_text=texts, expected_text=wanted)
        if require_ready and not legacy.ready(access, quantities[0], focused):
            return None
        final_texts = [access.text(row) for row in quantities]
        after = paths.snapshot(access)
        if not paths.coherent(before, after) or texts != final_texts:
            return rejected('Marker/names/text/current edges changed during page observation', observed_text=texts, final_text=final_texts)
        # Exact ordered identities plus per-parent ordered child lists reject mixed
        # persistent widget rebinding; correct complete rebinding is expected.
        return quantities

    def qualify_invalid_quantity_label(self, access, editor, receipt_ids, page):
        """Qualify only the observed factory's one error prefix, outside timers."""
        cache = self.editor_paths[editor]
        paths = cache['paths']
        parent = paths.edges[cache['marker']]
        original = paths.stable_children[parent]
        assert len(original) == 16 and not paths.validation_prefixes, 'One billed factory16-sibling qualification only'
        def attempt():
            owner = current_node(access, parent)
            if not owner or owner['role'] != 'panel' or owner['name'] != '':
                return None
            access.a.atspi_accessible_clear_cache(parent)
            current = tuple(access.children(parent))
            if len(current) != 17 or current[1:] != original or not current[0]:
                return None
            label = current[0]
            row = current_node(access, label)
            if not row or row['role'] != 'label' or row['name'] != INVALID_QUANTITY or row['parent'] != parent:
                return None
            # Registration never replaces/rebaselines any original child list.
            paths.validation_prefixes[parent] = label
            accepted = False
            try:
                before = paths.snapshot(access)
                fields = self.page_state(access, editor, receipt_ids, page)
                last_ready = bool(fields and fields[-1]['name'] == QUANTITY_PREFIX + receipt_ids[-1]
                                  and access.text(fields[-1]) == '2' and ready(access, fields[-1], editor, True)
                                  and access.text(fields[-1]) == '2')
                after = paths.snapshot(access)
                accepted = bool(last_ready and paths.coherent(before, after))
                if accepted:
                    return {'parent_node': hex(parent), 'label_node': hex(label), 'literal_name': INVALID_QUANTITY,
                            'original16_children': [hex(node) for node in original], 'childless_current_owned_label': True,
                            'two_coherent_current_snapshots': True, 'all_ordered_source_names_and_raw_values_checked': True,
                            'outside_latency_intervals': True}
            finally:
                if not accepted:
                    paths.validation_prefixes.pop(parent, None)
            return None
        proof = observe(attempt, 'one exact live invalid-quantity label before unchanged16 siblings')
        self.report.setdefault('validation_prefix_qualifications', []).append(proof)

    def turn(self, access, editor, receipt_ids, target_page, direction, expected_text=None):
        pager = find(access, NEXT if direction == 'next' else PREVIOUS, editor, ('button', 'push button'))
        focus(access, pager, self.x11)
        begin = time.perf_counter()
        self.x11.key('space')
        quantities = observe(lambda: self.page_state(access, editor, receipt_ids, target_page, expected_text), 'exact bounded current page')
        first = quantities[0]
        if not legacy.ready(access, first):
            access.focus(first)
        observe(lambda: self.page_state(access, editor, receipt_ids, target_page, expected_text, True), 'visible exact current quantity page')
        visible = (time.perf_counter() - begin) * 1000
        focus(access, first, self.x11)
        observe(lambda: self.page_state(access, editor, receipt_ids, target_page, expected_text, True, True), 'exact current quantity page focused')
        return {'page': target_page, 'direction': direction, 'first_receipt_line_id': receipt_ids[25 * (target_page - 1)], 'attached_quantity_entries': len(quantities),
                'correct_visible_ms': visible, 'focused_usable_ms': (time.perf_counter() - begin) * 1000}

    def focused_control(self, access, editor):
        focused = [row for row in walk(access, editor) if access.focused(row) and member(access, row, editor)]
        if len(focused) != 1:
            return None
        row = current_node(access, focused[0]['node'])
        if not row or not access.focused(row):
            return None
        ancestry, node = [], row['node']
        for _ in range(40):
            current = current_node(access, node)
            if not current:
                return None
            ancestry.append({'node': hex(node), 'parent': hex(current['parent'] or 0),
                             'name': current['name'], 'role': current['role']})
            if node == editor:
                break
            node = current['parent']
        else:
            raise RuntimeError('Focused control ownership exceeded bound')
        if not member(access, row, editor) or not access.focused(row):
            return None
        return {'node': row['node'], 'name': row['name'], 'role': row['role'],
                'parent': row['parent'], 'ancestry': ancestry}

    def user_wins_invalid_reveal(self, access, editor, receipt_ids, pages, window, profile, baseline, result, name):
        """Functional real-key cancellation probe; excluded from timing sample arrays."""
        probe = {'result': 'incomplete', 'outside_timing_arrays': True, 'retained_invalid_receipt_id': receipt_ids[-1]}
        result['hidden_invalid_user_focus_wins'] = probe
        try:
            for page in range(pages - 1, 0, -1):
                press(access, PREVIOUS, self.x11, editor)
                observe(lambda: self.page_state(access, editor, receipt_ids, page, '1'), 'user-wins preparation exact page')
            preview = find(access, PREVIEW, editor, ('button', 'push button'))
            focus(access, preview, self.x11)
            dispatch_begin = time.perf_counter()
            # XTest.key has no sleeps: actual Tab follows actual Space immediately.
            self.x11.key('space')
            self.x11.key('Tab')
            probe['space_tab_dispatch_ms'] = (time.perf_counter() - dispatch_begin) * 1000
            control = observe(lambda: self.focused_control(access, editor), 'actual unique owned focus after immediate Tab')
            probe['initial_focus'] = {**control, 'node': hex(control['node']), 'parent': hex(control['parent'] or 0)}
            assert all(edge['name'] != QUANTITY_PREFIX + receipt_ids[-1] for edge in control['ancestry']), 'Immediate Tab did not move from invalid quantity'
            time.sleep(.2)
            settled = self.focused_control(access, editor)
            assert settled and settled == control, 'Deferred invalid reveal stole user-selected focus'
            invalid = find(access, QUANTITY_PREFIX + receipt_ids[-1], editor, ('entry', 'text'))
            assert invalid and member(access, invalid, editor) and access.text(invalid) == '2'
            assert self.page_state(access, editor, receipt_ids, pages) is not None and payload(profile) == baseline
            image = profile / 'invalid-user-tab-focus-retained.png'
            self.capture(window, image)
            probe.update(result='qualified', settled_focus={**settled, 'node': hex(settled['node']), 'parent': hex(settled['parent'] or 0)},
                         no_jump_back_after_minimum_s=.2, raw_invalid_text='2', durable_state_unchanged=True, screenshot=str(image),
                         geometry='Actual current accessibility identity; modal window-key GTK geometry unavailable')
            self.checkpoint(name + ':completed-user-wins-invalid-reveal')
        except Exception as error:
            probe.update(result='failed_or_untested', error=repr(error))
            self.checkpoint(name + ':failed-user-wins-invalid-reveal')
            raise

    def preview_pages(self, access, editor, recipe, result, name):
        total_lines = len(recipe['receipt_line_ids'])
        count = 2 * total_lines
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
            self.checkpoint(f'{name}:completed-preview-detail-page:{page}')
        return result

    def workflow(self, name):
        profile, recipe = self.copied(name, name + '-workflow')
        result = {'result': 'incomplete', 'page_samples': [], 'input_samples': [], 'native_phase_records': []}
        self.report['return_workflows'][name] = result
        before, baseline = state(profile), payload(profile)
        write(profile / 'before-state.json', before)
        result.update(profile=str(profile), baseline_generation=baseline[0], baseline_payload_sha256=hashlib.sha256(baseline[1].encode()).hexdigest())
        self.checkpoint(name + ':before-first-return-app-opening')
        app, window, access = self.app(profile, name + '-workflow', True)
        standard_geometry = self.x11.geometry(window)
        origin = self.x11.origin(window)
        assert origin[0] >= 0 and origin[1] >= 0 and origin[0] + standard_geometry[0] <= 1450 and origin[1] + standard_geometry[1] <= 1050, 'Natural standard client must fit the declared display'
        result['standard_client_geometry'] = {'size': standard_geometry, 'origin': list(origin), 'display': [1450, 1050], 'natural_default': True}
        self.purchases(access)
        receipt_ids = recipe['receipt_line_ids']
        quantity, editor, opening = self.open_source(access, recipe['source_reference'], receipt_ids[0])
        result['source_to_first_focused_quantity_ms'] = opening
        self.qualify_editor(access, editor, receipt_ids)
        self.checkpoint(name + ':completed-source-opening')
        fill(access, 'تاریخ مؤثر مدنی', '2026-04-03', self.x11, editor)
        fill(access, 'تاریخ سند طرف', '2026-04-03', self.x11, editor)
        fill(access, 'شرح و دلیل ثبت', f'M6 performance exact original cost {name}', self.x11, editor)
        fill(access, 'شماره یا مرجع سند', f'M6-PERF-RETURN-{name}', self.x11, editor)
        if recipe['kind'] == 'billed':
            fill(access, 'مرجع بستانکار تأمین‌کننده', f'M6-PERF-CREDIT-{name}', self.x11, editor)
        pages = math.ceil(len(receipt_ids) / 25)
        def first_preview_coverage():
            entered = [row['receipt_line_id'] for row in result['input_samples']]
            assert entered == receipt_ids, 'First Preview requires every exact source input in order'
            result['editor_first_preview_coverage'] = {'entered_lines': len(entered), 'receipt_line_ids': entered,
                'all_entered_before_first_preview': True, 'native_scope': 'After ReturnRows::new, before Review setup/dialog presentation/layout, to first Preview dispatch; includes remaining setup/layout, common metadata, all inputs, pagers and inter-action checkpoints; excludes ReturnRows construction and later reveal/preview/post'}
            self.checkpoint(name + ':all-inputs-before-first-preview-dispatch')
        def accepted_editor():
            rows = observe(lambda: [row for row in phase_records(profile) if row['phase'] == 'editor'], 'terminal editor phase receipt')
            checks = strict_phases(rows)
            assert len(checks) == 1 and checks[0]['measurable_accepted'], 'Exactly one positive complete accepted editor phase required'
            result['native_phase_records'] = phase_records(profile)
            result['strict_phase_checks'] = strict_phases(result['native_phase_records'])
        for page in range(1, pages + 1):
            if page > 1:
                result['page_samples'].append(self.turn(access, editor, receipt_ids, page, 'next'))
                self.checkpoint(f'{name}:completed-next-page:{page}')
            quantities = observe(lambda: self.page_state(access, editor, receipt_ids, page), 'bounded editor entries')
            for row in quantities:
                focus(access, row, self.x11)
                begin = time.perf_counter()
                self.x11.key('a', 'Control_L')
                self.x11.type_ascii('1')
                observe(lambda: access.text(row) == '1' and ready(access, row, editor, True), 'exact current quantity input')
                input_ready_ms = (time.perf_counter() - begin) * 1000
                self.editor_paths[editor]['expected'][row['name'][len(QUANTITY_PREFIX):]] = '1'
                assert self.page_state(access, editor, receipt_ids, page) is not None, 'Full current page changed after exact input'
                result['input_samples'].append({'receipt_line_id': row['name'][len(QUANTITY_PREFIX):], 'input_ready_ms': input_ready_ms})
                self.checkpoint(f'{name}:completed-input:{len(result["input_samples"])}')
            assert payload(profile) == baseline
        # Deliberately invalid hidden final row: preview must reveal/focus that actual row.
        if pages > 1:
            last = find(access, QUANTITY_PREFIX + receipt_ids[-1], editor, ('entry', 'text'))
            fill(access, last['name'], '2', self.x11, editor)
            self.editor_paths[editor]['expected'][receipt_ids[-1]] = '2'
            assert self.page_state(access, editor, receipt_ids, pages) is not None
            for page in range(pages - 1, 0, -1):
                result['page_samples'].append(self.turn(access, editor, receipt_ids, page, 'previous', '1'))
                self.checkpoint(f'{name}:completed-previous-page:{page}')
            first_preview_coverage()
            press(access, PREVIEW, self.x11, editor)
            invalid = observe(lambda: find(access, QUANTITY_PREFIX + receipt_ids[-1], editor, ('entry', 'text')), 'offpage invalid source row revealed')
            observe(lambda: ready(access, invalid, editor, True) and access.text(invalid) == '2', 'actual invalid last row retained/focused')
            assert payload(profile) == baseline
            accepted_editor()
            self.qualify_invalid_quantity_label(access, editor, receipt_ids, pages)
            result['hidden_invalid_last_row_retained_focused'] = True
            self.checkpoint(name + ':completed-hidden-invalid-focus')
            settled_begin = time.perf_counter()
            self.capture(window, profile / 'invalid-last-row-focused.png')
            result['hidden_invalid_passive_settled_observations'] = []
            for minimum in (.5, 1., 2.):
                remaining = minimum - (time.perf_counter() - settled_begin)
                if remaining > 0:
                    time.sleep(remaining)
                # Passive only: no keys, focus request or scroll mutation before correction.
                fields = self.page_state(access, editor, receipt_ids, pages)
                assert fields and fields[-1]['name'] == QUANTITY_PREFIX + receipt_ids[-1]
                assert access.text(fields[-1]) == '2' and ready(access, fields[-1], editor, True)
                assert payload(profile) == baseline
                image = profile / f'invalid-last-row-settled-min{minimum:g}s.png'
                self.capture(window, image)
                result['hidden_invalid_passive_settled_observations'].append({
                    'minimum_delay_s': minimum, 'capture_completion_delay_s': time.perf_counter() - settled_begin,
                    'receipt_line_id': receipt_ids[-1], 'raw_text': '2', 'actual_focus': True,
                    'current_membership': True, 'durable_state_unchanged': True,
                    'no_focus_scroll_or_key_mutation': True, 'screenshot': str(image),
                    'pixel_visibility': 'requires independent settled screenshot review; modal GTK geometry unavailable'})
                self.checkpoint(f'{name}:passive-hidden-invalid-settled:{minimum:g}s')
            result['hidden_invalid_narrow_settled_observations'] = []
            self.x11.resize(window, 620, 800)
            self.x11.check()
            observe(lambda: self.x11.geometry(window) == [620, 800], 'actual narrow620x800 app geometry')
            resize_begin = time.perf_counter()
            for minimum in (.5, 1., 2.):
                remaining = minimum - (time.perf_counter() - resize_begin)
                if remaining > 0:
                    time.sleep(remaining)
                fields = self.page_state(access, editor, receipt_ids, pages)
                assert fields and fields[-1]['name'] == QUANTITY_PREFIX + receipt_ids[-1]
                assert access.text(fields[-1]) == '2' and ready(access, fields[-1], editor, True)
                assert payload(profile) == baseline and self.x11.geometry(window) == [620, 800]
                image = profile / f'invalid-last-row-narrow-settled-min{minimum:g}s.png'
                self.capture(window, image)
                result['hidden_invalid_narrow_settled_observations'].append({
                    'minimum_delay_s': minimum, 'capture_completion_delay_s': time.perf_counter() - resize_begin,
                    'geometry': [620, 800], 'receipt_line_id': receipt_ids[-1], 'raw_text': '2',
                    'actual_focus': True, 'current_membership': True, 'durable_state_unchanged': True,
                    'no_focus_scroll_or_key_mutation_after_resize': True, 'screenshot': str(image),
                    'pixel_visibility': 'requires independent settled screenshot review; modal GTK geometry unavailable'})
                self.checkpoint(f'{name}:passive-narrow-hidden-invalid-settled:{minimum:g}s')
            self.x11.resize(window, *standard_geometry)
            self.x11.check()
            observe(lambda: self.x11.geometry(window) == standard_geometry, 'restored exact natural standard client before later timed actions')
            time.sleep(.5)
            assert payload(profile) == baseline
            result['geometry_restored_before_later_timed_actions'] = standard_geometry
            self.checkpoint(name + ':restored-standard-geometry')
            self.user_wins_invalid_reveal(access, editor, receipt_ids, pages, window, profile, baseline, result, name)
            fill(access, invalid['name'], '1', self.x11, editor)
            self.editor_paths[editor]['expected'][receipt_ids[-1]] = '1'
            assert self.page_state(access, editor, receipt_ids, pages) is not None
            # Verify every raw input retained after error and correction, separately from latency samples.
            for page in range(pages - 1, 0, -1):
                self.turn(access, editor, receipt_ids, page, 'previous', '1')
            result['first_page_input_retained_after_hidden_error'] = True
            self.checkpoint(name + ':completed-input-retention-after-correction')
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
        if pages == 1:
            first_preview_coverage()
        begin = time.perf_counter()
        self.x11.key('space')
        observe(lambda: legacy.sensitive(access, post), 'actual return preview enables post')
        result['preview_observed_ms'] = (time.perf_counter() - begin) * 1000
        accepted_editor()
        assert payload(profile) == baseline
        observe(lambda: any(row['phase'] == 'preview' and row['result'] == 'verified'
                            for row in phase_records(profile)[phase_begin:]), 'terminal preview phase before checkpoint')
        self.checkpoint(name + ':completed-preview-endpoint')
        result['preview_pages'] = []
        self.preview_pages(access, editor, recipe, result['preview_pages'], name)
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
        write(profile / 'before-state.json', before)
        write(profile / 'after-state.json', after)
        self.checkpoint(name + ':completed-durable-visible-post')
        result['raw_literal_proof'] = literal_return(before, after, recipe, 1000, result['entered_total_irr'])
        result['native_phase_records'] = phase_records(profile)
        result['strict_phase_checks'] = strict_phases(result['native_phase_records'])
        result['accepted_phase_coverage'] = {phase: sum(row['measurable_accepted'] for row in result['strict_phase_checks'] if row['phase'] == phase)
                                             for phase in ('editor', 'preview', 'post')}
        assert result['accepted_phase_coverage']['editor'] == 1, 'Exactly one accepted editor phase per workflow'
        result['memory_kib_after_post'] = helper.rss(app.pid)
        result['read_only_page_edit_continuous_heartbeat'] = 'Measured only ready editor to first Preview dispatch; later error reveal, edits, preview and post outside this editor scope'
        self.checkpoint(name + ':completed-exact-literal-and-phase-proof')
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
        self.checkpoint(name + ':completed-exact-restart')
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
                                          'natural_client_geometry': self.x11.geometry(window),
                                          'steady_rss_kib': memory['VmRSS'], 'kernel_peak_rss_kib': memory['VmHWM'],
                                          'rss_observer_samples': len(memory_samples), 'rss_observer_raw_kib': memory_samples,
                                          'screenshot': str(screenshot)})
                if index == 0:
                    baseline = payload(profile)
            finally:
                stopping.set()
                monitor_thread.join()
                if result['samples'] and result['samples'][-1]['index'] == index:
                    result['samples'][-1]['rss_observer_raw_kib'] = list(memory_samples)
                    result['samples'][-1]['rss_observer_samples'] = len(memory_samples)
                native.stop(app)
            assert payload(profile) == baseline, 'Startup changed initialized durable state'
            self.checkpoint(f'{name}:completed-startup:{index}')
        rows = result['samples'][1:]
        result.update(result='measured', repeated_startup_ms=quantiles([row['startup_ms'] for row in rows]),
                      initial_rss_kib=quantiles([row['steady_rss_kib'] for row in rows]),
                      initial_hwm_kib=quantiles([row['kernel_peak_rss_kib'] for row in rows]))
        self.checkpoint(name + ':completed-startup-cohort')
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
              'rejected_source_observations': [],
              'x11_error_observations': [], 'completed_checkpoint_boundaries': [], 'owned_process_records': [],
              'hardware': {'lscpu': helper.command('lscpu'), 'os': Path('/etc/os-release').read_text(),
                           'filesystem': helper.command('df', '-T', '/workspace'), 'meminfo': Path('/proc/meminfo').read_text(),
                           'cgroup_cpu_max': Path('/sys/fs/cgroup/cpu.max').read_text().strip(),
                           'cgroup_memory_max': Path('/sys/fs/cgroup/memory.max').read_text().strip(),
                           'loadavg_before': Path('/proc/loadavg').read_text().strip(),
                           'cgroup_cpu_stat_before': Path('/sys/fs/cgroup/cpu.stat').read_text(),
                           'screen': '1450x1050x24 private Xvfb/Openbox, X11/Cairo, scale1',
                           'rust': helper.command('rustc', '--version'), 'gtk_libadwaita': helper.command('pkg-config', '--modversion', 'gtk4', 'libadwaita-1')},
              'boundaries': {'startup': 'Popen to owned mapped window, usable Save, real Tab focus response and completed PNG; pre-PNG/capture separate. Initial stopped-backup launch n1 separately; repeated n10 new processes with warm/uncontrolled OS pages. No physical OS-cold claim.',
                             'source_picker': 'Actual Ctrl+A/ASCII query to exact first current reference/count/page/action, allocated visible sensitive then actual focus. Qualified direct source-child snapshots before/after every observation, fresh parent-child membership, debounce/layout/AT-SPI5ms poll included. Initial picker/index discovery and zero warmup outside timer.',
                             'long_editor': 'Actual native pager dispatch through exact page marker/all current source quantity names/text and visible/sensitive allocated first quantity, separately actual focus. At most25 persistent identity paths qualified outside dispatch, all roles/names/text/marker/current edges refreshed before/after each poll. Natural initial client geometry is recorded, narrow620x800 invalid2 captures are untimed, and exact initial client size restored before later timed actions. Passive standard/narrow invalid2 settled captures require independent pixel review; modal GTK window-key geometry unavailable.',
                             'editor_heartbeat': 'C9 native16ms starts after ReturnRows::new, before Review setup/dialog presentation/layout, and ends at first Preview collect/dispatch with explicit return/preview_requested result. Includes remaining setup/layout, common metadata, all20/200 real-key inputs, Next/Previous and inter-action checkpoint I/O; excludes ReturnRows construction and later invalid reveal/correction/preview/post. Initial gap remains included. Exact ordered input association precedes first dispatch. One positive complete accepted editor trace per workflow, separate50ms maximum; no pooling/subtraction.',
                             'preview_post': 'Actual real-key preview/post dispatch through usable Post or durable return and visible result. Existing native16ms dispatch-to-render/busy-release phases independently retain every tick/terminal gap; zero-tick/rejected/absent unmeasured. n1 per20billed/20unbilled/200billed workflow.',
                             'lifecycle_and_partial_checkpoints': 'm6-scoped-v3: early Python faulthandler plus fsynced exact PID/PGID/starttime/comm/command spawn ledger before native discovery and external identity-verified SIGTERM cleanup. Journaling remains inside startup timing; partial raw JSON checkpoints occur after latency endpoints, including before first return opening. Editor trace deliberately includes inter-action checkpoint I/O. X11 private owned-PID discovery still drains with XSync/rejects only exact retired-window PID-property BadWindow. Native hard-exit cause remains unknown; no reliability/cause claim from diagnostic passes.',
                             'memory': 'Application PID only; /proc10ms startup sampling +500ms idle; later search/editor/preview/post RSS/HWM points separate. Helpers excluded.'},
              'historical_M5_strict_misses_retained': {'pages_ms': [51.488974,51.002871,51.904600], 'choice_ms':73.664108,'internal_PDF_ms':80.439368,'global_ms':289.749728,'PDF_busy_ms':107.151621,'persistence_busy_ms':96.586120},
              'limits': ['Shared virtual hardware: no dedicated4core8GBSSD, genuine physical-cache-cold or causal Rust-speed/competitor/production/statutory claim.',
                         'No full five-year100k-doc/million-journal/50k-item/10k-party reference profile;200line returns do not establish1000line or posted-ledger capacity.',
                         'n10/n20 p99 equals worst; sequential page samples and n1 commands/memory are descriptive, not independent tail estimates.',
                         'Editor GTK heartbeat is bounded to ready editor→first Preview; later error reveal/page/edit and unrelated read-only activity remain unmeasured. Latency and accessibility readiness do not prove pixels or isolated CPU.',
                         'Observer XSync contributes to lifecycle discovery; per-completed-observation atomic checkpoint I/O occurs between actions and may affect subsequent caches/work. Protocol v2 qualifies direct source hierarchy and stable editor identity paths before dispatch; every poll refreshes ownership/DEFUNCT/current edges and all names/roles/text before/after. Reduced discovery RPC is a new observer method, never subtracted from or causally compared with preserved v1.2 misses. Search still includes native input,50ms debounce, rendering and RPC; no isolated product speed inference.',
                         'Typical20line INVOICE, annual reports/import, Windows/Wayland/physical print/screen readers/hardware loss/leaks/professional review remain unmeasured.']}
    observer = Observer(args, report, dataset)
    observer.checkpoint('initial-guarded-report-before-GUI')
    try:
        observer.setup()
        for name in ('source120-billed', 'source120-unbilled'):
            observer.searches(name)
            observer.x11.check()
        for name in ('return20-billed', 'return20-unbilled', 'return200-billed'):
            observer.workflow(name)
            observer.x11.check()
        for name in dataset['startup_cohorts']:
            observer.startups(name)
            observer.x11.check()
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
