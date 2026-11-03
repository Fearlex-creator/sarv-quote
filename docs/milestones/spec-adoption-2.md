# Spec adoption, pass 2 (2026-11-03)

Owner decisions applied: cheque register (ADR-0005), incremental validation with a safety net (ADR-0007),
legal values gated behind "not reviewed" (ADR-0004), AGPL-3.0-only licence (ADR-0006), commits by area on
branch `spec-adoption`, e2e harness fixed, `desktop/.preview-*.mjs` ignored by lint.

## Verification (measured, clean runs)
- `cargo test -p sarv-quote`: **264 passed, 0 failed, 3 ignored** (46 suites); with `--features pdf-export`:
  **307 passed, 0 failed** (a stale `BusinessProfile` initializer in `tests/sales_exports.rs` was fixed).
- `pnpm --dir desktop run check` clean · `run lint` clean · `run test:unit` 14 passed · `run build` succeeds.
- `pnpm --dir desktop run test` (Playwright): **25 passed, 0 failed**. The harness now starts the seeded Rust QA
  adapter (repo root as working directory) and the dev server with the QA variables, and finds an installed
  Chrome/Chromium. Two specs had drifted from the redesign (sidebar starts collapsed; Contacts renamed) and were updated.
- Mutation checks (break the code, see the test fail, restore): payable sign in reconciliation, clearing-direction
  rule, a skipped validation group, aging bucket boundary.

## Measured effect of incremental validation
Tiny edit save: 1k products 3.8 → 1.9 ms; 10k 50 → 20 ms; 50k 220 → 110 ms.

## Found while building
- A negative sign was silently dropped by digit normalisation (caught by a failing test; fixed).
- Role names serialise PascalCase (`"Bank"`), which the e2e caught in the clear-cheque dialog.
- The generic payment "Reverse" button could desynchronise a cheque's status from the ledger; reversing a
  cheque's payment outside the register is now rejected by validation and the button is hidden.
- The agent service finds `target/release/sarv-agent` relative to the working directory (harness fix).

## Not done
See "Open items and next slices" in `../SPEC-COVERAGE.md`. Tafsili/dimensions, Moadian invoicing, payroll runs,
fixed assets, multi-currency, sync/team mode, roles and approvals remain not started.
