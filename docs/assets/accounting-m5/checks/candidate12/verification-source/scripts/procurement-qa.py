#!/usr/bin/env python3
"""Disposable M4 native forms and independent raw SQLite verification; no live data."""
import ctypes as C
import hashlib,importlib.util,json,os,sqlite3,subprocess,tempfile,time,shutil
from pathlib import Path
from qa_access import Access
spec=importlib.util.spec_from_file_location('native_qa',Path(__file__).with_name('qa-native.py'))
native=importlib.util.module_from_spec(spec);spec.loader.exec_module(native)
ROOT,ENVROOT=native.ROOT,native.ENVROOT

def state(profile):
    with sqlite3.connect(f'file:{profile}/sarv.sqlite3?mode=ro',uri=True) as db:
        return json.loads(db.execute('SELECT payload FROM state WHERE id=1').fetchone()[0])

def main():
    output=ROOT/'artifacts/procurement-native';output.mkdir(parents=True,exist_ok=True)
    run=Path(tempfile.mkdtemp(prefix='run-',dir=output));profile=run/'full';profile.mkdir()
    source_binary=Path(os.environ.get('SARV_QA_BINARY',str(ROOT/'target/debug/sarv-quote')))
    binary=run/'sarv-quote';shutil.copy2(source_binary,binary)
    seed=Path('/workspace/sarv-evidence/bin/sarv-M3-final')
    report={'result':'failed','run':str(run),'binary':str(binary),'binary_source':str(source_binary),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'seed_binary':str(seed),'seed_sha256':hashlib.sha256(seed.read_bytes()).hexdigest(),'scope':'private X11 GTK4 native financial forms, raw SQLite independent arithmetic, external XTest separately','checks':[]}
    processes=[];logs=[];x11=None
    def start(command,name,env=None,**kw):
        log=(run/f'{name}.log').open('w');logs.append(log)
        p=subprocess.Popen(command,env=env,stdout=log,stderr=log,start_new_session=True,**kw);processes.append(p);return p
    def check(name,condition,**details):
        report['checks'].append({'name':name,'passed':bool(condition),**details})
        if not condition:raise AssertionError(name)
    try:
        with tempfile.TemporaryFile() as fd:
            start(['Xvfb','-displayfd',str(fd.fileno()),'-screen','0','1450x1050x24','-nolisten','tcp','-xkbdir',str(ENVROOT/'sysroot/usr/share/X11/xkb')],'xvfb',pass_fds=(fd.fileno(),))
            def getdisplay():fd.seek(0);return fd.read().decode().strip()
            display=':'+native.wait_for(getdisplay,label='private X11 display')
        runtime=native.private_accessibility_environment(display,run)
        env=os.environ.copy();env.update(DISPLAY=display,GDK_BACKEND='x11',GSK_RENDERER='cairo',XDG_RUNTIME_DIR=str(runtime),SARV_TEST_INSTANCE='procurement-native-qa',SARV_DATA_DIR=str(profile),SARV_CORE_SMOKE='full',ADW_DEBUG_COLOR_SCHEME='prefer-light')
        for key in ['SARV_UI_SMOKE','SARV_ACCOUNTING_SMOKE','SARV_PROCUREMENT_SMOKE']:env.pop(key,None)
        start(['openbox','--sm-disable','--config-file',str(ENVROOT/'sysroot/etc/xdg/openbox/rc.xml')],'openbox',env)
        baseline=start([str(seed)],'M3-seed',env);baseline.wait(timeout=120)
        check('immutable M3 synthetic opening seed',baseline.returncode==0 and (profile/'core-native-verification.json').exists())
        before=state(profile);check('seed opening1251/no operational effects',len(before['accounting']['journals'])==1)
        before_profile=run/'M3-before';before_profile.mkdir()
        with sqlite3.connect(profile/'sarv.sqlite3') as source,sqlite3.connect(before_profile/'sarv.sqlite3') as target:source.backup(target)
        (profile/'M4-synthetic-prices.csv').write_text('code,name,measure,price,currency,source_at,aliases\nSKU-A,کالای الف خرید,عدد,15,rial,2026-03-22T09:00:00Z,نمونه الف\nSKU-B,کالای ب خرید,عدد,20,rial,2026-03-22T09:00:00Z,نمونه ب\n')
        env.pop('SARV_CORE_SMOKE');env['SARV_PROCUREMENT_SMOKE']='full'
        app=start([str(binary)],'M4-full-native',env)
        failure=profile/'procurement-native-failure.txt';manifest=profile/'procurement-native-verification.json'
        native.wait_for(lambda:manifest.exists() or failure.exists() or app.poll() is not None,seconds=180,label='native M4 full workflow')
        check('native typed workflow completed',manifest.exists() and not failure.exists(),failure=failure.read_text() if failure.exists() else None,app_returncode=app.poll())
        raw=state(profile);a=raw['accounting'];records=a['procurement']['records']
        check('source opening preserved exactly',a['core']==before['accounting']['core'])
        check('4additive resources and PO/2receipts/2bills/transfer',len(records)==10 and len(a['journals'])==5,record_count=len(records),journal_count=len(a['journals']))
        kinds={kind:[r for r in records if kind in r['data']] for kind in ['Resources','Order','Receive','Bill','Transfer']}
        check('PO12pieces@15=180 nonfinancial',len(kinds['Order'])==1 and kinds['Order'][0]['journal_id'] is None and kinds['Order'][0]['data']['Order']['total']==180)
        check('partial receipt75+105 from frozenPO',sorted(r['data']['Receive']['total'] for r in kinds['Receive'])==[75,105])
        check('partial matched bill30+150',sorted(r['data']['Bill']['total'] for r in kinds['Bill'])==[30,150])
        transfer=kinds['Transfer'][0];moves=transfer['movements'];check('transfer6pieces70 paired/nojournal',transfer['journal_id'] is None and transfer['data']['Transfer']['quantity_thousandths']==6000 and transfer['data']['Transfer']['value']==70 and len(moves)==2)
        pools={m['warehouse_id']:m['after'] for m in moves}
        check('literal final W1=30/350 W2=12/130',pools['W1']['quantity_thousandths']==30000 and pools['W1']['value']==350 and pools['W2']['quantity_thousandths']==12000 and pools['W2']['value']==130)
        mapping={m['role']:m['account_id'] for m in a['core']['setups'][0]['command']['mappings']};sums={}
        for j in a['journals']:
            for l in j['lines']:sums[l['account']['id']]=sums.get(l['account']['id'],0)+l['debit']-l['credit']
        expected={'Cash':251,'Bank':300,'Receivables':200,'Inventory':480,'ChequesReceivable':200,'Payables':-329,'ChequesPayable':-100,'Grni':-50,'OwnerCapital':-952}
        check('independent final controls, untouched unrelated balances',all(sums[mapping[k]]==v for k,v in expected.items()),expected=expected)
        check('independent trial1431/1431',sum(v for v in sums.values() if v>0)==1431 and -sum(v for v in sums.values() if v<0)==1431)
        source=kinds['Order'][0]['command']['Order']['lines'][0]['price_reference'];check('catalogsource snapshot frozen',all(k in source for k in ['catalog=','SHA256=','SKU=SKU-A','قیمت‌ریال=15','2026-03-22T09:00:00Z']))
        check('actual reports produced',all((profile/name).exists() and (profile/name).stat().st_size>100 for name in ['M4-controls.csv','M4-controls.pdf','M4-order.pdf']))
        report['native_manifest']=json.loads(manifest.read_text())
        x11=native.X11(display);report.setdefault('x11_errors',[])
        class XError(C.Structure):
            _fields_=[('type',C.c_int),('display',C.c_void_p),('resource',C.c_ulong),('serial',C.c_ulong),('code',C.c_ubyte),('request',C.c_ubyte),('minor',C.c_ubyte)]
        @C.CFUNCTYPE(C.c_int,C.c_void_p,C.POINTER(XError))
        def stale_window_error(_,error):
            if error.contents.code!=3:report['x11_errors'].append({'code':error.contents.code,'request':error.contents.request})
            return 0
        x11.x.XSetErrorHandler.argtypes=[C.c_void_p];x11.x.XSetErrorHandler(C.cast(stale_window_error,C.c_void_p))
        window=native.wait_for(lambda:x11.find(app.pid),label='owned M4 window');x11.focus(window)
        access=Access(app.pid)
        def focus(name,roles=None):
            native.wait_for(lambda:access.find(name,roles),label=f'accessible {name}')
            for _ in range(120):
                target=access.find(name,roles)
                if target and access.focused(target):return target
                x11.key('Tab');time.sleep(.018)
            raise AssertionError(f'real Tab focus {name}')
        def fill(name,text):
            focus(name,['text','entry']);x11.key('a','Control_L');x11.type_ascii(text)
        def press(name):focus(name,['button','push button']);x11.key('Return');time.sleep(.3)
        def screenshot(name):subprocess.run(['/usr/bin/import','-display',display,'-window',str(window),str(profile/name)],check=True)
        x11.key('8','Alt_L');time.sleep(.4)
        press('سفارش خرید تازه');fill('شماره یا مرجع سند','KBD-CANCEL');fill('شرح و دلیل ثبت','Keyboard synthetic cancelled order')
        press('افزودن کالا و قیمت');fill('مقدار در واحد مشخص‌شده (تا سه رقم اعشار)','1');fill('قیمت هر واحد ردیف','0.1');fill('مرجع قیمت دستی','KBD-MANUAL-PRICE');fill('دلیل قیمت دستی یا تغییر قیمت منبع','Keyboard explicit manual price')
        press('افزودن ردیف به سفارش')
        check('actual XTest fractionalIRR error retains typed input',access.text(access.find('قیمت هر واحد ردیف',['text','entry']))=='0.1' and any('کسری از ریال' in r['name'] for r in access.tree()))
        screenshot('M4-keyboard-retained-price.png');fill('قیمت هر واحد ردیف','15');press('افزودن ردیف به سفارش')
        check('actual Enter adds staged line',any(r['name']=='ویرایش ردیف' for r in access.tree()))
        press('ویرایش ردیف');fill('مقدار در واحد مشخص‌شده (تا سه رقم اعشار)','2');press('اعمال ویرایش ردیف')
        check('actual keyboard edits staged quantity',any('2.000' in r['name'] for r in access.tree()))
        press('حذف ردیف');check('actual keyboard removes staged line',not any(r['name']=='ویرایش ردیف' for r in access.tree()))
        focus('انصراف',['button','push button']);x11.key('Escape');time.sleep(.35)
        check('actual Escape cancellation preserves posted aggregate',state(profile)==raw and not any(r['name']=='تأیید و ثبت نسخهٔ بازبینی‌شده' for r in access.tree()))
        # A real entered query observes exactly the expected immutable source row.
        x11.key('f','Control_L');native.wait_for(lambda:any(r['name']=='جست‌وجوی شماره، منبع، کالا یا طرف' and r['role'] in ['text','entry'] and access.focused(r) for r in access.tree()),label='real Ctrl+F focuses active native search');x11.type_ascii('REC-A');native.wait_for(lambda:any('REC-A' in r['name'] for r in access.tree()) and not any('REC-B' in r['name'] for r in access.tree()),label='native normalized document search')
        check('actual typed immutable document search',True);x11.key('a','Control_L');x11.key('BackSpace');time.sleep(.2)
        native.stop(app);env.pop('SARV_PROCUREMENT_SMOKE');env.update(SARV_UI_SMOKE='capture',SARV_CAPTURE_PAGE='inventory')
        for appearance in ['light','dark']:
            env['ADW_DEBUG_COLOR_SCHEME']='prefer-'+appearance;env['SARV_CAPTURE_NAME']=f'M4-restart-{appearance}.png'
            app=start([str(binary)],f'layout-{appearance}',env);window=native.wait_for(lambda:x11.find(app.pid),label='restarted owned window');native.wait_for(lambda:(profile/env['SARV_CAPTURE_NAME']).exists(),label='mapped restart');access=Access(app.pid);x11.focus(window)
            for page,key in [('inventory','9'),('procurement','8')]:
                x11.key(key,'Alt_L');time.sleep(.3)
                for width,height in [(1280,900),(650,760)]:
                    x11.resize(window,width,height);native.wait_for(lambda:x11.geometry(window)==[width,height],label='native geometry');time.sleep(.2);screenshot(f'M4-after-{page}-{appearance}-{width}.png');check(f'actual {page} {appearance} {width}px native layout',True)
            native.stop(app);time.sleep(.2)
        check('posted operational records exact across restart/layout',state(profile)==raw)
        # The untouched M3 binary reads its own isolated schema4 snapshot.
        env['SARV_DATA_DIR']=str(before_profile);env['SARV_CAPTURE_PAGE']='accounting';env['SARV_CAPTURE_NAME']='M3-before.png';env['ADW_DEBUG_COLOR_SCHEME']='prefer-light'
        app=start([str(seed)],'M3-before-layout',env);window=native.wait_for(lambda:x11.find(app.pid),label='immutable M3 baseline window');native.wait_for(lambda:(before_profile/'M3-before.png').exists(),label='immutable M3 baseline mapped')
        for width,height in [(1280,900),(650,760)]:
            x11.resize(window,width,height);native.wait_for(lambda:x11.geometry(window)==[width,height],label='M3 baseline geometry');time.sleep(.2);subprocess.run(['/usr/bin/import','-display',display,'-window',str(window),str(before_profile/f'M3-before-light-{width}.png')],check=True)
        check('no unexpected X11 errors',not report['x11_errors'])
        report['result']='passed'
        print(json.dumps(report,ensure_ascii=False,indent=2))
    except Exception as e:
        report['error']=str(e)
        if 'access' in locals():(run/'failed-access-tree.json').write_text(json.dumps([{k:v for k,v in r.items() if k!='node'} for r in access.tree()],ensure_ascii=False,indent=2))
        print(json.dumps(report,ensure_ascii=False,indent=2));raise
    finally:
        (run/'procurement-independent-native.json').write_text(json.dumps(report,ensure_ascii=False,indent=2))
        if x11:x11.x.XCloseDisplay(x11.d)
        for p in reversed(processes):
            if p.poll() is None:
                p.terminate()
                try:p.wait(timeout=3)
                except subprocess.TimeoutExpired:p.kill()
        for log in logs:log.close()
if __name__=='__main__':main()
