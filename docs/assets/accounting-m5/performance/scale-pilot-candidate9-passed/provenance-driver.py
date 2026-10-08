from pathlib import Path
import hashlib,json,subprocess,time

root=Path('/workspace/sarv-quote')
manifest_path=root/'docs/assets/accounting-m5/checks/candidate9/manifest.json'
manifest=json.loads(manifest_path.read_text())
binary=Path(manifest['binary'])
output=root/'artifacts/performance-m5/candidate9-scale-functional-pilot'
receipt_path=root/'artifacts/performance-m5/candidate9-scale-functional-provenance.json'
assert not output.exists() and not receipt_path.exists()

def sha(path): return hashlib.sha256(path.read_bytes()).hexdigest()
def snapshot():
    paths=sorted([*root.glob('src/**/*.rs'),root/'src/style.css',root/'Cargo.toml',root/'Cargo.lock'],
        key=lambda p:p.relative_to(root).as_posix().encode())
    digest=hashlib.sha256(); files={}
    for path in paths:
        name=path.relative_to(root).as_posix(); data=path.read_bytes()
        digest.update(name.encode()+b'\0'+data+b'\0'); files[name]=hashlib.sha256(data).hexdigest()
    return {'sha256':digest.hexdigest(),'files':files,'binary_sha256':sha(binary),
        'observers':{name:sha(root/'scripts'/name) for name in (
            'native-performance-M5.py','native-performance-M4.py','native-performance-M2.py',
            'accounting-performance.py','qa-native.py','qa_access.py')}}

before=snapshot()
assert before['sha256']==manifest['production_source_sha256']
assert before['files']==manifest['source_file_sha256']
assert before['binary_sha256']==manifest['binary_sha256']
receipt={'functional_qualification_only':True,'accepted_performance_measurement':False,
    'concurrency':'First candidate9 M2 functional qualification is quiet and isolated among owned work per root authorization; no owned concurrent apps/builds/gates. Existing user/older desktops remain untouched and shared virtual scheduling uncontrolled. Functional qualification only, not accepted final measurement.',
    'source_manifest':str(manifest_path),'manifest_sha256':sha(manifest_path),'before':before,
    'initial_C9_build_M5_observer_sha256':'472e83fa8231a77a32293f4b00713552ec61bec9d802c16b3ff76781b2b2a40d',
    'external_observer_epoch_M5_sha256':before['observers']['native-performance-M5.py'],
    'compiled_protocol':'m5-action-v1.2','external_protocol':'m5-cached-v3',
    'external_M2_observer_unchanged':True,'fresh_output':str(output),'utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime())}
receipt_path.write_text(json.dumps(receipt,indent=2)+'\n')
try:
    result=subprocess.run(['dbus-run-session','--','/usr/bin/python3',str(root/'scripts/native-performance-M2.py'),
        '--binary',str(binary),'--expect-sha',manifest['binary_sha256'],'--output',str(output)],cwd=root)
    receipt['driver_return_code']=result.returncode
    receipt['end_utc']=time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime())
finally:
    after=snapshot(); receipt['after']=after
    receipt['source_stable']=before['sha256']==after['sha256'] and before['files']==after['files']
    receipt['binary_stable']=before['binary_sha256']==after['binary_sha256']
    receipt['observers_stable']=before['observers']==after['observers']
    receipt_path.write_text(json.dumps(receipt,indent=2)+'\n')
assert receipt['source_stable'] and receipt['binary_stable'] and receipt['observers_stable']
raise SystemExit(result.returncode)
