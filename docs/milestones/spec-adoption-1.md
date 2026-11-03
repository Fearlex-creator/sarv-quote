# Spec adoption, pass 1 (2026-11-03)

**Done:** see the first table in `../SPEC-COVERAGE.md`.

**Verification run:** `cargo test -p sarv-quote` 235 passed / 0 failed / 3 ignored (before: unchanged
suites, plus 10 new in `tests/iran.rs`); `pnpm --dir desktop run check` clean; `run test:unit` 14 passed;
eslint clean on all changed files; production `vite build` succeeds; full release `pnpm desktop:build
--no-bundle` succeeds (2m20s).

**Not verified:** the Playwright e2e suite. In this environment every spec that needs the QA backend
times out waiting for workspace data, **identically with and without my UI changes** (baseline run of
`quick-sale.spec.ts` with `desktop/src` stashed). Treat the new UI hints, amount-in-words line and
normalised command search as type-checked and unit-tested but not browser-tested. The repository's
existing `lint` script also fails on git-ignored scratch files `desktop/.preview-*.mjs` (unchanged).

**Deferred, with reasons:** ADR-0002 (stack rewrite), ADR-0005 (cheque ledger integration),
ADR-0007 (incremental validation). Everything else in the spec is tracked in `../SPEC-COVERAGE.md`.

**Skills used:** `codebase-design` (one deep `iran` module behind small functions), `tdd` (hand-computed
goldens, exhaustive transition test), `diagnosing-bugs` (save-latency feedback loop; the sign-loss bug
found by a failing test), `domain-modeling` (roles, ADRs). Installed in `.claude/skills/`.
