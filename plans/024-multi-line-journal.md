# Plan 024: Manual journals can have any number of lines

> **Executor instructions**: Follow this plan step by step. Run every verification command. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-desktop/src/ui/views/accounting.rs crates/sarv-desktop/src/ui/pickers.rs`

## Status

- **Priority**: P2
- **Effort**: M
- **Risk**: LOW (the engine validates balance)
- **Depends on**: 004 (journal `open_journal`), 005 (money field), 014 (date field)
- **Category**: direction (feature)
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

The journal dialog offers exactly one debit account, one credit account and one amount. Real manual entries are rarely that simple: a salary accrual, a split expense, an adjustment with several counter-accounts. The engine's `journal_preview` / `journal_post` already accept an arbitrary `lines` array and verify that it balances; only the form is limiting.

## Current state

- `crates/sarv-desktop/src/ui/views/accounting.rs`:
  - State (`AccountingState`, top of file): `modal_open`, `debit_account`, `credit_account`, `post_key`, `posting`, plus combobox states for the two pickers.
  - Dialog: `mount_journal_dialog` or similar (find with `grep -n "journal-amount" crates/sarv-desktop/src/ui/views/accounting.rs`); width 900 because two 420-wide account pickers sit side by side (recorded decision: `ACCOUNT_PICKER_WIDTH = 420`, journal/banking dialogs 900).
  - Save handler (`:246-303`) builds:
    ```rust
    let mut payload = json!({
        "reason": reason, "source_id": key, "request_key": key,
        "lines": [
            { "account_id": st.accounting.debit_account, "debit": amt.to_string(), "credit": "0", "description": "" },
            { "account_id": st.accounting.credit_account, "debit": "0", "credit": amt.to_string(), "description": "" }
        ]
    });
    match st.query("journal_preview", payload.clone()) {
        Ok(pr) => payload["expected_generation"] = pr["_generation"].clone(),
        Err(e) => { st.toast_err(e); st.accounting.posting = false; return; }
    }
    match st.run("journal_post", payload, Some("Posted")) { … }
    ```
  Amounts are **rial** strings (the screen converts toman→rial before sending; AGENTS.md).
- Account picker: `crates/sarv-desktop/src/ui/pickers.rs` — `entity_combo(...)` with an explicit width; the sale dialog shows how a **list** of comboboxes is kept per line: `SalesState { lines: Vec<SaleLine>, line_combos: Vec<ComboboxState>, … }` (`crates/sarv-desktop/src/ui/views/sales.rs:56-62`) and how its line editor renders rows and an "Add line" button (`grep -n "line_combos" crates/sarv-desktop/src/ui/views/sales.rs`). Use it as the structural pattern.
- Picker portals: comboboxes inside a dialog must be built into the `portals` vec passed to `ui.dialog_described` so menus paint above the sheet (doc comment at `ui/glass.rs:296-299`).
- Tests: `test_journal_post_uses_preview_generation` (`crates/sarv-desktop/src/main.rs:496`) posts a two-line journal by setting `rt.state.accounting.debit_account` / `credit_account` and `forms.set("journal-amount", …)` directly. `test_purchase_and_journal_dialogs_headlessly` (`:468`) checks element keys in the dialog.
- Money helpers: `st.parse_money_input(text)` (rials; `Result` after plan 005), `st.fmt.money(rials_str)`, `ui.money_field` (plan 005).

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |

## Scope

**In scope**: `crates/sarv-desktop/src/ui/views/accounting.rs`, `src/ui/views/overview.rs` (only if it touches journal state fields that are renamed), `src/fa_messages.rs`, `src/main.rs` (tests).

**Out of scope**: engine; recurring journals; templates; importing journals from CSV.

## Git workflow

- Branch `advisor/024-multi-line-journal`; commit `feat(desktop): multi-line manual journals`.

## Steps

### Step 1: Line model

Replace `debit_account` / `credit_account` with:

```rust
pub struct JournalLine { pub account_id: String, pub combo: ComboboxState }
pub lines: Vec<JournalLine>,   // amounts and descriptions live in Forms: journal-debit-{i}, journal-credit-{i}, journal-desc-{i}
```

`open_journal` starts with two blank lines and clears the `journal-` form prefix. Keep small accessor helpers so other code (Overview, tests) does not reach into fields.

### Step 2: Grid

Each row: account picker (width 420), description field (grow), debit money field, credit money field, and a remove button (ghost icon button; disabled while only two rows remain). Under the grid: "Add line" (`btn-Add line` — check whether the sale dialog already uses that exact key; inside different dialogs duplicates are fine, but they must never be mounted at once) and a totals row: total debit, total credit, and the difference, coloured `tone_fg(Tone::Danger)` when non-zero and showing "Balanced" when zero. Totals are a display convenience computed from the parsed fields — the engine remains the authority.

Removing row `i` must also shift the form fields of later rows down (`journal-debit-{i+1}` → `journal-debit-{i}` …); write a helper `fn remove_line(st, i)` and unit-test it through a headless test.

### Step 3: Save

Validation before preview (each failure is one toast and resets `posting`):

- every line with any amount has an account → else "Select an account on every line."
- a line may have a debit **or** a credit, not both → "A line is either a debit or a credit."
- ignore fully blank lines; at least two non-blank lines → "Add at least two lines."
- amounts parse (plan 005 rules) and are positive.
- do **not** block on imbalance client-side with a different message than the engine's; but if the totals differ, show "Debits and credits must be equal." and stop — this avoids a round trip; the engine check still runs on preview.

Build `lines` as `{account_id, debit, credit, description}` with rial strings; then the existing preview → `expected_generation` → post sequence, unchanged.

### Step 4: Keep the simple case simple

With exactly two lines and the user typing an amount in line 1's debit, pre-fill line 2's credit with the same amount if it is empty (only once, when line 2's credit field has never been edited). This keeps the common two-line entry as fast as today.

### Step 5: Strings and tests

Persian: `("Add line", "افزودن ردیف")` (check existing), `("Debit", "بدهکار")`, `("Credit", "بستانکار")` (check existing), `("Balanced", …)` exists, `("Select an account on every line.", "برای هر ردیف حساب انتخاب کنید.")`, `("A line is either a debit or a credit.", "هر ردیف یا بدهکار است یا بستانکار.")`, `("Add at least two lines.", "دست‌کم دو ردیف وارد کنید.")`, `("Debits and credits must be equal.", "جمع بدهکار و بستانکار باید برابر باشد.")`.

Update the two existing journal tests to the new state shape. New tests:

1. `test_three_line_journal_posts_balanced` — one debit 300, two credits 100 + 200; journal count +1; the stored journal has three lines.
2. `test_unbalanced_journal_is_blocked_with_one_toast`.
3. `test_removing_a_journal_line_shifts_the_fields`.
4. `test_two_line_journal_mirrors_the_amount`.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass.

## Done criteria

- [ ] `grep -n "debit_account\|credit_account" crates/sarv-desktop/src` returns nothing
- [ ] Desktop suite passes with 4 new tests and the 2 updated ones
- [ ] `plans/README.md` updated

## STOP conditions

- The kit cannot host N comboboxes in a scrolling dialog body without the menus being clipped (portals anchor incorrectly after scroll). Report with a snapshot (`--scene` + `--snap`); a fallback is a fixed maximum of 8 lines without scrolling.
- Banking's dialog shares journal state fields — check `crates/sarv-desktop/src/ui/views/banking.rs` before renaming.

## Maintenance notes

- Excel-style paste into the grid and keyboard row navigation are natural follow-ups (the spec's "EntryGrid").
- When tafsili/dimension levels are designed, each line gains more pickers; keep `JournalLine` the single place lines are described.
