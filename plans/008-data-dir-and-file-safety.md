# Plan 008: Books live in the right place on every OS, with private permissions and portable file writes

> **Executor instructions**: Follow this plan step by step. Run every verification command. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-desktop/src/data.rs crates/sarv-quote/src/store.rs`

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: MED (data location; the "never overwrite" export guarantee)
- **Depends on**: 003 (same file `data.rs`)
- **Category**: bug / security
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

Three problems with where and how the books are written:

1. **Windows**: the data directory falls back to the relative path `sarv_books` because Windows sets neither `XDG_DATA_HOME` nor `HOME`. The released `Sarv.exe` therefore keeps a company's books next to wherever it was launched from; launched from a different folder it opens empty books. Directory-creation errors are ignored.
2. **Permissions**: the books folder, the SQLite file and every backup are created with default permissions (world-readable under a normal umask), while the far less sensitive agent token file is created `0600`.
3. **Portability of writes**: backups and exports are published with a hard link plus a directory `fsync`. Hard links fail on FAT/exFAT USB sticks and many network shares; opening a directory as a file fails on Windows. Interrupted writes leave `.sarv-write-*.tmp` files that nothing removes.

## Current state

- `crates/sarv-desktop/src/data.rs:14-27`:
  ```rust
  pub fn open() -> Self {
      let dir = if let Ok(v) = std::env::var("SARV_DATA_DIR") {
          PathBuf::from(v)
      } else if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
          PathBuf::from(xdg).join("ir.sarv.accounting/books")
      } else if let Ok(home) = std::env::var("HOME") {
          PathBuf::from(home).join(".local/share/ir.sarv.accounting/books")
      } else {
          PathBuf::from("sarv_books")
      };
      let _ = std::fs::create_dir_all(&dir);
      Engine { dir }
  }
  ```
- `crates/sarv-quote/src/store.rs:200-202`: `fs::create_dir_all(directory)?; let conn = Connection::open(directory.join("sarv.sqlite3"))?;` — no mode.
- `crates/sarv-quote/src/store.rs:562-591` `write_new(path, bytes)`:
  ```rust
  let staging = parent.join(format!(".sarv-write-{}.tmp", crate::domain::id()));
  let mut file = OpenOptions::new().write(true).create_new(true).open(&staging)…;
  …
  if let Err(error) = fs::hard_link(&staging, path) { let _ = fs::remove_file(&staging); return Err(error).context(…); }
  fs::remove_file(&staging)?;
  fs::File::open(parent)?.sync_all()?;
  ```
  Guarantee to preserve (comment in the code): "Neither an existing file nor a symlink can be overwritten."
- Exemplar for private files — `crates/sarv-quote/src/agent.rs:140-148`:
  ```rust
  #[cfg(unix)]
  {
      use std::os::unix::fs::OpenOptionsExt;
      options.mode(0o600);
  }
  ```
- Tests that pin the write guarantees: `crates/sarv-quote/tests/export_hardening.rs`, `storage_hardening.rs`, `process_recovery.rs`.

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Engine tests | `cargo test -p sarv-quote` | all pass |
| Hardening tests | `cargo test -p sarv-quote --test export_hardening --test storage_hardening --test process_recovery` | all pass |
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |
| Windows type-check (optional, if target installed) | `cargo check -p sarv-quote --target x86_64-pc-windows-gnu` | exit 0 |

## Scope

**In scope**: `crates/sarv-desktop/src/data.rs`, `crates/sarv-quote/src/store.rs`, new tests in `crates/sarv-quote/tests/storage_hardening.rs`, `crates/sarv-desktop/src/main.rs` (tests), `crates/sarv-desktop/src/ui/views/settings.rs` (show the folder), `src/fa_messages.rs`, `README.md` (one paragraph).

**Out of scope**: adding a crate dependency such as `dirs` (resolve from environment variables by hand — three lines per OS); encryption of the books.

## Git workflow

- Branch `advisor/008-data-dir-and-file-safety`; commits `fix(desktop): store Windows and macOS books under the user profile`, `fix(engine): private permissions for books and backups`, `fix(engine): publish files without hard links where unsupported`.

## Steps

### Step 1: Resolve the data directory per OS

Extract a pure function in `data.rs`:

```rust
/// `get` reads an environment variable. Pure so every platform can be tested anywhere.
pub fn books_dir(os: &str, get: &dyn Fn(&str) -> Option<String>) -> Result<PathBuf, String>
```

Order: `SARV_DATA_DIR` (any OS) → on `"windows"`: `%APPDATA%\ir.sarv.accounting\books` → on `"macos"`: `$HOME/Library/Application Support/ir.sarv.accounting/books` → otherwise `XDG_DATA_HOME/ir.sarv.accounting/books`, else `$HOME/.local/share/ir.sarv.accounting/books`. If nothing resolves return `Err("No home folder found. Set SARV_DATA_DIR.")` — never a relative path.

**Existing users must not lose their books**: on macOS, if the new location does not exist but the old one (`$HOME/.local/share/ir.sarv.accounting/books/sarv.sqlite3`) does, keep using the old one. On Windows, if `%APPDATA%\…\books\sarv.sqlite3` does not exist but `.\sarv_books\sarv.sqlite3` (current directory) does, keep using `.\sarv_books` and remember that in a field `legacy_location: bool` so Settings can show a notice. Do not move files automatically.

`Engine::open()` calls it with `std::env::consts::OS` and `std::env::var(..).ok()`. Replace `let _ = create_dir_all` by storing the error: add `pub open_error: Option<String>` to `Engine`; `Books::refresh` should surface it through `self.error` when set.

Unit tests (in `data.rs` `#[cfg(test)]`): one per branch using a closure over a `HashMap` — linux XDG, linux HOME, windows APPDATA, macos, override, nothing-set → Err.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop data::` → all pass.

