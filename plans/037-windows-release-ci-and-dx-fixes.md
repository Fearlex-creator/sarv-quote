# Plan 037: Fix Windows release packaging, `reveal` command injection, `save_text_export`, MSRV, and DX leaks

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md` — unless a reviewer dispatched you and told you they
> maintain the index.
>
> **Drift check (run first)**: `git diff --stat 1e6866ac..HEAD -- .github/workflows/release.yml .github/workflows/ci.yml Cargo.toml crates/sarv-desktop/build.rs crates/sarv-desktop/src/ui/documents.rs crates/sarv-desktop/src/data.rs crates/sarv-desktop/src/test_helper.rs crates/sarv-desktop/src/tests/setup.rs`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts against the live code before proceeding; on a
> mismatch, treat it as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: S–M
- **Risk**: LOW
- **Depends on**: none
- **Category**: dx
- **Planned at**: commit `1e6866ac`, 2026-11-19

## Why this matters

Five build, security, and developer-experience defects affect releases and day-to-day work:

1. **Broken Windows release**: `.github/workflows/release.yml:87` runs `cargo build --release -p sarv-desktop -p sarv-quote --bin sarv-agent`, which skips building `sarv-desktop.exe` right before copying `target/release/sarv-desktop.exe`.
2. **Windows `cmd /C start` shell injection & unsanitized `save_text_export`**: `documents.rs:103-108` passes file paths through `cmd /C start` (interpreting `&`, `|`, `%VAR%`), and `save_text_export` joins raw filenames without `safe_name`/`unique_path` or `0o600` permissions.
3. **Stale MSRV**: `Cargo.toml:12` declares `rust-version = "1.85"` while the codebase uses Rust 1.88 let-chains (`if ... && let ...`).
4. **Temp-dir leaks & dead file**: `Engine::demo()` leaks ~50+ SQLite directories in `/tmp` per test run; `crates/sarv-desktop/src/test_helper.rs` is orphaned and uncompiled.
5. **Stale git SHA & CI gaps**: `crates/sarv-desktop/build.rs:4` watches `.git/HEAD` but not the branch ref file, so `SARV_GIT_SHA` goes stale on incremental builds; `.github/workflows/ci.yml` never compile-checks Windows.

## Current state

- `.github/workflows/release.yml:86-94`:
  ```yaml
  - name: Build agent and desktop app
    run: cargo build --release -p sarv-desktop -p sarv-quote --bin sarv-agent
  ...
  cp target/release/sarv-desktop.exe "$out/Sarv.exe"
  ```
- `crates/sarv-desktop/src/ui/documents.rs:82-110`:
  ```rust
  pub fn save_text_export(st: &State, filename: &str, content: &str) -> Result<PathBuf, String> {
      let dir = exports_dir(st)?;
      let path = dir.join(filename);
      std::fs::write(&path, content.as_bytes()).map_err(|e| e.to_string())?;
      Ok(path)
  }
  ...
  #[cfg(target_os = "windows")]
  let mut cmd = {
      let mut c = std::process::Command::new("cmd");
      c.args(["/C", "start", ""]).arg(target);
      c
  };
  ```
- `Cargo.toml:12`: `rust-version = "1.85"`.
- `crates/sarv-desktop/src/data.rs:81-84`: `Engine::demo()` creates `sarv-demo-<pid>-<count>` in `temp_dir()` and never removes it on `Drop`.
- `crates/sarv-desktop/src/test_helper.rs`: 5-line file not referenced by `main.rs`.
- `crates/sarv-desktop/build.rs:4`: emits `cargo:rerun-if-changed=../../.git/HEAD` only.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Desktop unit/UI tests | `cargo test -p sarv-desktop --bin sarv-desktop` | exit 0, all pass |
| Workspace clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, no warnings |
| Format check | `cargo fmt --check` | exit 0 |

## Scope

**In scope**:
- `.github/workflows/release.yml`
- `.github/workflows/ci.yml`
- `Cargo.toml`
- `crates/sarv-desktop/build.rs`
- `crates/sarv-desktop/src/ui/documents.rs`
- `crates/sarv-desktop/src/data.rs`
- `crates/sarv-desktop/src/test_helper.rs` (delete)
- `crates/sarv-desktop/src/tests/setup.rs`
- `crates/sarv-desktop/src/tests/settings.rs` (only `test_recovery_screen_restores_a_backup` to opt out of ephemeral auto-delete before `drop(rt)` and clean up `dir` at test end)

**Out of scope**:
- Changing the pinned `rune` / `rune-kit` git commit.
- Adding macOS release packaging or Windows code signing.

## Git workflow

- Branch: `advisor/037-windows-release-ci-and-dx-fixes`
- Conventional commits (e.g., `fix(ci): build sarv-desktop.exe explicitly in Windows release workflow`).
- Do NOT push or open a PR unless instructed.

## Steps

### Step 1: Fix Windows release build & add Windows check to CI

1. In `.github/workflows/release.yml:87`, split the build into two explicit commands:
   ```yaml
   run: |
     cargo build --release -p sarv-desktop --bin sarv-desktop
     cargo build --release -p sarv-quote --bin sarv-agent
   ```
2. In `.github/workflows/ci.yml`, add a lightweight `windows-check` job (using `windows-latest` or `x86_64-pc-windows-gnu` matching `release.yml`) that runs `cargo check -p sarv-desktop --bin sarv-desktop` so `#[cfg(windows)]` code paths cannot silently rot.
3. In `Cargo.toml:12`, update `rust-version = "1.88"` (the minimum version supporting let-chains used across the workspace).

