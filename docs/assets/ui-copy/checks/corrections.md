# Qualification corrections

The first compile check passed; strict Clippy caught a single-element loop in the updated native settings smoke. It was replaced by direct field selection. Subsequent final4 qualification passes.

The first native sidebar run reached17 successful checks, then exposed a navigation-focus assumption after the menu reorder. The integration lead deliberately added conditional current-row focus when keyboard focus is already inside the sidebar. The independent reviewer approved its source logic; body focus remains outside that predicate.

The next sidebar observer reached36 successful checks and then raised KeyError while constructing its expected State: the untouched empty profile omits Settings, whereas a successful settings save materializes the existing default fields. The observer now supplies those explicitly in its independent expected State. No production/settings change was needed for this observer correction. Final native results are recorded separately; failed attempts are not counted as passing acceptance.

Earlier qualified intermediates are summarized alongside this file; final4 is the sole current frozen source/build acceptance archive. The historical M6 source, metrics and artifacts remain unchanged.
