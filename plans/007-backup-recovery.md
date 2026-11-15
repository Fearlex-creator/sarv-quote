# Plan 007: A user whose books fail to load can still restore a backup

> **Executor instructions**: Follow this plan step by step. Run every verification command. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-quote/src/desktop_bridge.rs crates/sarv-quote/src/store.rs crates/sarv-desktop/src/ui/views/settings.rs crates/sarv-desktop/src/ui/mod.rs crates/sarv-desktop/src/data.rs`

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: MED (touches the restore path — the user's last line of defence)
- **Depends on**: 003 (the `Engine` shape changes there)
- **Category**: bug
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

Every desktop action — including "list backups" and "restore backup" — first opens the store, which parses and fully validates the saved books. If that fails (a damaged row, or a newer release with a stricter validator), the error text tells the user to "keep the file", every action fails, and the app shows the first-run setup screen as if the books were empty. There is no path back to a backup from inside the app. Separately, the engine writes a safety copy before every restore and schema upgrade (`before-restore-*.json`, `before-schema*-*.json`), but the backup list and the restore action only accept the `sarv-backup-` prefix, so those safety copies can never be used and are never cleaned up.

## Current state

- `crates/sarv-quote/src/desktop_bridge.rs:1106-1109` — `dispatch` calls `Store::open(directory)?` before looking at the action.
- `crates/sarv-quote/src/store.rs:200-214` — `Store::open` fails on unparseable JSON (`"دادهٔ محلی نامعتبر؛ فایل را نگه دارید"`) or `state.validate()` error.
- `desktop_bridge.rs:1165-1184` — `"backup_list"` lists only `name.starts_with("sarv-backup-") && name.ends_with(".json")` and calls the full `Store::restore_preview` per file (parse + validate, twice).
- `desktop_bridge.rs:1185-1222` — `"backup_restore_preview" | "backup_restore"`:
  ```rust
  ensure!(
      !name.contains('/') && !name.contains('\\') && !name.contains("..")
          && name.starts_with("sarv-backup-") && name.ends_with(".json"),
      "Invalid backup filename"
  );
  ```
  Restore requires `p["confirmed"] == true` and checks `expected_generation` when supplied, then `store.restore(candidate)` which writes `before-restore-{id}.json` first (`store.rs:541-551`).
- `store.rs:506-511` `Store::restore_preview(path) -> Result<State>` is an associated function — it does **not** need an open store.
- Desktop: `crates/sarv-desktop/src/data.rs:69-99` `Books::refresh` stores the failure in `self.error` and leaves `workspace` empty; `crates/sarv-desktop/src/ui/mod.rs:23` then routes to `views::setup::view` because `!s.books.has_company()`.
- Desktop restore dialog: `crates/sarv-desktop/src/ui/views/settings.rs:325-381` (`mount_restore`), shows `rows.iter().take(6)`.
- Engine test pattern: `crates/sarv-quote/tests/desktop_bridge.rs:1-19` (`fixture()` + `call(dir, action, json)`); storage tests in `crates/sarv-quote/tests/storage_hardening.rs`.

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Engine tests | `cargo test -p sarv-quote` | all pass |
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |

## Scope

**In scope**: `crates/sarv-quote/src/desktop_bridge.rs`, `crates/sarv-quote/src/store.rs`, `crates/sarv-quote/tests/desktop_bridge.rs` (or a new `tests/recovery.rs`), `crates/sarv-desktop/src/data.rs`, `src/ui/mod.rs`, `src/ui/views/settings.rs`, a new `src/ui/views/recovery.rs` (+ `views/mod.rs`), `src/fa_messages.rs`, `src/main.rs` (tests).

**Out of scope**: the backup file format; the 64 MiB limit; `crates/sarv-quote/src/agent.rs` (backups stay a desktop-only capability, as the comment at `desktop_bridge.rs:1154` says).

## Git workflow

- Branch `advisor/007-backup-recovery`; commits `feat(engine): recover from backups when the books do not load`, `feat(desktop): recovery screen`, `fix(engine): list and restore safety backups`.

## Steps

### Step 1: Recovery entry points that do not need a loaded store

In `store.rs` add:

```rust
/// Replace books that fail to load with a verified backup. The unreadable row is
/// first copied to `unreadable-{id}.json` (raw payload, not a backup envelope) so
/// nothing is destroyed. Never call this when `Store::open` succeeds.
pub fn recover_from_backup(directory: &Path, backup: &Path) -> Result<PathBuf>
```

Implementation: `ensure!(Store::open(directory).is_err(), "Books load normally; use Restore backup")`; `let candidate = Self::restore_preview(backup)?`; open the SQLite connection directly; read `(generation, payload)`; write the raw payload with `write_new` to `directory/unreadable-{id}.json`; `UPDATE state SET generation=?1, payload=?2 WHERE id=1 AND generation=?3` with `generation+1` and the serialized candidate; `ensure!(changed == 1)`; finally `Store::open(directory)?` must succeed, else return the error.

In `desktop_bridge.rs` change `dispatch` so that when `Store::open` fails **and** the action is one of `backup_list`, `backup_restore_preview`, `recover_from_backup`, it is served without a store:

- `backup_list` → same listing (factor the body into a function taking `&Path`).
- `backup_restore_preview` → `{"backup": summary(candidate), "current": null}`.
- `recover_from_backup` → requires `p["confirmed"] == true`, applies the same filename validation, calls `Store::recover_from_backup`.

For any other action keep returning the open error unchanged.

**Verify**: `cargo build -p sarv-quote` → exit 0.

### Step 2: Safety backups are listable and restorable

Introduce one function `fn backup_file_kind(name: &str) -> Option<&'static str>` returning `"manual"` for `sarv-backup-*.json`, `"before-restore"` for `before-restore-*.json`, `"before-upgrade"` for `before-schema*-*.json`, and `None` otherwise (keep the `/`, `\`, `..` rejection). Use it in both the list (add a `"kind"` field to each row) and the restore filename check.

