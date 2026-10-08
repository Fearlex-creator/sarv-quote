#!/usr/bin/env python3
"""Guarded native current-ID proof for persistent Sales cards and line actions."""
import copy,hashlib,importlib.util,json,os,sqlite3,subprocess,sys,tempfile,time
from pathlib import Path
ROOT=Path('/workspace/sarv-quote');sys.path.insert(0,str(ROOT/'scripts'))
from qa_access import Access
spec=importlib.util.spec_from_file_location('native',ROOT/'scripts/qa-native.py');native=importlib.util.module_from_spec(spec);spec.loader.exec_module(native)
manifest=json.loads(Path(sys.argv[1]).read_text());binary=Path(manifest['binary'])
run=Path(tempfile.mkdtemp(prefix='slot-reuse-',dir=ROOT/'artifacts/sales-native'));checks=[];processes=[];logs=[];x=None
observer=Path(__file__);observer_before=hashlib.sha256(observer.read_bytes()).hexdigest()
verification_paths=[observer,ROOT/'scripts/qa_access.py',ROOT/'scripts/qa-native.py',ROOT/'artifacts/sales-native/run-t_qx8es0/detail-pages/synthetic-raw-fixture.json']
verification_before={str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in verification_paths}
def provenance():
 h=hashlib.sha256();files={}
 for name in sorted(manifest['frozen_source_provenance']['files'],key=lambda n:n.encode()):
  raw=(ROOT/name).read_bytes();h.update(name.encode()+b'\0'+raw+b'\0');files[name]=hashlib.sha256(raw).hexdigest()
 return {'sha256':h.hexdigest(),'files':files}
def payload(profile):
 with sqlite3.connect(f'file:{profile}/sarv.sqlite3?mode=ro',uri=True) as db:return db.execute('select generation,payload from state where id=1').fetchone()
def state(profile):return json.loads(payload(profile)[1])
def check(name,passed,**data):
 receipt={'name':name,'passed':bool(passed),**data};checks.append(receipt)
 with (run/'progress.jsonl').open('a') as out:out.write(json.dumps(receipt,ensure_ascii=False)+'\n')
 assert passed,name
before=provenance();assert before=={k:manifest['frozen_source_provenance'][k] for k in ['sha256','files']};assert hashlib.sha256(binary.read_bytes()).hexdigest()==manifest['binary_sha256']
source=ROOT/'artifacts/performance-m5/candidate2-functional-pilot/sales-paged-search';source_before=payload(source)
list_profile=run/'list';list_profile.mkdir()
with sqlite3.connect(f'file:{source}/sarv.sqlite3?mode=ro',uri=True) as src,sqlite3.connect(list_profile/'sarv.sqlite3') as dst:src.backup(dst)
editor_source=ROOT/'artifacts/sales-native/run-t_qx8es0/draft';editor_before=payload(editor_source)
editor_profile=run/'editor';editor_profile.mkdir()
with sqlite3.connect(f'file:{editor_source}/sarv.sqlite3?mode=ro',uri=True) as src,sqlite3.connect(editor_profile/'sarv.sqlite3') as dst:src.backup(dst)
seeded=state(editor_profile);books=copy.deepcopy(seeded['accounting'])
draft=json.loads((ROOT/'artifacts/sales-native/run-t_qx8es0/detail-pages/synthetic-raw-fixture.json').read_text())['draft'];draft=copy.deepcopy(draft)
draft.update(id='m5-reuse-identity-draft',reference='KBD-REUSE-30',revision=1,supplied_total='9299',reason='synthetic distinct quantities for current-line proof')
for i,line in enumerate(draft['lines']):line.update(id=f'reuse-line-{i:03d}',quantity=str(i+1))
seeded['sales_drafts']=[draft]
with sqlite3.connect(editor_profile/'sarv.sqlite3') as db:
 generation=db.execute('select generation from state where id=1').fetchone()[0]
 assert db.execute('update state set generation=?,payload=? where id=1 and generation=?',(generation+1,json.dumps(seeded,ensure_ascii=False,separators=(',',':')),generation)).rowcount==1
