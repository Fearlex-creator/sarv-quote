"""Read-only independent arithmetic over retained JSON/SQLite, no Sarv imports."""
import csv
import hashlib
import json
from pathlib import Path
import sqlite3

ROOT = Path('/workspace/sarv-quote')
P = ROOT / 'docs/assets/accounting-m6/native/run-illjtffj'
OUT = ROOT / 'docs/assets/accounting-m6/review/final-native'
checks = []

def check(name, ok, actual=None):
    checks.append({'name': name, 'passed': bool(ok), 'actual': actual})
    assert ok, name

def controls(state, date):
    a = state['accounting']
    mapping = {x['account_id']: x['role'] for x in a['core']['setups'][0]['command']['mappings']}
    sums = dict.fromkeys(mapping.values(), 0)
    for j in a['journals']:
        if j['date'] > date:
            continue
        check('balanced journal ' + j['id'], sum(x['debit'] for x in j['lines']) == sum(x['credit'] for x in j['lines']))
        for line in j['lines']:
            sums[mapping[line['account']['id']]] += line['debit'] - line['credit']
    check('balanced net controls ' + date, sum(sums.values()) == 0)
    sums['TB'] = sum(max(x, 0) for x in sums.values())
    return sums

base = dict(Cash=251, Bank=300, Receivables=299, ChequesReceivable=200,
            Inventory=422, SupplierAdvances=0, FixedAssets=0, Payables=-329,
            ChequesPayable=-100, Grni=-50, CustomerAdvances=0, Loans=0,
            OwnerCapital=-952, OwnerDrawings=0, RetainedEarnings=0,
            Sales=-199, SalesReturns=100, Cogs=58, OperatingExpense=0,
            BankFees=0, InventoryLoss=0, TB=1630)

for branch in ('main', 'unbilled'):
    before = json.loads((P / f'M6-{branch}-before-state.json').read_text())
    after = json.loads((P / f'M6-{branch}-final-state.json').read_text())
    a, b = after['accounting'], before['accounting']
    db = ROOT / f'artifacts/purchase-returns-native/run-illjtffj/{branch}/sarv.sqlite3'
    with sqlite3.connect(f'file:{db}?mode=ro', uri=True) as conn:
        generation, payload = conn.execute('select generation,payload from state where id=1').fetchone()
    check(branch + ' raw SQLite matches final portable State', json.loads(payload) == after, generation)
    for key in ('catalogs', 'customers', 'quotes', 'draft'):
        check(branch + ' preserves ' + key, before[key] == after[key])
    for key in ('companies', 'periods', 'accounts', 'core', 'sales'):
        check(branch + ' preserves accounting ' + key, b[key] == a[key])
    for key in ('journals', 'requests', 'audit'):
        check(branch + ' preserves accounting prefix ' + key, a[key][:len(b[key])] == b[key])
    for key in ('records', 'requests'):
        check(branch + ' preserves procurement prefix ' + key, a['procurement'][key][:len(b['procurement'][key])] == b['procurement'][key])
    additions = a['procurement']['records'][len(b['procurement']['records']):]
    check(branch + ' exact new Return count', len(additions) == (2 if branch == 'main' else 1))
    if branch == 'main':
        expected = [('2026-03-31', base),
                    ('2026-04-01', dict(base, Inventory=377, Payables=-284, TB=1585)),
                    ('2026-04-02', dict(base, Inventory=317, Payables=-224, TB=1525))]
        outcomes = [(3000,45,25000,292,22000,247),(4000,60,22000,247,18000,187)]
    else:
        expected = [('2026-04-02', dict(base, Inventory=512, Payables=-359, Grni=-110, TB=1720)),
                    ('2026-04-03', dict(base, Inventory=497, Payables=-359, Grni=-95, TB=1705))]
        outcomes = [(1000,15,31000,382,30000,367)]
    for date, literal in expected:
        actual = controls(after, date)
        check(branch + ' literal controls ' + date, actual == literal, actual)
    for record, (qty,value,bq,bv,aq,av) in zip(additions,outcomes):
        cmd = record['command']['Return']; data = record['data']['Return']
        check(branch + ' exact return total/line', data['total'] == value and len(data['lines']) == 1 and data['lines'][0]['quantity_thousandths'] == qty and data['lines'][0]['value'] == value)
        movement = record['movements'][0]
        check(branch + ' original warehouse movement', len(record['movements']) == 1 and movement['direction']=='Out' and movement['quantity_thousandths']==qty and movement['value']==value and movement['before']['quantity_thousandths']==bq and movement['before']['value']==bv and movement['after']['quantity_thousandths']==aq and movement['after']['value']==av and movement['warehouse_id'].lower()=='w1')
        journal = next(j for j in a['journals'] if j['id']==record['journal_id'])
        mapping = {x['account_id']:x['role'] for x in a['core']['setups'][0]['command']['mappings']}
        pairs = [(mapping[x['account']['id']],x['debit'],x['credit']) for x in journal['lines']]
        check(branch + ' exact original-cost journal', pairs == [('Payables' if branch=='main' else 'Grni',value,0),('Inventory',0,value)], pairs)
        if branch=='main':
            check('billed lineage and separate credit states', data['return_state']=='Returned' and data['credit_state']=='Posted' and data['credit_id'] and data['credit_id']!=data['return_id'] and data['bill_id']=='f47b4602-6f1b-4afa-a45d-bdca0e54af48' and data['lines'][0]['receipt_line_id']=='ea16af10-b102-4265-8d4b-9c326ca14862' and data['lines'][0]['bill_line_id']=='66b0fc14-55dc-46ec-900f-0fc30d10d4d2')
        else:
            check('unbilled receipt lineage and no credit/bill', data['return_state']=='Returned' and data['credit_id'] is None and data['credit_state'] is None and data['bill_id'] is None and data['receipt_id']=='5a67b808-5206-4130-a689-0b5ad99fbc6c' and data['lines'][0]['receipt_line_id']=='newreceipt90-line' and data['lines'][0]['bill_line_id'] is None)

