from pathlib import Path
import hashlib,json,subprocess,time,sys,shutil
ROOT=Path('/workspace/sarv-quote'); manifest=Path(sys.argv[1]); expected=json.loads(manifest.read_text());out=manifest.resolve().parent/'preserved';assert not out.exists();out.mkdir(parents=True)
binary=Path(expected['binary_path']);binary_hash=expected['binary_sha256']
wrapper_paths=[Path(__file__),Path('/tmp/sarv-M6-preserved-quote.py')]
wrapper_before={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in wrapper_paths}
helpers=['core-qa.py','jalali-qa.py','native-sidebar-qa.py','accounting-qa.py','procurement-qa.py','sales-qa.py','qa-native.py','qa_access.py','native-save-close.py']
def guard():
 assert {str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in wrapper_paths}==wrapper_before,'Preserved coordinator changed'
 files={name:hashlib.sha256((ROOT/name).read_bytes()).hexdigest() for name in expected['frozen_source_provenance']['files']};assert files==expected['frozen_source_provenance']['files'];assert hashlib.sha256(binary.read_bytes()).hexdigest()==binary_hash;assert hashlib.sha256((ROOT/'target/release/sarv-quote').read_bytes()).hexdigest()==binary_hash
 return {name:hashlib.sha256((ROOT/'scripts'/name).read_bytes()).hexdigest() for name in helpers}
before=guard();report={'binary':str(binary),'binary_sha256':binary_hash,'production_source_sha256':expected['frozen_source_provenance']['sha256'],'wrapper_hashes_before':wrapper_before,'helper_hashes_before':before,'checks':[],'passed':False,'separate_sidebar_receipt':'docs/assets/accounting-m6/native/sidebar-candidate6/sidebar-verification.json','separate_sidebar_checks':34}
quote_receipt=out/'quote-source.json'
commands=[('core','dbus-run-session -- python3 scripts/core-qa.py'),('jalali','dbus-run-session -- python3 scripts/jalali-qa.py'),('ledger','dbus-run-session -- python3 scripts/accounting-qa.py'),('procurement','dbus-run-session -- python3 scripts/procurement-qa.py'),('sales','SARV_SALES_QA_FATAL_CRITICAL=1 SARV_SALES_SOURCE_MANIFEST='+str(manifest.resolve())+' dbus-run-session -- python3 scripts/sales-qa.py'),('quote','dbus-run-session -- python3 /tmp/sarv-M6-preserved-quote.py '+str(binary)+' '+binary_hash+' '+str(quote_receipt)),('held-save-close','SARV_CLOSE_TEST_BINARY='+str(binary)+' SARV_CLOSE_TEST_DIR='+str(ROOT/'artifacts/m6-preserved'/('held-save-close-'+manifest.parent.name))+' dbus-run-session -- python3 scripts/native-save-close.py')]
for label,command in commands:
 assert guard()==before
 log=out/(label+'.log');started=time.monotonic()
 with log.open('w') as f:result=subprocess.run(['bash','-c','source /workspace/sarv-environment/activate.sh\nexport SARV_QA_BINARY='+str(binary)+'\n'+command],cwd=ROOT,stdout=f,stderr=subprocess.STDOUT)
 receipt={'label':label,'command':command,'exit_code':result.returncode,'elapsed_seconds':time.monotonic()-started,'log_sha256':hashlib.sha256(log.read_bytes()).hexdigest()};report['checks'].append(receipt);print(json.dumps(receipt),flush=True);(out/'manifest.json').write_text(json.dumps(report,indent=2)+'\n')
 if result.returncode:print(log.read_text()[-2500:],flush=True);sys.exit(result.returncode)
 if label=='quote':
  profile=json.loads(quote_receipt.read_text())['profile'];commands.append(('quote-real-key-stress','dbus-run-session -- python3 scripts/qa-native.py --stress --profile '+profile))
assert guard()==before;report.update(passed=True,helper_hashes_after=guard(),wrapper_hashes_after=wrapper_before,production_binary_stable=True)
for path in [Path(__file__),Path('/tmp/sarv-M6-preserved-quote.py')]:shutil.copy2(path,out/path.name)
report['wrapper_hashes']={name:hashlib.sha256((out/name).read_bytes()).hexdigest() for name in ['sarv-M6-preserved.py','sarv-M6-preserved-quote.py']};(out/'manifest.json').write_text(json.dumps(report,indent=2)+'\n');print('PASS preserved native scripts, source/binary/helpers unchanged',flush=True)
