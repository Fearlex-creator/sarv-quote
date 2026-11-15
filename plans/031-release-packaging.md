# Plan 031: Releases are built from tested commits, for Windows and Linux, with checksums and a proper Windows app

> **Executor instructions**: Follow this plan step by step. Run every verification command you can run locally; the workflow itself can only be fully verified on GitHub — say clearly in your report what was verified where. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- .github/workflows scripts/install-desktop.sh crates/sarv-desktop/Cargo.toml crates/sarv-desktop/src/main.rs crates/sarv-quote/src/pdf_fonts.rs`

## Status

- **Priority**: P1 for a first public release
- **Effort**: M
- **Risk**: MED (release pipeline; cannot be fully exercised locally)
- **Depends on**: 001 (CI) must be DONE; 008 (Windows data directory) and 030 (version, crash log) should be DONE before the first release built with this
- **Category**: production readiness
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

The only release automation is a Windows zip built on a tag push, with no tests run, no checksum, and an executable that (as built today) opens a console window behind the app and has a generic icon. Linux users must install a Rust toolchain plus cairo/pango headers and run a script that cannot uninstall. The repository's public "Latest" release (`v0.1.0`) was built from a commit that predates the Rune desktop, and a later `v0.2.0-rc.1` release with `setup.exe`/`.deb` assets came from a pipeline that is not on this branch. The desktop build also depends entirely on one third-party git repository staying available.

## Current state

- `.github/workflows/release-windows.yml` (89 lines): triggers on `push: tags: ["v*"]` and `workflow_dispatch` (input `tag`); MSYS2 MINGW64 with cairo/pango/fontconfig; `rustup toolchain install stable-x86_64-pc-windows-gnu`; `cargo build --release -p sarv-desktop -p sarv-quote --bin sarv-agent`; collects `Sarv.exe`, `sarv-agent.exe` and every `/mingw64/` DLL reported by `ldd` into `Sarv-${TAG}-windows-x64/`, zips it, and `gh release create "$TAG" "$ASSET" --prerelease` (or `upload --clobber`). No test step, no checksum, `permissions: contents: write`.
- `crates/sarv-desktop/src/main.rs:1` — no `#![windows_subsystem = "windows"]`; no `build.rs`; no icon resource. Fonts are embedded with `include_bytes!("../../../assets/fonts/…")` (lines 13-16), so the exe needs no font files at run time.
- `crates/sarv-quote/assets` is a **symlink** to `../../assets` (git mode `120000`); `crates/sarv-quote/src/pdf_fonts.rs:55-67` uses `include_bytes!("../assets/fonts/…")` through it. A Windows checkout may materialise the symlink as a text file unless `core.symlinks` is enabled.
- `scripts/install-desktop.sh` (41 lines): copies `target/release/sarv-desktop` and `sarv-agent` to `${XDG_DATA_HOME:-$HOME/.local/share}/sarv/bin`, the icon `assets/brand/sarv.png` to `hicolor/128x128/apps/ir.sarv.accounting.png`, and writes `ir.sarv.accounting.desktop` (`Exec`, `Icon`, `Terminal=false`, `Categories=Office;Finance;`, `StartupNotify=true`). No uninstall, no `StartupWMClass`, no `TryExec`. `assets/brand/sarv.png` is 2.5 KB — check its pixel size with `file assets/brand/sarv.png`.
- `sarv-agent` is built and shipped, but nothing in the Rune desktop starts the agent service, so the binary cannot connect to anything (see plan 009's context). Shipping it is an owner decision; default here: keep building it but leave it out of the user-facing bundles until the desktop hosts it.
- Rune dependency: `crates/sarv-desktop/Cargo.toml` — `rune` and `rune-kit` from `https://github.com/anomalyco/rune` at `rev = "b556a38a1ff88aa8690d21b45ebf9b9167038674"`. **The pin must not be changed.** There is no vendored copy or mirror.
- The repository is public; release notes and assets are public.
- CI from plan 001: `.github/workflows/ci.yml` (fmt, clippy, engine tests, pdf-export tests, desktop headless tests) on push/PR.

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Workflow syntax | `python3 -c "import yaml,sys; [yaml.safe_load(open(f)) for f in sys.argv[1:]]" .github/workflows/*.yml` | exit 0 |
| Linux release build | `cargo build --release -p sarv-desktop` | exit 0 |
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |
| Engine PDF tests | `cargo test -p sarv-quote --features pdf-export` | all pass |
| Shell lint (if installed) | `shellcheck scripts/*.sh` | no errors |

## Scope

**In scope**: `.github/workflows/release-windows.yml` → rename to `release.yml` (one workflow, two build jobs), `scripts/install-desktop.sh`, new `scripts/package-linux.sh`, `crates/sarv-desktop/src/main.rs` (one attribute line), `crates/sarv-desktop/build.rs` + `crates/sarv-desktop/Cargo.toml` (`[build-dependencies]` for the Windows resource only), new `assets/brand/sarv.ico`, `crates/sarv-quote/src/pdf_fonts.rs` (paths) and removal of the `crates/sarv-quote/assets` symlink, `README.md` (Install section), new `docs/RELEASING.md`.

**Out of scope**: code signing (needs a certificate the owner must obtain — document the gap); macOS bundles; auto-update; changing the Rune pin; deleting or editing existing GitHub releases (owner action — see Maintenance notes).

## Git workflow

- Branch `advisor/031-release-packaging`; commits `build(windows): GUI subsystem and icon`, `build: no symlink for PDF fonts`, `build(release): test gate, checksums and a Linux bundle`, `build(linux): installer with uninstall`, `docs: releasing guide`.
- Do not push tags. Do not run `gh release` commands.

## Steps

### Step 1: Windows GUI subsystem and icon

- Add as the very first line of `crates/sarv-desktop/src/main.rs`: `#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]` (debug builds keep the console for development).
- Because a GUI-subsystem app has no stderr, this must ship together with plan 030's log file. If plan 030 is not DONE, add the attribute anyway but note the dependency in your report.
- Icon: create `assets/brand/sarv.ico` from the existing PNG (sizes 16, 32, 48, 256). If ImageMagick is available: `magick assets/brand/sarv.png -define icon:auto-resize=256,48,32,16 assets/brand/sarv.ico`. If the source PNG is smaller than 256 px, the result will be blurry — still generate it, and flag that the owner should supply a larger master image.
- Embed it: in `crates/sarv-desktop/build.rs` (create or extend the one from plan 030), under `#[cfg(windows)]`-equivalent logic (`std::env::var("CARGO_CFG_TARGET_OS") == "windows"`), compile a resource with the `winresource` crate added under `[target.'cfg(windows)'.build-dependencies]`. Confirm it supports the GNU toolchain used by the workflow (it needs `windres`, present in MSYS2's binutils — add `mingw-w64-x86_64-binutils` to the workflow's package list if not already pulled in).

**Verify**: `cargo build -p sarv-desktop` on Linux → exit 0 (the Windows-only code must not affect it).

### Step 2: Remove the font symlink

Change the `include_bytes!` paths in `crates/sarv-quote/src/pdf_fonts.rs` from `"../assets/fonts/…"` to `"../../../assets/fonts/…"` (the same form `crates/sarv-desktop/src/main.rs:13` uses), then `git rm crates/sarv-quote/assets`. Search for other users of the symlink: `grep -rn "sarv-quote/assets\|\.\./assets/" crates/ scripts/ .github/`.

**Verify**: `cargo test -p sarv-quote --features pdf-export` → all pass.

### Step 3: One release workflow with a test gate

Rename the workflow file to `.github/workflows/release.yml` (`git mv`) and restructure:

- `on:` unchanged (tags `v*`, manual dispatch with `tag`).
- Job `verify` (ubuntu-latest): the same steps as `ci.yml` (install apt packages, fmt check, clippy, engine tests, pdf-export tests, desktop headless tests). If `ci.yml` supports `workflow_call`, reuse it with `uses: ./.github/workflows/ci.yml`; otherwise add `on: workflow_call:` to `ci.yml` and call it.
- Job `version-check` (or a step in `verify`): fail if the tag (without the leading `v` and any `-rc.N` suffix) does not match `[workspace.package] version` in `Cargo.toml`.
- Job `windows` (`needs: verify`): the existing steps, plus: `git config --global core.symlinks true` is no longer needed after Step 2; bundle only `Sarv.exe` + DLLs + `LICENSE` (leave `sarv-agent.exe` out per "Current state" unless the owner says otherwise — keep the build of it so it stays compiling); produce `Sarv-${TAG}-windows-x64.zip`.
- Job `linux` (`needs: verify`, ubuntu-22.04 for an older glibc baseline): install build deps; `cargo build --release -p sarv-desktop`; run `scripts/package-linux.sh "$TAG"` (next step) to produce `Sarv-${TAG}-linux-x64.tar.gz`.
- Job `publish` (`needs: [windows, linux]`): download both artifacts (use `actions/upload-artifact@v4` / `download-artifact@v4` between jobs), generate `SHA256SUMS.txt` with `sha256sum`, then the existing `gh release create … --prerelease` / `upload --clobber` logic with all three files. Only this job has `permissions: contents: write`; the others `contents: read`.
- Pin the Rust toolchain from `rust-toolchain.toml` (plan 001) instead of floating `stable`: for the Windows GNU job install `<channel>-x86_64-pc-windows-gnu` reading the channel from the file.

**Verify**: the YAML syntax command above → exit 0. State in your report that the workflow has not run on GitHub.

### Step 4: Linux bundle and installer

`scripts/package-linux.sh <tag>`: creates `Sarv-<tag>-linux-x64/` containing `sarv-desktop` (from `target/release`), `sarv.png`, `ir.sarv.accounting.desktop.in` (template), `install.sh`, `uninstall.sh`, `LICENSE`, and a short `README.txt` listing the runtime packages needed (cairo, pango, fontconfig — derive the list from `ldd target/release/sarv-desktop | grep -E "cairo|pango|fontconfig|freetype|harfbuzz"`); then tars it. `set -euo pipefail`; fail if the binary is missing.

Rework `scripts/install-desktop.sh` so the same logic serves both the repo and the bundle:

- Accept `--uninstall`: remove `$app_dir/sarv-desktop`, `$app_dir/sarv-agent`, the `.desktop` file and the icon; run `update-desktop-database` if present; **never touch the books** (`ir.sarv.accounting/` data directory) — print where they are and that they were kept.
- Add to the desktop entry: `TryExec=$app_dir/sarv-desktop`, `StartupWMClass=` set to the window class the app reports (find it: run the app and `xprop WM_CLASS` on X11, or check Rune's `AppSpec` for an app id; if unknown, omit and note it), `Name[fa]=سرو` (present), `Comment[fa]=حسابداری محلی`, `Keywords=accounting;invoice;حسابداری;`.
- Install the icon into the size directory that matches the PNG's real size (read it with `file`), not unconditionally `128x128`.
- Install `sarv-agent` only if it exists (it will not be in the bundle).

**Verify**: in a temp HOME: `HOME=$(mktemp -d) XDG_DATA_HOME= bash scripts/install-desktop.sh` after a release build → files appear; `… --uninstall` → they are gone; `shellcheck` clean if available.

### Step 5: Mirror the Rune dependency (documentation + config, no pin change)

Write in `docs/RELEASING.md` a section "Build dependencies we do not control": the Rune repository URL and pinned rev; the instruction for the owner to create a mirror (fork or a bare mirror under the project's GitHub account) and keep the pinned commit reachable there; and a ready-to-use, **commented-out** `.cargo/config.toml` snippet using `[patch."https://github.com/anomalyco/rune"]` pointing at the mirror URL with the same `rev`. Do not create the fork yourself and do not add an active patch.

### Step 6: `docs/RELEASING.md` and README

`docs/RELEASING.md`: the release checklist — CI green on the commit; bump `[workspace.package] version`; update `CHANGELOG.md`; tag `vX.Y.Z` (or `-rc.N`); the workflow builds, checks the version, and publishes a **pre-release**; smoke test on Windows and Linux (open existing books, post a sale, export a PDF, check Settings → About version); then promote the release by hand. Also: the known gaps — unsigned Windows binary (SmartScreen warning) and what is needed to sign (an OV/EV code-signing certificate or Azure Trusted Signing; a secret in the repo settings; a `signtool` step); no macOS bundle; no auto-update.

`README.md` Install section: add "Download" instructions for the Windows zip and the Linux tarball, how to verify with `SHA256SUMS.txt`, and keep the build-from-source path.

## Done criteria

- [ ] `test ! -e crates/sarv-quote/assets` (symlink removed) and `cargo test -p sarv-quote --features pdf-export` passes
- [ ] `head -1 crates/sarv-desktop/src/main.rs` shows the `windows_subsystem` attribute
- [ ] `.github/workflows/release.yml` parses; contains jobs `verify`, `windows`, `linux`, `publish`; `grep -n "sha256sum" .github/workflows/release.yml` matches
- [ ] `bash scripts/install-desktop.sh --uninstall` works in a temp HOME
- [ ] `git diff a4776749 -- crates/sarv-desktop/Cargo.toml | grep '^[-+].*rev = '` prints nothing
- [ ] `docs/RELEASING.md` exists; `plans/README.md` updated

## STOP conditions

- `winresource` (or any resource crate) cannot build with the MSYS2 GNU toolchain without changing the toolchain — skip the icon, keep the subsystem attribute, report.
- Removing the symlink breaks a test or script that reads `crates/sarv-quote/assets/...` at run time (not compile time) — report the path.
- You would need a secret, a certificate, or to run `gh release …` / push a tag to proceed — stop; those are owner actions.

## Maintenance notes

- **Owner actions after merge** (cannot be done by an executor): run the workflow once via manual dispatch on a test tag and fix what the first real run reveals; decide what to do with the public `v0.1.0` "Latest" release (it predates the Rune app) and the `v0.2.0-rc.1` assets built from another branch; create the Rune mirror; obtain a code-signing certificate.
- The `ldd`-based DLL collection on Windows is fragile when MSYS2 packages change; the smoke test in the checklist is what catches a missing DLL.
