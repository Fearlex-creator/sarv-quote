#!/usr/bin/env python3
"""Capture publicly readable official workflow text with no account or financial actions."""
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
from hashlib import sha256
from html.parser import HTMLParser
from pathlib import Path
from urllib.error import HTTPError
from urllib.parse import quote, urljoin
from urllib.request import Request, urlopen
import json

ROOT = Path(__file__).resolve().parent
SOURCES = {
    'hesabfa-purchase-return': 'https://www.hesabfa.com/help/topics/خرید-هزینه/برگشت-از-خرید',
    'hesabfa-purchase-returns': 'https://www.hesabfa.com/help/topics/خرید-هزینه/فاکتورهای-برگشت-ازخرید',
    'hesabfa-purchase': 'https://www.hesabfa.com/help/topics/خرید-هزینه/خرید-جدید',
    'holoo-supermarket': 'https://holoo.co.ir/product/سوپر-مارکت/',
    'sepidar-warehouse': 'https://www.sepidarsystem.com/products/sepidar/warehouse-system/',
    'sepidar-commercial': 'https://www.sepidarsystem.com/products/sepidar/commercial-accounting-software/',
    'zoho-vendor-credits': 'https://www.zoho.com/us/books/help/vendor-credits/',
    'zoho-vendor-credit-create': 'https://www.zoho.com/us/books/help/vendor-credits/basic-functions.html',
    'zoho-vendor-credit-functions': 'https://www.zoho.com/us/books/help/vendor-credits/functions.html',
    'zoho-bills': 'https://www.zoho.com/us/books/help/bills/',
    'grandtotal-product': 'https://www.mediaatelier.com/en-US/GrandTotal/',
    'grandtotal-guides': 'https://www.mediaatelier.com/en-US/Video-Guides/',
}

class Reader(HTMLParser):
    def __init__(self):
        super().__init__()
        self.skip = 0
        self.parts = []
        self.links = []
    def handle_starttag(self, tag, attrs):
        if tag in ('script', 'style'):
            self.skip += 1
        if tag == 'a':
            self.links.extend(value for key, value in attrs if key == 'href' and value)
    def handle_endtag(self, tag):
        if tag in ('script', 'style'):
            self.skip = max(0, self.skip - 1)
    def handle_data(self, data):
        if not self.skip and data.strip():
            self.parts.append(data.strip())

def capture(pair):
    name, url = pair
    entry = {'id': name, 'requested_url': url, 'checked_at_utc': datetime.now(timezone.utc).isoformat()}
    try:
        encoded = quote(url, safe=':/%?=&')
        request = Request(encoded, headers={'User-Agent': 'Sarv-public-workflow-research/1.0'})
        with urlopen(request, timeout=30) as response:
            raw = response.read(4 * 1024 * 1024 + 1)
            if len(raw) > 4 * 1024 * 1024:
                raise ValueError('response exceeds bounded capture size')
            entry.update(status=response.status, final_url=response.url)
        parser = Reader()
        parser.feed(raw.decode('utf-8', 'replace'))
        captured = '\n'.join(parser.parts) + '\n'
        path = ROOT / (name + '.txt')
        path.write_text(captured, encoding='utf-8')
        entry.update(text_path=path.name, text_sha256=sha256(captured.encode()).hexdigest(),
                     response_sha256=sha256(raw).hexdigest(), response_bytes=len(raw),
                     links=sorted(set(urljoin(entry['final_url'], link) for link in parser.links)))
    except HTTPError as error:
        entry.update(status=error.code, final_url=error.url, error=str(error))
    except Exception as error:
        entry.update(status='unavailable', error=str(error))
    return entry

if __name__ == '__main__':
    with ThreadPoolExecutor(max_workers=5) as pool:
        entries = list(pool.map(capture, SOURCES.items()))
    (ROOT / 'source-register.json').write_text(json.dumps(entries, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps([{key: value for key, value in entry.items() if key in ('id', 'status', 'final_url', 'error')} for entry in entries], ensure_ascii=False, indent=2))
