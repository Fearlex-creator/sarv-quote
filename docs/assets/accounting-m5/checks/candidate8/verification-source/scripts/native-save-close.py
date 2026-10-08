"""Synthetic X11 save/close regression. Requires native runtime in PATH.

Supply SARV_CLOSE_TEST_DIR (a never-used output directory) and
SARV_CLOSE_TEST_BINARY (the release executable). No existing profile is opened.
"""
import ctypes as C, json, os, pathlib, sqlite3, subprocess, time, tempfile
profile = pathlib.Path(os.environ['SARV_CLOSE_TEST_DIR']).resolve()
profile.mkdir(exist_ok=False)
env = {key: value for key, value in os.environ.items() if not key.startswith('SARV_')}
env.update(GDK_BACKEND='x11', GSK_RENDERER='cairo', SARV_DATA_DIR=str(profile))
display_file = tempfile.TemporaryFile()
xvfb = subprocess.Popen(['Xvfb', '-displayfd', str(display_file.fileno()), '-screen', '0', '1400x1000x24', '-nolisten', 'tcp'], env=env, pass_fds=(display_file.fileno(),), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
wm = app = None
try:
    for _ in range(100):
        display_file.seek(0)
        number = display_file.read().strip()
        if number:
            break
        if xvfb.poll() is not None:
            raise RuntimeError('Xvfb exited before readiness')
        time.sleep(0.02)
    assert number, 'Xvfb did not allocate a display'
    env['DISPLAY'] = ':' + number.decode()
    wm = subprocess.Popen(['openbox', '--sm-disable'], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    log = open(profile / 'native-close.log', 'w')
    app = subprocess.Popen(['dbus-run-session', '--', str(pathlib.Path(os.environ['SARV_CLOSE_TEST_BINARY']).resolve())], env=env, stdout=log, stderr=log)
    limit = time.monotonic() + 10
    while not (profile / 'sarv.sqlite3').exists() and time.monotonic() < limit:
        time.sleep(0.02)
    time.sleep(0.8)
    connection = sqlite3.connect(profile / 'sarv.sqlite3', timeout=0.1)
    assert connection.execute('select generation from state').fetchone()[0] == 0
    connection.execute('BEGIN IMMEDIATE')
    x = C.CDLL('libX11.so.6')
    t = C.CDLL('libXtst.so.6')
    x.XOpenDisplay.argtypes = [C.c_char_p]
    x.XOpenDisplay.restype = C.c_void_p
    x.XStringToKeysym.argtypes = [C.c_char_p]
    x.XStringToKeysym.restype = C.c_ulong
    x.XKeysymToKeycode.argtypes = [C.c_void_p, C.c_ulong]
    x.XKeysymToKeycode.restype = C.c_uint
    x.XFlush.argtypes = [C.c_void_p]
    x.XCloseDisplay.argtypes = [C.c_void_p]
    t.XTestFakeKeyEvent.argtypes = [C.c_void_p, C.c_uint, C.c_int, C.c_ulong]
    display = x.XOpenDisplay(env['DISPLAY'].encode())
    assert display

    def key(name, down):
        code = x.XKeysymToKeycode(display, x.XStringToKeysym(name.encode()))
        assert code
        t.XTestFakeKeyEvent(display, code, down, 0)
    for character in 'close-synthetic':
        key('minus' if character == '-' else character, 1)
        key('minus' if character == '-' else character, 0)
    x.XFlush(display)
    time.sleep(0.05)
    key('Alt_L', 1)
    key('F4', 1)
    key('F4', 0)
    key('Alt_L', 0)
    x.XFlush(display)
    time.sleep(0.2)
    assert app.poll() is None, 'window closed before locked SQLite save could commit'
    connection.rollback()
    connection.close()
    assert app.wait(timeout=10) == 0
    payload, generation = sqlite3.connect(profile / 'sarv.sqlite3').execute('select payload,generation from state').fetchone()
    data = json.loads(payload)
    assert data['draft']['customer'] == 'close-synthetic', data['draft']['customer']
    assert generation == 1, generation
    report = {'result': 'passed', 'synthetic_only': True, 'native_input': 'X11 XTest text followed by Alt+F4', 'checks': ['Window remains open while detached save waits for actual SQLite write lock', 'Close exits successfully only after durable commit', 'Reopened SQLite retains exact typed customer and one generation'], 'limits': ['Process restart proof; hardware power loss untested', 'No Wayland coverage']}
    (profile / 'native-close-verification.json').write_text(json.dumps(report, indent=2))
    print(json.dumps(report))
    x.XCloseDisplay(display)
finally:
    if app and app.poll() is None:
        app.terminate()
    if wm:
        wm.terminate()
    xvfb.terminate()
