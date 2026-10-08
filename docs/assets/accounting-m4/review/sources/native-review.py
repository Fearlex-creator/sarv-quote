#!/usr/bin/env python3
"""Independent disposable native keyboard/state/source-button reviewer."""
import ctypes as C
import hashlib,importlib.util,json,os,re,sqlite3,subprocess,tempfile,time,sys,unicodedata
from pathlib import Path
sys.path.insert(0,'/workspace/sarv-quote/scripts')
from qa_access import Access
spec=importlib.util.spec_from_file_location('native','/workspace/sarv-quote/scripts/qa-native.py');native=importlib.util.module_from_spec(spec);spec.loader.exec_module(native)
ROOT=Path('/workspace/sarv-quote')
def state(p):
 with sqlite3.connect(f'file:{p}/sarv.sqlite3?mode=ro',uri=True) as db:return json.loads(db.execute('select payload from state where id=1').fetchone()[0])
def copy_db(source,dest):
 dest.mkdir()
 with sqlite3.connect(f'file:{source}/sarv.sqlite3?mode=ro',uri=True) as old,sqlite3.connect(dest/'sarv.sqlite3') as new:old.backup(new)
def main():
 binary=Path(os.environ['SARV_QA_BINARY']);digest=hashlib.sha256(binary.read_bytes()).hexdigest();assert digest==os.environ['SARV_QA_EXPECT_SHA']
 fixture=Path(os.environ['SARV_M4_FIXTURE']);run=Path(tempfile.mkdtemp(prefix='run-',dir=ROOT/'artifacts/independent-m4/native'));processes=[];logs=[];x11=None
 report={'result':'failed','binary':str(binary),'binary_sha256':digest,'fixture':str(fixture),'run':str(run),'scope':'independent external XTest keys, AT-SPI own-process discovery/focus plus actual chooser Open action, raw SQLite exact-state; disposable copy only','checks':[]}
 def start(cmd,name,env=None,**kw):
  log=(run/f'{name}.log').open('w');logs.append(log);p=subprocess.Popen(cmd,env=env,stdout=log,stderr=log,start_new_session=True,**kw);processes.append(p);return p
 def check(name,test,**details):
  report['checks'].append({'name':name,'passed':bool(test),**details});print('CHECK',name,bool(test),flush=True);assert test,name
 try:
  with tempfile.TemporaryFile() as fd:
   start(['Xvfb','-displayfd',str(fd.fileno()),'-screen','0','1450x1050x24','-nolisten','tcp','-xkbdir',str(native.ENVROOT/'sysroot/usr/share/X11/xkb')],'xvfb',pass_fds=(fd.fileno(),))
   def display_value():fd.seek(0);return fd.read().decode().strip()
   display=':'+native.wait_for(display_value,label='owned private display')
  runtime=native.private_accessibility_environment(display,run);env=os.environ.copy();env.update(DISPLAY=display,GDK_BACKEND='x11',GSK_RENDERER='cairo',XDG_RUNTIME_DIR=str(runtime),SARV_TEST_INSTANCE='independent-m4-review',ADW_DEBUG_COLOR_SCHEME='prefer-light',G_DEBUG='fatal-criticals')
  for k in ['SARV_UI_SMOKE','SARV_ACCOUNTING_SMOKE','SARV_CORE_SMOKE','SARV_PROCUREMENT_SMOKE']:env.pop(k,None)
  start(['openbox','--sm-disable','--config-file',str(native.ENVROOT/'sysroot/etc/xdg/openbox/rc.xml')],'openbox',env)
  stage=run/'stage'
  if (fixture/'M3-before/sarv.sqlite3').exists():copy_db(fixture/'M3-before',stage)
  else:
   recoveries=list((fixture/'full').glob('before-schema5-*.json'));assert len(recoveries)==1;seed=json.loads(recoveries[0].read_text());assert seed['format']=='sarv-accounting-backup-v4' and seed['state']['schema']==4;stage.mkdir()
   with sqlite3.connect(stage/'sarv.sqlite3') as db:db.execute('create table state(id integer primary key,generation integer not null,payload text not null)');db.execute('insert into state values(1,0,?)',[json.dumps(seed['state'])])
   report['stage_original_v4_recovery']=str(recoveries[0])
  (stage/'M4-synthetic-prices.csv').write_text('code,name,measure,price,currency,source_at,aliases\nSKU-A,کالای الف خرید,عدد,15,rial,2026-03-22T09:00:00Z,نمونه الف\nSKU-B,کالای ب خرید,عدد,20,rial,2026-03-22T09:00:00Z,نمونه ب\n')
  env.update(SARV_DATA_DIR=str(stage),SARV_PROCUREMENT_SMOKE='stage');app=start([str(binary)],'stage',env);x11=native.X11(display);xerrors=report.setdefault('x11_errors',[])
  class XError(C.Structure):
   _fields_=[('type',C.c_int),('display',C.c_void_p),('resource',C.c_ulong),('serial',C.c_ulong),('code',C.c_ubyte),('request',C.c_ubyte),('minor',C.c_ubyte)]
  handler_type=C.CFUNCTYPE(C.c_int,C.c_void_p,C.POINTER(XError))
  def handler(_,err):xerrors.append({'code':err.contents.code,'request':err.contents.request});return 0
  xhandler=handler_type(handler);x11.x.XSetErrorHandler.argtypes=[handler_type];x11.x.XSetErrorHandler.restype=C.c_void_p;x11.x.XSetErrorHandler(xhandler)
  window=native.wait_for(lambda:x11.find(app.pid),label='owned staged window')
  native.wait_for(lambda:(stage/'procurement-stage-ready.json').exists() or (stage/'procurement-native-failure.txt').exists(),seconds=180,label='native stage pause')
  check('native stage pauses before PO effect',(stage/'procurement-stage-ready.json').exists(),failure=(stage/'procurement-native-failure.txt').read_text() if (stage/'procurement-native-failure.txt').exists() else None)
  time.sleep(.4);access=Access(app.pid);x11.focus(window);before=state(stage)
  def find(name,roles=None):return access.find(name,roles)
  def focus(name,roles=None):
   x11.focus(window);row=native.wait_for(lambda:find(name,roles),label=f'owned control {name}')
   if access.focus(row):
    native.wait_for(lambda:access.focused(row),label=f'focused {name}');return row
   for _ in range(200):
    row=find(name,roles)
    if row and access.focused(row):return row
    x11.key('Tab');time.sleep(.015)
   raise AssertionError(f'real Tab focus {name}')
  def mapped_sensitive(row):
   access.a.atspi_accessible_clear_cache(row['node']);states=access.a.atspi_accessible_get_state_set(row['node']);return bool(states and access.a.atspi_state_set_contains(states,25) and access.a.atspi_state_set_contains(states,24)) # introspected SHOWING/SENSITIVE
  def ctrl_search(name,tag):
   x11.key('f','Control_L');row=native.wait_for(lambda:next((r for r in access.tree() if r['name']==name and r['role']=='entry' and mapped_sensitive(r) and access.focused(r)),None),label=f'actual CtrlF {tag} exact search focus');check(f'actual CtrlF focuses {tag} exact named SearchEntry',row is not None and mapped_sensitive(row) and access.focused(row));return row
  def fill(name,value):focus(name,['text','entry']);x11.key('a','Control_L');x11.type_ascii(value)
  def press(name,roles=None,key='space'):focus(name,roles or ['button','push button','page tab']);x11.key(key);time.sleep(.25)
  def enabled(name):
   r=find(name,['button','push button'])
   if not r:return False
   access.a.atspi_accessible_clear_cache(r['node']);s=access.a.atspi_accessible_get_state_set(r['node']);return bool(s and access.a.atspi_state_set_contains(s,24))
  def control_nodes():
   candidates=[]
   for pane in access.tree():
    if pane['role']!='scroll pane':continue
    access.a.atspi_accessible_clear_cache(pane['node']);states=access.a.atspi_accessible_get_state_set(pane['node'])
    if not states or not access.a.atspi_state_set_contains(states,25):continue
    queue=[pane['node']];rows=[]
    while queue:
     node=queue.pop(0);rows.append({'node':node,'name':access.string(access.a.atspi_accessible_get_name,node,None),'role':access.string(access.a.atspi_accessible_get_role_name,node,None)});queue.extend(access.children(node))
    count=sum(' • دفتر ' in r['name'] and ' • تفصیل ' in r['name'] and ' • اختلاف ' in r['name'] for r in rows)
    if count:candidates.append((count,rows))
   return min(candidates,key=lambda x:x[0])[1] if candidates else []
  def normalized(text):
   return ''.join(c for c in text.translate(str.maketrans('۰۱۲۳۴۵۶۷۸۹٠١٢٣٤٥٦٧٨٩٬٫','01234567890123456789,.')) if unicodedata.category(c)!='Cf')
  def exact_controls(rows):
   expected={'صندوق':251,'بانک':300,'حساب‌های دریافتنی':200,'اسناد دریافتنی':200,'موجودی کالا':480,'پیش‌پرداخت تأمین‌کنندگان':0,'حساب‌های پرداختنی':329,'اسناد پرداختنی':100,'کالای دریافت‌شده، صورتحساب‌نشده':50,'پیش‌دریافت مشتریان':0,'بهای تمام‌شده کالای فروش‌رفته':0,'ضایعات و کسری موجودی':0}
   # Only decimal glyphs/isolate controls are normalized; labels and actual money units remain exact.
   observed={}
   for text in rows:
    role,ledger,detail,difference=text.split(' • ');amounts=[]
    for prefix,value in [('دفتر',ledger),('تفصیل',detail),('اختلاف',difference)]:
     match=re.fullmatch(prefix+r' ([0-9]+) ریال',normalized(value));assert match,(prefix,value);amounts.append(int(match.group(1)))
    observed[role]=amounts
   assert observed=={role:[value,value,0] for role,value in expected.items()},observed
   return observed
  def expanded(name):
   r=find(name,['button','push button'])
   if not r:return False
   access.a.atspi_accessible_clear_cache(r['node']);s=access.a.atspi_accessible_get_state_set(r['node']);return bool(s and access.a.atspi_state_set_contains(s,10)) # introspected ATSPI_STATE_EXPANDED
  def shot(name):subprocess.run(['import','-display',display,'-window',str(window),str(run/name)],check=True)
  action='تأیید و ثبت نسخهٔ بازبینی‌شده';preview='بازبینی مقدار و اثر سند'
  fill('جمع مستقل سند در واحد مبلغ انتخاب‌شده','181');press(preview);native.wait_for(lambda:any('برابر نیست' in r['name'] for r in access.tree()),label='literal mismatched source total error')
  check('actual XTest181 total error retains value and no effect',not enabled(action) and access.text(find('جمع مستقل سند در واحد مبلغ انتخاب‌شده',['text','entry']))=='181' and state(stage)==before);shot('stage-total181-error.png')
  fill('جمع مستقل سند در واحد مبلغ انتخاب‌شده','180');press(preview);native.wait_for(lambda:enabled(action),label='exact180 preview enables post')
  fill('جمع مستقل سند در واحد مبلغ انتخاب‌شده','179');native.wait_for(lambda:not enabled(action),label='modified source disables post');check('editing reviewed content invalidates post',not enabled(action) and state(stage)==before)
  fill('جمع مستقل سند در واحد مبلغ انتخاب‌شده','180');press(preview);native.wait_for(lambda:enabled(action),label='fresh exact preview')
  for width,height in [(1280,900),(650,760)]:x11.resize(window,width,height);native.wait_for(lambda:x11.geometry(window)==[width,height]);time.sleep(.3);shot(f'stage-preview-light-{width}.png')
  x11.resize(window,1280,900);native.wait_for(lambda:x11.geometry(window)==[1280,900]);press(action,key='Return');native.wait_for(lambda:len(state(stage)['accounting']['procurement']['records'])==5,label='independent keyboard posted PO')
  posted=state(stage);orders=[r for r in posted['accounting']['procurement']['records'] if 'Order' in r['data']]
  check('real Enter posts one immutable180 PO/no financial effect',len(orders)==1 and orders[0]['data']['Order']['total']==180 and orders[0]['journal_id'] is None and posted['accounting']['journals']==before['accounting']['journals'] and posted['accounting']['core']==before['accounting']['core'])
  time.sleep(.8);ctrl_search('جست‌وجوی شماره، منبع، کالا یا طرف','purchases');press('سفارش خرید تازه');fill('شماره یا مرجع سند','INDEPENDENT-CANCEL');focus('انصراف');x11.key('Escape');time.sleep(.35);check('actual Escape cancels draft without changing posted state',state(stage)==posted)
  native.stop(app);env.pop('SARV_PROCUREMENT_SMOKE');env.update(SARV_UI_SMOKE='capture',SARV_CAPTURE_PAGE='procurement',SARV_CAPTURE_NAME='independent-stage-restart.png');app=start([str(binary)],'stage-restart',env);window=native.wait_for(lambda:x11.find(app.pid));native.wait_for(lambda:(stage/'independent-stage-restart.png').exists());access=Access(app.pid);x11.focus(window);check('keyboard PO and discarded draft exact across restart',state(stage)==posted);native.stop(app)
  full=run/'full';full.mkdir();saved=state(fixture/'full');env.update(SARV_DATA_DIR=str(full),SARV_CAPTURE_PAGE='backup',SARV_CAPTURE_NAME='independent-fresh.png');app=start([str(binary)],'fresh-restore',env);window=native.wait_for(lambda:x11.find(app.pid));native.wait_for(lambda:(full/'independent-fresh.png').exists());time.sleep(.4);access=Access(app.pid);x11.focus(window);fresh_before=state(full)
  x11.key('5','Alt_L');time.sleep(.3);press('انتخاب پشتیبان و بررسی بازیابی',['button','push button']);native.wait_for(lambda:any(r['role']=='dialog' and r['name']=='انتخاب فایل' for r in access.tree()),label='actual native restore file chooser')
  x11.key('l','Control_L');time.sleep(.2);x11.type_ascii(str(ROOT/'artifacts/independent-m4/native-full-backup.json'));time.sleep(.2)
  native.wait_for(lambda:enabled('Open'),label='selected backup permits native Open');opener=native.wait_for(lambda:next((r for r in access.tree() if r['name'] in ['Open','گشودن','باز کردن'] and r['role'] in ['button','push button']),None),label='actual chooser Open action');assert access.click(opener)
  native.wait_for(lambda:find('بازیابی و ذخیرهٔ نسخهٔ ایمنی',['button','push button']),label='verified restore preview');preview_names=[r['name'] for r in access.tree()]
  check('actual chooser verified preview distinguishes1opening/10operational/5journals',any('۱ افتتاحیه' in n and '۱۰ رویداد خرید و انبار' in n for n in preview_names) and any('۵ سند ثبت' in n for n in preview_names) and state(full)==fresh_before)
  shot('actual-full-restore-preview.png');press('بازیابی و ذخیرهٔ نسخهٔ ایمنی');native.wait_for(lambda:state(full)==saved,seconds=30,label='fresh UI restored exact complete operational snapshot');time.sleep(.7)
  check('actual native full restore retains full10records/5J/core/catalog/requests/audit',state(full)==saved);x11.key('8','Alt_L');time.sleep(.4);native.wait_for(lambda:any('6 سند عملیاتی • 4 رویداد تعریف منابع • 10 نتیجه' in r['name'] for r in access.tree()),label='rebuilt purchases truthfully distinguishes six documents and four resource events');check('document summary distinguishes6documents/4resourceevents',any('6 سند عملیاتی • 4 رویداد تعریف منابع • 10 نتیجه' in r['name'] for r in access.tree()));shot('actual-restored-purchases.png');x11.key('9','Alt_L');time.sleep(.4);ctrl_search('جست‌وجوی شماره، منبع، کالا یا طرف','inventory');shot('actual-restored-inventory.png')
  x11.resize(window,1280,900);native.wait_for(lambda:x11.geometry(window)==[1280,900]);x11.key('9','Alt_L');time.sleep(.3);ctrl_search('جست‌وجوی شماره، منبع، کالا یا طرف','sidebar inventory anchor');sidebar_reached=False
  for _ in range(100):
   row=find('موجودی و انتقال',['list item'])
   if row and mapped_sensitive(row) and access.focused(row):sidebar_reached=True;break
   x11.key('Tab');time.sleep(.025)
  check('actual Tab enters sidebar at selected Inventory row',sidebar_reached);x11.key('Up');time.sleep(.15);x11.key('Return');native.wait_for(lambda:native.events(full) and native.events(full)[-1]['page']=='procurement',label='selected sidebar UpEnter Purchases');check('actual sidebar UpEnter opens Purchases',native.events(full)[-1]['page']=='procurement');x11.key('Up');time.sleep(.15);x11.key('Return');native.wait_for(lambda:native.events(full) and native.events(full)[-1]['page']=='accounting',label='selected sidebar UpEnter Accounting');check('actual sidebar UpEnter opens Accounting unchanged',native.events(full)[-1]['page']=='accounting' and state(full)==saved);shot('sidebar-current-row-arrows.png')
  x11.key('7','Alt_L');time.sleep(.3);press('اسناد',['page tab']);time.sleep(.3)
  records=[r for r in saved['accounting']['procurement']['records'] if r['journal_id']]
  for record in records:
   jid=record['journal_id'];kind,input=next(iter(record['command'].items()));reference=input['reference']
   search_name=next((r['name'] for r in access.tree() if r['role']=='entry' and r['name'] in ['شماره، منبع، رویداد، شرح یا حساب…','جست‌وجوی اسناد','جست‌وجوی اسناد حسابداری','']),None);assert search_name is not None
   ctrl_search('جست‌وجوی اسناد حسابداری',f'ledger {reference}');fill(search_name,jid);native.wait_for(lambda:sum(r['name']=='منبع خرید و اثر موجودی' and r['role'] in ['button','push button'] for r in access.tree())==1,label='one exact source button')
   check(f'protected {kind} journal has no generic reversal',not any(r['name']=='سند معکوس با دلیل' for r in access.tree()))
   def source_nodes():
    dialog=next((r for r in access.tree() if 'فقط خواندنی' in r['name'] and r['role']=='dialog'),None)
    if not dialog:return []
    queue=[dialog['node']];rows=[]
    while queue:
     node=queue.pop(0);rows.append({'node':node,'name':access.string(access.a.atspi_accessible_get_name,node,None),'role':access.string(access.a.atspi_accessible_get_role_name,node,None)});queue.extend(access.children(node))
    return rows
   press('منبع خرید و اثر موجودی',['button','push button']);native.wait_for(lambda:any(reference in r['name'] for r in source_nodes()) and any('نسخهٔ ثبت‌شده ثابت است' in r['name'] for r in source_nodes()),label=f'correct immutable {reference} source')
   check(f'Alt7 source resolves immutable {reference}',any(reference in r['name'] for r in source_nodes()) and not any(r['name']==action for r in source_nodes()) and state(full)==saved)
   shot(f'ledger-source-{reference}.png')
   if reference=='BILL-B':
    focus('انصراف');reached=False
    for _ in range(80):
     target=find('جزئیات کامل منبع، شناسه‌ها و اثر ثبت‌شده',['button','push button'])
     if target and mapped_sensitive(target) and access.focused(target):reached=True;break
     x11.key('Tab');time.sleep(.025)
    check('actual Tab reaches full source disclosure from Cancel',reached);x11.key('space');time.sleep(.25)
   else:press('جزئیات کامل منبع، شناسه‌ها و اثر ثبت‌شده',['button','push button'])
   native.wait_for(lambda:any(record['id'] in r['name'] for r in source_nodes()),label=f'actual expanded immutable {reference} trace');check(f'actual Space expands canonical source trace {reference}',expanded('جزئیات کامل منبع، شناسه‌ها و اثر ثبت‌شده') and any(record['id'] in r['name'] for r in source_nodes()) and state(full)==saved);shot(f'ledger-trace-{reference}.png');press('جزئیات کامل منبع، شناسه‌ها و اثر ثبت‌شده',['button','push button']);check(f'actual Space collapses source trace {reference}',not expanded('جزئیات کامل منبع، شناسه‌ها و اثر ثبت‌شده') and state(full)==saved);focus('انصراف');x11.key('Escape');time.sleep(.3);x11.key('7','Alt_L');time.sleep(.3);press('اسناد',['page tab']);time.sleep(.3)
  x11.key('9','Alt_L');time.sleep(.3);press('کنترل و تفصیل',['page tab']);fill('پایان روز گزارش','2026-03-20');press('به‌روزرسانی پایان روز');native.wait_for(lambda:any('پیش از افتتاحیه' in r['name'] for r in access.tree()),label='preopening operational controls explicitly unavailable');shot('operational-preopening-unavailable.png');check('native cutoff before opening reports unavailable without stale or zero controls',state(full)==saved and not any(' • دفتر ' in r['name'] and ' • تفصیل ' in r['name'] and ' • اختلاف ' in r['name'] for r in control_nodes()))
  fill('پایان روز گزارش','2026-03-27');press('به‌روزرسانی پایان روز');native.wait_for(lambda:any('480' in r['name'] or '۴۸۰' in r['name'] for r in access.tree()),label='final inventory480 current controls');shot('operational-final-control.png');check('native final EOD controls return after unavailable cutoff',state(full)==saved)
  for appearance in ['light','dark']:
   native.stop(app);env['ADW_DEBUG_COLOR_SCHEME']='prefer-'+appearance;env['SARV_CAPTURE_NAME']=f'independent-{appearance}.png';app=start([str(binary)],f'layout-{appearance}',env);window=native.wait_for(lambda:x11.find(app.pid));native.wait_for(lambda:(full/env['SARV_CAPTURE_NAME']).exists());access=Access(app.pid);x11.focus(window)
   for page,key in [('procurement','8'),('inventory','9')]:
    x11.key(key,'Alt_L');time.sleep(.4)
    for width,height in [(1280,900),(650,760)]:x11.resize(window,width,height);native.wait_for(lambda:x11.geometry(window)==[width,height]);time.sleep(.3);shot(f'{page}-{appearance}-{width}.png')
   x11.key('9','Alt_L');time.sleep(.3);press('کنترل و تفصیل',['page tab']);fill('پایان روز گزارش','2026-03-27');press('به‌روزرسانی پایان روز');native.wait_for(lambda:any('480' in r['name'] or '۴۸۰' in r['name'] for r in access.tree()),label='layout explicit final cutoff')
   control_rows=[r['name'] for r in control_nodes() if ' • دفتر ' in r['name'] and ' • تفصیل ' in r['name'] and ' • اختلاف ' in r['name']];check(f'{appearance} explicit final has twelve literal reconciled controls',len(control_rows)==12,observed=exact_controls(control_rows));(run/f'controls-{appearance}-accessible.json').write_text(json.dumps([r['name'] for r in control_nodes()],ensure_ascii=False,indent=2));detail_names=[normalized(r['name']) for r in control_nodes()];check(f'{appearance} actual pool detail retains W1=30/350 W2=12/130 and distinct receipt paper/availability labels',any('انبار اول • مقدار پایه 30.000 • ارزش 350 ریال' in n for n in detail_names) and any('انبار دوم • مقدار پایه 12.000 • ارزش 130 ریال' in n for n in detail_names) and any('دسترسی مالی' in n for n in detail_names) and any('تاریخ برگهٔ منبع' in n for n in detail_names))
   for width,height,steps in [(1280,900,7),(650,760,14)]:
    x11.resize(window,width,height);native.wait_for(lambda:x11.geometry(window)==[width,height]);time.sleep(.3);ox,oy=x11.origin(window)
    for _ in range(25):x11.pointer(ox+width//2,oy+height-155,4)
    time.sleep(.25)
    for step in range(steps):
     shot(f'controls-{appearance}-{width}-scroll{step}.png');x11.pointer(ox+width//2,oy+height-155,5);time.sleep(.18)
    for _ in range(25):x11.pointer(ox+width//2,oy+height-155,5)
    time.sleep(.25);shot(f'controls-{appearance}-{width}-bottom.png')

   x11.key('7','Alt_L');time.sleep(.3);press('اسناد',['page tab']);time.sleep(.3)
   search_name=next((r['name'] for r in access.tree() if r['role']=='entry' and r['name'] in ['شماره، منبع، رویداد، شرح یا حساب…','جست‌وجوی اسناد','جست‌وجوی اسناد حسابداری','']),None);assert search_name is not None
   bill=next(r for r in records if next(iter(r['command'].values()))['reference']=='BILL-B');fill(search_name,bill['journal_id']);native.wait_for(lambda:sum(r['name']=='منبع خرید و اثر موجودی' and r['role'] in ['button','push button'] for r in access.tree())==1,label='narrow exact source button')
   press('منبع خرید و اثر موجودی');native.wait_for(lambda:any('BILL-B' in r['name'] for r in source_nodes()),label='narrow source dialog');shot(f'ledger-source-BILL-B-{appearance}-650.png');check(f'{appearance} narrow source retains immutable BILL-B',any('BILL-B' in r['name'] for r in source_nodes()) and state(full)==saved);focus('انصراف');x11.key('Escape');time.sleep(.3)
  check('full source drilldown/layout/restarts never alter financial state',state(full)==saved)
  legacy_backup=Path(os.environ.get('SARV_M4_LEGACY_BACKUP',str(ROOT/'artifacts/independent-m4/legacy-prefix-1/old-m3-manual-prefix-v4.json')));legacy_expected=json.loads(legacy_backup.read_text())['state'];legacy_expected['schema']=5
  native.stop(app);legacy=run/'legacy-prefix';legacy.mkdir();env.update(SARV_DATA_DIR=str(legacy),SARV_CAPTURE_PAGE='backup',SARV_CAPTURE_NAME='independent-legacy-fresh.png');app=start([str(binary)],'legacy-prefix-native',env);window=native.wait_for(lambda:x11.find(app.pid));native.wait_for(lambda:(legacy/'independent-legacy-fresh.png').exists());access=Access(app.pid);x11.focus(window);x11.key('5','Alt_L');time.sleep(.3);press('انتخاب پشتیبان و بررسی بازیابی');native.wait_for(lambda:any(r['role']=='dialog' and r['name']=='انتخاب فایل' for r in access.tree()),label='legacy actual restore chooser');x11.key('l','Control_L');time.sleep(.2);x11.type_ascii(str(legacy_backup.resolve()));native.wait_for(lambda:enabled('Open'),label='legacy native chooser Open');assert access.click(find('Open',['button','push button']));native.wait_for(lambda:find('بازیابی و ذخیرهٔ نسخهٔ ایمنی',['button','push button']),label='legacy valid original M3 restore preview');shot('legacy-prefix-restore-preview.png');press('بازیابی و ذخیرهٔ نسخهٔ ایمنی');native.wait_for(lambda:state(legacy)==legacy_expected,seconds=30,label='native archivedM3 prefix history exact restored');check('actual chooser restores archivedM3 manual prefix history exactly',state(legacy)==legacy_expected)
  x11.key('7','Alt_L');time.sleep(.4);press('اسناد',['page tab']);time.sleep(.3);ctrl_search('جست‌وجوی اسناد حسابداری','legacy ledger');fill('جست‌وجوی اسناد حسابداری',legacy_expected['accounting']['journals'][0]['id']);native.wait_for(lambda:any(r['name']=='سند معکوس با دلیل' and r['role'] in ['button','push button'] and mapped_sensitive(r) for r in access.tree()),label='legacy ordinary reversal available')
  check('legacy arbitrary prefix is ordinary manual journal without fake procurement source',not any(r['name']=='منبع خرید و اثر موجودی' and r['role'] in ['button','push button'] and mapped_sensitive(r) for r in access.tree()) and state(legacy)==legacy_expected);shot('legacy-prefix-ordinary-ledger.png');press('سند معکوس با دلیل');native.wait_for(lambda:find('دلیل ضروری',['text','entry']),label='legacy ordinary reversal dialog labelled-by reason field');shot('legacy-prefix-ordinary-reversal.png');focus('انصراف');x11.key('Escape');time.sleep(.3);check('legacy ordinary reversal dialog cancels without history mutation',state(legacy)==legacy_expected)
  native.stop(app);critical_logs={p.name:[line for line in p.read_text(errors='replace').splitlines() if '-CRITICAL' in line] for p in run.glob('*.log')};check('native process logs contain no GTK criticals',not any(critical_logs.values()),logs=critical_logs)
  check('pinned executable unchanged',hashlib.sha256(binary.read_bytes()).hexdigest()==digest);check('X11 errors only retired-window races',all(e['code']==3 for e in report['x11_errors']));report['result']='passed'
 except Exception as e:
  report['error']=str(e)
  if 'access' in locals():
   (run/'failure-tree.json').write_text(json.dumps([{k:v for k,v in r.items() if k!='node'} for r in access.tree()],ensure_ascii=False,indent=2))
   try:shot('failure.png')
   except Exception:pass
  raise
 finally:
  if x11:x11.x.XCloseDisplay(x11.d)
  for p in reversed(processes):native.stop(p)
  for f in logs:f.close()
  (run/'independent-native.json').write_text(json.dumps(report,ensure_ascii=False,indent=2));print(json.dumps(report,ensure_ascii=False,indent=2))
if __name__=='__main__':main()
