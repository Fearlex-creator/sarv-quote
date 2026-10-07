#!/usr/bin/env python3
"""Render the retained measurement JSON; no new data or smoothing."""
import json
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

root = Path(__file__).resolve().parent
report = json.loads((root / 'performance.json').read_text())
fig, axes = plt.subplots(1, 3, figsize=(12, 4), layout='constrained')
plots = [
    ('G1 small startup · n=10', report['datasets']['g1-small']['samples'][1:], 'startup_ms', 3000),
    ('50k items / 1k lines startup · n=10', report['datasets']['quote-scale']['samples'][1:], 'startup_ms', 3000),
    ('Exact-code product lookup · n=20', report['datasets']['quote-scale']['indexed_search_samples'], 'duration_ms', 200),
]
for ax, (title, samples, key, budget) in zip(axes, plots):
    values = [sample[key] for sample in samples]
    ax.plot(range(1, len(values) + 1), values, 'o-', color='#3267a8', markersize=4, linewidth=1)
    ax.axhline(budget, color='#a36c19', linestyle='--', linewidth=1, label=f'Proposed budget {budget:,} ms')
    ax.set_title(title, fontsize=10)
    ax.set_xlabel('Sample')
    ax.set_ylabel('Milliseconds')
    ax.set_xlim(.5, len(values) + .5)
    ax.set_ylim(0, budget * 1.07)
    ax.spines[['top', 'right']].set_visible(False)
    ax.grid(axis='y', alpha=.15)
    ax.legend(loc='upper center', bbox_to_anchor=(.5, -.22), frameon=False, fontsize=8)
fig.suptitle('M2 observed samples · shared executor · warm/uncontrolled OS cache\nStartup includes accessibility, real Tab focus and screenshot acquisition', fontsize=11)
fig.savefig(root / 'latency-samples.png', dpi=160)
fig.savefig(root / 'latency-samples.svg')
