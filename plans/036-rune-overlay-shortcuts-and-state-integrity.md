# Plan 036: Align Rune overlay z-order, Escape/shortcut guards, deferred nav, and palette focus

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md` — unless a reviewer dispatched you and told you they
> maintain the index.
>
> **Drift check (run first)**: `git diff --stat 1e6866ac..HEAD -- crates/sarv-desktop/src/ui/mod.rs crates/sarv-desktop/src/app.rs crates/sarv-desktop/src/main.rs crates/sarv-desktop/src/shortcuts.rs crates/sarv-desktop/src/ui/shell.rs crates/sarv-desktop/src/tests/shell.rs`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts against the live code before proceeding; on a
> mismatch, treat it as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: S–M
- **Risk**: LOW
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `1e6866ac`, 2026-11-19

## Why this matters

Four state/overlay mismatches make keyboard navigation and modals feel unpredictable in the Rune desktop:

1. **Overlay z-order vs Escape conflict**: `ui/mod.rs` paints `confirm` underneath `detail`, `startup_summary`, switchers, and `palette`, while `request_close_top_dialog` closes `confirm` first and `main.rs` disables palette keys whenever `confirm.is_some()`.
2. **Shortcut guard holes & wrong Settings labels**: `any_dialog_open()` omits 6 dialogs (`period_switch_open`, `company_switch_open`, `period_add_open`, `acct_archive_open`, `acct_open`, `spend_open`), global shortcuts (`Cmd+K`, `F1`, `Cmd+B`, etc.) fire during first-run setup/recovery or over active dialogs, and `shortcuts::CREATE` advertises `Alt+<KEY>` while `handle_key` requires `Cmd+Alt`.
3. **Dirty `Sheets` navigation leak**: shortcuts and sidebar actions set `modal_open = true` *before* `st.nav(...)` asks to leave a dirty sheet; cancelling leaves an invisible open modal that permanently blocks shortcuts.
4. **Command palette focus**: `open_palette()` never focuses `"palette-query"`, forcing an extra click after `Cmd+K`.

## Current state

- `crates/sarv-desktop/src/ui/mod.rs:52-61`:
  ```rust
  operation_dialog::mount(s, &ui, &mut layer);
  confirm::mount(s, &ui, &mut layer);
  detail::mount(s, &ui, &mut layer);
  views::accounting::mount_ledger(s, &ui, &mut layer);
  views::accounting::mount_period_add_dialog(s, &ui, &mut layer);
  shell::mount_period_switcher(s, &ui, &mut layer);
  shell::mount_company_switcher(s, &ui, &mut layer);
  shell::mount_startup_summary(s, &ui, &mut layer);
  shell::mount_palette(s, &ui, &mut layer);
  shell::mount_toasts(s, &ui, &mut layer);
  ```
- `crates/sarv-desktop/src/app.rs`:
  - `any_dialog_open` (`lines 493-517`): missing 6 dialog flags.
  - `request_close_top_dialog` (`lines 520-552`): closes `confirm`, `startup_summary`, `detail`, `operation`, `palette`, `period_switch`, `company_switch`, `period_add` — mismatched with `ui/mod.rs`.
  - `open_palette` (`lines 652-657`): does not set `self.forms.edit("palette-query").focused = true`.
  - `fill_books_context` (`lines 700-708`): uses `ui-{pid}-{nanos}` without `self.key_counter`.
- `crates/sarv-desktop/src/main.rs:62-110`: global shortcuts do not check `books_ready` or `!s.any_dialog_open()`.
- `crates/sarv-desktop/src/shortcuts.rs:14-144`: calls `open_*` before `st.nav(...)`, and sets `keys: &["Alt", "..."]`.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Desktop unit/UI tests | `cargo test -p sarv-desktop --bin sarv-desktop` | exit 0, all pass |
| Workspace clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, no warnings |
| Format check | `cargo fmt --check` | exit 0 |

## Scope

**In scope** (the only files you should modify):
- `crates/sarv-desktop/src/ui/mod.rs`
- `crates/sarv-desktop/src/app.rs`
- `crates/sarv-desktop/src/main.rs`
- `crates/sarv-desktop/src/shortcuts.rs`
- `crates/sarv-desktop/src/ui/shell.rs` (only `nav_then` call sites; full sidebar redesign is Plan 039)
- `crates/sarv-desktop/src/tests/shell.rs`

**Out of scope**:
- Visual redesign of the sidebar or Settings (Plan 039).
- App-lock encryption changes (Plan 034).

## Git workflow

- Branch: `advisor/036-rune-overlay-shortcuts-and-state-integrity`
- Conventional commits (e.g., `fix(desktop): align overlay z-order with Escape close priority`).
- Do NOT push or open a PR unless instructed.

## Steps

### Step 1: Canonicalize overlay paint order and Escape close order

1. In `crates/sarv-desktop/src/ui/mod.rs` (`lines 52-61`), mount overlays bottom-to-top in this exact order:
   1. `operation_dialog::mount`
   2. `detail::mount`
   3. `views::accounting::mount_ledger`
   4. `views::accounting::mount_period_add_dialog`
   5. `shell::mount_period_switcher`
   6. `shell::mount_company_switcher`
   7. `shell::mount_startup_summary`
   8. `shell::mount_palette`
   9. `confirm::mount` (always above all modals/palette)
   10. `shell::mount_toasts` (always topmost)
2. In `crates/sarv-desktop/src/app.rs` (`request_close_top_dialog`, `lines 520-552`), close in the exact reverse (top-to-bottom) order: `confirm` → `palette_open` → `startup_summary_open` → `company_switch_open` → `period_switch_open` → `accounting.period_add_open` → `accounting.ledger_open` → `detail.open` → `operation.open` → per-view dialogs.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop shell` → exit 0.

