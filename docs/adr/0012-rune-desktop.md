# 0012 — Native Rune desktop shell

**Status:** accepted · **Date:** 2026-11-13

## Context
Sarv has gone through three desktop shells:
1. A native Rust + GTK4 + libadwaita application (`v1`, quotation and early accounting milestones).
2. A Tauri v2 + React/TypeScript web-view shell (`desktop/`).
3. The current native Rust desktop (`crates/sarv-desktop`) built on Rune and `rune-kit`.

Maintaining a separate web/TypeScript frontend and IPC bridge duplicated validation, formatting, and build tooling without improving desktop reliability or RTL/Persian rendering.

## Decision
- The desktop application is `crates/sarv-desktop`, built on Rune and `rune-kit` at a pinned revision.
- `crates/sarv-desktop` calls `sarv_quote::desktop::dispatch` in-process. There is no React, TypeScript, Tauri, or GTK shell.
- The UI does not invent ledger rules, journal lines, tax, or document IDs; all business logic and persistence remain in `crates/sarv-quote`.

## Consequences
- `docs/ACCOUNTING-SPEC.md` §4 (axum/sqlx/Postgres/Tauri/React stack) and §8 (build prompt) do not apply to the desktop shell; `docs/FRONTEND-UI-GUIDE.md` and `docs/DESIGN.md` document the Rune UI.
- Headless verification uses `cargo test -p sarv-desktop --bin sarv-desktop` and `cargo test -p sarv-desktop --test png_scenes` (GPU required for PNG snapshots) in place of Playwright browser tests.
- Features from the former React/Tauri shell that were not yet present after the Rune migration are tracked in `plans/`.