main = json.loads((P/'M6-main-final-state.json').read_text())
last_stock={}
for r in sorted(main['accounting']['procurement']['records']+main['accounting']['sales']['records'],key=lambda r:r['sequence']):
    for movement in r['movements']:
        last_stock[(movement['item_id'],movement['warehouse_id'])]=movement['after']
check('main final W1 literal18/187',last_stock[('ITEM-A','W1')]['quantity_thousandths']==18000 and last_stock[('ITEM-A','W1')]['value']==187)
check('main unchanged W2 literal12/130',last_stock[('ITEM-A','W2')]['quantity_thousandths']==12000 and last_stock[('ITEM-A','W2')]['value']==130)
record = main['accounting']['procurement']['records'][-2]
data = record['data']['Return']; command = record['command']['Return']
for name, kind, source in [('M6-actual-physical-return','purchase_return',data['return_id']),('M6-actual-supplier-credit','supplier_credit',data['credit_id'])]:
    with (P/(name+'.csv')).open(encoding='utf-8-sig',newline='') as f:
        rows=list(csv.DictReader(f))
    check(name+' distinct source identity', all(r['document_kind']==kind and r['report_source_id']==source for r in rows))
    metadata=[json.loads(r['snapshot_json']) for r in rows if r['row_kind']=='document_metadata']
    complete=[json.loads(r['snapshot_json']) for r in rows if r['row_kind']=='complete_snapshot']
    check(name+' metadata contains exact persisted record', len(metadata)==1 and metadata[0]==record)
    check(name+' complete snapshot contains exact persisted record',len(complete)==1 and complete[0]['document']==record)
    check(name+' exact frozen original allocation', any(r['amount_irr']=='45' and r['quantity_thousandths']=='3000' and r['original_value_irr']=='105' and r['original_quantity_thousandths']=='7000' and r['remaining_value_irr']=='60' and r['remaining_quantity_thousandths']=='4000' for r in rows))
    text=(P/(name+'.txt')).read_text()
    identities=[main['accounting']['companies'][0]['id'],'82ed70cd-c974-4f1c-8551-cb58d64259e9',data['return_id'],data['credit_id'],record['id'],record['journal_id'],data['bill_id'],data['lines'][0]['id'],data['lines'][0]['receipt_line_id'],data['lines'][0]['bill_line_id'],command['return_reference'],command['credit_reference'],'SKU-A','BILL-B','REC-B']
    check(name+' raw PDF preserves all exposed ASCII source identities', all(x in text for x in identities), identities)
    check(name+' localized numeric amount retained', '۴۵' in text)

receipt=json.loads((P/'purchase-returns-independent-native.json').read_text())
check('current Return observer exact to closed receipt',hashlib.sha256((ROOT/'scripts/purchase-returns-qa.py').read_bytes()).hexdigest()==receipt['observer_sha256'])
check('closed native receipt all78 PASS',len(receipt['checks'])==78 and receipt['result']=='passed' and all(x['passed'] for x in receipt['checks']))
check('closed native production before/after equal',receipt['source_before']==receipt['source_after'] and receipt['source_after']['sha256']=='11b48de2e85b9465a22f4a618e4675801b1db08a5a2c35b35cb892e8ea381cf3')
check('new native immutable binary hash exact',hashlib.sha256(Path(receipt['binary_source']).read_bytes()).hexdigest()==receipt['binary_sha256']=='4c552b8812c48630778e185ed52616ed0b5356d9beef91837adccefc01ba4e9d')
check('allcurrent production files exact',all(hashlib.sha256((ROOT/f).read_bytes()).hexdigest()==h for f,h in receipt['source_after']['files'].items()))
for fixture,h in receipt['seed_state_sha256_before'].items():
    with sqlite3.connect(f'file:{fixture}/sarv.sqlite3?mode=ro',uri=True) as conn:
        value=json.loads(conn.execute('select payload from state where id=1').fetchone()[0])
    current=hashlib.sha256(json.dumps(value,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()).hexdigest()
    check('protected fixture still exact '+fixture,current==h,current)
manifest=json.loads((P/'portable-manifest.json').read_text())
check('all48 portable hashes exact',len(manifest['files'])==48 and all(hashlib.sha256((P/f).read_bytes()).hexdigest()==h for f,h in manifest['files'].items()))
(OUT/'candidate8-independent-literals.json').write_text(json.dumps({'result':'passed','method':'standard-library JSON/CSV/SQLite arithmetic; no Sarv imports','checks':checks,'limits':'Native specific billed and unbilled synthetic cases only; no full PUR/INV/G4/PERF acceptance'},ensure_ascii=False,indent=2)+'\n')
print('PASS',len(checks),'independent assertions')
