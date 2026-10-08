"""Read-only review of retained C9 execution; no application or GUI execution."""
import hashlib
import json
from pathlib import Path
import re
import sqlite3

ROOT = Path(__file__).resolve().parents[5]
OUT = Path(__file__).resolve().parent
FREEZE = ROOT / "docs/assets/accounting-m5/checks/candidate9"
inputs = {}


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read(path):
    path = Path(path)
    if not path.is_absolute():
        path = ROOT / path
    inputs[str(path.relative_to(ROOT))] = sha(path)
    return json.loads(path.read_text())


def check_files(files, base=ROOT):
    for name, expected in files.items():
        assert sha(base / name) == expected, name


def check_observers(files):
    # Prefer this epoch's immutable copies so later production/observer repairs
    # do not make historical receipt inspection depend on the current worktree.
    for name, expected in files.items():
        archived = FREEZE / 'verification-source' / name
        path = archived if archived.exists() else ROOT / name
        assert sha(path) == expected, name


def sqlite_state(path):
    path = ROOT / path
    conn = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    assert conn.execute("pragma integrity_check").fetchone() == ("ok",)
    generation, payload = conn.execute("select generation,payload from state where id=1").fetchone()
    conn.close()
    state = json.loads(payload)
    inputs[str(path.relative_to(ROOT)) + ":payload"] = hashlib.sha256(payload.encode()).hexdigest()
    return generation, state


def state_summary(state):
    journals = state['accounting']['journals']
    for journal in journals:
        assert sum(line['debit'] for line in journal['lines']) == sum(line['credit'] for line in journal['lines'])
    return {'schema': state['schema'], 'journals': len(journals), 'all_journals_balanced': True}


def guard_pair(directory, label, verification=False):
    before = read(directory / (label + '-before.json'))
    after = read(directory / (label + '-after.json'))
    assert before == after, label
    assert before['source_file_sha256'] == manifest['source_file_sha256']
    assert before['production_source_sha256'] == manifest['production_source_sha256']
    assert before['binary_sha256'] == manifest['binary_sha256']
    observed = before['verification_source_file_sha256'] if verification else before['observer_sha256']
    check_observers(observed)
    if verification:
        assert observed == verification_files
    return {'source_files': len(before['source_file_sha256']), 'observer_files': len(observed), 'before_equals_after': True}


manifest = read(FREEZE / 'manifest.json')
assert sha(Path(manifest['binary'])) == manifest['binary_sha256']
check_files(manifest['source_file_sha256'], FREEZE / 'source')
digest = hashlib.sha256()
for name in sorted(manifest['source_file_sha256'], key=lambda value: value.encode()):
    digest.update(name.encode() + b'\0' + (FREEZE / 'source' / name).read_bytes() + b'\0')
assert digest.hexdigest() == manifest['production_source_sha256']
assert read(FREEZE / 'source-provenance-before.json') == read(FREEZE / 'source-provenance-after.json') == manifest['frozen_source_provenance']
verification_files = read(FREEZE / 'verification-source-before.json')
assert verification_files == read(FREEZE / 'verification-source-after.json')
check_files(verification_files, FREEZE / 'verification-source')
worktree_matches = all(sha(ROOT / name) == expected for name, expected in manifest['source_file_sha256'].items())
comparisons = {}
for epoch in ['candidate5', 'candidate8']:
    old = read(FREEZE.parent / epoch / 'manifest.json')
    check_files(old['source_file_sha256'], FREEZE.parent / epoch / 'source')
    comparisons[epoch] = {'identical_files': sum(old['source_file_sha256'][p] == h for p, h in manifest['source_file_sha256'].items()),
                          'changed_files': [p for p, h in manifest['source_file_sha256'].items() if old['source_file_sha256'][p] != h]}
gates = read(FREEZE / 'gates/manifest.json')
gate_review = []
for command in gates['checks']:
    label = command['label']
    assert command['exit_code'] == 0
    log = FREEZE / 'gates' / (label + '.log')
    assert sha(log) == command['log_sha256']
    inputs[str(log.relative_to(ROOT))] = sha(log)
    gate_review.append({'label': label, **guard_pair(FREEZE / 'gates', label, True)})
totals = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', (FREEZE / 'gates/all-targets.log').read_text())
test_totals = {'summaries': len(totals), 'passed': sum(int(x[0]) for x in totals), 'failed': sum(int(x[1]) for x in totals), 'ignored': sum(int(x[2]) for x in totals)}
assert test_totals == {'summaries': 35, 'passed': 220, 'failed': 0, 'ignored': 4}
preserved = read(FREEZE / 'preserved-gates/manifest.json')
preserved_review = []
for command in preserved['checks']:
    label = command['label']
    assert command['exit_code'] == 0
    log = FREEZE / 'preserved-gates' / (label + '.log')
    assert sha(log) == command['log_sha256']
    inputs[str(log.relative_to(ROOT))] = sha(log)
    preserved_review.append({'label': label, **guard_pair(FREEZE / 'preserved-gates', label)})
