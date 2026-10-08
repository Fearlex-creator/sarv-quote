#!/usr/bin/env python3
"""Literal 200-line raw-state/restart review; no production/native imports.

Supply before/after/restart JSON States or stopped SQLite files (mode=ro).
No inputs means preparation only, never current C9 posting acceptance.
"""
import argparse
import hashlib
import json
import sqlite3
from pathlib import Path

ROOT=Path(__file__).resolve().parents[5]
OWN=Path(__file__).resolve().parent
MANIFEST=ROOT/'docs/assets/accounting-m6/performance/dataset-candidate2/dataset-manifest.json'
DATASET=json.loads(MANIFEST.read_text())['datasets']['return200-billed']
RECIPE=Path(DATASET['recipe'])
recipe=json.loads(RECIPE.read_text())['recipe']
assert recipe['kind']=='billed' and recipe['line_count']==200
assert recipe['original_total_irr']==3000 and recipe['quantity_per_line_thousandths']==1000 and recipe['original_value_per_line_irr']==15
assert len(recipe['receipt_line_ids'])==len(set(recipe['receipt_line_ids']))==200
assert len(recipe['bill_line_ids'])==len(set(recipe['bill_line_ids']))==200

def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def load(path):
    path=Path(path).resolve()
    if path.is_dir():path=path/'sarv.sqlite3'
    if path.suffix in ('.sqlite3','.sqlite','.db'):
        with sqlite3.connect(path.as_uri()+'?mode=ro',uri=True) as db:
            assert db.execute('PRAGMA integrity_check').fetchone()==('ok',),f'integrity {path}'
            generation,payload=db.execute('SELECT generation,payload FROM state WHERE id=1').fetchone()
        return json.loads(payload),{'path':str(path),'kind':'SQLite mode=ro','integrity':'ok','generation':generation,'payload_sha256':hashlib.sha256(payload.encode() if isinstance(payload,str) else payload).hexdigest()}
    return json.loads(path.read_text()),{'path':str(path),'kind':'raw JSON State','file_sha256':sha(path)}

def controls(state):
    accounting=state['accounting'];balances={}
    for journal in accounting['journals']:
        assert sum(x['debit'] for x in journal['lines'])==sum(x['credit'] for x in journal['lines'])
        for line in journal['lines']:
            assert isinstance(line['debit'],int) and isinstance(line['credit'],int) and line['debit']>=0 and line['credit']>=0 and not(line['debit'] and line['credit'])
            key=line['account']['id'];balances[key]=balances.get(key,0)+line['debit']-line['credit']
    mapping={x['role']:x['account_id'] for x in accounting['core']['setups'][0]['command']['mappings']}
    values={k:balances.get(v,0) for k,v in mapping.items()}
    values.update(TB_debit=sum(max(v,0) for v in balances.values()),TB_credit=sum(max(-v,0) for v in balances.values()))
    # Fixture income totals have no unrelated earnings account adjustments.
    values['earnings']=-values['Sales']-values['SalesReturns']-values['Cogs']
    return values,mapping

