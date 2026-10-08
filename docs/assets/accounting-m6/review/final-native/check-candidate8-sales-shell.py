from pathlib import Path
import json,hashlib,sqlite3
ROOT=Path('/workspace/sarv-quote');OUT=ROOT/'docs/assets/accounting-m6/review/final-native';S=ROOT/'docs/assets/accounting-m6/native/sales-candidate8';B=ROOT/'docs/assets/accounting-m6/native/sidebar-candidate8';RAW=ROOT/'artifacts/sales-native/run-rqv4u0qx';checks=[]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def check(name,predicate):
 checks.append({'name':name,'passed':bool(predicate)});assert predicate,name
def state(p):
 with sqlite3.connect(f'file:{p}/sarv.sqlite3?mode=ro',uri=True) as c:
  check(str(p)+' integrity',c.execute('pragma integrity_check').fetchone()==('ok',));return json.loads(c.execute('select payload from state where id=1').fetchone()[0])
def controls(s):
 a=s['accounting'];mapping={x['account_id']:x['role'] for x in a['core']['setups'][0]['command']['mappings']};v=dict.fromkeys(mapping.values(),0)
 for j in a['journals']:
  check('balanced journal '+j['id'],sum(x['debit'] for x in j['lines'])==sum(x['credit'] for x in j['lines']))
  for x in j['lines']:v[mapping[x['account']['id']]]+=x['debit']-x['credit']
 check('net controls balanced',sum(v.values())==0);v['TB']=sum(max(x,0) for x in v.values());return v
base=dict(Cash=251,Bank=300,Receivables=200,ChequesReceivable=200,Inventory=480,SupplierAdvances=0,FixedAssets=0,Payables=-329,ChequesPayable=-100,Grni=-50,CustomerAdvances=0,Loans=0,OwnerCapital=-952,OwnerDrawings=0,RetainedEarnings=0,Sales=0,SalesReturns=0,Cogs=0,OperatingExpense=0,BankFees=0,InventoryLoss=0,TB=1431)
seed=state(RAW/'m4-before');check('seed allrole literal',controls(seed)==base)
sales=json.loads((S/'sales-independent-native.json').read_text());check('closed fresh Sales85',sales['result']=='passed' and len(sales['checks'])==85 and all(x['passed'] for x in sales['checks']))
check('active Sales completion observed exactly',next(x for x in sales['checks'] if x['name']=='completed Sales load clears global loading status')['passed'])
check('Sales production beforeafter equal',sales['frozen_source_provenance_before']==sales['frozen_source_provenance_after'])
check('native seed canonical hash exact',hashlib.sha256(json.dumps(seed,sort_keys=True,ensure_ascii=False,separators=(',',':')).encode()).hexdigest()==sales['seed_state_sha256'])
for name,expected in [('full',dict(base,Receivables=299,Inventory=422,Sales=-199,SalesReturns=100,Cogs=58,TB=1630)),('stage',dict(base,Receivables=260,Inventory=445,Sales=-100,SalesReturns=40,Cogs=35,TB=1531)),('draft',base),('detail-pages',base)]:
 s=state(RAW/name);check(name+' allrole literal',controls(s)==expected);a=s['accounting'];old=seed['accounting']
 for k in ['core','procurement','companies','periods','accounts']:check(name+' immutable '+k,a[k]==old[k])
 for k in ['journals','audit','requests']:check(name+' immutable '+k+' prefix',a[k][:len(old[k])]==old[k])
 if name=='full':
  rs=a['sales']['records'];kinds={kind:[r for r in rs if kind in r['data']] for kind in ['Order','Fulfill','Return']};check('full exactly2 orders2pairs2credits',all(len(v)==2 for v in kinds.values()) and len(a['journals'])==9 and not s.get('sales_drafts'))
  check('full exact fulfilled100/99 originalcost58',sorted(r['data']['Fulfill']['total'] for r in kinds['Fulfill'])==[99,100] and all(r['data']['Fulfill']['cost']==58 for r in kinds['Fulfill']))
  check('full exact original returns40/23 then60/35',sorted((r['data']['Return']['total'],r['data']['Return']['cost']) for r in kinds['Return'])==[(40,23),(60,35)])
  ids=[r['data'][k][i] for k,fields in [('Fulfill',['delivery_id','invoice_id']),('Return',['return_id','credit_id'])] for r in kinds[k] for i in fields];check('full8 distinct paired identities',len(set(ids))==8)
  linked=next(r for r in kinds['Order'] if r['command']['Order'].get('quotation'));link=linked['command']['Order']['quotation'];q=next(q for q in s['quotes'] if q['id']==link['quote_id']);check('authentic retained quote20 zero-tax order20',q['total_rial']==20 and q.get('tax_rial',0)==0 and linked['data']['Order']['total']==20 and link['quote_number']==q['number'])
