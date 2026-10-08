"""Read-only C12 native37 immutable row-identity inspection."""
from pathlib import Path
exec(compile((Path(__file__).parent / 'inspect-build.py').read_text(), str(Path(__file__).parent / 'inspect-build.py'), 'exec'))
source_dictionary = {k: manifest['frozen_source_provenance'][k] for k in ['sha256','files']}
identity_dir = ROOT / 'artifacts/sales-native/slot-reuse-lsmu2c93'
identity = read(identity_dir / 'slot-reuse-native.json')
assert identity['result'] == 'passed' and len(identity['checks']) == 37 and all(c['passed'] for c in identity['checks'])
assert identity['source_before'] == identity['source_after'] == source_dictionary
assert identity['verification_before'] == identity['verification_after']
check_observers(identity['verification_before'])
check_files(identity['synthetic_input']['files'], identity_dir)
assert identity['binary_sha256'] == manifest['binary_sha256']
focus = [c for c in identity['checks'] if 'receipt' in c]
assert len(focus) == 3 and all(c['receipt']['dialog_focus_exact'] and c['receipt']['window_focus_exact'] for c in focus)
_, identity_state = sqlite_state('artifacts/sales-native/slot-reuse-lsmu2c93/editor/sarv.sqlite3')
original = read(identity_dir / 'editor/synthetic-input-state.json')
assert identity_state['accounting'] == original['accounting']
lines = identity_state['sales_drafts'][0]['lines']
assert [x['id'] for x in lines] == [f'reuse-line-{i:03}' for i in range(24)] + ['reuse-line-027']
assert [x['quantity'] for x in lines] == [str(i + 1) for i in range(24)] + ['28']
(OUT / 'identity-final-state.json').write_text(json.dumps(identity_state, ensure_ascii=False, indent=2) + '\n')

logs = {}
for log in identity_dir.glob('*.log'):
    diagnostics = [line for line in log.read_text().splitlines() if 'WARNING' in line or 'CRITICAL' in line]
    logs[str(log.relative_to(ROOT))] = {'sha256': sha(log), 'diagnostics': diagnostics}
review = {'result': 'PASS actual C12 current-ID37; final performance review still pending',
          'scope': 'Native current immutable-ID/slot-reuse proof plus raw durable SQLite; private synthetic X11 only.',
          'production_source_sha256': manifest['production_source_sha256'], 'binary_sha256': manifest['binary_sha256'],
          'checks': 37, 'source36_before_equals_after': True, 'observer_before_equals_after': True,
          'observer_sha256': identity['observer_sha256'], 'verification_files': identity['verification_before'],
          'pager_focus_receipts': focus, 'retained_ids': [x['id'] for x in lines], 'retained_quantities': [x['quantity'] for x in lines],
          'accounting_equals_original': True, 'stale_owned_window_errors': identity['stale_owned_window_errors'],
          'unexpected_x11_errors': identity['unexpected_x11_errors'], 'logs': logs, 'inputs_sha256': inputs}
(OUT / 'identity37-inspection.json').write_text(json.dumps(review, ensure_ascii=False, indent=2) + '\n')
print(json.dumps({'result': review['result'], 'checks':37,'pager_focus_receipts':focus,'accounting_equals_original': True},indent=2))
