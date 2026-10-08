#!/usr/bin/env python3
"""Observe an immutable M5 binary using fresh synthetic profiles only.

Activate /workspace/sarv-environment/activate.sh and use dbus-run-session.
No builds, source edits, cache flushes or regular desktop profiles are performed.
Core observer dispatch/readiness matches the historical M3 harness.
The existing M2 scale harness is a separate, unchanged invocation.
M4 regression boundaries are retained; M5 adds synthetic Sales datasets and observers.
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
FOCUS_FAILURE_OUTPUT = None
EXTERNAL_OBSERVER_PROTOCOL = 'm5-cached-v3'


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
        'native-performance-M5.py', 'native-performance-M4.py', 'native-performance-M2.py', 'accounting-performance.py',
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


class CachedSalesNodes:
    """Owned persistent Sales nodes; never rediscover inside a timed predicate."""
    def __init__(self, access, root, nodes, metadata):
        self.access, self.root, self.nodes = access, root, nodes
        self.calls = {'predicates': 0, 'node_reads': 0, 'parent_checks': 0,
                      'process_checks': 0, 'role_reads': 0, 'name_reads': 0,
                      'state_reads': 0, 'accessible_id_reads': 0,
                      'parent_children_reads': 0, 'membership_edges_checked': 0}
        self.metadata = metadata
        self.roles, self.parents = {}, {}
        self.edges, self.paths = {}, {}
        self.api_id = getattr(access.a, 'atspi_accessible_get_accessible_id', None)
        if self.api_id is not None:
            self.api_id.argtypes = [C.c_void_p, C.c_void_p]
            self.api_id.restype = C.c_void_p
        metadata.update(protocol=EXTERNAL_OBSERVER_PROTOCOL, owned_pid=access.pid,
                        root_node=hex(root), calls=self.calls, predicate_stats={},
                        identity_receipts=[], validity='initializing',
                        api_accessible_id_available=self.api_id is not None,
                        logical_call_counts_not_exact_remote_IPC_counts=True,
                        timed_BFS_fallback=False,
                        lifetime='this owned process/current workspace or dialog only')
        self.root_role = self.role(root)
        self.root_parent = self.parent(root)
        assert self.root_parent, 'Cached current root must have a live exposed parent'
        self.edges[root] = self.root_parent
        for key, row in nodes.items():
            node = row['node']
            self.roles[key], self.parents[key] = self.role(node), self.parent(node)
            assert self.roles[key], f'Cached {key} has no native role'
            current = node
            self.paths[key] = []
            for _ in range(24):
                if current == root:
                    break
                parent = self.parent(current)
                assert parent, f'Cached {key} does not belong to current root'
                self.edges[current] = parent
                self.paths[key].append(current)
                current = parent
            assert current == root, f'Cached {key} ancestry escaped current root'
            self.paths[key].append(root)
        self.check_identity()
        metadata['discovered_nodes'] = {key: {'node': hex(row['node']),
            'parent': hex(self.parents[key]), 'role': self.roles[key],
            'initial_name': self.name(row['node']),
            'initial_accessible_id': self.accessible_id(row['node'])}
            for key, row in nodes.items()}
        metadata['validity'] = 'initialized; runtime transition qualification pending'

    def role(self, node):
        self.calls['role_reads'] += 1
        return self.access.string(self.access.a.atspi_accessible_get_role_name, node, None)

    def name(self, node):
        self.calls['name_reads'] += 1
        return self.access.string(self.access.a.atspi_accessible_get_name, node, None)

    def parent(self, node):
        self.calls['parent_checks'] += 1
        return self.access.a.atspi_accessible_get_parent(node, None)

    def accessible_id(self, node):
        if self.api_id is None:
            return None
        self.calls['accessible_id_reads'] += 1
        return self.access.string(self.api_id, node, None)

    def check_node(self, node):
        self.access.a.atspi_accessible_clear_cache(node)
        self.calls['process_checks'] += 1
        assert self.access.a.atspi_accessible_get_process_id(node, None) == self.access.pid, \
            'Cached Sales node no longer belongs to owned process'
        self.calls['state_reads'] += 1
        states = self.access.a.atspi_accessible_get_state_set(node)
        assert states and not self.access.a.atspi_state_set_contains(states, 6), \
            'Cached Sales node is defunct; timed rediscovery is forbidden'

    def check_identity(self):
        # Memoize direct children within this check only; never across updates.
        parent_children, exposed_edges = {}, {}
        self.check_node(self.root)
        assert self.role(self.root) == self.root_role and self.parent(self.root) == self.root_parent, \
            'Cached Sales current root changed or retired'
        for node, expected_parent in self.edges.items():
            self.check_node(node)
            assert self.parent(node) == expected_parent, 'Cached Sales current parent changed'
            if expected_parent not in parent_children:
                self.access.a.atspi_accessible_clear_cache(expected_parent)
                self.calls['parent_children_reads'] += 1
                parent_children[expected_parent] = set(self.access.children(expected_parent))
            self.calls['membership_edges_checked'] += 1
            exposed_edges[node] = node in parent_children[expected_parent]
        for key, row in self.nodes.items():
            assert self.role(row['node']) == self.roles[key], f'Cached {key} role changed'
        paths = {key: all(exposed_edges[node] for node in path)
                 for key, path in self.paths.items()}
        self.metadata['current_membership'] = {
            'paths_exposed': paths,
            'edges': [{'child': hex(node), 'parent': hex(self.edges[node]), 'currently_exposed': exposed}
                      for node, exposed in exposed_edges.items()],
            'direct_parent_children_memo_lifetime': 'this identity check only'}
        return all(paths.values())

    def read(self, key):
        self.calls['node_reads'] += 1
        return self.name(self.nodes[key]['node'])

    def predicate(self, label, callback, require_exposed=True):
        started = time.perf_counter()
        self.calls['predicates'] += 1
        stats = self.metadata['predicate_stats'].setdefault(label, {'n': 0, 'total_ms': 0, 'worst_ms': 0})
        try:
            exposed = self.check_identity()
            if require_exposed and not exposed:
                return None
            if not ready(self.access, {'node': self.root}):
                return None
            return callback()
        except Exception as error:
            self.metadata['validity'] = 'rejected'
            self.metadata['error'] = repr(error)
            raise
        finally:
            elapsed = (time.perf_counter() - started) * 1000
            stats['n'] += 1
            stats['total_ms'] += elapsed
            stats['worst_ms'] = max(stats['worst_ms'], elapsed)

    def zero_hidden(self):
        def current():
            if self.access.text(self.nodes['search']) != 'NO-MATCH-PERFORMANCE' \
                    or not normalize_digits(self.read('status')).startswith('0 نتیجه •') \
                    or not ready(self.access, self.nodes['status']):
                return False
            states = {}
            for key in ('action', 'title'):
                self.access.a.atspi_accessible_clear_cache(self.nodes[key]['node'])
                self.calls['state_reads'] += 1
                native_states = self.access.a.atspi_accessible_get_state_set(self.nodes[key]['node'])
                if not native_states:
                    return False
                states[key] = {'showing': bool(self.access.a.atspi_state_set_contains(native_states, 25)),
                               'sensitive': bool(self.access.a.atspi_state_set_contains(native_states, 24))}
            paths = self.metadata['current_membership']['paths_exposed']
            hidden = paths['search'] and paths['status'] and not states['action']['sensitive'] \
                and not paths['action'] and not paths['title']
            if hidden:
                self.metadata['zero_result_hide_qualification'] = {
                    'exact_query': 'NO-MATCH-PERFORMANCE', 'matches': 0, 'states': states,
                    'identity_and_current_status_ready': True,
                    'current_paths_exposed': dict(paths),
                    'retained_leaf_showing_flags_are_not_effective_visibility': True}
            return hidden
        return self.predicate('zero_result_current_tree_absence', current, require_exposed=False)

    def receipt(self, label, expected):
        self.check_identity()
        self.metadata['identity_receipts'].append({'label': label, 'expected': expected,
            'names': {key: self.read(key) for key in self.nodes},
            'nodes': {key: hex(row['node']) for key, row in self.nodes.items()},
            'accessible_ids': {key: self.accessible_id(row['node']) for key, row in self.nodes.items()},
            'rectangles': {key: self.access.rect(row) for key, row in self.nodes.items()},
            'current_membership': self.metadata['current_membership']})
        self.metadata['validity'] = 'current root/parents/process/roles checked; transitions retained'


def discover_sales_page_cache(access, page_root, search, metadata):
    """Discover initial populated slot0 outside query/page timers; no extra query."""
    started = time.perf_counter()
    action = observe(lambda: button(access, 'ادامهٔ پیش‌نویس', page_root)
                     or button(access, 'جزئیات و جریان کار', page_root), 'initial persistent Sales slot0')
    card = access.a.atspi_accessible_get_parent(action['node'], None)
    title = observe(lambda: helper.first_accessible(access, lambda row:
        access.string(access.a.atspi_accessible_get_role_name, row['node'], None) in ('label', 'heading')
        and bool(access.string(access.a.atspi_accessible_get_name, row['node'], None)), card),
        'initial persistent Sales title')
    status = observe(lambda: helper.first_accessible(access, lambda row:
        ' نتیجه •' in access.string(access.a.atspi_accessible_get_name, row['node'], None), page_root),
        'persistent Sales status')
    cache = CachedSalesNodes(access, page_root, {'search': search, 'title': title, 'action': action, 'status': status}, metadata)
    metadata['discovery_ms_outside_timers'] = (time.perf_counter() - started) * 1000
    metadata['warmup_effect'] = 'Initial populated slot discovery then original zero-result warmup; no added calibration query.'
    return cache


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
    details = {'pid': access.pid, 'target_name': access.string(access.a.atspi_accessible_get_name, row['node'], None),
        'target_role': access.string(access.a.atspi_accessible_get_role_name, row['node'], None),
        'target_ready': ready(access, row), 'target_rectangle': access.rect(row)}
    if FOCUS_FAILURE_OUTPUT is not None:
        FOCUS_FAILURE_OUTPUT.mkdir(exist_ok=True)
        index = len(list(FOCUS_FAILURE_OUTPUT.glob('*.json')))
        path = FOCUS_FAILURE_OUTPUT / f'focus-failure-{index}.json'
        details['accessible_tree'] = [{key: value for key, value in item.items() if key != 'node'} for item in access.tree()]
        json_write(path, details)
        window = x11.find(access.pid)
        if window:
            subprocess.run(['/usr/bin/import', '-display', os.environ['DISPLAY'], '-window', str(window),
                            str(path.with_suffix('.png'))], check=True)
        details = {key: value for key, value in details.items() if key != 'accessible_tree'}
        details['diagnostics'] = str(path)
    raise RuntimeError('Native Tab did not focus the pre-discovered control: ' + json.dumps(details, ensure_ascii=False))


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
    stock_dates = [opening['command']['as_of'] for opening in accounting['core']['openings']]
    stock_dates += [next(iter(row['command'].values()))['context']['date'] for row in records if row['movements']]
    latest_stock_date = max(stock_dates)
    assert latest_stock_date == '2026-03-27', 'Literal M4 fixture stock cutoff changed'
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
            'transfer_pools': pools, 'latest_operational_stock_date': latest_stock_date,
            'immutable_opening_equal': True}


def measure_procurement(output, opening, binary, env, start, x11, capture, search_samples, pilot, report_target=None):
    """Native source hooks only; phase timers are independent of smoke sequencing."""
    csv = ('code,name,measure,price,currency,source_at,aliases\n'
           'SKU-A,کالای الف خرید,عدد,15,rial,2026-03-22T09:00:00Z,نمونه الف\n'
           'SKU-B,کالای ب خرید,عدد,20,rial,2026-03-22T09:00:00Z,نمونه ب\n')
    baseline_core = json.loads(helper.payload(opening)[1])['accounting']['core']
    result = {'result': 'incomplete', 'source': 'native forms; actual two-row catalogue import and explicit picker'}
    if report_target is not None:
        report_target['procurement_measurement'] = result
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
        result['searches'][name] = {'samples': samples, 'result': 'incomplete'}
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


def json_write(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')


def synthetic_draft(index=0, long=False):
    """Deterministic raw input; context placeholders are resolved on a synthetic clone."""
    group = 'A' if index % 2 == 0 else 'B'
    reference = 'PERF-LONG-1000' if long else f'PERF-{group}-{index:03d}'
    count = 1000 if long else 1
    return {
        'id': f'm5-performance-{reference.lower()}', 'revision': 1,
        'company_id': '@company', 'period_id': '@period', 'setup_id': '@setup',
        'customer_id': 'CU-A' if group == 'A' else 'CU-B', 'reference': reference,
        'document_date': '@date', 'financial_date': '@date', 'date_display': 'Iso',
        'display_unit': 'Rial', 'document_discount': '0', 'supplied_total': str(count * 20),
        'reason': 'دادهٔ مصنوعی آزمون کارایی • بدون ثبت مالی', 'quotation': None,
        'lines': [
            {'id': f'perf-line-{index:03d}-{line:04d}', 'item_id': 'ITEM-A',
             'warehouse_id': 'W1', 'unit_id': 'PIECE', 'quantity': '1', 'unit_price': '20',
             'line_discount': '0', 'price_reference': 'synthetic-performance-seed',
             'price_date': '@date', 'override_reason': 'قیمت مصنوعی مستقل برای آزمون'}
            for line in range(count)],
    }


def prepare_dataset(output):
    """Preparation never starts Sarv or reads a normal desktop data profile."""
    output = output.resolve()
    assert output.is_relative_to(ROOT / 'docs/assets/accounting-m5/performance') or output.is_relative_to(ROOT / 'artifacts/performance-m5')
    output.mkdir(parents=True, exist_ok=False)
    json_write(output / 'paged-sales-drafts.json', [synthetic_draft(i) for i in range(120)])
    json_write(output / 'long-sales-draft.json', synthetic_draft(long=True))
    (output / 'M5-synthetic-prices.csv').write_text(
        'code,name,measure,price,currency,source_at,aliases\n'
        'SKU-A,کالای الف فروش,عدد,20,rial,2026-03-22T09:00:00Z,نمونه الف\n'
        'SKU-B,کالای ب فروش,عدد,30,rial,2026-03-22T09:00:00Z,نمونه ب\n')
    files = {p.name: {'sha256': helper.sha(p), 'bytes': p.stat().st_size}
             for p in sorted(output.iterdir()) if p.is_file()}
    json_write(output / 'dataset-plan.json', {
        'synthetic_only': True, 'generator': 'scripts/native-performance-M5.py --prepare-only',
        'seed': 'fixed integer index 0..119; no random state', 'files': files,
        'paged_sales': {'raw_drafts': 120, 'groups': {'PERF-A': 60, 'PERF-B': 60},
                        'native_page_size': 25, 'filtered_pages_per_group': 3,
                        'financial_effects': 0, 'catalog_rows': 2},
        'long_sales': {'raw_drafts': 1, 'canonical_raw_lines': 1000,
                       'native_page_size': 25, 'pages': 40, 'expected_net_irr': 20000,
                       'financial_effects': 0},
        'quotation_scale': {'source': str(QUOTE_SOURCE), 'products': 50000, 'draft_lines': 1000,
                            'observer': 'unchanged scripts/native-performance-M2.py'},
        'context_resolution': 'Use exact company/period/setup from the completed synthetic procurement profile and latest operational financial date. Seed only stopped fresh SQLite backups; preserve all accounting payload fields.',
        'limits': ['No five-year / 100000-document / million-journal-line / 10000-party claim.',
                   'Raw drafts are separate from posted sales; a 120-draft list does not certify a large posted financial ledger.'],
    })
    print(json.dumps({'prepared': str(output), 'files': files}, ensure_ascii=False), flush=True)


def write_sales_prices(profile):
    (profile / 'M5-synthetic-prices.csv').write_text(
        'code,name,measure,price,currency,source_at,aliases\n'
        'SKU-A,کالای الف فروش,عدد,20,rial,2026-03-22T09:00:00Z,نمونه الف\n'
        'SKU-B,کالای ب فروش,عدد,30,rial,2026-03-22T09:00:00Z,نمونه ب\n')


def seed_sales_profile(source, destination, long=False):
    """Fixture injection is outside timing, in a stopped fresh synthetic clone only."""
    before = helper.payload(source)
    copy_database(source, destination)
    generation, serialized = helper.payload(destination)
    state = json.loads(serialized)
    assert state['schema'] == 6, 'M5 fixture must be initialized by the actual schema6 binary'
    accounting_before = json.dumps(state['accounting'], sort_keys=True, ensure_ascii=False)
    setup = state['accounting']['core']['setups'][0]
    dates = [row['command']['as_of'] for row in state['accounting']['core']['openings']]
    for module in ('procurement', 'sales'):
        for row in state['accounting'].get(module, {}).get('records', []):
            command = next(iter(row['command'].values()))
            dates.append(command['context']['date'])
    context = {'@company': setup['command']['company_id'], '@period': setup['command']['period_id'],
               '@setup': setup['id'], '@date': max(dates)}
    drafts = [synthetic_draft(long=True)] if long else [synthetic_draft(i) for i in range(120)]
    for draft in drafts:
        for key in ('company_id', 'period_id', 'setup_id', 'document_date', 'financial_date'):
            draft[key] = context[draft[key]]
        for line in draft['lines']:
            line['price_date'] = context['@date']
    assert not state.get('sales_drafts'), 'Do not overwrite preexisting synthetic drafts'
    state['sales_drafts'] = drafts
    assert json.dumps(state['accounting'], sort_keys=True, ensure_ascii=False) == accounting_before
    with sqlite3.connect(destination / 'sarv.sqlite3') as db:
        db.execute('PRAGMA synchronous=FULL')
        changed = db.execute('UPDATE state SET generation=?,payload=? WHERE id=1 AND generation=?',
                             (generation + 1, json.dumps(state, ensure_ascii=False, separators=(',', ':')), generation)).rowcount
        assert changed == 1
    assert helper.payload(source) == before, 'Synthetic fixture source changed'
    json_write(destination / 'synthetic-seed-manifest.json', {
        'synthetic_only': True, 'fixture_injection_outside_timings': True,
        'accounting_payload_unchanged': True, 'source_payload_sha256': hashlib.sha256(before[1].encode()).hexdigest(),
        'drafts': len(drafts), 'lines': sum(len(d['lines']) for d in drafts),
        'resolved_context': context, 'seed_shape': core_shape(destination),
    })


def sales_phases(profile):
    path = profile / 'sales-performance-phases.jsonl'
    return [json.loads(line) for line in path.read_text().splitlines() if line.endswith('}')] if path.exists() else []


def strict_sales_phases(records):
    return [{'operation': row['operation'], 'phase': row['phase'], 'result': row['result'],
             'ticks': row['ticks'], 'actual_ms': row['max_gap_ms'], 'budget_ms': 50,
             'measurable': row['ticks'] > 0 and row['result'] in ('verified', 'worker_finished'),
             'passes': row['ticks'] > 0 and row['result'] in ('verified', 'worker_finished') and row['max_gap_ms'] <= 50,
             'scope': 'Observed timer responsiveness including worker wait, GTK and probe work; no continuous CPU trace'}
            for row in records]


def sales_raw_proof(profile, baseline):
    """Literal contract outputs, independent of production reconciliation calculation."""
    state = json.loads(helper.payload(profile)[1])
    accounting = state['accounting']
    records = accounting['sales']['records']
    groups = {kind: [r for r in records if kind in r['data']] for kind in ('Order', 'Fulfill', 'Return')}
    assert len(records) == 6 and all(len(v) == 2 for v in groups.values())
    assert accounting['core'] == baseline['core'] and accounting['procurement'] == baseline['procurement']
    assert accounting['journals'][:len(baseline['journals'])] == baseline['journals']
    assert len(accounting['journals']) == 9 and not state.get('sales_drafts', [])
    assert sorted(r['data']['Order']['total'] for r in groups['Order']) == [20, 199]
    assert all(r['journal_id'] is None and not r['movements'] for r in groups['Order'])
    assert sorted((r['data']['Fulfill']['total'], r['data']['Fulfill']['cost']) for r in groups['Fulfill']) == [(99, 58), (100, 58)]
    assert sorted((r['data']['Return']['total'], r['data']['Return']['cost']) for r in groups['Return']) == [(40, 23), (60, 35)]
    financial_dates = [next(iter(r['command'].values()))['context']['date'] for r in records if r['movements']]
    assert financial_dates == ['2026-03-28', '2026-03-29', '2026-03-30', '2026-03-31'], 'Literal M5 financial dates changed'
    assert all(sum(l['quantity_thousandths'] for l in r['data']['Fulfill']['lines']) == 5000 for r in groups['Fulfill'])
    assert sorted(sum(l['quantity_thousandths'] for l in r['data']['Return']['lines']) for r in groups['Return']) == [2000, 3000]
    assert all(r['journal_id'] and len(r['movements']) == 1 for kind in ('Fulfill', 'Return') for r in groups[kind])
    latest_pool = groups['Fulfill'][-1]['movements'][0]['after']
    assert latest_pool['quantity_thousandths'] == 25000 and latest_pool['value'] == 292
    linked = [r for r in groups['Order'] if r['command']['Order']['quotation']]
    assert len(linked) == 1
    quote_link = linked[0]['command']['Order']['quotation']
    assert any(q['id'] == quote_link['quote_id'] for q in state['quotes']), 'Quote conversion is not authentic retained source'
    mappings = {r['role']: r['account_id'] for r in accounting['core']['setups'][0]['command']['mappings']}
    sums = {}
    for journal in accounting['journals']:
        assert sum(l['debit'] for l in journal['lines']) == sum(l['credit'] for l in journal['lines'])
        for line in journal['lines']:
            account_id = line['account']['id']
            sums[account_id] = sums.get(account_id, 0) + line['debit'] - line['credit']
    expected = {'Cash': 251, 'Bank': 300, 'Receivables': 299, 'Inventory': 422,
                'ChequesReceivable': 200, 'Payables': -329, 'ChequesPayable': -100,
                'Grni': -50, 'OwnerCapital': -952, 'Sales': -199, 'SalesReturns': 100, 'Cogs': 58}
    expected.update({role: 0 for role in ('SupplierAdvances', 'CustomerAdvances', 'InventoryLoss', 'OperatingExpense', 'BankFees')})
    assert all(sums.get(mappings[role], 0) == value for role, value in expected.items())
    assert sum(v for v in sums.values() if v > 0) == -sum(v for v in sums.values() if v < 0) == 1630
    return {'records': 6, 'journals': 9, 'records_by_kind': {k: len(v) for k, v in groups.items()},
            'typed_sales_movements': 4, 'signed_controls_irr': expected,
            'trial_debit_irr': 1630, 'trial_credit_irr': 1630, 'inventory_irr': 422,
            'profit_irr': 41, 'W1_quantity_thousandths': 25000, 'W1_value_irr': 292,
            'financial_dates': financial_dates, 'final_report_as_of': '2026-03-31',
            'original_cost_returns_irr': [23, 35], 'original_credit_returns_irr': [40, 60],
            'authentic_quote_link': quote_link, 'core_procurement_and_previous_journals_unchanged': True}


def measure_sales_stage(output, source, binary, env, start, x11, capture, report_target=None):
    """Real Save/restart/preview/submit on the native staged order; no smoke timing loop."""
    profile = output / 'sales-external-stage'
    copy_database(source, profile)
    write_sales_prices(profile)
    env.update(SARV_DATA_DIR=str(profile), SARV_SALES_SMOKE='stage', SARV_SALES_PERFORMANCE_TRACE='1')
    app = start([str(binary)], 'sales-external-stage', env)
    marker, failure = profile / 'sales-stage-ready.json', profile / 'sales-native-failure.txt'
    native.wait_for(lambda: marker.exists() or failure.exists() or app.poll() is not None,
                    seconds=180, label='native saved Sales stage')
    assert marker.exists() and not failure.exists(), 'Native Sales staging failed'
    staged = json.loads(marker.read_text())
    result = {'stage_manifest': staged, 'n': 1, 'synthetic_only': True,
              'boundary': 'Pre-discovered native controls; real XTest Save/Space; SQLite durable payload then callback usability independently awaited. Polling/UI/layout included. Restart/correction distinct; no repeated-command percentile.'}
    if report_target is not None:
        report_target['external_stage'] = result
    draft_id = staged['draft_id']
    reference = staged['reference']
    generation, serialized = helper.payload(profile)
    state = json.loads(serialized)
    accounting_before = state['accounting']
    original = next(d for d in state['sales_drafts'] if d['id'] == draft_id)
    original_total = original['supplied_total']
    phase_count = len(sales_phases(profile))
    access = Access(app.pid)
    window = native.wait_for(lambda: x11.find(app.pid), label='Sales external stage window')
    x11.focus(window)
    total = observe(lambda: text_control(access, 'جمع خالص مستقل سفارش'), 'staged Sales total')
    editor_root = observe(lambda: sales_editor_root(access, total), 'Sales dialog accessibility root')
    save = observe(lambda: button(access, 'ذخیرهٔ پیش‌نویس', editor_root), 'staged Sales Save')
    # Input is deliberately invalid; ordinary raw Save must remain available.
    keyboard_focus(access, total, x11)
    observe(lambda: ready(access, total, focus=True) and x11.find(app.pid) == window,
            'owned staged Sales window and exact total entry focus before keys')
    x11.key('a', 'Control_L')
    x11.type_ascii('INVALID-M5-RAW-TOTAL')
    try:
        observe(lambda: access.text(total) == 'INVALID-M5-RAW-TOTAL' and ready(access, total, focus=True),
                'exact invalid raw Sales total after real typing')
    except Exception:
        result['invalid_raw_typing_failure'] = {
            'observed_text': access.text(total), 'expected_text': 'INVALID-M5-RAW-TOTAL',
            'entry_ready': ready(access, total), 'exact_entry_focused': access.focused(total),
            'entry_rectangle': access.rect(total), 'owned_window': window,
            'current_owned_window': x11.find(app.pid), 'pid': app.pid,
        }
        json_write(profile / 'invalid-raw-typing-failure.json', result['invalid_raw_typing_failure'])
        capture(window, profile / 'invalid-raw-typing-failure.png')
        raise
    begin = time.perf_counter()
    x11.key('s', 'Control_L')
    def saved_total(expected, min_revision):
        current_generation, payload = helper.payload(profile)
        current = json.loads(payload)
        draft = next((d for d in current.get('sales_drafts', []) if d['id'] == draft_id), None)
        return draft if (current_generation > generation and draft and draft['supplied_total'] == expected
                         and draft['revision'] >= min_revision) else None
    raw = observe(lambda: saved_total('INVALID-M5-RAW-TOTAL', original['revision'] + 1), 'invalid raw Sales Save durable')
    result['invalid_raw_save_durable_ms'] = (time.perf_counter() - begin) * 1000
    observe(lambda: ready(access, save), 'Sales Save receipt callback usable')
    result['invalid_raw_save_callback_usable_ms'] = (time.perf_counter() - begin) * 1000
    assert json.loads(helper.payload(profile)[1])['accounting'] == accounting_before, 'Raw Save posted financial effects'
    result['invalid_raw_saved_revision'] = raw['revision']
    capture(window, profile / 'invalid-raw-saved.png')
    native.stop(app)
    env.pop('SARV_SALES_SMOKE')
    app = start([str(binary)], 'sales-external-restart', env)
    access = Access(app.pid)
    window = native.wait_for(lambda: x11.find(app.pid), label='Sales restart window')
    x11.focus(window)
    page_root, search = sales_scope(access, x11)
    sales_zero_result(access, page_root, search, x11)
    keyboard_focus(access, search, x11)
    x11.key('a', 'Control_L')
    x11.type_ascii(reference)
    def current_saved_heading():
        row = helper.find_name(access, f'پیش‌نویس • {reference}', page_root)
        return row if row and access.text(search) == reference and ready(access, row) else None
    heading = observe(current_saved_heading, 'current visible restarted saved Sales reference')
    card = access.a.atspi_accessible_get_parent(heading['node'], None)
    opener = observe(lambda: button(access, 'ادامهٔ پیش‌نویس', card), 'restarted saved Sales action')
    observe(lambda: ready(access, opener), 'restarted saved Sales action visible and usable')
    keyboard_focus(access, opener, x11)
    x11.key('space')
    total = observe(lambda: text_control(access, 'جمع خالص مستقل سفارش'), 'reopened raw Sales total')
    observe(lambda: access.text(total) == 'INVALID-M5-RAW-TOTAL', 'exact invalid raw user input retained after restart')
    restart_editor_root = observe(lambda: sales_editor_root(access, total), 'restart Sales dialog root')
    result['invalid_raw_restart_preserved'] = True
    assert json.loads(helper.payload(profile)[1])['accounting'] == accounting_before
    # A clean saved editor may be cancelled without discarding durable input.
    restart_baseline = helper.payload(profile)
    x11.key('Escape')
    observe(lambda: not ready(access, {'node': restart_editor_root}) and ready(access, search),
            'clean saved Sales editor Escape closed and parent search usable')
    assert helper.payload(profile) == restart_baseline, 'Clean editor cancellation changed durable state'
    result['clean_editor_cancel_preserved'] = True
    page_root, search = sales_scope(access, x11)
    heading = observe(lambda: helper.find_name(access, f'پیش‌نویس • {reference}', page_root), 'cancelled saved Sales reference')
    opener = observe(lambda: button(access, 'ادامهٔ پیش‌نویس', access.a.atspi_accessible_get_parent(heading['node'], None)), 'reopen after cancellation')
    observe(lambda: ready(access, opener), 'cancelled saved Sales action visible and usable')
    keyboard_focus(access, opener, x11)
    x11.key('space')
    total = observe(lambda: text_control(access, 'جمع خالص مستقل سفارش'), 'raw Sales total after cancellation')
    editor_root = observe(lambda: sales_editor_root(access, total), 'reopened Sales dialog root')
    keyboard_focus(access, total, x11)
    x11.key('a', 'Control_L')
    x11.type_ascii(original_total)
    observe(lambda: access.text(total) == original_total and ready(access, total, focus=True),
            'exact corrected raw Sales total before Save timer')
    begin = time.perf_counter()
    x11.key('s', 'Control_L')
    corrected = observe(lambda: saved_total(original_total, raw['revision'] + 1), 'corrected Sales total durable')
    result['corrected_raw_save_durable_ms'] = (time.perf_counter() - begin) * 1000
    save = observe(lambda: button(access, 'ذخیرهٔ پیش‌نویس', editor_root), 'corrected editor Save')
    observe(lambda: ready(access, save), 'corrected Save callback usable')
    corrected_baseline = helper.payload(profile)
    preview = observe(lambda: button(access, 'بازبینی سفارش', editor_root), 'external Sales preview')
    post = observe(lambda: button(access, 'ثبت سفارش بازبینی‌شده', editor_root), 'external Sales submit')
    assert not sensitive(access, post), 'Preview was already enabled before external observation'
    keyboard_focus(access, preview, x11)
    begin = time.perf_counter()
    x11.key('space')
    observe(lambda: sensitive(access, post), 'current Sales preview and submit usable')
    result['order_preview_observed_ms'] = (time.perf_counter() - begin) * 1000
    assert helper.payload(profile) == corrected_baseline, 'Sales preview mutated durable state'
    capture(window, profile / 'external-order-preview.png')
    keyboard_focus(access, post, x11)
    begin = time.perf_counter()
    x11.key('space')
    def durable_order():
        current = json.loads(helper.payload(profile)[1])
        records = current['accounting'].get('sales', {}).get('records', [])
        matching = [r for r in records if next(iter(r['command'].values()))['context']['source_id'] == draft_id]
        return matching[0] if len(matching) == 1 and not any(d['id'] == draft_id for d in current.get('sales_drafts', [])) else None
    order = observe(durable_order, 'one exact Sales order durable and draft consumed')
    result['order_submit_durable_ms'] = (time.perf_counter() - begin) * 1000
    observe(lambda: not ready(access, {'node': editor_root}) and ready(access, search),
            'Sales submit receipt closed editor and parent search usable')
    result['order_submit_callback_ready_ms'] = (time.perf_counter() - begin) * 1000
    current = json.loads(helper.payload(profile)[1])
    assert order['journal_id'] is None and order['data']['Order']['total'] == int(original_total)
    assert current['accounting']['journals'] == accounting_before['journals'], 'Nonfinancial order wrote journals'
    assert current['accounting']['core'] == accounting_before['core']
    assert current['accounting']['procurement'] == accounting_before['procurement']
    result['nonfinancial_order_proof'] = {'id': order['id'], 'total_irr': int(original_total), 'journal_id': None,
        'source_revision': next(iter(order['command'].values()))['context']['source_revision'],
        'raw_consumed': True, 'core_procurement_journals_unchanged': True}
    observe(lambda: len(sales_phases(profile)) >= phase_count + 4, 'external Sales four operation timer records')
    result['native_phase_records_before_observer'] = sales_phases(profile)[:phase_count]
    result['native_phase_records'] = sales_phases(profile)[phase_count:]
    result['strict_phase_checks'] = strict_sales_phases(result['native_phase_records'])
    result['memory_kib_after_external_workflow'] = helper.rss(app.pid)
    native.stop(app)
    return result


def sales_editor_root(access, control):
    node = control['node']
    for _ in range(24):
        if not node:
            return None
        role = access.string(access.a.atspi_accessible_get_role_name, node, None)
        if role == 'dialog':
            return node
        node = access.a.atspi_accessible_get_parent(node, None)
    return None


def sales_scope(access, x11):
    x11.key('0', 'Alt_L')
    anchor = observe(lambda: helper.find_name(access, 'فروش‌ها • سفارش، تحویل و برگشت'), 'Sales page anchor')
    root = access.a.atspi_accessible_get_parent(anchor['node'], None)
    assert root
    search = observe(lambda: helper.find_name(access, 'جست‌وجوی شماره، مشتری، کالا یا پیش‌نویس', root), 'Sales search')
    # Every workspace exists in accessibility even while hidden. Real Ctrl+F
    # reveals the current Sales search; existence cannot satisfy readiness.
    sales_focus_search(access, search, x11)
    return root, search


def sales_focus_search(access, search, x11):
    """Like M4 Purchases, reveal search after a focused result scrolls it away."""
    x11.key('f', 'Control_L')
    observe(lambda: ready(access, search), 'visible sensitive current Sales search')
    keyboard_focus(access, search, x11)
    observe(lambda: ready(access, search, focus=True), 'visible exact Sales search focus')


def sales_zero_result(access, page_root, search, x11):
    """Reject a pre-filter same-reference row before discovering a stable action."""
    sales_focus_search(access, search, x11)
    x11.key('a', 'Control_L')
    x11.type_ascii('NO-MATCH-PERFORMANCE')
    observe(lambda: access.text(search) == 'NO-MATCH-PERFORMANCE' and helper.first_accessible(access, lambda row:
        normalize_digits(access.string(access.a.atspi_accessible_get_name, row['node'], None)).startswith('0 نتیجه •')
        and ready(access, row), page_root), 'exact zero-row Sales filter before current result')


def normalize_digits(value):
    return value.translate(str.maketrans('۰۱۲۳۴۵۶۷۸۹٠١٢٣٤٥٦٧٨٩', '01234567890123456789'))


def posted_sales_search(access, x11, profile, samples, report_target=None):
    page_root, search = sales_scope(access, x11)
    baseline = helper.payload(profile)
    rows = []
    measurement = {'samples': rows, 'result': 'incomplete'}
    if report_target is not None:
        report_target['posted_search'] = measurement
    measurement['cached_observation'] = {}
    cache = discover_sales_page_cache(access, page_root, search, measurement['cached_observation'])
    def caption(row):
        return access.string(access.a.atspi_accessible_get_name, row['node'], None)
    def count(expected):
        return helper.first_accessible(access, lambda row:
            normalize_digits(caption(row)).startswith(f'{expected} نتیجه •') and ready(access, row), page_root)
    keyboard_focus(access, search, x11)
    x11.key('a', 'Control_L')
    x11.type_ascii('NO-MATCH-PERFORMANCE')
    observe(lambda: count(0), 'zero-row posted Sales warmup')
    observe(cache.zero_hidden, 'owned cached slot0 insensitive/current-tree absent at exact zero results')
    cache.receipt('zero-result hide', {'matches': 0, 'cached_hide_qualified': True})
    for index in range(samples):
        query = ('INV-M5-A', 'INV-M5-B')[index % 2]
        sales_focus_search(access, search, x11)
        begin = time.perf_counter()
        x11.key('a', 'Control_L')
        x11.type_ascii(query)
        dispatch_ms = (time.perf_counter() - begin) * 1000
        def result():
            def current():
                title = cache.read('title')
                action_name = cache.read('action')
                if access.text(search) != query or not normalize_digits(cache.read('status')).startswith('1 نتیجه •') \
                        or not ready(access, cache.nodes['status']):
                    return None
                action = cache.nodes['action']
                return action if query in title and 'صورتحساب' in title \
                    and ready(access, cache.nodes['title']) \
                    and action_name == 'جزئیات و جریان کار' and ready(access, action) else None
            return cache.predicate('posted_current_result', current)
        action = observe(result, f'correct current posted Sales invoice {query}')
        visible_ms = (time.perf_counter() - begin) * 1000
        keyboard_focus(access, action, x11)
        observe(lambda: ready(access, action, focus=True), 'posted Sales focused usable Detail')
        rows.append({'index': index, 'query': query, 'matches': 1, 'key_dispatch_ms': dispatch_ms,
                     'correct_visible_ms': visible_ms, 'focused_usable_ms': (time.perf_counter() - begin) * 1000})
        cache.receipt('posted current focused result', {'query': query, 'matches': 1})
        assert helper.payload(profile) == baseline, 'Posted Sales search changed durable state'
    measurement.update({'result': 'measured', 'correct_visible_ms': helper.stats([r['correct_visible_ms'] for r in rows]),
            'focused_usable_ms': helper.stats([r['focused_usable_ms'] for r in rows]),
            'boundary': 'm5-cached-v3: same real alternating invoice-reference/current one-row invoice/visible sensitive Detail then focused usable endpoint; discovery of persistent slot0 nodes moved before timers, current names/roles/parents/process/states/geometry and every ancestry parent-child membership reread inside. Six-record financial fixture, distinct from120 raw drafts; no causal product-speed comparison with original traversal method.'})
    return measurement


def measure_sales(output, procurement_source, binary, env, start, x11, capture, search_samples, pilot, report_target=None):
    """Sales native workflow, read-only paged searches and a raw1000-line editor."""
    result = {'result': 'incomplete', 'workflows': {}, 'search_samples': []}
    if report_target is not None:
        report_target['sales_measurement'] = result
    source_before = helper.payload(procurement_source)
    result['external_stage'] = measure_sales_stage(output, procurement_source, binary, env, start, x11, capture, result)
    for mode in ('full', 'draft'):
        profile = output / f'sales-native-{mode}'
        copy_database(procurement_source, profile)
        write_sales_prices(profile)
        env.update(SARV_DATA_DIR=str(profile), SARV_SALES_SMOKE=mode, SARV_SALES_PERFORMANCE_TRACE='1')
        app = start([str(binary)], f'sales-native-{mode}', env)
        marker, failure = profile / 'sales-native-verification.json', profile / 'sales-native-failure.txt'
        native.wait_for(lambda: marker.exists() or failure.exists() or app.poll() is not None,
                        seconds=180, label=f'Sales {mode} native workflow')
        assert marker.exists() and not failure.exists(), f'Sales {mode} native workflow failed'
        manifest = json.loads(marker.read_text())
        assert manifest.get('synthetic_only') is True, 'Sales probe must declare synthetic provenance'
        result['workflows'][mode] = {'manifest': manifest, 'shape': core_shape(profile),
            'native_phase_records': sales_phases(profile),
            'strict_phase_checks': strict_sales_phases(sales_phases(profile)),
            'memory_kib_after_workflow': helper.rss(app.pid)}
        if mode == 'full':
            result['workflows'][mode]['independent_raw_proof'] = sales_raw_proof(profile, json.loads(source_before[1])['accounting'])
        window = native.wait_for(lambda: x11.find(app.pid), label=f'Sales {mode} window')
        if mode == 'full':
            x11.focus(window)
            result['posted_search'] = posted_sales_search(Access(app.pid), x11, profile, search_samples, result)
            result['workflows'][mode]['native_phase_records'] = sales_phases(profile)
            result['workflows'][mode]['strict_phase_checks'] = strict_sales_phases(sales_phases(profile))
        capture(window, profile / 'workflow-completed.png')
        native.stop(app)
    env.pop('SARV_SALES_SMOKE')
    paged = output / 'sales-paged-search'
    seed_sales_profile(procurement_source, paged)
    env['SARV_DATA_DIR'] = str(paged)
    app = start([str(binary)], 'sales-paged-search', env)
    access = Access(app.pid)
    window = native.wait_for(lambda: x11.find(app.pid), label='Sales paged search window')
    x11.focus(window)
    page_root, search = sales_scope(access, x11)
    baseline = helper.payload(paged)
    result['paged_cached_observation'] = {}
    page_cache = discover_sales_page_cache(access, page_root, search, result['paged_cached_observation'])
    def caption(row):
        return access.string(access.a.atspi_accessible_get_name, row['node'], None)
    def summary_count(count):
        return page_cache.predicate('paged_summary', lambda:
            normalize_digits(page_cache.read('status')).startswith(f'{count} نتیجه •')
            and ready(access, page_cache.nodes['status']))
    def reference_action(reference):
        def current():
            action = page_cache.nodes['action']
            return action if page_cache.read('title') == f'پیش‌نویس • {reference}' \
                and ready(access, page_cache.nodes['title']) \
                and page_cache.read('action') == 'ادامهٔ پیش‌نویس' and ready(access, action) else None
        return page_cache.predicate('paged_current_reference', current)
    keyboard_focus(access, search, x11)
    x11.key('a', 'Control_L')
    x11.type_ascii('NO-MATCH-PERFORMANCE')
    observe(lambda: summary_count(0), 'zero-row Sales search warmup')
    observe(page_cache.zero_hidden, 'owned cached paged slot0 insensitive/current-tree absent at exact zero results')
    page_cache.receipt('zero-result hide', {'matches': 0, 'cached_hide_qualified': True})
    for index in range(search_samples):
        group = 'A' if index % 2 == 0 else 'B'
        query = f'PERF-{group}'
        expected = f'PERF-{group}-{index % 2:03d}'
        sales_focus_search(access, search, x11)
        begin = time.perf_counter()
        x11.key('a', 'Control_L')
        x11.type_ascii(query)
        dispatch_ms = (time.perf_counter() - begin) * 1000
        def current_result():
            return reference_action(expected) if access.text(search) == query and summary_count(60) else None
        action = observe(current_result, f'correct current {query} result')
        visible_ms = (time.perf_counter() - begin) * 1000
        keyboard_focus(access, action, x11)
        observe(lambda: ready(access, action, focus=True), 'focused usable Sales draft action')
        result['search_samples'].append({'index': index, 'query': query, 'expected': expected,
            'matches': 60, 'key_dispatch_ms': dispatch_ms, 'correct_visible_ms': visible_ms,
            'focused_usable_ms': (time.perf_counter() - begin) * 1000})
        page_cache.receipt('paged current focused result', {'query': query, 'reference': expected, 'matches': 60})
        assert helper.payload(paged) == baseline, 'Read-only Sales search changed durable state'
    result['correct_visible_ms'] = helper.stats([r['correct_visible_ms'] for r in result['search_samples']])
    result['focused_usable_ms'] = helper.stats([r['focused_usable_ms'] for r in result['search_samples']])
    result['search_budgets'] = {key: {'budget_ms': 200, 'p95_ms': result[key]['p95'],
                                   'passes': result[key]['p95'] <= 200}
                              for key in ('correct_visible_ms', 'focused_usable_ms')}
    result['search_boundary'] = 'm5-cached-v3: same real XTest query/exact current first draft/60-match summary/visible sensitive Continue then component-first focused usable endpoint; persistent discovery before timer, fresh names/roles/current parent/process/states/geometry and every ancestry parent-child membership inside. Observation-method delta, not product-only speed.'
    # Current B query: visit every filtered page and reject stale first-page rows.
    result['filtered_page_samples'] = []
    for page_index, expected in [(1, 'PERF-B-051'), (2, 'PERF-B-101')]:
        next_page = observe(lambda: button(access, 'صفحهٔ بعد', page_root), 'next filtered page')
        keyboard_focus(access, next_page, x11)
        begin = time.perf_counter()
        x11.key('space')
        action = observe(lambda: reference_action(expected), f'filtered page {page_index + 1} correct row')
        duration = (time.perf_counter() - begin) * 1000
        result['filtered_page_samples'].append({'page': page_index + 1, 'expected': expected, 'usable_visible_ms': duration,
                                               'budget_ms': 150, 'passes': duration <= 150})
        page_cache.receipt('filtered current page', {'page': page_index + 1, 'reference': expected, 'matches': 60})
        assert summary_count(60) and helper.payload(paged) == baseline
        if page_index == 2:
            assert not sensitive(access, button(access, 'صفحهٔ بعد', page_root)), 'Last filtered page not bounded'
    capture(window, paged / 'search-last-filtered-page.png')
    result['paged_search_shape'] = core_shape(paged)
    result['paged_search_memory_kib'] = helper.rss(app.pid)
    native.stop(app)

    large = output / 'sales-long-editor'
    seed_sales_profile(procurement_source, large, long=True)
    env['SARV_DATA_DIR'] = str(large)
    app = start([str(binary)], 'sales-long-editor', env)
    access = Access(app.pid)
    window = native.wait_for(lambda: x11.find(app.pid), label='Sales long editor window')
    x11.focus(window)
    page_root, search = sales_scope(access, x11)
    baseline = helper.payload(large)
    sales_zero_result(access, page_root, search, x11)
    keyboard_focus(access, search, x11)
    x11.key('a', 'Control_L')
    x11.type_ascii('PERF-LONG-1000')
    opener = observe(lambda: helper.find_name(access, 'پیش‌نویس • PERF-LONG-1000', page_root), 'long draft correct result')
    card = access.a.atspi_accessible_get_parent(opener['node'], None)
    action = observe(lambda: button(access, 'ادامهٔ پیش‌نویس', card), 'long draft Continue')
    observe(lambda: ready(access, action) and access.text(search) == 'PERF-LONG-1000',
            'current visible long draft Continue before timing')
    keyboard_focus(access, action, x11)
    begin = time.perf_counter()
    x11.key('space')
    total = observe(lambda: text_control(access, 'جمع خالص مستقل سفارش'), 'native long draft editor')
    editor_root = observe(lambda: sales_editor_root(access, total), 'long Sales editor dialog root')
    def long_page_action(page):
        marker = helper.first_accessible(access, lambda row:
            normalize_digits(caption(row)) == f'1000 ردیف • صفحه {page}', editor_root)
        if not marker:
            return None
        action = button(access, 'ویرایش ردیف', editor_root)
        if action and not ready(access, action):
            access.focus(action)
        return action if action and ready(access, action) else None
    first_edit = observe(lambda: long_page_action(1), '1000-line first page visible usable Edit')
    result['long_editor_open_to_first_page_ms'] = (time.perf_counter() - begin) * 1000
    keyboard_focus(access, first_edit, x11)
    observe(lambda: ready(access, first_edit, focus=True), '1000-line first page focused usable Edit')
    result['long_editor_open_to_focused_action_ms'] = (time.perf_counter() - begin) * 1000
    # The dialog did not exist before opening dispatch. Its historical opening
    # discovery endpoint stays intact; cache only the39 loaded-page actions.
    result['long_cached_observation'] = {}
    cache_discovery_started = time.perf_counter()
    page_marker = observe(lambda: helper.first_accessible(access, lambda row:
        normalize_digits(caption(row)) == '1000 ردیف • صفحه 1', editor_root), 'persistent long page marker')
    long_next = observe(lambda: button(access, 'ردیف‌های بعد', editor_root), 'persistent long Next')
    long_cache = CachedSalesNodes(access, editor_root,
        {'marker': page_marker, 'action': first_edit, 'next': long_next}, result['long_cached_observation'])
    result['long_cached_observation']['discovery_ms_outside_timers'] = (time.perf_counter() - cache_discovery_started) * 1000
    result['long_cached_observation']['warmup_effect'] = 'Discovery after original first-page visible/focused opening endpoint; no added query/page navigation.'
    long_cache.receipt('long initial focused page', {'page': 1})
    def cached_long_page_action(page):
        def current():
            if normalize_digits(long_cache.read('marker')) != f'1000 ردیف • صفحه {page}':
                return None
            action = long_cache.nodes['action']
            if long_cache.read('action') != 'ویرایش ردیف':
                return None
            if not ready(access, action):
                access.focus(action)  # Same historical visible-phase assistance.
            return action if ready(access, action) else None
        return long_cache.predicate('long_current_page', current)
    result['long_editor_pages'] = []
    for page in range(2, 41):
        next_lines = long_cache.nodes['next']
        long_cache.check_identity()
        assert long_cache.read('next') == 'ردیف‌های بعد', 'Cached long Next caption changed'
        keyboard_focus(access, next_lines, x11)
        observe(lambda: ready(access, next_lines, focus=True), 'long Sales pager focused and usable')
        begin = time.perf_counter()
        x11.key('space')
        edit = observe(lambda: cached_long_page_action(page), f'1000-line page {page} visible usable Edit')
        duration = (time.perf_counter() - begin) * 1000
        keyboard_focus(access, edit, x11)
        observe(lambda: ready(access, edit, focus=True), f'1000-line page {page} focused usable Edit')
        focused_duration = (time.perf_counter() - begin) * 1000
        result['long_editor_pages'].append({'page': page, 'ready_ms': duration,
            'focused_usable_ms': focused_duration, 'budget_ms': 150, 'passes': duration <= 150,
            'focused_passes': focused_duration <= 150})
        long_cache.receipt('long current focused page', {'page': page})
        assert helper.payload(large) == baseline, 'Read-only Sales paging changed durable data'
    assert not sensitive(access, button(access, 'ردیف‌های بعد', editor_root)), 'Long editor last page is not bounded'
    result['long_editor_page_ms'] = helper.stats([r['ready_ms'] for r in result['long_editor_pages']])
    result['long_editor_focused_page_ms'] = helper.stats([r['focused_usable_ms'] for r in result['long_editor_pages']])
    capture(window, large / '1000-line-last-native-page.png')
    result['long_editor_memory_kib'] = helper.rss(app.pid)
    result['long_editor_shape'] = core_shape(large)
    result['long_editor_boundary'] = 'm5-cached-v3 loaded pages: same exact current scoped page marker plus visible sensitive Edit then focused usable Edit; persistent marker/slot0 action/Next discovered before page timers, current roles/parents/process/names/states/geometry and every ancestry parent-child membership refreshed inside. Historical opening discovery and visible-phase focus assistance retained. No invented visible lineUUID; observation-method change is not causal product-speed evidence.'
    result['native_read_only_phases'] = {'paged_search': sales_phases(paged), 'long_editor': sales_phases(large)}
    native.stop(app)
    env.pop('SARV_SALES_PERFORMANCE_TRACE')
    assert helper.payload(procurement_source) == source_before, 'Sales observers changed procurement source'
    result['result'] = 'measured'
    return result, paged, large


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--prepare-only', action='store_true', help='Write deterministic synthetic inputs; no application invocation')
    parser.add_argument('--binary', type=Path)
    parser.add_argument('--expect-sha')
    parser.add_argument('--source-manifest', type=Path, help='Required root-issued frozen production source manifest')
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--samples', type=int, default=10)
    parser.add_argument('--search-samples', type=int, default=20)
    parser.add_argument('--pilot', action='store_true', help='Development observer check only: n1 startup/n2 search, not acceptance evidence')
    args = parser.parse_args()
    if args.prepare_only:
        prepare_dataset(args.output)
        return
    assert args.binary and args.expect_sha and args.source_manifest, 'Final epoch needs root frozen binary/hash/source manifest'
    expected_source = json.loads(args.source_manifest.read_text())
    if 'frozen_source_provenance' in expected_source:
        expected_source = expected_source['frozen_source_provenance']
    assert frozen_source_provenance()['files'] == expected_source['files'], 'Source differs from root freeze manifest'
    assert frozen_source_provenance()['sha256'] == expected_source['sha256'], 'Source aggregate differs from root freeze manifest'
    if args.pilot:
        args.samples, args.search_samples = 1, 2
    else:
        assert args.samples >= 10 and args.search_samples >= 20
    binary = args.binary.resolve()
    assert helper.sha(binary) == args.expect_sha, 'Wrong immutable executable'
    assert os.environ.get('DBUS_SESSION_BUS_ADDRESS', '').startswith('unix:path=/tmp/dbus-'), 'Use private dbus-run-session'
    output = args.output.resolve()
    assert output.is_relative_to(ROOT / 'artifacts/performance-m5'), 'Output must be a fresh ignored performance-m5 directory'
    output.mkdir(parents=True, exist_ok=False)
    global FOCUS_FAILURE_OUTPUT
    FOCUS_FAILURE_OUTPUT = output / 'focus-failures'
    processes, logs = [], []
    report = {
        'result': 'incomplete', 'synthetic_only': True, 'development_pilot': args.pilot, 'binary': str(binary),
        'external_observer_protocol': EXTERNAL_OBSERVER_PROTOCOL,
        'external_method_delta': 'Sales posted/paged queries and loaded long pages cache persistent owned current nodes before timed dispatch; exact current readiness/focus and budgets retained with stronger fresh parent-child membership on every cached ancestry edge; zero-result oracle uses exact0/statusready/actioninsensitive/absence from exposed ancestry and records retained stale SHOWING flags. Startup/core/procurement/long opening unchanged. Direct parent-child enumeration memoized only within one predicate, no timed BFS fallback. C2/C5 original method and failed v2 separately attributed; no product-speed causal claim.',
        'binary_sha256': args.expect_sha, 'utc': helper.command('date', '-u', '+%Y-%m-%dT%H:%M:%SZ'),
        'root_freeze_manifest': str(args.source_manifest.resolve()),
        'root_freeze_manifest_sha256': helper.sha(args.source_manifest),
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
            'database_source_configuration': 'SQLite WAL; synchronous=FULL; busy timeout3s (src/store.rs). Synthetic fixture injection uses FULL and SQLite consistent backup API.',
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

    report['capture_samples'] = []

    def capture(window, path):
        begin = time.perf_counter()
        subprocess.run(['/usr/bin/import', '-display', display, '-window', str(window), str(path)], check=True)
        report['capture_samples'].append({'path': str(path), 'elapsed_ms': (time.perf_counter() - begin) * 1000})

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
                   XDG_RUNTIME_DIR=str(runtime), SARV_TEST_INSTANCE='m5-performance-owned',
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
        report['core_workflow']['search_samples'] = samples
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
            capture, args.search_samples, args.pilot, report)
        report['procurement_measurement'] = procurement
        print(json.dumps({'procurement_searches': {name: row['correct_visible_ms'] for name, row in procurement['searches'].items()}}, ensure_ascii=False), flush=True)
        sales, sales_profile, sales_large = measure_sales(output, procurement_profile, binary, env, start, x11, capture, args.search_samples, args.pilot, report)
        report['sales_measurement'] = sales
        assert json.loads((QUOTE_SOURCE / 'native-responsiveness-verification.json').read_text())['synthetic_only']
        for name, source in [('core-small-posted', workflow), ('quote-scale', QUOTE_SOURCE), ('procurement-small-posted', procurement_profile), ('sales-small-posted', output / 'sales-native-full'), ('sales-paged-drafts', sales_profile), ('sales-long-draft', sales_large)]:
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
                    interactive_duration = (time.perf_counter() - begin) * 1000
                    capture(window, screenshot)
                    duration = (time.perf_counter() - begin) * 1000
                    time.sleep(.5)
                    memory = helper.rss(app.pid)
                    dataset['samples'].append({'index': index, 'startup_ms': duration, 'interactive_before_png_ms': interactive_duration,
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
        report['frozen_source_stable'] = report['frozen_source_provenance_after'] == report['frozen_source_provenance_before'] and report['frozen_source_provenance_after']['files'] == expected_source['files']
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
