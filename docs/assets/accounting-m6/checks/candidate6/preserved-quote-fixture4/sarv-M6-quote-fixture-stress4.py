from pathlib import Path
import json,hashlib,sqlite3,subprocess,time,shutil,tempfile
ROOT=Path('/workspace/sarv-quote');base=ROOT/'docs/assets/accounting-m6/checks/candidate6';qualification=json.loads((base/'manifest.json').read_text());source=Path(json.loads((base/'preserved/quote-source.json').read_text())['profile']);out=base/'preserved-quote-fixture4';assert not out.exists();out.mkdir(parents=True);profile=Path(tempfile.mkdtemp(prefix='quote-rows-',dir=ROOT/'artifacts/m6-preserved'))
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
library=ROOT/'target/debug/libsarv_quote.rlib';helper=Path('/tmp/sarv-M6-prepare-quote');helper_source=Path('/tmp/sarv-M6-prepare-quote.rs');helper_before={str(p):sha(p) for p in [library,helper,helper_source]}
prepared=subprocess.run([str(helper),str(profile)],capture_output=True,text=True,check=True);(out/'fixture-helper.log').write_text(prepared.stdout+prepared.stderr)
s=state(profile);draft=s['draft'];q=sourcebefore['quotes'][-1];assert len(draft['lines'])>=2
assert {k:v for k,v in s.items() if k not in ['draft','quotes']}=={k:v for k,v in sourcebefore.items() if k not in ['draft','quotes']}
assert s['quotes'][:len(sourcebefore['quotes'])]==sourcebefore['quotes'];assert len(s['quotes']) in [len(sourcebefore['quotes']),len(sourcebefore['quotes'])+1]
recipe={'synthetic_only':True,'typed_Store_revision_approval_fixture':True,'source':str(source),'profile':str(profile),'source_state_sha256':hashlib.sha256(json.dumps(sourcebefore,sort_keys=True,ensure_ascii=False,separators=(',',':')).encode()).hexdigest(),'draft':draft,'source_quote_id':q['id'],'row_count':len(draft['lines']),'original_whole_state_prefix_exact':True,'all_other_payload_exact':True,'helper_hashes':helper_before,'source_native_receipt_scope':'Copied original23-check programmatic proof; not a claim that modified draft reran that proof. Actual Store revise/approve prepares a four-row fixture afterward, asserting exact same-content approval ID/count. Original history remains a complete prefix; all other payload unchanged. Keyboard observer tests this prepared approved draft.'}
(out/'fixture-recipe.json').write_text(json.dumps(recipe,ensure_ascii=False,indent=2)+'\n');(out/'before-state.json').write_text(json.dumps(s,ensure_ascii=False,indent=2)+'\n')
report={'passed':False,'binary_sha256':qualification['binary_sha256'],'production_source_sha256':qualification['frozen_source_provenance']['sha256'],'observer_sha256':helperhash,'wrapper_sha256':selfhash,'fixture_recipe':str(out/'fixture-recipe.json'),'preserved_prior_failure':'docs/assets/accounting-m6/checks/candidate6/preserved/quote-real-key-stress.log','prior_failed_runs':['artifacts/independent-native/run-o5kjxvkv','artifacts/independent-native/run-ca8mvfpn'],'scope':'One real-key stress invocation with explicit current four-row synthetic draft; previously failed empty-draft run retained'};command='source /workspace/sarv-environment/activate.sh\ndbus-run-session -- python3 scripts/qa-native.py --stress --profile '+str(profile)
start=time.monotonic()
with (out/'test.log').open('w') as f:r=subprocess.run(['bash','-c',command],cwd=ROOT,stdout=f,stderr=subprocess.STDOUT)
guard();assert {str(p):sha(p) for p in [library,helper,helper_source]}==helper_before;shutil.copy2(helper_source,out/helper_source.name);report['fixture_helper_hashes']=helper_before;report.update(command=command,exit_code=r.returncode,elapsed_seconds=time.monotonic()-start,source_state_unchanged=True,source_binary_observer_wrapper_unchanged=True,test_log_sha256=sha(out/'test.log'),passed=r.returncode==0);shutil.copy2(__file__,out/Path(__file__).name);(out/'manifest.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report),flush=True);raise SystemExit(r.returncode)
