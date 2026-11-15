# Releasing Sarv

This document covers the release process, how to mirror build dependencies we do not control, and known packaging gaps.

## Release checklist

1. **Ensure CI is green on the release commit**
   Confirm `.github/workflows/ci.yml` passes on the commit you plan to tag.
2. **Bump the workspace version**
   Update `[workspace.package] version` in [Cargo.toml](../Cargo.toml). The release workflow's `version-check` job fails if the tag (minus the leading `v` and any `-rc.N` suffix) does not match this version.
3. **Update `CHANGELOG.md`**
   Summarize user-facing changes, migrations, and known issues for the release.
4. **Tag and push**
   Tag the commit as `vX.Y.Z` (or `vX.Y.Z-rc.N` for a release candidate) and push the tag. You can also trigger `.github/workflows/release.yml` manually via `workflow_dispatch` with the `tag` input.
5. **Wait for the release workflow**
   [release.yml](../.github/workflows/release.yml) runs `verify` (CI), `version-check`, `windows` (MSYS2 MINGW64 portable zip), `linux` (`ubuntu-22.04` tarball), and `publish` (generates `SHA256SUMS.txt` and creates/updates a GitHub **pre-release**).
6. **Smoke test on Windows and Linux**
   Download `Sarv-vX.Y.Z-windows-x64.zip`, `Sarv-vX.Y.Z-linux-x64.tar.gz`, and `SHA256SUMS.txt`, verify checksums, and test on both platforms:
   - Open existing books (and test first-run setup on a fresh profile).
   - Post a sale.
   - Export a printable PDF and check Persian font rendering.
   - Check **Settings → About** for the expected version and commit SHA.
7. **Promote the release**
   Once smoke tests pass, edit the GitHub release by hand to clear the **pre-release** flag (and mark as Latest if appropriate).

## Known gaps

- **Unsigned Windows binary**: `Sarv.exe` is not Authenticode-signed, so Windows SmartScreen will show an "Unknown publisher" warning on first launch. To sign releases, obtain an OV/EV code-signing certificate (or configure Azure Trusted Signing), store the signing credentials as repository secrets, and add a `signtool` step to the `windows` job in [release.yml](../.github/workflows/release.yml) before zipping.
- **No macOS bundle**: macOS users must build from source with `cargo build --release -p sarv-desktop`.
- **No auto-update**: Users download and install new releases manually.

## Build dependencies we do not control

`sarv-desktop` depends on `rune` and `rune-kit` from `https://github.com/anomalyco/rune` pinned to `rev = "b556a38a1ff88aa8690d21b45ebf9b9167038674"` in [crates/sarv-desktop/Cargo.toml](../crates/sarv-desktop/Cargo.toml).

If that upstream repository becomes unavailable or force-pushes over history, builds will fail unless a mirror keeps the pinned commit reachable:

1. Create a fork or bare mirror of `https://github.com/anomalyco/rune` under the project's GitHub account (for example `https://github.com/<org>/rune`).
2. Ensure commit `b556a38a1ff88aa8690d21b45ebf9b9167038674` is pushed and reachable in that mirror.
3. Point Cargo at the mirror without changing `crates/sarv-desktop/Cargo.toml` by adding the following to `.cargo/config.toml` (uncomment and replace `<org>`):

```toml
# [patch."https://github.com/anomalyco/rune"]
# rune = { git = "https://github.com/<org>/rune", rev = "b556a38a1ff88aa8690d21b45ebf9b9167038674" }
# rune-kit = { git = "https://github.com/<org>/rune", rev = "b556a38a1ff88aa8690d21b45ebf9b9167038674" }
```
