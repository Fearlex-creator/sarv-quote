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
        command = ['dbus-run-session', '--', '/usr/bin/python3', str(ROOT / 'scripts/native-performance-M6.py'),
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
