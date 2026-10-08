# M5 local workflow checkpoint

Local workflow checkpoint: `sarv-m5-workflow-local` on `codex/accounting-baseline-m1`. Native qualification, one final performance epoch and independent final review are complete. Full PERF/G4 release acceptance remains partial; this checkpoint does not claim production or statutory acceptance.

This bounded native workflow connects goods orders, partial paired delivery/invoices, receivables and original-cost restock/credits. Orders and saved drafts have no financial effect. Operational and financial documents retain distinct linked identities. The existing `core-policy-v1` remains explicit: integer IRR, perpetual stock, moving weighted average and original-cost returns. Actual statutory tax integrations are excluded by user instruction.

## Evidence checklist

| Scope | Coverage and evidence |
| --- | --- |
| Exact goods posting, original allocations, duplicate-safe natural identity, locks and immutable history | Implemented; independent candidate5 financial/data7 proof retains its exact financial-source attribution. Current unchanged financial paths are independently compared. Full SAL/PUR/INV remain partial. |
| Schema5→6, original verified recovery, old M4 refusal, fresh restore and interrupted writes | Evidenced in candidate5 independent proof; C12 actual old-writer checks pass. Hardware power-loss and disk-full remain untested. |
| C12 release, source and verification provenance | Completed: full36 production and53 verification files before/after build, immutable executable. [Manifest](../docs/assets/accounting-m5/checks/candidate12/manifest.json). |
| C12 fmt, strict Clippy and tests | Completed: ordinary220 passed/0 failed/4 ignored,35 suite summaries; four separate opt-in tests pass. [Root7 receipt](../docs/assets/accounting-m5/checks/candidate12/gates/manifest.json). |
| Async quotation save and callback/focus integrity | C12 callback13 pass, including held writes and newer interaction precedence. [Receipt](../docs/assets/accounting-m5/checks/candidate12/quote-callback/manifest.json). |
| Current native sales, narrow layout and material GTK diagnostics | C12 fresh83 pass with fatal criticals enabled and no material GTK diagnostics; reused-row identity37 passes. Rejected C10 and diagnostic C11 remain immutable. |
| Preserved setup/Jalali/sidebar/manual books/purchases/quotations/settings/held-close paths | Fresh C12 seven-workflow qualification passes: core27/Jalali44/sidebar26/ledger16/purchases30/quotation23/held-close3. [Receipt](../docs/assets/accounting-m5/checks/candidate12/preserved-gates/manifest.json). Earlier C9 guards and results retain their original attribution. |
| Performance and independent checkpoint review | C12 measurement complete: functional/provenance PASS; strict performance PARTIAL/FAILED. Independent final timing review passes for the bounded workflow checkpoint. Historical C8 strict FAIL remains visible; no production, full G4 or tax-compliance claim. |

## Measured limits

Declared shared executor: four CPU equivalents,16GiB RAM, EPYC9V74 KVM, Debian13.6, GTK4.22.5/libadwaita1.9.4, private X11/Cairo. Six n10 startup p95 values are0.90–1.09s; initial RSS p95 ranges95–173MiB. Four n20 focused lookup p95 values are101–135ms;1,000-line navigation p95 is98ms visible/100ms focused across39 transitions. These are cloud measurements, not dedicated reference-hardware or physical-cold-cache proof.

Fiveof22 quotation action scopes miss50ms: three pages51.489/51.003/51.905ms, choice73.664ms and internal-helper PDF80.439ms. Broad global/PDF/persistence probes remain289.750/107.152/96.586ms and contain disclosed fixture/render/observer confounds; their misses are retained. Full quote later RSS/HWM is331/376MiB, separate from initial RSS. The [complete final report](../docs/assets/accounting-m5/performance/final-candidate12/README.md) preserves raw samples, terminal segments, exact hardware/source attribution and unmeasured conditions. No budget was relaxed.

## Synthetic demonstration

1. Use a disposable copy of the protected M4 fixture; confirm inventory480IRR, AR200, AP329 and GRNI50. Open **فروش‌ها**.
2. Select the customer, item A, warehouse W1 and catalog price directly in the order form. Enter10 pieces at20IRR and1IRR document discount. Save incomplete raw input, restart and resume; correct it and preview199IRR before submitting. No journal or stock movement is created by the order.
3. Fulfill5 pieces: inspect separate delivery/invoice identities, net100IRR and cost58IRR. Identical same/different-key retry must return the original effect.
4. Return2 and then3 from the first invoice. Credits are40/60IRR; original costs are23/35IRR. Fulfilled order remaining stays5 throughout both returns.
5. Fulfill the final5: net99IRR and cost58IRR. Final inventory422, AR299, net COGS58, earnings41 and trial balance1630/1630 must match independent detail. Use source drill-down, search, keyboard focus and narrow layout.
6. Export, close, restart and restore a checksummed backup to a fresh disposable profile; compare the entire durable state and controls, rather than totals alone.

Sample [invoice PDF](../docs/assets/accounting-m5/exports/candidate1/invoice.pdf), [credit PDF](../docs/assets/accounting-m5/exports/candidate1/credit.pdf) and [control PDF](../docs/assets/accounting-m5/exports/candidate1/sales-reconciliation.pdf) retain original candidate1 export attribution. Every page of those12 PDFs/15 pages was visually inspected; unchanged exporter source is separately attributed. They are synthetic accounting examples, not statutory invoice certification.

## Native before and after

[Before: clipped narrow source card](../docs/assets/accounting-m5/native/candidate9/main/full/M5-sales-light-650x550.png) → [after: reachable focused source action](../docs/assets/accounting-m5/native/candidate12/main/full/M5-sales-light-650x550-focused-source.png). These650×550 captures differ in focus/scroll position; they demonstrate layout and reachability, not controlled speed.

Fresh [dark empty search](../docs/assets/accounting-m5/native/candidate12/main/full/M5-sales-dark-650x550-zero.png), [saved narrow editor](../docs/assets/accounting-m5/native/candidate12/main/stage/M5-stage-narrow-editor.png) and [final detail page](../docs/assets/accounting-m5/native/candidate12/main/detail-pages/M5-real-detail-last-page.png) have [root visual inspection](../docs/assets/accounting-m5/native/candidate12/root-visual-inspection.json) and separate actual native keyboard/focus checks. GTK interaction proves control behavior; exact raw-state/oracle proofs establish accounting correctness.

The [full requirement map](evidence.md), [fixture checklist](fixtures-M5.md), [performance report](performance-M5.md), [independent review](independent-review-M5.md) and [roadmap](roadmap.md) distinguish completed bounded workflows from partial/missing/untested broader requirements. Next connected dependencies are purchase returns and remaining inventory workflows, followed by treasury; advanced/statutory modules remain separately gated. No remote push, merge or hosted workflow is authorized or performed.
