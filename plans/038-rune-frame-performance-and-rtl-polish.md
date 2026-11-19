# Plan 038: Eliminate per-frame Rune overhead and fix RTL/bidi/design-system polish bugs

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md` — unless a reviewer dispatched you and told you they
> maintain the index.
>
> **Drift check (run first)**: `git diff --stat 1e6866ac..HEAD -- crates/sarv-desktop/src/app.rs crates/sarv-desktop/src/ui/kit.rs crates/sarv-desktop/src/ui/rows.rs crates/sarv-desktop/src/ui/shell.rs crates/sarv-desktop/src/ui/views/mod.rs crates/sarv-desktop/src/ui/views/lock.rs crates/sarv-desktop/src/ui/views/setup.rs crates/sarv-desktop/src/ui/views/sheets.rs crates/sarv-desktop/src/ui/views/overview.rs crates/sarv-desktop/src/ui/views/settings.rs crates/sarv-desktop/src/ui/views/inventory.rs crates/sarv-desktop/src/ui/views/contacts.rs crates/sarv-desktop/src/i18n.rs crates/sarv-desktop/src/fa_messages.rs`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts against the live code before proceeding; on a
> mismatch, treat it as a STOP condition.

## Status

- **Priority**: P2
- **Effort**: M
- **Risk**: LOW
- **Depends on**: none
- **Category**: perf
- **Planned at**: commit `1e6866ac`, 2026-11-19

## Why this matters

Every Rune frame currently performs avoidable disk I/O, theme reconstruction, and full-array clones, while every state refresh eagerly rebuilds all 8 views (twice on unlock/company-switch). At the same time, several small RTL/Persian/design-system defects violate `docs/DESIGN.md` and `AGENTS.md`:

1. **Per-frame overhead**: `build_toolbar` calls `app_lock_enabled()` (`is_file()` syscall every frame); `lock.rs` parses `app_lock.json` twice per frame; `Ui::new` rebuilds `Kit` every frame; `cmp_key` allocates 2 folded `String`s per sort comparison even though `SortKey::text` already folds; `mount_startup_summary` clones `workspace["cheques"]` 3 times per frame; `refresh()` + `apply_settings()` eagerly rebuilds all 8 views twice on unlock/company-switch.
2. **RTL & design-system polish**: `setup.rs` ignores `SARV_TODAY` and shows Latin `"1405"` next to Persian dates; `sheets.rs` and `Fmt::qty` break Persian separators and RTL negative signs; Latin refs/SKUs/phones/filenames lack `i18n::isolate`; `overview.rs` hardcodes `"IRR"` in Persian Rial mode and toasts `"Name"` on empty input; `settings.rs` uses literal `760.0` width and doubled `-dialog-dialog` keys.

## Current state

- `crates/sarv-desktop/src/app.rs:454-456`: `app_lock_enabled()` hits `security::is_enabled(&self.engine.dir)` on disk every call.
- `crates/sarv-desktop/src/ui/kit.rs:22-42`: `sarv_kit(dark)` builds a fresh `Kit` every frame.
- `crates/sarv-desktop/src/ui/rows.rs:58-79`: `SortKey::text` folds text at construction, then `cmp_key` folds it again on every comparison.
- `crates/sarv-desktop/src/ui/shell.rs:724-736, 788-790`: `cheque_bucket` clones `workspace["cheques"]` 3 times per render.
- `crates/sarv-desktop/src/ui/views/setup.rs:123, 129, 212`: uses `chrono::Local::now().date_naive()` instead of `crate::data::today()`, and raw `&fiscal.0`.
- `crates/sarv-desktop/src/ui/views/sheets.rs:173-188, 208` and `crates/sarv-desktop/src/i18n.rs:100-108, 285-298`: ASCII commas/minus without `\u{2066}...\u{2069}` bidi isolation.
- `crates/sarv-desktop/src/ui/views/overview.rs:256-259, 440-444, 620`: `"Name"` toast, hardcoded `"IRR"`, un-isolated `#{short}`.
- `crates/sarv-desktop/src/ui/views/settings.rs:513, 636, 656-659, 806`: un-isolated backup/law keys, literal `760.0`, `"law-data-dialog"` / `"app-lock-dialog"` passed to `ui.dialog` (which appends `-dialog`).

