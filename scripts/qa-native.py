#!/usr/bin/env python3
"""Independent real-X11-key/resize/restart checks against a synthetic smoke profile.

Run after sourcing /workspace/sarv-environment/activate.sh:
  dbus-run-session -- /usr/bin/python3 scripts/qa-native.py --profile PATH
Only profiles inside artifacts with passing native smoke reports are accepted.
The source profile is read-only; all mutations occur in a new private copy.
"""
import argparse
import ctypes as C
import ctypes.util
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import sqlite3
import subprocess
import tempfile
import time
from qa_access import Access

ROOT = Path(__file__).resolve().parents[1]
ENVROOT = Path('/workspace/sarv-environment')


def wait_for(predicate, seconds=15, label='condition'):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        result = predicate()
        if result:
            return result
        time.sleep(.05)
    raise RuntimeError(f'Timed out waiting for {label}')


def private_accessibility_environment(display, profile):
    """Align this QA client and D-Bus activation with its disposable display.

    The regular desktop's runtime/bus must never be reconfigured. A private
    dbus-run-session address is required before changing activation variables.
    """
    address = os.environ.get('DBUS_SESSION_BUS_ADDRESS', '')
    if not address.startswith('unix:path=/tmp/dbus-'):
        raise RuntimeError('Run QA inside its own dbus-run-session')
    runtime = profile / 'runtime'
    runtime.mkdir(mode=0o700)
    data_home = profile / 'xdg-data'
    data_home.mkdir(mode=0o700)
    os.environ.update(DISPLAY=display, XDG_RUNTIME_DIR=str(runtime), XDG_DATA_HOME=str(data_home))
    subprocess.run(['/usr/bin/dbus-update-activation-environment', 'DISPLAY', 'XDG_RUNTIME_DIR', 'XDG_DATA_HOME'], check=True)
    return runtime


class ClientMessage(C.Structure):
    _fields_ = [('type', C.c_int), ('serial', C.c_ulong), ('send_event', C.c_int),
                ('display', C.c_void_p), ('window', C.c_ulong),
                ('message_type', C.c_ulong), ('format', C.c_int),
                ('data', C.c_long * 5)]


class XEvent(C.Union):
    _fields_ = [('client', ClientMessage), ('pad', C.c_long * 24)]


