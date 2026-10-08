#!/usr/bin/env python3
"""Measure an owned Sarv release process in a private X11/D-Bus session.

Source /workspace/sarv-environment/activate.sh, then run under dbus-run-session.
Only the explicitly allowlisted synthetic evidence fixtures may be cloned.
Copies use SQLite's consistent backup API. No builds or source/live data edits.
"""
import argparse
import ctypes as C
import hashlib
import json
import math
import os
from pathlib import Path
import sqlite3
import subprocess
import tempfile
import threading
import time

from qa_access import Access
from importlib.util import spec_from_file_location, module_from_spec

ROOT = Path(__file__).resolve().parents[1]
spec = spec_from_file_location('native', ROOT / 'scripts/qa-native.py')
native = module_from_spec(spec)
spec.loader.exec_module(native)
ENVROOT = Path('/workspace/sarv-environment')
SOURCES = {
    'g1-small': Path('/workspace/sarv-evidence/independent-ledger-final/profile'),
    'quote-scale': Path('/workspace/sarv-evidence/native-before-current'),
}


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def payload(profile):
    with sqlite3.connect(f'file:{profile}/sarv.sqlite3?mode=ro', uri=True) as db:
        generation, serialized = db.execute('SELECT generation,payload FROM state WHERE id=1').fetchone()
    return generation, serialized


def shape(profile):
    generation, serialized = payload(profile)
    state = json.loads(serialized)
    return {'schema': state['schema'], 'generation': generation,
            'payload_sha256': hashlib.sha256(serialized.encode()).hexdigest(),
            'customers': len(state['customers']), 'quotes': len(state['quotes']),
            'products': sum(len(c['products']) for c in state['catalogs']),
            'draft_lines': len(state['draft']['lines']),
            'accounting': {key: len(value) for key, value in state.get('accounting', {}).items()
                           if isinstance(value, list)}}


def stats(values):
    values = sorted(values)
    def quantile(p):
        return values[max(0, math.ceil(p * len(values)) - 1)]
    return {'n': len(values), 'p50': quantile(.5), 'p95': quantile(.95),
            'p99': quantile(.99), 'worst': max(values), 'method': 'nearest rank'}


def command(*args):
    return subprocess.check_output(args, text=True).strip()


def source_hash():
    files = sorted([*ROOT.glob('src/**/*.rs'), ROOT / 'src/style.css', ROOT / 'Cargo.toml', ROOT / 'Cargo.lock'])
    digest = hashlib.sha256()
    for path in files:
        digest.update(str(path.relative_to(ROOT)).encode() + b'\0' + path.read_bytes() + b'\0')
    return digest.hexdigest()


def rss(pid):
    try:
        fields = dict(line.split(':', 1) for line in Path(f'/proc/{pid}/status').read_text().splitlines() if ':' in line)
        return {key: int(fields[key].split()[0]) for key in ('VmRSS', 'VmHWM')}
    except (FileNotFoundError, KeyError):
        return None


def first_accessible(access, predicate, root=None):
    """Stop at the requested control; avoid enumerating unrelated row widgets."""
    app = root or access.app()
    if not app:
        return None
    from collections import deque
    queue = deque([app])
    for _ in range(8000):
        if not queue:
            break
        node = queue.popleft()
        if not node:
            continue
        access.a.atspi_accessible_clear_cache(node)
        row = {'node': node}
        if predicate(row):
            return row
        queue.extend(access.children(node))
    return None


def find_name(access, name, root=None):
    def matches(row):
        row['name'] = access.string(access.a.atspi_accessible_get_name, row['node'], None)
        if row['name'] != name:
            return False
        row['role'] = access.string(access.a.atspi_accessible_get_role_name, row['node'], None)
        return True
    return first_accessible(access, matches, root)


