from pathlib import Path
import sys
script=Path('/workspace/sarv-quote/scripts/product-qa.py')
sys.path.insert(0,str(script.parent))
source=script.read_text().replace("binary = ROOT / 'target/release/sarv-quote'", "binary = Path(os.environ['SARV_QUOTE_DIAG_BINARY'])")
anchor="        key('4', 'Alt_L')\n        key('f', 'Control_L')\n        x.type_ascii(approved['number'])"
assert source.count(anchor)==1
extra='''        key('1', 'Alt_L')
        before_disclosure = state(profile)[1]
        for caption, field_caption, marker in [
            ('جزئیات پیشنهاد • تاریخ، اعتبار و مرجع', 'مرجع مشتری', 'M4-EXPANDER-REFERENCE'),
            ('یادداشت برای مشتری', 'یادداشت برای مشتری', 'M4-EXPANDER-NOTES'),
            ('ویرایش متن CSV سفارش', 'متن CSV سفارش • item,quantity', None),
        ]:
            button_row = control(caption, ['button'])
            for _ in range(100):
                if access.focused(button_row): break
                key('Tab')
            else: raise AssertionError('Cannot focus disclosure '+caption)
            key('Return')
            capture('disclosure-'+str(len(report['checks']))+'-open.png')
            field_row = control(field_caption, ['text', 'entry'])
            original = access.text(field_row)
            for _ in range(100):
                if access.focused(field_row): break
                key('Tab')
            else: raise AssertionError('Cannot focus disclosure input '+field_caption)
            expected = marker if marker is not None else original.replace('3.125','4.125')
            key('a','Control_L')
            for i, chunk in enumerate(expected.split('\\n')):
                if i: key('Return')
                if chunk: x.type_text(chunk)
            for _ in range(100):
                if access.focused(button_row): break
                key('Tab', 'Shift_L')
            else: raise AssertionError('Cannot return to disclosure '+caption)
            key('Return')
            key('Return')
            field_row = control(field_caption, ['text', 'entry'])
            check('Expanded/collapsed disclosure retains edited '+caption, access.text(field_row)==expected, expected=expected, actual=access.text(field_row))
            for _ in range(100):
                if access.focused(field_row): break
                key('Tab')
            key('a','Control_L')
            if original:
                for i, chunk in enumerate(original.split('\\n')):
                    if i: key('Return')
                    if chunk: x.type_text(chunk)
            else: key('BackSpace')
            for _ in range(100):
                if access.focused(button_row): break
                key('Tab', 'Shift_L')
            key('Return')
        check('Disclosure edits preserve approved quote and accounting histories',
              state(profile)[1]['quotes']==before_disclosure['quotes'] and
              state(profile)[1].get('accounting')==before_disclosure.get('accounting'))
        report['result']='passed'
        raise SystemExit(0)
'''
source=source.replace(anchor,extra+'\n'+anchor)
exec(compile(source,str(script),'exec'),{'__name__':'__main__','__file__':str(script)})
