# Plan 039: Redesign the Rune sidebar, top toolbar, and Settings for calm, minimal usability

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md` — unless a reviewer dispatched you and told you they
> maintain the index.
>
> **Drift check (run first)**: `git diff --stat 1e6866ac..HEAD -- crates/sarv-desktop/src/app.rs crates/sarv-desktop/src/ui/shell.rs crates/sarv-desktop/src/ui/views/settings.rs crates/sarv-desktop/src/fa_messages.rs crates/sarv-desktop/src/tests/shell.rs crates/sarv-desktop/src/tests/settings.rs docs/DESIGN.md`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts against the live code before proceeding; on a
> mismatch, treat it as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: LOW–MED
- **Depends on**: plans/036-rune-overlay-shortcuts-and-state-integrity.md
- **Category**: direction
- **Planned at**: commit `1e6866ac`, 2026-11-19

## Why this matters

The desktop shell currently feels cramped, noisy, and intimidating for everyday business owners:

1. **Bloated, cramped sidebar (21–22 rows)**: Commit `31c99a5e` added 6 redundant/broken custom shortcut rows (`nav-cheque-search`, `nav-crm`, `nav-account-card`, `nav-account-statement`, `nav-trial-balance`, `nav-security`), duplicating existing pages (`Contacts`, `Settings`, `Reports` tab 1 twice!) and causing two sidebar items to highlight at once. Combined with heavy accountant jargon (`Inventory cardex`, `Accounting voucher`, `Phonebook`) and 3 footer rows (Company, Language, Theme), the sidebar overflows and feels tiny (`PaneState::new(276.0).range(136.0, 640.0)` with `32.0` row height).
2. **Overloaded top toolbar**: Packs title + rail + search + refresh + lock + `"Saved on this device"` badge + past-year badge + fiscal year button onto every screen.
3. **Chaotic Settings screen**: Crams **6 header buttons** (`Add company`, `Your business`, `Restore backup`, `Backup`, `Law data`, `Save preferences` — even though toggles already auto-save!) on top of **6 stacked sections** in one scrolling General tab (including a duplicate Companies list).

Simplifying navigation to **3 logical, plain-language groups (~10 calm rows)**, widening/giving the sidebar comfortable breathing room, decluttering the toolbar, and splitting Settings into **4 clean, purposeful tabs** with zero header-button clutter makes Sarv feel calm, minimal, and effortless to use — matching ADR-0001 ("Minimal, easy to use, calm neutrals, restraint over decoration").

## Current state

- `crates/sarv-desktop/src/app.rs`:
  - `sidebar: PaneState::new(276.0).range(136.0, 640.0)` (`line 297`).
  - `Page::sidebar_label` (`lines 66-76`): uses `"Sales invoice"`, `"Purchase invoice"`, `"Accounting voucher"`, `"Receipts & payments"`, `"Phonebook"`, `"Inventory cardex"`.
  - `Page::GROUPS` (`lines 114-130`): 4 groups (`daily`, `people`, `books`, `workspace`).
- `crates/sarv-desktop/src/ui/shell.rs`:
  - `build_sidebar` (`lines 49-217`): adds 2 header rows + 11–12 pages + 6 custom duplicate rows (`lines 89-162`) + 3 footer rows (Company, Language, Theme) with `height(32.0)`.
  - `build_toolbar` (`lines 243-309`): always renders `"Saved on this device"` badge (`lines 270-274`).
- `crates/sarv-desktop/src/ui/views/settings.rs`:
  - `view` (`lines 72-397`): only 2 tabs (`General`, `Keyboard shortcuts`), 6 stacked sections in `General` (Appearance, Features, Security, Your business, About, Companies), and 6 buttons in the `ui.page(...)` header (`lines 359-394`).
- `crates/sarv-desktop/src/main.rs:62-66`: `F1` jumps to `Page::Settings` and sets `s.settings.tab_idx = 1` (update to the new Shortcuts tab index!).

Design rules from `docs/DESIGN.md`:
- §1: Minimal, easy to use, calm neutrals, hairlines, one accent (teal), clear hierarchy.
- §7: Deterministic element keys: `nav-{page}`, `btn-{English label key}`, `field-{form key}`, `{dialog}-dialog`.
- §9: No hard-coded `Color::` literals or numeric dialog widths/sheet radii in views.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Desktop unit/UI tests | `cargo test -p sarv-desktop --bin sarv-desktop` | exit 0, all pass |
| Workspace clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, no warnings |
| Format check | `cargo fmt --check` | exit 0 |

## Scope

**In scope**:
- `crates/sarv-desktop/src/app.rs`
- `crates/sarv-desktop/src/ui/shell.rs`
- `crates/sarv-desktop/src/ui/views/settings.rs`
- `crates/sarv-desktop/src/main.rs` (only the `F1` shortcuts tab index if changed)
- `crates/sarv-desktop/src/fa_messages.rs` (add any new clean Persian labels)
- `crates/sarv-desktop/src/tests/shell.rs`
- `crates/sarv-desktop/src/tests/settings.rs`
- `docs/DESIGN.md`

**Out of scope**:
- Changing accounting domain rules or engine dispatch actions in `crates/sarv-quote`.
- Removing any underlying `Page` screen or capability (every existing feature stays reachable cleanly via its page tabs or Command Palette `Cmd+K`).

## Git workflow

- Branch: `advisor/039-minimal-sidebar-navigation-and-settings-redesign`
- Conventional commits (e.g., `feat(desktop): simplify sidebar navigation and organize Settings into calm tabs`).
- Do NOT push or open a PR unless instructed.

## Steps

### Step 1: Simplify the sidebar to 3 calm, logical groups and give it breathing room

1. In `crates/sarv-desktop/src/app.rs`:
   - Give the sidebar a comfortable minimum & default width: `sidebar: PaneState::new(260.0).range(220.0, 360.0)`.
   - Simplify `Page::sidebar_label(self)` to clear, everyday labels (removing heavy jargon — `Sales`, `Purchases`, `Payments`, `Banking`, `Contacts`, `Inventory`, `Accounting`, `Reports`, `Sheets`, `Settings`, `Overview`, `POS`, i.e., reuse `self.label()` with crisp Persian translations already in `fa_messages.rs`).
   - Reorganize `Page::GROUPS` into **3 logical groups** (removing the separate 1-item `workspace` group — `Settings` moves cleanly into the footer beside the company switcher):
     1. `"daily"` (`"Daily work"`): `[Page::Overview, Page::Sales, Page::Pos, Page::Purchases, Page::Payments, Page::Banking]`
     2. `"people"` (`"Products & people"`): `[Page::Inventory, Page::Contacts]`
     3. `"books"` (`"Books"`): `[Page::Reports, Page::Accounting, Page::Sheets]` (Reports first because everyday owners check Profit & Loss / Balances far more often than manual journals)
2. In `crates/sarv-desktop/src/ui/shell.rs` (`build_sidebar`):
   - **Delete all 6 redundant custom sidebar rows** (`nav-cheque-search`, `nav-crm`, `nav-account-card`, `nav-account-statement`, `nav-trial-balance`, `nav-security`, `lines 89-162`). Each destination already has a clear tab inside its page (Cheques in Payments, Accounts in Accounting, Trial Balance in Reports, Security in Settings) and remains instant-searchable via `Cmd+K` Command Palette!
   - Increase `SidebarRow` height from `32.0` to `36.0` and `.row_gap(4.0)` so rows feel spacious and easy to click.
   - Keep the 2 high-value header rows (`Search anything…` + `New sales invoice` using `st.nav_then(Page::Sales, sales::open_sale)`).
   - Clean up the sidebar footer to **2 calm rows**:
     1. Company & fiscal year selector (`org-selector`, height `44.0`)
     2. `Settings` (`nav-settings`, height `36.0`, highlighted when `s.page == Page::Settings`)
   - Remove `lang-toggle` and `theme-toggle` from the sidebar footer (they remain one click away in Settings → General and via `Cmd+L` / `Cmd+D` shortcuts). **Note**: check `tests/shell.rs` for any tests clicking `lang-toggle` or `theme-toggle` and update those tests to use `Cmd+L`/`Cmd+D` or Settings!

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop shell` → exit 0.

