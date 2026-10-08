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
        key('BackSpace')
        tab_to(label='نام‌های مستعار تأییدشده · جداشده با |')
        action('بررسی و تأیید فهرست تازه')
        native.wait_for(lambda: has_text('متن خالی'))
        field_row=control('نام فهرست',['text','entry'])
        check('Async invalid catalog validation retains empty name and exact saved state',
              access.text(field_row)=='' and state(profile)==before_modal and
              not any(r['role']=='dialog' and r['name']=='تأیید فهرست تازه' for r in tree()))
        for _ in range(100):
            if access.focused(field_row): break
            key('Tab')
        else: raise AssertionError('Invalid-name field is not keyboard reachable after validation')
        key('a','Control_L')
        x.type_ascii('M4 REENABLED RETAINED NAME')
        tab_to(label='نام‌های مستعار تأییدشده · جداشده با |')
        key('a','Control_L')
        x.type_ascii('M4-retry-alias')
        alias_row=control('نام‌های مستعار تأییدشده · جداشده با |',['text','entry'])
        check('Validation failure reenables keyboard editing of name and EntryRow',
              access.text(field_row)=='M4 REENABLED RETAINED NAME' and access.text(alias_row)=='M4-retry-alias')
        close_row=[r for r in tree() if r['role']=='button' and r['name']=='Close'][-1]
        assert access.click(close_row), 'Native failed-validation parent Cancel'
        native.wait_for(lambda: not any(r['role']=='dialog' for r in tree()))
        key('Tab')
        check('Cancel after validation error preserves full saved state and main focus',
              state(profile)==before_modal and any(access.focused(r) for r in tree()))

        report['result']='passed'
        raise SystemExit(0)
'''
source=source.replace(anchor,extra+'\n'+anchor)
exec(compile(source,str(script),'exec'),{'__name__':'__main__','__file__':str(script)})
