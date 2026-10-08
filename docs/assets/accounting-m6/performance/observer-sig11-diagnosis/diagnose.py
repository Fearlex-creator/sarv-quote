#!/usr/bin/env python3
"""Scoped owned native faulthandler diagnostic; guarded outside every child."""
import argparse
import faulthandler
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time

ROOT=Path(__file__).resolve().parents[5]
SELF=Path(__file__).resolve()
RECEIPT=ROOT/'docs/assets/accounting-m6/performance/candidate8-runtime2-coordinator-receipt.json'
RECEIPT_SHA='98ffd3f333f439ee4beb70ba82d28ae425848eef0e325cfb0cdd7acb4a88dd03'
OBS_SHA='cb145aed5a13d289f8222083f3d23e67ef9c95759745c2d4aea9ecc391945408'
sys.path.insert(0,str(ROOT/'scripts'))
def module(name,path):
 s=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(s);s.loader.exec_module(m);return m
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def save(p,v):p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n')
def inspection(record):
 p=Path(f"/proc/{record['pid']}")
 if not (p/'stat').exists():return None
 s=(p/'stat').read_text();v=s[s.rfind(')')+2:].split()
 return {'state':v[0],'comm':s[s.find('(')+1:s.rfind(')')],'pgid':int(v[2]),'starttime_ticks':int(v[19]),'command':[x.decode() for x in (p/'cmdline').read_bytes().split(b'\0') if x]}
def cleanup(records):
 audit=[]
 for record in reversed(records):
  current=inspection(record);row={'record':record,'before':current,'action':'already inactive'}
  if current and current['state']!='Z':
   assert current['pgid']==record['pgid'] and current['starttime_ticks']==record['starttime_ticks'] and current['command']==record['command']
   os.kill(record['pid'],signal.SIGTERM);row['action']='SIGTERM exact owned identity'
  audit.append(row)
 for _ in range(30):
  if all((p:=inspection(r)) is None or p['state']=='Z' for r in records):break
  time.sleep(.1)
 for row in audit:
  row['after']=inspection(row['record']);assert row['after'] is None or row['after']['state']=='Z';row['owned_identity_active']=False
 return audit

def child(args):
 faulthandler.enable(all_threads=True)
 held=module('held_m6_diagnostic',ROOT/'scripts/native-performance-M6.py')
 receipt=json.loads(RECEIPT.read_text());dataset=json.loads((ROOT/receipt['dataset_manifest']['path']).read_text())
 args.binary=Path(receipt['binary']);args.search_samples=2;args.samples=1;args.pilot=True
 report={'result':'incomplete','diagnostic_only':True,'timed_budget_claim':False,'binary':str(args.binary),'startup':{},'source_picker_search':{},'return_workflows':{},'capture_samples':[],'owned_process_records':[],'x11_error_observations':[],'rejected_source_observations':[],'completed_checkpoint_boundaries':[]}
 class Probe(held.Observer):
  def start(self,*a,**kw):
   process=super().start(*a,**kw)
   with (args.output/'owned-processes.jsonl').open('a') as stream:
    stream.write(json.dumps(self.report['owned_process_records'][-1])+'\n');stream.flush();os.fsync(stream.fileno())
   return process
  def source_picker(self,access):
   print('ENTER source_picker ownedPID='+str(access.pid),flush=True);v=super().source_picker(access);print('DONE source_picker',flush=True);return v
  def open_source(self,*a):
   print('ENTER open_source',flush=True);v=super().open_source(*a);print('DONE open_source',flush=True);return v
  def qualify_editor(self,*a):
   print('ENTER qualify_editor',flush=True);v=super().qualify_editor(*a);print('DONE qualify_editor',flush=True);return v
 observer=Probe(args,report,dataset);profiles=[]
 try:
  observer.setup()
  if args.prelude=='two-sources':
   observer.searches('source120-billed');observer.searches('source120-unbilled')
  profile,recipe=observer.copied('return20-billed','return20-opening');profiles.append(profile);baseline=held.payload(profile)
  app,window,access=observer.app(profile,'return20-opening',False);observer.purchases(access)
  q,editor,elapsed=observer.open_source(access,recipe['source_reference'],recipe['receipt_line_ids'][0])
  observer.qualify_editor(access,editor,recipe['receipt_line_ids'])
  report['quantity20_current_empty']=bool(observer.page_state(access,editor,recipe['receipt_line_ids'],1))
  assert report['quantity20_current_empty'] and held.payload(profile)==baseline
  observer.capture(window,args.output/'opened20-editor.png')
  report['result']='qualified_scoped20_opening_no_input_or_post'
  observer.checkpoint('diagnostic-completed20-opening')
 except BaseException as error:
  report['error']=repr(error);raise
 finally:
  observer.close();save(args.output/'diagnostic-child.json',report)

def main():
 parser=argparse.ArgumentParser();parser.add_argument('--output',type=Path,required=True);parser.add_argument('--prelude',choices=['none','two-sources'],default='none');parser.add_argument('--child',action='store_true');args=parser.parse_args();args.output=args.output.resolve()
 if args.child:return child(args)
 assert args.output.is_relative_to(ROOT/'artifacts/performance-m6') and not args.output.exists();args.output.mkdir(parents=True)
 coord=module('guard_diagnostic',ROOT/'docs/assets/accounting-m6/performance/coordinator.py');receipt=json.loads(RECEIPT.read_text());ownsha=sha(SELF);assert sha(ROOT/'scripts/native-performance-M6.py')==OBS_SHA
 report={'result':'incomplete','diagnostic_only':True,'timed_budget_claim':False,'sidecar_sha256':ownsha,'held_observer_sha256':OBS_SHA,'prelude':args.prelude,'guard_before':coord.guard(receipt,RECEIPT,RECEIPT_SHA),'start_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime())}
 command=['dbus-run-session','--','/usr/bin/python3','-X','faulthandler',str(SELF),'--child','--prelude',args.prelude,'--output',str(args.output)];report['command']=command
 try:
  with (args.output/'native-faulthandler.log').open('x') as stream:
   process=subprocess.Popen(command,cwd=ROOT,stdout=stream,stderr=subprocess.STDOUT,start_new_session=True);report['child_wrapper_pid']=process.pid;save(args.output/'diagnostic-guardian.json',report);report['exit_code']=process.wait()
  report['result']='completed_scoped_diagnostic' if report['exit_code']==0 else 'native_child_failed'
 finally:
  ledger=args.output/'owned-processes.jsonl';records=[json.loads(line) for line in ledger.read_text().splitlines()] if ledger.exists() else []
  report['owned_process_cleanup']=cleanup(records);report['all_journaled_owned_processes_inactive']=True
  report['guard_after']=coord.guard(receipt,RECEIPT,RECEIPT_SHA);report['all_guards_stable']=report['guard_after']==report['guard_before'] and sha(SELF)==ownsha and sha(ROOT/'scripts/native-performance-M6.py')==OBS_SHA
  assert report['all_guards_stable'];report['end_utc']=time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime());save(args.output/'diagnostic-guardian.json',report)
 print(json.dumps({k:report[k] for k in ('result','exit_code','all_journaled_owned_processes_inactive','all_guards_stable')}))
if __name__=='__main__':main()
