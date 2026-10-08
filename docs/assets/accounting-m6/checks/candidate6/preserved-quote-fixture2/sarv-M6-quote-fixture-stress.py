from pathlib import Path
import json,hashlib,sqlite3,subprocess,time,shutil,tempfile
ROOT=Path('/workspace/sarv-quote');base=ROOT/'docs/assets/accounting-m6/checks/candidate6';qualification=json.loads((base/'manifest.json').read_text());source=Path(json.loads((base/'preserved/quote-source.json').read_text())['profile']);out=base/'preserved-quote-fixture2';assert not out.exists();out.mkdir(parents=True);profile=Path(tempfile.mkdtemp(prefix='quote-rows-',dir=ROOT/'artifacts/m6-preserved'))
def state(p):
 with sqlite3.connect('file:'+str(p/'sarv.sqlite3')+'?mode=ro',uri=True) as db:return json.loads(db.execute('SELECT payload FROM state WHERE id=1').fetchone()[0])
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
selfhash=sha(Path(__file__));helperhash=sha(ROOT/'scripts/qa-native.py');sourcebefore=state(source)
def guard():
 assert state(source)==sourcebefore
 assert sha(Path(__file__))==selfhash and sha(ROOT/'scripts/qa-native.py')==helperhash
 assert {n:sha(ROOT/n) for n in qualification['frozen_source_provenance']['files']}==qualification['frozen_source_provenance']['files']
 assert sha(ROOT/'target/release/sarv-quote')==qualification['binary_sha256']
guard()
with sqlite3.connect('file:'+str(source/'sarv.sqlite3')+'?mode=ro',uri=True) as src:
 with sqlite3.connect(profile/'sarv.sqlite3') as dst:src.backup(dst)
shutil.copy2(source/'native-verification.json',profile/'native-verification.json')
s=state(profile);q=s['quotes'][-1];draft=dict(s['draft']);draft.update({k:q[k] for k in draft if k in q and k!='lines'});draft['lines']=[{'query':line['query'],'quantity_milli':line['quantity_milli'],'chosen_code':line['code']} for line in q['lines']];assert len(draft['lines'])>=2;s['draft']=draft;assert {k:v for k,v in s.items() if k!='draft'}=={k:v for k,v in sourcebefore.items() if k!='draft'}
with sqlite3.connect(profile/'sarv.sqlite3') as db:
 generation=db.execute('SELECT generation FROM state WHERE id=1').fetchone()[0];assert db.execute('UPDATE state SET generation=?,payload=? WHERE id=1 AND generation=?',(generation+1,json.dumps(s,ensure_ascii=False,separators=(',',':')),generation)).rowcount==1
recipe={'synthetic_only':True,'raw_draft_fixture_only':True,'source':str(source),'profile':str(profile),'source_state_sha256':hashlib.sha256(json.dumps(sourcebefore,sort_keys=True,ensure_ascii=False,separators=(',',':')).encode()).hexdigest(),'draft':draft,'source_quote_id':q['id'],'row_count':len(draft['lines']),'all_non_draft_payload_exact':True,'source_native_receipt_scope':'Copied original23-check programmatic proof; not a claim that modified draft reran that proof. Explicit four-row revision-shaped fixture prepared afterward for real key stress. Actual UI/Store validates fixture on startup and save.'}
(out/'fixture-recipe.json').write_text(json.dumps(recipe,ensure_ascii=False,indent=2)+'\n');(out/'before-state.json').write_text(json.dumps(s,ensure_ascii=False,indent=2)+'\n')
report={'passed':False,'binary_sha256':qualification['binary_sha256'],'production_source_sha256':qualification['frozen_source_provenance']['sha256'],'observer_sha256':helperhash,'wrapper_sha256':selfhash,'fixture_recipe':str(out/'fixture-recipe.json'),'preserved_prior_failure':'docs/assets/accounting-m6/checks/candidate6/preserved/quote-real-key-stress.log','prior_failed_run':'artifacts/independent-native/run-o5kjxvkv','scope':'One real-key stress invocation with explicit current four-row synthetic draft; previously failed empty-draft run retained'};command='source /workspace/sarv-environment/activate.sh\ndbus-run-session -- python3 scripts/qa-native.py --stress --profile '+str(profile)
start=time.monotonic()
with (out/'test.log').open('w') as f:r=subprocess.run(['bash','-c',command],cwd=ROOT,stdout=f,stderr=subprocess.STDOUT)
guard();report.update(command=command,exit_code=r.returncode,elapsed_seconds=time.monotonic()-start,source_state_unchanged=True,source_binary_observer_wrapper_unchanged=True,test_log_sha256=sha(out/'test.log'),passed=r.returncode==0);shutil.copy2(__file__,out/Path(__file__).name);(out/'manifest.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report),flush=True);raise SystemExit(r.returncode)
