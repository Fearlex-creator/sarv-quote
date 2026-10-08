# Current schema 5 procurement checkpoint

M4 adds an optional immutable procurement aggregate: resource additions, purchase
orders, partial receipts, matched supplier bills, paired warehouse movements,
frozen policy/master/price/cost snapshots and retry aliases. Existing quotations,
manual books and M3 openings are preserved; no operational documents or resources
are inferred from their balances. Current writes require schema5. Schemas1–4
remain readable only after validation of their original schema and checksum.
An older schema claiming procurement records is rejected.

Legacy manual journals may contain event names that M4 now reserves for typed
procurement. They and their exact request/audit history remain readable and
restorable. Protected reversal and native source navigation use the actual linked
core/procurement record, rather than historical event text. Ordinary reversals of
these legacy manual journals remain available subject to fiscal locks and normal
account validation. New manual reserved events, including forged new persistence
appends, are rejected. Root regression tests and independent fixtures created by
the actual archived M3 implementation verify both sides of this boundary.

Before upgrading an existing profile, Store publishes and verifies a synced,
uniquely named `before-schema5-<uuid>.json` recovery matching the exact preceding
logical state. A schema4 recovery keeps `sarv-accounting-backup-v4`; current
backups use `sarv-accounting-backup-v5`. The conditional SQLite UPDATE changes
schema and generation together, and subsequent opens do not migrate again.
The database remains the existing generation-checked SQLite aggregate, WAL and
`synchronous=FULL`; this checkpoint does not introduce a normalized ledger store.

The final frozen migration tests retain both historical quotations/manual journals and the detailed M3 opening. They verify generation41→42, exact full-state preservation except schema, one original verified v4 recovery, fresh v5 restore and restart. Competing receipt candidates have one durable winner; a different-key retry resolves the same effect, and mutated posted cost is rejected without changing the durable generation. The ordinary final repository run passes five procurement migration cases and two Store cases. The separately run actual M3 executable rejects schema5 without changing payload/generation; an independent archived M3 implementation verifies and restores three original pre-M4 v4 recovery profiles.

The final independent suite also exercises actual process contention and SIGKILL before commit and after durable commit/lost adoption, requiring exact old-or-complete state and SQLite integrity `ok`. Current [schema4 recovery evidence](docs/assets/accounting-m4/migration/schema4), [procurement restore](docs/assets/accounting-m4/restore/po-receipt/verification.json), preserved older-schema restores and [M4 delivery](requirements/milestone-M4.md) retain exact boundaries. These are software interruption/recovery results; no physical power-loss or disk-full claim follows.

Automatic downgrade is unsupported. Retain the full upgraded profile and v5
backups. To recover the state before M4, restore the verified v4 recovery into a
separate fresh profile with the compatible M3 build; that old state excludes all
new procurement events. Final software interruption checks retain the exact
old row when an owned child is killed while startup waits for a writer, before
the schema UPDATE; the next open upgrades once. This does not establish physical
power-loss safety or interruption inside that UPDATE.

## Historical schema 4 core-foundation checkpoint

M3 adds typed, company-specific policy/account-role snapshots and an immutable, reconciled fresh-company opening with its party, unit, item, warehouse, cashbox, bank, open-invoice/bill, advance, outstanding-cheque, stock and GRNI detail. Empty core state is omitted; existing manual books receive no policies, chart, registry or balances automatically. Current writes require schema4. Schema1/2/3 remain readable and are upgraded only after a synced, uniquely named `before-schema4-<uuid>.json` recovery has been checksum/domain-validated and matched to the exact preceding logical snapshot. Schema3 recovery retains `sarv-accounting-backup-v3`; new backups use v4. The conditional SQLite UPDATE changes schema and generation together; repeated opens do not migrate again.

Schema3 legitimately retains Jalali settings and document snapshots. Schema1/2 claiming Jalali, or any schema below4 claiming a core policy/opening, is rejected. Unknown schemas and mismatched envelope/state versions are rejected. Compatible old writers must reject upgraded state rather than discard new data. Preserve the upgraded profile and v4 backups; recovery into a separate fresh profile using the compatible old version only recovers the state before M3. There is no automatic downgrade or conversion of existing manual balances into subledgers.

The current migration checks retain three historical approved quotations, three manual journals and a schema3 Jalali preference byte-for-byte except the schema field; generation37 becomes38 once. They verify the exact v3 recovery, fresh v4 restore and restart. A process-kill check holds a SQLite write lock and interrupts Store startup before its initial write/migration; the old row is unchanged, and the next open completes with verified recovery and integrity `ok`. This establishes that precise software boundary, not interruption inside the schema UPDATE or physical power-loss safety. The separate opening review probes test financial commit interruption and competing requests. Final evidence and remaining limitations are tracked in [M3 delivery](requirements/milestone-M3.md).

## Historical schema3 Jalali/native checkpoint

Schema 3 explicitly versions the serialized Jalali presentation preference introduced in M2. Schema 1 and M1 schema 2 profiles upgrade on open only after a uniquely named `before-schema3-<uuid>.json` recovery is atomically published, synced, checksum/domain-validated, and matched byte-for-byte to the old logical state. Schema1 recovery retains v1 format; schema2 recovery retains `sarv-accounting-backup-v2`. The conditional row UPDATE changes schema and generation together; repeated open is idempotent. No quote, posting, date or historical snapshot is reinterpreted. New backups use `sarv-accounting-backup-v3`; readers accept and verify v1/v2 originals before upgrading the in-memory restore candidate to3. Ordinary writes require current schema3. A schema1/2 payload claiming Jalali is invalid.

