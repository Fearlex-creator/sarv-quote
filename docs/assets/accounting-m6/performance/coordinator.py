#!/usr/bin/env python3
"""One immutable M6 pilot/final invocation; no build, analysis or curation."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import time


ROOT = Path(__file__).resolve().parents[4]
SELF = Path(__file__).resolve()


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def save(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')


def process_identity(pid):
    directory = Path(f'/proc/{pid}')
    try:
        stat = (directory / 'stat').read_text()
        suffix = stat[stat.rfind(')') + 2:].split()
        return {'pid': pid, 'state': suffix[0], 'comm': stat[stat.find('(') + 1:stat.rfind(')')],
                'pgid': int(suffix[2]), 'starttime_ticks': int(suffix[19]),
                'command': [part.decode() for part in (directory / 'cmdline').read_bytes().split(b'\0') if part]}
    except FileNotFoundError:
        return None


def same_identity(record, current):
    return current is not None and all(current.get(key) == record.get(key)
        for key in ('pid', 'pgid', 'starttime_ticks', 'comm', 'command'))


def public_identity(record, current):
    if current is None or same_identity(record, current):
        return current
    # A reused/unexpected process may have unrelated arguments. Compare them
    # internally, but never place those arguments or its name in evidence.
    return {**{key: current[key] for key in ('pid', 'state', 'pgid', 'starttime_ticks')},
            'mismatch_fields': [key for key in ('pid', 'pgid', 'starttime_ticks', 'comm', 'command') if current.get(key) != record.get(key)],
            'comm_sha256': hashlib.sha256(current['comm'].encode()).hexdigest(),
            'command_sha256': hashlib.sha256(json.dumps(current['command']).encode()).hexdigest()}


def cleanup_owned(ledger, binary, wrapper_starttime):
    """Stop only exact newly journaled private workers; never discover/kill by name."""
    result = {'journal': str(ledger), 'records': [], 'errors': [], 'all_owned_inactive': True,
              'closure_scope': 'Known complete journaled identities only; native exit before journaling may leave an unknown worker. Missing final raw report remains incomplete.'}
    if not ledger.exists():
        result['journal_missing'] = True
        result['errors'].append({'error': 'Started observer wrapper has no durable process journal; closure unknown'})
        result['all_owned_inactive'] = False
        return result
    if not ledger.read_bytes().strip():
        result['journal_empty'] = True
        result['errors'].append({'error': 'Started observer wrapper has empty process journal; closure unknown'})
        result['all_owned_inactive'] = False
        return result
    seen = set()
    for number, line in enumerate(ledger.read_text().splitlines(), 1):
        try:
            record = json.loads(line)
            for key in ('pid', 'pgid', 'starttime_ticks'):
                assert type(record[key]) is int and record[key] > 1, 'Invalid owned process integer'
            assert record['pid'] == record['pgid'], 'Owned workers must have independent new sessions'
            assert record['starttime_ticks'] >= wrapper_starttime, 'Historical process rejected'
            assert isinstance(record['comm'], str) and record['comm'], 'Missing process name'
            command = record['command']
            assert isinstance(command, list) and command and all(isinstance(value, str) for value in command)
            assert command == [str(binary)] or command == ['openbox', '--sm-disable', '--config-file',
                '/workspace/sarv-environment/sysroot/etc/xdg/openbox/rc.xml'] or (
                len(command) == 10 and command[0:2] == ['Xvfb', '-displayfd']
                and command[2].isdigit() and command[3:] == ['-screen', '0', '1450x1050x24',
                    '-nolisten', 'tcp', '-xkbdir', '/workspace/sarv-environment/sysroot/usr/share/X11/xkb']), 'Nonallowlisted private command'
            identity = (record['pid'], record['starttime_ticks'])
            assert identity not in seen, 'Duplicate owned identity'
            seen.add(identity)
            result['records'].append({'record': record})
        except (AssertionError, KeyError, TypeError, ValueError) as error:
            result['errors'].append({'line': number, 'error': repr(error)})
    for row in reversed(result['records']):
        record = row['record']
        try:
            current = process_identity(record['pid'])
            row['before'] = public_identity(record, current)
            if current is None or current['state'] == 'Z':
                row['action'] = 'already inactive'
            elif same_identity(record, current):
                try:
                    # A pidfd pins the process even if it exits and its PID is
                    # reused between the final /proc check and the signal.
                    descriptor = os.pidfd_open(record['pid'], 0)
                    try:
                        assert same_identity(record, process_identity(record['pid'])), 'Identity changed before signal'
                        signal.pidfd_send_signal(descriptor, signal.SIGTERM)
                    finally:
                        os.close(descriptor)
                    row['action'] = 'SIGTERM exact owned identity'
                except ProcessLookupError:
                    row['action'] = 'exited before SIGTERM'
            else:
                row['action'] = 'identity mismatch untouched'
                result['errors'].append({'pid': record['pid'], 'error': 'Recorded identity no longer matches; no signal'})
        except (AssertionError, OSError, ValueError) as error:
            row['error'] = repr(error)
            result['errors'].append({'pid': record['pid'], 'error': repr(error)})
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        if not any((current := process_identity(row['record']['pid'])) is not None
                   and current['state'] != 'Z' and same_identity(row['record'], current) for row in result['records']):
            break
        time.sleep(.1)
    for row in result['records']:
        current = process_identity(row['record']['pid'])
        row['after'] = public_identity(row['record'], current)
        row['owned_identity_active'] = current is not None and current['state'] != 'Z' and same_identity(row['record'], current)
        if row['owned_identity_active']:
            result['all_owned_inactive'] = False
    if result['errors']:
        result['all_owned_inactive'] = False
    return result


def guard(receipt, receipt_path, receipt_sha):
    result = {'coordinator_sha256': sha(SELF), 'receipt_sha256': sha(receipt_path)}
    assert result['coordinator_sha256'] == receipt['coordinator_sha256'], 'Coordinator bytes changed'
    assert result['receipt_sha256'] == receipt_sha, 'Coordinator receipt bytes changed'
    for label in ('source_manifest', 'verification_manifest', 'dataset_manifest'):
        record = receipt[label]
        result[label + '_sha256'] = sha(ROOT / record['path'])
        assert result[label + '_sha256'] == record['sha256'], label + ' bytes changed'
    source = json.loads((ROOT / receipt['source_manifest']['path']).read_text())
    frozen = source['frozen_source_provenance']
    paths = sorted([*ROOT.glob('src/**/*.rs'), ROOT / 'src/style.css', ROOT / 'Cargo.toml', ROOT / 'Cargo.lock'],
                   key=lambda path: path.relative_to(ROOT).as_posix().encode())
    digest, files = hashlib.sha256(), {}
    for path in paths:
        name, data = path.relative_to(ROOT).as_posix(), path.read_bytes()
        digest.update(name.encode() + b'\0' + data + b'\0')
        files[name] = hashlib.sha256(data).hexdigest()
    assert files == frozen['files'] and digest.hexdigest() == frozen['sha256'], 'Production changed'
    assert digest.hexdigest() == receipt['production_sha256'], 'Wrong production epoch'
    result.update(production_sha256=digest.hexdigest(), production_files=files)
    verification = json.loads((ROOT / receipt['verification_manifest']['path']).read_text())['files']
    result['verification_files'] = {name: sha(ROOT / name) for name in verification}
    assert result['verification_files'] == verification, 'Verification inputs changed'
    dataset = json.loads((ROOT / receipt['dataset_manifest']['path']).read_text())
    result['dataset_files'] = {}
    for name, record in dataset['files'].items():
        path = Path(name)
        assert path.is_relative_to(ROOT / 'artifacts/performance-m6'), 'Unprotected dataset path'
        result['dataset_files'][name] = {'bytes': path.stat().st_size, 'sha256': sha(path)}
        assert result['dataset_files'][name] == record, 'Dataset file changed'
    result['seed_files'] = {key: sha(Path(dataset[key])) for key in ('seed_state', 'seed_backup')}
    assert all(result['seed_files'][key] == dataset[key + '_sha256'] for key in result['seed_files']), 'Seed changed'
    result['binary_sha256'] = sha(Path(receipt['binary']))
    assert result['binary_sha256'] == receipt['binary_sha256'] == source['binary_sha256'], 'Executable changed'
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--receipt', type=Path, required=True)
    parser.add_argument('--receipt-sha', required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--pilot', action='store_true')
    args = parser.parse_args()
    args.receipt, args.output = args.receipt.resolve(), args.output.resolve()
    assert args.receipt.is_relative_to(SELF.parent), 'Receipt outside owned evidence'
    assert args.output.is_relative_to(ROOT / 'artifacts/performance-m6') and not args.output.exists(), 'Fresh synthetic output required'
    assert sha(args.receipt) == args.receipt_sha, 'Receipt hash must match before parsing'
    receipt = json.loads(args.receipt.read_text())
    args.output.mkdir(parents=True)
    result = {'result': 'incomplete', 'development_pilot': args.pilot, 'observer_launched': False,
              'start_utc': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()),
              'receipt': receipt, 'receipt_sha256': args.receipt_sha}
    process = None
    def interrupted(signum, frame):
        raise KeyboardInterrupt('Coordinator interrupted by signal ' + str(signum))
    signal.signal(signal.SIGTERM, interrupted)
    try:
        result['guard_before'] = guard(receipt, args.receipt, args.receipt_sha)
        assert hasattr(os, 'pidfd_open') and hasattr(signal, 'pidfd_send_signal'), 'Verified owned cleanup needs Linux pidfd support'
        command = ['dbus-run-session', '--', '/usr/bin/python3', '-X', 'faulthandler', str(ROOT / 'scripts/native-performance-M6.py'),
                   '--binary', receipt['binary'], '--expect-sha', receipt['binary_sha256'],
                   '--source-manifest', str(ROOT / receipt['source_manifest']['path']),
                   '--verification-manifest', str(ROOT / receipt['verification_manifest']['path']),
                   '--dataset-manifest', str(ROOT / receipt['dataset_manifest']['path']),
                   '--samples', '10', '--search-samples', '20', '--output', str(args.output / 'observer')]
        if args.pilot:
            command.append('--pilot')
        result['command'] = command
        save(args.output / 'coordinator.json', result)
        with (args.output / 'coordinator-observer.log').open('x') as log:
            process = subprocess.Popen(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            result['observer_wrapper_identity'] = process_identity(process.pid)
            result.update(observer_launched=True, observer_wrapper_pid=process.pid)
            save(args.output / 'coordinator.json', result)
            result['observer_exit_code'] = process.wait()
        result['result'] = 'completed' if result['observer_exit_code'] == 0 else 'observer_failed'
    except BaseException as error:
        result['error'] = repr(error)
        if process and process.poll() is None:
            os.killpg(process.pid, signal.SIGINT)
            try:
                result['observer_exit_code'] = process.wait(timeout=45)
            except subprocess.TimeoutExpired:
                result['observer_cleanup_timeout'] = True
                os.killpg(process.pid, signal.SIGTERM)
                result['observer_exit_code'] = process.wait(timeout=10)
    finally:
        if process:
            try:
                identity = result['observer_wrapper_identity']
                result['external_owned_cleanup'] = cleanup_owned(args.output / 'observer/owned-processes.jsonl',
                    Path(receipt['binary']), identity['starttime_ticks'])
                if not result['external_owned_cleanup']['all_owned_inactive']:
                    result['result'] = 'rejected_incomplete_owned_cleanup'
            except BaseException as error:
                result.update(external_cleanup_error=repr(error), result='rejected_incomplete_owned_cleanup')
        try:
            result['guard_after'] = guard(receipt, args.receipt, args.receipt_sha)
            result['all_coordinator_guards_stable'] = result.get('guard_before') == result['guard_after']
        except BaseException as error:
            result.update(guard_after_error=repr(error), all_coordinator_guards_stable=False)
        if not result['all_coordinator_guards_stable']:
            result['result'] = 'rejected_changed_provenance'
        result['end_utc'] = time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())
        save(args.output / 'coordinator.json', result)
        print(json.dumps({key: result[key] for key in ('result', 'development_pilot', 'observer_launched', 'all_coordinator_guards_stable')}), flush=True)
    return 0 if result['result'] == 'completed' else 1


if __name__ == '__main__':
    raise SystemExit(main())
