"""Standalone plots of every retained candidate2 sample; no application launch."""
import json
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

root = Path(__file__).resolve().parent
p = json.loads((root / 'performance.json').read_text())
s = p['sales_measurement']
a = json.loads((root / 'analysis.json').read_text())
plt.rcParams.update({'font.size': 10, 'axes.spines.top': False,
                     'axes.spines.right': False, 'svg.fonttype': 'none'})
fig, axes = plt.subplots(5, 1, figsize=(12, 16), constrained_layout=True)
colors = ['#146b82', '#74539a', '#c17b22', '#2a8057', '#b74e54', '#53648e']
labels = ['Core posted', '50k items / 1k quote lines', 'Procurement posted',
          'Sales posted', '120 raw Sales drafts', '1k-line raw Sales draft']
for color, label, dataset in zip(colors, labels, p['datasets'].values()):
    axes[0].plot(range(1, 11), [row['startup_ms'] for row in dataset['samples'][1:]],
                 'o-', color=color, label=label, linewidth=1.2, markersize=3)
axes[0].axhline(3000, color='#a34336', linestyle='--', label='Startup target 3,000 ms')
axes[0].set(title='New process startup: all n=10 samples per fixture; OS caches uncontrolled',
            xlabel='Sample order; fresh-backup first launch reported separately',
            ylabel='Launch to usable Save + Tab + PNG (ms)', ylim=(0, 3200))
for color, label, values in [
    (colors[0], 'Core focused Edit', [x['focused_usable_action_ms'] for x in p['core_workflow']['search_samples']]),
    (colors[2], 'Procurement focused Detail', [x['focused_usable_ms'] for x in p['procurement_measurement']['searches']['document']['samples']]),
    (colors[3], 'Posted Sales focused Detail', [x['focused_usable_ms'] for x in s['posted_search']['samples']]),
]:
    axes[1].plot(range(1, 21), values, 'o-', color=color, label=label, markersize=3)
axes[1].axhline(200, color='#a34336', linestyle='--', label='Lookup p95 target 200 ms')
axes[1].set(title='Small posted fixtures: real query to focused usable action, n=20 per stream',
            xlabel='Alternating exact-reference sample order', ylabel='Elapsed (ms)', ylim=(0, 240))
for color, key, label in [(colors[0], 'correct_visible_ms', 'Visible Continue'),
                         (colors[4], 'focused_usable_ms', 'Focused usable Continue')]:
    axes[2].plot(range(1, 21), [x[key] for x in s['search_samples']], 'o-', color=color,
                 label=label, markersize=3)
axes[2].axhline(200, color='#a34336', linestyle='--', label='Lookup p95 target 200 ms')
axes[2].set(title='120 raw drafts / 60 current matches: strict lookup budget missed',
            xlabel='Alternating PERF-A / PERF-B sample order', ylabel='Real query to action (ms)', ylim=(0, 400))
for color, key, label in [(colors[0], 'ready_ms', 'Current marker + visible Edit'),
                         (colors[4], 'focused_usable_ms', 'Focused usable Edit')]:
    axes[3].plot([x['page'] for x in s['long_editor_pages']],
                 [x[key] for x in s['long_editor_pages']], 'o-', color=color,
                 label=label, markersize=3)
axes[3].axhline(150, color='#a34336', linestyle='--', label='Loaded navigation p95 target 150 ms')
axes[3].set(title='1,000-line raw editor: every page 2–40; strict navigation budget missed',
            xlabel='Current native 25-row page; n=39 interactions in one process',
            ylabel='Real Next action to usable result (ms)', ylim=(0, 900))
keys = ['maximum_heartbeat_gap_ms', 'unchanged_editor_page_ms',
        'pdf_busy_heartbeat_ms', 'broad_persistence_busy_heartbeat']
names = ['Global heartbeat', '17 unchanged pages\n(maximum)', 'PDF-busy heartbeat',
         'Broad persistence\nheartbeat']
values = [a['strict_budgets'][k]['actual_ms'] for k in keys]
budgets = [a['strict_budgets'][k]['budget_ms'] for k in keys]
axes[4].bar(range(4), values,
            color=['#a34336' if v > b else '#2a8057' for v, b in zip(values, budgets)], alpha=.85)
axes[4].scatter(range(4), budgets, color='#222222', marker='_', s=500, label='Strict original budget')
for index, value in enumerate(values):
    axes[4].text(index, value + 8, f'{value:.3f}', ha='center')
axes[4].set(title='Unchanged quotation-scale harness: one complete workflow; maxima, no population tail',
            ylabel='Milliseconds', xticks=range(4), xticklabels=names, ylim=(0, 380))
for ax in axes:
    ax.grid(axis='y', alpha=.18)
    ax.legend(loc='upper right', framealpha=.95, fontsize=8)
axes[2].legend(loc='lower left', framealpha=.95, fontsize=8)
fig.suptitle('M5 candidate2 native observations — 9b32342c… — performance acceptance partial',
             fontsize=14, fontweight='bold')
fig.savefig(root / 'latency-samples.svg')
fig.savefig(root / 'latency-samples.png', dpi=140)
