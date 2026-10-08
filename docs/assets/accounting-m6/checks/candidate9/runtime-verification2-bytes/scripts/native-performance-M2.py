#!/usr/bin/env python3
"""Run the existing complete native scale probe in a new synthetic profile.

Each output directory must be new. This harness never builds or touches source
fixtures. It records strict performance misses independently from functional QA.
"""
import argparse, hashlib, json, os, pathlib, platform, re, subprocess, tempfile, time

parser = argparse.ArgumentParser()
parser.add_argument('--binary', required=True, type=pathlib.Path)
parser.add_argument('--expect-sha', required=True)
parser.add_argument('--output', required=True, type=pathlib.Path)
args = parser.parse_args()
binary = args.binary.resolve()
sha = hashlib.sha256(binary.read_bytes()).hexdigest()
assert sha == args.expect_sha, 'executable hash differs from requested immutable build'
output = args.output.resolve()
output.mkdir(parents=True, exist_ok=False)
profile = output / 'synthetic-profile'
profile.mkdir()
env = {k: v for k, v in os.environ.items() if not k.startswith('SARV_')}
env.update(SARV_DATA_DIR=str(profile), SARV_RESPONSIVENESS='1', SARV_PERFORMANCE_TRACE='1',
           SARV_TEST_INSTANCE='m2-performance-owned', GDK_BACKEND='x11', GSK_RENDERER='cairo')
processes = []
logs = []
def start(command, label, **kwargs):
    log = (output / (label + '.log')).open('w')
    logs.append(log)
    process = subprocess.Popen(command, env=env, stdout=log, stderr=log, **kwargs)
    processes.append(process)
    return process
try:
    with tempfile.TemporaryFile() as display_fd:
        xvfb = start(['Xvfb', '-displayfd', str(display_fd.fileno()), '-screen', '0', '1450x1050x24',
                      '-nolisten', 'tcp', '-xkbdir', '/workspace/sarv-environment/sysroot/usr/share/X11/xkb'],
                     'xvfb', pass_fds=(display_fd.fileno(),))
        limit = time.monotonic() + 10
        while time.monotonic() < limit:
            display_fd.seek(0)
            display = display_fd.read().strip()
            if display:
                break
            assert xvfb.poll() is None, 'private Xvfb stopped'
            time.sleep(.02)
        assert display, 'private Xvfb did not become ready'
    with tempfile.TemporaryDirectory(prefix='sarv-m2-runtime-') as runtime:
        env.update(DISPLAY=':' + display.decode(), XDG_RUNTIME_DIR=runtime)
        start(['openbox', '--sm-disable'], 'openbox')
        app = start([str(binary)], 'native')
        report_file = profile / 'native-responsiveness-verification.json'
        limit = time.monotonic() + 135
        while not report_file.exists() and time.monotonic() < limit:
            assert app.poll() is None, 'owned application stopped before native report'
            time.sleep(.05)
        assert report_file.exists(), 'native probe timed out without report'
        report = json.loads(report_file.read_text())
        gaps = report.get('maximum_heartbeat_gap_ms_including_setup_and_editor_paging')
        navigation = report.get('maximum_unchanged_editor_page_click_ms') if report.get('editor_page_clicks') else None
        pdf = report.get('maximum_heartbeat_gap_ms_while_pdf_busy') if report.get('heartbeat_ticks_while_pdf_busy', 0) else None
        trace = {}
        for operation, value in re.findall(r'SARV_MAIN_THREAD (\S+) ([0-9.]+)ms', (output / 'native.log').read_text()):
            trace.setdefault(operation, []).append(float(value))
        status = pathlib.Path('/proc', str(app.pid), 'status').read_text()
        memory_kib = {key: int(value) for key, value in re.findall(r'^(VmRSS|VmHWM):\s+(\d+) kB$', status, re.M)}
        result = {'application_memory_kib_after_full_workflow': memory_kib, 'binary': str(binary), 'sha256': sha, 'native_report': str(report_file),
                  'functional_result': report.get('result'), 'main_thread_trace_ms': trace, 'strict_budgets': {
                    'maximum_heartbeat_gap_ms': {'actual': gaps, 'budget': 50, 'passes': gaps is not None and gaps <= 50},
                    'unchanged_editor_page_ms': {'actual': navigation, 'budget': 150, 'passes': navigation is not None and navigation <= 150},
                    'pdf_busy_heartbeat_ms': {'actual': pdf, 'budget': 50, 'passes': pdf is not None and pdf <= 50}},
                  'measurement': 'n=1 complete native workflow; timer gaps include observer and scheduling; no p95 claim',
                  'host': platform.platform(),
                  'limits': ['private X11/Cairo executor; Wayland and physical desktop untested',
                             '20 ms heartbeat is a responsiveness proxy, not a continuous CPU trace',
                             'probe screenshots and setup are included in global gaps']}
        assert hashlib.sha256(binary.read_bytes()).hexdigest() == sha, 'executable changed during measurement'
        (output / 'performance.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps({k: v for k, v in result.items() if k != 'main_thread_trace_ms'}, indent=2), flush=True)
finally:
    for process in reversed(processes):
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)
    for log in logs:
        log.close()
