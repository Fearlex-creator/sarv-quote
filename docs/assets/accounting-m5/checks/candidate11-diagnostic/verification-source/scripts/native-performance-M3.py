#!/usr/bin/env python3
"""Observe an immutable M3 binary using fresh synthetic profiles only.

Activate /workspace/sarv-environment/activate.sh and use dbus-run-session.
No builds, source edits, cache flushes or regular desktop profiles are performed.
The existing M2 scale harness is a separate, unchanged invocation.
"""
import argparse
import ctypes as C
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import tempfile
import threading
import time
from importlib.util import module_from_spec, spec_from_file_location

from qa_access import Access

ROOT = Path(__file__).resolve().parents[1]
SPEC = spec_from_file_location('m2_observer_helpers', ROOT / 'scripts/accounting-performance.py')
helper = module_from_spec(SPEC)
SPEC.loader.exec_module(helper)
native = helper.native
QUOTE_SOURCE = Path('/workspace/sarv-evidence/native-before-current')


def ready(access, row, focus=False):
    if not row:
        return False
    access.a.atspi_accessible_clear_cache(row['node'])
    states = access.a.atspi_accessible_get_state_set(row['node'])
    rect = access.rect(row)
    # AT-SPI sensitive=24, showing=25, visible=30, focused=12.
    return bool(rect and rect['width'] > 0 and rect['height'] > 0 and states
                and all(access.a.atspi_state_set_contains(states, s) for s in (24, 25, 30))
                and (not focus or access.focused(row)))


def sensitive(access, row):
    access.a.atspi_accessible_clear_cache(row['node'])
    states = access.a.atspi_accessible_get_state_set(row['node'])
    return bool(states and access.a.atspi_state_set_contains(states, 24))


def observe(predicate, label, seconds=15):
    """Bounded5ms observer poll; discovery is done before timed dispatch."""
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        result = predicate()
        if result:
            return result
        time.sleep(.005)
    raise RuntimeError(f'Timed out waiting for {label}')


def button(access, name):
    return helper.first_accessible(access, lambda row:
        access.string(access.a.atspi_accessible_get_name, row['node'], None) == name
        and access.string(access.a.atspi_accessible_get_role_name, row['node'], None) in ('button', 'push button'))


def keyboard_focus(access, row, x11):
    """Use real Tab when GTK rejects the component focus request."""
    if access.focus(row) and access.focused(row):
        return
    for _ in range(160):
        if access.focused(row):
            return
        x11.key('Tab')
        time.sleep(.01)
    raise RuntimeError('Native Tab did not focus the pre-discovered search control')


def text_control(access, name):
    return helper.first_accessible(access, lambda row:
        access.string(access.a.atspi_accessible_get_name, row['node'], None) == name
        and access.string(access.a.atspi_accessible_get_role_name, row['node'], None) in ('entry', 'text'))


def phases(profile):
    path = profile / 'core-performance-phases.jsonl'
    if not path.exists():
        return []
    return [json.loads(line) for line in path.read_text().splitlines() if line.endswith('}')]


def copy_database(source, destination):
    destination.mkdir()
    with sqlite3.connect(f'file:{source}/sarv.sqlite3?mode=ro', uri=True) as src:
        with sqlite3.connect(destination / 'sarv.sqlite3') as dst:
            src.backup(dst)