class X11:
    def __init__(self, display):
        self.x = C.CDLL(ctypes.util.find_library('X11'))
        self.t = C.CDLL(ctypes.util.find_library('Xtst'))
        signatures = {
            'XOpenDisplay': (C.c_void_p, [C.c_char_p]),
            'XDefaultRootWindow': (C.c_ulong, [C.c_void_p]),
            'XInternAtom': (C.c_ulong, [C.c_void_p, C.c_char_p, C.c_int]),
            'XGetWindowProperty': (C.c_int, [C.c_void_p, C.c_ulong, C.c_ulong,
                C.c_long, C.c_long, C.c_int, C.c_ulong, C.POINTER(C.c_ulong),
                C.POINTER(C.c_int), C.POINTER(C.c_ulong), C.POINTER(C.c_ulong),
                C.POINTER(C.POINTER(C.c_ubyte))]),
            'XFree': (C.c_int, [C.c_void_p]),
            'XStringToKeysym': (C.c_ulong, [C.c_char_p]),
            'XKeysymToKeycode': (C.c_uint, [C.c_void_p, C.c_ulong]),
            'XFlush': (C.c_int, [C.c_void_p]),
            'XSetInputFocus': (C.c_int, [C.c_void_p, C.c_ulong, C.c_int, C.c_ulong]),
            'XRaiseWindow': (C.c_int, [C.c_void_p, C.c_ulong]),
            'XSendEvent': (C.c_int, [C.c_void_p, C.c_ulong, C.c_int, C.c_long, C.POINTER(XEvent)]),
            'XGetGeometry': (C.c_int, [C.c_void_p, C.c_ulong, C.POINTER(C.c_ulong),
                C.POINTER(C.c_int), C.POINTER(C.c_int), C.POINTER(C.c_uint),
                C.POINTER(C.c_uint), C.POINTER(C.c_uint), C.POINTER(C.c_uint)]),
            'XTranslateCoordinates': (C.c_int, [C.c_void_p, C.c_ulong, C.c_ulong,
                C.c_int, C.c_int, C.POINTER(C.c_int), C.POINTER(C.c_int), C.POINTER(C.c_ulong)]),
            'XCloseDisplay': (C.c_int, [C.c_void_p]),
        }
        for name, (result, args) in signatures.items():
            fn = getattr(self.x, name)
            fn.restype, fn.argtypes = result, args
        self.t.XTestFakeKeyEvent.argtypes = [C.c_void_p, C.c_uint, C.c_int, C.c_ulong]
        self.t.XTestFakeMotionEvent.argtypes = [C.c_void_p, C.c_int, C.c_int, C.c_int, C.c_ulong]
        self.t.XTestFakeButtonEvent.argtypes = [C.c_void_p, C.c_uint, C.c_int, C.c_ulong]
        self.d = self.x.XOpenDisplay(display.encode())
        assert self.d, f'Cannot open private display {display}'
        self.root = self.x.XDefaultRootWindow(self.d)

    def atom(self, name):
        return self.x.XInternAtom(self.d, name.encode(), 0)

    def prop(self, window, name):
        actual, fmt, count, left = C.c_ulong(), C.c_int(), C.c_ulong(), C.c_ulong()
        data = C.POINTER(C.c_ubyte)()
        self.x.XGetWindowProperty(self.d, window, self.atom(name), 0, 4096, 0, 0,
            C.byref(actual), C.byref(fmt), C.byref(count), C.byref(left), C.byref(data))
        try:
            if not data:
                return []
            if fmt.value == 32:
                values = C.cast(data, C.POINTER(C.c_ulong))
                return [values[i] for i in range(count.value)]
            return C.string_at(data, count.value)
        finally:
            if data:
                self.x.XFree(data)

    def find(self, pid):
        return next((w for w in self.prop(self.root, '_NET_CLIENT_LIST')
                     if self.prop(w, '_NET_WM_PID') == [pid]), None)

    def focus(self, window):
        self.x.XRaiseWindow(self.d, window)
        self.x.XSetInputFocus(self.d, window, 2, 0)
        self.x.XFlush(self.d)

    def key(self, key, *modifiers):
        codes = [self.x.XKeysymToKeycode(self.d, self.x.XStringToKeysym(n.encode()))
                 for n in (*modifiers, key)]
        assert all(codes), (key, modifiers)
        for code in codes:
            self.t.XTestFakeKeyEvent(self.d, code, 1, 0)
        for code in reversed(codes):
            self.t.XTestFakeKeyEvent(self.d, code, 0, 0)
        self.x.XFlush(self.d)

    def type_ascii(self, text):
        special = {'/': ('slash', ()), '.': ('period', ()), '-': ('minus', ()),
                   '_': ('minus', ('Shift_L',)), ':': ('semicolon', ('Shift_L',)),
                   '@': ('2', ('Shift_L',)), '|': ('backslash', ('Shift_L',)),
                   ',': ('comma', ()), '+': ('equal', ('Shift_L',)),
                   ' ': ('space', ())}
        for char in text:
            key, mods = special.get(char, (char.lower(), ('Shift_L',) if char.isupper() else ()))
            self.key(key, *mods)

    def type_text(self, text):
        """Use GTK's real Unicode keyboard input (Ctrl+Shift+U, hex, Space).

        Changing XKB mappings between individual characters races GTK's map
        cache; the native input method is deterministic and preserves XKB.
        """
        for char in text:
            if ord(char) < 128:
                self.type_ascii(char)
            else:
                self.key('u', 'Control_L', 'Shift_L')
                time.sleep(.025)
                self.type_ascii(format(ord(char), 'x'))
                # Space commits the Unicode composition without invoking an
                # explicit Enter-to-choose controller in a search popup.
                self.key('space')
            time.sleep(.025)

    def pointer(self, x, y, button=1):
        self.t.XTestFakeMotionEvent(self.d, -1, x, y, 0)
        self.t.XTestFakeButtonEvent(self.d, button, 1, 0)
        self.t.XTestFakeButtonEvent(self.d, button, 0, 0)
        self.x.XFlush(self.d)

    def origin(self, window):
        x, y, child = C.c_int(), C.c_int(), C.c_ulong()
        self.x.XTranslateCoordinates(self.d, window, self.root, 0, 0,
                                     C.byref(x), C.byref(y), C.byref(child))
        return x.value, y.value

    def resize(self, window, width, height):
        event = XEvent()
        event.client.type = 33
        event.client.display = self.d
        event.client.window = window
        event.client.message_type = self.atom('_NET_MOVERESIZE_WINDOW')
        event.client.format = 32
        event.client.data[:] = [(1 << 8) | (1 << 9) | (1 << 10) | (1 << 11) | (1 << 12),
                                 20, 20, width, height]
        self.x.XSendEvent(self.d, self.root, 0, (1 << 20) | (1 << 19), C.byref(event))
        self.x.XFlush(self.d)

    def geometry(self, window):
        root, x, y = C.c_ulong(), C.c_int(), C.c_int()
        width, height, border, depth = (C.c_uint() for _ in range(4))
        self.x.XGetGeometry(self.d, window, C.byref(root), C.byref(x), C.byref(y),
            C.byref(width), C.byref(height), C.byref(border), C.byref(depth))
        return [width.value, height.value]


