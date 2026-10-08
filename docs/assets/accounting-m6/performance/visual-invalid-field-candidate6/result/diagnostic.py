#!/usr/bin/env python3
"""One untimed, unposted hidden-invalid-field visual diagnostic on a fresh copy."""
import argparse
import ctypes as C
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sys
import time

ROOT = Path(__file__).resolve().parents[5]
SELF = Path(__file__).resolve()
sys.path.insert(0, str(ROOT / 'scripts'))


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def enclosed(inner, outer):
    return bool(inner and outer and inner[2] > 0 and inner[3] > 0
                and inner[0] >= outer[0] - 1 and inner[1] >= outer[1] - 1
                and inner[0] + inner[2] <= outer[0] + outer[2] + 1
                and inner[1] + inner[3] <= outer[1] + outer[3] + 1)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--expect-self-sha', required=True)
    args = parser.parse_args()
    args.output = args.output.resolve()
    assert args.output.is_relative_to(ROOT / 'artifacts/performance-m6') and not args.output.exists()
    assert digest(SELF) == args.expect_self_sha
    assert os.environ.get('DBUS_SESSION_BUS_ADDRESS', '').startswith('unix:path=/tmp/dbus-')
    receipt_path = ROOT / 'docs/assets/accounting-m6/performance/candidate6-runtime6-coordinator-receipt.json'
    receipt_sha = 'bb670b463efc208723e18f3e915e089e0a4774d092be294202d868fbf8146104'
    receipt = json.loads(receipt_path.read_text())
    coordinator = module('visual_unchanged_coordinator', receipt_path.parent / 'coordinator.py')
    before = coordinator.guard(receipt, receipt_path, receipt_sha)
    held_path = ROOT / 'scripts/native-performance-M6.py'
    assert digest(held_path) == '39409332a7caae630d45cf94858df7dc4baf9c0f56b377f47fbbe757b505fc0c'
    held = module('visual_unchanged_m6_observer', held_path)
    dataset = json.loads((ROOT / receipt['dataset_manifest']['path']).read_text())
    args.binary = Path(receipt['binary'])
    args.output.mkdir(parents=True)
    report = {'result': 'incomplete', 'visual_diagnostic_only': True, 'timed_budget_claim': False,
              'sidecar_sha256': args.expect_self_sha, 'receipt_sha256': receipt_sha,
              'guard_before': before, 'capture_samples': [], 'owned_process_records': [],
              'x11_error_observations': [], 'rejected_source_observations': [],
              'completed_checkpoint_boundaries': [], 'input_receipt_ids': [], 'page_replay': [],
              'observations': [], 'initial_utc': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())}
    observer = held.Observer(args, report, dataset)
    profile = None
    baseline = None
    try:
        observer.setup()
        observer.env.update(SARV_UI_SMOKE='capture', SARV_CAPTURE_PAGE='order',
                            SARV_CAPTURE_NAME='visual-initial.png')
        profile, recipe = observer.copied('return200-billed', 'visual-profile')
        baseline = held.payload(profile)
        app, window, access = observer.app(profile, 'visual-app', False)
        held.observe(lambda: (profile / 'visual-initial.png').exists(), 'neutral initial smoke capture complete')
        assert held.payload(profile) == baseline
        observer.purchases(access)
        ids = recipe['receipt_line_ids']
        quantity, editor, _ = observer.open_source(access, recipe['source_reference'], ids[0])
        for name, value in [('تاریخ مؤثر مدنی', '2026-04-03'), ('تاریخ سند طرف', '2026-04-03'),
                            ('شرح و دلیل ثبت', 'M6 unposted hidden invalid visual diagnosis'),
                            ('شماره یا مرجع سند', 'M6-VISUAL-RETURN200'),
                            ('مرجع بستانکار تأمین‌کننده', 'M6-VISUAL-CREDIT200')]:
            held.fill(access, name, value, observer.x11, editor)
        for page in range(1, 9):
            if page > 1:
                report['page_replay'].append(observer.turn(access, editor, ids, page, 'next'))
            quantities = held.observe(lambda: observer.page_state(access, editor, ids, page), 'exact bounded visual replay page')
            for row in quantities:
                held.fill(access, row['name'], '1', observer.x11, editor)
                report['input_receipt_ids'].append(row['name'][len(held.QUANTITY_PREFIX):])
            assert held.payload(profile) == baseline
            observer.checkpoint(f'visual-200-entry-replay-page:{page}')
            print(json.dumps({'phase': 'actual-quantity-replay', 'page': page, 'inputs': len(report['input_receipt_ids'])}), flush=True)
        assert report['input_receipt_ids'] == ids
        last_name = held.QUANTITY_PREFIX + ids[-1]
        held.fill(access, last_name, '2', observer.x11, editor)
        for page in range(7, 0, -1):
            report['page_replay'].append(observer.turn(access, editor, ids, page, 'previous', '1'))
        assert held.payload(profile) == baseline
        preview = held.find(access, held.PREVIEW, editor, ('button', 'push button'))
        held.focus(access, preview, observer.x11)
        dispatch = time.monotonic()
        observer.x11.key('space')
        observer.capture(window, profile / 'invalid-immediate.png')
        report['immediate_capture_delay_s'] = time.monotonic() - dispatch
        invalid = held.observe(lambda: held.find(access, last_name, editor, ('entry', 'text')), 'exact last invalid field revealed')
        held.observe(lambda: held.ready(access, invalid, editor, True) and access.text(invalid) == '2', 'raw2 actual focus without assistance')
        for minimum in (.5, 1.0, 2.0):
            remaining = dispatch + minimum - time.monotonic()
            if remaining > 0:
                time.sleep(remaining)
            current = held.find(access, last_name, editor, ('entry', 'text'))
            assert current and held.member(access, current, editor) and access.focused(current) and access.text(current) == '2'
            screenshot = profile / f'invalid-settled-min{minimum:g}s.png'
            observer.capture(window, screenshot)
            report['observations'].append({'scheduled_minimum_delay_s': minimum,
                'actual_capture_completion_delay_s': time.monotonic() - dispatch,
                'current_receipt_line_id': ids[-1], 'raw_text': access.text(current),
                'actual_focus': access.focused(current), 'current_membership': held.member(access, current, editor),
                'entry_screen_rect': access.rect(current), 'screenshot': str(screenshot),
                'durable_state_unchanged': held.payload(profile) == baseline,
                'no_focus_scroll_or_key_mutation_since_preview': True})
            observer.checkpoint(f'settled-passive-observation-min:{minimum}')
        # All passive captures precede this neutral key. The existing smoke
        # controller records real GTK focused-widget and nearest scroller bounds.
        event_path = profile / 'keyboard-events.jsonl'
        events = lambda: [json.loads(s) for s in event_path.read_text().splitlines() if s.strip()] if event_path.exists() else []
        old_events = len(events())
        observer.x11.key('Shift_L')
        event = held.observe(lambda: next((r for r in events()[old_events:] if r['key'] == 'Shift_L'), None), 'neutral Shift native geometry event')
        assert access.focused(invalid) and access.text(invalid) == '2' and held.member(access, invalid, editor)
        root_window, x, y, child = C.c_ulong(), C.c_int(), C.c_int(), C.c_ulong()
        observer.x11.x.XTranslateCoordinates(observer.x11.d, window, observer.x11.root, 0, 0,
            C.byref(x), C.byref(y), C.byref(child))
        observer.x11.check()
        entry = access.rect(invalid)
        entry_window = [entry['x'] - x.value, entry['y'] - y.value, entry['width'], entry['height']] if entry else None
        report['settled_native_geometry'] = {'after_passive_capture_delay_s': time.monotonic() - dispatch,
            'neutral_key': 'Shift_L', 'event': event, 'window_origin_screen': [x.value, y.value],
            'whole_entry_window_rect_from_ATSPI': entry_window,
            'focused_GTK_widget_enclosed_in_nearest_scroller': enclosed(event.get('focus_bounds'), event.get('focus_clip_bounds')),
            'whole_entry_enclosed_in_native_scroller_from_ATSPI': enclosed(entry_window, event.get('focus_clip_bounds')),
            'focused_GTK_widget_enclosed_in_window': enclosed(event.get('focus_bounds'), [0, 0, *event['window_size']]),
            'raw_text2_and_focus_unchanged_after_neutral_key': True}
        report['visual_result'] = ('focused-widget-clipped' if not report['settled_native_geometry']['focused_GTK_widget_enclosed_in_nearest_scroller'] else 'focused-widget-enclosed; inspect-whole-entry-and-pixels')
        assert held.payload(profile) == baseline
        held.write(profile / 'before-and-after-unposted-state.json', held.state(profile))
        observer.x11.key('Escape')
        held.observe(lambda: not held.find(access, last_name, roles=('entry', 'text')) and held.find(access, held.RETURN, roles=('button', 'push button')), 'cancel unposted diagnostic dialog')
        assert held.payload(profile) == baseline
        report.update(result='completed-unposted-visual-diagnostic', posted=False, cancelled=True,
                      original_copy_payload_generation_and_bytes_unchanged=True)
        observer.checkpoint('completed-unposted-cancel')
    except BaseException as error:
        report['error'] = repr(error)
        raise
    finally:
        observer.close()
        if profile and baseline:
            report['stopped_copy_payload_generation_and_bytes_unchanged'] = held.payload(profile) == baseline
        report['guard_after'] = coordinator.guard(receipt, receipt_path, receipt_sha)
        report['all_guards_stable'] = before == report['guard_after'] and digest(SELF) == args.expect_self_sha
        held.write(args.output / 'visual-diagnostic.json', report)
        assert report['all_guards_stable']
        print(json.dumps({'result': report['result'], 'visual_result': report.get('visual_result'),
                          'guards': report['all_guards_stable']}), flush=True)


if __name__ == '__main__':
    main()