callback = read(FREEZE / 'quote-callback/manifest.json')
assert callback['before'] == callback['after']
assert callback['before']['files'] == manifest['source_file_sha256']
assert callback['before']['binary_sha256'] == manifest['binary_sha256']
assert callback['result'] == 'passed' and len(callback['report']['checks']) == 13
callback_delivery = read(FREEZE / 'quote-callback/delivery-manifest.json')
check_files(callback_delivery['files'], FREEZE / 'quote-callback')
generation, callback_state = sqlite_state('docs/assets/accounting-m5/checks/candidate9/quote-callback/profile/sarv.sqlite3')
assert {'synthetic_only': True, 'generation': generation, 'state': callback_state} == read(FREEZE / 'quote-callback/profile/final-state.json')
assert generation == 12 and len(callback_state['draft']['lines']) == 211
quantity = sum(line['quantity_milli'] for line in callback_state['draft']['lines'])
assert quantity == 213000 and quantity * 1251 // 1000 == 266463
assert not callback_state['quotes'] and not callback_state.get('sales_drafts') and not callback_state.get('accounting', {}).get('journals')
main_dir = ROOT / 'artifacts/sales-native/run-5br6f6ku'
main = read(main_dir / 'sales-independent-native.json')
assert main['result'] == 'passed' and len(main['checks']) == 66 and all(c['passed'] for c in main['checks'])
source_dictionary = {k: manifest['frozen_source_provenance'][k] for k in ['sha256', 'files']}
assert main['frozen_source_provenance_before'] == main['frozen_source_provenance_after'] == source_dictionary
assert main['binary_sha256'] == manifest['binary_sha256']
assert main['observer_sha256'] == verification_files['scripts/sales-qa.py']
assert main['native_manifest'] == read(main_dir / 'full/sales-native-verification.json')
native_states = {}
for profile in ['full', 'stage', 'draft', 'detail-pages']:
    generation, state = sqlite_state(f'artifacts/sales-native/run-5br6f6ku/{profile}/sarv.sqlite3')
    native_states[profile] = {'generation': generation, **state_summary(state)}
    if profile == 'full':
        full = state
        (OUT / 'native-full-state.json').write_text(json.dumps({'generation': generation, 'state': state}, ensure_ascii=False, indent=2) + '\n')
balances = {}
for journal in full['accounting']['journals']:
    for line in journal['lines']:
        key = line['account']['id']
        balances[key] = balances.get(key, 0) + line['debit'] - line['credit']
mapping = full['accounting']['sales']['records'][0]['frozen_setup']['command']['mappings']
role_balances = {item['role']: balances.get(item['account_id'], 0) for item in mapping}
expected = {'Cash': 251, 'Bank': 300, 'Receivables': 299, 'Inventory': 422, 'ChequesReceivable': 200, 'Payables': -329, 'ChequesPayable': -100, 'Grni': -50, 'OwnerCapital': -952, 'Sales': -199, 'SalesReturns': 100, 'Cogs': 58}
assert all(role_balances[k] == v for k, v in expected.items())
assert sum(v for v in balances.values() if v > 0) == 1630
assert -sum(v for v in balances.values() if v < 0) == 1630
components = []
for record in full['accounting']['sales']['records']:
    kind, data = next(iter(record['data'].items()))
    components.append({'kind': kind, 'total': data['total'], 'cost': data.get('cost')})
