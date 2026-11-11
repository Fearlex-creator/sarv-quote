# AGENT_PLAN

Living memory for the quality loop. Update after every finished backlog item.

## Product Understanding

Sarv is Iranian double-entry accounting. `crates/sarv-quote` is the engine; `crates/sarv-desktop` is the native Rune UI. The UI calls `sarv_quote::desktop::dispatch` only — it does not invent journal lines, tax, or document ids. Preview before post; pass `expected_generation`; reuse dialog `request_key`s. Money: sales drafts send the typed string with `display_unit`; purchases/payments/journals/banking parse rials. Do not bump the Rune pin.

## Backlog

1. Widen account pickers to 420; party/product stay 280; keep `{key}-trigger` as the menu anchor.
2. POS: focus the search field when the page opens (barcode → catalog). No second payment path.
3. Sheets: after a successful save, confirm `dirty` clears; add a test if it does not.
4. Out of scope for this loop: payroll, bank feeds, card processors, team server.

## In Progress

- Account picker width 420.

## Done

- `dbd642db` — docs: add AGENT_PLAN.md for the quality loop
- `2dd9ffdd` — fix(desktop): unmount finished setup guide in Auto mode
- _(pending)_ — solid dialog panels (drop glass/blur; keep portal append order)

## Decisions

- One verified desktop-bin commit per backlog item when clean; otherwise one coherent commit.
- `cargo test -p sarv-desktop --bin sarv-desktop` must stay green between commits.
- GPU snaps (`SARV_DEMO=1`, `--scene invoice` / `--scene overview`) for dialog opacity and guide visibility; needs `required_permissions: ["all"]`.
- Do not bump the Rune pin.
- Auto + five steps done unmounts `setup-guide-card`; `GuideMode::Open` still mounts it; Closed keeps the collapsed header so the user can reopen.
- Dialog sheets use solid `layer_01`; overview frost keeps `alpha(0.72)` glass.

## Open Risks

- Demo fixture is intentionally 4 of 5 setup steps; guide-hide tests force all five complete in-memory.
- Widening account selects to 420 may need a looser menu-alignment assertion if the field center drifts.
- Large uncommitted Rune migration tree exists beside this loop; commits stay scoped to quality-loop files where possible.
