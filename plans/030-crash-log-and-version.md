# Plan 030: The app records crashes and errors to a log file and shows its version

> **Executor instructions**: Follow this plan step by step. Run every verification command. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-desktop/src/main.rs crates/sarv-desktop/src/app.rs crates/sarv-desktop/src/data.rs crates/sarv-desktop/src/ui/views/settings.rs crates/sarv-desktop/Cargo.toml Cargo.toml`

## Status

- **Priority**: P1 for a first public release
- **Effort**: S–M
- **Risk**: LOW
- **Depends on**: 008 (data directory resolution) recommended
- **Category**: dx / production readiness
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

When Sarv misbehaves on a user's machine there is nothing to look at: the desktop has no logging at all (no `eprintln!`, `log`, `tracing`), no panic hook, release binaries are stripped, and no screen shows which version is running. A crash in an accounting app closes the window and leaves the owner with no explanation and the developer with no report. This plan adds the minimum: a local log file, a crash record, and a visible version — all on the user's own disk, with nothing sent anywhere.

## Current state

- `crates/sarv-desktop/src/main.rs:56-118` — `fn main()` builds `App::new(AppSpec::<State> { … }).chrome(chrome).run()`. No panic hook.
- `grep -rn "eprintln!\|println!\|log::\|tracing\|set_hook\|CARGO_PKG_VERSION" crates/sarv-desktop/src` → nothing.
- Errors surface only as toasts: `State::run` (`crates/sarv-desktop/src/app.rs:347-363`) toasts the engine's message; `Books::refresh` stores a failure in `Books.error` (`crates/sarv-desktop/src/data.rs:87,96`).
- Root `Cargo.toml`: `[workspace.package] version = "0.1.0"`; `[profile.release] strip = true, lto = "thin"`. Published tags include `v0.1.0` and `v0.2.0-rc.1`, so the manifest version lags the tags.
- Data directory: `Engine::open()` in `data.rs:14-27` → `<data>/ir.sarv.accounting/books`; `Engine.dir` is that path. Logs should live beside the books folder: `<data>/ir.sarv.accounting/logs/`.
- Settings page: `crates/sarv-desktop/src/ui/views/settings.rs` — sections built with `ui.section(key, title, ui.panel_rows(key, rows))`; header actions via `ui.btn(Btn::Glass, "Label", None, |st| …)`.
- i18n: English keys; Persian in `crates/sarv-desktop/src/fa_messages.rs`.
- Privacy position of the product: local-first, no telemetry. Keep it: **no network calls** in this plan.
- The books contain financial data. Logs must not: log action names, error messages, timings and identifiers of screens — never payloads, amounts, party names or document contents.
- Workspace dependencies available without adding crates: `chrono`, `serde`, `serde_json`. Do not add a logging crate; a ~60-line file logger is enough.

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |
| Build | `cargo build -p sarv-desktop` | exit 0 |

## Scope

**In scope**: new `crates/sarv-desktop/src/logfile.rs`, `src/main.rs`, `src/app.rs`, `src/data.rs`, `src/ui/views/settings.rs`, new `crates/sarv-desktop/build.rs`, `crates/sarv-desktop/Cargo.toml` (only `build = "build.rs"` if needed), root `Cargo.toml` (version, release profile), `src/fa_messages.rs`, `CHANGELOG.md` (create).

**Out of scope**: telemetry, crash uploading, update checks (they need a network policy decision from the owner — listed in `plans/README.md`); engine crate.

## Git workflow

- Branch `advisor/030-crash-log-and-version`; commits `feat(desktop): local log file and crash record`, `feat(desktop): show version in Settings`, `build: set the workspace version and keep symbols for crash reports`.

## Steps

### Step 1: File logger

`src/logfile.rs`:

```rust
/// Append-only local log beside the books: `<data>/ir.sarv.accounting/logs/sarv.log`.
/// Never write payloads, amounts or names — action names and error text only.
pub fn init(books_dir: &std::path::Path)            // remembers the logs dir in a OnceLock; creates it (0700 on Unix)
pub fn line(level: &str, message: &str)             // "2026-11-13T10:22:01+03:30 INFO  message\n"; best-effort, never panics
pub fn logs_dir() -> Option<std::path::PathBuf>
```

Rotation: at `init`, if `sarv.log` is larger than 1 MiB rename it to `sarv.log.1` (replacing any old one). Each `line` opens the file in append mode, writes, and closes (simple and crash-safe; the volume is tiny). Demo mode (`SARV_DEMO`) logs into the demo temp directory like everything else.

Call `logfile::init(&engine.dir)` from `State::with_engine` (so tests also exercise it, inside their temp dirs). Log: app start with version and OS (`std::env::consts::OS`), the books directory path, every `State::run` failure as `ERROR action=<name> message=<engine text>`, every `Books::refresh` failure, and navigation is **not** logged.

Engine error text can include a document reference the user typed; that is acceptable. Do not log the request payload.

Unit tests in `logfile.rs` (use `tempfile::tempdir()`, already a dev-dependency): a line is written with a timestamp and level; rotation happens above the size limit; `line` before `init` is a no-op and does not panic.

### Step 2: Panic hook

In `main()` before `App::new`, install `std::panic::set_hook` that: builds a text with timestamp, version, OS, the panic message, `location()` (file:line), and `std::backtrace::Backtrace::force_capture()`; writes it to `<logs>/crash-<YYYYMMDD-HHMMSS>.txt` and appends a one-line summary to `sarv.log`; then calls the previous hook (keep it with `take_hook`) so the default stderr output still happens. The logs directory must be resolvable before `State::open` runs: compute the data directory first (use plan 008's `books_dir` function if landed, else the same logic `Engine::open` uses) and call `logfile::init` early in `main`.

On the next start, if a crash file newer than the last "start" line exists, show a one-time info toast after the first frame: "Sarv closed unexpectedly last time. A report was saved on this device." with (if plan 025's toast actions exist) a "Show in folder" action. Track "seen" by renaming the file to `crash-…-seen.txt`.

Keep at most 10 crash files (delete the oldest).

### Step 3: Useful backtraces in release builds

In root `Cargo.toml` change `[profile.release]` from `strip = true` to `strip = "debuginfo"` and add `debug = "line-tables-only"`… these conflict (stripping debuginfo removes line tables). Choose: `strip = "none"`, `debug = "line-tables-only"` and measure the binary size before/after (`ls -l target/release/sarv-desktop` after `cargo build --release -p sarv-desktop`); if the size grows by more than ~30%, fall back to `strip = "debuginfo"` with no `debug` setting (symbol names remain, line numbers do not) and record the measurement in the PR description. Either way the panic `location()` (file:line of the panic itself) is always available.

### Step 4: Version in the app

- Set `[workspace.package] version` to the next release the owner intends; if unknown, set `0.2.0` (the published pre-release tag is `v0.2.0-rc.1`) and flag it in your report.
- `crates/sarv-desktop/build.rs`: run `git rev-parse --short HEAD` and emit `cargo:rustc-env=SARV_GIT_SHA=<sha>`; if git is unavailable emit `unknown`. Add `cargo:rerun-if-changed=../../.git/HEAD`.
- Settings: a new section "About" with rows: "Version" → `env!("CARGO_PKG_VERSION")` + ` (` + `env!("SARV_GIT_SHA")` + `)` wrapped in `i18n::isolate`; "Books folder" (if plan 008 did not add it); "Logs folder" with a `Btn::Ghost` "Show in folder" (reuse plan 025's `documents::reveal` if present; otherwise show the path only).
- Also print the version on `--version`: check whether Rune's `App` already handles CLI flags (`--scene`, `--snap` are handled by Rune); if `std::env::args().any(|a| a == "--version")`, print `sarv-desktop <version> (<sha>)` and exit before creating the app.

Persian: `("About", "درباره")`, `("Version", "نسخه")`, `("Logs folder", "پوشهٔ گزارش‌ها")`, and the crash toast sentence: «سرو بار قبل به‌طور غیرمنتظره بسته شد. گزارشی روی همین دستگاه ذخیره شد.»

### Step 5: Changelog

Create `CHANGELOG.md` (Keep a Changelog format) with an "Unreleased" section listing what landed from `plans/` so far (read the status table in `plans/README.md`), and a `0.1.0` entry pointing at the git tag.

### Step 6: Tests (`main.rs`)

1. `test_failed_action_is_logged_without_payload`: trigger a failing `run` (journal with the same account on both sides, amount `123456`); read `sarv.log` from the runtime's logs dir; assert it contains `ERROR action=journal_post` and does **not** contain `123456`.
2. `test_settings_shows_the_version`: navigate to Settings; assert an element with key `about-version` exists.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass. Manual: `cargo run -p sarv-desktop -- --version` prints the version.

## Done criteria

- [ ] `grep -n "set_hook" crates/sarv-desktop/src/main.rs` matches
- [ ] `grep -rn "CARGO_PKG_VERSION" crates/sarv-desktop/src` matches
- [ ] `grep -rn "http\|TcpStream\|reqwest\|ureq" crates/sarv-desktop/src/logfile.rs` returns nothing
- [ ] Desktop suite passes with the new tests; `CHANGELOG.md` exists
- [ ] `plans/README.md` updated

## STOP conditions

- Rune installs its own panic hook or aborts on panic in a way that bypasses `set_hook` (check `grep -rn "set_hook\|panic = " ~/.cargo/git/checkouts/rune-*/b556a38/crates/rune/src Cargo.toml`). Report what you find.
- The log would need to include payload data to be useful for a specific error — do not add it; report the case.

## Maintenance notes

- A support workflow can now say: "Settings → About → Logs folder → send `sarv.log` and the newest `crash-*.txt`." Add that sentence to the README when plan 032 rewrites it.
- Any future update check or crash upload is a privacy decision for the owner and must be opt-in.
