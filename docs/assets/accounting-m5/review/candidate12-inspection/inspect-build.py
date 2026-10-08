"""Read-only review of retained C9 execution; no application or GUI execution."""
import hashlib
import json
from pathlib import Path
import re
import sqlite3

ROOT = Path(__file__).resolve().parents[5]
OUT = Path(__file__).resolve().parent
FREEZE = ROOT / "docs/assets/accounting-m5/checks/candidate12"
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
for epoch in ['candidate5', 'candidate9', 'candidate11-diagnostic']:
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
generation, callback_state = sqlite_state('docs/assets/accounting-m5/checks/candidate12/quote-callback/profile/sarv.sqlite3')
assert {'synthetic_only': True, 'generation': generation, 'state': callback_state} == read(FREEZE / 'quote-callback/profile/final-state.json')
assert generation == 12 and len(callback_state['draft']['lines']) == 211
quantity = sum(line['quantity_milli'] for line in callback_state['draft']['lines'])
assert quantity == 213000 and quantity * 1251 // 1000 == 266463
assert not callback_state['quotes'] and not callback_state.get('sales_drafts') and not callback_state.get('accounting', {}).get('journals')

financial_paths = ['src/accounting.rs','src/core.rs','src/domain.rs','src/lib.rs','src/procurement.rs','src/sales.rs','src/sales_draft.rs','src/store.rs','src/accounting_export.rs','src/core_export.rs','src/procurement_export.rs','src/sales_export.rs']
c5 = read(FREEZE.parent / 'candidate5/manifest.json')
assert all(c5['source_file_sha256'][p] == manifest['source_file_sha256'][p] for p in financial_paths)
review = {'result': 'PASS frozen C12 build, root gates and callback receipts; checkpoint approval pending',
          'scope': 'Read-only retained evidence inspection; no new application, financial probe, migration or GUI execution.',
          'production_source_sha256': manifest['production_source_sha256'], 'binary_sha256': manifest['binary_sha256'],
          'source_files': len(manifest['source_file_sha256']), 'verification_files': len(verification_files),
          'source_comparisons': comparisons, 'unchanged_C5_financial_paths': financial_paths,
          'worktree_matches_C12_at_inspection': worktree_matches,
          'root_gates': gate_review, 'ordinary_tests': test_totals,
          'callback': {'checks': 13, 'generation': generation, 'quantity_milli': quantity, 'literal_total_rial': 266463, 'portable_files_hashed': len(callback_delivery['files'])},
          'remaining': ['native83','current-ID37','complete preserved7','final performance report and independent analysis'],
          'inputs_sha256': inputs}
(OUT / 'build-gates-callback-inspection.json').write_text(json.dumps(review, ensure_ascii=False, indent=2) + '\n')
print(json.dumps({k: review[k] for k in ['result','source_files','verification_files','ordinary_tests','source_comparisons','callback']}, indent=2))