def core_shape(profile):
    state = json.loads(helper.payload(profile)[1])
    accounting = state.get('accounting', {})
    core = accounting.get('core', {})
    result = helper.shape(profile)
    result['core'] = {key: len(value) for key, value in core.items() if isinstance(value, list)}
    if core.get('openings'):
        opening = core['openings'][0]
        command = opening['command']
        result['opening'] = {
            'counts': {key: len(command[key]) for key in ('balances', 'parties', 'units', 'items',
                        'warehouses', 'cash', 'banks', 'open_items', 'advances', 'cheques', 'stock', 'grni')},
            'packs': sum(len(item['packs']) for item in command['items']),
            'import_source': command.get('import_source'),
            'command_sha256': opening['command_sha256'],
            'source_id': command['source_id'], 'journal_id': opening['journal_id'],
            'as_of': command['as_of'],
            'ledger_debit_irr': sum(row['debit'] for row in command['balances']),
            'ledger_credit_irr': sum(row['credit'] for row in command['balances']),
            'stock_value_irr': sum(row['value'] for row in command['stock']),
            'grni_value_irr': sum(row['value'] for row in command['grni']),
        }
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--expect-sha', required=True)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--samples', type=int, default=10)
    parser.add_argument('--search-samples', type=int, default=20)
    parser.add_argument('--pilot', action='store_true', help='Development observer check only: n1 startup/n2 search, not acceptance evidence')
    args = parser.parse_args()
    if args.pilot:
        args.samples, args.search_samples = 1, 2
    else:
        assert args.samples >= 10 and args.search_samples >= 20
    binary = args.binary.resolve()
    assert helper.sha(binary) == args.expect_sha, 'Wrong immutable executable'
    assert os.environ.get('DBUS_SESSION_BUS_ADDRESS', '').startswith('unix:path=/tmp/dbus-'), 'Use private dbus-run-session'
    output = args.output.resolve()
    assert output.is_relative_to(ROOT / 'artifacts/performance-m3'), 'Output must be a fresh ignored performance-m3 directory'
    output.mkdir(parents=True, exist_ok=False)
    processes, logs = [], []
    report = {
        'result': 'incomplete', 'synthetic_only': True, 'development_pilot': args.pilot, 'binary': str(binary),
        'binary_sha256': args.expect_sha, 'utc': helper.command('date', '-u', '+%Y-%m-%dT%H:%M:%SZ'),
        'source_head': helper.command('git', '-C', str(ROOT), 'rev-parse', 'HEAD'),
        'source_status': helper.command('git', '-C', str(ROOT), 'status', '--short'),
        'source_code_sha256': helper.source_hash(), 'datasets': {},
        'hardware': {
            'lscpu': helper.command('lscpu'), 'os': Path('/etc/os-release').read_text(),
            'filesystem': helper.command('df', '-T', '/workspace'),
            'meminfo': Path('/proc/meminfo').read_text(),
            'cgroup_memory_max': Path('/sys/fs/cgroup/memory.max').read_text().strip(),
            'cgroup_cpu_max': Path('/sys/fs/cgroup/cpu.max').read_text().strip(),
            'screen': '1450x1050x24 private Xvfb/Openbox, X11, Cairo, scale 1',
            'rust': helper.command('rustc', '--version'),
            'gtk_libadwaita': helper.command('pkg-config', '--modversion', 'gtk4', 'libadwaita-1'),
        },
        'boundaries': {
            'startup': 'Popen to owned mapped window, usable visible editor Save, real XTest Tab focus roundtrip and completed PNG capture; includes AT-SPI polling and screenshot observer overhead.',
            'first_start': 'One new process on a fresh SQLite backup, initialization/migration may be included; OS cache uncontrolled.',
            'repeated_startup': 'n new processes on the same initialized backup; OS pages warm/uncontrolled, accessibility session initialized. Process-cold, not physical/cache-cold.',
            'core_search': 'Pre-discovery of search/list excluded. Real XTest Ctrl+A and ASCII keys to exact current ID label, then native Tab to focused, visible, sensitive Edit action. Includes debounce, rendering,5ms AT-SPI polling and result-focus navigation; no pure UI latency claim. Correct-visible-action boundary is recorded separately.',
            'preview': 'Real XTest Space on already-discovered focused preview button to enabled post button, after source reason mutation invalidates old verification. Includes accessibility polling; n=1.',
            'post': 'Real XTest Space on already-discovered focused post button to committed SQLite payload containing one opening and one journal; includes durable database observer polling; n=1.',
            'rss': 'Application-only /proc/PID/status KiB; startup sampled every10ms and steady after500ms idle. After-workflow separate point after posting. Helpers excluded.',
        },
        'limits': [
            'No OS cache flush, physical SSD evidence, dedicated reference hardware, Windows or Wayland claim.',
            'Virtual shared executor scheduling and font/library cache state are uncontrolled; hashing and backup warm file pages.',
            'Native forms fixture is distinct from the35-row CSV fixture. No CSV import claim when import_source is absent.',
            'n10/n20 nearest-rank p99 equals worst; n1 preview/post and workflow memory are not tail estimates.',
            'No five-year/100000-document/million-journal-line/10000-party or10000-row staged-import benchmark.',
        ],
    }

    def start(command, name, env=None, **kwargs):
        log = (output / f'{name}.log').open('w')
        logs.append(log)
        process = subprocess.Popen(command, env=env, stdout=log, stderr=log,
                                   start_new_session=True, **kwargs)
        processes.append(process)
        return process

    def capture(window, path):
        subprocess.run(['/usr/bin/import', '-display', display, '-window', str(window), str(path)], check=True)

    def editable(access, row, text):
        a = access.a
        a.atspi_accessible_get_editable_text_iface.argtypes = [C.c_void_p]
        a.atspi_accessible_get_editable_text_iface.restype = C.c_void_p
        a.atspi_editable_text_set_text_contents.argtypes = [C.c_void_p, C.c_char_p, C.c_void_p]
        a.atspi_editable_text_set_text_contents.restype = C.c_int
        iface = a.atspi_accessible_get_editable_text_iface(row['node'])
        assert iface and a.atspi_editable_text_set_text_contents(iface, text.encode(), None)

    try:
        with tempfile.TemporaryFile() as fd:
            start(['Xvfb', '-displayfd', str(fd.fileno()), '-screen', '0', '1450x1050x24',
                   '-nolisten', 'tcp', '-xkbdir', str(helper.ENVROOT / 'sysroot/usr/share/X11/xkb')],
                  'xvfb', pass_fds=(fd.fileno(),))
            def number():
                fd.seek(0)
                return fd.read().decode().strip()
            display = ':' + native.wait_for(number, label='private X11 display')
        runtime = native.private_accessibility_environment(display, output)
        env = {key: value for key, value in os.environ.items() if not key.startswith('SARV_')}
        env.update(DISPLAY=display, GDK_BACKEND='x11', GSK_RENDERER='cairo',
                   XDG_RUNTIME_DIR=str(runtime), SARV_TEST_INSTANCE='m3-performance-owned',
                   SARV_PERFORMANCE_TRACE='1', SARV_CORE_PERFORMANCE_TRACE='1')
        start(['openbox', '--sm-disable', '--config-file',
               str(helper.ENVROOT / 'sysroot/etc/xdg/openbox/rc.xml')], 'openbox', env)
        x11 = native.X11(display)
        Access(0)

        # This opt-in application probe uses native editable forms on a fresh profile.
        workflow = output / 'core-workflow'
        workflow.mkdir()
        env.update(SARV_DATA_DIR=str(workflow), SARV_CORE_SMOKE='stage')
        app = start([str(binary)], 'core-stage', env)
        marker = workflow / 'core-stage-ready.json'
        native.wait_for(lambda: marker.exists() or (workflow / 'core-native-failure.txt').exists(),
                        seconds=110, label='native staged core ready')
        assert marker.exists(), 'Native staging failed; inspect core-native-failure.txt'
        staged = json.loads(marker.read_text())
        assert staged['result'] == 'ready' and staged['preview_total_irr'] == 1251
        report['core_workflow'] = {'stage_manifest': staged}
        baseline = helper.payload(workflow)
        access = Access(app.pid)
        window = native.wait_for(lambda: x11.find(app.pid), label='owned staged window')
        x11.focus(window)
        search = native.wait_for(lambda: helper.find_name(access, 'جستجوی سطرهای جزئیات'), label='core search')
        (workflow / 'staged-accessibility.json').write_text(json.dumps(
            [{key: value for key, value in row.items() if key != 'node'} for row in access.tree()], ensure_ascii=False, indent=2) + '\n')
        native.wait_for(lambda: ready(access, search), label='allocated visible search after stage transition')
        list_heading = native.wait_for(lambda: helper.first_accessible(access, lambda row:
            'فقط پیش‌نویس موقت' in access.string(access.a.atspi_accessible_get_name, row['node'], None)), label='staged list root')
        list_root = access.a.atspi_accessible_get_parent(list_heading['node'], None)
        samples = []
        for index in range(args.search_samples):
            query = ('CU-A', 'CU-B')[index % 2]
            keyboard_focus(access, search, x11)
            native.wait_for(lambda: access.focused(search), label='search focused before timing')
            begin = time.perf_counter()
            x11.key('a', 'Control_L')
            x11.type_ascii(query)
            dispatch_ms = (time.perf_counter() - begin) * 1000
            def result():
                if access.text(search) != query:
                    return None
                filtered = helper.first_accessible(access, lambda candidate:
                    access.string(access.a.atspi_accessible_get_name, candidate['node'], None).startswith('1 سطر • '), list_root)
                if not filtered:
                    return None
                row = helper.first_accessible(access, lambda candidate:
                    access.string(access.a.atspi_accessible_get_name, candidate['node'], None).startswith(query + ' • '), list_root)
                if not row:
                    return None
                card = access.a.atspi_accessible_get_parent(row['node'], None)
                edit = helper.find_name(access, 'ویرایش سطر', card)
                if edit and not ready(access, edit):
                    access.focus(edit)
                return edit if edit and ready(access, edit) else None
            edit = observe(result, label='correct staged row Edit action')
            observed_ms = (time.perf_counter() - begin) * 1000
            keyboard_focus(access, edit, x11)
            observe(lambda: ready(access, edit, focus=True), label='result edit focus usable')
            samples.append({'index': index, 'query': query, 'key_dispatch_ms': dispatch_ms,
                            'correct_visible_action_ms': observed_ms,
                            'focused_usable_action_ms': (time.perf_counter() - begin) * 1000})
            assert helper.payload(workflow) == baseline, 'Staged lookup wrote financial state'
        report['core_workflow']['search_samples'] = samples
        report['core_workflow']['search_ms'] = helper.stats([s['focused_usable_action_ms'] for s in samples])
        report['core_workflow']['correct_visible_action_ms'] = helper.stats([s['correct_visible_action_ms'] for s in samples])
        report['core_workflow']['search_memory_kib'] = helper.rss(app.pid)
        capture(window, workflow / 'staged-search.png')

        tab = native.wait_for(lambda: helper.find_name(access, 'بازبینی'), label='core Review tab')
        assert access.click(tab)
        preview = native.wait_for(lambda: button(access, 'بررسی تراز و تطبیق کنترل‌ها'), label='preview')
        post = native.wait_for(lambda: button(access, 'ثبت افتتاحیهٔ تطبیق‌شده'), label='post')
        reason = native.wait_for(lambda: text_control(access, 'توضیح منشأ مانده‌ها'), label='source reason')
        # A changed reason prevents an old successful preview from satisfying readiness.
        editable(access, reason, access.text(reason) + ' / synthetic performance observer')
        native.wait_for(lambda: not sensitive(access, post), label='old preview invalidated')
        keyboard_focus(access, preview, x11)
        native.wait_for(lambda: ready(access, preview, focus=True), label='preview focused')
        phase_baseline_count = len(phases(workflow))
        begin = time.perf_counter()
        x11.key('space')
        observe(lambda: sensitive(access, post), label='new preview verified')
        report['core_workflow']['preview_observed_ms'] = (time.perf_counter() - begin) * 1000
        assert helper.payload(workflow) == baseline, 'Preview mutated durable state'
        capture(window, workflow / 'preview-ready.png')
        keyboard_focus(access, post, x11)
        native.wait_for(lambda: ready(access, post, focus=True), label='post focused')
        begin = time.perf_counter()
        x11.key('space')
        def durable():
            state = json.loads(helper.payload(workflow)[1])
            accounting = state['accounting']
            return len(accounting['core']['openings']) == 1 and len(accounting['journals']) == 1
        observe(durable, label='one durable opening and journal')
        report['core_workflow']['post_durable_observed_ms'] = (time.perf_counter() - begin) * 1000
        time.sleep(.5)
        if not args.pilot:
            observe(lambda: any(record['phase'] == 'post' for record in phases(workflow)[phase_baseline_count:]),
                    label='final native phase metrics appended', seconds=5)
        report['core_workflow']['native_phase_records_before_observer'] = phases(workflow)[:phase_baseline_count]
        report['core_workflow']['native_phase_records'] = phases(workflow)[phase_baseline_count:]
        records = report['core_workflow']['native_phase_records']
        heartbeat = {}
        for name, expected_result in [('preview', 'verified'), ('post', 'worker_finished')]:
            matching = [record for record in records if record['phase'] == name]
            record = matching[-1] if matching else None
            measurable = bool(record and record['result'] == expected_result and record['ticks'] > 0)
            heartbeat[name] = {'budget_ms': 50, 'actual_ms': record['max_gap_ms'] if measurable else None,
                'passes': measurable and record['max_gap_ms'] <= 50,
                'measurable': measurable, 'result': record['result'] if record else 'absent'}
        report['core_workflow']['strict_phase_heartbeat_budgets'] = heartbeat
        report['core_workflow']['application_memory_kib_after_workflow'] = helper.rss(app.pid)
        report['core_workflow']['posted_shape'] = core_shape(workflow)
        posted = report['core_workflow']['posted_shape']['opening']
        assert posted['ledger_debit_irr'] == posted['ledger_credit_irr'] == 1251
        assert posted['stock_value_irr'] == 300 and posted['grni_value_irr'] == 50
        assert posted['import_source'] is None, 'Native forms provenance changed; update methodology'
        capture(window, workflow / 'after-post.png')
        native.stop(app)
        env.pop('SARV_CORE_SMOKE')
        env.pop('SARV_CORE_PERFORMANCE_TRACE')
        print(json.dumps({'core_search': report['core_workflow']['search_ms'],
                          'preview_ms': report['core_workflow']['preview_observed_ms'],
                          'post_ms': report['core_workflow']['post_durable_observed_ms']}, ensure_ascii=False), flush=True)

        assert json.loads((QUOTE_SOURCE / 'native-responsiveness-verification.json').read_text())['synthetic_only']
        for name, source in [('core-small-posted', workflow), ('quote-scale', QUOTE_SOURCE)]:
            source_before = helper.payload(source)
            profile = output / name
            copy_database(source, profile)
            dataset = {'source': str(source), 'source_shape': core_shape(source), 'samples': []}
            report['datasets'][name] = dataset
            env['SARV_DATA_DIR'] = str(profile)
            for index in range(args.samples + 1):
                assert helper.sha(binary) == args.expect_sha, 'Executable changed'
                observed = []
                stop_sampler = threading.Event()
                begin = time.perf_counter()
                app = start([str(binary)], f'{name}-{index}', env)
                def monitor():
                    while not stop_sampler.is_set():
                        value = helper.rss(app.pid)
                        if value:
                            observed.append(value)
                        stop_sampler.wait(.01)
                sampler = threading.Thread(target=monitor, daemon=True)
                sampler.start()
                try:
                    window = native.wait_for(lambda: x11.find(app.pid), label='owned startup window')
                    access = Access(app.pid)
                    save = native.wait_for(lambda: helper.find_name(access, 'ذخیرهٔ پیش‌نویس'), label='editor Save')
                    native.wait_for(lambda: ready(access, save), label='visible usable editor Save')
                    x11.focus(window)
                    focused = native.wait_for(lambda: helper.find_focused(access), label='initial focus')
                    x11.key('Tab')
                    native.wait_for(lambda: helper.find_focused(access, focused['node']), label='real Tab focus change')
                    screenshot = profile / f'startup-{index}.png'
                    capture(window, screenshot)
                    duration = (time.perf_counter() - begin) * 1000
                    time.sleep(.5)
                    memory = helper.rss(app.pid)
                    dataset['samples'].append({'index': index, 'startup_ms': duration,
                        'steady_rss_kib': memory['VmRSS'], 'kernel_peak_rss_kib': memory['VmHWM'],
                        'rss_observer_samples': len(observed), 'screenshot': str(screenshot)})
                    if index == 0:
                        baseline = helper.payload(profile)
                        dataset['initialized_shape'] = core_shape(profile)
                finally:
                    stop_sampler.set()
                    sampler.join()
                    native.stop(app)
                assert helper.payload(profile) == baseline, 'Startup observer changed initialized state'
            measured = dataset['samples'][1:]
            dataset['repeated_process_startup_ms'] = helper.stats([s['startup_ms'] for s in measured])
            dataset['steady_rss_kib'] = helper.stats([s['steady_rss_kib'] for s in measured])
            dataset['kernel_peak_rss_kib'] = helper.stats([s['kernel_peak_rss_kib'] for s in measured])
            dataset['proposed_startup_budget'] = {'budget_ms': 3000,
                'p95_ms': dataset['repeated_process_startup_ms']['p95'],
                'passes': dataset['repeated_process_startup_ms']['p95'] <= 3000}
            dataset['proposed_initial_rss_budget'] = {'budget_bytes': 250000000,
                'p95_bytes': dataset['steady_rss_kib']['p95'] * 1024,
                'passes': dataset['steady_rss_kib']['p95'] * 1024 <= 250000000}
            dataset['final_shape'] = core_shape(profile)
            assert helper.payload(source) == source_before, 'Synthetic source changed'
            print(json.dumps({'dataset': name, 'startup': dataset['repeated_process_startup_ms'],
                              'rss': dataset['steady_rss_kib']}, ensure_ascii=False), flush=True)
        report['result'] = 'measured'
    except Exception as error:
        report['error'] = repr(error)
        raise
    finally:
        report['source_code_sha256_after'] = helper.source_hash()
        report['binary_sha256_after'] = helper.sha(binary)
        (output / 'performance.json').write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
        for process in reversed(processes):
            native.stop(process)
        for log in logs:
            log.close()


if __name__ == '__main__':
    main()
