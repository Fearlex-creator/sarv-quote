# Exact quantity readiness correction

Root authorized this bounded compiled-observer correction after [candidate5's preserved failure](../scale-pilot-candidate5-failed/README.md). The [one-line source diff](quantity-readiness.patch) replaces literal ASCII quantity comparison with production's exact `quantity` parser and equality to1000 thousandths. Valid ASCII/Persian representations of1 mean the same value; invalid or different quantities still fail readiness. All other durable/render/visible/focus predicates,20ms independent clock, terminal-gap accounting, broad sampler/reset/report fields and50ms budget stay unchanged.

[Provenance](provenance.json) records before/after owned source SHA256s and successful `cargo check --offline --all-targets`. Full [candidate5 source](candidate5-responsiveness.rs) and [candidate6 corrected source](candidate6-responsiveness.rs) preserve the observer versions. Corrected `src/ui/responsiveness.rs` SHA256 is `a25789ec22f1aacc316a0a499dfa7cede1d3041b85a967595444e1b08e7d65e1`.

Candidate6 executable/full source freeze and functional qualification are pending. No new pilot or final measurement has run on this correction. Original [C2 strict misses](../final-candidate2/README.md) and candidate5 failed run remain unchanged.
