# 0003 — Iranian identifier checks are advisory

**Status:** accepted · **Date:** 2026-11-03

## Context
Spec §3.8 requires validating national ID, legal ID, postal code and Sheba. In Sarv, `national_id`
is a free-text "National / registration ID" and existing tests save `"ID-1"`. Foreign customers and
suppliers have no Iranian identifiers.

## Decision
`crates/sarv-quote/src/iran/{ids,sheba}.rs` and `desktop/src/lib/iran-ids.ts` implement the checks
(shared fixtures: `crates/sarv-quote/tests/fixtures/iran_ids.json`, asserted by both Rust and Node
tests). The UI shows a quiet, non-blocking note when a value does not look like an Iranian ID.
The engine does **not** reject saves on checksum failure.

## Consequences
Hard enforcement (for a Moadian-registered buyer, where the ID is mandatory) belongs to the
e-invoice flow, where the party type is known. Postal code and Sayad ID are shape-only because no
public checksum exists.