M2→M1 downgrade is unsupported: M1 rejects schema3/v3 backups and cannot represent a saved Jalali preference or document snapshot. Merely switching current settings to ISO cannot remove historical Jalali snapshots safely. Keep the full M2 profile and v3 backups; restore the verified pre-M2 v2 recovery into a **different fresh profile** using compatible M1 when needed. Its state predates new M2 entries/preferences. Never run an old writer against upgraded data or change issued snapshots to satisfy it. This is a recovery path, not an automatic reverse migration.

Current migration test retains3 approved quotes and3 manual journals through2→3, exact quote/accounting hashes, generation17→18, repeat-open18 and fresh-profile full-state equality. Interrupted schema upgrades and physical power loss remain untested. [M2 delivery](requirements/milestone-M2.md) records final proof and limits.

The sections below preserve the historical M1 schema1→2 implementation and evidence.

# Storage and recovery checkpoint

Baseline: `4aa82c2988fb2ea48a2fa2ae9824c06187f3d76a`. One SQLite `state` row,
optimistic generation, WAL, `synchronous=FULL`, JSON schema 1. No normalized
ledger tables or historical migration files existed. The accounting slice retains
that transactional store; it does not rebuild quotations or import real data.
Mutations reserve SQLite writes with `BEGIN IMMEDIATE` before reading the committed
snapshot, so a worker waits safely for an existing writer instead of attempting a
deferred read-to-write upgrade. Busy timeout is three seconds; a timeout retains
uncommitted input and reports failure.

## Schema 1 → 2

Schema 2 adds an optional accounting aggregate and retains quotation, customer,
catalog, settings, IDs, dates and calculation snapshots. Empty accounting is
omitted from serialization. Existing quote calculations retain their own policy.
Before upgrading an existing schema-1 row, opening the profile writes a uniquely
named `before-schema2-<uuid>.json` v1 recovery snapshot through synced, atomic,
no-overwrite publication. It verifies the checksum, domain validation and exact
serialized logical state before changing the database. An atomic conditional
UPDATE upgrades schema and generation together. A competing writer makes the
upgrade fail; reopening an already upgraded profile does not migrate again.

An unreadable, unsupported or invalid legacy payload is rejected without replacing
its row. A backup/write/verification failure stops the upgrade. Leftover recovery
files are usable snapshots, not evidence that a migration committed. SQLite
integrity checks and process-kill tests establish software recovery only, not
physical power-loss guarantees.

## Backup and replacement

New backups use `sarv-accounting-backup-v2`, containing the complete quote and
accounting state, policies represented by its model, exact journal/request/audit
history and SHA-256 corruption checksum. The size limit remains 64 MiB. This is
not encryption or an authenticated signature. There are no attachments in this
slice and no scheduled backup or verified-restore status UI yet.

Legacy `sarv-quote-backup-v1` with schema 1 remains readable: verify its original
checksum and full state before upgrading the in-memory candidate. Unknown
formats/schemas and inconsistent accounting histories are rejected. Restore into
a fresh profile is tested. Explicit replacement of an existing profile requires
a verified recovery snapshot matching the exact current state; generic persistence
commands cannot rewrite or remove posted history. A replaced profile's previous
state remains available in its safety backup.

## Downgrade and demonstration

Automatic downgrade is unsupported. Old binaries reject schema 2; do not run an
old writer against upgraded data. The pre-upgrade v1 recovery file can be restored
into a fresh profile using the compatible `4aa82c2` quotation application. It
contains only data existing before accounting was added; new journals cannot be
represented by that old application. Retain the upgraded profile and v2 backups.

Use a fresh synthetic `SARV_DATA_DIR` for checks. Activate the cloud tools with
`source /workspace/sarv-environment/activate.sh`. Repeatable commands:

```sh
cargo test --offline --locked --test accounting_migration --test detached_save
cargo test --offline --locked --test accounting_kernel --test process_recovery
SARV_ACCOUNTING_SMOKE=full SARV_DATA_DIR=/tmp/sarv-accounting-demo \
  /workspace/sarv-environment/run-desktop.sh
```

The native smoke profile must be a fresh directory. It generates synthetic opening
and adjustment journals, demonstrates reversal/lock/reopen, exports exact trial
balances and verifies a full-state restore to a fresh profile. Final command
outcomes, source/binary hashes and limitations are linked by the milestone report.
An accountant must review the user-defined chart and expected postings before
production use; no tax, inventory, settlement or closing policy is selected here.

## Retained rehearsal evidence

The final checkpoint rehearsed a three-approved-quote schema1 profile, verified v1 recovery, generation42→43 migration, repeat-open idempotence, full-state fresh v2 restore and integrity `ok`. Compatible `4aa82c2` library restored the actual pre-upgrade recovery into a fresh schema1 profile; quote serialization hashes and extracted old/new quote PDF text matched. [Raw results](docs/assets/accounting-m1/migration/migration-verification.json), [checkpoint](requirements/milestone-M1.md). Interrupted schema upgrades remain untested; journal process-kill tests do not substitute for that check.
