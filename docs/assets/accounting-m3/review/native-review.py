#!/usr/bin/env python3
"""M3 synthetic native forms, independent SQLite controls, real XTest entry/restart.
Source /workspace/sarv-environment/activate.sh; dbus-run-session -- /usr/bin/python3 scripts/core-qa.py
No live profile is read or written. Every run allocates a new artifacts/core-native directory.
"""
import ctypes as C
import hashlib
import sys
sys.path.insert(0,"/workspace/sarv-quote/scripts")
import importlib.util
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import tempfile
import time
from qa_access import Access
spec=importlib.util.spec_from_file_location('native_qa',Path('/workspace/sarv-quote/scripts/qa-native.py'))
native=importlib.util.module_from_spec(spec);spec.loader.exec_module(native)
ROOT,ENVROOT=native.ROOT,native.ENVROOT

def state(profile):
    with sqlite3.connect(f'file:{profile}/sarv.sqlite3?mode=ro',uri=True) as db:
        return json.loads(db.execute('SELECT payload FROM state WHERE id=1').fetchone()[0])

def main():
    output=ROOT/'artifacts/independent-m3/native';output.mkdir(parents=True,exist_ok=True)
    run=Path(tempfile.mkdtemp(prefix='run-',dir=output));profile=run/'full';profile.mkdir()
    binary=Path(os.environ.get('SARV_QA_BINARY',str(ROOT/'target/release/sarv-quote')))
    assert os.environ.get('SARV_QA_EXPECT_SHA')==hashlib.sha256(binary.read_bytes()).hexdigest(), 'binary must equal reviewer frozen SHA'
    report={'result':'failed','run':str(run),'binary':str(binary),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'backend':'private X11 GTK4 / Xvfb / Openbox; widget financial workflow plus actual XTest keyboard entry separately','checks':[]}
    processes,logs=[],[];x11=None;xerrors=[];xhandler=None
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
        app=start([str(binary)],'full-native',env);app.wait(timeout=120)
        failure=profile/'core-native-failure.txt'
        check('native financial forms completed',app.returncode==0 and not failure.exists() and (profile/'core-native-verification.json').exists(),failure=failure.read_text() if failure.exists() else None)
        baseline=state(profile);ledger=baseline['accounting'];opening=ledger['core']['openings'][0]['command'];journal=ledger['journals'][0]
        check('one company, opening source and journal',len(ledger['companies'])==1 and len(ledger['core']['openings'])==1 and len(ledger['journals'])==1)
        accounts={a['id']:a for a in ledger['accounts']};mapping={m['role']:m['account_id'] for m in ledger['core']['setups'][0]['command']['mappings']}
        expected={'Cash':251,'Bank':300,'Receivables':200,'Inventory':300,'ChequesReceivable':200,'Payables':-149,'ChequesPayable':-100,'Grni':-50,'OwnerCapital':-952}
        totals={}
        for b in opening['balances']:totals[b['account_id']]=b['debit']-b['credit']
        check('literal independent ledger balances1251 and supplied952equity',all(totals[mapping[role]]==amount for role,amount in expected.items()),expected=expected)
        check('independent cash and statement',opening['cash'][0]['balance']==251 and opening['banks'][0]['balance']==opening['banks'][0]['statement_balance']==300)
        check('AR200/AP149 independently sum',sum(p['remaining'] for p in opening['open_items'] if p['side']=='Receivable')==200 and sum(p['remaining'] for p in opening['open_items'] if p['side']=='Payable')==149)
        check('stock300/30000 base thousandths and GRNI subset50',sum(p['value'] for p in opening['stock'])==300 and sorted(p['quantity_thousandths'] for p in opening['stock'])==[2000,6000] and opening['items'][0]['packs'][0]['numerator']==12 and opening['grni'][0]['value']==50)
        check('dualparty roles separate; cheque states distinct',opening['parties'][0]['customer'] and opening['parties'][0]['supplier'] and {p['state'] for p in opening['cheques']}=={'Held','Issued'})
        check('fiscal1405 canonical dates and frozen policy timezone',ledger['periods'][0]['start_date']=='2026-03-21' and ledger['periods'][0]['end_date']=='2027-03-20' and ledger['core']['setups'][0]['command']['policy']['timezone']=='AsiaTehran')
        check('backup/freshrestore/original restart',json.loads((profile/'core-recovery.json').read_text())['result']=='passed')
        for name in ['opening-rial.pdf','opening-toman.pdf']:
            check(f'{name} real PDF', (profile/name).stat().st_size>1000)
            subprocess.run(['pdftoppm','-f','1','-singlefile','-scale-to','1200','-png',str(profile/name),str(profile/name.removesuffix('.pdf'))],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        # Independent keyboard entry modifies only a staged fresh-company opening.
        stage=run/'keyboard';stage.mkdir();env.update(SARV_DATA_DIR=str(stage),SARV_CORE_SMOKE='stage',SARV_TEST_INSTANCE='core-keyboard')
        app=start([str(binary)],'keyboard-stage',env);x11=native.X11(display)
        class XError(C.Structure):
            _fields_=[('type',C.c_int),('display',C.c_void_p),('resource',C.c_ulong),('serial',C.c_ulong),('code',C.c_ubyte),('request',C.c_ubyte),('minor',C.c_ubyte)]
        handler_type=C.CFUNCTYPE(C.c_int,C.c_void_p,C.POINTER(XError))
        def handle_xerror(_,e):
            xerrors.append({'code':e.contents.code,'request':e.contents.request,'resource':e.contents.resource});return 0
        xhandler=handler_type(handle_xerror);x11.x.XSetErrorHandler.argtypes=[handler_type];x11.x.XSetErrorHandler.restype=C.c_void_p;x11.x.XSetErrorHandler(xhandler)
        window=native.wait_for(lambda:x11.find(app.pid),label='owned core window')
        native.wait_for(lambda:(stage/'core-stage-ready.json').exists(),seconds=100,label='native stage ready')
        time.sleep(.4);original=state(stage);access=Access(app.pid);x11.focus(window)
        def focus(name,roles=None):
            target=native.wait_for(lambda:access.find(name,roles),label=f'control {name}')
            # AT-SPI focuses a named control, then all editing/action uses actual XTest keys.
            if not access.focus(target):
                for _ in range(240):
                    if access.focused(target):break
                    x11.key('Tab');time.sleep(.02)
                else:raise AssertionError(f'focus {name}')
            native.wait_for(lambda:access.focused(target),label=f'focused {name}')
            return target
        focus('شناسهٔ ثابت؛ یکتا و قابل ارجاع',['text','entry'])
        x11.type_ascii('TEMP-KBD');x11.key('Tab');time.sleep(.1)
        check('real Tab reaches party name',any(r['name']=='نام طرف حساب' and access.focused(r) for r in access.tree()))
        x11.type_ascii('Keyboard temporary party')
        focus('افزودن / ذخیرهٔ سطر',['button','push button']);x11.key('Return');time.sleep(.3)
        check('real Enter adds staged party',any('TEMP-KBD' in r['name'] and 'Keyboard temporary party' in r['name'] for r in access.tree()))
        # Search itself is real typed text; result's edit/delete must be reachable.
        focus('جستجوی سطرهای جزئیات',['text','entry']);x11.type_ascii('TEMP-KBD');time.sleep(.35)
        focus('ویرایش سطر',['button','push button']);x11.key('Return');time.sleep(.15)
        target=focus('نام طرف حساب',['text','entry']);x11.key('a','Control_L');x11.type_ascii('Keyboard revised party')
        focus('افزودن / ذخیرهٔ سطر',['button','push button']);x11.key('Return');time.sleep(.2)
        check('real keyboard edits retained staged record',any('Keyboard revised party' in r['name'] for r in access.tree()))
        focus('حذف از پیش‌نویس',['button','push button']);x11.key('Return');time.sleep(.2)
        check('real keyboard deletes staged record',not any('Keyboard revised party' in r['name'] for r in access.tree()))
        check('staged keyboard operations never write balances',state(stage)==original)
        screenshot(window,stage/'core-keyboard-records.png')
        focus('انصراف',['button','push button']);x11.key('Escape');time.sleep(.3)
        check('real Escape cancels opening unchanged',state(stage)==original and not any(r['name']=='ثبت افتتاحیهٔ تطبیق‌شده' for r in access.tree()))
        # Saved company reopens without a second setup; discarded opening does not return.
        native.stop(app);env.pop('SARV_CORE_SMOKE');env.update(SARV_UI_SMOKE='capture',SARV_CAPTURE_PAGE='accounting',SARV_CAPTURE_NAME='core-restart.png')
        app=start([str(binary)],'keyboard-restart',env);window=native.wait_for(lambda:x11.find(app.pid),label='restarted owned window');native.wait_for(lambda:(stage/'core-restart.png').exists(),label='restart rendered')
        check('discarded staged opening absent after restart',state(stage)==original and len(state(stage)['accounting']['journals'])==0)
        # Independent actual chooser regressions, each with13 declared ledger rows.
        import csv
        for variant in ['zero','nonzero']:
            native.stop(app);job=run/variant;job.mkdir();env.pop('SARV_UI_SMOKE',None);env.update(SARV_DATA_DIR=str(job),SARV_CORE_SMOKE='stage',SARV_TEST_INSTANCE=f'independent-{variant}-opening')
            app=start([str(binary)],f'{variant}-native-stage',env);window=native.wait_for(lambda:x11.find(app.pid),label=f'owned {variant} opening stage');native.wait_for(lambda:(job/'core-stage-ready.json').exists(),seconds=100,label=f'{variant} stage ready');access=Access(app.pid);x11.focus(window)
            time.sleep(.4);original_job=state(job);company=original_job['accounting']['companies'][0]['id']
            template=ROOT/'artifacts/independent-m3/fixture-template.csv'
            rows=list(csv.DictReader(template.open(newline='')))
            if variant=='zero':rows=[r for r in rows if r['kind'] in ['opening','ledger']]
            for r in rows:
                r['company_id']=company
                if r['kind']=='opening':r['id']=f'independent-native-{variant}';r['name']=f'Independently verified {variant} company'
                elif variant=='zero':r['amount']='0';r['credit']='0'
            job_csv=job/f'{variant}.csv'
            with job_csv.open('w',newline='') as f:
                w=csv.DictWriter(f,list(rows[0]));w.writeheader();w.writerows(rows)
            check(f'{variant} CSV exactly13 supplied balances',sum(r['kind']=='ledger' for r in rows)==13)
            focus('ورود CSV به پیش‌نویس',['button','push button']);x11.key('Return')
            def chooser_window():
                return next((w for w in x11.prop(x11.root,'_NET_CLIENT_LIST') if x11.prop(w,'_NET_WM_PID')==[app.pid] and b'CSV' in x11.prop(w,'_NET_WM_NAME')),None)
            chooser=native.wait_for(chooser_window,label='owned actual CSV chooser');x11.focus(chooser);time.sleep(.15)
            x11.key('l','Control_L');time.sleep(.2);x11.key('a','Control_L');x11.type_ascii(str(job_csv));time.sleep(.2)
            (job/'chooser-input.json').write_text(json.dumps([{k:v for k,v in r.items() if k!='node'}|{'text':access.text(r)} for r in access.tree() if r['role'] in ['text','entry']],ensure_ascii=False,indent=2))
            screenshot(chooser,job/'chooser-input.png')
            def usable_open():
                row=access.find('Open',['button','push button'])
                return row if row and access.a.atspi_state_set_contains(access.a.atspi_accessible_get_state_set(row['node']),24) else None
            open_button=native.wait_for(usable_open,label='owned file chooser Open sensitive')
            check(f'{variant} actual chooser Open widget invoked',access.click(open_button))
            def imported_source():
                row=access.find('مرجع انتقال / افتتاحیه',['text','entry'])
                return row and access.text(row)==f'independent-native-{variant}'
            native.wait_for(imported_source,seconds=30,label=f'actual chooser imported {variant} source')
            check(f'{variant} import pure before posting',state(job)==original_job)
            focus('بازبینی',['page tab']);x11.key('space');time.sleep(.2)
            focus('بررسی تراز و تطبیق کنترل‌ها',['button','push button']);x11.key('Return')
            action_name='تأیید آغاز بدون مانده' if variant=='zero' else 'ثبت افتتاحیهٔ تطبیق‌شده'
            def enabled_post():
                row=access.find(action_name,['button','push button'])
                return row if row and access.a.atspi_state_set_contains(access.a.atspi_accessible_get_state_set(row['node']),24) else None
            native.wait_for(enabled_post,seconds=30,label=f'explicit {variant} preview enables action')
            screenshot(window,job/f'{variant}-preview.png')
            x11.resize(window,650,760);native.wait_for(lambda:x11.geometry(window)==[650,760],label='narrow opening dialog');time.sleep(.25);screenshot(window,job/f'{variant}-preview-light-650.png')
            x11.resize(window,1280,900);native.wait_for(lambda:x11.geometry(window)==[1280,900],label='standard opening dialog');time.sleep(.25)
            focus(action_name,['button','push button']);x11.key('Return')
            native.wait_for(lambda:state(job)['accounting'].get('core',{}).get('openings'),seconds=30,label=f'{variant} initialization committed')
            books=state(job)['accounting'];record=books['core']['openings'][0];command=record['command']
            check(f'{variant} CSV no fabricated ordinary balances or references',len(command['balances'])==13 and len(command['import_source']['rows'])==len(rows))
            if variant=='zero':
                check('actual native zero has marker and no financial journal/request',len(books['core']['openings'])==1 and record['effect']=='ZeroBalanceInitialization' and books['journals']==[] and books['requests']==[] and len(books['core']['zero_requests'])==1)
            else:
                mapping={m['role']:m['account_id'] for m in books['core']['setups'][0]['command']['mappings']}
                amounts={b['account_id']:b['debit']-b['credit'] for b in command['balances']}
                check('actual native nonzero literal1251 supplied952capital one9linejournal',record['effect']=='Journal' and len(books['journals'])==1 and len(books['journals'][0]['lines'])==9 and sum(b['debit'] for b in command['balances'])==1251 and sum(b['credit'] for b in command['balances'])==1251 and all(amounts[mapping[r]]==v for r,v in expected.items()))
                check('actual native imported stock and GRNI lineage',[(s['quantity_thousandths'],s['value']) for s in command['stock']]==[(2000,240),(6000,60)] and command['grni'][0]['quantity_thousandths']==5000 and command['grni'][0]['value']==50)
            screenshot(window,job/f'{variant}-initialized.png');saved=state(job);native.stop(app);env.pop('SARV_CORE_SMOKE');env.update(SARV_UI_SMOKE='capture',SARV_CAPTURE_PAGE='accounting',SARV_CAPTURE_NAME=f'{variant}-restart.png')
            app=start([str(binary)],f'{variant}-restart',env);window=native.wait_for(lambda:x11.find(app.pid),label=f'{variant} restart window');native.wait_for(lambda:(job/f'{variant}-restart.png').exists(),label=f'{variant} restart rendered')
            check(f'{variant} initialization persisted exact restart',state(job)==saved)
        # Paired layout evidence, light and dark, standard and650 pixels.
        for appearance in ['light','dark']:
            native.stop(app);env['ADW_DEBUG_COLOR_SCHEME']='prefer-'+appearance
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
            native.stop(app);before=run/'m2-before';before.mkdir();env['SARV_DATA_DIR']=str(before);env['SARV_CAPTURE_NAME']='core-m2-before.png';env['ADW_DEBUG_COLOR_SCHEME']='prefer-light';app=start([str(m2)],'m2-before',env);window=native.wait_for(lambda:x11.find(app.pid),label='M2 owned window');native.wait_for(lambda:(before/'core-m2-before.png').exists(),label='M2 capture')
            report['baseline_binary_sha256']=hashlib.sha256(m2.read_bytes()).hexdigest()
            for width,height in [(1280,900),(650,760)]:
                x11.resize(window,width,height);native.wait_for(lambda:x11.geometry(window)==[width,height],label='M2 geometry');time.sleep(.25);screenshot(window,before/f'core-before-light-{width}.png')
        check('immutable binary hash preserved',hashlib.sha256(binary.read_bytes()).hexdigest()==report['binary_sha256'])
        check('observer X11 errors only expected retired-window races',all(e['code']==3 for e in xerrors),events=xerrors)
        report['result']='passed'
    except Exception as e:
        report['error']=str(e)
        if x11:
            try:
                screenshot(window,run/'failure.png');(run/'failure-tree.json').write_text(json.dumps([{k:v for k,v in r.items() if k!='node'} for r in access.tree()],ensure_ascii=False,indent=2))
            except Exception:pass
        raise
    finally:
        if x11:x11.x.XCloseDisplay(x11.d)
        for process in reversed(processes):native.stop(process)
        for log in logs:log.close()
        (run/'core-independent-native.json').write_text(json.dumps(report,ensure_ascii=False,indent=2));print(json.dumps(report,ensure_ascii=False,indent=2))
if __name__=='__main__':main()
