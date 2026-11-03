# 0002 — Adopt the spec incrementally on the existing engine

**Status:** accepted for now · **Date:** 2026-11-03 · **Revisit:** when Team mode (spec §4.5) is started

## Context
The spec's fixed stack is a 26-crate workspace on axum + sqlx + Postgres with SQLite for Solo mode,
sync, WASM plugins and RLS. Sarv is a working app: one `sarv-quote` engine crate (~58k lines with
tests) on rusqlite, a Tauri shell and a React UI, plus an audited, replayable ledger whose financial
guards are covered by hardening tests (`storage_hardening`, `storage_financial_guards`,
`accounting_kernel`, `process_recovery`).

## Decision
Do **not** rewrite to the spec's stack. Take the spec's *domain* requirements and add them as new,
tested modules on the current engine, in dependency order, starting with pure rules that need no
schema change (`iran::*`). Postgres/axum/sync arrive with Team mode, designed then, with the
existing store as the Solo adapter.

## Consequences
- The spec's crate layout, `xtask check-deps`, sqlx/sea-query and specta items are **deferred, not
  rejected**; see `docs/SPEC-COVERAGE.md`.
- Existing user data stays valid. Any schema bump goes through `Store::open`'s recovery-backup path.
- The spec's Claude Code prompt ("start with M0 foundation") would discard working code; it is not
  followed literally.
