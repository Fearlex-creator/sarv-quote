# Plan 034: Fix app-lock encryption vulnerabilities, WAL checkpointing, crash-safe sealing, and UI tests

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md` — unless a reviewer dispatched you and told you they
> maintain the index.
>
> **Drift check (run first)**: `git diff --stat 1e6866ac..HEAD -- crates/sarv-desktop/src/security.rs crates/sarv-desktop/src/data.rs crates/sarv-desktop/src/app.rs crates/sarv-desktop/src/ui/views/lock.rs crates/sarv-desktop/src/ui/views/settings.rs crates/sarv-desktop/src/tests/settings.rs`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts against the live code before proceeding; on a
> mismatch, treat it as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: LOW–MED
- **Depends on**: none
- **Category**: security
- **Planned at**: commit `1e6866ac`, 2026-11-19

## Why this matters

Commit `31c99a5e` added password-based App Lock with on-disk file encryption (`crates/sarv-desktop/src/security.rs`), but introduced five concrete security and data-loss hazards:

1. Legacy v1 lock records and missing `app_lock.json` return a hardcoded all-zero DEK `[0u8; 32]`, silently encrypting books with a publicly known key or sealing & deleting `sarv.sqlite3` without a lock file.
2. `write_atomic_private` writes plaintext/ciphertext with default `0o644` permissions before calling `set_permissions(0o600)`, never calls `sync_all()`, and immediately deletes the source file — a crash can permanently destroy `sarv.sqlite3`.
3. `Engine::close_store` drops SQLite without `PRAGMA wal_checkpoint(TRUNCATE)` and `seal_workspace_data` ignores `sarv.sqlite3-wal`/`-shm`, leaving plaintext WAL frames on disk.
4. `lock_app` drops `active_dek` and marks locked even if sealing fails, leaves sensitive forms/records/modals in memory, and partial `unseal_workspace_data` failures leave `sarv.sqlite3` plaintext while blocking unlock; `Engine::open` also ignores `.sarvlock`/`app_lock.json` in legacy directory detection.
5. `test_app_lock_ui_flow` runs 4× 600,000 PBKDF2 rounds synchronously in debug mode and never tests successful unlock, password change, disable, or `Drop` auto-sealing.

Fixing these first protects user books from corruption and plaintext leaks, and gives fast, thorough tests for any future crypto upgrades.

## Current state

- `crates/sarv-desktop/src/security.rs`:
  - `verify_and_record` (`lines 127-169`):
    ```rust
    let Some(mut rec) = load(dir)? else {
        return Ok(VerifyOutcome::Unlocked([0u8; 32]));
    };
    ...
    } else {
        // Legacy v1 compatibility
        let derived = pbkdf2_hmac_sha256(password.as_bytes(), &salt, iters);
        (constant_time_eq(&derived, &expected), [0u8; 32])
    };
    ```
  - `write_atomic_private` (`lines 307-318`):
    ```rust
    fn write_atomic_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
        let tmp = path.with_file_name(format!(".{name}.tmp"));
        fs::write(&tmp, bytes).map_err(|e| format!("Failed to write file: {e}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600));
        }
        fs::rename(&tmp, path).map_err(|e| format!("Failed to save file: {e}"))?;
        Ok(())
    }
    ```
  - `seal_workspace_data` (`lines 218-256`) only seals `sarv.sqlite3`, top-level `*.json`, and `exports/*` without checking for non-empty `-wal`/`-journal` sidecars.
- `crates/sarv-desktop/src/data.rs`:
  - `Engine::open` (`lines 38-54`) only checks `sarv.sqlite3.exists()`, missing `sarv.sqlite3.sarvlock` and `app_lock.json`.
  - `Engine::close_store` (`lines 126-130`) drops `*slot = None` without checkpointing WAL.
- `crates/sarv-desktop/src/app.rs`:
  - `lock_app` (`lines 458-479`) clears only `"lock-password"` from `self.forms`, leaves `detail`, `toasts`, and per-view states in memory, and sets `self.locked = true` even if `seal_workspace_data` fails.
- `crates/sarv-desktop/src/ui/views/lock.rs`:
  - `attempt_unlock` (`lines 57-89`) calls `unseal_workspace_data` and returns early on error without staging/rollback.
- `crates/sarv-desktop/src/ui/views/settings.rs`:
  - `lines 696, 714` pass `security::DEFAULT_ITERATIONS` directly instead of a test-overridable constant.
- `crates/sarv-desktop/src/tests/settings.rs`:
  - `test_app_lock_ui_flow` (`lines 458-503`) tests only enable, lock, and 3-attempt lockout.

Conventions from `AGENTS.md` and `CONTEXT.md`:
- Books live in a SQLite-backed profile directory (`SARV_DATA_DIR`).
- Keep `DEFAULT_ITERATIONS = 600_000` for production builds; only lower iterations under `#[cfg(test)]`.
- Never log or print password or key material.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Desktop unit/UI tests | `cargo test -p sarv-desktop --bin sarv-desktop` | exit 0, all pass |
| Workspace clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, no warnings |
| Format check | `cargo fmt --check` | exit 0 |

## Scope

**In scope** (the only files you should modify):
- `crates/sarv-desktop/src/security.rs`
- `crates/sarv-desktop/src/data.rs`
- `crates/sarv-desktop/src/app.rs`
- `crates/sarv-desktop/src/ui/views/lock.rs`
- `crates/sarv-desktop/src/ui/views/settings.rs`
- `crates/sarv-desktop/src/tests/settings.rs`
- `crates/sarv-quote/src/store.rs` (only if a `pub fn checkpoint_wal(&self) -> Result<(), ...>` method is needed on `Store`)

**Out of scope** (do NOT touch):
- Replacing ChaCha20/PBKDF2 with Argon2id/AEAD (separate future migration plan).
- Moving PBKDF2/sealing off the UI thread.
- Any accounting logic in `crates/sarv-quote`.

## Git workflow

- Branch: `advisor/034-app-lock-security-and-crash-safety`
- Conventional commits matching `git log` (e.g., `fix(desktop): harden app-lock DEK handling and crash-safe sealing`).
- Do NOT push or open a PR unless instructed.

## Steps

### Step 1: Eliminate all-zero DEK in `security.rs` and add `UI_ITERATIONS`

1. In `crates/sarv-desktop/src/security.rs`:
   - Add `pub const UI_ITERATIONS: u32 = if cfg!(test) { 1_000 } else { DEFAULT_ITERATIONS };`.
   - Enforce a sane bound when reading `rec.iterations` in `verify_and_record`: reject `rec.iterations == 0` or `rec.iterations > 5_000_000` with `Err("Invalid lock file parameters".to_string())`.
   - Change `verify_and_record` when `load(dir)?` is `None`: return `Err("App lock is not enabled".to_string())` instead of `Ok(VerifyOutcome::Unlocked([0u8; 32]))`.
   - In the legacy v1 branch (`rec.version < 2`): if `constant_time_eq(&derived, &expected)` is `true`, generate a fresh random 32-byte DEK via `getrandom::fill(&mut dek)`, upgrade the lock file to v2 via `save_with_dek(dir, password, &dek, iters)?`, and return `Ok(VerifyOutcome::Unlocked(dek))`. Never return `[0u8; 32]`.
2. In `crates/sarv-desktop/src/ui/views/settings.rs` (`lines 696, 714`), pass `security::UI_ITERATIONS` instead of `security::DEFAULT_ITERATIONS`.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop security` → exit 0, all security tests pass.

### Step 2: Make `write_atomic_private` crash-safe and eliminate Unix permission TOCTOU

In `crates/sarv-desktop/src/security.rs`:
1. Rewrite `write_atomic_private(path: &Path, bytes: &[u8]) -> Result<(), String>`:
   - Use `OpenOptions::new().write(true).create(true).truncate(true)` and on `#[cfg(unix)]` set `.mode(0o600)` via `std::os::unix::fs::OpenOptionsExt` *before* writing any bytes.
   - Write all bytes, call `file.sync_all()`, and drop `file`.
   - On `fs::rename(&tmp, path)` failure, remove `&tmp` (`let _ = fs::remove_file(&tmp);`) before returning `Err`.
   - On `#[cfg(unix)]`, open the parent directory (`path.parent()`) and call `dir_file.sync_all()` (best-effort if unsupported by filesystem, otherwise return error).

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop security` → exit 0.

### Step 3: Checkpoint SQLite WAL before closing & guard WAL sidecars during seal

1. In `crates/sarv-quote/src/store.rs`, add a small method `pub fn checkpoint_wal(&self) -> Result<(), String>` on `Store` that executes `PRAGMA wal_checkpoint(TRUNCATE)` and checks that `busy == 0` and `log == checkpointed` (returning `Err` if frames remain uncheckpointed).
2. In `crates/sarv-desktop/src/data.rs`:
   - Update `Engine::close_store(&self) -> Result<(), String>` to call `store.checkpoint_wal()` before dropping `*slot = None`.
   - Add `fn has_books(dir: &Path) -> bool` checking `dir.join("sarv.sqlite3").exists() || dir.join("sarv.sqlite3.sarvlock").exists() || dir.join("app_lock.json").exists()`, and use it in `Engine::open` (`lines 38-54`) for both the primary and legacy directory checks.
3. In `crates/sarv-desktop/src/security.rs` inside `seal_workspace_data`:
   - Before sealing `sarv.sqlite3`, check `sarv.sqlite3-wal` and `sarv.sqlite3-journal`: if either exists with `len() > 0`, return `Err("Database has uncheckpointed WAL data; cannot seal safely".to_string())`.
   - After sealing `sarv.sqlite3`, remove any 0-byte `sarv.sqlite3-wal` and `sarv.sqlite3-shm` sidecars.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → exit 0.

### Step 4: Harden `lock_app`, `attempt_unlock`, and in-memory state wipe

1. In `crates/sarv-desktop/src/app.rs` (`lock_app`, `lines 458-479`):
   - Close the store (`if let Err(e) = self.engine.close_store()`) and seal workspace data (`if let Some(dek) = self.active_dek.as_ref()`) *before* discarding `active_dek` and locking. If closing or sealing fails, log the error, toast a visible `Tone::Danger` error, keep `self.active_dek` intact, and return without falsely locking.
   - On success, drop `self.active_dek = None`, set `self.locked = true`, and reset all sensitive/transient state: `self.books = Books::default()`, `self.forms = Forms::default()`, `self.toasts.clear()`, `self.detail = Default::default()`, `self.operation = Default::default()`, `self.confirm = None`, `self.palette_open = false`, `self.period_switch_open = false`, `self.company_switch_open = false`, `self.startup_summary_open = false`, and all per-view states (`sales`, `purchases`, `payments`, `banking`, `contacts`, `inventory`, `accounting`, `reports`, `sheets`, `settings`, `overview`, `pos`) to `Default::default()`. Then set `self.forms.edit("lock-password").focused = true`.
2. In `crates/sarv-desktop/src/security.rs` (`unseal_workspace_data`):
   - Unseal `sarv.sqlite3.sarvlock` and top-level `*.json.sarvlock` first; if an optional file under `exports/` fails to unseal, collect/log the warning rather than leaving `sarv.sqlite3` plaintext while aborting unlock.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → exit 0.

### Step 5: Add full app-lock UI & security tests

Add tests covering:
- `security.rs` unit tests:
  - Missing `app_lock.json` returns `Err` (never `[0u8; 32]`).
  - Legacy v1 record upgrades to v2 with a non-zero random DEK on first unlock.
  - `seal_workspace_data` refuses to seal when a non-empty `sarv.sqlite3-wal` exists.
  - `DEFAULT_ITERATIONS == 600_000`.
- `data.rs` unit tests:
  - `has_books` recognizes `sarv.sqlite3.sarvlock` and `app_lock.json`.
- `tests/settings.rs` headless UI tests:
  - Full lifecycle: enable app lock → lock (`lock_app` wipes forms/detail/toasts/view modals) → unlock with correct password (books reload, `startup_summary_open == true`) → change password → lock → unlock with new password → disable app lock (`.sarvlock` removed, `sarv.sqlite3` opens cleanly).
  - `Drop for State` auto-seals `sarv.sqlite3` to `sarv.sqlite3.sarvlock` when dropped unlocked.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check` → exit 0.

## Test plan

- Model UI tests after `test_app_lock_ui_flow` in `crates/sarv-desktop/src/tests/settings.rs:458-503`.
- Model crypto/file tests after existing `#[cfg(test)] mod tests` in `crates/sarv-desktop/src/security.rs`.
- Verification: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass, including ≥6 new tests, and runtime stays fast thanks to `UI_ITERATIONS`.

## Done criteria

- [ ] `cargo test -p sarv-desktop --bin sarv-desktop` exits 0 with all new tests passing
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` exits 0
- [ ] `cargo fmt --check` exits 0
- [ ] `grep -rn "\[0u8; 32\]" crates/sarv-desktop/src/security.rs` does not appear in any `VerifyOutcome::Unlocked` return path
- [ ] Only in-scope files are modified (`git status`)
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back (do not improvise) if:
- `Store` in `crates/sarv-quote/src/store.rs` does not expose its `rusqlite::Connection` in a way that supports `PRAGMA wal_checkpoint(TRUNCATE)` without changing public engine APIs.
- Any existing test in `crates/sarv-desktop/src/tests/` relies on `verify_and_record` returning `Ok(Unlocked([0u8; 32]))` when no lock file exists.
- A verification command fails twice after a reasonable fix attempt.

## Maintenance notes

- A follow-up plan can migrate `LockRecord` v2 → v3 (Argon2id + `chacha20poly1305` + `zeroize` + AAD) safely on top of the UI tests added here.
- Reviewers should check that `lock_app` clears every new view state field added in the future (see Plan 036 for centralizing dialog/view resets).