(editor_profile/'synthetic-raw-fixture.json').write_text(json.dumps({'synthetic_only':True,'raw_only':True,'accounting_unchanged':True,'draft':draft},ensure_ascii=False,indent=2))
result={'result':'failed','run':str(run),'binary_sha256':manifest['binary_sha256'],'observer_sha256':observer_before,'verification_before':verification_before,'synthetic_sources':{str(source):{'generation':source_before[0],'payload_sha256':hashlib.sha256(source_before[1].encode()).hexdigest()},str(editor_source):{'generation':editor_before[0],'payload_sha256':hashlib.sha256(editor_before[1].encode()).hexdigest()}},'source_before':before,'checks':checks}
try:
 def start(cmd,name,env=None,**kw):
  log=(run/f'{name}.log').open('w');logs.append(log);p=subprocess.Popen(cmd,env=env,stdout=log,stderr=log,start_new_session=True,**kw);processes.append(p);return p
 with tempfile.TemporaryFile() as fd:
  start(['Xvfb','-displayfd',str(fd.fileno()),'-screen','0','1450x1050x24','-nolisten','tcp','-xkbdir',str(native.ENVROOT/'sysroot/usr/share/X11/xkb')],'xvfb',pass_fds=(fd.fileno(),))
  def display_number():fd.seek(0);return fd.read().decode().strip()
  display=':'+native.wait_for(display_number)
 runtime=native.private_accessibility_environment(display,run);env=os.environ.copy();env.update(DISPLAY=display,GDK_BACKEND='x11',GSK_RENDERER='cairo',XDG_RUNTIME_DIR=str(runtime),SARV_TEST_INSTANCE='m5-slot-reuse-native',SARV_UI_SMOKE='capture',SARV_CAPTURE_PAGE='sales',SARV_CAPTURE_NAME='ready.png',ADW_DEBUG_COLOR_SCHEME='prefer-light')
 for key in ['SARV_SALES_SMOKE','SARV_CORE_SMOKE','SARV_PROCUREMENT_SMOKE','SARV_ACCOUNTING_SMOKE']:env.pop(key,None)
 start(['openbox','--sm-disable','--config-file',str(native.ENVROOT/'sysroot/etc/xdg/openbox/rc.xml')],'openbox',env)
 x=native.X11(display)
 def flag(row,n):
  access.a.atspi_accessible_clear_cache(row['node']);states=access.a.atspi_accessible_get_state_set(row['node']);return bool(states and access.a.atspi_state_set_contains(states,n))
 def focus(caption,roles=('button','push button'),index=0):
  for _ in range(180):
   candidates=[r for r in access.tree() if r['name']==caption and r['role'] in roles and flag(r,24)]
   if len(candidates)>index and access.focused(candidates[index]):return candidates[index]
   x.key('Tab');time.sleep(.015)
  raise AssertionError('exact real Tab focus '+caption+' index '+str(index))
 def press(caption,index=0):focus(caption,index=index);x.key('Return');time.sleep(.15)
 def fill(caption,value):focus(caption,('text','entry'));x.key('a','Control_L');x.type_ascii(value);native.wait_for(lambda:access.text(access.find(caption,['text','entry']))==value,label='actual text '+caption)
 def shot(name):subprocess.run(['/usr/bin/import','-display',display,'-window',str(window),str(run/name)],check=True)
 def launch(profile,name):
  env['SARV_DATA_DIR']=str(profile);p=start([str(binary)],name,env);w=native.wait_for(lambda:x.find(p.pid),label=name+' window');native.wait_for(lambda:(profile/'ready.png').exists(),label=name+' ready');x.focus(w);return p,w,Access(p.pid)
 app,window,access=launch(list_profile,'list')
 initial=payload(list_profile);check('120 synthetic raw-draft list retained',len(state(list_profile)['sales_drafts'])==120)
 for group,reference in [('A','PERF-A-000'),('B','PERF-B-001'),('A','PERF-A-000')]:
  x.key('f','Control_L');x.key('a','Control_L');x.type_ascii('PERF-'+group)
  native.wait_for(lambda:access.text(access.find('جست‌وجوی شماره، مشتری، کالا یا پیش‌نویس',['text','entry']))=='PERF-'+group and any(r['name']=='پیش‌نویس • '+reference and flag(r,25) for r in access.tree()),label='current '+reference)
  press('ادامهٔ پیش‌نویس');native.wait_for(lambda:access.find('شماره یا مرجع سفارش',['text','entry']),label='opened current draft')
  check('reused first card opens current '+reference,access.text(access.find('شماره یا مرجع سفارش',['text','entry']))==reference and payload(list_profile)==initial)
  x.key('Escape');native.wait_for(lambda:not access.find('سفارش فروش • پیش‌نویس و بازبینی',['dialog']),label='readonly current draft close')
 x.key('f','Control_L');x.key('a','Control_L');x.type_ascii('NO-MATCH-SLOT-REUSE')
 native.wait_for(lambda:any('۰ نتیجه' in r['name'] and flag(r,25) for r in access.tree()),label='zero current rows')
 opens=[r for r in access.tree() if r['name'] in ('ادامهٔ پیش‌نویس','جزئیات و جریان کار') and r['role']=='button']
 check('zero-result persistent cards are absent or all insensitive and not showing',all(not flag(r,24) and not flag(r,25) for r in opens),accessible_action_count=len(opens))
 check('filter activation/cancellation preserves source and copied payload',payload(list_profile)==initial and payload(source)==source_before)
 native.stop(app)
 app,window,access=launch(editor_profile,'editor');press('ادامهٔ پیش‌نویس');press('ردیف‌های بعد')
 native.wait_for(lambda:any(r['name']=='۳۰ ردیف • صفحه ۲' for r in access.tree()),label='30-line current page2')
 press('ویرایش ردیف');qty='مقدار در واحد مشخص‌شده (تا سه رقم اعشار)'
 check('reused page2 Edit resolves immutable line025 quantity26',access.text(access.find(qty,['text','entry']))=='26')
 fill(qty,'26.125');press('اعمال ردیف در پیش‌نویس');native.wait_for(lambda:not access.find('ردیف فروش • کالا و قیمت',['dialog']),label='line applied')
 def save(label):
  previous=state(editor_profile)['sales_drafts'][0]['revision'];x.key('s','Control_L');native.wait_for(lambda:state(editor_profile)['sales_drafts'][0]['revision']==previous+1,label=label);current=state(editor_profile);check(label,current['accounting']==books);return current['sales_drafts'][0]['lines']
 lines=save('real CtrlS accepts distinct-quantity raw fixture through Store');check('exact current ID edited with all others retained',lines[25]['id']=='reuse-line-025' and lines[25]['quantity']=='26.125' and lines[:25]==draft['lines'][:25] and lines[26:]==draft['lines'][26:])
 press('حذف ردیف');lines=save('delete current page2 row saves without books');check('current line025 deleted and next line026 retained',len(lines)==29 and all(l['id']!='reuse-line-025' for l in lines) and lines[25]['id']=='reuse-line-026')
 press('بالا');lines=save('move current reused row across page boundary');check('current immutable line026 moved above024 across page boundary',lines[24]['id']=='reuse-line-026' and lines[25]['id']=='reuse-line-024')
 press('ویرایش ردیف');check('reused slot after reorder opens current line024 quantity25',access.text(access.find(qty,['text','entry']))=='25');x.key('Escape');time.sleep(.15)
 press('حذف ردیف');lines=save('delete reordered current ID without books');check('reordered line024 deleted, current line027 remains',len(lines)==28 and all(l['id']!='reuse-line-024' for l in lines) and lines[25]['id']=='reuse-line-027')
 press('ردیف‌های قبل');press('پایین',24);lines=save('move last first-page row down across boundary');check('line026 moved down with current line027 moved up',lines[24]['id']=='reuse-line-027' and lines[25]['id']=='reuse-line-026')
 press('ردیف‌های بعد');press('ویرایش ردیف');check('cross-page Down then Edit resolves current line026 quantity27',access.text(access.find(qty,['text','entry']))=='27');x.key('Escape');time.sleep(.15)
 for remaining in [27,26,25]:
  press('حذف ردیف');lines=save('delete tail raw row to count'+str(remaining));check('tail count current '+str(remaining),len(lines)==remaining)
 native.wait_for(lambda:any(r['name']=='۲۵ ردیف • صفحه ۱' for r in access.tree()),label='durably clamped page1 after tail deleted')
 check('deletion clamps persistent page and disables Next',not flag(access.find('ردیف‌های بعد',['button']),24))
 press('ویرایش ردیف');check('clamped reused first Edit resolves line000 quantity1',access.text(access.find(qty,['text','entry']))=='1');x.key('Escape');time.sleep(.15)
 press('بازبینی سفارش');native.wait_for(lambda:any('جمع' in r['name'] and ('برابر' in r['name'] or 'تطبیق' in r['name']) and flag(r,25) for r in access.tree()),label='current deleted/reordered total rejected')
 check('review uses current rows and rejects stale independently supplied total',not flag(access.find('ثبت سفارش بازبینی‌شده',['button']),24) and state(editor_profile)['accounting']==books)
 shot('M5-reused-lines-current-review-error.png');native.stop(app)
 check('immutable synthetic sources unchanged',payload(source)==source_before and payload(editor_source)==editor_before)
 after=provenance();check('frozen production source exact before/after',after==before);check('frozen binary and observer unchanged',hashlib.sha256(binary.read_bytes()).hexdigest()==manifest['binary_sha256'] and hashlib.sha256(observer.read_bytes()).hexdigest()==observer_before)
 verification_after={str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in verification_paths};check('observer helpers and synthetic fixture file unchanged',verification_after==verification_before);result['verification_after']=verification_after
 result.update(result='passed',source_after=after)
except Exception as error:
 result['error']=str(error)
 if 'access' in locals():
  (run/'failed-tree.json').write_text(json.dumps([{k:v for k,v in r.items() if k!='node'} for r in access.tree()],ensure_ascii=False,indent=2))
 if x and 'window' in locals():shot('failed-native-window.png')
 raise
finally:
 (run/'slot-reuse-native.json').write_text(json.dumps(result,ensure_ascii=False,indent=2));print(json.dumps({k:v for k,v in result.items() if k not in ['source_before','source_after']},ensure_ascii=False,indent=2))
 if x:x.x.XCloseDisplay(x.d)
 for p in reversed(processes):
  if p.poll() is None:
   p.terminate()
   try:p.wait(timeout=3)
   except subprocess.TimeoutExpired:p.kill()
 for log in logs:log.close()
