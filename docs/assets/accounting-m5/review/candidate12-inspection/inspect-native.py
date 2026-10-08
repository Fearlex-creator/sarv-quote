"""Read-only raw C12 native83 inspection; no GTK application execution."""
from pathlib import Path
exec(compile((Path(__file__).parent / 'inspect-build.py').read_text(), str(Path(__file__).parent / 'inspect-build.py'), 'exec'))
main_dir = ROOT / 'artifacts/sales-native/run-yji_0ewe'
main = read(main_dir / 'sales-independent-native.json')
assert main['result'] == 'passed' and len(main['checks']) == 83 and all(c['passed'] for c in main['checks'])
source_dictionary = {k: manifest['frozen_source_provenance'][k] for k in ['sha256', 'files']}
assert main['frozen_source_provenance_before'] == main['frozen_source_provenance_after'] == source_dictionary
assert main['binary_sha256'] == manifest['binary_sha256']
assert main['observer_sha256'] == verification_files['scripts/sales-qa.py']
assert main['native_manifest'] == read(main_dir / 'full/sales-native-verification.json')
native_states = {}
for profile in ['full', 'stage', 'draft', 'detail-pages']:
    generation, state = sqlite_state(f'artifacts/sales-native/run-yji_0ewe/{profile}/sarv.sqlite3')
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

assert main['fatal_criticals'] is True and main['material_native_diagnostics'] == []
assert main['x11_errors'] == []
focus = [c for c in main['checks'] if c.get('receipt') and 'detail pager' in c['name']]
assert len(focus) == 2 and [c['receipt']['offset'] for c in focus] == [25, 0]
assert [c['receipt']['target_name'] for c in focus] == ['sales-detail-previous', 'sales-detail-next']
assert all(all(c['receipt'][k] for k in ['grab_accepted','mapped','sensitive','dialog_focus_exact','window_focus_exact']) for c in focus)
assert [json.loads(line) for line in (main_dir / 'detail-pages/sales-detail-pager-focus.jsonl').read_text().splitlines()] == [c['receipt'] for c in focus]
inputs[str((main_dir / 'detail-pages/sales-detail-pager-focus.jsonl').relative_to(ROOT))] = sha(main_dir / 'detail-pages/sales-detail-pager-focus.jsonl')
narrow = [c for c in main['checks'] if c['name'].startswith('narrow ')]
assert len(narrow) == 12 and all(c['passed'] for c in narrow)
logs = {}
for log in main_dir.glob('*.log'):
    if log.name.startswith('M5-'):
        diagnostics = [line for line in log.read_text().splitlines() if 'WARNING' in line or 'CRITICAL' in line]
        assert not diagnostics, log.name
        logs[str(log.relative_to(ROOT))] = {'sha256': sha(log), 'diagnostics': diagnostics}
review = {'result': 'PASS actual C12 native83; identity37 and final performance still pending',
          'scope': main['scope'], 'production_source_sha256': manifest['production_source_sha256'], 'binary_sha256': manifest['binary_sha256'],
          'checks': 83, 'source36_before_equals_after': True, 'observer_sha256': main['observer_sha256'],
          'fatal_criticals': True, 'material_native_diagnostics': [], 'x11_errors': [],
          'states': native_states, 'role_balances': role_balances, 'trial_balance': [1630,1630], 'components': components,
          'detail_pager_focus': focus, 'narrow_actual_keyboard_checks': narrow, 'native_application_logs': logs,
          'limits': ['Actual private X11 synthetic execution only; no general warning-free/platform/accessibility claim.', 'Independent financial/data7 remains C5 execution on unchanged implementation; this fresh C12 native fixture is separate.', 'No performance or local checkpoint acceptance until remaining receipts are reviewed.'],
          'inputs_sha256': inputs}
(OUT / 'native83-inspection.json').write_text(json.dumps(review, ensure_ascii=False, indent=2) + '\n')
print(json.dumps({'result': review['result'], 'checks': 83, 'states': native_states, 'trial_balance': [1630,1630], 'material_native_diagnostics': [], 'detail_pager_focus': focus}, indent=2))
