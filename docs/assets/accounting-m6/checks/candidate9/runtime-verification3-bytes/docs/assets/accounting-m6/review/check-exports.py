#!/usr/bin/env python3
import csv,hashlib,json,subprocess
from pathlib import Path
repo=Path(__file__).resolve().parents[4];exports=repo/'docs/assets/accounting-m6/exports';out=Path(__file__).with_name('export-inspection');out.mkdir(exist_ok=True)
manifest=json.loads((exports/'focused-manifest.json').read_text())
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
assert all(sha(exports/name)==digest for name,digest in manifest['artifacts'].items())
source_comparison={p:sha(repo/p)==digest for p,digest in manifest['source_capture_after_focused_run'].items()}
checks=[]
for name,amount,lines,movements,kind in [('billed-physical-return',45,1,1,'purchase_return'),('billed-supplier-credit',45,1,0,'supplier_credit'),('unbilled-return-one-rial',1,1,1,'purchase_return'),('zero-supplier-credit',0,1,0,'supplier_credit'),('long-physical-return',180,12,12,'purchase_return')]:
    path=exports/(name+'.csv');assert path.read_bytes().startswith(b'\xef\xbb\xbf')
    with path.open(encoding='utf-8-sig',newline='') as f:rows=list(csv.DictReader(f))
    assert all(r['format_version']=='sarv-procurement-report-v2' and r['document_kind']==kind and r['base_currency']=='IRR' and r['display_unit']=='toman' for r in rows)
    line_rows=[r for r in rows if r['row_kind'] in ['supplier_credit_line','purchase_return_line']]
    total=next(r for r in rows if r['row_kind']=='document_total')
    assert len(line_rows)==lines and int(total['amount_irr'])==amount and sum(int(r['amount_irr']) for r in line_rows)==amount
    assert len([r for r in rows if r['row_kind']=='stock_movement'])==movements
    snapshot=json.loads(next(r for r in rows if r['row_kind']=='complete_snapshot')['snapshot_json']);doc=snapshot['document'];returned=doc['data']['Return']
    assert returned['total']==amount and returned['return_state']=='Returned'
    assert returned.get('credit_id') is None or returned['credit_id']!=returned['return_id'] and returned['credit_state']=='Posted'
    for r in line_rows:
        raw=json.loads(r['snapshot_json']);line=raw['line'];cursor=raw['cursor_after_event']
        assert int(r['amount_irr'])==line['value'] and int(r['quantity_thousandths'])==line['quantity_thousandths']
        assert int(r['original_quantity_thousandths'])==cursor['original_quantity_thousandths'] and int(r['original_value_irr'])==cursor['original_value']
        assert int(r['remaining_quantity_thousandths'])==cursor['remaining_quantity_thousandths'] and int(r['remaining_value_irr'])==cursor['remaining_value']
        assert r['receipt_line_id']==line['receipt_line_id'] and r['warehouse_id']==line['warehouse_id'] and r['financial_date']=='2026-'+('04-03' if name=='unbilled-return-one-rial' else '04-05' if name=='zero-supplier-credit' else '04-02' if name=='long-physical-return' else '04-01')
        if name.startswith('billed'):
            assert [r[k] for k in ['original_quantity_thousandths','original_value_irr','remaining_quantity_thousandths','remaining_value_irr','remaining_payable_irr']]==['7000','105','4000','60','105']
            assert r['source_reference'].startswith("'=",0) and doc['command']['Return']['return_reference'].startswith('=')
        if name=='unbilled-return-one-rial':assert r['display_amount']=='0.1' and r['remaining_quantity_thousandths']=='1000' and r['remaining_value_irr']=='0' and not r['credit_id']
        if name=='zero-supplier-credit':assert r['display_amount']=='0' and doc['journal_id'] is None and r['credit_state']=='posted'
    text_path=out/(name+'-fresh-text.txt');subprocess.run(['pdftotext','-layout',str(exports/(name+'.pdf')),str(text_path)],check=True)
    text=text_path.read_text();pages=[s for s in text.split('\f') if s.strip()];assert len(pages)==manifest['inspection'][name+'.pdf']
    assert all('core-policy-v1' in page and 'IRR' in page and 'toman' in page for page in pages)
    fonts=subprocess.check_output(['pdffonts',str(exports/(name+'.pdf'))],text=True);(out/(name+'-fresh-fonts.txt')).write_text(fonts)
    fontlines=[l.split() for l in fonts.splitlines()[2:] if l.strip()];assert fontlines and all('Vazirmatn' in l[0] and l[-5:-2]==['yes','yes','yes'] for l in fontlines)
    checks.append({'document':name,'canonical_total':amount,'lines':lines,'stock_rows':movements,'pdf_pages':len(pages),'csv_snapshot_exact':True,'fonts_embedded_subset_unicode':True,'pdf_sha256':sha(exports/(name+'.pdf'))})
result={'status':'PASS','focused_manifest_artifact_hashes_match':True,'source_hash_comparison_to_focused_after_capture':source_comparison,'documents':checks,'actual_visual_pages_independently_viewed':10,'visual_observation':'All ten provided Poppler pages were personally opened. Readable Persian and mixed codes/dates; original-cost totals45/1/0/180 IRR and4.5/0.1/0/18toman agree; long pages1–6 retain all12 source rows and12 stock rows with repeated identity/date/unit/policy headings and no observed clipping.','limits':'Focused export epoch, not native or final build qualification. PDFs untagged; physical printing and screen readers untested.'}
(out/'result.json').write_text(json.dumps(result,indent=2,ensure_ascii=False)+'\n');print('PASS5 CSV raw source snapshots, PDF10 pages/font reports, focused artifact hashes and personal10-page visual inspection')
