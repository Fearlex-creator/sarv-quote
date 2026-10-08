"""Inspect actual retained C12 preserved executions, without starting Sarv."""
from pathlib import Path
exec(compile((Path(__file__).parent / 'inspect-build.py').read_text(), str(Path(__file__).parent / 'inspect-build.py'), 'exec'))

directory = FREEZE / 'preserved-gates'
preserved = read(directory / 'manifest.json')
expected = {'core': 27, 'jalali': 44, 'sidebar': 26, 'ledger': 16, 'procurement': 30, 'quote': 23, 'held-save-close': 3}
assert len(preserved['checks']) == 7 and {c['label'] for c in preserved['checks']} == set(expected)
results = []
for command in preserved['checks']:
    label = command['label']
    assert command['exit_code'] == 0
    log = directory / (label + '.log')
    assert sha(log) == command['log_sha256']
    inputs[str(log.relative_to(ROOT))] = sha(log)
    guard = guard_pair(directory, label)
    raw = log.read_text()
    decoder = json.JSONDecoder()
    records = []
    for index, char in enumerate(raw):
        if char != '{':
            continue
        try:
            value, _ = decoder.raw_decode(raw[index:])
            if isinstance(value, dict) and value.get('result') == 'passed' and 'checks' in value:
                records.append(value)
        except ValueError:
            pass
    if label == 'quote':
        paths = re.findall(r'/workspace/sarv-quote/artifacts/[^\s]+', raw)
        receipt = read(Path(paths[-1]) / 'native-verification.json')
        assert receipt['result'] == 'passed'
        (OUT / 'preserved-quote-verification.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + '\n')
    else:
        receipt = records[-1]
    checks = receipt['checks']
    count = len(checks) if isinstance(checks, list) else checks
    assert count == expected[label]
    if isinstance(checks, list):
        assert all(not isinstance(c, dict) or c.get('passed') is True for c in checks)
    diagnostics = [line for line in raw.splitlines() if 'WARNING' in line or 'CRITICAL' in line]
    results.append({'label': label, 'exit_code': 0, 'checks': count, 'guards': guard, 'runner_log_material_diagnostics': diagnostics})
review = {'result': 'PASS actual complete preserved7; native83/37 and final performance still pending',
          'scope': 'Source36/binary plus16 selected-observer per-command guards; not all53 verification paths. Diagnostic scan covers retained runner logs.',
          'production_source_sha256': manifest['production_source_sha256'], 'binary_sha256': manifest['binary_sha256'],
          'checks': results, 'inputs_sha256': inputs}
(OUT / 'preserved-inspection.json').write_text(json.dumps(review, ensure_ascii=False, indent=2) + '\n')
print(json.dumps({'result': review['result'], 'counts': {x['label']: x['checks'] for x in results}, 'material_diagnostics_in_runner_logs': sum(len(x['runner_log_material_diagnostics']) for x in results)}, indent=2))
