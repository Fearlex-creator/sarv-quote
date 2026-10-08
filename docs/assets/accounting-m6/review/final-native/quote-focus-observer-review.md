# Independent quote focus observation review

Reviewer `m6_final_native_review`; source/observer read-only, no Cargo or GUI process. Diff inspected against archived runtime4 `runtime-verification4-bytes/scripts/qa-native.py` SHA256 `d34299269c82ddfa30e8fd97759d8ef7e1d0fd13014f580792a3ed627592f82e`; proposed observer SHA256 `3989989f0063ad1df02b6f5db23fce64b4ff4ac511dbc4943ebcd1f5021cc558`.

Exact requested `rowN-quantity` maps correctly to the native quantity accessible name `تعداد ردیف fa(N+1)`: editor creation and row reuse update the global index/name consistently, and smoke instrumentation reports the same row.index. Fallback traverses only the explicitly owned application PID, requires a unique matching entry/text control whose own FOCUSED state is true, and sends no key or focus action. Original key event remains separately retained from the proposed settled observation. No timeout, budget or saved-State assertion is relaxed.

**Static observer gap:** the proposed ancestry loop clears the child cache but reads parent.children before clearing that parent; it also does not reject DEFUNCT states or recheck field focus/name after ancestry. A cached parent→retired-child edge could therefore weaken the intended current-membership proof. Requested bounded repair: refresh parent before children(), reject absent/DEFUNCT state at each ancestry edge, and recheck exact field name/current FOCUSED state immediately before recording. This is an observer-proof issue, not evidence of a product focus defect. Integrator notified before runtime5 freeze. Static fallback acceptance remains pending this repair; no live fallback exercise is inferred.

## Separate current stress5 execution

Raw run `artifacts/independent-native/run-2lrwyt1a` records50 PASS, receipt SHA256 `729bfcfe6bc5497430a4d1e6d598ce4b7fa6b02064b5243c6e14d73419b3c836`. Personally read every outcome. **Zero** checks contain a nonnull settled_focus_observation: every focus predicate was satisfied by original key-event instrumentation. Do not attribute this pass to the fallback or claim it was exercised. Prior empty-draft/async-log failures and original partial wrapper remain retained.

Independently read final SQLite and the protected four-row fixture using standard-library sqlite3/json. Final draft has1000 exact P-101 source rows: global index99 quantity2375, index100 quantity3125, every other quantity(index+1)*1000; query/chosen_code exact. Quotes, catalogs and accounting are unchanged from protected fixture. Final canonical full-State SHA256 `afa48f860250e1c2b4159f3546ea7f6476687218402451bb1027e86c09715f93`.

Personally viewed fresh-process restart, narrow lower controls and final pagination boundary PNGs. Final original event reports focus row99-quantity with bounds `[495,599,56,24]`, table clip `[18,301,1076,328]`, viewport1390×930; field fits both and PNG shows selected2.375 in visible row100. Image SHA256 `e4ecdb413ec6bc8fecef170c5a99cda32f404c515d6956e67413e7f113b62306`; restart `9239ea83c1f15fb93ce478967a9b4b6646d18fd47067d6cde1f424deff16c6d1`; narrow lower controls `81615e69d99778fb22043e284f19bd8d09b6d3c64ad46f1977c28c2519328473`.

Stress5 is accepted for these current real-key/saved-State/clip/restart cases independently of the unexercised fallback. This is functional qualification, not a quiet performance epoch or broad accessibility/production gate.

## Bounded static finding closed after stress5

Current hardened observer SHA256 `a4ba1500f756b81810a40386b8d37782cdb010a0b22d0eee6062216bf23c6fb4` was inspected after the stress5 run closed. It refreshes each parent before enumerating children, rejects absent/DEFUNCT state for each visited node and rechecks the exact field name and FOCUSED state immediately before recording. No new interaction, timeout/budget or State-assertion change was introduced. The static cache/membership finding above is closed for this implementation.

Personally read the retained actual-AST adversarial checker, then independently reexecuted its eight cases with the output-writing statement excluded, writing only this reviewer's owned result. Live exact focus is accepted; retired child, defunct field, defunct parent, focus lost during ancestry, renamed control, duplicate focused names and foreign app are rejected. The checker enforces fresh parent cache and retains the original Up/row100 event separately from the requested settled row99 observation. All eight isolated cases pass, and observer source remains unchanged. Result `quote-focus-hardened-result.json` SHA256 `6a2ab508ad5acdf4835ef6bd7c8ead198b481cf7b9ec839c17e428395f732296`.

This closes only static logic qualification. It does not relabel stress5 with hardened bytes: that actual50-check run used observer398998 and its fallback count was zero. No native fallback exercise or performance result is claimed.
