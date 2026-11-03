# 0007 — Incremental validation on save, full validation at safety points

**Status:** accepted and applied (owner approval 2026-11-03) · **Date:** 2026-11-03

## Measured problem
`cargo run --release -p sarv-quote --example write_latency -- N` saves one tiny edit through
`Store::change`. Before: 3.8 ms at 1k products, 50 ms at 10k, ~220 ms at 50k. `write_breakdown` showed
`State::validate()` dominating (it re-fingerprints every catalog on every write).

## Decision
- `State::validate()` is unchanged in meaning (full check) but is split into three pure parts:
  the **accounting group**, the **approved quotation data** (catalogs, customers, quotes) and the **draft**.
- `State::validate_changes(&base)` runs only the parts whose inputs differ from an already fully
  validated `base`. The exact input sets are documented on the function; the accounting group also reads
  `quotes` (sales orders link to approved quotes) and, below schema 3, the draft.
- `Store::change` uses `validate_changes` against `self.state`, which was fully validated when opened,
  restored or last committed.

## Safety net (the fast path trusts its base, so these never do)
| Point | Behaviour |
| --- | --- |
| Open, restore, restore preview | full `validate()` (unchanged) |
| Backup / export (`Store::backup`) | full `validate()` first; no backup is written from a book that fails it |
| Period close (`period_lock` with `locked: true`) | full `validate()` inside the same transaction; failure blocks the close and shows the reason |
| After an import (opening-balance import, zero opening) | `Store::start_background_check()` validates a copy in the background; status is `unchecked / running / ok / failed` |
| `failed` status | shown on the Overview (`IntegrityNotice`), returned by the `integrity` action and in `workspace`, and **blocks period close** with the cause |

## Proof
`tests/incremental_validation.rs`: every one of 12 mutations (4 valid, 8 invalid across all three
groups) and 300 seeded random combinations get the **same verdict and the same error message** on both
paths; an unchanged state is accepted; hidden corruption inside the base passes a fast save but is
refused by the full validator, by backup and by period close; the background check reports ok/failed and
a failed status blocks close with its reason. The agreement tests were mutation-checked: disabling a
group in the fast path makes them fail.

## Result
Save of a tiny edit at 50k products: ~220 ms → ~110 ms; 10k: 50 → 20 ms; 1k: 3.8 → 1.9 ms (measured
with other load on the machine). The remaining cost is clone + two serialisations + fsync, which only a
storage redesign (normalised tables or delta log) removes; that stays a separate decision
(`decisions.md` DATA-CAP).

## Known limit
`validate_changes` still re-runs the whole accounting group whenever anything in it changes, so an
accounting-heavy book (many journals) pays O(journals) per accounting write. Validating only appended
audit events needs replay-state caching in `accounting.rs`; not attempted without its own design pass.
