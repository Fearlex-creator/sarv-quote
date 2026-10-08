#!/usr/bin/env python3
import os,json,tempfile,subprocess,sys,time,hashlib
from pathlib import Path
import importlib.util
sys.path.insert(0,'/workspace/sarv-quote/scripts')
from qa_access import Access
spec=importlib.util.spec_from_file_location('native','/workspace/sarv-quote/scripts/qa-native.py');native=importlib.util.module_from_spec(spec);spec.loader.exec_module(native)
root=Path('/workspace/sarv-quote');binary=Path('/workspace/sarv-evidence/bin/sarv-M3-final');assert hashlib.sha256(binary.read_bytes()).hexdigest()=='10bf9929d00cdf98280a51a2c12860ec39939cdbe8620eb2d827b2bb1de6cee1'
run=Path(tempfile.mkdtemp(prefix='dark-modal-',dir=root/'artifacts/independent-m3/native'));processes=[];handles=[];x11=None
try:
 def start(cmd,name,env=None,**kw):
  f=(run/f'{name}.log').open('w');handles.append(f);p=subprocess.Popen(cmd,env=env,stdout=f,stderr=f,start_new_session=True,**kw);processes.append(p);return p
 with tempfile.TemporaryFile() as fd:
  start(['Xvfb','-displayfd',str(fd.fileno()),'-screen','0','1450x1050x24','-nolisten','tcp','-xkbdir',str(native.ENVROOT/'sysroot/usr/share/X11/xkb')],'xvfb',pass_fds=(fd.fileno(),))
  def d():fd.seek(0);return fd.read().decode().strip()
  display=':'+native.wait_for(d,label='owned display')
 runtime=native.private_accessibility_environment(display,run);env=os.environ.copy();env.update(DISPLAY=display,GDK_BACKEND='x11',GSK_RENDERER='cairo',XDG_RUNTIME_DIR=str(runtime),SARV_TEST_INSTANCE='independent-dark-modal',SARV_DATA_DIR=str(run/'profile'),SARV_CORE_SMOKE='stage',ADW_DEBUG_COLOR_SCHEME='prefer-dark')
 for k in ['SARV_UI_SMOKE','SARV_ACCOUNTING_SMOKE']:env.pop(k,None)
 start(['openbox','--sm-disable','--config-file',str(native.ENVROOT/'sysroot/etc/xdg/openbox/rc.xml')],'openbox',env);app=start([str(binary)],'app',env);x11=native.X11(display);window=native.wait_for(lambda:x11.find(app.pid));native.wait_for(lambda:(run/'profile/core-stage-ready.json').exists(),seconds=100);time.sleep(.4);access=Access(app.pid);x11.focus(window)
 def capture(name,width,height):
  x11.resize(window,width,height);native.wait_for(lambda:x11.geometry(window)==[width,height]);time.sleep(.25);subprocess.run(['import','-display',display,'-window',str(window),str(run/name)],check=True)
 for w,h in [(1280,900),(650,760)]:capture(f'details-dark-{w}.png',w,h)
 row=access.find('بازبینی',['page tab']);assert row and access.click(row);time.sleep(.3)
 for w,h in [(1280,900),(650,760)]:capture(f'preview-dark-{w}.png',w,h)
 (run/'modal-check.json').write_text(json.dumps({'result':'captured','binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'theme':'dark','widths':[1280,650],'views':['details','reconciled preview'],'source':'actual staged native forms, no opening committed','limits':'visual inspection required'},indent=2));print(run)
finally:
 if x11:x11.x.XCloseDisplay(x11.d)
 for p in reversed(processes):native.stop(p)
 for f in handles:f.close()