Design rules from `docs/DESIGN.md`:
- §6: Latin identifiers (codes, references, filenames) are wrapped with `i18n::isolate`.
- §7: Dialogs use `{dialog}-dialog` and `{dialog}-close`.
- §9: No hard-coded numeric dialog widths in views (use `DIALOG_SM` / `DIALOG_MD` / `DIALOG_LG`).

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Desktop unit/UI tests | `cargo test -p sarv-desktop --bin sarv-desktop` | exit 0, all pass |
| Workspace clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, no warnings |
| Format check | `cargo fmt --check` | exit 0 |

## Scope

**In scope**:
- `crates/sarv-desktop/src/app.rs`
- `crates/sarv-desktop/src/ui/kit.rs`
- `crates/sarv-desktop/src/ui/rows.rs`
- `crates/sarv-desktop/src/ui/shell.rs`
- `crates/sarv-desktop/src/ui/views/mod.rs`
- `crates/sarv-desktop/src/ui/views/lock.rs`
- `crates/sarv-desktop/src/ui/views/setup.rs`
- `crates/sarv-desktop/src/ui/views/sheets.rs`
- `crates/sarv-desktop/src/ui/views/overview.rs`
- `crates/sarv-desktop/src/ui/views/settings.rs`
- `crates/sarv-desktop/src/ui/views/inventory.rs`
- `crates/sarv-desktop/src/ui/views/contacts.rs`
- `crates/sarv-desktop/src/i18n.rs`
- `crates/sarv-desktop/src/fa_messages.rs`

**Out of scope**:
- Sidebar/Settings layout redesign (Plan 039).
- Background worker thread for PDF/crypto (separate spike).

## Git workflow

- Branch: `advisor/038-rune-frame-performance-and-rtl-polish`
- Conventional commits (e.g., `perf(desktop): cache Kit, app_lock_enabled, and lazy-build active view rows`).
- Do NOT push or open a PR unless instructed.

## Steps

### Step 1: Eliminate per-frame disk I/O, `Kit` rebuilds, sort allocations, and redundant clones

1. In `crates/sarv-desktop/src/app.rs`:
   - Add cached `pub app_lock_cached: bool` initialized from `security::is_enabled(&engine.dir)` at startup and updated via `pub fn refresh_lock_status(&mut self)` whenever app lock is enabled or disabled in `settings.rs`. Have `app_lock_enabled(&self)` return `self.app_lock_cached`.
2. In `crates/sarv-desktop/src/ui/views/lock.rs`:
   - Cache lock lockout metadata on `LockState` (updated on lock/unlock/wrong-password) instead of reading & parsing `app_lock.json` twice per render frame.
3. In `crates/sarv-desktop/src/ui/kit.rs`:
   - Cache `sarv_kit(false)` and `sarv_kit(true)` in `std::sync::OnceLock<Kit>` (cloning or returning `&'static Kit` if `Ui` supports it; if `Kit` is `Clone`, cloning from `OnceLock` still avoids recomputing OKLCH tokens every frame).
4. In `crates/sarv-desktop/src/ui/rows.rs`:
   - Verify all `SortKey::Text(...)` constructions go through `SortKey::text(...)` (pre-folded), then compare `x.cmp(y)` directly in `cmp_key` without calling `fold_text` inside the comparator.
5. In `crates/sarv-desktop/src/ui/shell.rs` (`mount_startup_summary`):
   - Compute the 3 cheque buckets in a single borrowed pass over `s.books.workspace["cheques"].as_array()` instead of cloning the array 3 times per frame.
