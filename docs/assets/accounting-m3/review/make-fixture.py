#!/usr/bin/env python3
import csv
from pathlib import Path
header=['version','kind','id','company_id','name','role','amount','credit','unit','quantity','numerator','denominator','party_id','item_id','warehouse_id','unit_id','base_unit_id','customer','supplier','indivisible','reference','settlement_reference','bank_reference','holder','state','document_date','due_date','as_of','statement_amount']
rows=[]
def row(kind,id,**v):rows.append(dict(version='sarv-opening-v1',kind=kind,id=id,company_id='COMPANY',**v))
row('opening','independent-csv-opening',name='Independent literal supplied balances',as_of='2026-03-21')
for role,d,c in [('cash',251,0),('bank',300,0),('receivables',200,0),('inventory',300,0),('cheques_receivable',200,0),('payables',0,149),('cheques_payable',0,100),('grni',0,50),('owner_capital',0,952),('supplier_advances',0,0),('customer_advances',0,0),('cogs',0,0),('inventory_loss',0,0)]:row('ledger','ledger-'+role,role=role,amount=d,credit=c,unit='IRR',reference='literal-'+role)
for id in ['party-a','party-b']:row('party',id,name=id,customer='true',supplier='true')
row('unit','piece',name='piece');row('unit','carton',name='carton');row('item','item-a',name='item A',reference='A',base_unit_id='piece',indivisible='true');row('pack','carton-pack',item_id='item-a',unit_id='carton',numerator=12,denominator=1)
row('warehouse','w1',name='W1');row('warehouse','w2',name='W2')
row('cash','c1',name='C1',amount=251,unit='IRR',reference='count251')
row('bank','b1',name='B1',amount=300,statement_amount=300,unit='IRR',reference='statement300',bank_reference='bank-number-B1')
for k,id,p,ref,amt,doc,due in [('ar','ar-a','party-a','INV-A',125,'2026-02-01','2026-04-01'),('ar','ar-b','party-b','INV-B',75,'2026-02-02','2026-04-02'),('ap','ap-a','party-a','BILL-A',100,'2026-02-03','2026-04-03'),('ap','ap-b','party-b','BILL-B',49,'2026-02-04','2026-04-04')]:row(k,id,party_id=p,reference=ref,amount=amt,unit='IRR',document_date=doc,due_date=due)
row('cheque_receivable','cr1',party_id='party-a',reference='CR1',settlement_reference='SETTLED-OLD-INVOICE200',bank_reference='CR-bank',holder='C1',state='held',document_date='2026-02-10',due_date='2026-05-10',amount=200,unit='IRR')
row('cheque_payable','cp1',party_id='party-a',reference='CP1',settlement_reference='SETTLED-OLD-BILL100',bank_reference='CP-bank',holder='supplier A',state='issued',document_date='2026-02-11',due_date='2026-05-11',amount=100,unit='IRR')
row('stock','stock-w1',item_id='item-a',warehouse_id='w1',unit_id='carton',quantity='2',amount=240,unit='IRR',reference='count-2-cartons')
row('stock','stock-w2',item_id='item-a',warehouse_id='w2',unit_id='piece',quantity='6',amount=60,unit='IRR',reference='count6-includesR1')
row('grni','r1',party_id='party-a',item_id='item-a',warehouse_id='w2',unit_id='piece',quantity='5',amount=50,unit='IRR',reference='R1',document_date='2026-03-01')
with Path(__file__).with_name('fixture-template.csv').open('w',newline='') as f:
 w=csv.DictWriter(f,header);w.writeheader();w.writerows(rows)
print(len(rows),'literal rows')
