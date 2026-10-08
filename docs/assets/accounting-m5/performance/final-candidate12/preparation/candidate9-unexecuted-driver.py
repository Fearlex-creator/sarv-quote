from pathlib import Path
import hashlib,json,os,subprocess,time
root=Path('/workspace/sarv-quote')
manifest_path=root/'docs/assets/accounting-m5/checks/candidate9/manifest.json'
verification_path=root/'docs/assets/accounting-m5/checks/candidate9/verification-source-before.json'
manifest=json.loads(manifest_path.read_text());verification=json.loads(verification_path.read_text())
binary=Path(manifest['binary']);own=Path(__file__).resolve()
receipt_path=root/'artifacts/performance-m5/final-candidate9-source-provenance.json'
outputs=[root/'artifacts/performance-m5/final-candidate9',root/'artifacts/performance-m5/final-candidate9-scale']
assert not receipt_path.exists() and not any(p.exists() for p in outputs),'Fresh final output required'
assert not os.environ.get('SARV_SALES_FOCUS_QA'),'Diagnostic focus receipts forbidden during benchmark'
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def utc():return time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime())
def snapshot():
 paths=sorted([*root.glob('src/**/*.rs'),root/'src/style.css',root/'Cargo.toml',root/'Cargo.lock'],key=lambda p:p.relative_to(root).as_posix().encode())
 digest=hashlib.sha256();files={}
 for path in paths:
  name=path.relative_to(root).as_posix();data=path.read_bytes();digest.update(name.encode()+b'\0'+data+b'\0');files[name]=hashlib.sha256(data).hexdigest()
 return {'sha256':digest.hexdigest(),'files':files,'binary_sha256':sha(binary),'observers':{name:sha(root/'scripts'/name) for name in ('native-performance-M5.py','native-performance-M4.py','native-performance-M2.py','accounting-performance.py','qa-native.py','qa_access.py')},'verification_file_sha256':{name:sha(root/name) for name in verification},'wrapper_sha256':sha(own),'manifest_sha256':sha(manifest_path),'verification_manifest_sha256':sha(verification_path)}
def exact(s):
 assert len(s['files'])==36 and len(s['verification_file_sha256'])==52
 assert s['sha256']==manifest['production_source_sha256'] and s['files']==manifest['source_file_sha256']
 assert s['binary_sha256']==manifest['binary_sha256'] and s['verification_file_sha256']==verification
 assert s['observers']['native-performance-M5.py']=='472e83fa8231a77a32293f4b00713552ec61bec9d802c16b3ff76781b2b2a40d'
 assert s['observers']['native-performance-M2.py']=='579a255004d9a98ab94611e45873c13c65de9a0dcf609b355b24c3dd44e040c5'
before=snapshot();exact(before)
receipt={'functional_qualification_only':False,'authorized_final_measurement_epoch':True,'performance_acceptance':'pending strict analysis','concurrency':'Root explicit quiet final GO; owned GUI/build/test/render work stopped. Existing user/older desktops untouched and uncontrolled; shared virtual scheduling uncontrolled. One final changed-source epoch; no repeated epoch to waive misses. Small progress/status reads only during measurement.','source_manifest':str(manifest_path),'verification_manifest':str(verification_path),'before':before,'initial_C9_build_M5_observer_sha256':before['observers']['native-performance-M5.py'],'external_observer_epoch_M5_sha256':before['observers']['native-performance-M5.py'],'compiled_protocol':'m5-action-v1.2','external_protocol':'m5-cached-v3','external_M2_observer_unchanged':True,'diagnostic_focus_receipts_enabled':False,'fresh_outputs':list(map(str,outputs)),'utc':utc(),'runs':[]}
def write():receipt_path.write_text(json.dumps(receipt,indent=2)+'\n')
write()
returncode=1
try:
 for index,(script,output,log) in enumerate([('native-performance-M5.py',outputs[0],'/tmp/sarv-m5-final-candidate9.log'),('native-performance-M2.py',outputs[1],'/tmp/sarv-m5-final-candidate9-scale.log')]):
  snap=snapshot();exact(snap);assert snap==before,'Guard changed before sequential suite'
  args=['dbus-run-session','--','/usr/bin/python3',str(root/'scripts'/script),'--binary',str(binary),'--expect-sha',manifest['binary_sha256'],'--output',str(output)]
  if index==0:args+=['--source-manifest',str(manifest_path),'--samples','10','--search-samples','20']
  run={'script':script,'command':args,'before':snap,'start_utc':utc()};receipt['runs'].append(run);write()
  with open(log,'xb') as stream:result=subprocess.run(args,cwd=root,stdout=stream,stderr=subprocess.STDOUT)
  run.update({'driver_return_code':result.returncode,'end_utc':utc(),'after':snapshot(),'observer_log':log});write()
  exact(run['after']);assert run['before']==run['after'],'Guard changed during suite'
  returncode=result.returncode
  if returncode:break
finally:
 after=snapshot();receipt.update({'after':after,'end_utc':utc(),'source_stable':before['sha256']==after['sha256'] and before['files']==after['files'],'binary_stable':before['binary_sha256']==after['binary_sha256'],'observers_stable':before['observers']==after['observers'],'full_verification_stable':before['verification_file_sha256']==after['verification_file_sha256'],'wrapper_stable':before['wrapper_sha256']==after['wrapper_sha256'],'all_snapshot_guards_stable':before==after,'driver_return_code':returncode});write()
assert receipt['all_snapshot_guards_stable']
raise SystemExit(returncode)
