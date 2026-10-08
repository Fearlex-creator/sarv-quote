import os,subprocess,tempfile,time,sys
from pathlib import Path
out=Path(sys.argv[1]); assert not out.exists(); out.parent.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryFile() as fd, out.open('w') as log:
 x=subprocess.Popen(['Xvfb','-displayfd',str(fd.fileno()),'-screen','0','1280x900x24','-nolisten','tcp','-xkbdir','/workspace/sarv-environment/sysroot/usr/share/X11/xkb'],pass_fds=(fd.fileno(),),stdout=log,stderr=log)
 try:
  deadline=time.monotonic()+10;d=b''
  while time.monotonic()<deadline:
   fd.seek(0);d=fd.read().strip()
   if d:break
   if x.poll() is not None:raise RuntimeError('Private Xvfb failed')
   time.sleep(.02)
  assert d,'Private Xvfb timeout'
  env=os.environ.copy();env.update(DISPLAY=':'+d.decode(),GDK_BACKEND='x11',GSK_RENDERER='cairo',G_DEBUG='fatal-criticals')
  subprocess.run(['dbus-run-session','--','cargo','test','--offline','--locked','--test','qa_invariants','native_widget_lifecycle_does_not_synthesize_new_or_empty_edit_actions','--','--ignored','--exact'],env=env,stdout=log,stderr=log,check=True)
 finally:
  x.terminate();x.wait(timeout=5)
