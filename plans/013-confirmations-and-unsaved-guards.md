# Plan 013: Irreversible actions ask first, and a stray click cannot discard a form

> **Executor instructions**: Follow this plan step by step. Run every verification command. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-desktop/src/ui/glass.rs crates/sarv-desktop/src/ui/views crates/sarv-desktop/src/app.rs`

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: LOW–MED
- **Depends on**: 004 (uses its `open_*` functions and `mint_key`)
- **Category**: bug / UX
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

Project rule (AGENTS.md): "Preview an action before posting it." Several buttons post to the ledger on a single click with no review:

- **Return** (Sales and Purchases) returns *all* remaining quantity of the selected document immediately.
- **Reverse** posts a reversing journal immediately.
- **Lock/Reopen fiscal period** toggles immediately.
- **Cheque actions** (deposit, clear, bounce, return, void) post immediately, with no preview and no `expected_generation`; "clear" silently picks the first bank account; each uses one fixed request key per action per cheque.

In the other direction, long forms are fragile: every dialog closes on an outside click or Esc, and reopening the sale dialog wipes its lines. A 10-line invoice is one stray click from gone. The Sheets page tracks a `dirty` flag but never shows it or warns before navigating away.

## Current state

- `crates/sarv-desktop/src/ui/glass.rs:282-388` — `Ui::dialog` / `Ui::dialog_described(layer, portals, key, title, description, open, width, body, footer, on_close)`. The panel ends with `.modal(true, on_close)` (line 363): outside click and Esc both call `on_close`.
- Return: `ui/views/sales.rs:821-841` (`return_selected`) → `ops::post_operation(st, &id, "sale_return", &reference, "Restock customer return and issue credit", &key)`; same in `ui/views/purchases.rs` (~514-527) with `"purchase_return"`.
- `ui/ops.rs:31-75` — `post_operation` does `operation_options` → builds lines with **all** remaining quantities → `{operation}_preview` → copies `_generation` → `{operation}_post`. The preview result is discarded apart from the generation.
- Reverse: `ui/views/accounting.rs:323-350` (`reverse_selected`) — previews via `journal_reverse_preview`, then posts at once.
- Period: `accounting.rs:352-377` (`toggle_period`).
- Cheques: `ui/views/payments.rs:453-463`:
  ```rust
  let req = format!("{action}-{cheque_id}");
  let mut payload = json!({ "cheque_id": cheque_id, "request_key": req, "source_id": req });
  if needs_fund { let Some(fund) = first_fund_account(st) else { … }; payload["fund_account_id"] = json!(fund); }
  let _ = st.run(action, payload, Some("Posted"));
  ```
- Sheets: `ui/views/sheets.rs` has `dirty` in its state (`grep -n dirty crates/sarv-desktop/src/ui/views/sheets.rs`); `State::nav` (`app.rs:265-279`) and `select_company` (`app.rs:307-313`, sets `sheets.loaded = false`) ignore it. `fa_messages.rs` already has an unused `" · unsaved changes"` key.
- Rules: `State::run` toasts errors (do not toast again). Money shown with `st.fmt.money(..)`; quantities from the engine are thousandths → display with `st.fmt.qty(..)`; quantities sent back are display units (`ops::qty_from_thousandths`).
- i18n: English key in code, Persian in `src/fa_messages.rs`.

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |

## Scope

**In scope**: `crates/sarv-desktop/src/app.rs`, `src/ui/glass.rs`, new `src/ui/confirm.rs` (+ `pub mod confirm;` in `ui/mod.rs`, mount call in `root_view`), `src/ui/ops.rs`, `src/ui/views/{sales,purchases,accounting,payments,sheets,contacts,inventory,banking}.rs`, `src/fa_messages.rs`, `src/main.rs` (tests).

**Out of scope**: editing return quantities per line (plan 020 adds partial operations — this plan shows the lines read-only); engine changes.

## Git workflow

- Branch `advisor/013-confirmations-and-unsaved-guards`; commits per feature: `feat(desktop): review before returns and reversals`, `feat(desktop): cheque actions confirm and choose the bank`, `fix(desktop): forms survive an outside click`.

## Steps

### Step 1: A reusable confirmation dialog

Add to `State` a field `pub confirm: Option<Confirm>` and in `ui/confirm.rs`:

```rust
pub struct Confirm {
    pub title: String,          // already translated
    pub message: String,        // already translated; plain sentence saying what will happen
    pub lines: Vec<(String, String)>, // label → value rows (e.g. account, debit, credit; item, quantity)
    pub confirm_label: String,  // a verb: "Post return", "Reverse entry", "Lock period"
    pub danger: bool,
    pub action: std::rc::Rc<dyn Fn(&mut State)>,
}
pub fn mount(s: &State, ui: &Ui, layer: &mut Vec<El>)
```

`mount` renders `ui.dialog_described` (width `480.0`), the lines via `ui.panel_rows`, footer `Cancel` (Ghost) and the confirm button (`Btn::Danger` when `danger`, else `Btn::Primary`). The confirm button handler takes the `Confirm` out of state (`st.confirm.take()`) **before** running `action`, so a double click runs it once. Call `confirm::mount` from `ui/mod.rs` `root_view` after the page view so it paints above page dialogs.

If `State` must be `Clone`/`Send` and `Rc<dyn Fn>` is not allowed, store an `enum ConfirmAction { SaleReturn{..}, PurchaseReturn{..}, ReverseJournal{..}, PeriodLock{..}, Cheque{..} }` and dispatch on it instead.

### Step 2: Split `post_operation` into preview and post

In `ui/ops.rs` add:

```rust
pub struct OperationReview { pub payload: Value, pub lines: Vec<(String, String)>, pub total: String }
/// Everything `post_operation` did before posting. Toasts its own errors.
pub fn review_operation(st: &mut State, record_id: &str, operation: &str, reference: &str, reason: &str, request_key: &str) -> Result<OperationReview, String>
pub fn post_reviewed(st: &mut State, operation: &str, payload: Value) -> Result<Value, String>
```

`review_operation` returns the payload with `expected_generation` set, and display lines built from `operation_options` (item name + `st.fmt.qty(remaining)`); read the `{operation}_preview` response once with a debug print to find its total field and include it. Keep `post_operation` as `review` + `post` for the sale and purchase create flows that chain it.

### Step 3: Wire confirmations

- **Sales Return / Purchases Return**: mint the key with `st.mint_key("return")`; call `review_operation`; on `Ok` set `st.confirm = Some(Confirm { title: t("Return goods"), message: t("All remaining quantities on this document will be returned."), lines, confirm_label: t("Post return"), danger: true, action: post_reviewed(...) })`.
- **Reverse**: after the existing `journal_reverse_preview`, show a confirm with the journal reference, date and amount; confirm posts `journal_reverse`.
- **Lock / Reopen period**: confirm with the period name; message differs for lock ("No entries can be posted in this fiscal year until it is reopened.") and reopen; `danger: true` for reopen.
- **Cheques**: each action button opens a confirm showing cheque number, amount, due date (`st.fmt.date`) and the action. For `cheque_clear`, when more than one fund account exists (`s.books.banking["accounts"]`), the confirm must let the user choose: render one confirm button per account (label = account name) instead of a single button; with exactly one account keep a single button. Mint the key per click with `st.mint_key(action)` instead of the fixed `{action}-{cheque_id}`. Add `"expected_generation": crate::data::s(&st.books.workspace["_generation"])` to the payload — first confirm the workspace response carries `_generation` (`grep -n "_generation" crates/sarv-quote/src/desktop_bridge.rs | head`); if it does not, omit it and note that in your report.

Add every new string to `fa_messages.rs`.

**Verify**: desktop tests pass; update tests that clicked these buttons and expected an immediate post (they now need a second click on the confirm button, key `btn-{confirm label}`).

### Step 4: Forms do not close on an outside click

Add a `dismiss_on_outside: bool` concept without changing every call site: introduce `Ui::form_dialog(…same args as dialog_described…)` that wraps the body the same way but passes a no-op as the `.modal(true, …)` callback and wires the real `on_close` only to the ✕ button and the Cancel button. Check how rune's `.modal` distinguishes Esc from an outside click (`grep -n "pub fn modal" -B6 -A12 ~/.cargo/git/checkouts/rune-*/b556a38/crates/rune/src/**/*.rs`); if both arrive through the same callback, Esc also becomes a no-op in form dialogs — then handle Esc in `main.rs` `handle_key` by calling a `State::request_close_top_dialog()` (next step).

Use `form_dialog` for: sale, purchase, payment, journal, banking, contact (new and profile), product (new and profile), transfer, business profile. Keep `dialog` for palette, ledger, restore list and the confirm dialog.

### Step 5: Dirty check on close

Give each form dialog a cheap dirty test: `fn is_dirty(st: &State) -> bool` = any field under its prefix is non-empty or (sale/purchase) any line has an item. `Forms` needs `pub fn any_with_prefix(&self, prefix: &str) -> bool` (`app.rs`). Cancel/✕/Esc call `close_or_confirm`: if not dirty close; if dirty set `st.confirm` with title "Discard this form?", confirm label "Discard", `danger: true`, action = close.

Product/contact **profile** dialogs are pre-filled, so "non-empty" is always true: for those compare against a snapshot taken on open (store `opened_snapshot: String` = concatenation of the prefix's values).

### Step 6: Sheets dirty guard

Show `ui.t(" · unsaved changes")` next to the Sheets title when `dirty`. In `State::nav` and `select_company`: if `self.page == Page::Sheets && self.sheets.dirty`, set `self.confirm` ("Leave without saving the sheet?", "Leave", danger) whose action performs the navigation; otherwise navigate as today.

### Step 7: Tests (`main.rs`)

1. `test_return_requires_confirmation`: select an invoice, click `btn-Return`; assert no new sale record yet and `rt.state.confirm.is_some()`; click the confirm button; assert the return posted.
2. `test_reverse_requires_confirmation` (pattern: `test_journal_post_uses_preview_generation`).
3. `test_cheque_clear_offers_fund_accounts` — register a received cheque (pattern: `test_cheque_strip_labels_are_translated_keys`), deposit, clear; assert the confirm is shown and posting uses the chosen account.
4. `test_dirty_sale_survives_cancel`: open sale, add a line item, click Cancel → confirm shown, sale dialog still open; click Discard → closed.
5. `test_sheet_dirty_blocks_navigation`.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass, 5 new.

## Done criteria

- [ ] `grep -n 'format!("{action}-{cheque_id}")' crates/sarv-desktop/src/ui/views/payments.rs` returns nothing
- [ ] `grep -rn "st.confirm = Some" crates/sarv-desktop/src/ui/views | wc -l` ≥ 6
- [ ] Desktop suite passes with the 5 new tests
- [ ] `plans/README.md` updated

## STOP conditions

- Rune's `.modal` cannot be given a no-op dismiss without also breaking focus trapping (dialog loses keyboard focus containment). Report; do not fork the kit.
- A cheque action previously succeeded twice on the same cheque only because of the fixed key replay (deposit → return → deposit): confirm the second deposit now posts with a fresh key; if the engine rejects it for another reason, report.

## Maintenance notes

- Plan 020 replaces the read-only lines in the return confirm with editable quantities.
- Every future one-click ledger action must go through `Confirm`; add that sentence to `docs/DESIGN.md` section "Dialog contract" (plan 011).