def verify(before,after,restarted):
    checks=[]
    def check(name,value):
        assert value,name
        checks.append(name)
    prepared,_=load(DATASET['state'])
    check('before is exact immutable prepared full State',before==prepared)
    check('schema7 unchanged',before['schema']==after['schema']==restarted['schema']==7)
    check('complete stopped/restarted State exact',after==restarted)
    check('no unknown top-level keys',set(before)==set(after))
    for key in before.keys()-{'accounting'}:check('original whole State '+key,before[key]==after[key])
    a,b=before['accounting'],after['accounting']
    check('no unknown accounting keys',set(a)==set(b))
    for key in ('companies','periods','accounts','core','sales'):check('original accounting '+key,a[key]==b[key])
    for key in ('journals','requests','audit'):check('original '+key+' exact prefix',b[key][:len(a[key])]==a[key])
    check('no unknown procurement keys',set(a['procurement'])==set(b['procurement']))
    for key in ('records','requests'):check('original procurement '+key+' exact prefix',b['procurement'][key][:len(a['procurement'][key])]==a['procurement'][key])
    records=b['procurement']['records'][len(a['procurement']['records']):]
    check('exactly one new Return record',len(records)==1 and set(records[0]['data'])==set(records[0]['command'])=={'Return'})
    record=records[0];data=record['data']['Return'];command=record['command']['Return'];context=command['context']
    check('distinct Returned/Posted identities',bool(data['return_id']) and bool(data['credit_id']) and data['return_id']!=data['credit_id'] and data['return_state']=='Returned' and data['credit_state']=='Posted')
    check('original billed source fixed',data['bill_id']==recipe['bill_record_id'] and data['receipt_id'] is None and data['supplier_id']==command['supplier_id']=='SU-A')
    check('exact total/date/frozen context',data['total']==command['supplied_total']==3000 and context['date']==command['document_date']==recipe['return_date']=='2026-04-03' and context['source_revision']==1)
    setup=a['core']['setups'][0]
    check('original setup and company/period',context['setup_id']==setup['id'] and context['company_id']==setup['command']['company_id'] and context['period_id']==setup['command']['period_id'] and record['frozen_setup']==setup)
    check('command identities match typed record',command['return_id']==data['return_id'] and command['credit_id']==data['credit_id'])
    check('return/credit reference explicit',bool(command['return_reference']) and bool(command['credit_reference']) and command['return_reference']!=command['credit_reference'])
    lines=data['lines'];submitted=command['lines'];movements=record['movements']
    check('200 typed/command/movement rows',len(lines)==len(submitted)==len(movements)==200)
    check('200 unique line IDs and stock movement IDs',len({x['id'] for x in lines})==len({x['id'] for x in movements})==200)
    check('ordered original receipt IDs',[x['receipt_line_id'] for x in lines]==recipe['receipt_line_ids'])
    check('ordered original bill IDs',[x['bill_line_id'] for x in lines]==recipe['bill_line_ids'])
    for i,(line,cmd,movement) in enumerate(zip(lines,submitted,movements)):
        check(f'line{i+1} exact original quantity/value/item/warehouse',line['quantity_thousandths']==1000 and line['value']==15 and line['item_id']=='ITEM-A' and line['warehouse_id']=='W1')
        check(f'line{i+1} exact submitted source/value',cmd=={'id':line['id'],'receipt_line_id':recipe['receipt_line_ids'][i],'bill_line_id':recipe['bill_line_ids'][i],'quantity_thousandths':1000,'supplied_value':15})
        check(f'movement{i+1} exact linked original source',movement['source_line_id']==line['id'] and movement['item_id']=='ITEM-A' and movement['warehouse_id']=='W1' and movement['direction']=='Out' and movement['quantity_thousandths']==1000 and movement['value']==15)
        check(f'movement{i+1} literal pool transition',movement['before']=={'item_id':'ITEM-A','warehouse_id':'W1','quantity_thousandths':(225-i)*1000,'value':3292-15*i} and movement['after']=={'item_id':'ITEM-A','warehouse_id':'W1','quantity_thousandths':(224-i)*1000,'value':3277-15*i})
    journals=b['journals'][len(a['journals']):]
    check('one linked original-cost journal',len(journals)==1 and journals[0]['id']==record['journal_id'])
    journal=journals[0]
    before_controls,mapping=controls(before);after_controls,_=controls(after)
    for key,wanted in {'Inventory':3422,'Payables':-3329,'Grni':-50,'Receivables':299,'Sales':-199,'SalesReturns':100,'Cogs':58,'earnings':41,'TB_debit':4630,'TB_credit':4630}.items():check('literal before '+key,before_controls[key]==wanted)
    for key,wanted in {'Inventory':422,'Payables':-329,'Grni':-50,'Receivables':299,'Sales':-199,'SalesReturns':100,'Cogs':58,'earnings':41,'TB_debit':1630,'TB_credit':1630}.items():check('literal after '+key,after_controls[key]==wanted)
    check('only expected control changes',{key:after_controls[key]-value for key,value in before_controls.items() if key not in ('Inventory','Payables','TB_debit','TB_credit')}==dict.fromkeys([key for key in before_controls if key not in ('Inventory','Payables','TB_debit','TB_credit')],0))
    check('literal paired DrAP3000/CrInventory3000',[(x['account']['id'],x['debit'],x['credit']) for x in journal['lines']]==[(mapping['Payables'],3000,0),(mapping['Inventory'],0,3000)])
    for key in ('company_id','period_id','date','source_id','source_revision','actor','reason'):check('journal context '+key,journal[key]==context[key])
    check('journal exact event','procurement.return.v1'==journal['event'])
    for alias in b['requests'][len(a['requests']):]:
        check('new journal request alias same financial command',alias['journal_id']==journal['id'] and alias['command']['event']=='procurement.return.v1' and [(x['account_id'],x['debit'],x['credit']) for x in alias['command']['lines']]==[(mapping['Payables'],3000,0),(mapping['Inventory'],0,3000)] and alias['command']['source_id']==context['source_id'])
    check('new journal request exists',len(b['requests'])>len(a['requests']))
    for alias in b['procurement']['requests'][len(a['procurement']['requests']):]:check('new procurement aliases point only to Return',alias['record_id']==record['id'] and alias['company_id']==context['company_id'] and bool(alias['request_key']) and bool(alias['business_sha256']))
    check('new procurement request exists',len(b['procurement']['requests'])>len(a['procurement']['requests']))
    for event in b['audit'][len(a['audit']):]:check('new audit allowed JournalPosted only',event['operation']=='JournalPosted' and event['entity_id']==journal['id'] and event['company_id']==context['company_id'] and event['actor']==context['actor'] and event['reason']==context['reason'])
    check('posting audit exists',len(b['audit'])>len(a['audit']))
    # W2 opening is PIECE: sum all raw original movement deltas, no unit calculator.
    stock=[x for opening in a['core']['openings'] for x in opening['command']['stock'] if x['item_id']=='ITEM-A' and x['warehouse_id']=='W2']
    check('W2 initial base-piece opening',len(stock)==1 and stock[0]['unit_id']=='PIECE')
    quantity,value=stock[0]['quantity_thousandths'],stock[0]['value']
    for ledger in ('procurement','sales'):
        for item in a[ledger]['records']:
            for movement in item.get('movements',[]):
                if movement['item_id']=='ITEM-A' and movement['warehouse_id']=='W2':
                    quantity+=movement['after']['quantity_thousandths']-movement['before']['quantity_thousandths'];value+=movement['after']['value']-movement['before']['value']
    check('W2 unchanged12pieces/130',quantity==12000 and value==130 and all(x['warehouse_id']=='W1' for x in movements))
    return {'checks':checks,'checks_passed':len(checks),'controls_before':before_controls,'controls_after':after_controls,'return_id':data['return_id'],'credit_id':data['credit_id'],'record_id':record['id'],'journal_id':journal['id'],'all200_original_links_movements_exact':True,'all_original_whole_State_prefixes_and_quotes_exact':True,'entire_State_after_restart_exact':True,'W1_after':{'quantity_thousandths':25000,'value':292},'W2_unchanged':{'quantity_thousandths':12000,'value':130}}

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--before');parser.add_argument('--after');parser.add_argument('--restart');parser.add_argument('--output',default=str(OWN/'candidate9-200-raw-plan.json'))
    args=parser.parse_args();output=Path(args.output).resolve();assert output.parent==OWN,'Reviewer output must remain in owned directory'
    receipt={'reviewer':'m6_observer_native_reviewer','recipe_sha256':sha(RECIPE),'dataset_manifest_sha256':sha(MANIFEST),'declared_fixture':'return200-billed','production_calculators_or_GUI_imported':False}
    if not any((args.before,args.after,args.restart)):
        receipt.update(result='PREPARED ONLY; current C9 post/restart UNTESTED',ordered_receipt_ids=recipe['receipt_line_ids'],ordered_bill_ids=recipe['bill_line_ids'],literal_total_irr=3000,expected_pool_before={'W1':[225000,3292],'W2':[12000,130]},expected_pool_after={'W1':[25000,292],'W2':[12000,130]},required='Before exact prepared dataset; all200 Q1000/V15 source/command/movement links and pool transitions; one DrAP/CrInventory3000 journal; distinct ReturnReturned/CreditPosted; only legitimate linked aliases/audit; original whole-State/quotes/prefixes unchanged; full State exact after real restart',no_actual_post_acceptance=True)
    else:
        assert all((args.before,args.after,args.restart)),'Provide all before/after/restart artifacts'
        before,one=load(args.before);after,two=load(args.after);restart,three=load(args.restart)
        receipt.update(result='PASS independent raw200 financial and supplied restart-State comparison',inputs=[one,two,three],**verify(before,after,restart),limitation='Requires separate independent observer/provenance evidence that supplied artifacts came from actual C9 native post and real process restart; this checker alone does not prove UI, restart orchestration or performance')
    output.write_text(json.dumps(receipt,ensure_ascii=False,indent=2)+'\n');print(json.dumps({'result':receipt['result'],'output':str(output)},ensure_ascii=False))

if __name__=='__main__':main()