**Verify**: `cargo check --workspace --all-targets` → exit 0.

### Step 2: Remove Windows `cmd /C start` and harden `save_text_export`

In `crates/sarv-desktop/src/ui/documents.rs`:
1. On `#[cfg(target_os = "windows")]` in `reveal(path: &Path, folder: bool)`:
   - Replace `Command::new("cmd").args(["/C", "start", ""]).arg(target)` with `Command::new("explorer")` (passing `format!("/select,{}", path.display())` when `folder` is true and `path` is a file, or `.arg(target)` otherwise) so no shell interprets metacharacters.
2. In `save_text_export(st: &State, filename: &str, content: &str)`:
   - Reject any `filename` that does not consist of a single `std::path::Component::Normal` (or split stem/ext and pass stem through `safe_name` + `unique_path` matching `write_ext`).
   - On `#[cfg(unix)]`, set file permissions to `0o600` (matching `write_ext`).
   - Add unit tests in `documents.rs` verifying path traversal inputs (`../evil.csv`, `a/b.csv`) are sanitized/rejected and repeated exports produce unique filenames.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop documents` → exit 0.

### Step 3: Clean up `Engine::demo()` temp directories, delete `test_helper.rs`, and fix `build.rs`

1. Delete `crates/sarv-desktop/src/test_helper.rs`.
2. In `crates/sarv-desktop/src/data.rs`:
   - Add `ephemeral: bool` to `Engine` (`false` in `open`/`open_at`, `true` in `demo`).
   - Implement `Drop for Engine`: if `self.ephemeral`, close the store and call `let _ = std::fs::remove_dir_all(&self.dir);`.
3. In `crates/sarv-desktop/src/tests/setup.rs` (`lines 8, 43`):
   - Replace manual `temp_dir()` folder creation with `tempfile::tempdir()` so panicking tests don't leak directories.
4. In `crates/sarv-desktop/build.rs`:
   - Read `../../.git/HEAD`; if it starts with `ref: `, emit `cargo:rerun-if-changed=../../.git/<ref_path>` in addition to `../../.git/HEAD`.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check` → exit 0.

## Test plan

- New unit tests in `crates/sarv-desktop/src/ui/documents.rs` for `save_text_export` path sanitization and uniquification.
- New unit test in `crates/sarv-desktop/src/data.rs` confirming `Engine::demo()` removes its temp directory on `Drop`.
- Verification: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass.

## Done criteria

- [ ] `cargo test -p sarv-desktop --bin sarv-desktop` exits 0
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` exits 0
- [ ] `cargo fmt --check` exits 0
- [ ] `crates/sarv-desktop/src/test_helper.rs` does not exist
- [ ] `grep -rn '"cmd"' crates/sarv-desktop/src/ui/documents.rs` returns no matches
- [ ] Only in-scope files are modified (`git status`)
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back (do not improvise) if:
- Any test in `crates/sarv-desktop/src/tests/` inspects `engine.dir` *after* dropping `State`/`Engine` (requiring `ephemeral` cleanup to stay on `Engine::drop` only).
- A verification command fails twice after a reasonable fix attempt.

## Maintenance notes

- Keep `rust-version` in `Cargo.toml` aligned whenever newer Rust syntax is adopted.
