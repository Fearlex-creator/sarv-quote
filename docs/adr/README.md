# Architecture decision records

Decisions taken while adopting `docs/ACCOUNTING-SPEC.md` and `docs/FRONTEND-UI-GUIDE.md`.
The older register in `decisions.md` stays the record for earlier product decisions.

| ADR | Decision |
| --- | --- |
| [0001](0001-keep-sarv-identity.md) | Keep the Sarv name and visual identity; the spec's "Daftar" is a working name |
| [0002](0002-adopt-spec-incrementally.md) | Adopt the spec on the existing engine; do not rewrite to axum/sqlx/Postgres now |
| [0003](0003-advisory-identifier-validation.md) | Iranian identifier checks are advisory; stored contact data stays free text |
| [0004](0004-law-as-data-with-verify-flags.md) | Law values are effective-dated, flagged `verify`, and not wired into any calculation yet |
| [0005](0005-cheque-lifecycle-and-ledger-integration.md) | Cheque lifecycle is a pure state machine; ledger integration needs a design pass first |
| [0006](0006-licence-unchanged.md) | Licence stays MIT; the spec's AGPL-3.0 suggestion is the owner's decision |
| [0007](0007-save-cost-and-incremental-validation.md) | Saves cost O(total data); incremental validation is the recommended fix, pending approval |
