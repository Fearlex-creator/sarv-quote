# Plan 022: Users can add a second business and switch between businesses from the sidebar

> **Executor instructions**: Follow this plan step by step. Run every verification command. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-desktop/src/ui/views/setup.rs crates/sarv-desktop/src/ui/views/settings.rs crates/sarv-desktop/src/ui/shell.rs crates/sarv-desktop/src/ui/mod.rs crates/sarv-desktop/src/app.rs`

## Status

- **Priority**: P2
- **Effort**: S–M
- **Risk**: LOW
- **Depends on**: 004 (form reset pattern), 013 (dirty guards) recommended
- **Category**: direction (feature)
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

`README.md` says "Create additional businesses in **Settings → Add company**." The engine supports several companies in one set of books, and the desktop even renders a company switcher — but only when two or more companies already exist, and nothing in the Rune app can create the second one. The Persian strings "Add company" and "Create another company" exist in the dictionary with no caller. An owner with a shop and a side business, or an accountant keeping books for two clients, cannot use the app as documented.

## Current state

- First-run flow: `crates/sarv-desktop/src/ui/mod.rs:22-24`:
  ```rust
  let content = if !s.books.has_company() || !s.books.initialized() {
      views::setup::view(s, cx, &ui, &mut layer)
  } else { match s.page { … } };
  ```
  `ready` (same condition) decides whether the shell/sidebar is shown.
- `crates/sarv-desktop/src/ui/views/setup.rs` (383 lines): `SetupState` (line ~45); steps: welcome (language/theme) → company and fiscal year (posts `setup_company`, ~line 93) → opening balances (`initialize_zero` or `opening_csv_preview`/`opening_csv_post`, ~208-260). Read the whole file before changing it.
- `Books::initialized()` (`crates/sarv-desktop/src/data.rs:124-126`) reads `workspace["initialized"]`, which is about the **selected** company; `has_company()` checks the companies list; `companies()` returns `(id, name)` pairs; `State::select_company(id)` (`app.rs:307-313`) switches and refreshes, resetting POS and Sheets.
- Switcher today: `crates/sarv-desktop/src/ui/views/settings.rs:171-186` — a row of buttons, shown only `if companies.len() > 1`.
- Sidebar footer company row: `crates/sarv-desktop/src/ui/shell.rs:77-90` — shows the company name and fiscal year, `on_select` navigates to Settings; key `org-selector`.
- Dictionary: `fa_messages.rs` has `"Add company"` (line ~33) and `"Create another company"` (~236).
- Per-company data: business profile, settings fields that are per-company vs global — check `"business_profile"` and `"settings"` arms in `crates/sarv-quote/src/desktop_bridge.rs` (README: "these details are saved separately for each business").
- Test pattern for first run: `test_first_run_is_a_welcome_then_the_books` in `crates/sarv-desktop/src/main.rs:917`.

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |

## Scope

**In scope**: `crates/sarv-desktop/src/ui/views/setup.rs`, `src/ui/views/settings.rs`, `src/ui/shell.rs`, `src/ui/mod.rs`, `src/app.rs`, `src/fa_messages.rs`, `src/main.rs` (tests).

**Out of scope**: engine changes; deleting or archiving a company; consolidated reports across companies.

## Git workflow

- Branch `advisor/022-add-company-and-switcher`; commits `feat(desktop): add another company`, `feat(desktop): switch company from the sidebar`.

## Steps

### Step 1: Setup can run for an additional company

Add `adding_company: bool` to `SetupState`. In `root_view`, show the setup view when `!has_company() || !initialized() || s.setup.adding_company`, and treat the window as not-ready (no sidebar) while adding — but render a "Cancel" button (`Btn::Ghost`) in the setup header in that mode that clears the flag and returns to the previous company.

In "adding" mode: skip the welcome (language/theme) step; start at the company step with empty fields (`st.forms.clear_prefix` for the setup form prefixes — list them with `grep -n 'forms.get("' crates/sarv-desktop/src/ui/views/setup.rs`); after `setup_company` succeeds, select the new company (`st.select_company(&new_id)` — get the id from the action's response; print it once to see the shape) and continue to the opening-balances step, which then operates on the new company. When the new company becomes initialized, clear `adding_company`.

If the user cancels after the company was created but before opening balances, the new company exists un-initialized; `root_view` would then force setup for it whenever it is selected. That is acceptable (README: "Unfinished businesses are guided through explicit opening-balance confirmation before creation resumes") — make sure Cancel switches back to a finished company so the user is not trapped.

### Step 2: Entry points

- Settings: a `Btn::Glass` "Add company" header action → `st.setup = SetupState { adding_company: true, ..Default::default() }` positioned at the company step.
- Always show the company list in Settings (even with one company), each row with its name and an "Open" action; mark the current one.

### Step 3: Sidebar switcher

Change the sidebar footer `org-selector` row: on select, open a small dialog (`DIALOG_SM`/480; key `company-switch`) listing companies (current one marked), plus an "Add company" row at the bottom. Choosing a company calls `st.select_company(id)` and closes. If a form dialog is open and dirty or Sheets is dirty (plan 013), go through the same guard used by `nav`.

After switching: page stays the same unless it is POS with `show_pos` false for that company (`apply_settings` already handles that — call it after `select_company`'s refresh; check whether `select_company` does).

### Step 4: Strings and tests

Add missing Persian keys (`("Switch company", "تغییر کسب‌وکار")`, `("Open", "باز کردن")` if absent).

Tests (`main.rs`):

1. `test_add_second_company_from_settings`: demo runtime → click `btn-Add company` → fill company name and fiscal year fields as `test_first_run_is_a_welcome_then_the_books` does → finish with zero opening → assert `rt.state.books.companies().len() == 2`, the new company is selected and initialized, the sidebar is back.
2. `test_switch_company_from_sidebar`: with two companies, click `org-selector`, choose the first; assert `books.company_id` changed and the Overview metrics reflect the other company (e.g. `sales_count` differs).
3. `test_cancel_adding_company_returns_to_books`.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass.

## Done criteria

- [ ] `grep -rn '"Add company"' crates/sarv-desktop/src/ui` has at least one caller outside `fa_messages.rs`
- [ ] `grep -n "companies.len() > 1" crates/sarv-desktop/src/ui/views/settings.rs` returns nothing
- [ ] Desktop suite passes with 3 new tests
- [ ] `plans/README.md` updated

## STOP conditions

- `setup_company` refuses a second company (engine error) — report the message; the README claim would then be wrong and the engine needs work first.
- Settings such as `show_pos`, language or theme turn out to be global rather than per-company and switching company produces surprising flips; report what is global and leave behaviour as is.

## Maintenance notes

- The demo fixture has one company; tests that assume `companies[0]` is the active one must select explicitly once a second exists.
- A future "archive company" needs an engine action first.
