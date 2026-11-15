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

## P02-T01 — Deterministic test clock
- Updated `today()` in `crates/sarv-desktop/src/data.rs` to honour `SARV_TODAY` when valid `YYYY-MM-DD`.
- Pinned `SARV_TODAY=2026-10-08` via `std::sync::Once` in `Engine::demo()`.
- Added `test_demo_clock_is_inside_the_fixture_year` in `crates/sarv-desktop/src/main.rs` and marked Plan 002 `DONE`.
- Verified: `SARV_TODAY=2026-10-08 cargo test -p sarv-desktop --bin sarv-desktop test_demo_clock` (1 passed); `cargo test -p sarv-desktop --bin sarv-desktop` (35 passed); fmt + clippy clean.

## P03-T01 — Long-lived Store in desktop
- `Engine` in `crates/sarv-desktop/src/data.rs` now holds `Mutex<Option<Store>>`, routes `call` through `refresh_if_stale` + `dispatch_in`, and exposes `with_state` for `ui/documents.rs` (eliminating `Store::open` in `documents.rs`).
- `Store::refresh_if_stale` in `crates/sarv-quote/src/store.rs` reads `(generation, payload)` in a single query when stale.
- Added `test_engine_sees_writes_from_another_store` and `test_integrity_status_survives_between_calls` in `main.rs`; marked Plan 003 `DONE`.
- Verified: `cargo test -p sarv-desktop --bin sarv-desktop` (37 passed), `cargo test -p sarv-quote --test storage_hardening --test process_recovery --test agent_access` (32 passed).

