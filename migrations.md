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
