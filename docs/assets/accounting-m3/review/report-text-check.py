#!/usr/bin/env python3
import json,csv,sys,re
from pathlib import Path
p=Path(sys.argv[1]);text=(p/'opening-toman.txt').read_text()
text=''.join(c for c in text if c not in '\u200e\u200f\u202a\u202b\u202c\u202d\u202e\u2066\u2067\u2068\u2069')
checks={v:v in text for v in ['125.1','1251','2026-03-21','1405/01/01','2026-02-01','2026-04-01','2026-02-10','2026-05-10','2026-03-01','SETTLED-OLD-INVOICE200','SETTLED-OLD-BILL100','2.000','24.000','6.000','5.000','24000','6000','5000','1 carton = 12 / 1 piece']}
assert all(checks.values()),checks
assert '"numerator"' not in text,'raw pack JSON in human PDF'
with (p/'opening-toman.csv').open(encoding='utf-8-sig',newline='') as f:rows=list(csv.DictReader(f))
controls=[r for r in rows if r['row_kind']=='control'];assert len(controls)==12 and all(int(r['difference_irr'])==0 for r in controls)
assert all(r['basis']=='initial_opening_beginning_of_day' and r['display_unit']=='toman' for r in rows)
report={'result':'passed','literal_PDF_substrings':checks,'CSV_controls':len(controls),'zero_difference':True,'basis':'initial_opening_beginning_of_day','display_unit':'toman','limits':'Physical printing/native-reader comprehension not established by extraction; actual rendered pages independently inspected.'}
(p/'independent-report-check.json').write_text(json.dumps(report,indent=2));print(json.dumps(report))
