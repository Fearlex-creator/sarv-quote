# 0006 — Licence: AGPL-3.0-only for the core

**Status:** accepted (supersedes the earlier "unchanged" decision) · **Date:** 2026-11-03

The owner decided (2026-11-03) to license the core under AGPL-3.0, with Apache-2.0 for any SDK or
plugin API, as the spec recommends. `decisions.md` DEC-03 had required that decision to be explicit.

- `LICENSE` is the SPDX `AGPL-3.0-only` text; `Cargo.toml` (workspace and `desktop/src-tauri`) say
  `AGPL-3.0-only`. "Only" rather than "or later" was chosen because it can be relaxed later but not
  tightened; change it if you prefer "or later".
- No SDK or plugin API exists yet, so no Apache-2.0 files were created.
- Bundled Vazirmatn stays under SIL OFL 1.1. Earlier published snapshots remain MIT for those who
  received them; a dependency/SBOM licence-compatibility review is still recommended before release.