### Step 2: Declutter the top toolbar

In `crates/sarv-desktop/src/ui/shell.rs` (`build_toolbar`, `lines 243-309`):
- Only show the status badge when there is an actual problem (`!s.books.error.is_empty()` → `Books could not be loaded`), instead of painting a permanent `"Saved on this device"` pill on every screen.
- Keep only essential, high-signal toolbar items: page title, collapsed icon rail (when sidebar is closed), Search (`Cmd+K`), Lock button (only when `s.app_lock_enabled()`), Past fiscal year warning badge (only when viewing a past year), and Fiscal year switcher button. Move manual `Refresh` out of the toolbar (or keep only if a test clicks `refresh-books` — grep `tests/` for `refresh-books` first!).

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → exit 0.

### Step 3: Reorganize Settings into 4 calm tabs and remove the 6-button header bar

In `crates/sarv-desktop/src/ui/views/settings.rs`:
1. Replace the 2-tab layout (`General`, `Keyboard shortcuts`) with **4 focused tabs**:
   - **Tab 0 — General**: Appearance & language (`Theme`, `Interface language`, `Display digits`, `Money unit`, `Calendar`) + `Point of sale` toggle. Every control already auto-saves on change, so **remove the redundant `"Save preferences"` header button** (check `tests/settings.rs` first; if a test clicks `btn-Save preferences`, keep or update the test).
   - **Tab 1 — Business & Backups**:
     - Business profile card with `"Edit business details"` button (`open_business`) inline.
     - Companies list with `"Add company"` button (`start_add_company`) inline.
     - Backups & folders card with `"Create backup"` and `"Restore backup"` buttons inline, plus Books folder and Exports folder (`Show in folder`).
   - **Tab 2 — Security**:
     - Dedicated, uncluttered App Lock section (status badge, plain-language explanation, Enable / Lock now / Change password / Disable controls).
   - **Tab 3 — Shortcuts & About**:
     - Keyboard shortcuts list (update `main.rs` `F1` handler to set `s.settings.tab_idx = 3`).
     - About section: Version, Logs folder (`Show in folder`), and Law data reference button (`Law data`).
