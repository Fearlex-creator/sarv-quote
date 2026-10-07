# افتتاحیهٔ CSV نسخهٔ ۱ / Opening CSV v1

This format stages a **fresh-company opening at the beginning of its fiscal start date**. It does not post the file, replace an existing opening, clear cheques, recreate historical revenue or infer an equity plug. Review the independent ledger balances and all control differences before the separate atomic posting action. Engineering policy defaults are `core-policy-v1`; legal/tax acceptance remains deferred.

The synthetic [sample](assets/accounting-m3/opening.csv) uses company ID `F2-core-v1`. For an actual synthetic company created in the app, explicitly replace that ID in every row with the displayed company ID. The importer rejects any row naming another company. Role keys select the company's existing validated role mapping; the file cannot define or guess account mappings. The company's configured period/setup IDs are selected from that company, and its fiscal start must equal the opening cutoff.

UTF-8, optional BOM, comma separator, ordinary CSV quoting, exact header order, and **29 columns**:

```csv
version,kind,id,company_id,name,role,amount,credit,unit,quantity,numerator,denominator,party_id,item_id,warehouse_id,unit_id,base_unit_id,customer,supplier,indivisible,reference,settlement_reference,bank_reference,holder,state,document_date,due_date,as_of,statement_amount
```

Each row requires `version=sarv-opening-v1`, `kind`, a stable globally unique row `id`, and the selected `company_id`. Unknown/reordered/extra headers, unknown kinds and duplicate IDs are errors. Resource identifiers are explicit exact strings; similar normalized names do not authorize a merge. Rows may reference definitions later in the same file. All resource references must resolve within this file and company. A pack ID is its source-row identity; the stored conversion belongs to the identified item/unit.

The table lists every remaining allowed field. **All listed fields are required; all other fields must be empty.** `amount` means remaining amount/value, never a price or a second stock receipt. `reference` changes meaning explicitly with the row kind.

| Kind | Fields | Meaning |
| --- | --- | --- |
| `opening` | `name,as_of` | Exactly one row; its ID is the source ID, name is the review reason, cutoff is canonical Gregorian |
| `ledger` | `role,amount,credit,unit,reference` | Independent GL debit in amount and credit in credit; explicit original ledger source reference |
| `party` | `name,customer,supplier` | Both roles can be true; AR and AP remain independent |
| `unit` | `name` | Stable base or selected pack unit definition |
| `item` | `name,reference,base_unit_id,indivisible` | Reference is SKU; explicit base unit and whole-base-unit constraint |
| `pack` | `item_id,unit_id,numerator,denominator` | One selected unit equals numerator/denominator base units; exact positive rational |
| `warehouse` | `name` | Warehouse identity; inventory role mapping is company-wide in v1 |
| `cash` | `name,amount,unit,reference` | Named cashbox and counted-funds source |
| `bank` | `name,amount,statement_amount,unit,reference,bank_reference` | Named account, explicit book and statement balances, account identity and statement source |
| `ar`, `ap` | `party_id,amount,unit,reference,document_date,due_date` | Unpaid invoice/bill remainder; reference is that historical document |
| `supplier_advance`, `customer_advance` | `party_id,amount,unit,reference,document_date` | Identified unapplied remaining advance; source is explicit |
| `cheque_receivable`, `cheque_payable` | `party_id,amount,unit,reference,settlement_reference,bank_reference,holder,state,document_date,due_date` | Reference is cheque identity; settlement reference is the historical already settled invoice/bill; document date is issue date; state must respectively be `held` or `issued` |
| `stock` | `item_id,warehouse_id,unit_id,quantity,amount,unit,reference` | Counted selected-unit quantity plus total value; source is count/provenance |
| `grni` | `party_id,item_id,warehouse_id,unit_id,quantity,amount,unit,reference,document_date` | Received-unbilled subset already included in stock; reference is receipt; document date is receipt date |

`role` uses these exact keys: `cash,bank,receivables,cheques_receivable,inventory,supplier_advances,fixed_assets,payables,cheques_payable,grni,customer_advances,loans,owner_capital,owner_drawings,retained_earnings,sales,sales_returns,cogs,operating_expense,bank_fees,inventory_loss`. All mapped control accounts require their own ledger row, including explicit `0,0` if zero; version 1 has no opening COGS/loss movements, so their ledger rows are explicit zero. Revenue and expense opening amounts cannot create fabricated activity.