assert components == [{'kind': 'Order', 'total': 199, 'cost': None}, {'kind': 'Fulfill', 'total': 100, 'cost': 58}, {'kind': 'Return', 'total': 40, 'cost': 23}, {'kind': 'Return', 'total': 60, 'cost': 35}, {'kind': 'Fulfill', 'total': 99, 'cost': 58}, {'kind': 'Order', 'total': 20, 'cost': None}]
identity_dir = ROOT / 'artifacts/sales-native/slot-reuse-28axdwy7'
identity = read(identity_dir / 'slot-reuse-native.json')
assert identity['result'] == 'passed' and len(identity['checks']) == 37 and all(c['passed'] for c in identity['checks'])
assert identity['source_before'] == identity['source_after'] == source_dictionary
assert identity['verification_before'] == identity['verification_after']
check_observers(identity['verification_before'])
check_files(identity['synthetic_input']['files'], identity_dir)
assert identity['binary_sha256'] == manifest['binary_sha256']
focus = [c for c in identity['checks'] if 'receipt' in c]
assert len(focus) == 3 and all(c['receipt']['dialog_focus_exact'] and c['receipt']['window_focus_exact'] for c in focus)
_, identity_state = sqlite_state('artifacts/sales-native/slot-reuse-28axdwy7/editor/sarv.sqlite3')
original = read(identity_dir / 'editor/synthetic-input-state.json')
assert identity_state['accounting'] == original['accounting']
lines = identity_state['sales_drafts'][0]['lines']
assert [x['id'] for x in lines] == [f'reuse-line-{i:03}' for i in range(24)] + ['reuse-line-027']
assert [x['quantity'] for x in lines] == [str(i + 1) for i in range(24)] + ['28']
(OUT / 'identity-final-state.json').write_text(json.dumps(identity_state, ensure_ascii=False, indent=2) + '\n')
perf_dir = ROOT / 'docs/assets/accounting-m5/performance/scale-pilot-candidate9-passed'
perf_provenance = read(perf_dir / 'source-provenance.json')
assert perf_provenance['functional_qualification_only'] and not perf_provenance['accepted_performance_measurement']
assert perf_provenance['before'] == perf_provenance['after']
assert perf_provenance['before']['files'] == manifest['source_file_sha256']
perf = read(perf_dir / 'performance.json')
native_perf = read(perf_dir / 'synthetic-profile/native-responsiveness-verification.json')
assert native_perf['result'] == 'passed' and len(native_perf['checks']) == 15
scopes = native_perf['additive_action_phases']
assert len(scopes) == 22 and all(scope['result'] == 'accepted' for scope in scopes)
for scope in scopes:
    assert abs(scope['elapsed_ms'] - (scope['ended_since_probe_ms'] - scope['started_since_probe_ms'])) < 0.0001
    assert abs(scope['max_gap_ms'] - max(scope['gaps_ms'] + [scope['terminal_gap_ms']])) < 0.0001
page_scopes = [scope for scope in scopes if 'page' in scope['operation']]
assert len(page_scopes) == 18
warnings = {}
for log in main_dir.glob('*.log'):
    matches = [line for line in log.read_text().splitlines() if 'WARNING' in line or 'CRITICAL' in line]
    if matches:
        inputs[str(log.relative_to(ROOT))] = sha(log)
        warnings[str(log.relative_to(ROOT))] = matches
review = {'result': 'passed bounded evidence inspection; performance remains PARTIAL and M5 OPEN',
          'scope': 'Read-only recomputation and raw SQLite inspection; no fresh application, financial probe, migration or GUI execution.',
          'production_source_sha256': manifest['production_source_sha256'], 'binary_sha256': manifest['binary_sha256'],
          'source_files': 36, 'verification_files': len(verification_files), 'source_comparisons': comparisons,
          'worktree_matches_C9_at_inspection': worktree_matches,
          'root_gates': gate_review, 'ordinary_tests': test_totals, 'preserved': preserved_review,
          'callback': {'checks': 13, 'generation': 12, 'quantity_milli': quantity, 'literal_total_rial': 266463, 'portable_files_hashed': len(callback_delivery['files'])},
          'native_main': {'checks': 66, 'states': native_states, 'role_balances': role_balances, 'trial_balance': [1630, 1630], 'components': components},
          'identity': {'checks': 37, 'pager_focus_receipts': focus, 'retained_ids': [x['id'] for x in lines], 'accounting_equals_original': True,
                       'stale_owned_window_errors': identity['stale_owned_window_errors'], 'unexpected_x11_errors': identity['unexpected_x11_errors']},
          'performance_diagnostic': {'native_checks': 15, 'accepted_scopes': 22, 'page_scopes': 18, 'page_gap_misses': sum(s['max_gap_ms'] > s['budget_ms'] for s in page_scopes),
                                     'missed_scopes': [{'operation': s['operation'], 'max_gap_ms': s['max_gap_ms'], 'budget_ms': s['budget_ms']} for s in scopes if s['max_gap_ms'] > s['budget_ms']],
                                     'reuse_wall_max_ms': max(perf['main_thread_trace_ms']['review_100_row_widget_reuse']), 'broad_strict_budgets': perf['strict_budgets'], 'accepted_final_measurement': False},
          'retained_native_warnings': warnings, 'inputs_sha256': inputs}
(OUT / 'independent-inspection.json').write_text(json.dumps(review, ensure_ascii=False, indent=2) + '\n')
print(json.dumps({k: review[k] for k in ['result', 'source_files', 'verification_files', 'ordinary_tests', 'source_comparisons', 'performance_diagnostic']}, indent=2))
