#!/usr/bin/env python3
"""Paired native copy screenshots; read-only synthetic source, private app/profile.
Run under dbus-run-session after activating the cloud GTK environment.
"""
import argparse
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

spec = importlib.util.spec_from_file_location('native', Path(__file__).with_name('qa-native.py'))
native = importlib.util.module_from_spec(spec)
spec.loader.exec_module(native)

def state(profile):
    with sqlite3.connect(f'file:{profile}/sarv.sqlite3?mode=ro', uri=True) as db:
        return db.execute('SELECT payload FROM state WHERE id=1').fetchone()[0]

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--source', type=Path, required=True)
    parser.add_argument('--label', required=True, choices=['before','after'])
    args = parser.parse_args()
    source = args.source.resolve()
    assert source.is_relative_to(native.ROOT/'artifacts'), 'Synthetic artifacts only'
    output = native.ROOT/'artifacts/ui-copy'
    output.mkdir(exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix=args.label+'-',dir=output))
    with sqlite3.connect(f'file:{source}/sarv.sqlite3?mode=ro',uri=True) as src, sqlite3.connect(run/'sarv.sqlite3') as dst:
        src.backup(dst)
    before = state(run)
    report = {'result':'failed','source':str(source),'profile':str(run), 'binary':str(args.binary), 'binary_sha256':hashlib.sha256(args.binary.read_bytes()).hexdigest(),'checks':[], 'synthetic_only':True}
    processes, logs = [], []
    x = None
    def start(command,name,env=None,**kwargs):
        log=(run/(name+'.log')).open('w');logs.append(log)
        p=subprocess.Popen(command,env=env,stdout=log,stderr=log,start_new_session=True,**kwargs);processes.append(p);return p
    try:
        with tempfile.TemporaryFile() as fd:
            start(['Xvfb','-displayfd',str(fd.fileno()),'-screen','0','1500x1100x24','-nolisten','tcp','-xkbdir',str(native.ENVROOT/'sysroot/usr/share/X11/xkb')],'xvfb',pass_fds=(fd.fileno(),))
            def display_name():
                fd.seek(0);return fd.read().decode().strip()
            display=':'+native.wait_for(display_name)
        runtime=native.private_accessibility_environment(display,run)
        env=os.environ.copy()
        env.update(DISPLAY=display,GDK_BACKEND='x11',GSK_RENDERER='cairo',XDG_RUNTIME_DIR=str(runtime),SARV_DATA_DIR=str(run),SARV_TEST_INSTANCE='copy-snapshots',SARV_UI_SMOKE='capture',SARV_CAPTURE_PAGE='sales',SARV_CAPTURE_NAME='initial.png')
        for key in ['SARV_ACCOUNTING_SMOKE','SARV_CORE_SMOKE','SARV_SALES_SMOKE','SARV_PROCUREMENT_SMOKE','SARV_RESPONSIVENESS_SMOKE']:
            env.pop(key,None)
        start(['openbox','--sm-disable','--config-file',str(native.ENVROOT/'sysroot/etc/xdg/openbox/rc.xml')],'openbox',env)
        app=start([str(args.binary)],'app',env)
        x=native.X11(display)
        window=native.wait_for(lambda:x.find(app.pid))
        native.wait_for(lambda:(run/'initial.png').exists())
        x.focus(window);x.resize(window,1280,900);time.sleep(.4)
        access=Access(app.pid)
        for key,page in [('0','sales'),('8','procurement'),('9','inventory'),('7','accounting'),('1','order'),('6','settings')]:
            x.key(key,'Alt_L');time.sleep(.4)
            native.wait_for(lambda:native.events(run)[-1]['page']==page)
            subprocess.run(['/usr/bin/import','-display',display,'-window',str(window),str(run/(page+'.png'))],check=True)
            (run/(page+'.a11y.json')).write_text(json.dumps([{k:v for k,v in r.items() if k!='node'} for r in access.tree()],ensure_ascii=False,indent=2)+'\n')
            assert state(run)==before,'Navigation must preserve byte-exact state'
            report['checks'].append({'name':page+' native render and byte-exact saved state','passed':True})
        assert state(source)==before,'Source fixture remains unchanged'
        report['result']='passed'
    except Exception as error:
        report['error']=repr(error)
        raise
    finally:
        if x:x.x.XCloseDisplay(x.d)
        for p in reversed(processes):native.stop(p)
        for log in logs:log.close()
        (run/'copy-snapshots.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
        print(json.dumps({'result':report['result'],'profile':str(run),'checks':len(report['checks'])}))

if __name__=='__main__':main()
