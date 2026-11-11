# AGENT_PLAN

Living memory for the quality loop. Update after every finished backlog item.

## Product Understanding

Sarv is Iranian double-entry accounting. `crates/sarv-quote` is the engine; `crates/sarv-desktop` is the native Rune UI. The UI calls `sarv_quote::desktop::dispatch` only — it does not invent journal lines, tax, or document ids. Preview before post; pass `expected_generation`; reuse dialog `request_key`s. Money: sales drafts send the typed string with `display_unit`; purchases/payments/journals/banking parse rials. Do not bump the Rune pin.

## Backlog

1. Hide the setup guide when all five steps are done in `GuideMode::Auto` (keep `Open` visible).
2. Make dialog panels solid `layer_01` (drop glass/blur on dialogs only; keep menus appended after the panel).
3. Widen account pickers to 420; party/product stay 280; keep `{key}-trigger` as the menu anchor.
4. POS: focus the search field when the page opens (barcode → catalog). No second payment path.
5. Sheets: after a successful save, confirm `dirty` clears; add a test if it does not.
6. Out of scope for this loop: payroll, bank feeds, card processors, team server.

## In Progress

- Creating this memory file and starting item 1 (hide finished setup guide).

## Done

_(none yet)_

## Decisions

- One verified desktop-bin commit per backlog item when clean; otherwise one coherent commit.
- `cargo test -p sarv-desktop --bin sarv-desktop` must stay green between commits.
- GPU snaps (`SARV_DEMO=1`, `--scene invoice` / `--scene overview`) for dialog opacity and guide visibility; needs `required_permissions: ["all"]`.
- Do not bump the Rune pin.

## Open Risks

- Demo fixture is intentionally 4 of 5 setup steps; guide-hide tests must force all five complete without breaking the demo snap that still shows the card.
- Widening account selects to 420 may need a looser menu-alignment assertion if the field center drifts.
- Large uncommitted Rune migration tree exists beside this loop; commits should stay scoped to quality-loop files.
