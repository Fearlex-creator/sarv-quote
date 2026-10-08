#!/usr/bin/env python3
"""Inspect only a supplied owned synthetic Sarv PID through installed AT-SPI."""
import argparse,json,pathlib,gi

gi.require_version('Atspi','2.0')
from gi.repository import Atspi
p=argparse.ArgumentParser();p.add_argument('--pid',type=int,required=True);p.add_argument('--out',type=pathlib.Path,required=True);args=p.parse_args()
Atspi.set_timeout(500,1000)
d=Atspi.get_desktop(0)
app=next((a for i in range(d.get_child_count()) if (a:=d.get_child_at_index(i)).get_process_id()==args.pid),None)
if app is None:raise SystemExit('Owned PID not exposed through AT-SPI; no other tree inspected')
queue=[app];rows=[];seen=0
while queue and seen<4000:
 a=queue.pop(0);seen+=1;role=a.get_role_name();name=a.get_name()
 if role in ['text','entry','combo box','button','push button','check box','spin button']:
  labels=[]
  for r in a.get_relation_set():
   if r.get_relation_type()==Atspi.RelationType.LABELLED_BY:
    labels.extend(r.get_target(j).get_name() for j in range(r.get_n_targets()))
  rows.append({'role':role,'name':name,'labelled_by':labels})
 queue.extend(a.get_child_at_index(i) for i in range(a.get_child_count()))
names={r['name'] for r in rows}
expected=['مشتری','تلفن','فهرست قیمت تأییدشده','واحد مبلغ','کد کالا یا نام مستعار برای ردیف تازه','تعداد ردیف تازه','تخفیف کل • تومان']
expected += [f'{kind} ردیف {n}' for n in '۱۲۳۴' for kind in ['تعداد','حذف','کد یا نام مستعار']]
checks={n:n in names for n in expected}
report={'result':'passed' if all(checks.values()) else 'failed','process_id':args.pid,'backend':'installed GI Atspi 2.0','nodes_inspected':seen,'checks':checks,'controls':rows,'scope':'AT-SPI names and label relationships; no screen reader or auditory usability test'}
args.out.parent.mkdir(parents=True,exist_ok=True);args.out.write_text(json.dumps(report,ensure_ascii=False,indent=2));print(json.dumps({'result':report['result'],'checks':checks},ensure_ascii=False))
