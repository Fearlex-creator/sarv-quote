# Plan 018: Users can add and archive accounts in the chart of accounts

> **Executor instructions**: Follow this plan step by step. Run every verification command. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-quote/src/desktop_bridge.rs crates/sarv-quote/src/accounting.rs crates/sarv-desktop/src/ui/views/accounting.rs crates/sarv-desktop/src/ui/pickers.rs`

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: LOW
- **Depends on**: none (004 recommended for the dialog pattern)
- **Category**: direction (feature)
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

The chart of accounts is fixed at whatever setup created: a small set of role accounts with a single operating-expense account. A business cannot separate rent from salaries from utilities, so its profit-and-loss statement is one line of expenses. Any accountant evaluating the product will stop here. The engine already implements adding and archiving accounts, with uniqueness and parent checks and audit records that replay; no action exposes them and the Chart tab is read-only.

## Current state

- `crates/sarv-quote/src/accounting.rs:132-143`:
  ```rust
  #[serde(deny_unknown_fields)]
  pub struct NewAccount {
      pub company_id: String,
      pub parent_id: Option<String>,
      pub code: String,
      pub name: String,
      pub classification: AccountClass,
      pub normal_balance: BalanceSide,
      pub postable: bool,
      pub control: bool,
  }
  ```
  `add_account(&mut self, input: NewAccount, actor: &str) -> Result<String>` (`:949-992`) rejects duplicate codes per company ("کد حساب در این کسب‌وکار تکراری است") and an invalid/archived parent. `archive_account(&mut self, account_id: &str, actor: &str, reason: &str) -> Result<()>` (`:993…`) refuses accounts mapped to a setup role — read the whole function to list every refusal (e.g. non-zero balance, has children) so the UI can explain them.
- `AccountClass` and `BalanceSide` enums: `grep -n "pub enum AccountClass" -A12 crates/sarv-quote/src/accounting.rs` and same for `BalanceSide`. Note their serde names (the JSON strings).
- Bridge: `crates/sarv-quote/src/desktop_bridge.rs` `dispatch_in` match. Model new arms on `"period_lock"` (~1676-1706) for `company(..)`, `actor(&p)`, `string(&p, key)?`, `store.change(|s| …)`. The `workspace` response already returns `"accounts"` for the company (`:1008`).
- Desktop Chart tab: `crates/sarv-desktop/src/ui/views/accounting.rs:53-90` (a `Table` over `s.books.list("accounts")`; classification is translated at ~line 81). Account pickers used by journal/banking dialogs: `crates/sarv-desktop/src/ui/pickers.rs` (`entity_combo`, `ACCOUNT_PICKER_WIDTH = 420`). Check whether pickers already hide `archived` and non-`postable` accounts.
- Dialog conventions: one `open_*` function resets fields and state; form fields via `ui.labeled("Label", ui.field(s, "key", ""))`; two columns via `ui.form_grid(left, right)`; `ui.dialog_described(layer, &mut portals, key, title, description, open, width, body, footer, on_close)`; segmented choice via `ui.tabs(id, &[(label, selected)], |st, idx| …)`. Save calls `st.run(action, payload, Some("Saved"))` which refreshes and toasts errors itself.
- i18n: English keys; Persian in `crates/sarv-desktop/src/fa_messages.rs`.
- Agent allow-list `crates/sarv-quote/src/agent.rs:43-69` — do not add these actions.

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Engine tests | `cargo test -p sarv-quote` | all pass |
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |

## Scope

**In scope**: `crates/sarv-quote/src/desktop_bridge.rs` (two arms), new `crates/sarv-quote/tests/chart_accounts.rs`, `crates/sarv-desktop/src/ui/views/accounting.rs`, `src/ui/pickers.rs` (filtering only), `src/fa_messages.rs`, `src/main.rs` (tests).

**Out of scope**: renaming/re-coding existing accounts (no engine function exists); tafsili / dimension levels (a separate structural design); changing role mappings.

## Git workflow

- Branch `advisor/018-chart-of-accounts`; commits `feat(engine): account_add and account_archive actions`, `feat(desktop): add and archive accounts`.

## Steps

### Step 1: Engine actions

`"account_add"`: build `NewAccount` from the payload (`company_id` from `company(&store.state,&p)`, `parent_id` optional, `code`, `name`, `classification`, `normal_balance`, `postable` default `true`, `control` always `false` — control accounts are reserved for the subledgers; reject `control: true` from the bridge). Deserialize the two enums with `serde_json::from_value(p["classification"].clone())` so the JSON names match the engine's. If `parent_id` is given, require the parent's classification to equal the child's. Return `{"account_id": id}`.

`"account_archive"`: `account_id`, `reason` (required text) → `archive_account`. Return `true`.

Both require `expected_generation` (same comparison as `desktop_bridge.rs:1143-1151`).

Tests in `tests/chart_accounts.rs` (helpers from `tests/desktop_bridge.rs:1-19`): add an expense account "Rent" under the existing expense group; it appears in `workspace.accounts`; post a journal to it and see it as its own row in `report`'s P&L; duplicate code rejected; mismatched parent classification rejected; `control:true` rejected; archive an unused account → no longer offered (`archived: true`); archive a role-mapped account → error; archive an account with a balance → whatever the engine does (assert the observed behaviour and state it in the report); store reopens and validates.

**Verify**: `cargo test -p sarv-quote --test chart_accounts` → all pass; `cargo test -p sarv-quote` → all pass.

### Step 2: Desktop — New account dialog

On the Accounting page, when the Chart tab is active, add `Btn::Glass` "New account" → `open_account(st)` (clears `acct-` fields, resets `AccountingState` fields `acct_open`, `acct_class_idx`, `acct_parent`). Dialog (width 640):

- Type: segmented control over the five classifications (Asset, Liability, Equity, Income, Expense — use the enum's actual variants), labels translated.
- Parent: account picker limited to non-archived accounts of the chosen classification (optional).
- Code (`acct-code`) and Name (`acct-name`). Suggest a code: the highest existing numeric code among the parent's children + 1, shown as the placeholder only.
- Normal balance is derived, not asked: debit for Asset/Expense, credit otherwise.
- Save → `account_add` with `expected_generation` from the workspace generation; success toast "Account added"; close.

### Step 3: Desktop — Archive

Add `Btn::Glass` "Archive account", enabled when a chart row is selected. It asks for confirmation (use plan 013's `Confirm` if landed; otherwise a small dialog with Cancel / Archive) and a reason field, then calls `account_archive`. The engine's refusal message is shown by `State::run`.

Show archived accounts in the Chart table with a muted "Archived" badge, and add a "Show archived" toggle defaulting off. In `pickers.rs`, make sure account pickers exclude `archived == true` and `postable == false` accounts.

Persian strings: `("New account", "حساب جدید")`, `("Archive account", "بایگانی حساب")`, `("Account added", "حساب افزوده شد")`, `("Parent account", "حساب والد")`, `("Account code", "کد حساب")`, `("Archived", "بایگانی‌شده")`, `("Show archived", "نمایش بایگانی‌شده‌ها")`, plus any classification labels missing from `fa_messages.rs`.

### Step 4: Desktop tests (`main.rs`; pattern `test_accounting_tabs_and_tables_headlessly` and `test_journal_post_uses_preview_generation`)

1. `test_new_expense_account_appears_in_chart_and_pickers`.
2. `test_archived_account_leaves_the_pickers`.
3. `test_duplicate_account_code_shows_one_error_toast`.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass.

## Done criteria

- [ ] `grep -n '"account_add"\|"account_archive"' crates/sarv-quote/src/desktop_bridge.rs` → both present; neither in `agent.rs`
- [ ] `cargo test -p sarv-quote --test chart_accounts` passes
- [ ] Desktop suite passes with 3 new tests
- [ ] `plans/README.md` updated

## STOP conditions

- Reports (`trial_balance`, P&L in `fn report`) do not pick up a user-created account automatically (they iterate roles rather than accounts). Report which report misses it.
- `archive_account` allows archiving an account with a non-zero balance and that makes the trial balance stop balancing — stop; that is an engine bug to fix first.

## Maintenance notes

- `parent_id` already gives a two-level hierarchy; the Iranian group → kol → moein → tafsili structure (the largest gap in `docs/SPEC-COVERAGE.md`) should build on it in its own design pass.
- Importing a standard Iranian chart as a template is a natural follow-up once accounts can be added.
