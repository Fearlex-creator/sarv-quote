import json
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
root=Path(__file__).resolve().parent
p=json.loads((root/'performance.json').read_text())
plt.rcParams.update({'font.size':10,'axes.spines.top':False,'axes.spines.right':False,'svg.fonttype':'none'})
fig,axes=plt.subplots(3,1,figsize=(11,10),constrained_layout=True)
for color,(key,label) in zip(['#147d92','#7956a3','#bd742b'],[('core-small-posted','Posted core'),('quote-scale','50,000 products / 1,000 quote lines'),('procurement-small-posted','Posted procurement')]):
 values=[row['startup_ms'] for row in p['datasets'][key]['samples'][1:]]
 axes[0].plot(range(1,11),values,'o-',color=color,label=label,linewidth=1.3,markersize=4)
axes[0].axhline(3000,color='#a34336',linestyle='--',label='Proposed startup target: 3,000 ms')
axes[0].set(title='New process startup: n=10 per fixture; OS cache warm / uncontrolled',xlabel='Sample order (first-copy initialization excluded)',ylabel='Launch to Save + Tab + PNG (ms)',ylim=(0,3200),xticks=range(1,11))
for color,(key,label) in zip(['#147d92','#7956a3'],[('correct_visible_action_ms','Correct visible Edit action'),('focused_usable_action_ms','Focused usable Edit action')]):
 values=[row[key] for row in p['core_workflow']['search_samples']]
 axes[1].plot(range(1,21),values,'o-',color=color,label=label,linewidth=1.2,markersize=3)
axes[1].axhline(200,color='#a34336',linestyle='--',label='Proposed lookup p95 target: 200 ms')
axes[1].set(title='Matched core exact-ID search: n=20; real keys and observer overhead',xlabel='Alternating CU-A / CU-B sample order',ylabel='Dispatch to result (ms)',ylim=(0,260),xticks=range(1,21))
for color,(key,label) in zip(['#147d92','#7956a3'],[('correct_visible_ms','Current visible Detail action'),('focused_usable_ms','Focused usable Detail action')]):
 values=[row[key] for row in p['procurement_measurement']['searches']['document']['samples']]
 axes[2].plot(range(1,21),values,'o-',color=color,label=label,linewidth=1.2,markersize=3)
axes[2].axhline(200,color='#a34336',linestyle='--',label='Proposed lookup p95 target: 200 ms')
axes[2].set(title='Procurement exact-reference search: n=20; native Tab navigation included',xlabel='Alternating REC-A / REC-B sample order',ylabel='Dispatch to result (ms)',ylim=(0,280),xticks=range(1,21))
for ax in axes:
 ax.grid(axis='y',alpha=.18)
 ax.legend(loc='upper right',frameon=False,fontsize=9)
fig.suptitle('M4 freeze3 native benchmark — 6eed73cc…',fontsize=14,fontweight='bold')
fig.savefig(root/'latency-samples.png',dpi=150)
fig.savefig(root/'latency-samples.svg')
