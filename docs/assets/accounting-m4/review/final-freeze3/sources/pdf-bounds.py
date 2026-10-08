#!/usr/bin/env python3
"""Actual PDF text bounding-box check supplements, never replaces, image review."""
import hashlib,json,subprocess,sys,xml.etree.ElementTree as ET
from pathlib import Path
folder=Path(sys.argv[1]);out=Path(sys.argv[2]);files=[];total_pages=0
for p in sorted(folder.glob('*.pdf')):
 xml=subprocess.check_output(['pdftotext','-bbox',str(p),'-']);root=ET.fromstring(xml);pages=root.findall('.//{*}page');assert pages,p
 words=[]
 for page in pages:
  width=float(page.attrib['width']);height=float(page.attrib['height'])
  for word in page.findall('.//{*}word'):
   box={key:float(word.attrib[key]) for key in ['xMin','xMax','yMin','yMax']}
   assert 30<=box['xMin']<=box['xMax']<=width-30 and 25<=box['yMin']<=box['yMax']<=height-25,(p,box,width,height,word.text)
   words.append(box)
 assert words,p;total_pages+=len(pages);files.append({'file':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'pages':len(pages),'words':len(words),'observed_bounds':{'xMin':min(w['xMin'] for w in words),'xMax':max(w['xMax'] for w in words),'yMin':min(w['yMin'] for w in words),'yMax':max(w['yMax'] for w in words)}})
assert files;report={'result':'passed','scope':'pdftotext bounding boxes within A4 margin; requires separate actual rendered page inspection','pdfs':len(files),'pages':total_pages,'files':files};out.write_text(json.dumps(report,indent=2));print(json.dumps(report))
