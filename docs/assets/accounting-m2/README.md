# Native M2 evidence

Synthetic fixtures only. This bundle accompanies local source tag `sarv-m2-local` on `codex/accounting-baseline-m1`, starting from M1 commit `d640288d90d9f21789cd213ecbaa85e16c69bcb5`. Nothing was pushed, merged or deployed. See [milestone and demonstration](../../../requirements/milestone-M2.md), [independent review](../../../requirements/independent-review-M2.md), [performance methodology](../../../requirements/performance-M2.md) and [migration/recovery](../../../migrations.md).

Final release executable SHA-256: `6c545a5642c53ac633a715f66f5f24cdfd107ccd58261724b688a89f1793a50d`. The executor retains it at `/workspace/sarv-evidence/bin/sarv-M2-layout-final`. Rebuild with the existing activated toolchain and `cargo build --release --offline --locked`; native harnesses require a graphical session. No executable, database, D-Bus runtime or credentials are committed in this bundle.

## Evidence index

- `checks/`: 141 ordinary Rust tests, two separately passing opt-in tests, format, strict Clippy, release build, quotation/restart and held-lock close logs.
- `jalali/`: 44 final-build checks, real keyboard events, standard/narrow forms, exact trial-balance PDF/CSV and fresh-profile restore evidence. Its financial fixture uses canonical opening/adjustment/reversal dates `2026-03-21`, `2026-04-21`, `2026-04-22`; the manual milestone demonstration separately proposes consecutive March days.
- `accounting/`: 16 final-build regression checks against the existing manual-ledger workflow; exact 1,251/349 amounts and 125.1-toman final balance.
- `sidebar/`: 19 final-build checks, light/dark/standard/narrow native screenshots and actual Tab/Space focus/activation events. GTK focus geometry is retained; custom tab AT-SPI coordinates were zero and are not presented as valid screen coordinates.
- `quotation/`: 40 checks, three retained quotes at unchanged generation 37 after restart, native screenshots and sample business PDF/CSV.
- `migration/`: verified schema-2 recovery and current schema-3 backup; three quotes and three journals preserved at generation 17→18, repeat-open 18 and exact fresh restore.
- `close/`: actual SQLite write-lock plus XTest text/Alt+F4; close waits for durable save and restart retains exact input.
- `review/`: independent ICU expectations for all 109,573 supported dates and 300 fiscal-year ranges; rerunnable Cargo probe, historical quote/export checks, actual M1 recovery/rejection, final source hashes and 15 final-build scale checks. The 64 native adaptive-tab checks use `86e32433…`; the earlier 24 financial checks use `85223898…`. Their unchanged component attribution is explicit in the review.
- `scale/`: convenient copies of the final independently checked 50,000-product/1,000-line native report, screenshots, exact 100,802,501-IRR PDF and raw-state/integrity checks.
- `performance/`: final-build repeated startup/lookup samples, hardware/runtime, initial memory, state-equality proof and representative native captures. Process-cold startup and warm/uncontrolled OS pages are distinguished from untested machine-cold launch.
- `before/`, `before-width-fix/`, `review/scale-adaptive-previous/`: predecessor evidence. These are not final-build results.

Use [manifest.json](manifest.json) to verify every retained file. Source provenance uses a tag and content hashes, avoiding a self-referential commit hash. Resolve the actual local commit with `git rev-parse sarv-m2-local`.

## Limits

The calendar is supported for Jalali years 1200–1499. Canonical dates, exact money and issued snapshots are preserved. Strict 50/150-ms responsiveness budgets remain unmet, and the full five-year/100k-document/one-million-journal-line reference dataset is untested. Migration interruption, hardware power loss, company timezone/midnight policy, screen readers, enlarged/high-contrast text, physical printing, Wayland, Windows, packaging and professional production accounting acceptance remain open. Current books are a manual ledger; connected sales/purchases/inventory, complete opening subledgers, treasury/cheques, closing and permissions remain tracked. Actual legal/tax integrations are deferred. No universal parity, production readiness or statutory compliance is claimed.
