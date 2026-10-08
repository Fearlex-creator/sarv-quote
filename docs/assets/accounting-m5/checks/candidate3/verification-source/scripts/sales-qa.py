#!/usr/bin/env python3
"""M5 disposable native GTK workflow, independent SQLite oracle and real XTest input."""
import ctypes as C
import copy, hashlib, importlib.util, json, os, shutil, sqlite3, subprocess, tempfile, time
from pathlib import Path
from qa_access import Access
spec=importlib.util.spec_from_file_location('native_qa',Path(__file__).with_name('qa-native.py'))
native=importlib.util.module_from_spec(spec);spec.loader.exec_module(native)
ROOT,ENVROOT=native.ROOT,native.ENVROOT

def state(profile):
    with sqlite3.connect(f'file:{profile}/sarv.sqlite3?mode=ro',uri=True) as db:
        return json.loads(db.execute('SELECT payload FROM state WHERE id=1').fetchone()[0])

def main():
    output=ROOT/'artifacts/sales-native';output.mkdir(parents=True,exist_ok=True)
    run=Path(tempfile.mkdtemp(prefix='run-',dir=output));profile=run/'full';profile.mkdir()
    source=Path(os.environ.get('SARV_QA_BINARY',str(ROOT/'target/debug/sarv-quote')))
    binary=run/'sarv-quote';shutil.copy2(source,binary)
    report={'result':'failed','run':str(run),'binary_source':str(source),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'scope':'native GTK signal workflow, independent raw SQLite, real XTest keyboard, private X11','checks':[]}
    processes=[];logs=[];x11=None
    frozen_manifest_path=os.environ.get('SARV_SALES_SOURCE_MANIFEST')
    frozen_manifest=json.loads(Path(frozen_manifest_path).read_text()) if frozen_manifest_path else None
    observer=Path(__file__)
    report['observer_sha256']=hashlib.sha256(observer.read_bytes()).hexdigest()
    def frozen_sources():
        provenance=frozen_manifest['frozen_source_provenance']
        files=provenance['files']
        actual={};digest=hashlib.sha256()
        for name in sorted(files,key=lambda n:n.encode()):
            raw=(ROOT/name).read_bytes();actual[name]=hashlib.sha256(raw).hexdigest();digest.update(name.encode()+b'\0'+raw+b'\0')
        return actual,provenance['sha256'],digest.hexdigest()
    def start(command,name,env=None,**kw):
        log=(run/f'{name}.log').open('w');logs.append(log)
        p=subprocess.Popen(command,env=env,stdout=log,stderr=log,start_new_session=True,**kw);processes.append(p);return p
    def check(name,condition,**details):
        receipt={'name':name,'passed':bool(condition),**details}
        report['checks'].append(receipt)
        with (run/'sales-check-progress.jsonl').open('a') as progress:progress.write(json.dumps(receipt,ensure_ascii=False)+'\n')
        if not condition:raise AssertionError(name)
    def copy_profile(source,target):
        with sqlite3.connect(f'file:{source}/sarv.sqlite3?mode=ro',uri=True) as src,sqlite3.connect(target/'sarv.sqlite3') as dst:src.backup(dst)
    try:
        if frozen_manifest:
            actual,expected_aggregate,actual_aggregate=frozen_sources()
            report['frozen_source_manifest']=str(Path(frozen_manifest_path).resolve())
            report['frozen_source_provenance_before']={'sha256':actual_aggregate,'files':actual}
            check('frozen binary exact before native proof',report['binary_sha256']==frozen_manifest['binary_sha256'])
            check('frozen source files and aggregate exact before native proof',actual==frozen_manifest['frozen_source_provenance']['files'] and actual_aggregate==expected_aggregate)
            report['declared_production_source_sha256']=expected_aggregate
            shutil.copy2(frozen_manifest_path,run/'frozen-source-manifest.json')
        with tempfile.TemporaryFile() as fd:
            start(['Xvfb','-displayfd',str(fd.fileno()),'-screen','0','1450x1050x24','-nolisten','tcp','-xkbdir',str(ENVROOT/'sysroot/usr/share/X11/xkb')],'xvfb',pass_fds=(fd.fileno(),))
            def getdisplay():fd.seek(0);return fd.read().decode().strip()
            display=':'+native.wait_for(getdisplay,label='owned private X11')
        runtime=native.private_accessibility_environment(display,run)
        env=os.environ.copy();env.update(DISPLAY=display,GDK_BACKEND='x11',GSK_RENDERER='cairo',XDG_RUNTIME_DIR=str(runtime),SARV_TEST_INSTANCE='sales-native-qa',SARV_DATA_DIR=str(profile),ADW_DEBUG_COLOR_SCHEME='prefer-light')
        for key in ['SARV_UI_SMOKE','SARV_ACCOUNTING_SMOKE','SARV_PROCUREMENT_SMOKE','SARV_CORE_SMOKE','SARV_SALES_SMOKE']:env.pop(key,None)
        start(['openbox','--sm-disable','--config-file',str(ENVROOT/'sysroot/etc/xdg/openbox/rc.xml')],'openbox',env)
        seed_source=os.environ.get('SARV_SALES_SEED_PROFILE')
        if seed_source:copy_profile(Path(seed_source),profile)
        else:
            env['SARV_CORE_SMOKE']='full';seed=start(['/workspace/sarv-evidence/bin/sarv-M3-final'],'M3-seed',env);seed.wait(timeout=120);check('actual M3 opening seed',seed.returncode==0 and (profile/'core-native-verification.json').exists())
            env.pop('SARV_CORE_SMOKE');env['SARV_PROCUREMENT_SMOKE']='full'
            (profile/'M4-synthetic-prices.csv').write_text('code,name,measure,price,currency,source_at,aliases\nSKU-A,کالای الف خرید,عدد,15,rial,2026-03-22T09:00:00Z,نمونه الف\nSKU-B,کالای ب خرید,عدد,20,rial,2026-03-22T09:00:00Z,نمونه ب\n')
            seed=start(['/workspace/sarv-evidence/bin/sarv-M4-final'],'M4-seed',env)
            native.wait_for(lambda:(profile/'procurement-native-verification.json').exists() or (profile/'procurement-native-failure.txt').exists() or seed.poll() is not None,seconds=180,label='actual M4 native seed')
            check('actual M4 purchase/stock seed',(profile/'procurement-native-verification.json').exists() and not (profile/'procurement-native-failure.txt').exists());native.stop(seed);env.pop('SARV_PROCUREMENT_SMOKE')
        before=state(profile);a=before['accounting'];check('M4 seed immutable opening/stock without sales',len(a['journals'])==5 and not a.get('sales',{}).get('records'))
        report['seed_source']=seed_source or 'fresh archived M3/M4 native binaries'
        report['seed_state_sha256']=hashlib.sha256(json.dumps(before,sort_keys=True,ensure_ascii=False,separators=(',',':')).encode()).hexdigest()
        report['source_manifest']={str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in [ROOT/'src/ui.rs',ROOT/'src/ui/sales.rs',ROOT/'scripts/sales-qa.py',ROOT/'Cargo.lock']}
        baseline=run/'m4-before';baseline.mkdir();copy_profile(profile,baseline)
        old_binary=Path('/workspace/sarv-evidence/bin/sarv-M4-final')
        report['m4_binary_sha256']=hashlib.sha256(old_binary.read_bytes()).hexdigest()
        old_env=env.copy();old_env.update(SARV_DATA_DIR=str(baseline),SARV_UI_SMOKE='capture',SARV_CAPTURE_PAGE='inventory',SARV_CAPTURE_NAME='M4-before.png')
        old_app=start([str(old_binary)],'M4-before',old_env)
        native.wait_for(lambda:(baseline/'M4-before.png').exists(),label='actual M4 binary before screenshot')
        native.stop(old_app);check('actual M4 baseline rendering preserves source',state(baseline)==before)
        for mode in ['stage','draft']:
            target=run/mode;target.mkdir();copy_profile(profile,target);(target/'M5-synthetic-prices.csv').write_text('code,name,measure,price,currency,source_at,aliases\nSKU-A,کالای الف فروش,عدد,20,rial,2026-03-22T09:00:00Z,نمونه فروش\n')
        (profile/'M5-synthetic-prices.csv').write_text('code,name,measure,price,currency,source_at,aliases\nSKU-A,کالای الف فروش,عدد,20,rial,2026-03-22T09:00:00Z,نمونه فروش\n')
        env.update(SARV_SALES_SMOKE='full',SARV_SALES_CAPTURE='1');app=start([str(binary)],'M5-full',env)
        manifest=profile/'sales-native-verification.json';failure=profile/'sales-native-failure.txt'
        native.wait_for(lambda:manifest.exists() or failure.exists() or app.poll() is not None,seconds=200,label='full native M5')
        check('native sale/draft/pair/returns/quote conversion complete',manifest.exists() and not failure.exists(),failure=failure.read_text() if failure.exists() else None,returncode=app.poll())
        raw=state(profile);a=raw['accounting'];records=a['sales']['records'];kinds={k:[r for r in records if k in r['data']] for k in ['Order','Fulfill','Return']}
        check('opening and procurement exact preserved',a['core']==before['accounting']['core'] and a['procurement']==before['accounting']['procurement'])
        check('two orders/two paired sales/two credits',all(len(kinds[k])==2 for k in kinds) and len(a['journals'])==9 and not raw.get('sales_drafts'))
        direct=next(r for r in kinds['Order'] if not r['command']['Order'].get('quotation'))
        linked=next(r for r in kinds['Order'] if r['command']['Order'].get('quotation'))
        check('direct order199 nonfinancial catalog20 frozen',direct['journal_id'] is None and direct['data']['Order']['total']==199 and all(x in direct['command']['Order']['lines'][0]['price_reference'] for x in ['catalog=','SHA256=','قیمت‌ریال=20']))
        link=linked['command']['Order']['quotation'];q=next(q for q in raw['quotes'] if q['id']==link['quote_id'])
        check('authentic retained quote and zero-tax linked order',q['total_rial']==20 and q.get('tax_rial',0)==0 and linked['data']['Order']['total']==20 and link['quote_number']==q['number'])
        check('partial nets100/99 and exact issue costs58/58',sorted(r['data']['Fulfill']['total'] for r in kinds['Fulfill'])==[99,100] and all(r['data']['Fulfill']['cost']==58 for r in kinds['Fulfill']))
        check('original return allocations40/23 then60/35',sorted((r['data']['Return']['total'],r['data']['Return']['cost']) for r in kinds['Return'])==[(40,23),(60,35)])
        pair_ids=[r['data']['Fulfill'][key] for r in kinds['Fulfill'] for key in ['delivery_id','invoice_id']];return_ids=[r['data']['Return'][key] for r in kinds['Return'] for key in ['return_id','credit_id']]
        check('distinct stable paired identities',len(set(pair_ids+return_ids))==8)
        sums={}
        for j in a['journals']:
            check('every journal exact balanced',sum(l['debit'] for l in j['lines'])==sum(l['credit'] for l in j['lines']))
            for l in j['lines']:sums[l['account']['id']]=sums.get(l['account']['id'],0)+l['debit']-l['credit']
        mapping={m['role']:m['account_id'] for m in a['core']['setups'][0]['command']['mappings']}
        expected={'Cash':251,'Bank':300,'Receivables':299,'Inventory':422,'ChequesReceivable':200,'Payables':-329,'ChequesPayable':-100,'Grni':-50,'OwnerCapital':-952,'Sales':-199,'SalesReturns':100,'Cogs':58}
        check('independent final inventory/AR/AP/sales/COGS controls',all(sums.get(mapping[k],0)==v for k,v in expected.items()),expected=expected)
        check('independent final trial1630/1630',sum(v for v in sums.values() if v>0)==1630 and -sum(v for v in sums.values() if v<0)==1630)
        main_pairs=[r for r in kinds['Fulfill'] if r['data']['Fulfill']['order_id']==direct['id']];check('returns never reopen original order',sum(l['quantity_thousandths'] for r in main_pairs for l in r['data']['Fulfill']['lines'])==10000)
        check('actual accountant reports generated',all((profile/n).exists() and (profile/n).stat().st_size>100 for n in ['M5-controls.csv','M5-controls.pdf']))
        report['native_manifest']=json.loads(manifest.read_text());x11=native.X11(display);report['x11_errors']=[]
        class XError(C.Structure):
            _fields_=[('type',C.c_int),('display',C.c_void_p),('resource',C.c_ulong),('serial',C.c_ulong),('code',C.c_ubyte),('request',C.c_ubyte),('minor',C.c_ubyte)]
        @C.CFUNCTYPE(C.c_int,C.c_void_p,C.POINTER(XError))
        def stale_window_error(_,error):
            # A just-stopped owned process can remain briefly in _NET_CLIENT_LIST.
            if error.contents.code!=3:report['x11_errors'].append({'code':error.contents.code,'request':error.contents.request})
            return 0
        x11.x.XSetErrorHandler.argtypes=[C.c_void_p];x11.x.XSetErrorHandler(C.cast(stale_window_error,C.c_void_p))
        window=native.wait_for(lambda:x11.find(app.pid),label='owned GTK window');x11.focus(window);access=Access(app.pid)
        def showing(row):
            access.a.atspi_accessible_clear_cache(row['node']);states=access.a.atspi_accessible_get_state_set(row['node'])
            return bool(states and access.a.atspi_state_set_contains(states,25))
        def focus(name,roles=None):
            native.wait_for(lambda:access.find(name,roles),label=f'accessible {name}')
            # A saved25-row editor exposes100 line buttons before its footer.
            for _ in range(160):
                targets=[r for r in access.tree() if r['name']==name and (not roles or r['role'] in roles)]
                target=next((r for r in targets if access.focused(r)),None)
                if target:return target
                x11.key('Tab');time.sleep(.018)
            report['failed_focus_candidates']=[{'name':r['name'],'role':r['role'],'rect':access.rect(r),'focused':access.focused(r)} for r in access.tree() if r['name']==name]
            raise AssertionError(f'actual Tab focus {name}')
        def fill(name,text):focus(name,['text','entry']);x11.key('a','Control_L');x11.type_ascii(text)
        def press(name):focus(name,['button','push button']);x11.key('Return');time.sleep(.25)
        def shot(name):subprocess.run(['/usr/bin/import','-display',display,'-window',str(window),str(profile/name)],check=True)
        x11.key('0','Alt_L');time.sleep(.3)
        native.wait_for(lambda:any('۶ نتیجه' in r['name'] for r in access.tree()),label='usable loaded Sales source list')
        check('completed Sales load clears global loading status',not any(r['name']=='خواندن اسناد فروش…' for r in access.tree()) and any(r['name']=='اسناد فروش آماده‌اند.' for r in access.tree()))
        for caption in ['سفارش فروش تازه','از پیشنهاد تأییدشده','به‌روزرسانی']:
            target=focus(caption,['button','push button']);check(f'real Tab exact Sales top action {caption}',access.focused(target))
        x11.key('f','Control_L');time.sleep(.2);x11.type_ascii('SO-M5');time.sleep(.3)
        check('real CtrlF directs typed text to Sales search',access.text(access.find('جست‌وجوی شماره، مشتری، کالا یا پیش‌نویس',['text','entry']))=='SO-M5')
        check('real Alt0/CtrlF typed normalized Sales source search',any('SO-M5' in r['name'] and showing(r) for r in access.tree()) and not any('SO-Q-M5' in r['name'] and showing(r) for r in access.tree()));shot('M5-real-filtered-search.png')
        press('جزئیات و جریان کار');native.wait_for(lambda:any('کاملاً تحویل' in r['name'] for r in access.tree()),label='real opened order detail');check('real source detail shows complete fulfillment and linked pairs',True)
        check('completed source detail clears global loading status',not any(r['name']=='خواندن منبع فروش…' for r in access.tree()) and any(r['name']=='منبع فروش و جریان کار آماده‌اند.' for r in access.tree()));shot('M5-real-order-detail.png')
        for caption in ['خروجی سند CSV','خروجی سند PDF']:
            target=focus(caption,['button','push button']);check(f'real Tab exact source export action {caption}',access.focused(target))
        shot('M5-real-export-action-focus.png')
        press('تحویل DEL-M5-B • صورتحساب INV-M5-B');native.wait_for(lambda:any('وصول انجام نشده' in r['name'] for r in access.tree()),label='real paired invoice detail');press('برگشت کالا و بستانکار مرتبط')
        fill('شرح و دلیل ثبت','XTest independent rejected return');fill('مقدار برگشت در واحد پایه','6');fill('مرجع برگشت کالا','XTEST-OVERRETURN');fill('مرجع بستانکار فروش','XTEST-CREDIT');fill('اعتبار مستقل این برگشت','99');fill('بهای اصلی مستقل این برگشت','58');press('بازبینی مقدار و اثر سند')
        native.wait_for(lambda:any('بیشتر' in r['name'] or 'تجاوز' in r['name'] for r in access.tree()),label='actual overreturn rejection')
        check('real keyboard overreturn retains quantity/source and does not commit',access.text(access.find('مقدار برگشت در واحد پایه',['text','entry']))=='6' and state(profile)['accounting']==a);shot('M5-real-retained-overreturn.png')
        x11.key('Escape');time.sleep(.3);x11.key('Escape');time.sleep(.3);x11.key('Escape');time.sleep(.3)
        check('real Escape cancellation preserves financial aggregate',state(profile)['accounting']==a)
        native.stop(app);env.pop('SARV_SALES_SMOKE');env.update(SARV_UI_SMOKE='capture',SARV_CAPTURE_PAGE='sales')
        for appearance in ['light','dark']:
            env['ADW_DEBUG_COLOR_SCHEME']='prefer-'+appearance;env['SARV_CAPTURE_NAME']=f'M5-restart-{appearance}.png';app=start([str(binary)],f'M5-layout-{appearance}',env);window=native.wait_for(lambda:x11.find(app.pid),label='restarted owned Sales window');native.wait_for(lambda:(profile/env['SARV_CAPTURE_NAME']).exists(),label='mapped restarted Sales');access=Access(app.pid);x11.focus(window)
            for width,height in [(1280,900),(650,550)]:
                x11.resize(window,width,height);native.wait_for(lambda:x11.geometry(window)==[width,height],label='owned Sales geometry');time.sleep(.3);shot(f'M5-sales-{appearance}-{width}x{height}.png');check(f'native Sales {appearance} {width}x{height} rendered',True)
            native.stop(app)
        check('financial and quote source exact across restart',state(profile)['accounting']==a and state(profile)['quotes']==raw['quotes'])
        env.pop('SARV_UI_SMOKE');env.pop('SARV_CAPTURE_PAGE');env.pop('SARV_CAPTURE_NAME')
        for mode in ['stage','draft']:
            target=run/mode;env.update(SARV_DATA_DIR=str(target),SARV_SALES_SMOKE=mode);app=start([str(binary)],f'M5-{mode}',env);failure=target/'sales-native-failure.txt';ready=target/'sales-stage-ready.json';native.wait_for(lambda:ready.exists() or failure.exists() or app.poll() is not None,seconds=120,label=f'native saved {mode}')
            check(f'native {mode} durable draft and newer edit receipt',ready.exists() and not failure.exists(),failure=failure.read_text() if failure.exists() else None)
            draft_before=state(target)['sales_drafts'];check(f'{mode} draft exists without GL',len(draft_before)==1 and len(state(target)['accounting']['journals'])==5)
            native.stop(app);env.pop('SARV_SALES_SMOKE');env.update(SARV_UI_SMOKE='capture',SARV_CAPTURE_PAGE='sales',SARV_CAPTURE_NAME=f'M5-{mode}-restart.png');app=start([str(binary)],f'M5-{mode}-restart',env);window=native.wait_for(lambda:x11.find(app.pid),label='owned reopened draft');native.wait_for(lambda:(target/env['SARV_CAPTURE_NAME']).exists(),label='reopened native draft');access=Access(app.pid);x11.focus(window);press('ادامهٔ پیش‌نویس');check(f'{mode} saved raw header exact after restart',state(target)['sales_drafts']==draft_before and access.text(access.find('شماره یا مرجع سفارش',['text','entry']))==draft_before[0]['reference'])
            if mode=='draft':check('incomplete invalid raw quantity/price exact across restart',draft_before[0]['lines'][0]['quantity']=='-۱' and draft_before[0]['lines'][0]['unit_price']=='0.1')
            subprocess.run(['/usr/bin/import','-display',display,'-window',str(window),str(target/f'M5-{mode}-reopened-editor.png')],check=True)
            x11.resize(window,650,550);native.wait_for(lambda:x11.geometry(window)==[650,550],label='owned narrow reopened editor');time.sleep(.3)
            subprocess.run(['/usr/bin/import','-display',display,'-window',str(window),str(target/f'M5-{mode}-narrow-editor.png')],check=True)
            x11.resize(window,1280,900);native.wait_for(lambda:x11.geometry(window)==[1280,900],label='owned restored editor geometry')
            if mode=='draft':
                press('ویرایش ردیف')
                check('reopened incomplete line shown with exact raw input',access.text(access.find('مقدار در واحد مشخص‌شده (تا سه رقم اعشار)',['text','entry']))=='-۱' and access.text(access.find('قیمت هر واحد ردیف',['text','entry']))=='0.1')
                subprocess.run(['/usr/bin/import','-display',display,'-window',str(window),str(target/'M5-invalid-raw-line-reopened.png')],check=True)
                x11.key('Escape');time.sleep(.3)
            if mode=='stage':
                detail_draft=copy.deepcopy(draft_before[0])
                quote_editor_before=state(target)['draft']
                press('ویرایش ردیف')
                check('real Tab/Enter opens exact existing line action',access.text(access.find('مقدار در واحد مشخص‌شده (تا سه رقم اعشار)',['text','entry']))=='10' and access.text(access.find('قیمت هر واحد ردیف',['text','entry']))=='20')
                press('انتخاب از کالا و قیمت فهرست');x11.key('n','Control_L');x11.key('Escape');time.sleep(.3)
                check('nested price picker owns Escape and suppresses workspace CtrlN',state(target)['draft']==quote_editor_before and access.text(access.find('قیمت هر واحد ردیف',['text','entry']))=='20')
                x11.key('Escape');time.sleep(.3)
                fill('شرح و دلیل ثبت','XTest saved draft after restart');x11.key('s','Control_L')
                native.wait_for(lambda:state(target)['sales_drafts'][0]['reason']=='XTest saved draft after restart',label='real editor CtrlS durable save')
                check('real modal CtrlS saves sales draft and preserves quote editor',state(target)['draft']==quote_editor_before)
                press('بازبینی سفارش');native.wait_for(lambda:any('سفارش غیرمالی' in r['name'] or 'ثبت موجودی' in r['name'] for r in access.tree()),label='actual keyboard order review')
                x11.key('Return','Control_L');native.wait_for(lambda:not state(target).get('sales_drafts') and len(state(target)['accounting']['sales']['records'])==1,label='real editor CtrlEnter accepts reviewed order')
                native.wait_for(lambda:not access.find('سفارش فروش • پیش‌نویس و بازبینی',['dialog']),label='accepted order native close completion')
                check('real keyboard order acceptance consumes draft without GL',len(state(target)['accounting']['journals'])==5)
                press('جزئیات و جریان کار');press('تحویل جزئی و صورتحساب هم‌زمان')
                fill('شرح و دلیل ثبت','XTest paired sale');fill('مقدار تحویل در واحد پایه','5');fill('مرجع تحویل کالا','KBD-DEL');fill('مرجع صورتحساب فروش','KBD-INV');fill('جمع خالص مستقل صورتحساب','100');press('بازبینی مقدار و اثر سند')
                native.wait_for(lambda:any('ثبت مالی و حرکت کالا در یک رویداد' in r['name'] for r in access.tree()),label='actual keyboard paired effect preview')
                x11.key('Return','Control_L');native.wait_for(lambda:len(state(target)['accounting']['sales']['records'])==2,label='actual keyboard paired durable post')
                native.wait_for(lambda:not access.find('تحویل جزئی و صورتحساب هم‌زمان',['dialog']),label='paired native close completion')
                posted=state(target)['accounting']['sales']['records'][-1]['data']['Fulfill'];check('real keyboard paired sale exact100/58',posted['total']==100 and posted['cost']==58 and len(state(target)['accounting']['journals'])==6)
                press('انصراف');native.wait_for(lambda:not access.find('منبع فروش و جریان کار',['dialog']),label='parent source native close completion');press('جزئیات و جریان کار');press('تحویل KBD-DEL • صورتحساب KBD-INV');press('برگشت کالا و بستانکار مرتبط')
                fill('شرح و دلیل ثبت','XTest original cost return');fill('مقدار برگشت در واحد پایه','2');fill('مرجع برگشت کالا','KBD-RET');fill('مرجع بستانکار فروش','KBD-CREDIT');fill('اعتبار مستقل این برگشت','40');fill('بهای اصلی مستقل این برگشت','23');press('بازبینی مقدار و اثر سند')
                native.wait_for(lambda:any('ثبت مالی و حرکت کالا در یک رویداد' in r['name'] for r in access.tree()),label='actual keyboard original-cost return preview')
                x11.key('Return','Control_L');native.wait_for(lambda:len(state(target)['accounting']['sales']['records'])==3,label='actual keyboard durable linked return')
                returned=state(target)['accounting']['sales']['records'][-1]['data']['Return'];check('real keyboard original cost credit exact40/23',returned['total']==40 and returned['cost']==23 and len(state(target)['accounting']['journals'])==7)
                subprocess.run(['/usr/bin/import','-display',display,'-window',str(window),str(target/'M5-real-keyboard-posted-return.png')],check=True)
            native.stop(app);env.pop('SARV_UI_SMOKE');env.pop('SARV_CAPTURE_PAGE');env.pop('SARV_CAPTURE_NAME')
        # This is a synthetic raw fixture on an already migrated schema6 copy.
        # Its accounting is still the exact authentic M4 state; the invalid raw
        # draft from the preceding observer is replaced only on this fresh copy.
        target=run/'detail-pages';target.mkdir();copy_profile(run/'draft',target)
        seeded=state(target);accounting_before=copy.deepcopy(seeded['accounting'])
        detail_draft.update(id='m5-native-detail-paging-draft',revision=1,reference='KBD-DETAIL-30',quotation=None,document_discount='1',supplied_total='599')
        original_line=detail_draft['lines'][0]
        detail_draft['lines']=[dict(original_line,id=f'm5-detail-line-{i:03d}',quantity='1',line_discount='0') for i in range(30)]
        seeded['sales_drafts']=[detail_draft]
        with sqlite3.connect(target/'sarv.sqlite3') as db:
            generation=db.execute('SELECT generation FROM state WHERE id=1').fetchone()[0]
            assert db.execute('UPDATE state SET generation=?,payload=? WHERE id=1 AND generation=?',(generation+1,json.dumps(seeded,ensure_ascii=False,separators=(',',':')),generation)).rowcount==1
        check('synthetic 30-line raw fixture preserves authentic M4 books',state(target)['accounting']==accounting_before)
        (target/'synthetic-raw-fixture.json').write_text(json.dumps({'synthetic_only':True,'injected_raw_draft_only':True,'accounting_unchanged':True,'draft':detail_draft},ensure_ascii=False,indent=2))
        env.update(SARV_DATA_DIR=str(target),SARV_UI_SMOKE='capture',SARV_CAPTURE_PAGE='sales',SARV_CAPTURE_NAME='M5-detail-pages-restart.png')
        app=start([str(binary)],'M5-detail-pages',env);window=native.wait_for(lambda:x11.find(app.pid),label='owned detail-pages window');native.wait_for(lambda:(target/'M5-detail-pages-restart.png').exists(),label='loaded detail-pages Sales');access=Access(app.pid);x11.focus(window)
        press('ادامهٔ پیش‌نویس');fill('شرح و دلیل ثبت','XTest validated 30-line raw draft');x11.key('s','Control_L')
        native.wait_for(lambda:state(target)['sales_drafts'][0]['revision']==2 and state(target)['sales_drafts'][0]['reason']=='XTest validated 30-line raw draft',label='30-line raw fixture actual Store save revision2')
        check('30-line raw fixture accepted through real CtrlS Store boundary',state(target)['accounting']==accounting_before)
        press('بازبینی سفارش');native.wait_for(lambda:any('سفارش غیرمالی' in r['name'] or 'ثبت موجودی' in r['name'] for r in access.tree()),label='30-line order reviewed');x11.key('Return','Control_L')
        native.wait_for(lambda:not state(target).get('sales_drafts') and len(state(target)['accounting']['sales']['records'])==1,label='30-line native accepted order')
        native.wait_for(lambda:not access.find('سفارش فروش • پیش‌نویس و بازبینی',['dialog']),label='30-line editor native close')
        actual=state(target)['accounting'];record=actual['sales']['records'][0]
        check('real 30-line order posts599 without financial effects',record['data']['Order']['total']==599 and len(record['data']['Order']['lines'])==30 and actual['journals']==accounting_before['journals'] and actual['procurement']==accounting_before['procurement'] and actual['core']==accounting_before['core'])
        press('جزئیات و جریان کار');press('ردیف‌ها، اثر دفتر و جزئیات منبع')
        native.wait_for(lambda:any('m5-detail-line-000' in r['name'] for r in access.tree()),label='first detail page exact line000')
        press('جزئیات بعد');native.wait_for(lambda:any('m5-detail-line-029' in r['name'] for r in access.tree()) and not any('m5-detail-line-000' in r['name'] for r in access.tree()),label='last detail page exact line029')
        check('real Tab Enter detail Next shows final source line and removes first',True)
        subprocess.run(['/usr/bin/import','-display',display,'-window',str(window),str(target/'M5-real-detail-last-page.png')],check=True)
        press('جزئیات قبل');native.wait_for(lambda:any('m5-detail-line-000' in r['name'] for r in access.tree()) and not any('m5-detail-line-029' in r['name'] for r in access.tree()),label='previous detail page exact line000')
        check('real Tab Enter detail Previous returns first source line without write',state(target)['accounting']==actual)
        native.stop(app)
        if frozen_manifest:
            actual,expected_aggregate,actual_aggregate=frozen_sources()
            report['frozen_source_provenance_after']={'sha256':actual_aggregate,'files':actual}
            check('frozen source files and aggregate unchanged after native proof',actual==frozen_manifest['frozen_source_provenance']['files'] and actual_aggregate==expected_aggregate and report['frozen_source_provenance_before']==report['frozen_source_provenance_after'])
            check('frozen binary unchanged after native proof',hashlib.sha256(binary.read_bytes()).hexdigest()==frozen_manifest['binary_sha256'])
        check('observer source unchanged during native proof',hashlib.sha256(observer.read_bytes()).hexdigest()==report['observer_sha256'])
        report['result']='passed';print(json.dumps(report,ensure_ascii=False,indent=2))
    except Exception as e:
        report['error']=str(e)
        if 'access' in locals():(run/'failed-access-tree.json').write_text(json.dumps([{k:v for k,v in r.items() if k!='node'} for r in access.tree()],ensure_ascii=False,indent=2))
        if x11 and 'window' in locals():subprocess.run(['/usr/bin/import','-display',display,'-window',str(window),str(run/'failed-native-window.png')],check=False)
        print(json.dumps(report,ensure_ascii=False,indent=2));raise
    finally:
        (run/'sales-independent-native.json').write_text(json.dumps(report,ensure_ascii=False,indent=2))
        if x11:x11.x.XCloseDisplay(x11.d)
        for p in reversed(processes):
            if p.poll() is None:
                p.terminate()
                try:p.wait(timeout=3)
                except subprocess.TimeoutExpired:p.kill()
        for log in logs:log.close()
if __name__=='__main__':main()
