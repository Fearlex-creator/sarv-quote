from pathlib import Path
import sys
script=Path('/workspace/sarv-quote/scripts/product-qa.py')
sys.path.insert(0,str(script.parent))
source=script.read_text().replace("binary = ROOT / 'target/release/sarv-quote'", "binary = Path(os.environ['SARV_QUOTE_DIAG_BINARY'])")
anchor="        key('4', 'Alt_L')\n        key('f', 'Control_L')\n        x.type_ascii(approved['number'])"
assert source.count(anchor)==1
extra='''        time.sleep(.8)
        before_modal=state(profile)
        key('2','Alt_L')
        key('f','Control_L')
        x.type_text('فهرست ساختگی مستقل')
        time.sleep(.4)
        catalog_row=native.wait_for(lambda: next((r for r in tree() if r['role']=='list item' and 'فهرست ساختگی مستقل' in r['name']),None))
        for _ in range(100):
            if access.focused(catalog_row): break
            key('Tab')
        key('Return')
        action('ساخت نسخهٔ تازه')
        field_row=control('نام فهرست',['text','entry'])
        for _ in range(100):
            if access.focused(field_row): break
            key('Tab')
        key('a','Control_L')
        x.type_ascii('M4 NESTED CANCEL SYNTHETIC')
        tab_to(label='نام‌های مستعار تأییدشده · جداشده با |')
        action('بررسی و تأیید فهرست تازه')
        native.wait_for(lambda: any(r['role']=='dialog' and r['name']=='تأیید فهرست تازه' for r in tree()))
        time.sleep(.3)
        close_row=[r for r in tree() if r['role']=='button' and r['name']=='Close'][-1]
        assert access.click(close_row), 'Native confirmation Close action'
        native.wait_for(lambda: not any(r['role']=='dialog' and r['name']=='تأیید فهرست تازه' for r in tree()))
        native.wait_for(lambda: any(r['role']=='dialog' and r['name']=='ساخت نسخهٔ تازهٔ فهرست' for r in tree()))
        time.sleep(.5)
        field_row=control('نام فهرست',['text','entry'])
        check('Nested confirmation Back preserves parent edited input',access.text(field_row)=='M4 NESTED CANCEL SYNTHETIC')
        key('Tab')
        check('Nested Back leaves active dialog keyboard reachable', any(access.focused(r) for r in tree() if r['role'] in ('text','entry','button')))
        close_row=[r for r in tree() if r['role']=='button' and r['name']=='Close'][-1]
        assert access.click(close_row), 'Native parent Cancel action'
        native.wait_for(lambda: not any(r['role']=='dialog' for r in tree()))
        key('Tab')
        native.wait_for(lambda: native.events(profile)[-1].get('dialog_open') is False)
        check('Parent Cancel returns to main focus and preserves full saved state',state(profile)==before_modal and any(access.focused(r) for r in tree()))

        report['result']='passed'
        raise SystemExit(0)
'''
source=source.replace(anchor,extra+'\n'+anchor)
exec(compile(source,str(script),'exec'),{'__name__':'__main__','__file__':str(script)})