6. In `crates/sarv-desktop/src/ui/views/mod.rs` and `app.rs`:
   - Add `ensure_page(s: &mut State, page: Page)` and call it for `self.page` in `refresh()`/`apply_settings()`/`set_lang()` and in `apply_nav(page)` instead of eagerly rebuilding all 8 views on every refresh.
   - In `apply_select_company` and `attempt_unlock`, avoid double-calling `ensure_page` by applying settings before the single `ensure_page`.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → exit 0.

### Step 2: Fix `SARV_TODAY` in `setup.rs`, RTL number isolation, and design-system token drift

1. In `crates/sarv-desktop/src/ui/views/setup.rs`:
   - Add `fn today_naive() -> chrono::NaiveDate` parsing `crate::data::today()` (falling back to `chrono::Local::now().date_naive()`), and use it at `lines 123, 212`.
   - Format the fiscal-year placeholder with `s.fmt.digits(&fiscal.0)` at `line 129`.
2. In `crates/sarv-desktop/src/i18n.rs` and `crates/sarv-desktop/src/ui/views/sheets.rs`:
   - Add `Fmt::grouped_i64(&self, n: i64) -> String` using Persian/Latin thousands separator (`٬`/`,`), minus sign (`−`), and `\u{2066}...\u{2069}` bidi isolation; use it in `sheets.rs:208`.
   - Wrap negative `Fmt::qty` outputs in `\u{2066}...\u{2069}` and fix the stale doc comment on `persian_digits` (`lines 100-108`).
3. Wrap Latin identifiers with `crate::i18n::isolate(...)`:
   - `overview.rs:620` (`#{short}`), `settings.rs:513` (backup `name`), `settings.rs:636` (law `key`), `inventory.rs:197,209` (`code`), `contacts.rs:154-160,175` (`phone`).
4. In `overview.rs`:
   - Replace `lines 440-444` (`"IRR"`) with `s.fmt.unit_label()`.
   - Replace `st.fmt.t("Name")` at `line 257` with `"Please enter a name."` (add Persian translation in `fa_messages.rs`).
5. In `settings.rs`:
   - Replace literal `760.0` (`line 659`) with `DIALOG_LG` (or `DIALOG_MD`).
   - Change `"law-data-dialog"` (`line 656`) to `"law-data"` and `"app-lock-dialog"` (`line 806`) to `"app-lock"` so keys are `law-data-dialog` and `app-lock-dialog` (not `-dialog-dialog`).
   - Fix `mount_restore` empty-selection toast (`lines 551-555`), localize `preview_summary` digits/labels (`lines 490-494`), toast `"Books restored from backup"` on restore, and clear `contacts.selected`, `inventory.selected`, and `reports.selected_account_id` on restore.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check` → exit 0.

## Test plan

- Add unit tests in `crates/sarv-desktop/src/i18n.rs` for `Fmt::grouped_i64` (positive/negative in Fa & En) and negative `Fmt::qty` bidi isolation.
- Add a test in `crates/sarv-desktop/src/tests/setup.rs` verifying `SARV_TODAY` controls the default fiscal year in setup.
- Verify `every_ui_key_has_a_persian_entry` still passes after adding `"Please enter a name."`.
- Verification: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass.

## Done criteria

- [ ] `cargo test -p sarv-desktop --bin sarv-desktop` exits 0
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` exits 0
- [ ] `cargo fmt --check` exits 0
- [ ] `grep -rn "760.0" crates/sarv-desktop/src/ui/views/settings.rs` returns no matches
- [ ] `grep -rn '"IRR"' crates/sarv-desktop/src/ui/views/overview.rs` returns no matches
- [ ] Only in-scope files are modified (`git status`)
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back (do not improvise) if:
- Any test in `crates/sarv-desktop/src/tests/` inspects another page's `rows` *before* navigating to that page (if so, call `views::<page>::ensure` in that test or report which test depends on eager cross-page row building).
- `rune_kit::Kit` is not `Clone` (if so, keep `sarv_kit` as-is and note it).
- A verification command fails twice after a reasonable fix attempt.

## Maintenance notes

- If any new code calls `security::enable` / `security::disable`, remember to call `st.refresh_lock_status()`.
