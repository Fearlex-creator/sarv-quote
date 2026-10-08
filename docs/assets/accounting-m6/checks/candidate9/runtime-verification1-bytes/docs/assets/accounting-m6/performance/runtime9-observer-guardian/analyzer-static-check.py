#!/usr/bin/env python3
"""Exercise new editor scope while retaining old-epoch interpretation."""
import importlib.util
import json
from pathlib import Path
import tempfile

HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('analysis_only',HERE.parent/'analyze.py')
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
base={'result':'incomplete','development_pilot':True,'startup':{},'source_picker_search':{},'return_workflows':{},
      'historical_M5_strict_misses_retained':{},'limits':[]}
cases=[]
def check(name,ok):
    assert ok,name;cases.append({'case':name,'result':'PASS'})
def run(raw):
    with tempfile.TemporaryDirectory() as directory:return module.analyze(raw,Path(directory))
old=run({**base,'protocol':'m6-scoped-v2.1'})
check('old epoch retains continuous editor heartbeat unmeasured',old['always_unmeasured'][0]=='Read-only page/edit continuous GTK heartbeat')
new=run({**base,'protocol':'m6-scoped-v3'})
check('v3 only excludes heartbeat outside declared window',new['always_unmeasured'][0]=='Read-only page/edit heartbeat outside ready editor→first Preview window')
phase={'phase':'editor','ticks':1,'gaps_ms':[16.,51.],'elapsed_ms':67.,'max_gap_ms':51.,'measurable_accepted':True,'record_valid':True}
workflow={'result':'incomplete','page_samples':[],'strict_phase_checks':[phase],
          'editor_first_preview_coverage':{'all_entered_before_first_preview':True,'entered_lines':200}}
raw={**base,'protocol':'m6-scoped-v3','return_workflows':{'return200-billed':workflow}}
result=run(raw)
check('editor terminal gap51 strictMISS retained',result['misses'][0]['scope']=='return200-billed:native-editor-maximum-gap' and result['misses'][0]['observed']==51.)
check('200 association with accepted editor remains bounded',not any('all-inputs' in scope or 'exactly-one' in scope for scope in result['unmeasured_or_incomplete_required_scopes']))
raw['return_workflows']['return200-billed']={**workflow,'editor_first_preview_coverage':{}}
result=run(raw)
check('missing200 association unmeasured',any('all-inputs' in scope for scope in result['unmeasured_or_incomplete_required_scopes']))
raw['return_workflows']['return200-billed']={**workflow,'strict_phase_checks':[phase,phase]}
check('duplicate accepted editor unmeasured',any('exactly-one' in scope for scope in run(raw)['unmeasured_or_incomplete_required_scopes']))
raw['return_workflows']['return200-billed']={**workflow,'strict_phase_checks':[{**phase,'record_valid':False,'measurable_accepted':False}]}
result=run(raw)
check('invalid native record cannot pass gap gate',not result['checks'] and any('invalid-editor' in scope for scope in result['unmeasured_or_incomplete_required_scopes']))
(HERE/'analyzer-static-check.json').write_text(json.dumps({'no_gui':True,'cases':cases,'total':len(cases)},indent=2)+'\n')
print(json.dumps({'pass':len(cases)}))