### Step 2: Show the books folder in Settings

In `ui/views/settings.rs`, in the business/company section, add one row: label `ui.t("Books folder")`, value `s.engine.dir.display().to_string()` rendered with `ui.muted`, wrapped in `crate::i18n::isolate(..)` so the Latin path keeps its direction in RTL. Add `("Books folder", "پوشهٔ دفاتر"),` to `fa_messages.rs`.

### Step 3: Private permissions (Unix)

In `store.rs`:
- After `fs::create_dir_all(directory)?` in `Store::open`, on Unix set the directory to `0o700` **only if this call created it** (check `directory.exists()` before creating) — do not change permissions of a folder the user pointed `SARV_DATA_DIR` at.
- After opening the connection, on Unix `chmod 0o600` the files `sarv.sqlite3`, `sarv.sqlite3-wal`, `sarv.sqlite3-shm` when they exist (ignore NotFound).
- In `write_new`, add `.mode(0o600)` under `#[cfg(unix)]` exactly like `agent.rs:143-148` — but only when the destination's parent is the store directory. `write_new` is also used for exports the user asks to save elsewhere; those keep default permissions. If `write_new` has no way to know, add a sibling `write_new_private` and use it from `backup`/`backup_state`.

Test in `storage_hardening.rs` (`#[cfg(unix)]`): open a store in a fresh tempdir subfolder, create a backup, assert `metadata.permissions().mode() & 0o077 == 0` for the folder, the sqlite file and the backup.

### Step 4: Portable publish in `write_new`

Keep the staging file and its `sync_all`. Publish with:

1. `fs::hard_link(&staging, path)`; on success remove staging (ignore the removal error but report nothing).
2. If the hard link fails with `ErrorKind::AlreadyExists` → return the existing "file exists" error (guarantee preserved).
3. If it fails for any other reason (unsupported filesystem): create the destination with `OpenOptions::new().write(true).create_new(true)` (this is the no-overwrite, no-follow-symlink check), copy the bytes, `sync_all`, then remove staging.
4. Directory sync only under `#[cfg(unix)]`.

Add `fn sweep_stale_staging(directory: &Path)` called from `Store::open`: delete `.sarv-write-*.tmp` in the store directory whose modified time is older than one hour. Ignore all errors.

Tests: existing `export_hardening.rs` must pass unchanged. Add `stale_staging_files_are_swept_on_open` (create a `.sarv-write-x.tmp`, set its mtime two hours back with `filetime`-free std API `File::set_modified`, open the store, assert it is gone; a fresh one stays).

**Verify**: hardening tests command → all pass.

### Step 5: README

In `README.md` replace the sentence "Without it, the app uses `ir.sarv.accounting/books` under the XDG data directory." with the three per-OS locations.

## Done criteria

- [ ] `grep -n '"sarv_books"' crates/sarv-desktop/src/data.rs` appears only in the legacy-location check
- [ ] `grep -n "let _ = std::fs::create_dir_all" crates/sarv-desktop/src/data.rs` → only inside `demo()`
- [ ] Engine and desktop test suites pass; new permission and sweep tests exist
- [ ] `plans/README.md` updated

## STOP conditions

- Any test in `export_hardening.rs` fails after Step 4 — the no-overwrite guarantee is more important than portability; revert Step 4 and report.
- The store directory is used by tests with pre-existing looser permissions that then fail — report which.

## Maintenance notes

- The Windows fallback path could not be exercised on this machine; ask a Windows user to confirm the books folder shown in Settings after the first release that includes this.
- A future "Move my books" action can use `legacy_location` to offer a one-click move.
