# Goal Progress Log

Started: 2026-11-15
Baseline: `cargo test -p sarv-desktop --bin sarv-desktop` (34 passed), `cargo test -p sarv-quote` (all passed).

---
## P01-T01 — Pin toolchain, format workspace, dev line tables
- Added `rust-toolchain.toml` (`channel = "1.98.1"`, components `rustfmt`, `clippy`), `rustfmt.toml` (`max_width = 140`), `rust-version = "1.85"` in workspace & crate manifests, and `[profile.dev] debug = "line-tables-only"`.
- Formatted workspace with `cargo fmt --all`.
- Verified: `cargo fmt --all -- --check` exited 0; `cargo test -p sarv-desktop --bin sarv-desktop` -> 34 passed, 0 failed.

## P01-T02 — Fix clippy lints, add CI workflow, document Poppler
- Fixed all clippy errors/warnings across `sarv-quote` and `sarv-desktop` (`domain.rs` MSRV modulo, `sheets.rs` `allow(clippy::result_unit_err)`, `views.rs` `sort_by_key`, `desktop_bridge.rs` needless borrow, `desktop_bridge` test `unused_mut`, `glass.rs`/`pickers.rs`/`resources.rs`/`overview.rs` `allow(clippy::too_many_arguments)`, `shell.rs` `PaletteAction` type alias, `payments.rs` duplicate branch, `pos.rs` `#[derive(Default)]`, `sheets.rs` elided lifetime, needless borrows in `inventory.rs`/`purchases.rs`/`sales.rs`).
- Created `.github/workflows/ci.yml` with fmt, clippy, `sarv-quote`, `sarv-quote --features pdf-export`, and `sarv-desktop` headless tests.
- Documented `poppler-utils` in `README.md` and marked Plan 001 `DONE` in `plans/README.md`.
- Verified: `cargo fmt --all -- --check` (exit 0), `cargo clippy --workspace --all-targets -- -D warnings` (exit 0), `cargo test -p sarv-desktop --bin sarv-desktop` (34 passed).