def snapshot(profile):
    with sqlite3.connect(f'file:{profile}/sarv.sqlite3?mode=ro', uri=True) as conn:
        generation, payload = conn.execute('SELECT generation,payload FROM state WHERE id=1').fetchone()
    state = json.loads(payload)
    return {'generation': generation, 'sha256': hashlib.sha256(payload.encode()).hexdigest(),
            'quotes': len(state['quotes']), 'settings': state['settings'], 'draft': state['draft'],
            'customers': state['customers'],
            'history_sha256': hashlib.sha256(json.dumps(state['quotes'], sort_keys=True).encode()).hexdigest(),
            'catalogs_sha256': hashlib.sha256(json.dumps(state['catalogs'], sort_keys=True).encode()).hexdigest()}


def events(profile):
    path = profile / 'keyboard-events.jsonl'
    return [json.loads(line) for line in path.read_text().splitlines()] if path.exists() else []


def stop(proc):
    if proc and proc.poll() is None:
        os.killpg(proc.pid, signal.SIGTERM)
        try:
            proc.wait(timeout=4)
        except subprocess.TimeoutExpired:
            os.killpg(proc.pid, signal.SIGKILL)
            proc.wait()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--profile', type=Path, required=True)
    parser.add_argument('--stress', action='store_true', help='Also exercise native settings/customer/product controls with real keys and AT-SPI names')
    args = parser.parse_args()
    source = args.profile.resolve()
    assert source.is_relative_to(ROOT / 'artifacts'), 'Source must be synthetic artifacts profile'
    assert json.loads((source / 'native-verification.json').read_text())['result'] == 'passed'
    assert os.environ.get('DBUS_SESSION_BUS_ADDRESS'), 'Run inside dbus-run-session'
    output = ROOT / 'artifacts' / 'independent-native'
    output.mkdir(parents=True, exist_ok=True)
    profile = Path(tempfile.mkdtemp(prefix='run-', dir=output))
    with sqlite3.connect(f'file:{source}/sarv.sqlite3?mode=ro', uri=True) as src:
        with sqlite3.connect(profile / 'sarv.sqlite3') as dst:
            src.backup(dst)
    report = {'result': 'failed', 'profile': str(profile), 'source': str(source),
              'binary_sha256': hashlib.sha256((ROOT / 'target/release/sarv-quote').read_bytes()).hexdigest(),
              'backend': 'X11 Xvfb + Openbox + real XTest key events', 'checks': []}
    processes = []
    logs = []
    x11 = None
    def start(command, name, env=None, **kwargs):
        log = (profile / f'{name}.log').open('w')
        logs.append(log)
        proc = subprocess.Popen(command, env=env, stdout=log, stderr=log,
                                start_new_session=True, **kwargs)
        processes.append(proc)
        return proc
    def check(name, condition, **details):
        report['checks'].append(dict(name=name, passed=bool(condition), **details))
        if not condition:
            raise AssertionError(name)
    try:
        with tempfile.TemporaryFile() as display_file:
            start(['Xvfb', '-displayfd', str(display_file.fileno()), '-screen', '0',
                   '1450x1050x24', '-nolisten', 'tcp', '-xkbdir',
                   str(ENVROOT / 'sysroot/usr/share/X11/xkb')], 'xvfb', pass_fds=(display_file.fileno(),))
            def read_display():
                display_file.seek(0)
                return display_file.read().decode().strip()
            display = ':' + wait_for(read_display, label='private Xvfb display')
        runtime = private_accessibility_environment(display, profile)
        env = os.environ.copy()
        env.update(DISPLAY=display, GDK_BACKEND='x11', GSK_RENDERER='cairo',
                   XDG_RUNTIME_DIR=str(runtime), SARV_DATA_DIR=str(profile),
                   SARV_TEST_INSTANCE='qa-independent', SARV_UI_SMOKE='capture',
                   SARV_CAPTURE_PAGE='order', SARV_FOCUS_ROW='1',
                   SARV_CAPTURE_NAME='initial.png')
        start(['openbox', '--sm-disable', '--config-file',
               str(ENVROOT / 'sysroot/etc/xdg/openbox/rc.xml')], 'openbox', env)
        x11 = X11(display)
        app = start([str(ROOT / 'target/release/sarv-quote')], 'app', env)
        window = wait_for(lambda: x11.find(app.pid), label='owned GTK window')
        wait_for(lambda: (profile / 'initial.png').exists(), label='initial native rendering')
        x11.focus(window)
        baseline = snapshot(profile)
        def press(key, *mods, page=None, focus=None):
            count = len(events(profile))
            begin = time.monotonic()
            x11.key(key, *mods)
            seen = wait_for(lambda: events(profile)[count:] if len(events(profile)) > count else None,
                            label=f'GTK received {mods}+{key}')
            target = seen[-1]
            if page:
                target = wait_for(lambda: next((e for e in events(profile)[count:]
                                  if e['page'] == page), None), label=f'{key} switched page')
            settled_focus = None
            if focus:
                def current_focus():
                    nonlocal settled_focus
                    found = next((e for e in events(profile)[count:] if e['focus'] == focus), None)
                    if found:
                        return found
                    # Page crossing saves asynchronously; the key's idle log can
                    # precede its callback. Read the live owned field without
                    # sending another key or changing the interaction epoch.
                    match = re.fullmatch(r'row(\d+)-quantity', focus)
                    if not match:
                        return None
                    probe = Access(app.pid)
                    name = 'تعداد ردیف ' + str(int(match[1]) + 1).translate(str.maketrans('0123456789', '۰۱۲۳۴۵۶۷۸۹'))
                    fields = [row for row in probe.tree() if row['name'] == name
                              and row['role'] in ('entry', 'text') and probe.focused(row)]
                    if len(fields) != 1:
                        return None
                    field = fields[0]
                    node = field['node']
                    ancestry = []
                    for _ in range(40):
                        probe.a.atspi_accessible_clear_cache(node)
                        states = probe.a.atspi_accessible_get_state_set(node)
                        if not states or probe.a.atspi_state_set_contains(states, 6):  # DEFUNCT
                            return None
                        parent = probe.a.atspi_accessible_get_parent(node, None)
                        if not parent:
                            return None
                        probe.a.atspi_accessible_clear_cache(parent)
                        if node not in probe.children(parent):
                            return None
                        ancestry.append(hex(node))
                        if parent == probe.app():
                            if (probe.string(probe.a.atspi_accessible_get_name, field['node'], None) != name
                                    or not probe.focused(field)):
                                return None
                            settled_focus = {'scope': 'read-only current AT-SPI field after asynchronous callback',
                                             'requested_focus': focus, 'name': name, 'role': field['role'],
                                             'focused': True, 'text': probe.text(field), 'ancestry': ancestry,
                                             'original_key_event': seen[-1]}
                            return seen[-1]
                        node = parent
                    raise RuntimeError('Owned quotation field ancestry exceeded bound')
                target = wait_for(current_focus, label=f'{key} moved focus')
            if page:
                # Crossfade completes after the shortcut; capture its settled allocation.
                time.sleep(.4)
            return {'event': target, 'settled_focus_observation': settled_focus,
                    'latency_seconds': round(time.monotonic()-begin, 3)}
        check('Tab enters first row quantity', True, **press('Tab', focus='row0-quantity'))
        check('Shift+Tab returns to first row query', True,
              **press('Tab', 'Shift_L', focus='row0-query'))
        check('Enter saves/reviews and retains query focus', True, **press('Return', focus='row0-query'))
        check('Ctrl+Down advances same-column row focus', True,
              **press('Down', 'Control_L', focus='row1-query'))
        check('Ctrl+Up returns same-column row focus', True,
              **press('Up', 'Control_L', focus='row0-query'))
        check('Ctrl+S saves existing draft and retains query focus', True,
              **press('s', 'Control_L', focus='row0-query'))
        time.sleep(.3)
        check('Save preserved exact payload', snapshot(profile) == baseline)
        check('Ctrl+Enter reaches approved workflow', True, **press('Return', 'Control_L'))
        time.sleep(.3)
        check('Repeat approval preserved existing quote count', snapshot(profile)['quotes'] == baseline['quotes'])
        for number, page in [(2, 'catalog'), (3, 'customers'), (4, 'quotes'),
                             (5, 'backup'), (6, 'settings')]:
            check(f'Alt+{number} opens {page}', True, **press(str(number), 'Alt_L', page=page))
        check('Ctrl+S on Settings', True, **press('s', 'Control_L', page='settings'))
        time.sleep(.3)
        check('Settings save preserved exact persisted settings', snapshot(profile)['settings'] == baseline['settings'])
        check('Alt+1 opens quotation', True, **press('1', 'Alt_L', page='order'))
        def tab_to(target=None, label=None, maximum=120):
            for _ in range(maximum):
                last = events(profile)[-1] if events(profile) else {}
                if (target and last.get('focus') == target) or (label and last.get('focus_label') == label):
                    return last
                press('Tab')
            raise AssertionError(f'Native Tab cannot reach focus={target}, label={label}')
        for width, height in [(900, 940), (900, 560), (700, 560)]:
            x11.resize(window, width, height)
            geometry = wait_for(lambda: x11.geometry(window) if x11.geometry(window) == [width, height] else None,
                                label=f'actual {width}x{height} client allocation')
            time.sleep(.25)
            check(f'Window accepts actual {width}x{height} allocation', geometry == [width, height], geometry=geometry)
            check(f'Keyboard navigation remains responsive at {width}x{height}', True,
                  **press('6', 'Alt_L', page='settings'))
            press('1', 'Alt_L', page='order')
            time.sleep(.5)
            subprocess.run(['/usr/bin/import', '-display', display, '-window', str(window),
                            str(profile / f'window-{width}x{height}.png')], check=True)
            if width == 900 and height == 560:
                # Native RTL boxes traverse left/right controls according to
                # their geometry. Observe reachability without assuming their
                # insertion order (quantity can precede item in this layout).
                desired, observed = {'new-item', 'new-quantity', 'discount'}, {}
                for _ in range(90):
                    event = events(profile)[-1]
                    if event.get('focus') in desired:
                        observed[event['focus']] = event
                    if desired == set(observed):
                        break
                    press('Tab')
                for target in sorted(desired):
                    report['checks'].append({'name': f'Real Tab reaches {target} at 900x560',
                                             'passed': target in observed, 'event': observed.get(target)})
                if 'discount' in observed:
                    tab_to(target='discount')
                time.sleep(.5)
                subprocess.run(['/usr/bin/import', '-display', display, '-window', str(window),
                                str(profile / 'window-900x560-lower-controls.png')], check=True)
        check('All key/resize actions preserved exact persisted state', snapshot(profile) == baseline)
        # A close/relaunch uses a fresh native process and the same isolated data.
        x11.key('F4', 'Alt_L')
        wait_for(lambda: app.poll() is not None, label='keyboard window close')
        before = snapshot(profile)
        env.update(SARV_CAPTURE_NAME='restart.png')
        app = start([str(ROOT / 'target/release/sarv-quote')], 'restart', env)
        window = wait_for(lambda: x11.find(app.pid), label='new process GTK window')
        wait_for(lambda: (profile / 'restart.png').exists(), label='restart native rendering')
        check('Fresh process retained payload, settings, quotes and draft exactly', snapshot(profile) == before,
              payload_sha256=before['sha256'], quotes=before['quotes'], generation=before['generation'])
        if args.stress:
            x11.focus(window)
            access = Access(app.pid)
            tree = wait_for(lambda: access.tree(), label='owned AT-SPI app')
            (profile / 'a11y-tree.json').write_text(json.dumps(tree, ensure_ascii=False, indent=2))
            check('Owned synthetic process exposes native AT-SPI controls', len(tree) > 30, nodes=len(tree))
            press('6', 'Alt_L', page='settings')
            tree = access.tree()
            (profile / 'a11y-settings-tree.json').write_text(json.dumps(tree, ensure_ascii=False, indent=2))
            def control(name, roles=None):
                return wait_for(lambda: access.find(name, roles), label=f'owned native control {name}')
            def native_focus(row):
                field_targets = {'کد کالا یا نام مستعار برای ردیف تازه': 'new-item',
                                 'تعداد ردیف تازه': 'new-quantity',
                                 'تخفیف کل • تومان': 'discount',
                                 'جست‌وجوی فهرست قیمت و کالا': 'search-catalog'}
                label = 'افزودن' if row['name'].startswith('افزودن P-101') else row['name']
                if row['name'] in field_targets:
                    tab_to(target=field_targets[row['name']])
                    return
                if row['role'] in ['button', 'push button'] or row['name'] in [next_title, 'نام کسب‌وکار']:
                    tab_to(label=label)
                    return
                if access.focus(row):
                    return
                # Native Tab traversal also scrolls the focused setting into
                # view. AT-SPI is used only to observe which control has focus.
                for _ in range(120):
                    if access.focused(row):
                        return
                    x11.key('Tab')
                    time.sleep(.035)
                raise AssertionError(f'Native control cannot receive keyboard focus: {row}')
            def edit(name, value):
                row = control(name, ['text', 'entry'])
                native_focus(row)
                x11.key('a', 'Control_L')
                x11.type_ascii(value)
                time.sleep(.2)
                return control(name, ['text', 'entry'])
            def activate(name):
                row = control(name, ['button', 'push button'])
                native_focus(row)
                x11.key('Return')
                time.sleep(.25)
            next_title = 'شمارهٔ بعدی (پس از نخستین تأیید افزایش می‌یابد)'
            edit(next_title, '0')
            press('s', 'Control_L', page='settings')
            check('Typed invalid numbering does not mutate settings or data', snapshot(profile) == before)
            check('Invalid numbering remains visible for correction', access.text(control(next_title, ['text'])) == '0')
            activate('بازگرداندن')
            check('Native Revert restores saved numbering',
                  access.text(control(next_title, ['text'])) == str(before['settings']['next_number']))
            edit('نام کسب‌وکار', 'Synthetic QA unsaved')
            check('Typed staged business edit remains off disk', snapshot(profile) == before)
            x11.key('F4', 'Alt_L')
            control('ادامهٔ ویرایش', ['button'])
            check('Dirty settings keyboard close keeps process alive', app.poll() is None)
            x11.key('Escape')
            time.sleep(.3)
            check('Keyboard Cancel retains staged business text',
                  access.text(control('نام کسب‌وکار', ['text'])) == 'Synthetic QA unsaved')
            activate('بازگرداندن')
            check('Native Revert restores exact saved state after cancel', snapshot(profile) == before)
            edit('نام کسب‌وکار', 'Synthetic QA saved')
            press('s', 'Control_L', page='settings')
            saved = wait_for(lambda: snapshot(profile) if snapshot(profile)['settings']['business_name']
                             == 'Synthetic QA saved' else None, label='typed business Ctrl+S persisted')
            check('Typed business Ctrl+S saves exact new value and retains approved history',
                  saved['history_sha256'] == before['history_sha256'])
            x11.key('F4', 'Alt_L')
            wait_for(lambda: app.poll() is not None, label='saved settings keyboard close')
            env.update(SARV_CAPTURE_NAME='keyboard-settings-restart.png')
            app = start([str(ROOT / 'target/release/sarv-quote')], 'settings-restart', env)
            window = wait_for(lambda: x11.find(app.pid), label='settings fresh process window')
            wait_for(lambda: (profile / 'keyboard-settings-restart.png').exists(), label='settings restart rendered')
            check('Real typed business save persists through fresh native process', snapshot(profile) == saved)
            x11.focus(window)
            access = Access(app.pid)
            wait_for(lambda: access.tree(), label='restarted owned AT-SPI app')
            press('2', 'Alt_L', page='catalog')
            edit('جست‌وجوی فهرست قیمت و کالا', 'P-101')
            time.sleep(.5)
            (profile / 'a11y-product-tree.json').write_text(json.dumps(access.tree(), ensure_ascii=False, indent=2))
            subprocess.run(['/usr/bin/import', '-display', display, '-window', str(window),
                            str(profile / 'product-search.png')], check=True)
            def ascii_digits(value):
                return value.translate(str.maketrans('۰۱۲۳۴۵۶۷۸۹', '0123456789'))
            # AdwActionRow labels its activatable suffix with the product's
            # title; that relation takes precedence over the button's label.
            add = wait_for(lambda: next((row for row in access.tree() if row['role'] == 'button'
                           and ('P-101' in ascii_digits(row['name']) or row['name'] == 'افزودن')), None),
                           label='searched product native Add')
            old = snapshot(profile)
            tab_to(label='افزودن')
            x11.key('Return')
            state = wait_for(lambda: snapshot(profile) if len(snapshot(profile)['draft']['lines'])
                             == len(old['draft']['lines']) + 1 else None, label='native product addition persisted')
            check('Exact typed product search + native Add appends one source item',
                  state['draft']['lines'][-1] == {'query': 'P-101', 'quantity_milli': 1000, 'chosen_code': 'P-101'})
            check('Product Add clears stale-price acknowledgement', state['draft']['stale_ack'] is False)
            check('Product Add preserves approved history and settings',
                  state['history_sha256'] == old['history_sha256'] and state['settings'] == old['settings'])
            check('Product Add preserves exact imported source prices', state['catalogs_sha256'] == old['catalogs_sha256'])
            press('3', 'Alt_L', page='customers')
            activate('مشتری تازه')
            x11.type_ascii('Synthetic independent QA')
            x11.key('Tab')
            # Contact details were added without changing the legacy phone
            # assertion. Skip the optional contact field when it is present.
            if any('نام مخاطب' in row['name'] for row in access.tree()):
                x11.key('Tab')
            x11.type_ascii('09120000001')
            x11.key('Return')
            added = wait_for(lambda: next((c for c in snapshot(profile)['customers']
                            if c['name'] == 'Synthetic independent QA'), None), label='customer Enter persisted')
            check('Native customer dialog Enter saves exact name and phone', added['phone'] == '09120000001')
            check('Customer creation preserves immutable approved history',
                  snapshot(profile)['history_sha256'] == old['history_sha256'])
            # Build a deliberately synthetic 1,000-line fixture inside this
            # disposable copy only. The normal app validates it when reopening.
            x11.key('F4', 'Alt_L')
            wait_for(lambda: app.poll() is not None, label='fixture setup owned process close')
            expected = [{'query': 'P-101', 'quantity_milli': (i+1)*1000, 'chosen_code': 'P-101'}
                        for i in range(1000)]
            with sqlite3.connect(profile / 'sarv.sqlite3') as connection:
                generation, payload = connection.execute('SELECT generation,payload FROM state WHERE id=1').fetchone()
                fixture = json.loads(payload)
                fixture['draft']['lines'] = expected
                fixture['draft']['stale_ack'] = True
                connection.execute('UPDATE state SET generation=?,payload=? WHERE id=1',
                                   (generation+1, json.dumps(fixture, ensure_ascii=False, separators=(',', ':'))))
            (profile / 'pagination-fixture.json').write_text(json.dumps({'source': 'owned demo synthetic P-101 only',
                'lines': 1000, 'quantity_milli': '(global_index+1)*1000'}, indent=2)+'\n')
            env.update(SARV_CAPTURE_NAME='pagination-start.png')
            app = start([str(ROOT / 'target/release/sarv-quote')], 'pagination', env)
            window = wait_for(lambda: x11.find(app.pid), label='validated synthetic paged editor process')
            wait_for(lambda: (profile / 'pagination-start.png').exists(), label='1,000-line editor native render')
            x11.focus(window)
            press('Tab', focus='row0-quantity')
            for index in range(1, 100):
                press('Down', 'Control_L', focus=f'row{index}-quantity')
            x11.key('a', 'Control_L')
            x11.type_ascii('2.375')
            press('Down', 'Control_L', focus='row100-quantity')
            expected[99]['quantity_milli'] = 2375
            check('Real Ctrl+Down crosses row 100/101 page boundary and saves first-page edit',
                  snapshot(profile)['draft']['lines'] == expected)
            x11.key('a', 'Control_L')
            x11.type_ascii('0')
            press('Up', 'Control_L', focus='row100-quantity')
            check('Invalid cell blocks real Ctrl+Up page crossing and preserves all canonical rows',
                  snapshot(profile)['draft']['lines'] == expected)
            x11.key('a', 'Control_L')
            x11.type_ascii('3.125')
            press('Up', 'Control_L', focus='row99-quantity')
            expected[100]['quantity_milli'] = 3125
            check('Corrected second-page edit saves to global row 101 without changing hidden rows',
                  snapshot(profile)['draft']['lines'] == expected)
            press('s', 'Control_L', focus='row99-quantity')
            check('Paged Ctrl+S retains global focus and all 1,000 exact canonical rows',
                  snapshot(profile)['draft']['lines'] == expected)
            check('Paged editing preserves immutable approved history and source prices',
                  snapshot(profile)['history_sha256'] == saved['history_sha256']
                  and snapshot(profile)['catalogs_sha256'] == saved['catalogs_sha256'])
            time.sleep(.5)
            visible = press('Shift_L', focus='row99-quantity')['event']
            bounds, clip = visible.get('focus_bounds'), visible.get('focus_clip_bounds')
            size = visible.get('window_size')
            def enclosed(inner, outer):
                return (inner and outer and inner[2] > 0 and inner[3] > 0
                        and inner[0] >= outer[0]-1 and inner[1] >= outer[1]-1
                        and inner[0]+inner[2] <= outer[0]+outer[2]+1
                        and inner[1]+inner[3] <= outer[1]+outer[3]+1)
            check('Rendered row 100 focus stays completely inside table clip and actual window after page/save',
                  enclosed(bounds, clip) and size and enclosed(bounds, [0, 0, *size]), event=visible)
            subprocess.run(['/usr/bin/import', '-display', display, '-window', str(window),
                            str(profile / 'pagination-real-key-boundary.png')], check=True)
        report['result'] = 'passed' if all(item['passed'] for item in report['checks']) else 'failed'
    except Exception as error:
        report['error'] = str(error)
        raise
    finally:
        if x11:
            x11.x.XCloseDisplay(x11.d)
        for process in reversed(processes):
            stop(process)
        for log in logs:
            log.close()
        (profile / 'independent-verification.json').write_text(json.dumps(report, ensure_ascii=False, indent=2)+'\n')
        (output / 'latest-profile.txt').write_text(str(profile)+'\n')
        print(json.dumps({'result': report['result'], 'checks': len(report['checks']),
                          'profile': str(profile)}, ensure_ascii=False), flush=True)
    if report['result'] != 'passed':
        raise SystemExit(1)


if __name__ == '__main__':
    main()
