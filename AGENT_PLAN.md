# AGENT_PLAN

Living memory for the quality loop. Update after every finished backlog item.

## Product Understanding

Sarv is Iranian double-entry accounting. `crates/sarv-quote` is the engine; `crates/sarv-desktop` is the native Rune UI. The UI calls `sarv_quote::desktop::dispatch` only — it does not invent journal lines, tax, or document ids. Preview before post; pass `expected_generation`; reuse dialog `request_key`s. Money: sales drafts send the typed string with `display_unit`; purchases/payments/journals/banking parse rials. Do not bump the Rune pin.

## Backlog

1. Sales invoice numbering: the desktop builds `ORD-` / prefix references in `sales.rs`. Show the engine's number and stop minting one in the UI. Impact 5, effort M, feature: invoice dialog.
2. Purchases / Sales return flows: confirm `remaining_quantity` is divided by 1000 and `expected_generation` is sent on every branch. Impact 4, effort M, feature: returns.
3. Invoice settlement: cash, card, transfer, cheque, on account, and split, with the engine preview in plain Persian before post. Impact 5, effort L — split before starting. Feature: invoice dialog.
4. Invoice as a full page with the sidebar collapsed, party search focused, and one primary action «پیش‌نمایش و ثبت». Impact 4, effort L. Feature: invoice dialog.
5. GPU snap pass for Payments cheque strip, Accounting empty card, Contacts/Inventory suite, and sidebar without brand. Impact 2, effort M. Needs a GPU. Feature: visual QA.
6. Out of scope, do not build: payroll, live bank feeds, card processors, a team server.

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
- `85bb17af` — docs: record quality-loop commits in AGENT_PLAN
- `4deeef1e` — feat(desktop): tidy sidebar and deepen contacts/inventory (no brand header; company footer; Inventory before Contacts; POS via Settings `show_pos`; full contact/product suites with brand/category search)
- Cleared from backlog: inventory on-page search; unfinished `show_pos` nav race.
- `c257264a` — fix(desktop): polish setup, backups, and empty sheets (one error toast, پشتیبان not پشتیبانی, first-run copy, blank sheet fills from the books).
- Suite: `cargo test -p sarv-desktop --bin sarv-desktop` — 35 passed.

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
- POS stays in `Page::GROUPS` but `Page::nav_pages(show_pos)` filters sidebar, rail, and palette. Settings Features switch persists `settings.show_pos`. Headless tests that open POS must `rt.update(|s,_| { s.show_pos = true; s.nav(Page::Pos); })` — mutating `rt.state` alone does not dirty the view.
- Contacts/Inventory use existing `PartyProfile` / `ProductProfile` fields (brand/category as the product “variables”); no new engine schema.
- Pages reviewed (2026-11-12): Overview, Sales, POS, Purchases, Payments, Banking, Contacts, Inventory, Accounting, Reports, Sheets, Settings, Setup, shell/palette/dialogs/toasts.
- A blank company name stays on the setup step and toasts «نام کسب‌وکار را بنویسید». The opening button says «شروع از صفر» only when cash and bank are both zero; any typed amount switches it to «مانده‌های افتتاحیه را ثبت کنید». The post path already branched that way.
- Backup create, business-profile save, and backup restore do not call `toast_err` after `run`. Success uses «پشتیبان ساخته شد: {name}» once. The old Persian for an empty backup list said پشتیبانی (support); it now says پشتیبان.
- A new sheet with no cells shows «این برگه خالی است». «آوردن از دفاتر» sits in the tab row. A button inside the grid column did not receive the press; the scrolling grid covers that band.
- First-run Persian: «سه گام تا شروع», «نام مغازه یا شرکت را بنویسید», «معمولاً از اول فروردین تا آخر اسفند است», «نام مغازه، سال مالی، و ماندهٔ اول کار. بعدش دفاتر باز می‌شود.»

## Open Risks

- Demo fixture is intentionally 4 of 5 setup steps; guide-hide tests force all five complete in-memory.
- `cargo clippy --all-targets -- -D warnings` is not clean on rustc 1.99. Pre-existing `collapsible_if` and `needless_borrow` fire in `sarv-quote` and `sarv-desktop`. This loop did not take that cleanup.
- GPU scene snaps still need a GPU. `png_scenes` was not run here.
- The sales invoice is still a modal, and the desktop still mints order and invoice references. That is the next invoice-dialog work, not a second sales system.
- Concurrent agents previously raced `shell.rs` / `app.rs` / Settings `show_pos` — keep POS reachable via `show_pos` for the suite.
