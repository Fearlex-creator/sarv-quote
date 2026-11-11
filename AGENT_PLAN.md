# AGENT_PLAN

Living memory for the quality loop. Update after every finished backlog item.

## Product Understanding

Sarv is Iranian double-entry accounting. `crates/sarv-quote` is the engine; `crates/sarv-desktop` is the native Rune UI. The UI calls `sarv_quote::desktop::dispatch` only — it does not invent journal lines, tax, or document ids. Preview before post; pass `expected_generation`; reuse dialog `request_key`s. Money: sales drafts send the typed string with `display_unit`; purchases/payments/journals/banking parse rials. Do not bump the Rune pin.

## Backlog

1. Settings / Setup / Sheets modules are still largely untracked beside the loop — empty-state polish, backup double-toast, and first-run copy QA once those files are owned.
2. Inventory table lacks an on-page search like Contacts now has.
3. Purchases / Sales return flows: confirm remaining_quantity /1000 and expected_generation on every branch (spot-check after concurrent edits).
4. Concurrent unfinished `show_pos` Settings toggle raced this session — either finish it (default on for demo) or keep POS always in `Page::GROUPS`.
5. GPU snap pass for Payments cheque strip + Accounting empty card after the solid-dialog fix.
6. Out of scope: payroll, bank feeds, card processors, team server.

## In Progress

_(idle)_

## Done

- `dbd642db` — docs: add AGENT_PLAN.md for the quality loop
- `2dd9ffdd` — fix(desktop): unmount finished setup guide in Auto mode
- `ca91d1e4` — fix(desktop): make dialog panels solid layer_01
- `6cfa45ec` — fix(desktop): widen account pickers to 420
- `b2a30ba1` — fix(desktop): focus POS search when the page opens
- `0b367954` — test(desktop): assert sheet dirty clears after save
- `cb289a65` — fix(desktop): pass expected_generation on journal and banking posts
- `e51690e5` — fix(desktop): label cheque status and actions in Payments
- `46126ad2` — fix(desktop): empty states for Accounting and banking dialog titles
- `9d7b074f` — fix(desktop): empty states for Reports P&L, trial, and party tabs
- `9fc72816` — fix(desktop): stop double-toasting failed posts; wire stock transfer generation
- `e3d1d6f0` — fix(desktop): filter Contacts by name, phone, or id
- Suite: `cargo test -p sarv-desktop --bin sarv-desktop` — 34 passed after contacts search.

## Decisions

- One verified desktop-bin commit per backlog item when clean; otherwise one coherent commit.
- `cargo test -p sarv-desktop --bin sarv-desktop` must stay green between commits.
- GPU snaps (`SARV_DEMO=1`, `--scene invoice` / `--scene overview`) for dialog opacity and guide visibility; needs `required_permissions: ["all"]`.
- Do not bump the Rune pin.
- Auto + five steps done unmounts `setup-guide-card`; `GuideMode::Open` still mounts it; Closed keeps the collapsed header so the user can reopen.
- Dialog sheets use solid `layer_01`; overview frost keeps `alpha(0.72)` glass.
- `entity_combo` / `select_field` take an explicit width; `PICKER_WIDTH=280`, `ACCOUNT_PICKER_WIDTH=420`. Journal/banking dialogs widened to 900 so two 420 account fields fit. `{key}-trigger` remains the menu anchor.
- Navigating to POS sets `forms.edit("pos-search").focused = true` so barcode input hits the catalog.
- Sheet `save` already cleared `dirty` on success; the headless sheet walk now asserts that contract.
- Journal/banking/transfer posts copy preview `_generation` into `expected_generation`; journal dialog mints a stable `request_key` on open.
- Cheque strip maps engine status/action keys to FA message keys; `cheque_clear` sends `fund_account_id` from the first Bank (else Cash) account.
- `State::run` already toasts engine errors — callers must not `toast_err` again after `run`.
- Pages reviewed (2026-11-12): Overview, Sales, POS, Purchases, Payments, Banking, Contacts, Inventory, Accounting, Reports, Sheets, Settings, Setup, shell/palette/dialogs/toasts.

## Open Risks

- Demo fixture is intentionally 4 of 5 setup steps; guide-hide tests force all five complete in-memory.
- Large uncommitted Rune migration tree exists beside this loop; many `crates/sarv-desktop` support files remain untracked outside the quality-loop commits.
- Another agent concurrently edited `shell.rs` / `app.rs` / Settings `show_pos` during this session; POS must stay reachable for the desktop suite.