Amounts require an explicit `unit`: `IRR`, `rial`, `ریال`, `toman` or `تومان`. Persian, Arabic and Latin digits are accepted. No grouping separators, exponent notation or guessing from a display preference. Amounts are nonnegative, at most 9,000,000,000,000,000 IRR; overflow and fractional rials reject. `25.1 toman` equals exactly `251 IRR`. Quantities are positive decimal selected units with at most three fractional digits and at most 1,000,000 selected units per row; pack conversion must produce exact base-unit thousandths, respect indivisibility and fit checked integers. Rational pack factors must be explicit positive integers. Booleans are exactly `true` or `false`. Dates are Gregorian `YYYY-MM-DD`, accept safely normalized digits and never infer Jalali from a year or slash.

The reader accepts at most 10 MiB and 50,000 data rows per file. These are staging safety bounds, not a limit on the local core's record count. Diagnostics carry CSV source row, column and actionable reason; at most 100 diagnostics are returned, with a truncation indicator. Invalid/cancelled staging writes nothing. Structurally valid files with a control difference retain the staged command so the review screen can display the difference; posting rejects until each difference and independent debit/credit total is zero. No automatic balance adjustment or duplicate merge occurs.

The original parsed field strings, physical source row numbers, row IDs/kinds and SHA-256 of the exact input bytes (including BOM/whitespace) remain in the immutable posted command. Trimmed values are used only for typed interpretation. Native-edited staged values retain their original import provenance. The actor and request key come from the application command, not a trusted file field.

## Literal sample checks

At `1405/01/01 = 2026-03-21`: cash 251, bank 300, AR 125+75=200, stock 240+60=300, received cheque 200; AP 100+49=149, issued cheque 100, GRNI 50, supplied capital 952. Debit and credit totals are independently **1,251 IRR = 125.1 toman**. W1 holds 2 cartons × 12 = 24 pieces/value 240. W2 holds 6 pieces/value 60, already including R1's five pieces/value 50. Supplier/customer advances, COGS and inventory loss controls are explicitly zero. Each of the cash, bank, AR, AP, stock, both cheque, GRNI and advance controls has zero difference. The historical cheque settlement references add neither AR/AP nor bank effects.

## Reconciliation exports

`core_export::opening_csv` and `opening_pdf` accept an immutable accounting snapshot, company, `MoneyUnit`, `DateDisplay` and new destination path. The `*_with_preferences` variants also accept `DisplayDigits`. CSV machine amount/quantity/date columns remain canonical ASCII and exact; presentation columns respect preferences. PDF uses licensed existing Vazirmatn/Noto Sans Arabic with Pango shaping, mixed-script isolation, measured row heights and repeated page headings. Oversized individual text/header content rejects before publication and recommends CSV. Files render completely to memory, then publish through the existing synced no-overwrite mechanism; existing files/symlinks remain intact.

The report cutoff is the **initial opening beginning-of-day**, not an end-of-day/current operational balance. Header metadata includes company identity, canonical Gregorian cutoff, selected display cutoff/calendar (PDF also explicitly shows Jalali), posted/reconciled included state, basis, policy version, IRR base and rial/toman display unit. Before a posted opening, export fails instead of certifying an absent register as zero.

CSV `format_version=sarv-opening-report-v1` is a reporting format, distinct from the import format. `control` rows contain role/account, exact `ledger_irr,detail_irr,difference_irr` and display values. Detail rows contain kind/ID, amount IRR, selected/base-unit thousandths, source reference and `snapshot_json` for the full typed resource. `total_debit` and `total_credit` rows state both exact and displayed totals. The `opening_snapshot` row's `snapshot_json` contains the complete immutable `OpeningRecord`, frozen policy/mappings, original source rows/checksum and journal ID; deserialize it as the documented public Rust/serde structure for independent reconstruction. All free-text cells use the existing spreadsheet-formula guard. These files report opening source detail; later operational flows and statutory reports require their separate milestones.

Repeatable checks:

```sh
source /workspace/sarv-environment/activate.sh
cargo test --test core_import --test core_exports
```

A reviewed explicitly zero fresh company is a nonfinancial `ZeroBalanceInitialization`, with all control ledger rows explicitly zero and no invented financial journal. CSV identifies `opening_effect=zero_balance_initialization` and `included_states=initialized_reconciled_zero_opening`; PDF calls it an initialization reference and states that it has no financial journal. Nonzero openings use `opening_effect=journal`. This distinction does not turn an uninitialized company into a verified zero.

Human ledger/control rows show the mapped account code and Persian account name, resolved from the company snapshot without guessing a missing account. PDF ledger rows omit the duplicated opaque account ID; CSV retains its canonical account ID and unchanged typed snapshots alongside the readable label.
