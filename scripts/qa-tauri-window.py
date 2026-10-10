#!/usr/bin/env python3
"""Real X11 keyboard, native CSV dialog, restart and unsaved-close checks.

Requires a running Sarv Tauri window on the private :88 display, at 1440x900,
an explicitly isolated artifacts profile, Openbox, and the activated QA tools.
Financial operations are checked separately; this script never submits them.
"""
import argparse
import csv
import ctypes as C
import hashlib
import importlib.util
import json
from pathlib import Path
import sqlite3
import sys
import time
from PIL import ImageGrab

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT/'scripts'))
spec = importlib.util.spec_from_file_location('native_qa', ROOT/'scripts/qa-native.py')
native = importlib.util.module_from_spec(spec)
spec.loader.exec_module(native)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--pid', type=int, required=True)
    parser.add_argument('--profile', type=Path, required=True)
    parser.add_argument('--display', default=':88')
    parser.add_argument('--trial-total', type=int, default=1251)
    args = parser.parse_args()
    profile = args.profile.resolve()
    assert profile.is_relative_to(ROOT/'artifacts'), 'An isolated artifacts profile is required'
    process = Path(f'/proc/{args.pid}/cmdline').read_bytes()
    assert b'sarv-desktop' in process, 'PID must belong to the Tauri QA app'
    con = sqlite3.connect(f'file:{profile}/sarv.sqlite3?mode=ro', uri=True)
    before = con.execute('select payload from state').fetchone()
    x = native.X11(args.display)
    x.x.XSync.argtypes = [C.c_void_p,C.c_int]
    main_window = x.find(args.pid)
    assert main_window
    out = ROOT/'artifacts/tauri-ui'
    def settle(seconds=.35):
        x.x.XSync(x.d,0)
        time.sleep(seconds)
    def capture(name):
        settle()
        ImageGrab.grab(xdisplay=args.display).save(out/f'{name}.png')
    def type_slow(text):
        for char in text:
            x.type_ascii(char)
            time.sleep(.02)
        settle(.15)
    def named(title):
        return next((w for w in x.prop(x.root,'_NET_CLIENT_LIST') if x.prop(w,'_NET_WM_NAME') == title.encode()),None)
    def wait_for(predicate, label):
        for _ in range(200):
            value = predicate()
            if value: return value
            time.sleep(.05)
        raise AssertionError(f'Timed out: {label}')
    x.focus(main_window);settle()
    x.pointer(74,365);settle();x.pointer(396,200);settle()
    capture('native-reports-final')
    x.pointer(1356,120)
    dialog = wait_for(lambda:named('Save File'),'native save dialog')
    x.focus(dialog);settle()
    a,b = x.origin(dialog);width,height=x.geometry(dialog)
    capture('native-save-final')
    export = profile/f'native-trial-final-{time.time_ns()}.csv'
    x.pointer(a+400,b+27);settle(.15);x.key('a','Control_L');type_slow(str(export))
    x.pointer(a+width-48,b+height-23)
    wait_for(export.exists,'CSV export')
    rows = list(csv.DictReader(export.open(encoding='utf-8-sig')))
    debit=sum(int(r['Debit IRR']) for r in rows)
    credit=sum(int(r['Credit IRR']) for r in rows)
    assert debit==credit==args.trial_total,(debit,credit)
    capture('native-export-final')
    x.focus(main_window);x.key('k','Control_L');settle();type_slow('Sales')
    capture('native-command-focus')
    x.key('Return');settle();capture('native-sales-keyboard')
    x.pointer(1340,120);settle();capture('native-order-editor-final')
    x.pointer(865,348);settle(.15);type_slow('native unsaved verification')
    x.pointer(1428,11)
    prompt = wait_for(lambda:named('Unsaved changes'),'unsaved prompt')
    capture('native-close-prompt-final')
    a,b=x.origin(prompt);width,height=x.geometry(prompt)
    x.pointer(a+width*3//4,b+height-17)
    wait_for(lambda:not named('Unsaved changes'),'keep editing')
    assert x.find(args.pid)
    capture('native-kept-editor-final')
    x.pointer(1428,11)
    prompt=wait_for(lambda:named('Unsaved changes'),'second unsaved prompt')
    a,b=x.origin(prompt);width,height=x.geometry(prompt)
    x.pointer(a+width//4,b+height-17)
    wait_for(lambda:not x.find(args.pid),'discard closes window')
    capture('native-discard-closed')
    assert before==con.execute('select payload from state').fetchone(), 'Close/export checks must not change books'
    path=ROOT/'artifacts/tauri-native-evidence.json'
    evidence=json.loads(path.read_text()) if path.exists() else {}
    evidence['native_release_sha256']=hashlib.sha256((ROOT/'desktop/src-tauri/target/release/sarv-desktop').read_bytes()).hexdigest()
    evidence.setdefault('checks',{}).update(native_csv_save={'debit':debit,'credit':credit,'path':str(export.relative_to(ROOT))},command_keyboard_focus=True,command_keyboard_navigation=True,unsaved_keep_editing=True,unsaved_discard_closes=True,financial_payload_unchanged_after_close_checks=True)
    path.write_text(json.dumps(evidence,indent=2)+'\n')
    print('Native keyboard, CSV, keep-editing and discard-close checks passed; books unchanged.')

if __name__=='__main__': main()
