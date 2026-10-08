# M4 local delivery

Local checkpoint reference: `sarv-m4-local` (created by the integration lead after final verification).

This bounded native milestone connects purchase orders, partial goods receipts,
receipt-matched supplier bills and paired warehouse transfers. It preserves
posted history and the existing quotation application. Sales, returns, treasury,
closing and statutory integrations remain tracked future work.

Final immutable release: `sarv-M4-freeze6`, executable SHA-256
`9be5f0ba6a7fe6f000b722042d426987e79647578ab9ca803a9869ee1a343d5e`,
production source aggregate
`4545e5f018a80a9f403c686ecd19ee2ec66bbc683c6a8d80bb293ba6dd98fc14`.
Only three UI files differ from freeze5; financial and export source remains
identical to independently reviewed freeze4. Existing native5 purchase samples
retain their original attribution. Frozen6 specifically verifies the changed
product/customer persistence routes and preservation of the quotation workflow.

The [milestone checklist](../../../requirements/milestone-M4.md),
[126-ID requirements-to-evidence map](../../../requirements/evidence.md) and
[full dependency roadmap](../../../requirements/roadmap.md) distinguish evidenced
subsets, partial requirements, missing workflows and untested release gates.
The attached requirements and execution brief are specifications; this index
records what the implementation actually demonstrates.

## Demonstration and artifacts

- [Native workflow, exact controls and manual steps](native/freeze5/README.md).
- Before: [M3 wide](native/freeze5/M3-before-light-1280.png) and
  [M3 narrow](native/freeze5/M3-before-light-650.png), rendered by the retained M3 executable.
- After: [Purchases wide](native/freeze5/M4-after-procurement-light-1280.png) and
  [Purchases narrow/dark](native/freeze5/M4-after-procurement-dark-650.png).
- Sample PDFs: [Purchase order](native/freeze5/M4-order.pdf) and
  [end-of-day controls](native/freeze5/M4-controls.pdf).
- [Migration and recovery contract](../../../migrations.md), with actual original
  schema4 recovery, fresh restore, CAS and interrupted-process evidence.
- [Independent review](../../../requirements/independent-review-M4.md).
- [Exact performance results and limits](../../../requirements/performance-M4.md).
- [Local executor checks](checks/README.md):194 ordinary passed,0 failed,
  3 ignored opt-ins; all3 opt-ins separately pass, as do formatting, strict
  all-target Clippy and the release build.
- [Actual frozen6 quotation/save-close](preserved-freeze6/README.md):23+3 checks.
- [Frozen6 external quotation interactions](quotation-focus/freeze6/README.md):
  28+4+3+3 preservation checks and6 scoped Escape checks, with exact keyboard,
  modal/disclosure/failure scope and clean actual application logs.
- [Independent frozen6 picker checks](review/final-freeze6/README.md):17 checks
  over nonempty posted history, real locks/CAS/retry and restart. Timing-edge
  EditableText injections and requested quantity focus are explicitly distinct
  from ordinary real-key interaction and automatic focus return.
- [Picker development diagnostics](native/picker-async-diagnostics/README.md)
  preserve before-fix source, rejected observer attempts and a separately
  attributed candidate-debug product test. Complete release acceptance uses
  the independent frozen6 picker evidence, not that candidate subset.

The [delivery manifest](delivery-manifest.json) hashes selected portable evidence,
current source/tests/check scripts, requirements and bundled font/license files.
It excludes itself and identifies the local checkpoint reference.

Each bundle identifies the immutable executable and source that produced it.
Historical freeze results remain historical; a later binary is never substituted
into their provenance. Original M3 evidence is verified against its Git tag.
Databases, executables, runtime caches and live financial data are excluded.

## Exact bounded accounting example

The synthetic opening has inventory300, AP149, GRNI50 and trial balance1251/1251
in integer IRR. A12-piece order at15IRR totals180 without a journal. Receipts5/75
and7/105 increase inventory and GRNI once. Matched bills30 and150 transfer
received costs from GRNI to AP without another stock receipt. A6-piece transfer
carries70IRR between warehouses without a journal: W1=30pieces/350IRR,
W2=12pieces/130IRR, total inventory480, AP329, GRNI50 and trial1431/1431.
Actual native evidence verifies10 records,5 journals and4 stock movements;
opening, cash, bank, receivables, cheques and capital remain unchanged.

## Measured limits

On the declared cloud executor, repeated process startup p95 is865.763ms core,
1250.606ms large quotation and862.787ms procurement (n10 each). Initial RSS p95
is94068/173756/97200KiB respectively. Physical/cache-cold startup is untested.
Focused core/procurement search p95 is124.378/141.436ms (n20 each), within200ms.
All22 accepted procurement timer phases meet50ms. The50k-product/1000-line
quotation passes15 functional checks but misses strict global280.552/50ms,
unchanged-page161.263/150ms and PDF-busy127.266/50ms targets. Shared
persistence-busy98.725/50ms also misses; this now covers picker work and cannot
isolate Ctrl+S latency. Later quotation RSS268672KiB/HWM319140KiB is separate
from initial memory. See the full report for screenshots/setup confounds, exact
endpoints, source provenance and reference-hardware/dataset/platform limits.

No full-product, production-ready, Iranian-tax-compliant, universal-parity or
universal-speed claim follows. Performance budgets, editable purchase drafts,
additional connected modules, large-catalog narrow-window/error presentation gaps and professional production review remain explicit
in the linked checklist. All checks are local; no hosted workflow, push, merge,
deployment, credentials, billing change or customer message was performed.
