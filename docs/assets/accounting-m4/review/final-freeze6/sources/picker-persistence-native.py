#!/usr/bin/env python3
"""Independent owned native picker persistence observer; synthetic clone only.

Actual chooser input uses XTest. A real SQLite writer lock and an external
generation-only update force failure/race boundaries without changing books.
Underlying invalid input uses explicitly attributed AT-SPI EditableText.
"""
import ctypes as C
import hashlib, importlib.util, json, os, sqlite3, subprocess, tempfile, time
from pathlib import Path

ROOT=Path('/workspace/sarv-quote')
spec=importlib.util.spec_from_file_location('review',ROOT/'artifacts/independent-m4/native-review.py')
review=importlib.util.module_from_spec(spec);spec.loader.exec_module(review)
native,Access=review.native,review.Access
binary=Path(os.environ['SARV_QA_BINARY']);expected_sha=os.environ['SARV_QA_EXPECT_SHA'];expected_source=os.environ['SARV_QA_EXPECT_SOURCE']
fixture=Path(os.environ['SARV_M4_PICKER_FIXTURE'])
catalog_fixture=Path(os.environ.get('SARV_M4_PICKER_CATALOG_FIXTURE',str(ROOT/'artifacts/performance-m4/freeze5-repeated/quote-scale')))
def source_hash():
    h=hashlib.sha256()
    for p in sorted([*ROOT.glob('src/**/*.rs'),ROOT/'src/style.css',ROOT/'Cargo.toml',ROOT/'Cargo.lock'],key=lambda p:str(p.relative_to(ROOT))):
        h.update(str(p.relative_to(ROOT)).encode()+b'\0'+p.read_bytes()+b'\0')
    return h.hexdigest()
def persisted(profile):
    with sqlite3.connect(f'file:{profile}/sarv.sqlite3?mode=ro',uri=True) as db:
        return db.execute('select generation,payload from state where id=1').fetchone()
def state(profile):return json.loads(persisted(profile)[1])
def non_draft(s):return {k:v for k,v in s.items() if k!='draft'}
assert source_hash()==expected_source and hashlib.sha256(binary.read_bytes()).hexdigest()==expected_sha
original=persisted(fixture)
original_catalog=persisted(catalog_fixture)
run=Path(tempfile.mkdtemp(prefix='picker-persistence-final6-',dir=ROOT/'artifacts/independent-m4/native'))
report={'result':'failed','binary_sha256':expected_sha,'production_source_sha256':expected_source,'fixture':str(fixture),'original_generation':original[0],'original_payload_sha256':hashlib.sha256(original[1].encode()).hexdigest(),'run':str(run),'checks':[],'scope':'Real owned native picker keyboard + SQLite writer lock/CAS race; exact full non-draft State/history; explicit accessibility newer-input injection, not performance'}
processes=[];logs=[];x=None;locked=None
def start(cmd,name,env=None,**kw):
    log=(run/(name+'.log')).open('w');logs.append(log)
    p=subprocess.Popen(cmd,env=env,stdout=log,stderr=log,start_new_session=True,**kw);processes.append(p);return p
def check(name,condition,**details):
    report['checks'].append({'name':name,'passed':bool(condition),**details});print('CHECK',name,bool(condition),flush=True);assert condition,name
