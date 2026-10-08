import json
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
root=Path(__file__).resolve().parent
p=json.loads((root/'performance.json').read_text())
plt.rcParams.update({'font.size':10,'axes.spines.top':False,'axes.spines.right':False,'svg.fonttype':'none'})
fig,axes=plt.subplots(2,1,figsize=(11,7.5),constrained_layout=True)
colors=['#147d92','#7956a3']
for color,(key,label) in zip(colors,[('core-small-posted','Posted core: 4 parties / 1 opening'),('quote-scale','Quote: 50,000 products / 1,000 lines')]):
 values=[row['startup_ms'] for row in p['datasets'][key]['samples'][1:]]
 axes[0].plot(range(1,len(values)+1),values,'o-',color=color,label=label,linewidth=1.3,markersize=5)
axes[0].axhline(3000,color='#a34336',linestyle='--',label='Proposed startup target: 3,000 ms')
axes[0].set(title='New-process startup: n=10 per fixture; OS cache warm / uncontrolled',xlabel='Sample order (first-copy initialization excluded)',ylabel='Observed launch to Save + Tab + PNG (ms)',ylim=(0,3200),xticks=range(1,11))
samples=p['core_workflow']['search_samples']
for color,(key,label) in zip(colors,[('correct_visible_action_ms','Correct visible Edit action'),('focused_usable_action_ms','Additional native Tab to focused Edit action')]):
 axes[1].plot(range(1,len(samples)+1),[row[key] for row in samples],'o-',color=color,label=label,linewidth=1.3,markersize=4)
axes[1].axhline(200,color='#a34336',linestyle='--',label='Proposed lookup p95 target: 200 ms')
axes[1].set(title='Core exact-ID search: n=20; real keys and observer overhead',xlabel='Alternating CU-A / CU-B sample order',ylabel='Observed dispatch to result (ms)',ylim=(140,280),xticks=range(1,21))
for ax in axes:
 ax.grid(axis='y',alpha=.18)
 ax.legend(loc='upper right',frameon=False,fontsize=9)
axes[0].legend(loc='upper right',bbox_to_anchor=(1,.83),frameon=False,fontsize=9)
fig.suptitle('M3 immutable native benchmark — 10bf9929…',fontsize=14,fontweight='bold')
fig.savefig(root/'latency-samples.png',dpi=170)
fig.savefig(root/'latency-samples.svg')
