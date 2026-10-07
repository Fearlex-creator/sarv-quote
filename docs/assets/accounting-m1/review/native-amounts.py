"""Independent literal expected outputs; read only a reviewer-created synthetic run."""
import csv, json, sqlite3, subprocess, sys
from pathlib import Path
profile = Path(sys.argv[1]).resolve()
with sqlite3.connect(f'file:{profile}/sarv.sqlite3?mode=ro', uri=True) as db:
    state = json.loads(db.execute('SELECT payload FROM state WHERE id=1').fetchone()[0])
    assert db.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
assert 'شرکت ساختگی' in state['accounting']['companies'][0]['name']
ledger = state['accounting']
assert len(ledger['journals']) == 3
assert [sum(line['debit'] for line in j['lines']) for j in ledger['journals']] == [1251, 349, 349]
balances = {}
for journal in ledger['journals']:
    assert sum(line['debit'] for line in journal['lines']) == sum(line['credit'] for line in journal['lines'])
    for line in journal['lines']:
        code = line['account']['code']
        balances[code] = balances.get(code, 0) + line['debit'] - line['credit']
assert balances == {'1001': 1251, '3001': -1251}
assert ledger['journals'][2]['kind']['Reversal']['journal_id'] == ledger['journals'][1]['id']
for filename, amount in [('trial-before-reversal-rial.pdf', '1600'), ('trial-final-toman.pdf', '125.1')]:
    assert amount in subprocess.check_output(['pdftotext', str(profile / filename), '-'], text=True)
rows = list(csv.DictReader((profile / 'trial-final-toman.csv').open(encoding='utf-8-sig')))
assert sorted((r['debit_irr'], r['credit_irr'], r['display_debit'], r['display_credit']) for r in rows) == [('0', '1251', '0', '125.1'), ('1251', '0', '125.1', '0')]
print(json.dumps({'result': 'passed', 'expected_journal_totals_irr': [1251, 349, 349], 'final_net_irr': balances, 'integrity_check': 'ok', 'exact_pdf_csv_units': True}, ensure_ascii=False))
