# AGENT_PLAN

Living memory for the quality loop. Update after every finished backlog item.

## Product Understanding

Sarv is Iranian double-entry accounting. `crates/sarv-quote` is the engine; `crates/sarv-desktop` is the native Rune UI. The UI calls `sarv_quote::desktop::dispatch` only — it does not invent journal lines, tax, or document ids. Preview before post; pass `expected_generation`; reuse dialog `request_key`s. Money: sales drafts send the typed string with `display_unit`; purchases/payments/journals/banking parse rials. Do not bump the Rune pin.

## Backlog

1. Out of scope for this loop: payroll, bank feeds, card processors, team server.

## In Progress

_(idle — quality loop items from the accepted plan are finished)_

## Done

- `dbd642db` — docs: add AGENT_PLAN.md for the quality loop
- `2dd9ffdd` — fix(desktop): unmount finished setup guide in Auto mode
- `ca91d1e4` — fix(desktop): make dialog panels solid layer_01
- `6cfa45ec` — fix(desktop): widen account pickers to 420
- `b2a30ba1` — fix(desktop): focus POS search when the page opens
- _(this commit)_ — test(desktop): assert sheet dirty clears after save
- GPU snaps (`SARV_DEMO=1`): overview shows guide at 4/5; invoice dialog panel samples solid `#121314` (no glass bleed). Suite: `cargo test -p sarv-desktop --bin sarv-desktop` — 30 passed.

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

## Open Risks

- Demo fixture is intentionally 4 of 5 setup steps; guide-hide tests force all five complete in-memory.
- Large uncommitted Rune migration tree exists beside this loop; many `crates/sarv-desktop` support files remain untracked outside the quality-loop commits.