### Step 2: Fix `any_dialog_open`, shortcut readiness guards, palette focus, and `mint_key`

1. In `crates/sarv-desktop/src/app.rs`:
   - Add `pub fn books_ready(&self) -> bool` returning `!self.books.load_failed() && self.books.has_company() && self.books.initialized() && !self.setup.adding_company`.
   - Add the 6 missing flags to `any_dialog_open()`: `self.period_switch_open`, `self.company_switch_open`, `self.accounting.period_add_open`, `self.accounting.acct_archive_open`, `self.accounting.acct_open`, `self.payments.spend_open`.
   - In `open_palette()`, set `self.forms.edit("palette-query").focused = true`.
   - Back `mint_key` and `fill_books_context` with a shared monotonic `AtomicU64` counter so `request_key`s never collide within the same clock tick.
2. In `crates/sarv-desktop/src/main.rs` (`handle_key`):
   - Gate `F1`, `F9`, `Cmd+B`, `Cmd+Alt+<key>`, and `Cmd+K` (when opening) on `s.books_ready()`.
   - Gate opening `palette` (`Cmd+K`) and `F1` on `!s.any_dialog_open()` (still allow `Cmd+K` to *close* an already-open palette when `s.confirm.is_none()`).
3. Check `crates/sarv-desktop/src/ui/views/settings.rs:91-94`: note it already prepends `mod_key()` to `sc.keys`, so `shortcuts::CREATE` `keys: &["Alt", "N"]` renders as `Ctrl+Alt+N` / `⌘+Alt+N`. Verify no other caller displays `sc.keys` without `mod_key()`; add a doc comment on `shortcuts::CREATE` noting `mod_key()` is implicit so future callers don't drift.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → exit 0.

### Step 3: Add `State::nav_then` so shortcuts & sidebar actions never mutate state before dirty `Sheets` confirms

1. In `crates/sarv-desktop/src/app.rs`:
   - Add `pub fn nav_then(&mut self, page: Page, after: impl FnOnce(&mut State) + 'static)`: if leaving dirty `Page::Sheets`, pass `after` into the `Confirm` dialog's `on_confirm` closure; otherwise call `self.apply_nav(page)` then `after(self)`.
2. Update `crates/sarv-desktop/src/shortcuts.rs` (`CREATE` actions) and `crates/sarv-desktop/src/ui/shell.rs` (`btn-New order` and any remaining custom nav rows) to call `st.nav_then(Page::..., |st| { ... })` instead of mutating modal/tab state before `st.nav(...)`.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check` → exit 0.

### Step 4: Add comprehensive keyboard shortcut & overlay tests

In `crates/sarv-desktop/src/tests/shell.rs`, add tests verifying:
- `handle_key` returns `false` and mutates nothing when `rt.state.locked == true`.
- `F1`, `F9`, `Cmd+L`, `Cmd+Shift+L`, and all `shortcuts::CREATE` chords work when ready and are blocked when a dialog is open or books are not ready.
- `open_palette()` focuses `"palette-query"`.
- Triggering a creation shortcut from dirty `Page::Sheets` and cancelling `confirm` leaves `any_dialog_open() == false` and target `modal_open == false`.
- Opening `confirm` while `palette` is open closes `confirm` on Escape first and keeps `palette` intact.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → exit 0, all new tests pass.

## Test plan

- Model after existing shortcut tests in `crates/sarv-desktop/src/tests/shell.rs:212-228, 373-493`.
- Verification: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass, including ≥5 new tests.

## Done criteria

- [ ] `cargo test -p sarv-desktop --bin sarv-desktop` exits 0
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` exits 0
- [ ] `cargo fmt --check` exits 0
- [ ] Overlay order in `ui/mod.rs` is the exact reverse of `request_close_top_dialog` in `app.rs`
- [ ] Only in-scope files are modified (`git status`)
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back (do not improvise) if:
- `Confirm::open` in `crates/sarv-desktop/src/ui/confirm.rs` takes `Fn` instead of `FnOnce` and cannot accept `after` without a small signature adjustment (if so, check if `Option<Box<dyn FnOnce(&mut State)>>` inside a `Cell`/`RefCell` or `Fn` fits cleanly; stop if it requires wide changes).
- A verification command fails twice after a reasonable fix attempt.

## Maintenance notes

- Whenever a new overlay/dialog is added, register it in `ui/mod.rs`, `any_dialog_open`, `request_close_top_dialog`, and `lock_app` together.