Make the listing cheap: instead of `Store::restore_preview` per file, deserialize only what is needed. If `Backup` (the envelope struct in `store.rs`) cannot be partially read, keep the full preview but only for the newest 20 files, and mark the rest `"valid": null`.

Retention: after a successful `backup_create`, delete `before-restore-*` and `before-schema*` files beyond the newest 5 of each kind. Never delete `sarv-backup-*` (user-created).

### Step 3: Engine tests

In `crates/sarv-quote/tests/` (new file `recovery.rs`, same helpers as `desktop_bridge.rs:1-19`):

1. `recover_restores_books_when_the_row_is_unreadable`: seed fixture; `backup_create`; corrupt the row with rusqlite (`UPDATE state SET payload='{'`); assert `dispatch(dir,"workspace",…)` errs; `backup_list` succeeds and lists the backup; `recover_from_backup` with `confirmed:true` succeeds; `workspace` now succeeds; a file `unreadable-*.json` exists.
2. `recover_is_refused_when_books_load`: on healthy books, `recover_from_backup` errs.
3. `safety_backup_can_be_restored`: `backup_create`, `backup_restore` (creates `before-restore-*`), then `backup_list` includes a row with `kind == "before-restore"` and `backup_restore` of that file succeeds.
4. `restore_rejects_other_filenames`: `../x.json`, `sarv.sqlite3`, `evil.json` are all rejected.

**Verify**: `cargo test -p sarv-quote --test recovery` → 4 passed; `cargo test -p sarv-quote` → all pass.

### Step 4: Desktop recovery screen

- `data.rs`: add `pub fn load_failed(&self) -> bool { !self.error.is_empty() && self.workspace.is_null() }` on `Books`.
- `ui/mod.rs` `root_view`: before the setup check, `if s.books.load_failed() { views::recovery::view(...) }` and treat the window as not-ready (no sidebar).
- New `ui/views/recovery.rs`: a centred card using the existing helpers (`ui.heading`, `ui.muted`, `ui.btn`, `ui.empty` — see `ui/glass.rs`): title `"Your books could not be opened"`, the engine error text, the books folder path (`s.engine.dir.display()`), a "Try again" button (`st.refresh()`), and the backup list (reuse the row-building code from `settings.rs:329-348` by extracting it into a shared function). Choosing a backup shows its summary; a `Btn::Danger` "Restore this backup" button calls `st.run("recover_from_backup", json!({"filename": name, "confirmed": true}), Some("Saved"))`.
- Add Persian strings for every new key to `fa_messages.rs` (e.g. `("Your books could not be opened", "دفاتر شما باز نشد"),`, `("Try again", "تلاش دوباره"),`, `("Restore this backup", "بازیابی این پشتیبان"),`).
- `settings.rs` `mount_restore`: show the `kind` next to each filename (translated labels "Manual backup", "Before restore", "Before upgrade") and raise `take(6)` to a scrollable list of all rows.

### Step 5: Desktop test

`test_recovery_screen_restores_a_backup` in `main.rs`: make a runtime; `rt.state.run("backup_create", json!({}), None)`; corrupt the row via `rusqlite`… the desktop crate has no rusqlite dependency, so instead expose a `#[doc(hidden)] pub fn corrupt_for_test(directory: &Path)` in the engine's `store.rs` behind `#[cfg(any(test, feature = "test-support"))]` — if adding a feature is awkward, skip the UI test, keep the engine tests, and say so in your report.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass.

## Done criteria

- [ ] `cargo test -p sarv-quote --test recovery` → 4 passed
- [ ] `cargo test -p sarv-quote` and desktop tests pass
- [ ] `grep -n "recover_from_backup" crates/sarv-desktop/src/ui/views/recovery.rs` matches
- [ ] `grep -n "recover_from_backup" crates/sarv-quote/src/agent.rs` returns nothing (not exposed to agents)
- [ ] `plans/README.md` updated

## STOP conditions

- `recover_from_backup` cannot be implemented without making private `Store` internals public beyond one function.
- Any existing test in `storage_hardening.rs`, `process_recovery.rs` or `export_hardening.rs` fails.
- You find yourself deleting or overwriting the unreadable payload without first writing it to `unreadable-*.json`.

## Maintenance notes

- Reviewer focus: the SQL `UPDATE … WHERE generation=?` in recovery must keep the optimistic check; the raw copy must be written and synced before the update.
- Future schema bumps must keep `backup_file_kind` in sync with the names `Store::open` writes.
