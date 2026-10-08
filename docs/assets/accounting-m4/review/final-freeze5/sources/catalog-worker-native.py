#!/usr/bin/env python3
"""Independent native loading/cancel/reopen/last-code observer; synthetic copy only."""
import hashlib, importlib.util, json, os, sqlite3, subprocess, tempfile, time
from pathlib import Path

ROOT = Path('/workspace/sarv-quote')
SOURCE_ROOT=Path(os.environ.get('SARV_QA_SOURCE_TREE',str(ROOT)))
spec = importlib.util.spec_from_file_location('review', ROOT / 'artifacts/independent-m4/native-review.py')
review = importlib.util.module_from_spec(spec); spec.loader.exec_module(review)
native, Access = review.native, review.Access
binary = Path(os.environ['SARV_QA_BINARY'])
expected_sha = os.environ['SARV_QA_EXPECT_SHA']
expected_source = os.environ['SARV_QA_EXPECT_SOURCE']
fixture = Path(os.environ['SARV_M4_CATALOG_FIXTURE'])

def source_hash():
    h = hashlib.sha256()
    for p in sorted([*SOURCE_ROOT.glob('src/**/*.rs'), SOURCE_ROOT/'src/style.css', SOURCE_ROOT/'Cargo.toml', SOURCE_ROOT/'Cargo.lock'], key=lambda p: str(p.relative_to(SOURCE_ROOT))):
        h.update(str(p.relative_to(SOURCE_ROOT)).encode()+b'\0'+p.read_bytes()+b'\0')
    return h.hexdigest()

def persisted(profile):
    with sqlite3.connect(f'file:{profile}/sarv.sqlite3?mode=ro', uri=True) as db:
        return db.execute('select generation,payload from state where id=1').fetchone()

assert source_hash() == expected_source and hashlib.sha256(binary.read_bytes()).hexdigest() == expected_sha
original = persisted(fixture)
original_state = json.loads(original[1])
catalog = next(c for c in original_state['catalogs'] if len(c['products']) == 50000)
last = catalog['products'][-1]
assert last['code'] == 'SCALE-50000'
run = Path(tempfile.mkdtemp(prefix='catalog-worker-final5-', dir=ROOT/'artifacts/independent-m4/native'))
processes, logs = [], []
x11 = None
report = {'result':'failed', 'binary_sha256':expected_sha, 'production_source_sha256':expected_source,
          'source_snapshot':str(SOURCE_ROOT),'repository_epoch_may_advance':False, 'fixture':str(fixture), 'fixture_generation':original[0], 'fixture_payload_sha256':hashlib.sha256(original[1].encode()).hexdigest(),
          'run':str(run), 'scope':'Actual owned GTK controls/XTest keys; disposable SQLite backup; worker-caption-triggered native Close/reopen/indexed last product; transient loading sampling reported separately and exact persisted bytes/generation. Not performance measurement or forced worker-error injection.', 'checks':[]}

def start(cmd, name, env=None, **kw):
    log = (run/(name+'.log')).open('w'); logs.append(log)
    p = subprocess.Popen(cmd, env=env, stdout=log, stderr=log, start_new_session=True, **kw); processes.append(p); return p

def check(name, condition, **details):
    report['checks'].append({'name':name,'passed':bool(condition),**details})
    print('CHECK',name,bool(condition),flush=True)
    assert condition, name

