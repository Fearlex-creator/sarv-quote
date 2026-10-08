# Prepared observer X11 failure handling and partial checkpoints

Root authorized this bounded observer-only repair after the runtime5 fatal pilot.
No production/schema/native-helper/coordinator/analyzer bytes change. No GUI
replay or build ran. Cleanup stopped only the run's verified three new processes.

[Exact diff](observer.diff) compares runtime5 observer
`38063e556740904a129b659f57b49a4cdd010a02385063c469ac0407d4bac6a1`
to prepared observer
`39409332a7caae630d45cf94858df7dc4baf9c0f56b377f47fbbe757b505fc0c`
(56587bytes), declaring `m6-scoped-v1.2`.

The local `ScopedX11` handler retains the ctypes callback and records every
error's request/minor/error code, resource, serial, exact current phase/property/
owned PID and private-display match. Only BadWindow3/X_GetProperty20/minor0
on the exact `_NET_WM_PID` window during newly owned application discovery in
the private connection returns an empty property result for a fresh discovery
retry. Other errors queue a Python exception so `finally` can preserve the report
and close owned processes. Synchronization drains older requests before assigning
each property scope and receives the current property's errors before checking
its result. This changes lifecycle discovery observation, not application source.

Atomic `performance.partial.json` writes occur after completed source query/page,
input, preview-detail page, preview endpoint, durable-visible post, exact literal/
phase proof, restart and startup terminal observations. The initial guarded report
is written before GUI setup. Each write closes a temporary JSON file then replaces
the partial report, preserving every completed raw array and checkpoint event.
The preview's native verified-phase receipt is awaited before checkpointing; the
post phase receipt is already awaited before its terminal observed endpoint.
No screenshot-capture checkpoint is added inside startup timing. Startup arrays
are checkpointed after screenshot completion,500ms idle, stopped/joined memory
sampling and the exact durable payload check. The raw memory array is copied and
its final count is settled after join, preventing an alias/count serialization race.
Checkpoints and their disk/cache effects between actions are disclosed; they are
outside active timers and supply no power-loss claim. New process records retain
PID/PGID/starttime/command for precise own-session cleanup if needed.

[Static exercises](static-check.json) pass syntax, nine exact/hostile error
classification cases, callback raw-record retention, unexpected-error Python
failure and atomic checkpoint interruption with intact prior complete JSON and
full sample arrays. These isolated exercises use no real display or native
infrastructure and do not establish native repair qualification.

Exact readiness/focus/source/quantity/state predicates, heartbeat positive-tick/
terminal-gap requirements, all thresholds and historical failures remain.
The source search method is unchanged: it includes actual key injection,50ms
configured debounce, layout, fresh descendants and full ancestry membership for
all up-to25 actions, observation retries and focus. Repeated sibling ancestry
RPC work can contribute substantial observer overhead, but the lost runtime5
arrays cannot quantify it or separate product search/render time. The retained
n2 summaries are descriptive observer-end-to-end misses; no subtraction or
causal Rust/product speed claim follows.

Independent reviewer `m6_final_native_review` accepted the bounded static repair
at the exact final observer hash above, independently replaying classification,
callback, Python failure and atomic checkpoint exercises and checking the settled
startup-sampler boundary. This supplies static repair review, not native trap or
performance qualification. Root runtime6 freeze and the [immutable receipt](../candidate6-runtime6-coordinator-receipt.json)
now pass the [full static guard](../candidate6-runtime6-static-guard.json); separate
pilot GO remains pending. The prior fatal pilot and missing-array limitation stay preserved.
