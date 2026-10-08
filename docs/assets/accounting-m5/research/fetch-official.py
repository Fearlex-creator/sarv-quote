#!/usr/bin/env python3
"""Unauthenticated read-only official-source capture; no financial/vendor account calls."""
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
from hashlib import sha256
from html.parser import HTMLParser
from pathlib import Path
import json
from urllib.parse import quote, urljoin
from urllib.request import Request, urlopen

ROOT = Path(__file__).resolve().parent
SOURCES = {
    'hesabfa-accounting': 'https://www.hesabfa.com/view/facilities/accounting',
    'hesabfa-help': 'https://www.hesabfa.com/help/topics',
    'hesabfa-sale': 'https://www.hesabfa.com/help/topics/فروش-درآمد/فروش-جدید',
    'hesabfa-return': 'https://www.hesabfa.com/help/topics/فروش-درآمد/برگشت-از-فروش',
    'hesabfa-invoices': 'https://www.hesabfa.com/help/topics/فروش-درآمد/فاکتورهای-فروش',
    'holoo-supermarket': 'https://holoo.co.ir/product/سوپر-مارکت/',
    'sepidar-sales': 'https://www.sepidarsystem.com/products/sepidar/customer-and-sales-system/',
    'sepidar-return': 'https://www.sepidarsystem.com/blog/sales-return-factor/',
    'zoho-sales-order': 'https://www.zoho.com/books/help/sales-order/',
    'zoho-partial-invoice': 'https://www.zoho.com/us/books/help/sales-order/convert-to-invoice.html',
    'zoho-credit-note': 'https://www.zoho.com/books/help/credit-note/',
    'grandtotal-product': 'https://www.mediaatelier.com/en-US/GrandTotal/',
    'grandtotal-guides': 'https://www.mediaatelier.com/en-US/Video-Guides/',
    'grandtotal-comparison': 'https://www.mediaatelier.com/en-US/Comparison/',
}
class Reader(HTMLParser):
    def __init__(self):
        super().__init__(); self.skip=0; self.parts=[]; self.links=[]
    def handle_starttag(self, tag, attrs):
        if tag in ('script','style'): self.skip+=1
        if tag=='a':
            for k,v in attrs:
                if k=='href' and v: self.links.append(v)
    def handle_endtag(self, tag):
        if tag in ('script','style'): self.skip=max(0,self.skip-1)
    def handle_data(self, data):
        if not self.skip and data.strip(): self.parts.append(data.strip())
def capture(pair):
    name,url=pair
    entry={'id':name,'requested_url':url,'checked_at_utc':datetime.now(timezone.utc).isoformat()}
    try:
        encoded=quote(url,safe=':/%?=&')
        with urlopen(Request(encoded,headers={'User-Agent':'Sarv-public-workflow-research/1.0'}),timeout=30) as response:
            raw=response.read(4*1024*1024); entry.update(status=response.status,final_url=response.url)
        parser=Reader(); parser.feed(raw.decode('utf-8','replace'))
        text='\n'.join(parser.parts)+'\n'; path=ROOT/(name+'.txt'); path.write_text(text,encoding='utf-8')
        entry.update(text_path=path.name,text_sha256=sha256(text.encode()).hexdigest(),response_sha256=sha256(raw).hexdigest(),response_bytes=len(raw),links=sorted(set(urljoin(entry['final_url'],v) for v in parser.links)))
    except Exception as error: entry.update(status='unavailable',error=str(error))
    return entry
if __name__=='__main__':
    entries=list(ThreadPoolExecutor(max_workers=5).map(capture,SOURCES.items()))
    (ROOT/'source-register.json').write_text(json.dumps(entries,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps([{k:v for k,v in row.items() if k in ('id','status','final_url','error','text_path')} for row in entries],ensure_ascii=False,indent=2))
