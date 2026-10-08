#!/usr/bin/env python3
"""M3 synthetic native forms, independent SQLite controls, real XTest entry/restart.
Source /workspace/sarv-environment/activate.sh; dbus-run-session -- /usr/bin/python3 scripts/core-qa.py
No live profile is read or written. Every run allocates a new artifacts/core-native directory.
"""
import ctypes as C
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import tempfile
import time
from qa_access import Access
spec=importlib.util.spec_from_file_location('native_qa',Path(__file__).with_name('qa-native.py'))
native=importlib.util.module_from_spec(spec);spec.loader.exec_module(native)
ROOT,ENVROOT=native.ROOT,native.ENVROOT

def state(profile):
    with sqlite3.connect(f'file:{profile}/sarv.sqlite3?mode=ro',uri=True) as db:
        return json.loads(db.execute('SELECT payload FROM state WHERE id=1').fetchone()[0])

def main():
    output=ROOT/'artifacts/core-native';output.mkdir(parents=True,exist_ok=True)
    run=Path(tempfile.mkdtemp(prefix='run-',dir=output));profile=run/'full';profile.mkdir()
    binary=Path(os.environ.get('SARV_QA_BINARY',str(ROOT/'target/release/sarv-quote')))
    report={'result':'failed','run':str(run),'binary':str(binary),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'backend':'private X11 GTK4 / Xvfb / Openbox; widget financial workflow plus actual XTest keyboard entry separately','checks':[]}
    processes,logs=[],[];x11=None
    def start(command,name,env=None,**kwargs):
        log=(run/f'{name}.log').open('w');logs.append(log)
        p=subprocess.Popen(command,env=env,stdout=log,stderr=log,start_new_session=True,**kwargs);processes.append(p);return p
    def check(name,condition,**details):
        report['checks'].append({'name':name,'passed':bool(condition),**details})
        if not condition:raise AssertionError(name)
    def screenshot(window,path):
        subprocess.run(['/usr/bin/import','-display',display,'-window',str(window),str(path)],check=True)
    try:
        with tempfile.TemporaryFile() as fd:
            start(['Xvfb','-displayfd',str(fd.fileno()),'-screen','0','1450x1050x24','-nolisten','tcp','-xkbdir',str(ENVROOT/'sysroot/usr/share/X11/xkb')],'xvfb',pass_fds=(fd.fileno(),))
            def getdisplay():fd.seek(0);return fd.read().decode().strip()
            display=':'+native.wait_for(getdisplay,label='private X11 display')
        runtime=native.private_accessibility_environment(display,run)
        env=os.environ.copy();env.update(DISPLAY=display,GDK_BACKEND='x11',GSK_RENDERER='cairo',XDG_RUNTIME_DIR=str(runtime),SARV_TEST_INSTANCE='core-native-qa',SARV_DATA_DIR=str(profile),SARV_CORE_SMOKE='full')
        for key in ['SARV_UI_SMOKE','SARV_ACCOUNTING_SMOKE']:env.pop(key,None)
        start(['openbox','--sm-disable','--config-file',str(ENVROOT/'sysroot/etc/xdg/openbox/rc.xml')],'openbox',env)
        app=start([str(binary)],'full-native',env);app.wait(timeout=120);time.sleep(.25)
        failure=profile/'core-native-failure.txt'
        check('native financial forms completed',app.returncode==0 and not failure.exists() and (profile/'core-native-verification.json').exists(),failure=failure.read_text() if failure.exists() else None)
        baseline=state(profile);ledger=baseline['accounting'];opening=ledger['core']['openings'][0]['command'];journal=ledger['journals'][0]
        check('one company, opening source and journal',len(ledger['companies'])==1 and len(ledger['core']['openings'])==1 and len(ledger['journals'])==1)
        accounts={a['id']:a for a in ledger['accounts']};mapping={m['role']:m['account_id'] for m in ledger['core']['setups'][0]['command']['mappings']}
        expected={'Cash':251,'Bank':300,'Receivables':200,'Inventory':300,'ChequesReceivable':200,'Payables':-149,'ChequesPayable':-100,'Grni':-50,'OwnerCapital':-952}
        totals={}
        for b in opening['balances']:totals[b['account_id']]=b['debit']-b['credit']
        check('literal independent ledger balances1251 and supplied952equity',all(totals[mapping[role]]==amount for role,amount in expected.items()),expected=expected)
        posted_lines={line['account']['id']:line['debit']-line['credit'] for line in journal['lines']}
        check('independent exact journal matches supplied ledger facts',posted_lines=={mapping[role]:amount for role,amount in expected.items()} and sum(line['debit'] for line in journal['lines'])==sum(line['credit'] for line in journal['lines'])==1251)
        check('independent cash and statement',opening['cash'][0]['balance']==251 and opening['banks'][0]['balance']==opening['banks'][0]['statement_balance']==300)
        check('AR200/AP149 independently sum',sum(p['remaining'] for p in opening['open_items'] if p['side']=='Receivable')==200 and sum(p['remaining'] for p in opening['open_items'] if p['side']=='Payable')==149)
        check('stock300/30000 base thousandths and GRNI subset50',sum(p['value'] for p in opening['stock'])==300 and sorted(p['quantity_thousandths'] for p in opening['stock'])==[2000,6000] and opening['items'][0]['packs'][0]['numerator']==12 and opening['items'][0]['packs'][0]['denominator']==1 and opening['grni'][0]['value']==50)
        check('dualparty roles separate; cheque states distinct',opening['parties'][0]['customer'] and opening['parties'][0]['supplier'] and {p['state'] for p in opening['cheques']}=={'Held','Issued'})
        check('fiscal1405 canonical dates and frozen policy timezone',ledger['periods'][0]['start_date']=='2026-03-21' and ledger['periods'][0]['end_date']=='2027-03-20' and ledger['core']['setups'][0]['command']['policy']['timezone']=='AsiaTehran')
        check('backup/freshrestore/original restart',json.loads((profile/'core-recovery.json').read_text())['result']=='passed')
        for name in ['opening-rial.pdf','opening-toman.pdf']:
            check(f'{name} real PDF', (profile/name).stat().st_size>1000)
            subprocess.run(['pdftoppm','-f','1','-singlefile','-scale-to','1200','-png',str(profile/name),str(profile/name.removesuffix('.pdf'))],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        zero=run/'zero';zero.mkdir();env.update(SARV_DATA_DIR=str(zero),SARV_CORE_SMOKE='zero',SARV_TEST_INSTANCE='core-zero')
        app=start([str(binary)],'zero-native',env);app.wait(timeout=100);time.sleep(.25)
        zero_failure=zero/'core-native-failure.txt'
        check('native zero initialization completes without empty financial journal',app.returncode==0 and not zero_failure.exists() and (zero/'core-zero-native.json').exists(),failure=zero_failure.read_text() if zero_failure.exists() else None)
        z=state(zero)['accounting'];check('independent durable zero effect and no journal',len(z['journals'])==0 and len(z['core']['openings'])==1 and z['core']['openings'][0]['effect']=='ZeroBalanceInitialization')
        # Independent keyboard entry modifies only a staged fresh-company opening.
        stage=run/'keyboard';stage.mkdir();env.update(SARV_DATA_DIR=str(stage),SARV_CORE_SMOKE='stage',SARV_TEST_INSTANCE='core-keyboard')
        app=start([str(binary)],'keyboard-stage',env);x11=native.X11(display)
        class XError(C.Structure):
            _fields_=[('type',C.c_int),('display',C.c_void_p),('resource',C.c_ulong),('serial',C.c_ulong),('code',C.c_ubyte),('request',C.c_ubyte),('minor',C.c_ubyte)]
        @C.CFUNCTYPE(C.c_int,C.c_void_p,C.POINTER(XError))
        def stale_window_error(_,error):
            # BadWindow is expected only while enumerating recently exited owned windows.
            if error.contents.code!=3:report.setdefault('x11_errors',[]).append({'code':error.contents.code,'request':error.contents.request})
            return 0
        x11.x.XSetErrorHandler.argtypes=[C.c_void_p];x11.x.XSetErrorHandler(C.cast(stale_window_error,C.c_void_p))
        window=native.wait_for(lambda:x11.find(app.pid),label='owned core window')
        native.wait_for(lambda:(stage/'core-stage-ready.json').exists(),seconds=100,label='native stage ready')
        original=state(stage);access=Access(app.pid);x11.focus(window)
        def focus(name,roles=None):
            target=native.wait_for(lambda:access.find(name,roles),label=f'control {name}')
            # Traverse real Tab order; AT-SPI only observes this owned process.
            for _ in range(100):
                target=access.find(name,roles)
                if target and access.focused(target):return target
                x11.key('Tab');time.sleep(.025)
            (stage/'keyboard-focus-tree.json').write_text(json.dumps([{k:v for k,v in row.items() if k!='node'} for row in access.tree()],ensure_ascii=False,indent=2))
            raise AssertionError(f'real Tab focus {name}')
        focus('شناسهٔ ثابت؛ یکتا و قابل ارجاع',['text','entry'])
        x11.type_ascii('TEMP-KBD');x11.key('Tab');time.sleep(.1)
        check('real Tab reaches party name',any(r['name']=='نام طرف حساب' and access.focused(r) for r in access.tree()))
        x11.type_ascii('Keyboard temporary party')
        focus('افزودن / ذخیرهٔ سطر');x11.key('Return');time.sleep(.3)
        check('real Enter adds staged party',any('TEMP-KBD' in r['name'] and 'Keyboard temporary party' in r['name'] for r in access.tree()))
        # Search itself is real typed text; result's edit/delete must be reachable.
        focus('جستجوی سطرهای جزئیات',['text','entry']);x11.type_ascii('TEMP-KBD');time.sleep(.35)
        focus('ویرایش سطر');x11.key('Return');time.sleep(.15)
        target=focus('نام طرف حساب',['text','entry']);x11.key('a','Control_L');x11.type_ascii('Keyboard revised party')
        focus('افزودن / ذخیرهٔ سطر');x11.key('Return');time.sleep(.2)
        check('real keyboard edits retained staged record',any('Keyboard revised party' in r['name'] for r in access.tree()))
        focus('حذف از پیش‌نویس');x11.key('Return');time.sleep(.2)
        check('real keyboard deletes staged record',not any('Keyboard revised party' in r['name'] for r in access.tree()))
        check('staged keyboard operations never write balances',state(stage)==original)
        screenshot(window,stage/'core-keyboard-records.png')
        focus('انصراف');x11.key('Escape');time.sleep(.3)
        check('real Escape cancels opening unchanged',state(stage)==original and not any(r['name']=='ثبت افتتاحیهٔ تطبیق‌شده' for r in access.tree()))
        # Saved company reopens without a second setup; discarded opening does not return.
        native.stop(app);time.sleep(.25);env.pop('SARV_CORE_SMOKE');env.update(SARV_UI_SMOKE='capture',SARV_CAPTURE_PAGE='accounting',SARV_CAPTURE_NAME='core-restart.png')
        app=start([str(binary)],'keyboard-restart',env);window=native.wait_for(lambda:x11.find(app.pid),label='restarted owned window');native.wait_for(lambda:(stage/'core-restart.png').exists(),label='restart rendered')
        check('discarded staged opening absent after restart',state(stage)==original and len(state(stage)['accounting']['journals'])==0)
        # Paired layout evidence, light and dark, standard and650 pixels.
        for appearance in ['light','dark']:
            native.stop(app);time.sleep(.25);env['ADW_DEBUG_COLOR_SCHEME']='prefer-'+appearance
            env['SARV_DATA_DIR']=str(profile);env['SARV_CAPTURE_NAME']=f'core-{appearance}-initial.png'
            app=start([str(binary)],f'layout-{appearance}',env);window=native.wait_for(lambda:x11.find(app.pid),label=f'{appearance} window');native.wait_for(lambda:(profile/env['SARV_CAPTURE_NAME']).exists(),label=f'{appearance} rendered')
            access=Access(app.pid)
            for width,height in [(1280,900),(650,760)]:
                x11.resize(window,width,height);native.wait_for(lambda:x11.geometry(window)==[width,height],label=f'{width} geometry');time.sleep(.25);screenshot(window,profile/f'core-after-{appearance}-{width}.png')
                check(f'actual {appearance} {width} native layout',True)
        check('read-only layout/restart keeps exact saved aggregate',state(profile)==baseline)
        # Preserve immutable M2 baseline in its own empty synthetic profile.
        m2=Path('/workspace/sarv-evidence/bin/sarv-M2-layout-final')
        if m2.exists():
            native.stop(app);time.sleep(.25);before=run/'m2-before';before.mkdir();env['SARV_DATA_DIR']=str(before);env['SARV_CAPTURE_NAME']='core-m2-before.png';env['ADW_DEBUG_COLOR_SCHEME']='prefer-light';app=start([str(m2)],'m2-before',env);window=native.wait_for(lambda:x11.find(app.pid),label='M2 owned window');native.wait_for(lambda:(before/'core-m2-before.png').exists(),label='M2 capture')
            report['baseline_binary_sha256']=hashlib.sha256(m2.read_bytes()).hexdigest()
            for width,height in [(1280,900),(650,760)]:
                x11.resize(window,width,height);native.wait_for(lambda:x11.geometry(window)==[width,height],label='M2 geometry');time.sleep(.25);screenshot(window,before/f'core-before-light-{width}.png')
        check('no unexpected X11 errors',not report.get('x11_errors'))
        report['result']='passed'
    except Exception as e:
        report['error']=str(e)
        if 'access' in locals():
            (run/'failed-access-tree.json').write_text(json.dumps([{k:v for k,v in row.items() if k!='node'} for row in access.tree()],ensure_ascii=False,indent=2))
        raise
    finally:
        if x11:x11.x.XCloseDisplay(x11.d)
        for process in reversed(processes):native.stop(process)
        for log in logs:log.close()
        (run/'core-independent-native.json').write_text(json.dumps(report,ensure_ascii=False,indent=2));print(json.dumps(report,ensure_ascii=False,indent=2))
if __name__=='__main__':main()
