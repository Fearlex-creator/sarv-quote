from pathlib import Path
import subprocess, os, json, hashlib, shutil, time, re
ROOT=Path('/workspace/sarv-quote')
ENVROOT=Path('/workspace/sarv-environment')
BINARY=Path('/workspace/sarv-evidence/bin/sarv-M3-final')
EXPECTED='10bf9929d00cdf98280a51a2c12860ec39939cdbe8620eb2d827b2bb1de6cee1'
def digest(p): return hashlib.sha256(p.read_bytes()).hexdigest()
assert digest(BINARY)==EXPECTED
assert digest(ROOT/'target/release/sarv-quote')==EXPECTED
out=ROOT/'artifacts/accounting-m3-regression'/time.strftime('final-10bf9929-%Y%m%dT%H%M%SZ',time.gmtime())
out.mkdir(parents=True,exist_ok=False)
env=os.environ.copy()
env.pop('DISPLAY',None)
env['SARV_QA_BINARY']=str(BINARY)
env['SARV_CLOSE_TEST_BINARY']=str(BINARY)
env['SARV_CLOSE_TEST_DIR']=str(out/'held-save-close-profile')
env['XDG_RUNTIME_DIR']=str(ENVROOT/'runtime')
commands=[('quote',ENVROOT/'check-native.py',['/usr/bin/python3']),('accounting',ROOT/'scripts/accounting-qa.py',['dbus-run-session','--','/usr/bin/python3']),('jalali',ROOT/'scripts/jalali-qa.py',['dbus-run-session','--','/usr/bin/python3']),('sidebar',ROOT/'scripts/native-sidebar-qa.py',['dbus-run-session','--','/usr/bin/python3']),('held-save-close',ROOT/'scripts/native-save-close.py',['/usr/bin/python3'])]
summary={'result':'running','binary':str(BINARY),'binary_sha256':EXPECTED,'source_edits':False,'harness_edits':False,'output':str(out),'runs':[]}
print(f'Output: {out}',flush=True)
for name,script,prefix in commands:
    start=time.monotonic()
    log=out/(name+'.log')
    with log.open('w') as f:
        process=subprocess.run(prefix+[str(script)],cwd=ROOT,env=env,stdout=f,stderr=subprocess.STDOUT)
    entry={'name':name,'exit_code':process.returncode,'elapsed_seconds':round(time.monotonic()-start,3),'log':str(log),'harness':str(script),'harness_sha256':digest(script)}
    text=log.read_text()
    profile=None
    if name=='quote':
        m=re.search(r'profile: (.+)',text)
        if m: profile=Path(m.group(1).strip())
        reports=['native-verification.json','settings-native-verification.json','preview-native-verification.json','process-restart-verification.json']
    elif name=='held-save-close':
        profile=Path(env['SARV_CLOSE_TEST_DIR']);reports=['native-close-verification.json']
    else:
        matches=re.findall(r'"profile":\s*"([^"]+)"',text)
        if matches:profile=Path(matches[-1])
        reports=[{'accounting':'accounting-independent-native.json','jalali':'jalali-independent-native.json','sidebar':'sidebar-verification.json'}[name]]
    if profile:
        entry['profile']=str(profile)
        dest=out/name
        dest.mkdir()
        for p in profile.iterdir():
            if p.is_file() and p.suffix in ('.json','.png','.pdf','.csv','.txt','.log'):
                shutil.copy2(p,dest/p.name)
        parsed=[]
        for filename in reports:
            p=profile/filename
            if p.exists():
                data=json.loads(p.read_text());parsed.append({'file':str(dest/filename),'result':data.get('result'),'checks':len(data.get('checks',[]))})
        entry['reports']=parsed
        entry['checks']=sum(r['checks'] for r in parsed)
    summary['runs'].append(entry)
    (out/'summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps(entry,ensure_ascii=False),flush=True)
    assert digest(BINARY)==EXPECTED and digest(ROOT/'target/release/sarv-quote')==EXPECTED
summary['result']='passed' if all(r['exit_code']==0 for r in summary['runs']) else 'failed'
summary['total_named_checks']=sum(r.get('checks',0) for r in summary['runs'])
(out/'summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
print(json.dumps({'result':summary['result'],'total_named_checks':summary['total_named_checks'],'summary':str(out/'summary.json')},ensure_ascii=False),flush=True)
raise SystemExit(0 if summary['result']=='passed' else 1)
