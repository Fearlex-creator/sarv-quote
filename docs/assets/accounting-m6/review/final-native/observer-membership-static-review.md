# Bounded independent measurement-observer repair review

Reviewer `m6_final_native_review`; read-only production/observer inspection, no Cargo or GUI execution. Actual archived file is `checks/candidate6/runtime-verification3-bytes/scripts/native-performance-M6.py` (SHA256 `540cd09dc0ee2c00267ccf39e90e66ed110bffb5ab37b5bde0be82ce61620197`). Current repaired observer SHA256 `38063e556740904a129b659f57b49a4cdd010a02385063c469ac0407d4bac6a1`.

Personally inspected the complete diff and surrounding caller/membership/readiness code. The formerly fatal retired-row membership assertion now rejects that observation with `None`. All three callers treat this as not-ready: `source_result` returns `None`, exact-zero warmup returns `False`, and original-source selection returns `None`. They retry a new bounded walk rather than reuse stale results. Parent-child membership refresh/DEFUNCT checks and the at-most500-node walk/25-action bounds remain strict.

Existing query text, exact status/count/page, exact isolated expected reference, native readiness and actual focus predicates remain unchanged. Rejections record event, monotonic timestamp, root/rows identities and each observed node's name/role/current membership. Protocol is explicitly incremented to `m6-scoped-v1.1`. No budget, deadline, sample size, quantile method or phase acceptance threshold changed.

Static repair is accepted for this narrow race-handling defect. It does not establish that any performance case is measured, within budget, or production-ready. The earlier pilot failure remains valid historical evidence. A fresh complete pilot and quiet final measurement epoch are still required.

Runtime4 manifest SHA256 `bd13107baf91622667abb21eae1f365d6913229674f30bc99ce395959f7e28fc`; independently rehashed all74 current files and matched at this review. This is a new observer epoch, separate from runtime3 attribution of closed native run-sjxogn6e.

Also sanity-checked `requirements/checkpoint-M6.md`: all local Markdown targets resolve. Its current78/34 counts, synthetic billed/unbilled literals, immutable source/binary fingerprints, partial PUR/INV and pending pilot/final epoch match reviewed evidence. Its statement that independent all-page export review is pending can now be narrowed: actual chooser physical pages1–2 and supplier-credit page1 were personally reviewed in candidate6; full sample regeneration/other document review remains separately pending. No broader qualification completion is inferred.
