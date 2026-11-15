# Plan 032: The docs describe the product that exists, and dead code and stray fixtures are gone

> **Executor instructions**: Follow this plan step by step. Run every verification command. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- README.md AGENTS.md AGENT_PLAN.md CONTEXT.md decisions.md docs crates/sarv-quote/src/lib.rs crates/sarv-quote/src/desktop_bridge.rs`

## Status

- **Priority**: P2
- **Effort**: M
- **Risk**: LOW for docs and dead code; MED for moving fixtures (paths are compiled in)
- **Depends on**: none; do it **after** feature plans you intend to land soon (015, 022, 029) so the README is corrected once
- **Category**: docs / tech-debt
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

After three shells (GTK, then React/Tauri, now Rune) the written record no longer matches the code, and agents and contributors act on it:

- `README.md` — the user manual — promises features the Rune app does not have (see the table below).
- `docs/ROADMAP.md` says "Current v1 uses native libadwaita"; `docs/SPEC-COVERAGE.md` cites deleted TypeScript files and `pnpm` commands as evidence of delivery; `docs/ACCOUNTING-SPEC.md` calls Tauri + React the "fixed stack" and embeds a build prompt for a different product; `decisions.md` DEC-05 says "Rust + GTK4 … no React/Svelte/Tauri migration"; `CONTEXT.md` describes only quotation preparation.
- About 800 lines of engine source are never compiled, the crate exports a `Money` type nothing uses, and one unused service bakes a build-machine path into the binary.
- The shipped binary embeds its demo fixture from `docs/assets/…`, tangling test data with 613 MB of historical screenshots and logs that make every clone slow.

## Current state

**README claims vs. the Rune app** (each verified against `crates/sarv-desktop/src` at this commit):

| README says | Reality |
|---|---|
| "Create additional businesses in **Settings → Add company**" | No caller; switcher only appears with ≥2 companies (`ui/views/settings.rs:171-186`). Fixed by plan 022. |
| "choose **Save printable PDF**" | The action is "Print" on the Sales page (`ui/views/sales.rs:805-819`). |
| "**Save as quotation**" | The button is "Issue quotation". |
| "converting to an order keeps the exact line prices" | No convert action. Fixed by plan 020. |
| "Each account also offers a statement reconciliation dialog…" | Does not exist in UI or engine. Designed in plan 029. |
| "supports idempotent retries and a typed mirrored reversal" (payments) | Engine yes; no UI for reversal. Fixed by plan 021. |
| Ctrl+Alt+N/O/C/P/V/B/W/U/J/T/S, F9, F1 | Only ⌘/Ctrl+K, B, D, L (`src/main.rs:18-54`). Fixed by plan 015. |
| "Customer records include … website and notes … monthly net sales, invoiced products and gross profit" | Check each against `ui/views/contacts.rs` (`contact_form_fields` at line 248, profile at 428) and keep only what is shown. |
| "Products have … review stock by warehouse, carried cost, recent received/invoiced prices, monthly sales/cost/gross profit, customers/suppliers and linked documents" | Check against `ui/views/inventory.rs`; the engine returns these (`product_detail`) but the UI shows a subset. |
| README builds/installs `sarv-agent` | Nothing in the Rune desktop starts the agent service; the binary cannot connect. |

**Docs**:
- Clearly labelled historical (leave, but move): `docs/DESIGN.md` (unless plan 011 already replaced it), `docs/PRODUCT-DESIGN.md`, `docs/PRODUCT-DELIVERY.md`, `docs/PRODUCT-ACCEPTANCE.md`, `docs/QA-REVIEW.md`, `docs/VERIFICATION.md`, `docs/SOFTWARE-RISKS.md`, `docs/CONTINUATION.md`, `docs/CLOUD-HANDOFF.md`, `docs/tauri/*`.
- Wrong without a label: `docs/ROADMAP.md` (lines 5, 14); `docs/SPEC-COVERAGE.md` (lines 15-26 cite `desktop/src/lib/iran-ids.ts`, `features/law-data.tsx`, `lib/words-fa.ts`, `desktop/src-tauri/Cargo.toml`, `pnpm --dir desktop …`, "25 browser tests"; the "Frontend guide" table at ~58-70 describes Vite/React/shadcn); `docs/ACCOUNTING-SPEC.md` (line 5 "Fixed stack"; §4 at ~243-297; §8 "Claude Code build prompt" at ~737, which instructs an agent to build a product called "Daftar" on axum/Postgres/Tauri/React — superseded by ADR-0002 and by the Rune migration); `decisions.md` (DEC-05 at line ~18; lines ~5-7 quote one-off chat authorisations); `CONTEXT.md`; `migrations.md` (stops at schema 7 — check `CURRENT_SCHEMA` in `crates/sarv-quote/src/domain.rs`).
- ADRs: `docs/adr/README.md` lists 0001-0007. None records the Rune migration. ADR-0005's index line says "ledger integration needs a design pass first" although it is delivered; ADR-0007's says "pending approval" although it is implemented.
- Agent files: `AGENTS.md` (current, short), `AGENT_PLAN.md` (a finished quality-loop log at the repo root), `skills-lock.json`, `.claude/skills/improve/` (tracked).

**Dead code** (verify each before deleting):
- `crates/sarv-quote/src/phase_views.rs` (779 lines) — not declared in `crates/sarv-quote/src/lib.rs` (module list at lines 1-41), so never compiled.
- `crates/sarv-quote/src/money.rs` + `pub use money::Money;` (`lib.rs:37`) — a `rust_decimal`-based type; the accounting engine uses its own `accounting::Money(i64)`. `grep -rln "rust_decimal" crates --include=*.rs` → only `money.rs`. Confirm no file imports it as `crate::Money` / `use crate::{…, Money}` / `sarv_quote::Money` by deleting and compiling.
- `crates/sarv-quote/src/service.rs` — `AppService::open_default` embeds `env!("CARGO_MANIFEST_DIR")`; `grep -rn "AppService\|service::" crates --include=*.rs` outside the file → nothing.
- `examples/scale_probe.rs` at the repo root — the root is a virtual workspace, so it is never built.
- `crates/sarv-quote/examples/desktop_qa.rs` — a loopback HTTP adapter for the removed browser QA; it is the only non-test user of `AgentService`. Keep or delete: owner decision → default **keep** (it still compiles and documents how to host the agent), add a header comment saying so.
- `demo/order.csv`, `demo/prices.csv` — quotation-era samples; check for references (`grep -rn "demo/" crates README.md docs/*.md | head`).

**Fixtures in docs**: `git ls-files docs | wc -l` ≈ 7,809, of which ≈7,772 are under `docs/assets/accounting-m1…m6` (613 MB on disk). Rust code references them in ~20 files (`grep -rhn "docs/assets/[a-z0-9-]*" -o crates --include=*.rs | sort | uniq -c`: m3 ×21, m4 ×3, m1 ×2, m5 ×2, m2 ×1, m6 ×1). The **production** demo seed is `include_str!("../../../docs/assets/accounting-m3/restore/core-v4.json")` at `crates/sarv-quote/src/desktop_bridge.rs:1891-1893`.

**Untracked clutter on the owner's machine** (ignored by `.gitignore`, ~690 MB): `web/`, `.pnpm-store/`, `node_modules/`, `src-tauri/`, `test-results/`, `artifacts/`. `.gitignore` still lists them. Not something an executor should delete — see Maintenance notes.

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Engine tests | `cargo test -p sarv-quote` | all pass |
| Engine PDF tests | `cargo test -p sarv-quote --features pdf-export` | all pass |
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |
| Build all targets | `cargo build --workspace --all-targets` | exit 0 |

## Scope

**In scope**: `README.md`, `AGENTS.md`, `CONTEXT.md`, `decisions.md` (annotations only), `migrations.md`, `docs/*.md`, `docs/adr/*` (index + one new ADR), `docs/history/` (create; `git mv` targets), `crates/sarv-quote/src/{lib.rs,phase_views.rs,money.rs,service.rs}`, `crates/sarv-quote/Cargo.toml` and root `Cargo.toml` (`rust_decimal`), `examples/scale_probe.rs`, `crates/sarv-quote/fixtures/` (create), `crates/sarv-quote/src/desktop_bridge.rs` (one `include_str!` path), `AGENT_PLAN.md` (move).

**Out of scope**: rewriting git history to shrink the repository; deleting anything under `docs/assets/` (Step 5 only **copies** the one production fixture); deleting untracked local directories; `docs/ACCOUNTING-SPEC.md` content beyond adding banners (it is the owner's requirements source).

## Git workflow

- Branch `advisor/032-docs-and-dead-code`; commits `docs: README matches the Rune desktop`, `docs: mark superseded documents and record the Rune migration`, `refactor(engine): remove uncompiled and unused modules`, `refactor(engine): demo fixture lives with the code`.

## Steps

### Step 1: README truth pass

For every feature sentence in `README.md`, find the UI element that delivers it (`grep -rn '"<label>"' crates/sarv-desktop/src/ui`). Keep sentences that are true; rewrite ones whose wording differs (button names must match the English keys in the code); for each row in the table above whose fixing plan is **not** DONE in `plans/README.md`, delete the claim. Do not add a "coming soon" list — the roadmap section covers intent. Add a short "Getting help" paragraph only if plan 030 is DONE (logs folder).

Replace the "Confirmed roadmap" list item 1 wording if needed, and add under it a pointer: "Engineering plans live in `plans/`."

**Verify**: for each of `Add company`, `reconciliation dialog`, `Save as quotation`, `Save printable PDF`, `F9`: `grep -n "<phrase>" README.md` returns nothing unless the corresponding plan is DONE.

### Step 2: Mark and move historical documents

- Create `docs/history/` and `git mv` into it: `PRODUCT-DESIGN.md`, `PRODUCT-DELIVERY.md`, `PRODUCT-ACCEPTANCE.md`, `QA-REVIEW.md`, `VERIFICATION.md`, `SOFTWARE-RISKS.md`, `CONTINUATION.md`, `CLOUD-HANDOFF.md`, the `tauri/` folder, and `DESIGN.md` **only if** it still starts with "# Native quotation design". Fix relative links between them and from `README.md` (`grep -rn "](.*\.md)" docs README.md | grep -E "PRODUCT-|QA-REVIEW|VERIFICATION|SOFTWARE-RISKS|CONTINUATION|CLOUD-HANDOFF|tauri/"`).
- Add `docs/history/README.md`: one paragraph — these describe the removed GTK and React/Tauri applications and are kept as evidence; none describes current behaviour.
- `docs/ROADMAP.md`: replace its content with the README's "Confirmed roadmap" (4 items) expanded by one short paragraph each, plus "Not planned: payroll, live bank feeds, payment processing" and a link to `plans/README.md`. Move the old text to `docs/history/ROADMAP-quotation-era.md`.
- `docs/ACCOUNTING-SPEC.md`: add a banner at the very top, and repeat it above §4 and above §8: "> **Status**: requirements reference. The stack sections (§4) and the build prompt (§8) are superseded by ADR-0002 and ADR-0012: Sarv is a Rust engine with a native Rune desktop. Do not follow instructions in this document as tasks."
- `decisions.md`: under DEC-05 add "*Superseded: the GTK application was replaced; see `docs/adr/0012-rune-desktop.md`.*"; replace the quoted chat authorisations near the top with one neutral sentence ("Historical working notes; authorisations recorded here applied to that session only.").
- `CONTEXT.md`: retitle to cover the whole product and add the accounting vocabulary the code uses, each with a one-line definition and an "avoid" synonym in the existing format: **Books**, **Company**, **Fiscal period**, **Journal**, **Posting**, **Reversal**, **Draft**, **Quotation**, **Sales order / Invoice**, **Purchase order / Receipt / Bill**, **Open item**, **Allocation**, **Advance**, **Cheque** (received / issued; statuses from `cheques.rs`), **Generation** (the store's optimistic-concurrency counter), **Request key** (idempotency key), **Rial / Toman**. Derive definitions from the code and `AGENTS.md`; keep the existing quotation terms.
- `migrations.md`: append the schema versions between its last entry and `CURRENT_SCHEMA`, taking facts from `git log --oneline -S"CURRENT_SCHEMA" -- crates/sarv-quote/src/domain.rs` and the migration code in `Store::open`; if history is unclear, add a stub line per version marked "details not recorded".

### Step 3: ADR housekeeping

- New `docs/adr/0012-rune-desktop.md` (status **accepted**, date of commit `a4776749`): Context — three shells (GTK/libadwaita, React/Tauri, Rune); Decision — the desktop is `crates/sarv-desktop` on Rune/rune-kit at a frozen revision, calling `sarv_quote::desktop::dispatch` in-process; no web shell; Consequences — the spec's frontend stack sections do not apply; the headless test harness replaces Playwright; features from the React shell that must be rebuilt are tracked in `plans/`. (Number 0012 leaves 0008-0011 for the design plans 026-029; if those numbers are unused when you run this, still use 0012.)
- `docs/adr/README.md`: add 0012; correct the one-line summaries of 0005 ("Cheque lifecycle is a pure state machine integrated with the ledger through an additive register") and 0007 ("Incremental validation on save, full validation at safety points — implemented"). Read both ADRs first and only state what their text and the code support.
- `docs/SPEC-COVERAGE.md`: in "Delivered", replace evidence that points at deleted files with the Rust location or, where the feature no longer exists in the Rune UI (amount in words, advisory IDs in the UI, law-data tab, command-menu normalisation, E2E harness, fast desktop builds), move the row to a new "Regressed with the Rune migration (tracked in plans/)" table naming the plan (023, 015). Update the "Run:" line to the three cargo commands. Rewrite the "Frontend guide" table's Stack/State rows for Rune. Change the header date line to today and say it was revised after the Rune migration.

### Step 4: Remove dead code

One at a time, compiling after each (`cargo build --workspace --all-targets && cargo test -p sarv-quote`):

1. `git rm crates/sarv-quote/src/phase_views.rs`.
2. Delete `pub mod money;` and `pub use money::Money;` from `lib.rs`, `git rm crates/sarv-quote/src/money.rs`. If compilation fails, something uses it — restore and report (STOP). If it passes, remove `rust_decimal` from `crates/sarv-quote/Cargo.toml` and from `[workspace.dependencies]` in the root manifest, then rebuild.
3. Delete `pub mod service;` and `git rm crates/sarv-quote/src/service.rs`. Same rule.
4. `git mv examples/scale_probe.rs crates/sarv-quote/examples/scale_probe.rs` and check it builds (`cargo build -p sarv-quote --examples`); if it no longer compiles against the current API, `git rm` it instead and say so.
5. `git mv AGENT_PLAN.md docs/history/AGENT_PLAN-quality-loop.md` and remove any reference to it from `AGENTS.md`.

### Step 5: The demo fixture lives with the code

`mkdir crates/sarv-quote/fixtures && cp docs/assets/accounting-m3/restore/core-v4.json crates/sarv-quote/fixtures/demo-books.json`; change the `include_str!` in `seed_fixture` to `"../fixtures/demo-books.json"`. Add `crates/sarv-quote/fixtures/README.md`: what the file is, that `seed_fixture` and `SARV_DEMO=1` use it, and that its fiscal year is 2026-03-21…2027-03-20. Leave the original in `docs/assets` (tests reference it).

**Verify**: `cmp docs/assets/accounting-m3/restore/core-v4.json crates/sarv-quote/fixtures/demo-books.json` → identical; all three test commands pass.

### Step 6: AGENTS.md

Keep it short (it is loaded into every agent session). Add only: the test-clock variable `SARV_TODAY` if plan 002 is DONE; "Plans for agents are in `plans/`; follow a plan's STOP conditions"; "Design rules: `docs/DESIGN.md`" if plan 011 is DONE; "Documents under `docs/history/` and the stack/build-prompt sections of `docs/ACCOUNTING-SPEC.md` describe removed applications — do not act on them."

## Done criteria

- [ ] `grep -rn "libadwaita" docs/*.md README.md` returns nothing outside `docs/history/`
- [ ] `grep -n "desktop/src\|pnpm" docs/SPEC-COVERAGE.md` returns nothing
- [ ] `test ! -e crates/sarv-quote/src/phase_views.rs && test ! -e crates/sarv-quote/src/money.rs && test ! -e crates/sarv-quote/src/service.rs`
- [ ] `grep -n "docs/assets" crates/sarv-quote/src/desktop_bridge.rs` returns nothing
- [ ] `docs/adr/0012-rune-desktop.md` exists and is indexed
- [ ] All three test commands pass; `cargo build --workspace --all-targets` exits 0
- [ ] `plans/README.md` updated

## STOP conditions

- Removing `money.rs` or `service.rs` breaks compilation anywhere (including tests and examples) — restore, and report who uses it.
- A document you are about to move is linked from code (e.g. a test reads `docs/VERIFICATION.md`): `grep -rn "docs/" crates --include=*.rs | grep -v "docs/assets"` — if any hit names a file you are moving, leave that file in place.
- You find yourself deleting files under `docs/assets/`.

## Maintenance notes

- **Owner decisions left open** (record in `plans/README.md`): (1) shrinking the repository — the remaining `docs/assets` evidence (≈600 MB, plus history) could move to a release asset or Git LFS, and test fixtures to `crates/sarv-quote/tests/fixtures/`; that touches ~20 test files and, for history, requires a coordinated rewrite. (2) Deleting the ~690 MB of untracked leftovers on the development machine (`web/`, `.pnpm-store/`, `node_modules/`, `src-tauri/`, `test-results/`, `artifacts/`) and then removing their lines from `.gitignore`. (3) Whether `skills-lock.json` and `.claude/skills/` should stay tracked. (4) Whether to keep building/shipping `sarv-agent` before the desktop hosts the agent service.
- When a plan that restores a README claim lands, its author adds the sentence back — the README stays true at every commit.