try:
    with tempfile.TemporaryFile() as fd:
        start(['Xvfb','-displayfd',str(fd.fileno()),'-screen','0','1450x1050x24','-nolisten','tcp','-xkbdir',str(native.ENVROOT/'sysroot/usr/share/X11/xkb')],'xvfb',pass_fds=(fd.fileno(),))
        def display_value():fd.seek(0);return fd.read().decode().strip()
        display=':'+native.wait_for(display_value,label='private X display')
    runtime=native.private_accessibility_environment(display,run)
    config=run/'xdg-config/gtk-4.0';config.mkdir(parents=True)
    (config/'settings.ini').write_text('[Settings]\ngtk-enable-animations=false\n')
    env=os.environ.copy();env.update(DISPLAY=display,GDK_BACKEND='x11',GSK_RENDERER='cairo',XDG_CONFIG_HOME=str(config.parent),XDG_RUNTIME_DIR=str(runtime),SARV_TEST_INSTANCE='independent-picker-final6',G_DEBUG='fatal-criticals')
    for k in ['SARV_UI_SMOKE','SARV_ACCOUNTING_SMOKE','SARV_CORE_SMOKE','SARV_PROCUREMENT_SMOKE','SARV_RESPONSIVENESS_SMOKE']:env.pop(k,None)
    start(['openbox','--sm-disable','--config-file',str(native.ENVROOT/'sysroot/etc/xdg/openbox/rc.xml')],'openbox',env)
    profile=run/'profile';review.copy_db(fixture,profile)
    setup=state(profile);catalog=next(c for c in state(catalog_fixture)['catalogs'] if len(c['products'])==50000)
    setup['catalogs'].append(catalog);code=catalog['products'][-1]['code'];price=catalog['products'][-1]['price_rial']
    customer={'id':'bda5e019-f536-4806-ab16-34e42d974a6e','name':'QA CUSTOMER','phone':'02100000000','contact_name':'QA CONTACT','email':'qa@example.invalid','address':'SYNTHETIC QA ADDRESS'}
    setup['customers'].append(customer)
    setup['draft']['catalog_id']=catalog['id'];setup['draft']['lines']=[{'query':'QA-PICKER-TARGET','quantity_milli':1000,'chosen_code':None}]
    # This direct mutation is synthetic fixture setup before application launch.
    # Financial/history remain exact. Additional50k synthetic catalog is a
    # separately attributed fixture copied before launch, never a live import.
    with sqlite3.connect(profile/'sarv.sqlite3') as db:db.execute('update state set payload=? where id=1',(json.dumps(setup,ensure_ascii=False,separators=(',',':')),))
    baseline=persisted(profile);books=non_draft(setup)
    report['setup']={'method':'observer-owned prelaunch single unmatched line plus50k synthetic catalog and labelled synthetic QA customer; no financial/schema/history changes','catalog_fixture':str(catalog_fixture),'catalog_payload_sha256':hashlib.sha256(original_catalog[1].encode()).hexdigest(),'catalog_id':catalog['id'],'catalog_products':len(catalog['products']),'customer':customer,'code':code,'price_rial':price,'generation':baseline[0]}
    env.update(SARV_DATA_DIR=str(profile),SARV_UI_SMOKE='capture',SARV_CAPTURE_PAGE='order',SARV_CAPTURE_NAME='startup.png')
    x=native.X11(display)
    def launch(name):
        p=start([str(binary)],name,env);window=native.wait_for(lambda:x.find(p.pid),label='owned window');x.focus(window)
        access=Access(p.pid)
        native.wait_for(lambda:access.app(),label='own accessibility app')
        return p,window,access
    app,window,access=launch('app')
    native.wait_for(lambda:(profile/'startup.png').exists(),seconds=45,label='startup capture')
    def walk(scope=None):
        nodes=[scope or access.app()]
        while nodes:
            node=nodes.pop(0)
            if not node:continue
            yield {'node':node,'name':access.string(access.a.atspi_accessible_get_name,node,None),'role':access.string(access.a.atspi_accessible_get_role_name,node,None)}
            nodes.extend(access.children(node))
    def find(name,roles=None,scope=None):return next((r for r in walk(scope) if r['name']==name and (not roles or r['role'] in roles)),None)
    def flag(row,n):
        access.a.atspi_accessible_clear_cache(row['node']);ss=access.a.atspi_accessible_get_state_set(row['node']);return bool(ss and access.a.atspi_state_set_contains(ss,n))
    def focus(name,roles=None,scope=None):
        row=native.wait_for(lambda:find(name,roles,scope),label=name)
        if not access.focus(row):
            for _ in range(120):
                x.key('Tab');time.sleep(.015)
                if access.focused(row):break
        native.wait_for(lambda:access.focused(row),label='actual focus '+name);return row
    def shot(name):subprocess.run(['import','-display',display,'-window',str(window),str(run/name)],check=True)
    def dialog():return find('انتخاب کالا از فهرست قیمت',['dialog'])
    def line_action():
        # Actual labelled-by text overrides the source accessible-name on this
        # unresolved row. The fixture intentionally renders exactly one line.
        action=native.wait_for(lambda:find('جست‌وجو و انتخاب انسانی کالای ردیف ۱',['button','push button']) or find('یافت نشد؛ انتخاب…',['button','push button']),label='actual single unresolved line choice')
        if not access.focus(action):
            for _ in range(120):
                x.key('Tab');time.sleep(.015)
                if access.focused(action):break
        native.wait_for(lambda:access.focused(action),label='real unresolved line button focus')
        return action
    def picker():
        line_action();x.key('space')
        d=native.wait_for(dialog,label='mapped current picker');s=focus('جست‌وجوی نام، کد یا نام مستعار کالا',['entry','text'],d['node'])
        x.key('a','Control_L');x.type_ascii(code)
        native.wait_for(lambda:any(code in r['name'] for r in walk(d['node'])),label='indexed exact result')
        time.sleep(.12);return s
    check('startup synthetic one-line fixture and all books/history exact',persisted(profile)==baseline and non_draft(state(profile))==books)
    search=picker();x.key('Escape');native.wait_for(lambda:not dialog(),label='actual picker cancellation')
    check('Escape before accepted choice performs no write',persisted(profile)==baseline)
    query=focus('کد یا نام مستعار ردیف ۱',['entry','text']);search=picker()
    a=access.a
    a.atspi_accessible_get_editable_text_iface.restype=C.c_void_p;a.atspi_accessible_get_editable_text_iface.argtypes=[C.c_void_p]
    a.atspi_editable_text_set_text_contents.restype=C.c_int;a.atspi_editable_text_set_text_contents.argtypes=[C.c_void_p,C.c_char_p,C.c_void_p]
    iface=a.atspi_accessible_get_editable_text_iface(query['node'])
    before_stale=persisted(profile)
    injected=bool(iface and a.atspi_editable_text_set_text_contents(iface,b'QA-CHANGED-QUERY',None))
    x.key('Return')
    native.wait_for(lambda:any('درخواست ردیف تغییر کرده' in r['name'] for r in walk()),seconds=12,label='stale original-query choice rejected')
    check('changed underlying query rejects stale choice without a separate form write',injected and access.text(query)=='QA-CHANGED-QUERY' and persisted(profile)==before_stale and dialog() is not None and flag(search,24),input_scope='owned EditableText before real Return')
    assert a.atspi_editable_text_set_text_contents(iface,b'QA-PICKER-TARGET',None)
    locked=sqlite3.connect(profile/'sarv.sqlite3');locked.execute('begin immediate')
    before=persisted(profile);x.key('Return')
    native.wait_for(lambda:not flag(search,24),label='disabled search during accepted worker')
    x.key('Escape');time.sleep(.2)
    check('accepted real writer-lock choice keeps dialog open and old full state exact',dialog() is not None and persisted(profile)==before and app.poll() is None)
    shot('writer-lock-busy.png')
    native.wait_for(lambda:flag(search,24),seconds=12,label='writer-lock error restores search')
    check('busy error re-enables retained indexed search without partial draft write',persisted(profile)==before and access.text(search)==code and dialog() is not None)
    shot('writer-lock-error.png');locked.rollback();locked.close();locked=None
    access.focus(search);x.key('Return')
    native.wait_for(lambda:not dialog(),seconds=15,label='retry choice durable receipt closes dialog')
    committed=persisted(profile);chosen=state(profile)
    check('retry commits form plus explicit product exactly once with original price and financial history',committed[0]==before[0]+1 and chosen['draft']['lines'][0]=={'query':'QA-PICKER-TARGET','quantity_milli':1000,'chosen_code':code} and non_draft(chosen)==books,source_price_rial=price)
    quantity=focus('تعداد ردیف ۱',['entry','text'])
    check('real quantity receives focus after successful chooser',access.focused(quantity))
    shot('choice-quantity-focus.png')
    # Reopening with a committed choice may render a small dropdown rather than
    # the explicit unknown-line browser; use fresh app/profile state below.
    native.stop(app)
    with sqlite3.connect(profile/'sarv.sqlite3') as db:
        changed=state(profile);changed['draft']['lines'][0]['chosen_code']=None
        db.execute('update state set generation=generation+1,payload=? where id=1',(json.dumps(changed,ensure_ascii=False,separators=(',',':')),))
    baseline2=persisted(profile);app,window,access=launch('app-newer')
    native.wait_for(lambda:find('جست‌وجو و انتخاب انسانی کالای ردیف ۱',['button','push button']) or find('یافت نشد؛ انتخاب…',['button','push button']),label='restarted unknown selector')
    quantity=focus('تعداد ردیف ۱',['entry','text']);search=picker()
    locked=sqlite3.connect(profile/'sarv.sqlite3');locked.execute('begin immediate');x.key('Return')
    native.wait_for(lambda:not flag(search,24),label='accepted atomic choice before newer input')
    iface=a.atspi_accessible_get_editable_text_iface(quantity['node'])
    injected=bool(iface and a.atspi_editable_text_set_text_contents(iface,b'bad-newer-quantity',None))
    check('accessibility injection sets newer invalid underlying widget while atomic choice waits',injected and access.text(quantity)=='bad-newer-quantity',input_scope='AT-SPI EditableText on own underlying widget, not ordinary focus route')
    locked.rollback();locked.close();locked=None
    native.wait_for(lambda:not dialog(),seconds=15,label='accepted old choice receipt')
    native.wait_for(lambda:access.text(quantity)=='bad-newer-quantity',label='newer invalid remains')
    saved=state(profile)
    check('older atomic receipt preserves newer invalid widget and exactly one old valid choice',persisted(profile)[0]==baseline2[0]+1 and saved['draft']['lines'][0]['chosen_code']==code and saved['draft']['lines'][0]['quantity_milli']==1000 and access.text(quantity)=='bad-newer-quantity' and non_draft(saved)==books)
    shot('newer-invalid-retained.png');native.stop(app)
    # Fresh restart sees committed valid choice, never the invalid newer text.
    app,window,access=launch('app-restart')
    quantity=focus('تعداد ردیف ۱',['entry','text'])
    check('restart recovers accepted valid choice and unchanged whole financial history',state(profile)==saved and non_draft(state(profile))==books and access.text(quantity) not in ['bad-newer-quantity',''])
    # The same detached choice pipeline selects a full customer snapshot. An
    # unrelated newer invalid quantity must not undo untouched identity fields.
    before_customer=persisted(profile)
    focus('انتخاب مشتری از دفتر مشتری‌ها',['button','push button']);x.key('space')
    cd=native.wait_for(lambda:find('انتخاب مشتری',['dialog']),label='current customer picker')
    cs=focus('جست‌وجوی مشتری، تلفن، رابط، ایمیل یا نشانی',['entry','text'],cd['node'])
    x.key('a','Control_L');x.type_ascii('QA CUSTOMER')
    native.wait_for(lambda:any(r['name']=='QA CUSTOMER' for r in walk(cd['node'])),label='exact synthetic customer result')
    time.sleep(.12);locked=sqlite3.connect(profile/'sarv.sqlite3');locked.execute('begin immediate');x.key('Return')
    native.wait_for(lambda:not flag(cs,24),label='accepted customer worker waits on real writer')
    iface=a.atspi_accessible_get_editable_text_iface(quantity['node'])
    injected=bool(iface and a.atspi_editable_text_set_text_contents(iface,b'bad-newer-customer-quantity',None))
    check('unrelated newer invalid widget arrives during accepted customer transaction',injected and access.text(quantity)=='bad-newer-customer-quantity',input_scope='owned AT-SPI EditableText')
    locked.rollback();locked.close();locked=None
    native.wait_for(lambda:not find('انتخاب مشتری',['dialog']),seconds=15,label='customer atomic receipt closes')
    customer_saved=state(profile);draft=customer_saved['draft']
    check('customer receipt selectively retains full snapshot while preserving newer invalid quantity',persisted(profile)[0]==before_customer[0]+1 and draft['customer_id']==customer['id'] and draft['customer']==customer['name'] and draft['phone']==customer['phone'] and draft['customer_contact']==customer['contact_name'] and draft['customer_email']==customer['email'] and draft['customer_address']==customer['address'] and draft['lines'][0]['quantity_milli']==1000 and access.text(quantity)=='bad-newer-customer-quantity' and non_draft(customer_saved)==books)
    shot('customer-newer-invalid-retained.png')
    native.stop(app)
    # Generation race is a separately attributed observer-owned writer, with
    # identical payload. The accepted candidate must never overwrite it.
    with sqlite3.connect(profile/'sarv.sqlite3') as db:
        race_setup=state(profile);race_setup['draft']['lines'][0]['chosen_code']=None
        db.execute('update state set generation=generation+1,payload=? where id=1',(json.dumps(race_setup,ensure_ascii=False,separators=(',',':')),))
    app,window,access=launch('app-race');search=picker();race_before=persisted(profile)
    locked=sqlite3.connect(profile/'sarv.sqlite3');locked.execute('begin immediate');x.key('Return')
    native.wait_for(lambda:not flag(search,24),label='accepted candidate before external generation race')
    locked.execute('update state set generation=generation+1 where id=1');locked.commit();locked.close();locked=None
    race_external=persisted(profile)
    native.wait_for(lambda:flag(search,24),seconds=12,label='CAS rejection restores search')
    check('external generation race rejects old accepted choice without overwriting payload or history',race_external[0]==race_before[0]+1 and race_external[1]==race_before[1] and persisted(profile)==race_external and non_draft(state(profile))==books and dialog() is not None and any('پنجرهٔ دیگری' in r['name'] for r in walk()))
    shot('stale-generation-error.png');x.key('Escape');native.wait_for(lambda:not dialog(),label='error-state native cancel')
    check('error-state actual Escape leaves newer external generation and exact payload untouched',persisted(profile)==race_external)
    native.stop(app);app,window,access=launch('app-race-restart');search=picker();x.key('Return')
    native.wait_for(lambda:not dialog(),seconds=15,label='fresh generation choice after restart')
    check('restart uses current generation and permits one exact retry without rewriting books',persisted(profile)[0]==race_external[0]+1 and state(profile)['draft']['lines'][0]['chosen_code']==code and non_draft(state(profile))==books)
    native.stop(app)
    bad=[]
    for path in run.glob('app*.log'):
        bad.extend(line for line in path.read_text(errors='replace').splitlines() if 'WARNING' in line or 'CRITICAL' in line)
    check('all actual owned app logs clean',not bad,diagnostics=bad)
    check('read-only source fixtures production source and binary unchanged',persisted(fixture)==original and persisted(catalog_fixture)==original_catalog and source_hash()==expected_source and hashlib.sha256(binary.read_bytes()).hexdigest()==expected_sha)
    report['result']='passed'
except BaseException as error:
    report['error']=repr(error)
    if 'access' in locals():
        (run/'failure-tree.json').write_text(json.dumps([{k:v for k,v in r.items() if k!='node'} for r in walk()],ensure_ascii=False,indent=2))
        try:shot('failure.png')
        except Exception:pass
    raise
finally:
    if locked:
        locked.rollback();locked.close()
    if x:x.x.XCloseDisplay(x.d)
    for p in reversed(processes):native.stop(p)
    for log in logs:log.close()
    (run/'picker-persistence-native.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps(report,ensure_ascii=False,indent=2),flush=True)
