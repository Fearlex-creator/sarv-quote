# Candidate6 runtime3 functional pilot1 — failed, incomplete

One authorized `--pilot` invocation through the frozen coordinator closed with
`observer_failed`. Both coordinator and observer before/after provenance guards
pass. No second invocation ran. This n1/n2 diagnostic supplies no performance
acceptance and no strict-budget waiver.

Raw immutable output: `/workspace/sarv-quote/artifacts/performance-m6/candidate6-runtime3-pilot1`.
The portable copy retains all seven raw JSON/log files, coordinator/receipt/analyzer
bytes, strict analysis and raw stopped-profile State. SQLite/WAL/SHM are excluded.
The stopped profile equals the prepared source120-billed State exactly and SQLite
integrity is `ok` ([check](stopped-profile-check.json)).

The observer stopped at `source_rows`' current-parent membership assertion during
the exact zero-result warmup. No query/page/input/preview/post/startup sample
completed; full200-entry/400-detail/offpage-invalid/post/restart proof remains
pending. [Raw observer report](observer/performance.json), [coordinator](coordinator.json)
and [traceback](coordinator-observer.log) retain the failure. [Analysis](analysis.json)
reports PARTIAL/incomplete, not a latency pass. The application log is empty;
the wrapper retains an AT-SPI `dbind-WARNING` cache `GetItems` message. Xvfb/Openbox
environment logs remain separately visible.

Static source inspection shows a persistent picker rows GtkBox whose labels and
actions are cleared/recreated asynchronously on search changes. A possible
observer race is collection of a row just before replacement followed by a hard
assertion when its membership correctly becomes false. This is a hypothesis,
not established product innocence or a reproduced native diagnosis. A proposed
repair rejects that observation as not-ready and retries a fresh bounded subtree
while retaining exact current membership/query/count/page/reference/readiness
predicates. No observer edit, relaxed predicate or GUI replay is authorized by
this report. Root must coordinate any correction and issue a fresh verification
freeze and separate pilot GO.

All historical M5 strict misses and every M6 budget remain unchanged. Final quiet
n10/n20 measurement remains unauthorized pending complete functional qualification
and independent/preserved-workflow review.
