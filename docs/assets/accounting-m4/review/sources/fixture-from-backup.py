#!/usr/bin/env python3
"""Rebuild only a disposable synthetic fixture from portable validated backups.

Envelope checksums are checked here; semantic history validation belongs to the
current Store/app and is exercised again by native-review.py. No live profile
is read, overwritten or transported in the portable evidence.
"""
import hashlib,json,shutil,sqlite3,sys
from pathlib import Path

full,opening,out=map(Path,sys.argv[1:])
def read(path,schema):
 b=json.loads(path.read_text())
 assert set(b)=={'format','checksum','state'}
 assert b['format']==f'sarv-accounting-backup-v{schema}' and b['state']['schema']==schema
 payload=json.dumps(b['state'],ensure_ascii=False,separators=(',',':'))
 assert hashlib.sha256(payload.encode()).hexdigest()==b['checksum']
 return b,payload
b,payload=read(full,5);old,_=read(opening,4)
assert len(b['state']['accounting']['procurement']['records'])==10
assert len(b['state']['accounting']['journals'])==5
assert len(old['state']['accounting']['core']['openings'])==1
out.mkdir(parents=True,exist_ok=False);profile=out/'full';profile.mkdir()
with sqlite3.connect(profile/'sarv.sqlite3') as db:
 db.execute('create table state(id integer primary key,generation integer not null,payload text not null)')
 db.execute('insert into state values(1,0,?)',[payload])
shutil.copyfile(opening,profile/'before-schema5-portable-original.json')
print(json.dumps({'result':'passed','scope':'synthetic fixture transport adapter; Store validates semantic history in the actual native run','full_checksum':b['checksum'],'original_v4_checksum':old['checksum'],'fixture':str(out)},indent=2))
