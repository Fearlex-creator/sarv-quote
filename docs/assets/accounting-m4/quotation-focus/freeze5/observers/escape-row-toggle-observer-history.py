from pathlib import Path
import sys
script=Path('/workspace/sarv-quote/scripts/product-qa.py')
sys.path.insert(0,str(script.parent))
source=script.read_text().replace("binary = ROOT / 'target/release/sarv-quote'", "binary = Path(os.environ['SARV_QUOTE_DIAG_BINARY'])")
anchor="        key('4', 'Alt_L')\n        key('f', 'Control_L')\n        x.type_ascii(approved['number'])"
assert source.count(anchor)==1
extra='''        before_escape=state(profile)
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
        else: raise AssertionError('Composer name is not keyboard reachable')
        key('a','Control_L')
        x.type_ascii('M4 ESCAPE RETAINED NAME')
        event=native.events(profile)[-1]
        ancestry=[]
        ancestor=field_row['node']
        for _ in range(30):
            if not ancestor: break
            ancestry.append({'role':access.string(access.a.atspi_accessible_get_role_name,ancestor,None),'name':access.string(access.a.atspi_accessible_get_name,ancestor,None)})
            ancestor=access.a.atspi_accessible_get_parent(ancestor,None)
        report['modal_focus_observation']={'actual_focused_field':access.focused(field_row),'accessible_ancestry':ancestry,'latest_window_event':event,'limitation':'Window key-controller events may not observe subsequent modal key events; actual focused field ancestry is authoritative here'}
        check('Actual focused name field belongs to visible composer dialog',access.focused(field_row) and any(r['role']=='dialog' and r['name']=='ساخت نسخهٔ تازهٔ فهرست' for r in ancestry),event=event,ancestry=ancestry)
        key('Escape','Shift_L')
        check('Modified Escape preserves composer and edited input',any(r['role']=='dialog' and r['name']=='ساخت نسخهٔ تازهٔ فهرست' for r in tree()) and access.text(field_row)=='M4 ESCAPE RETAINED NAME')
        tab_to(label='واحد قیمت منبع')
        def showing(row):
            states=access.a.atspi_accessible_get_state_set(row['node'])
            return bool(states and all(access.a.atspi_state_set_contains(states, value) for value in (25,30)))
        def popup_toman():
            return any(r['name']=='تومان' and showing(r) for r in tree())
        key('Return')
        native.wait_for(popup_toman, label='actual visible source-currency popup option')
        capture('currency-popover-before-Escape.png')
        key('Escape')
        native.wait_for(lambda: not popup_toman(),label='actual source-currency popup closed')
        check('Dropdown Escape closes only popup and retains composer input',any(r['role']=='dialog' and r['name']=='ساخت نسخهٔ تازهٔ فهرست' for r in tree()) and access.text(field_row)=='M4 ESCAPE RETAINED NAME')
        tab_to(label='نام‌های مستعار تأییدشده · جداشده با |')
        action('بررسی و تأیید فهرست تازه')
        native.wait_for(lambda: any(r['role']=='dialog' and r['name']=='تأیید فهرست تازه' for r in tree()))
        time.sleep(.3)
        confirm_save=control('تأیید منبع و ذخیرهٔ فهرست',['button','push button'])
        for _ in range(100):
            if access.focused(confirm_save): break
            key('Tab')
        else: raise AssertionError('Nested confirm Save is not keyboard reachable')
        report['nested_Escape_focus']={'role':confirm_save['role'],'name':confirm_save['name'],'actual_focused':access.focused(confirm_save),'limitation':'GtkSearchEntry may consume Escape as stop-search; this tests native nested-dialog dismissal from the actual Save button'}
        key('Escape')
        native.wait_for(lambda: not any(r['role']=='dialog' and r['name']=='تأیید فهرست تازه' for r in tree()))
        field_row=control('نام فهرست',['text','entry'])
        check('Nested confirmation Escape closes only confirmation and retains parent inputs',any(r['role']=='dialog' and r['name']=='ساخت نسخهٔ تازهٔ فهرست' for r in tree()) and access.text(field_row)=='M4 ESCAPE RETAINED NAME' and state(profile)==before_escape)
        close_row=[r for r in tree() if r['role']=='button' and r['name']=='Close'][-1]
        witnessed=False
        for step in range(80):
            event=native.events(profile)[-1]
            if access.focused(close_row) or (event.get('focus_scope')=='dialog' and event.get('focus_type') in ('GtkFlowBox','GtkFlowBoxChild')):
                witnessed=True
                break
            key('Tab', *(['Shift_L'] if step<40 else []))
        if not witnessed:
            tab_to(label='نام فهرست')
        report['escape_focus']={'native_header_or_FlowBox_witnessed':witnessed,'event':native.events(profile)[-1],'limitation':None if witnessed else 'Exact FlowBox/header focus was not witnessed; actual name-field Escape is checked instead'}
        capture('composer-plain-Escape-focus.png')
        key('Escape')
        native.wait_for(lambda: not any(r['role']=='dialog' for r in tree()))
        key('Tab')
        check('Plain Escape closes composer at witnessed focus with exact saved state',state(profile)==before_escape and any(access.focused(r) for r in tree()))
        catalog_row=native.wait_for(lambda: next((r for r in tree() if r['role']=='list item' and 'فهرست ساختگی مستقل' in r['name']),None))
        for _ in range(100):
            if access.focused(catalog_row): break
            key('Tab')
        key('Return')
        action('ساخت نسخهٔ تازه')
        close_row=[r for r in tree() if r['role']=='button' and r['name']=='Close'][-1]
        assert access.click(close_row), 'Native composer Close action'
        native.wait_for(lambda: not any(r['role']=='dialog' for r in tree()))
        key('Tab')
        check('Native Close still preserves full saved state and main focus',state(profile)==before_escape and any(access.focused(r) for r in tree()))
        report['result']='passed'
        raise SystemExit(0)
'''
source=source.replace(anchor,extra+'\n'+anchor)
exec(compile(source,str(script),'exec'),{'__name__':'__main__','__file__':str(script)})
