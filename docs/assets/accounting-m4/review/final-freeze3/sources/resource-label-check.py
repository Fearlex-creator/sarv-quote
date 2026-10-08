#!/usr/bin/env python3
"""Independent human role-label and typed snapshot preservation check."""
import csv,hashlib,json,subprocess,sys,unicodedata
from pathlib import Path
root=Path(sys.argv[1]);out=Path(sys.argv[2]);rows=list(csv.DictReader((root/'masters.csv').open(encoding='utf-8-sig')));parties=[r for r in rows if r['row_kind']=='party'];assert len(parties)==1
row=parties[0];assert row['label']=='شخص New supplier after opening • مشتری خیر • تأمین‌کننده بله'
p=json.loads(row['snapshot_json']);assert p['customer'] is False and p['supplier'] is True and p['id']=='supplier-new'
metadata=next(r for r in rows if r['row_kind']=='document_metadata');record=json.loads(metadata['snapshot_json']);source=record['command']['Resources']['parties'];assert source==[p]
text=subprocess.check_output(['pdftotext',str(root/'masters.pdf'),'-']).decode();compact=''.join(c for c in text if not c.isspace() and unicodedata.category(c)!='Cf');assert 'مشتریخیر' in compact and 'تأمینکنندهبله' in compact and 'true' not in text.lower() and 'false' not in text.lower()
report={'result':'passed','scope':'Human resource-role label localization only; canonical typed snapshot booleans remain unchanged','checks':['exact human CSV role label','typed party false/true booleans retained','full immutable command resource snapshot equals party snapshot','PDF extracted human roles Persian خیر/بله, no literal true/false'],'csv_sha256':hashlib.sha256((root/'masters.csv').read_bytes()).hexdigest(),'pdf_sha256':hashlib.sha256((root/'masters.pdf').read_bytes()).hexdigest(),'party_snapshot':p};out.write_text(json.dumps(report,ensure_ascii=False,indent=2));print(json.dumps(report,ensure_ascii=False,indent=2))