log=(ROOT/'docs/assets/accounting-m6/checks/candidate8/sales/run.log').read_text();ix=log.rfind('\n{\n  "result"');logged,_=json.JSONDecoder().raw_decode(log[ix:].lstrip());check('entire logged report equals native receipt',logged==sales)
sm=json.loads((S/'portable-manifest.json').read_text());check('wrapperparser failure retained',sm['original_wrapper_result']=='failed-post-native-parser' and 'unavailable' in (ROOT/'docs/assets/accounting-m6/checks/candidate8/sales/wrapper-parser-failure.txt').read_text())
for folder,key in [(S,'portable_files'),(B,'portable_files')]:
 m=json.loads((folder/'portable-manifest.json').read_text());check(str(folder)+' allportable hashes exact',all(sha(folder/f)==h for f,h in m[key].items()))
shell=json.loads((B/'sidebar-verification.json').read_text());bm=json.loads((B/'portable-manifest.json').read_text());check('fresh shell34 closed',shell['result']=='passed' and len(shell['checks'])==34 and all(c['passed'] for c in shell['checks']));check('shell guarded beforeafter exact',bm['guard_before']==bm['guard_after'] and bm['guard_stable']);check('shell20portable count',len(bm['portable_files'])==20)
qualpath=ROOT/'docs/assets/accounting-m6/checks/candidate8/manifest.json';qual=json.loads(qualpath.read_text());check('shell manifest guard hash exact',sha(qualpath)==bm['guard_after']['manifest'])
for group in ['frozen_source_provenance','verification_source_provenance','compile_input_provenance']:
 check('current exact '+group,all(sha(ROOT/f)==h for f,h in qual[group]['files'].items()))
runtimepath=ROOT/'docs/assets/accounting-m6/checks/candidate8/runtime-verification1.json';runtime=json.loads(runtimepath.read_text());check('runtime1 exact all74 bytes',len(runtime['files'])==74 and all(sha(ROOT/f)==h for f,h in runtime['files'].items()));check('runtime1 manifest SHA exact',sha(runtimepath)==sm['current_runtime_manifest_sha256'])
check('candidate8 immutablebinary exact',sha(Path(sales['binary_source']))==sales['binary_sha256']==shell['binary_sha256']==qual['binary_sha256']);check('Sales observer/helpers exact',all(sha(ROOT/'scripts'/f)==h for f,h in sm['current_helpers'].items()));check('Shell helpers exact',all(sha(ROOT/f)==h for f,h in bm['guard_after']['helpers'].items()))
sv=['full/M5-sales-light-1280x900.png','full/M5-sales-dark-1280x900.png','full/M5-sales-light-650x550.png','full/M5-sales-dark-650x550.png'];bv=['sidebar-standard-light.png','sidebar-standard-dark.png','shell-wide-sidebar-hidden.png','shell-wide-sidebar-restored.png','shell-wide-hidden-page-change.png','sidebar-narrow-light-focus.png','sidebar-narrow-dark-focus.png','shell-narrow-close-restored-focus.png']
r={'reviewer':'m6_final_native_review','checks':checks,'passed':True,'sales_receipt_sha256':sha(S/'sales-independent-native.json'),'shell_receipt_sha256':sha(B/'sidebar-verification.json'),'sales_portable_file_count':len(sm['portable_files']),'personally_viewed_Sales_images':{f:sha(S/f) for f in sv},'personally_viewed_Shell_images':{f:sha(B/f) for f in bv},'original_wrapper_failed':True,'missing_original_guard_dicts_unavailable':True,'status_R03_scope':'Observed wrongpagePurchases fixed plus activeSales completed exactATSPi readycheck; not all completion interleavings','quiet_final_performance_accepted':False,'200visibility_accepted':False,'hold_review_activity_for_pilot':True}
(OUT/'candidate8-sales-shell-independent-result.json').write_text(json.dumps(r,indent=2)+'\n');print('PASS',len(checks),'bounded assertions; reviewHOLD resultSHA',sha(OUT/'candidate8-sales-shell-independent-result.json'))