try:
    with tempfile.TemporaryFile() as fd:
        start(['Xvfb','-displayfd',str(fd.fileno()),'-screen','0','1450x1050x24','-nolisten','tcp','-xkbdir',str(native.ENVROOT/'sysroot/usr/share/X11/xkb')], 'xvfb', pass_fds=(fd.fileno(),))
        def get_display(): fd.seek(0); return fd.read().decode().strip()
        display = ':'+native.wait_for(get_display,label='private display')
    runtime = native.private_accessibility_environment(display,run)
    config=run/'xdg-config';gtkconfig=config/'gtk-4.0';gtkconfig.mkdir(parents=True)
    (gtkconfig/'settings.ini').write_text('[Settings]\ngtk-enable-animations=false\n')
    report['private_gtk_animation_policy']='Animations disabled in disposable XDG_CONFIG_HOME to make close lifetime independent of transition duration. Retained default-animation attempts are distinct.'
    env = os.environ.copy(); env.update(XDG_CONFIG_HOME=str(config),DISPLAY=display,GDK_BACKEND='x11',GSK_RENDERER='cairo',XDG_RUNTIME_DIR=str(runtime),SARV_TEST_INSTANCE='independent-catalog-worker-final5',ADW_DEBUG_COLOR_SCHEME='prefer-light',G_DEBUG='fatal-criticals')
    for k in ['SARV_UI_SMOKE','SARV_ACCOUNTING_SMOKE','SARV_CORE_SMOKE','SARV_PROCUREMENT_SMOKE','SARV_RESPONSIVENESS_SMOKE']: env.pop(k,None)
    start(['openbox','--sm-disable','--config-file',str(native.ENVROOT/'sysroot/etc/xdg/openbox/rc.xml')], 'openbox',env)
    profile=run/'profile'; review.copy_db(fixture,profile)
    env.update(SARV_DATA_DIR=str(profile),SARV_UI_SMOKE='capture',SARV_CAPTURE_PAGE='catalog',SARV_CAPTURE_NAME='startup.png',SARV_EXPAND='1')
    app=start([str(binary)],'app',env); x11=native.X11(display)
    window=native.wait_for(lambda:x11.find(app.pid),label='owned window')
    native.wait_for(lambda:(profile/'startup.png').exists(),seconds=60,label='catalog startup')
    access=Access(app.pid); x11.focus(window)
    check('startup retains exact original payload and generation',persisted(profile)==original)

    def state(row, flag):
        access.a.atspi_accessible_clear_cache(row['node'])
        states=access.a.atspi_accessible_get_state_set(row['node'])
        return bool(states and access.a.atspi_state_set_contains(states,flag))
    def iter_scope():
        appnode=access.app();queue=[(appnode,0)] if appnode else [];dialog=None
        while queue:
            node,depth=queue.pop(0)
            role=access.string(access.a.atspi_accessible_get_role_name,node,None)
            if role=='dialog' and access.string(access.a.atspi_accessible_get_name,node,None)=='ساخت نسخهٔ تازهٔ فهرست':dialog=node;break
            if depth<4:queue.extend((child,depth+1) for child in access.children(node) if child)
        queue=[dialog or appnode] if (dialog or appnode) else []
        while queue:
            node=queue.pop(0)
            # GTK's name/children events invalidate cache; only target state
            # checks clear it. Do not walk100 rows before returning a header.
            yield {'node':node,'name':access.string(access.a.atspi_accessible_get_name,node,None),'role':access.string(access.a.atspi_accessible_get_role_name,node,None)}
            queue.extend(child for child in access.children(node) if child)
    def inside_composer(row):
        node=row['node']
        for _ in range(50):
            node=access.a.atspi_accessible_get_parent(node,None)
            if not node:return False
            role=access.string(access.a.atspi_accessible_get_role_name,node,None)
            if role=='dialog':return access.string(access.a.atspi_accessible_get_name,node,None)=='ساخت نسخهٔ تازهٔ فهرست'
            if role in ('frame','application'):return False
        return False
    def find(name, roles=None, showing=True):
        return next((r for r in iter_scope() if r['name']==name and (not roles or r['role'] in roles) and (name!='Close' or inside_composer(r)) and (not showing or state(r,25))),None)
    def product_edit(code):
        return next((r for r in iter_scope() if r['role'] in ('button','push button') and code in r['name'] and 'حذف' not in r['name'] and state(r,25)),None)
    def focus(name,roles=None):
        row=native.wait_for(lambda:find(name,roles),label=name)
        if access.focus(row):
            native.wait_for(lambda:access.focused(row),label='focus '+name); return row
        for _ in range(250):
            row=find(name,roles)
            if row and access.focused(row):return row
            x11.key('Tab',*(['Shift_L'] if name=='Close' else []));time.sleep(.012)
        raise AssertionError('real Tab never focused '+name)
    def press(name):
        if name=='Close':
            native.wait_for(lambda:find('ساخت نسخهٔ تازهٔ فهرست',['dialog']),label='mapped current top Composer before Escape')
            x11.key('Tab');time.sleep(.05);x11.key('Escape')
            report['native_close_input']='Actual XTest plain Escape on final5 current top Composer; header action attempts on4 retained separately, no pointer/header-focus claim.'
            return
        focus(name,['button','push button']);x11.key('space')
    def shot(name):
        subprocess.run(['import','-display',display,'-window',str(window),str(run/name)],check=True)
    def tree_save(name):
        (run/name).write_text(json.dumps([{k:v for k,v in r.items() if k!='node'} for r in iter_scope()],ensure_ascii=False,indent=2))
    def exact(): return persisted(profile)==original
    x11.key('2','Alt_L');time.sleep(.2)
    # Expander activation is a real keyboard action; title and role are captured.
    report['manager_setup']='Existing SARV_EXPAND capture hook opens original manager expander; Composer opening and cancellation use actual keyboard controls.'
    native.wait_for(lambda:find('ساخت نسخهٔ تازه',['button','push button']),label='expanded original catalogue')
    focus('ساخت نسخهٔ تازه',['button','push button'])
    footer=find('بازیابی پس از اجرای مجدد تأیید شد',['label'])
    assert footer is not None
    def footer_text():return access.text(footer)
    x11.key('space')
    deadline=time.monotonic()+3;started=False
    while time.monotonic()<deadline:
        if footer_text()=='آماده‌سازی کالاهای فهرست…':started=True;break
        if footer_text()=='کالاهای فهرست برای ویرایش آماده‌اند.':break
        time.sleep(.001)
    press('Close')
    native.wait_for(lambda:not find('ساخت نسخهٔ تازهٔ فهرست'),label='closed copied composer')
    cancelled='آماده‌سازی فهرست لغو شد؛ فهرست اصلی تغییر نکرد.'
    deadline=time.monotonic()+1
    while time.monotonic()<deadline and footer_text()!=cancelled:time.sleep(.005)
    discarded=footer_text()==cancelled
    report['worker_start_caption_observed']=started
    report['weak_discard_callback_observed']=discarded
    report['transient_loading_sampling_limit']='Previous default-motion full/scoped AT-SPI walks returned after Ready; neither loading visibility nor weak-discard branch is inferred if not directly observed.'
    check('actual final5 plain Escape closes copied dialog with exact State and generation',app.poll() is None and exact() and not find('ساخت نسخهٔ تازهٔ فهرست'),worker_start_observed=started,weak_discard_callback_observed=discarded)
    tree_save('catalog-cancel-callback-tree.json');shot('catalog-cancel-callback.png')
    check('late worker completion after cancellation cannot mutate source or generation',exact())
    # Reopen, allow completion, query the off-page last product from the new copy.
    press('ساخت نسخهٔ تازه')
    native.wait_for(lambda:product_edit('SCALE-00001'),seconds=30,label='reopened initial indexed100 rows')
    search=focus('جست‌وجوی کالا در ویرایش فهرست تازه',['entry','text'])
    check('fresh reopened composer becomes editable after worker completion',state(search,24) and exact())
    tree_save('catalog-ready-tree.json');shot('catalog-ready.png')
    x11.key('a','Control_L');x11.type_ascii('SCALE-50000')
    native.wait_for(lambda:product_edit('SCALE-50000'),label='indexed last product')
    rows=list(iter_scope());names=[r['name'] for r in rows]
    canonical=lambda n: ''.join(c for c in n.translate(str.maketrans('۰۱۲۳۴۵۶۷۸۹','0123456789')) if c not in ',٬')
    check('indexed last code is available with exact original price and full 50000-product copy',any('SCALE-50000' in n for n in names) and any('150000' in canonical(n) for n in names) and any('50000 کالا' in canonical(n) for n in names) and exact(),last_original=last)
    shot('catalog-last-code.png');tree_save('catalog-last-code-tree.json')
    press('Close');time.sleep(.3)
    check('discarding completed searched copy preserves exact original source and generation',exact() and persisted(fixture)==original)
    # A fresh mapped Composer must retain its own Escape after prior close/search.
    press('ساخت نسخهٔ تازه')
    native.wait_for(lambda:product_edit('SCALE-00001'),seconds=30,label='fresh current top Composer initial100 ready before Escape')
    press('Close')
    time.sleep(.6)
    check('fresh copied dialog native close and late completion preserve source and allow original manager',exact() and not find('ساخت نسخهٔ تازهٔ فهرست') and find('ساخت نسخهٔ تازه',['button','push button']) is not None)
    native.stop(app)
    bad=[line for line in (run/'app.log').read_text(errors='replace').splitlines() if 'WARNING' in line or 'CRITICAL' in line]
    check('actual GTK app log is clean',not bad,diagnostics=bad)
    check('original fixture current source and pinned executable remain unchanged',persisted(fixture)==original and source_hash()==expected_source and hashlib.sha256(binary.read_bytes()).hexdigest()==expected_sha)
    report['result']='passed'
except Exception as error:
    report['error']=str(error)
    if 'access' in locals():
        tree_save('failure-tree.json')
        try:shot('failure.png')
        except Exception:pass
    raise
finally:
    if x11:x11.x.XCloseDisplay(x11.d)
    for p in reversed(processes):native.stop(p)
    for f in logs:f.close()
    (run/'catalog-worker-native.json').write_text(json.dumps(report,ensure_ascii=False,indent=2))
    print(json.dumps(report,ensure_ascii=False,indent=2),flush=True)
