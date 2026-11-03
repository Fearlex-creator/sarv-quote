# 0007 — Save cost grows with total data; incremental validation recommended

**Status:** proposed (not applied) · **Date:** 2026-11-03

## Measured
`cargo run --release -p sarv-quote --example write_latency -- N` saves one tiny edit through
`Store::change`: 3.8 ms at 1k products, 50 ms at 10k, ~220 ms at 50k. `write_breakdown` shows
`State::validate()` dominating (~300 ms at 50k) versus clone 25 ms, serialise 21 ms, parse 37 ms.
`validate()` re-fingerprints every catalog on each write. A byte-identical re-parse was removed
(~9 %); it was not the bottleneck.

## Recommendation
Validate only what a change touched (compare against the already-validated base state) and keep full
validation for open, restore and import. Same storage format; existing data stays valid.
A normalised-table schema would scale further but needs a migration and rehearsal (`decisions.md`
DATA-CAP), so it is a separate decision.

## Why it is not applied
"Validate everything on every write" looks deliberate (storage hardening tests). Weakening it changes
an integrity guarantee, so it needs the owner's approval.