2. Pass `vec![]` (no cluttered 6-button toolbar!) as the `actions` argument to `ui.page("settings-screen", ...)` — every action now lives right next to the section it belongs to.
3. Add any new tab/section English keys to `fa_messages.rs` so `every_ui_key_has_a_persian_entry` passes.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → exit 0.

### Step 4: Update `docs/DESIGN.md` and headless tests

1. Update `docs/DESIGN.md`:
   - Document `Ui::form_dialog` (and `Ui::secret_field` / `Ui::date_field` / `Ui::money_field` / `Ui::hinted`) in §5.
   - Update §7 to reflect the clean `nav-{page}` sidebar keys and `org-selector` / `nav-settings` footer.
2. Update any tests in `crates/sarv-desktop/src/tests/shell.rs` and `settings.rs` that referenced removed custom sidebar keys (`nav-security`, `nav-crm`, etc.) or Settings tab indices, and add tests verifying:
   - Sidebar renders only the 3 clean groups + `nav-settings` footer (no duplicate rows).
   - All 4 Settings tabs render and switch cleanly, and `F1` opens Tab 3 (Shortcuts & About).

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check` → exit 0.

## Test plan

- Update `crates/sarv-desktop/src/tests/shell.rs` and `crates/sarv-desktop/src/tests/settings.rs` to match the simplified sidebar and 4-tab Settings structure.
- Add assertions that clicking each of the 4 Settings tabs renders its expected controls and that `nav-account-statement` / `nav-crm` / `nav-security` are gone.
- Verification: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass.

## Done criteria

- [ ] `cargo test -p sarv-desktop --bin sarv-desktop` exits 0
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` exits 0
- [ ] `cargo fmt --check` exits 0
- [ ] `grep -rn "nav-cheque-search\|nav-crm\|nav-account-card\|nav-account-statement\|nav-trial-balance\|nav-security" crates/sarv-desktop/src/` returns no matches
- [ ] `settings.rs` `ui.page("settings-screen", ...)` passes empty `vec![]` header actions (actions live inline in their respective tabs)
- [ ] Only in-scope files are modified (`git status`)
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back (do not improvise) if:
- Any test outside `tests/shell.rs` and `tests/settings.rs` (e.g. `tests/walkthrough.rs` or `tests/png_scenes.rs`) fails because it depends on `lang-toggle`, `theme-toggle`, `refresh-books`, or `btn-Save preferences` — inspect and report before changing out-of-scope test files.
- `every_ui_key_has_a_persian_entry` fails on a newly introduced label key (add the exact key to `fa_messages.rs`).
- A verification command fails twice after a reasonable fix attempt.

## Maintenance notes

- Keep the sidebar capped at ~10–11 primary items; sub-views (like Cheques, Trial Balance, Chart of Accounts, App Lock) belong as tabs inside their parent screen and as entries in the `Cmd+K` Command Palette, never as duplicate sidebar rows.
- Future UX polish: consider giving `Overview` a calm 3-card daily summary + 2 quick-action buttons ("New sale", "Record payment") so small-business owners can do 80% of daily work from one screen.
