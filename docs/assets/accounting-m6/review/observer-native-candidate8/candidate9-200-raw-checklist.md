# Independent C9 200-line raw post/restart checklist

Prepared before the new C9 pilot from immutable `return200-billed` recipe and literal expectations in `requirements/checkpoint-M6-followup.md`. **Current C9 post/restart UNTESTED.** No GUI, database mutation or production calculator is used. Old receipts remain unchanged.

`check-candidate9-200-raw.py` accepts raw JSON States or stopped SQLite profiles opened with `mode=ro` and an independent integrity check. It requires before, after and restart artifacts together and confines receipts to this review directory. The default invocation generates only `candidate9-200-raw-plan.json`, listing all200 ordered original receipt/bill IDs and explicit literals. As a bounded checker-shape sanity check, historical C6 before/after/stopped JSON passed872 raw assertions; no new acceptance receipt was emitted and this proves no current C9 behavior.

Required independent checks:

- Before State equals the entire frozen prepared dataset, schema7. Catalogs, customers, quotations, draft, settings, core, sales, company/period/accounts and every original journal/request/audit/procurement prefix remain exact after posting.
- Exactly one new typed Return record and one linked `procurement.return.v1` journal. Distinct nonempty Return/Credit IDs with `Returned`/`Posted`, original supplier/bill source, original frozen setup and explicit date/reference/context.
- All200 original receipt and bill IDs in order;200 unique new line IDs; each raw and submitted quantity1000/value15, item ITEM-A/warehouseW1. Every linked outward movement has Q1000/V15 and unique ID. Literal transitions: W1 Q225000/V3292 → Q224000/V3277, continuing by Q1000/V15 to Q25000/V292.
- Sole journal is DrAP3000/CrInventory3000. All journals balance exactly. Independent raw-journal totals before/after: Inventory3422→422, AP credit3329→329, GRNI credit50, AR299, sales199, returns100, COGS58, earnings41 unchanged, TB side4630→1630. All unrelated mapped controls remain exact. W2 is independently derived from original base-piece opening and raw movement deltas as Q12000/V130 and receives no new movement.
- New journal/procurement request aliases are permitted only when linked to the same new command/effect/record; new audit is permitted only for linked JournalPosted entries. Existing aliases/audit entries remain byte-for-byte equivalent as JSON values.
- Entire supplied after State equals the independently supplied stopped/restarted State, including quotation data and all new IDs. SQLite artifacts additionally require integrity `ok`.

Example future read-only invocation:

```sh
python3 docs/assets/accounting-m6/review/observer-native-candidate8/check-candidate9-200-raw.py \
  --before ACTUAL_BEFORE_STATE.json --after ACTUAL_AFTER_STATE.json \
  --restart ACTUAL_RESTART_STATE.json \
  --output docs/assets/accounting-m6/review/observer-native-candidate8/candidate9-ACTUAL-EPOCH-raw-result.json
```

The checker must be paired with independent frozen observer/command/provenance evidence proving actual C9 native posting and a real process restart. Supplied equal JSON alone cannot prove restart orchestration. Settled images/geometry establish visible-field UX separately; raw focus or screenshots cannot substitute for financial correctness. This preparation qualifies no UI, performance, posting or restart result.