def find_focused(access, exclude=None):
    return first_accessible(access, lambda row: row['node'] != exclude and access.focused(row))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/sarv-quote')
    parser.add_argument('--expect-sha', required=True)
    parser.add_argument('--samples', type=int, default=10)
    parser.add_argument('--search-samples', type=int, default=20)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    assert args.samples >= 10 and args.search_samples >= 10
    assert sha(args.binary) == args.expect_sha, 'Wrong release binary'
    assert os.environ.get('DBUS_SESSION_BUS_ADDRESS', '').startswith('unix:path=/tmp/dbus-'), 'Use private dbus-run-session'
    output = args.output.resolve()
    assert output.is_relative_to(Path('/workspace/sarv-evidence')) or output.is_relative_to(ROOT / 'artifacts')
    output.mkdir(parents=True, exist_ok=False)
    logs, processes = [], []
    report = {'result': 'incomplete', 'binary': str(args.binary), 'binary_sha256': args.expect_sha,
              'source_head': command('git', '-C', str(ROOT), 'rev-parse', 'HEAD'),
              'source_status': command('git', '-C', str(ROOT), 'status', '--short'),
              'source_code_sha256': source_hash(), 'utc': command('date', '-u', '+%Y-%m-%dT%H:%M:%SZ'),
              'synthetic_only': True, 'datasets': {},
              'hardware': {'lscpu': command('lscpu'), 'os': Path('/etc/os-release').read_text(),
                           'filesystem': command('df', '-T', '/workspace'),
                           'meminfo': Path('/proc/meminfo').read_text(),
                           'cgroup_memory_max': Path('/sys/fs/cgroup/memory.max').read_text().strip(),
                           'screen': '1450x1050x24 Xvfb, scale 1, X11, Cairo, Openbox',
                           'rust': command('rustc', '--version'),
                           'gtk_libadwaita': command('pkg-config', '--modversion', 'gtk4', 'libadwaita-1'),
                           'database_source_configuration': 'SQLite WAL; synchronous=FULL; busy timeout 3s (src/store.rs)'} ,
              'boundaries': {
                  'startup': 'Popen dispatch to owned mapped GTK window, visible editor save control, real XTest Tab focus roundtrip, and X11 screenshot completion. Screenshot acquisition/AT-SPI polling observer overhead included.',
                  'first_on_clone': 'First process on freshly backed-up synthetic database; schema migration may be included. OS page cache uncontrolled.',
                  'repeated_startup': 'New process each sample, same initialized profile; OS page cache warm/uncontrolled, AT-SPI session preinitialized. Not machine-cold.',
                  'search': 'AT-SPI text replacement dispatch to exact expected product title in a visible selectable and sensitive list item; includes debounce, rendering, AT-SPI observer overhead.',
                  'rss': 'Owned application /proc VmRSS and kernel VmHWM in KiB, polled every 10ms from process launch. Steady after 500ms idle; peak excludes X server, accessibility registry and helpers.'},
              'limits': ['No OS-cache flush, physical cold launch, SSD hardware declaration, or reference four-core/8GB certification. Binary hashing and SQLite copying warm their file pages before launch; library/font cache state is uncontrolled.',
                         'No five-year/100,000-document/1,000,000-journal-line/10,000-party fixture; small G1 and existing 50,000-product/1,000-line quote only.',
                         'External AT-SPI polling/screenshot observer affects latency; p99 for n=10 is simply the worst sample.',
                         'No report/posting/import/Wayland/package/printer benchmark in this harness.']}
    def start(argv, name, env=None, **kwargs):
        log = (output / f'{name}.log').open('w')
        logs.append(log)
        proc = subprocess.Popen(argv, env=env, stdout=log, stderr=log, start_new_session=True, **kwargs)
        processes.append(proc)
        return proc
    try:
        with tempfile.TemporaryFile() as display_file:
            start(['Xvfb', '-displayfd', str(display_file.fileno()), '-screen', '0', '1450x1050x24',
                   '-nolisten', 'tcp', '-xkbdir', str(ENVROOT / 'sysroot/usr/share/X11/xkb')], 'xvfb', pass_fds=(display_file.fileno(),))
            def display_number():
                display_file.seek(0)
                return display_file.read().decode().strip()
            display = ':' + native.wait_for(display_number)
        runtime = native.private_accessibility_environment(display, output)
        env = os.environ.copy()
        env.update(DISPLAY=display, GDK_BACKEND='x11', GSK_RENDERER='cairo', XDG_RUNTIME_DIR=str(runtime), SARV_TEST_INSTANCE='performance-owned')
        for key in ('SARV_UI_SMOKE', 'SARV_ACCOUNTING_SMOKE', 'SARV_RESPONSIVENESS', 'SARV_NATIVE_RESPONSIVENESS', 'SARV_SMOKE', 'SARV_SEED_BACKUP', 'SARV_CAPTURE_PAGE', 'SARV_FOCUS_ROW'):
            env.pop(key, None)
        start(['openbox', '--sm-disable', '--config-file', str(ENVROOT / 'sysroot/etc/xdg/openbox/rc.xml')], 'openbox', env)
        x11 = native.X11(display)
        Access(0)  # Initialize observer/session before launch, no app targeted.
        for name, source in SOURCES.items():
            if name == 'quote-scale':
                proof = json.loads((source / 'native-responsiveness-verification.json').read_text())
                assert proof['synthetic_only'] and proof['result'] == 'passed'
            else:
                assert 'شرکت مصنوعی بازبین' in payload(source)[1], 'Missing synthetic G1 marker'
            profile = output / name
            profile.mkdir()
            with sqlite3.connect(f'file:{source}/sarv.sqlite3?mode=ro', uri=True) as src:
                with sqlite3.connect(profile / 'sarv.sqlite3') as dst:
                    src.backup(dst)
            dataset = {'source': str(source), 'source_shape': shape(source), 'samples': []}
            report['datasets'][name] = dataset
            env['SARV_DATA_DIR'] = str(profile)
            for index in range(args.samples + 1):
                assert sha(args.binary) == args.expect_sha, 'Release changed during measurement'
                begin = time.perf_counter()
                app = start([str(args.binary)], f'{name}-{index}', env)
                stop_sampler = threading.Event()
                observed = []
                def monitor():
                    while not stop_sampler.is_set():
                        value = rss(app.pid)
                        if value:
                            observed.append(value)
                        stop_sampler.wait(.01)
                sampler = threading.Thread(target=monitor, daemon=True)
                sampler.start()
                try:
                    window = native.wait_for(lambda: x11.find(app.pid), label='owned window')
                    access = Access(app.pid)
                    control = native.wait_for(lambda: find_name(access, 'ذخیرهٔ پیش‌نویس'), label='usable editor save control')
                    x11.focus(window)
                    focused_before = native.wait_for(lambda: find_focused(access), label='initial focused editor control')['node']
                    x11.key('Tab')
                    native.wait_for(lambda: find_focused(access, focused_before), label='native Tab focus roundtrip')
                    screenshot = profile / f'startup-{index}.png'
                    subprocess.run(['/usr/bin/import', '-display', display, '-window', str(window), str(screenshot)], check=True)
                    duration = (time.perf_counter() - begin) * 1000
                    time.sleep(.5)
                    steady = rss(app.pid)
                    sample = {'index': index, 'startup_ms': duration, 'steady_rss_kib': steady['VmRSS'],
                              'kernel_peak_rss_kib': steady['VmHWM'], 'rss_observer_samples': len(observed), 'screenshot': str(screenshot)}
                    dataset['samples'].append(sample)
                    if index == 0:
                        dataset['initialized_shape'] = shape(profile)
                        baseline_payload = payload(profile)
                    if name == 'quote-scale' and index == args.samples:
                        x11.key('k', 'Control_L')
                        search = native.wait_for(lambda: find_name(access, 'جست‌وجوی نام، کد یا نام مستعار کالا'), label='native product picker')
                        picker_root = access.a.atspi_accessible_get_parent(search['node'], None)
                        native.wait_for(lambda: first_accessible(access, lambda row: 'نمایش حداکثر' in access.string(access.a.atspi_accessible_get_name, row['node'], None), picker_root), label='index ready')
                        (profile / 'picker-search-node.json').write_text(json.dumps({k:v for k,v in search.items() if k != 'node'}, ensure_ascii=False, indent=2))
                        a = access.a
                        a.atspi_accessible_get_editable_text_iface.argtypes = [C.c_void_p]
                        a.atspi_accessible_get_editable_text_iface.restype = C.c_void_p
                        a.atspi_editable_text_set_text_contents.argtypes = [C.c_void_p, C.c_char_p, C.c_void_p]
                        a.atspi_editable_text_set_text_contents.restype = C.c_int
                        edit = a.atspi_accessible_get_editable_text_iface(search['node'])
                        assert edit
                        state = json.loads(payload(profile)[1])
                        products = state['catalogs'][0]['products']
                        searches = []
                        for q in range(args.search_samples):
                            product = products[-1] if q % 2 == 0 else products[-2]
                            start_time = time.perf_counter()
                            assert a.atspi_editable_text_set_text_contents(edit, product['code'].encode(), None)
                            def correct_result():
                                row = find_name(access, product['name'], picker_root)
                                if not row or row['role'] != 'list item':
                                    return None
                                rect = access.rect(row)
                                states = access.a.atspi_accessible_get_state_set(row['node'])
                                # AT-SPI STATE_SELECTABLE=22, STATE_SENSITIVE=24.
                                selectable = states and access.a.atspi_state_set_contains(states, 22)
                                sensitive = states and access.a.atspi_state_set_contains(states, 24)
                                showing = states and access.a.atspi_state_set_contains(states, 25)
                                visible = states and access.a.atspi_state_set_contains(states, 30)
                                return row if rect and rect['width'] > 0 and rect['height'] > 0 and selectable and sensitive and showing and visible else None
                            native.wait_for(correct_result, label='correct selectable product')
                            searches.append({'query': product['code'], 'expected_title': product['name'], 'duration_ms': (time.perf_counter()-start_time)*1000})
                        dataset['indexed_search_samples'] = searches
                        dataset['indexed_search_ms'] = stats([sample['duration_ms'] for sample in searches])
                        dataset['steady_rss_picker_kib'] = rss(app.pid)
                        subprocess.run(['/usr/bin/import', '-display', display, '-window', str(window), str(profile/'picker-search.png')], check=True)
                        x11.key('Escape')
                finally:
                    stop_sampler.set()
                    sampler.join()
                    native.stop(app)
                assert payload(profile) == baseline_payload, 'Read-only measurement altered initialized state'
            measured = dataset['samples'][1:]
            dataset['repeated_process_startup_ms'] = stats([s['startup_ms'] for s in measured])
            dataset['steady_rss_kib'] = stats([s['steady_rss_kib'] for s in measured])
            dataset['peak_rss_kib'] = stats([s['kernel_peak_rss_kib'] for s in measured])
            dataset['final_shape'] = shape(profile)
            print(json.dumps({'dataset':name, 'startup':dataset['repeated_process_startup_ms'], 'rss':dataset['steady_rss_kib']}, ensure_ascii=False), flush=True)
        report['result'] = 'measured'
    except Exception as error:
        report['error'] = repr(error)
        raise
    finally:
        report['source_code_sha256_after'] = source_hash()
        report['binary_sha256_after'] = sha(args.binary)
        (output / 'performance.json').write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
        for process in reversed(processes):
            native.stop(process)
        for log in logs:
            log.close()


if __name__ == '__main__':
    main()
