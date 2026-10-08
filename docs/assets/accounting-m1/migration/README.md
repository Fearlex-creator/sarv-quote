# Synthetic recovery rehearsal

All included customer/contact/quote data are synthetic native-test fixtures. No regular profile was read. The compatible headless verifier was assembled from `git show 4aa82c2:<file>` for Cargo.toml, Cargo.lock and src/lib.rs/domain.rs/imports.rs/store.rs/export.rs; only its package name was changed to `sarv-quote-legacy-verifier` in manifest/lockfile. Its driver is `legacy-verifier-main.rs`. No old binary writes an upgraded profile.

The new-library driver is `probe/`, depending on this repository. For a fresh directory, copy `legacy-v1.json` there, then activate the executor and run:

```sh
CARGO_TARGET_DIR=/workspace/sarv-quote/target cargo run --release --offline --locked --manifest-path docs/assets/accounting-m1/migration/probe/Cargo.toml -- /tmp/sarv-migration-rehearsal-fresh
```

This tests exact quote snapshots, verified pre-upgrade recovery, stable generation, fresh full-state v2 restore, quote PDF generation and integrity. Reconstruct the old-source driver above and run it with `verified-before-schema2-v1.json` and a different fresh destination to repeat compatible schema1 restore. PDF text equality is verified using `pdftotext` on legacy/restored and current/upgraded outputs. `migration-verification.json` is the observed original rehearsal. Process termination during schema upgrade and physical power loss were not tested.
