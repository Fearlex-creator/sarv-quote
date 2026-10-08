#!/usr/bin/env python3
"""Analyze a retained M6 epoch after every owned measurement process closes."""
import argparse
import json
import math
from pathlib import Path
import re


def quantiles(values):
    values = sorted(values)
    if not values:
        return {'n': 0, 'status': 'unmeasured'}
    return {'n': len(values), **{key: values[math.ceil(len(values)*fraction)-1]
            for key, fraction in [('p50', .5), ('p95', .95), ('p99', .99)]},
            'worst': values[-1], 'method': 'nearest rank'}


def analyze(raw, directory):
    checks, unmeasured = [], []
    def check(name, values, limit, metric='p95', unit='ms'):
        summary = quantiles(values)
        if not values:
            unmeasured.append(name)
            return
        checks.append({'scope': name, 'limit': limit, 'unit': unit, 'metric': metric,
                       'observed': summary[metric], 'quantiles': summary,
                       'status': 'PASS' if summary[metric] <= limit else 'MISS'})
    for name, data in raw['startup'].items():
        samples = data['samples']
        check(f'{name}:initial-backup-process-startup', [r['startup_ms'] for r in samples if r['index'] == 0], 3000, 'worst')
        check(f'{name}:repeated-process-startup', [r['startup_ms'] for r in samples if r['index'] > 0], 3000, 'worst')
        check(f'{name}:initial-steady-RSS', [r['steady_rss_kib']*1024 for r in samples], 250000000, 'worst', 'bytes')
        if data['result'] != 'measured' or len(samples) != (2 if raw['development_pilot'] else 11):
            unmeasured.append(f'{name}:complete-declared-startup-samples')
    for name, data in raw['source_picker_search'].items():
        for boundary in ('correct_visible_ms', 'focused_usable_ms'):
            check(f'{name}:query:{boundary}', [r[boundary] for r in data['samples']], 200)
            check(f'{name}:filtered-page:{boundary}', [r[boundary] for r in data['filtered_pages']], 150)
        if data['result'] != 'measured' or len(data['samples']) != (2 if raw['development_pilot'] else 20):
            unmeasured.append(f'{name}:complete-declared-query-samples')
    phases = []
    for name, data in raw['return_workflows'].items():
        for direction in ('next', 'previous'):
            samples = [r for r in data['page_samples'] if r['direction'] == direction]
            if samples:
                for boundary in ('correct_visible_ms', 'focused_usable_ms'):
                    check(f'{name}:editor:{direction}:{boundary}', [r[boundary] for r in samples], 150)
        for row in data.get('strict_phase_checks', []):
            gaps = row['gaps_ms']
            if row.get('record_valid') is False:
                phases.append({'workflow': name, **row, 'gap_quantiles': {'n': 0, 'status': 'invalid native record'}})
                unmeasured.append(f"{name}:invalid-{row.get('phase')}-native-record")
                continue
            assert len(gaps) == row['ticks'] + 1
            assert abs(sum(gaps)-row['elapsed_ms']) < 1e-5
            assert abs(max(gaps)-row['max_gap_ms']) < 1e-6
            phases.append({'workflow': name, **row, 'gap_quantiles': quantiles(gaps)})
            if row['measurable_accepted']:
                check(f"{name}:native-{row['phase']}-maximum-gap", gaps, 50, 'worst')
        required_phases = ('editor', 'preview', 'post') if raw.get('protocol') == 'm6-scoped-v3' else ('preview', 'post')
        for phase in required_phases:
            if not any(r['measurable_accepted'] and r['phase'] == phase for r in data.get('strict_phase_checks', [])):
                unmeasured.append(f'{name}:accepted-{phase}-positive-tick-heartbeat')
        if data['result'] != 'measured':
            unmeasured.append(f'{name}:complete-native-workflow')
        if raw.get('protocol') == 'm6-scoped-v3':
            coverage = data.get('editor_first_preview_coverage', {})
            expected_lines = 200 if name == 'return200-billed' else 20
            if not coverage.get('all_entered_before_first_preview') or coverage.get('entered_lines') != expected_lines:
                unmeasured.append(f'{name}:all-inputs-before-first-preview-editor-scope')
            if sum(row['measurable_accepted'] and row['phase'] == 'editor' for row in data.get('strict_phase_checks', [])) != 1:
                unmeasured.append(f'{name}:exactly-one-accepted-editor-phase')
    expected = {'startup': 6, 'source_picker_search': 2, 'return_workflows': 3}
    for key, count in expected.items():
        if len(raw[key]) != count:
            unmeasured.append(f'{key}:missing-declared-cohort')
    warnings = []
    for path in sorted(directory.glob('*.log')):
        if path.name in ('xvfb.log', 'openbox.log'):
            continue
        for index, line in enumerate(path.read_text(errors='replace').splitlines(), 1):
            if re.search(r'(Gtk|Gdk|Adwaita|GLib).*(WARNING|CRITICAL)|assertion.*failed', line, re.I):
                warnings.append({'file': path.name, 'line': index, 'text': line})
    misses = [r for r in checks if r['status'] == 'MISS']
    valid = raw['result'] == 'measured' and raw.get('all_provenance_guards_stable') is True
    strict = 'FAILED' if misses else ('PARTIAL' if unmeasured or not valid else 'PASS in measured bounded scopes')
    return {'development_pilot': raw['development_pilot'], 'performance_acceptance': 'qualification only' if raw['development_pilot'] else strict,
            'strict_measured_gate': strict, 'epoch_complete_and_guarded': valid,
            'checks': checks, 'misses': misses, 'unmeasured_or_incomplete_required_scopes': unmeasured,
            'all_native_phase_records_with_terminal_gaps': phases,
            'application_warnings': warnings, 'warning_count': len(warnings),
            'historical_M5_strict_misses_retained': raw['historical_M5_strict_misses_retained'],
            'always_unmeasured': [('Read-only page/edit heartbeat outside ready editor→first Preview window' if raw.get('protocol') == 'm6-scoped-v3'
                                   else 'Read-only page/edit continuous GTK heartbeat'), 'Dedicated reference hardware and real OS-cold',
                                 'Full reference five-year dataset', 'Typical invoice/annual report/import budgets',
                                 'Production, tax/statutory and competitor acceptance'],
            'limits': raw['limits']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('epoch', type=Path)
    parser.add_argument('fresh_analysis', type=Path)
    args = parser.parse_args()
    raw = json.loads((args.epoch/'performance.json').read_text())
    result = analyze(raw, args.epoch)
    with args.fresh_analysis.open('x') as stream:
        stream.write(json.dumps(result, ensure_ascii=False, indent=2)+'\n')
    print(json.dumps({'status': result['strict_measured_gate'], 'misses': len(result['misses']),
                      'unmeasured': len(result['unmeasured_or_incomplete_required_scopes'])}))


if __name__ == '__main__':
    main()
