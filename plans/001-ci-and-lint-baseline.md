# Plan 001: CI runs fmt, clippy and both test suites on every push

> **Executor instructions**: Follow this plan step by step. Run every verification command and confirm the expected result before moving on. If anything in "STOP conditions" occurs, stop and report — do not improvise. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- Cargo.toml .github crates/sarv-quote/src/views.rs crates/sarv-quote/src/desktop_bridge.rs crates/sarv-quote/src/sheets.rs`
> If these changed since the plan was written, re-run the baseline commands in Step 1 and work from what they report today.

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: LOW
- **Depends on**: none
- **Category**: dx
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

The repository has 44 engine test files and 34 headless desktop tests, but nothing runs them automatically: the only workflow is `.github/workflows/release-windows.yml`, which builds on a `v*` tag without testing. `cargo fmt --check` and `cargo clippy -D warnings` already fail, so nobody can use them as a gate. Every other plan in `plans/` relies on "tests are green" as its done criterion; this plan makes that signal automatic and trustworthy.

## Current state

- `Cargo.toml` (workspace root) — members `crates/sarv-quote` (engine) and `crates/sarv-desktop` (Rune UI). `edition = "2024"`. No `rust-version`, no `[workspace.lints]`. `[profile.dev] debug = 0` (lines 37-38) strips line numbers from panics.
- No `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `justfile` or `Makefile`.
- `.github/workflows/release-windows.yml` — the only workflow; installs floating `stable`.
- Measured at planning time:
  - `cargo fmt --all -- --check` → fails; 64 files differ (almost all of `crates/sarv-desktop`, many in `crates/sarv-quote`).
  - `cargo clippy --workspace --all-targets -- -D warnings` → fails with 5 errors in the `sarv-quote` lib (`src/views.rs` ×2, `src/desktop_bridge.rs` ×2, `src/sheets.rs` ×1: `sort_by_key`, `Result<_, ()>`, collapsible `if`, needless borrow). Without `-D` there are about 21 more warnings in the `sarv-desktop` bin.
  - `cargo test -p sarv-desktop --bin sarv-desktop` → 34 passed.
- The `pdf-export` test suite shells out to Poppler tools (`pdftotext`, `pdfinfo`, `pdffonts`, `pdftoppm`); the README does not say so.
- `crates/sarv-desktop/tests/png_scenes.rs` needs a GPU and must NOT run in CI.
- The desktop depends on `rune` / `rune-kit` from `https://github.com/anomalyco/rune` at a pinned rev. **Do not change that pin.**

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Format check | `cargo fmt --all -- --check` | exit 0 |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| Engine tests | `cargo test -p sarv-quote` | `test result: ok` for every binary |
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | `34 passed` (or more) |

## Scope

**In scope**:
- `rust-toolchain.toml` (create), `rustfmt.toml` (create), `Cargo.toml` (root)
- `.github/workflows/ci.yml` (create)
- Any `.rs` file under `crates/`, but only for mechanical formatting and for the clippy fixes
- `README.md` (one sentence about Poppler)

**Out of scope**:
- `crates/sarv-desktop/Cargo.toml` rune/rune-kit `rev` — frozen by owner decision.
- `.github/workflows/release-windows.yml` — plan 031 owns it.
- Any behaviour change. A clippy fix that changes what the code does is not allowed here.

## Git workflow

- Branch: `advisor/001-ci-and-lint-baseline`
- Commits, in this order and kept separate: `style: cargo fmt the workspace` (formatting only, nothing else), `fix(engine): clear clippy errors`, `fix(desktop): clear clippy warnings`, `build: pin toolchain and add CI`.
- Do not push or open a PR unless told to.

## Steps

### Step 1: Record the baseline

Run the four commands in the table and save the output summaries. Tests must pass before you change anything.

**Verify**: both test commands report `test result: ok`.

### Step 2: Pin the toolchain

Run `rustc --version`. Create `rust-toolchain.toml`:

```toml
[toolchain]
channel = "<the exact version printed, e.g. 1.90.0>"
components = ["rustfmt", "clippy"]
```

Add `rust-version = "1.85"` under `[workspace.package]` in the root `Cargo.toml` and `rust-version.workspace = true` in both crate manifests' `[package]`.

**Verify**: `cargo build -p sarv-quote` → exit 0.

### Step 3: Format

Look at 3–4 long lines in `crates/sarv-desktop/src/app.rs` (e.g. the `GROUPS` constant is ~140 columns). The existing code is written wide. Create `rustfmt.toml` with `max_width = 140` so the formatting diff stays small, then run `cargo fmt --all`.

Commit this alone as `style: cargo fmt the workspace`.

**Verify**: `cargo fmt --all -- --check` → exit 0; both test commands still pass.

### Step 4: Fix clippy

Run `cargo clippy --workspace --all-targets -- -D warnings`, fix what it reports, repeat until clean. Use the fix clippy suggests. For `Result<_, ()>`, introduce a small error type or `Option` only if the function is private; if it is `pub`, add `#[allow(clippy::result_unit_err)]` with a one-line comment rather than changing the signature.

**Verify**: clippy command → exit 0; both test commands pass.

### Step 5: Readable panics in dev

In root `Cargo.toml` change `[profile.dev] debug = 0` to `debug = "line-tables-only"`.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → passes.

### Step 6: Add CI

Create `.github/workflows/ci.yml`: trigger on `push` and `pull_request`; one job on `ubuntu-latest`:

1. `actions/checkout@v4`
2. `sudo apt-get update && sudo apt-get install -y libcairo2-dev libpango1.0-dev poppler-utils pkg-config` plus whatever windowing dev packages the `sarv-desktop` build asks for (run the build, read the error, add the package — typically `libxkbcommon-dev libwayland-dev libx11-dev`).
3. `actions/cache@v4` on `~/.cargo/registry`, `~/.cargo/git`, `target`, key `linux-${{ hashFiles('Cargo.lock') }}`.
4. `cargo fmt --all -- --check`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `cargo test -p sarv-quote`
7. `cargo test -p sarv-quote --features pdf-export`
8. `cargo test -p sarv-desktop --bin sarv-desktop`

Do not add `--test png_scenes`. Set `permissions: contents: read`.

**Verify**: `python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/ci.yml'))"` → exit 0.

### Step 7: Document Poppler

In `README.md`, after the sentence "Linux PDF rendering needs cairo and pango.", add: "The `pdf-export` tests also need the Poppler command-line tools (`poppler-utils`)."

## Test plan

No new tests. The gate itself is the deliverable.

## Done criteria

- [ ] `cargo fmt --all -- --check` exits 0
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` exits 0
- [ ] `cargo test -p sarv-quote` and `cargo test -p sarv-desktop --bin sarv-desktop` pass
- [ ] `.github/workflows/ci.yml` and `rust-toolchain.toml` exist
- [ ] `git diff a4776749 -- crates/sarv-desktop/Cargo.toml | grep rev` prints nothing
- [ ] `plans/README.md` status row updated

## STOP conditions

- Any test fails after the format commit (formatting must not change behaviour — investigate, do not paper over).
- A clippy fix would require changing a public function signature or runtime behaviour.
- The desktop crate cannot be built on the CI image because the pinned Rune revision needs a system library you cannot identify.

## Maintenance notes

- The first CI run on GitHub is the real verification of Step 6; expect one or two missing apt packages.
- `png_scenes` stays a local, GPU-only check.
- Later plans assume fmt and clippy are clean and say so in their done criteria.
