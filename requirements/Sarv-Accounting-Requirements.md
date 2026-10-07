# Sarv accounting product blueprint

Product requirements and execution contract for Sina

Version 1.0 | 7 October 2026 | Planning deliverable

## 1 Product direction and decisions

Build Sarv into a free, Persian-first, offline accounting product with a reliable Rust accounting core and a professional native desktop experience. Preserve the working quotation application while expanding it through independently verifiable releases. Broad capability is the destination; each release must have a bounded audience, workflow and acceptance gate.

The recommended first accounting audience is small Iranian service businesses, followed immediately by small trading firms and wholesalers. If wholesalers are the initial launch audience, inventory, pack units and cheques become launch requirements; a service-only release must not be presented as complete for them. The existing quotation workflow remains useful throughout this transition.

### What success means

- Correct books: every posted economic event is balanced, durable, traceable and reconcilable
- Faster everyday work: customer and product selection, entry, review, correction and reporting are demonstrably efficient on declared hardware
- Trustworthy UX: users can understand status, amounts, units, consequences, errors and recovery without learning implementation details
- Real freedom: the proposed local core has no subscription, record quota, mandatory account, forced internet access or paid data-export gate
- Evidence-based breadth: parity is tracked against named workflows and product editions, rather than a claim to contain every feature of every competitor

### Decisions Sina must own

| ID | Decision | Proposed starting position |
|---|---|---|
| DEC-01 | Initial market and launch scope | Service core first, then trading; choose before estimating launch |
| DEC-02 | Meaning and funding of free | Free offline core for everyone; funding and external-service subsidies remain undecided |
| DEC-03 | Source and distribution license | Choose proprietary-free, permissive open source or copyleft after dependency/license review; this blueprint selects none |
| DEC-04 | Accounting and stock policy | Iranian accountant approves chart, recognition, rounding, close and valuation policy |
| DEC-05 | Platforms and collaboration | Linux first; Windows requires release proof; multiuser, mobile and cloud are separate projects |
| DEC-06 | Release authority | Define who accepts product scope, accounting correctness, security and statutory readiness |

Hosting, SMS, bank connectivity, electronic-signature services, support and compliance maintenance may have external costs. Free local software does not eliminate those costs. Any optional paid service needs explicit scope, transparent pricing and permission; it must not quietly weaken the free-core promise.

## 2 Starting point and architecture

### Baseline that must be verified

The reported application is an offline Persian quotation tool built in Rust with GTK4 and libadwaita. Repository: [Sarv Quote](https://github.com/Fearlex-creator/sarv-quote). Reference commit c21c4ed is the reported published baseline. A later task reported local commit 4a1dbf7, but its remote publication has not been established. A newer UI pass is active and may supersede both.

Prior task reports describe paged quotation rows, adaptive fields, focus fixes, searchable import previews, replace confirmation and recovery backups, managers/settings polish, 86 Rust tests and one graphical test. Large-catalog and 1,000-line PDF checks were reported. These are historical task claims, not requirement-level verification of the current checkout. Physical printing, screen readers, final-build Wayland and hardware power-loss recovery were explicitly untested. No implemented general ledger or accounting-posting engine is established here.

BASE-01: At Gate 0 record actual repository, branch, HEAD, dirty files, active tasks, schema version and known fixtures. Run the application and baseline tests. Do not reset, overwrite or merge an active UI branch without coordination. Preserve the newer product/price picker, customer/product indexing and typography work if verified.

BASE-02: Create an evidence register with requirement ID, state, owner, commit, test/fixture, screenshot or report, reviewer and remaining gap. States are proposed, implemented, evidenced, accepted, blocked and deferred. A passing test count is not a substitute for named workflow proof.

### Proposed architecture

ARCH-01: Keep a modular Rust monolith. Separate exact domain types and calculations, application commands, persistence/migrations, reports/import/export, and GTK presentation. The accounting kernel must run headlessly in tests. Avoid a rewrite or microservices without demonstrated need.

ARCH-02: Prefer a single local transactional database for single-user operation; confirm the current storage design before selecting or migrating it. If SQLite is adopted, specify transaction mode, durability settings, constraints and safe backup API. Do not place a live database in a shared or cloud-synced folder as a multiuser solution.

ARCH-03: One command boundary owns each complete posting. Use database constraints and domain validation, stable internal IDs, versioned schemas, unique operation keys and explicit state machines. UI callbacks may request a command but cannot write financial tables directly.

ARCH-04: Keep GTK work on its main thread. Use bounded workers for queries, imports, backup and PDF generation, with cancellation at safe boundaries and immutable results returned to the UI. Virtualize or page large lists. Maintain migration compatibility with existing quotations and their printed totals.

## 3 Iranian benchmark and parity register

Evidence date: 7 October 2026. E1 means official workflow/help documentation; E2 means an official product or pricing claim; E3 means incomplete or edition-dependent evidence. These levels do not imply hands-on verification, certification or measured performance. Source codes link to the register in section 18.

| Product and edition | Verified advertised scope | Evidence and limits | Sarv target |
|---|---|---|---|
| Hesabfa online accounting | Journals, sales/purchases, banks/cheques, stock, opening balances, production and closing | E1/E2, IR1; full plan entitlements require checking | ACC, SAL, PUR, INV, TRE; MFG later |
| Hesabfa five plans | Monthly journal quotas vary; pricing page advertises unlimited users | E2, IR2; do not assume every module in every plan | Proposed offline core without record quotas; shared use separate |
| Hesabfa delivery model | Web service; FAQ states no offline app and no direct database export, with Excel export available | E1/E2, IR3 | Local-first operation and complete export, DATA-01 to 04 |
| Holoo APEX supermarket | Simple 81812, advanced 81813, comprehensive 81841, network 81844; POS/barcode/scale and higher-tier operational controls | E2, IR4; exact entitlements vary by tier | Trading core, then POS pack; no all-tier parity claim |
| Holoo APEX restaurant | Simple 84955, advanced 84957, comprehensive 84941, network 84944; recipes, meal cost, tables/kitchen and delivery workflows | E2, IR5; features belong to the relevant editions | Restaurant pack after inventory and production costing |
| Holoo APEX manufacturing 10285 | GL, treasury, trade, warehouse and production; wider subsystem family is advertised | E2/E3, IR10; legacy URL says code 31 but displayed edition is 10285; full entitlements unknown | Core accounting plus separately validated advanced packs |
| Holoo network editions above | Four concurrent users advertised, with expansion options | E2, IR4/IR5; not a universal Holoo license rule | Shared-use server and permission design, ADV-06 |
| Sepidar business modules | Accounting, sales, inventory and treasury; commercial package connects these jobs | E2, IR6 to IR9; modules/dependencies vary; Dasht is separate | Integrated core with documented module boundaries |
| Iranian tax submission | Vendors advertise taxpayer-system connectivity | E2/E3 only; not tax-authority proof or a compliance specification | IR-05 release gate; no statutory claim yet |

PAR-01: Maintain a live feature-by-feature matrix at implementation time: user job, competitor product/version/edition, jurisdiction, official URL, check date, evidence excerpt, Sarv requirement, current result and acceptance artifact. Mark unknowns explicitly. Refresh material claims before release.

PAR-02: Compare user outcomes on the same fixture: steps, errors, task time, recovery, accessibility and correctness. Do not reproduce proprietary assets, schemas obtained without permission, branding or protected implementation details. Benchmarking a public workflow does not require cloning its visual design.

Holoo's full catalog/add-on entitlement matrix and vendor claims of industry-wide breadth remain outside verified coverage. The selected inspected pages establish useful examples, not exhaustive Holoo parity.

## 4 Global patterns worth adapting

The goal is to learn established accounting interactions and controls while designing a coherent Persian-native product. Country editions, paid plans and integrations differ. None of the sources below demonstrates suitability for Iranian statutory use or proves Sarv will be faster.

| Product scope | Official pattern | Sarv adaptation | Evidence |
|---|---|---|---|
| Zoho Books US feature catalog | Guided accounting, bank reconciliation, items and inventory features | Clear business forms, document lifecycles, staged bank import; advanced inventory remains explicit | E2, GL1 |
| Xero US and Xero Central | Home summaries, contact history and bank statement matching | Actionable home, party timeline, match existing entries before creating transactions | E1/E2, GL2/GL3 |
| QuickBooks Online | Audit log and role-specific bill approval/payment permissions | Always-recorded changes; prepare, approve and pay are separate permissions | E1, GL4/GL5; plan availability varies |
| QuickBooks Desktop Enterprise Manufacturing | Industry-specific manufacturing/wholesale tools | Separate BOM, landed-cost and fulfillment packs instead of an oversized first release | E2, GL6 |
| ERPNext ledger and inventory docs | Retained ledger effects, reversals and inventory/accounting relationship | Linked corrections, explicit valuation policy and stock-to-GL reconciliation | E1, GL7/GL8 |
| ERPNext closing docs | Closing voucher is a distinct accounting operation | Separate closing transfers, checklist, lock date and controlled reopen | E1, GL9 |
| GnuCash desktop | Double entry, splits, register and statement reconciliation | Expert register beside simple forms; distinguish cleared from reconciled | E1/E2, GL10/GL11 |

### Product principles from these patterns

UX-01: One document workspace should cover quotation, invoice, bill, return and payment with consistent status, party, lines, totals, attachments and linked history. Show specialized fields only when relevant; preserve quick expert entry.

UX-02: Separate business status from accounting status and external submission status. A saved draft is not posted; a posted sale is not paid; a deposited cheque is not cleared; a locally valid invoice is not accepted by a government service.

UX-03: Every summary must disclose its cutoff, currency/unit and included states, and drill down to the responsible entries. A dashboard amount without an explainable path to source records is incomplete.

UX-04: Recovery is part of the primary workflow. Explain why an action failed, retain safe user input, identify what was committed, and provide the next permitted action. Never turn an uncertain network response into an apparent successful payment or submission.

PAR-03: A competitor feature can be deliberately deferred only with its target phase and prerequisite recorded. Deferred does not mean shipped, unnecessary or permanently rejected. Review the backlog against the chosen market before making a completeness claim.

## 5 Release sequence and dependency gates

These are proposed gates, not promised dates. Estimate only after baseline inspection and a thin vertical slice. Each phase has a releaseable scope; the entire roadmap is not permission to implement everything in one unchecked run.

| Gate | Scope and prerequisites | Exit evidence |
|---|---|---|
| G0 Preserve and specify | Verify active UI baseline; approve initial market; domain model, exact-money policy, golden fixtures, license audit and migration plan | Running current quote app; requirement/evidence register; accountant review plan; backup and restore baseline |
| G1 Prove the kernel | Chart, periods, parties, exact money, opening balances, journals, atomic/idempotent posting, audit and locks | Balanced/retry/crash tests; approved posting fixtures; migration rehearsal; quote regression passes |
| G2 Service accounting | Sales invoices/returns, bills/expenses, receipts/payments, advances, bank reconciliation, reports, export and recovery | One opening-to-close service fixture; AR/AP/bank controls reconcile; native UX review; restored books match |
| G3 Trading accounting | Units/packs, warehouses, receipts/deliveries, stock valuation, COGS, returns, counts and operational cheques | Trading fixture including partial flows/backdating; stock/cheque/GL controls reconcile; target wholesalers complete pilot |
| G4 Release hardening | Permissions, period close, migration/import safety, performance, documentation and platform packaging | Independent QA, all blocking defects fixed, measured budgets, installer/upgrade/uninstall/restore tested |
| G5 Statutory capability | Current official requirements, applicability review, reviewed mappings, test environment and approved credential flow | Tax specialist signoff; accepted official test fixtures where available; response/retry proof; explicit limitations |
| G6 Advanced modules | Independently approved module scope after its dependencies pass | Module-specific acceptance in section 16; shared kernel and controls still pass |

GATE-01: G2 is a useful service-business accounting release. If DEC-01 selects trading first, G3 is required before that market's launch. G4 is required for any public production release. G5 is required before advertising an integrated statutory submission capability; whether any release can be used for statutory books must be separately assessed for its intended users.

GATE-02: Do not close a gate with unexplained subledger differences, unverified recovery, incorrect totals, silent data loss, unrestricted locked-period mutation or a material unresolved permission defect. Performance misses may justify a documented scope or budget revision, never a false passing result.

GATE-03: At each gate produce a working checkpoint, migration/rollback guidance, known limitations and next bounded proposal. Keep existing quotations usable; do not strand real data behind an unfinished architecture migration. A gate can stop for an exact external blocker or decision, with independent authorized work continuing.

## 6 Accounting kernel and exact money

ACC-01: Support company-isolated books, a versioned hierarchical chart, account type/normal balance and posting restrictions. Parties can be both customer and supplier while AR/AP remain distinct. Archive referenced accounts and parties. Manual/imported AR, AP, inventory or cheque control postings must include valid subledger effects; reject unsupported direct journals.

ACC-02: Every committed journal has valid company, period, source, financial date and at least two meaningful lines. Each line has one nonnegative debit or credit amount. Base-currency debits equal credits exactly. Never balance by silently discarding a difference.

ACC-03: Atomically commit source state, journal, stock effects, allocations, numbering and activity record. Enforce durable request-key uniqueness and at most one effect per company/source revision/economic event, even across different request keys. Repeating an identical command returns its existing result; reusing a key with different content fails. Test double click, timeout, crash and concurrent retries with identical and distinct request keys.

ACC-04: Drafts are editable. Posted financial effects are corrected through a linked credit, reversal or amendment with reason and lineage. Snapshot financially relevant master data and policies so later price, unit, account, tax or party edits cannot change issued totals.

ACC-05: Period locks apply to UI, imports, API, reversal, costing and maintenance commands. Authorized reopen requires a reason and history; previously issued reports can identify later adjustments. Closing transfers and lock controls are distinct. Carry balance-sheet balances forward without duplicating them through a second opening import.

### Numerical policy to approve before implementation

MNY-01: Use checked fixed-point integers or a checked decimal type; no binary floating point for quantities, prices, rates or financial calculations. Declare scale, maximum range, overflow behavior and serialization. Invalid or overflowing input is rejected with a recoverable error.

MNY-02: For the initial Iran base-currency profile, propose canonical IRR storage. Rial/toman presentation is explicit: one toman equals ten rials. Conversions must be exact; disallow or explicitly round values that cannot be represented at the chosen scale. Imports must name their unit. Exports and printed totals label it. Switching a display preference must not alter economic value.

MNY-03: Define calculation order: exact quantity times price, line discount, allocated document discount, taxable base, tax, charges and payable total. Persist inclusive/exclusive-tax mode, effective policy version and rounding stage. Choose tie-breaking and allowed scales with the accountant; do not infer them from locale or another product.

MNY-04: Allocate document discounts and residuals deterministically, with stable tie-breaking. Store original allocations; partial credits reverse their attributable original amounts, and the final full return reverses the remaining exact total. A visible rounding account may handle an approved residual; never conceal one in revenue or tax.

ACC-06: Golden fixtures must prove opening trial balance, manual adjustments, accrual/prepayment, advances, partial settlement, refunds, bad debt, close/reopen and the event matrix. Verify current-year earnings before close, historical P&L after close, and no doubled retained earnings/opening balances. An Iranian accountant must approve expected outcomes before production use.

## 7 Posting matrix and accountant fixtures

The examples below are proposed accrual/perpetual-inventory design fixtures. Amounts are artificial IRR values; the tax amounts are test inputs, not Iranian rates or legal guidance. Dr means debit and Cr means credit. Recognition timing, account mappings and exceptions require accountant approval.

| Economic event | Proposed financial effect | Required control |
|---|---|---|
| Quote or order | No GL posting and no on-hand movement | Optional reservation is distinct from shipment |
| Stock receipt valued at 1,500 before bill | Dr Inventory 1,500; Cr Goods received not invoiced 1,500 | Receipt and later supplier bill cannot capitalize the same stock twice |
| Supplier bill 1,500 plus input tax 150 | Dr Goods received not invoiced 1,500; Dr Recoverable tax 150; Cr AP 1,650 | Only recoverable tax goes to that account; cost differences follow reviewed policy |
| Two-item sale net 2,000 plus output tax 200 | Dr AR 2,200; Cr Sales 2,000; Cr Tax payable 200 | Revenue event is separate from payment and delivery states |
| Issue the same two items with total cost 1,200 | Dr COGS 1,200; Cr Inventory 1,200 | Post once at the approved stock/recognition event |
| Customer pays 1,000 | Dr Bank 1,000; Cr AR 1,000 | Allocate to the invoice; outstanding becomes 1,200; no new revenue |
| Return one original item | Dr Sales returns 1,000; Dr Tax payable 100; Cr AR or customer credit 1,100 | Link original quantity, discount and tax allocation |
| Restock that eligible returned item | Dr Inventory 600; Cr COGS 600 | Use original issued cost; damaged/non-restockable returns require a different approved disposition |
| Supplier payment of 1,650 | Dr AP 1,650; Cr Bank 1,650 | Supplier allocation and bank matching remain linked |
| Separate fixture: accepted receivable cheque for a fresh 2,200 invoice | Dr Cheques receivable 2,200; Cr AR 2,200 | Proposed policy; bank remains unchanged until clearance |
| Clear that cheque | Dr Bank 2,200; Cr Cheques receivable 2,200 | Bounce/return uses an explicit linked transition, reopens receivable/allocation as applicable |
| Issue and clear supplier cheque | Issue: Dr AP, Cr Cheques payable. Clear: Dr Cheques payable, Cr Bank | Amount and state transitions balance; replacement cannot settle AP twice |

The primary sale fixture has 100 AR remaining after the 1,000 payment and 1,100 credit. The 2,200 cheque rows use an independent invoice, not that residual balance.

POST-01: Express each permitted event as a reviewed posting template with accounts, amount derivation, prerequisites, idempotency key, stock/settlement effects and reversal. Reject unsupported state transitions. Preserve the policy version applied to each event.

POST-02: Extend fixtures to direct expense, capital purchase, deposits, owner capital/drawings, loans/interest, bank fees, cash transfer, write-off and tax settlement. Separate expense from inventory and asset purchases. Transfers cannot create income. Clearing accounts must have an exception report and reconciliation path.

POST-03: Partial fulfillment, late bills, cost corrections, returns after settlement, bounced cheques and locked periods must be tested end to end. Golden outputs include journals, open items, stock value, P&L and balance sheet, rather than isolated formula tests.

## 8 Sales purchases and open items

SAL-01: Preserve quotation creation, revisions, customer search and catalog selection. Quote to order to delivery to invoice is linked by stable IDs and line-level remaining quantities. Direct invoices remain available. A revised quote cannot silently change an accepted order or posted invoice.

SAL-02: Support customer-specific prices, price-list effective dates, pack prices, line/document discounts, explicit currency/unit, terms, due dates and configurable tax categories. Show the selected price's source and allow an authorized override with reason. Freeze selected values on the document.

SAL-03: Track partial deliveries, partial invoicing, partial receipts, installment schedules, credit limits, advances and unapplied receipts. Separate business, fulfillment, posting and collection states. Define overpayment as an explicit advance/credit; allocations cannot exceed the available payment or eligible balance.

SAL-04: Link credit notes and sales returns to original lines, remaining returnable quantity, price/discount/tax allocation and fulfillment. Support refund or later credit application. Test return before payment, after partial/full payment, damaged goods and the final return rounding remainder.

SAL-05: Produce previewable quotations, invoices, delivery notes, statements and receipts. Persist numbering scheme and document identity; provide voided-number history where required. Document numbers must remain unique within the approved company/type/period scope, including retry and import.

PUR-01: Support purchase order to receipt to supplier bill to payment, plus a direct bill/expense route. Partial receipts/bills, supplier references, advances, credits, purchase returns and refunds have explicit remaining amounts and quantities.

PUR-02: Detect likely duplicate supplier bills using company, supplier, invoice reference, date and amount, with an explicit review outcome. Do not auto-merge financial documents on a fuzzy match. Distinguish goods receipt from billed liability through a reviewed clearing policy.

PUR-03: Map each purchase line to stock, expense, asset or prepayment. Stage freight and landed costs under a documented allocation basis in the trading release; retain allocation lineage and locked-period behavior. Basic asset purchases must remain recordable before an asset submodule exists.

PTY-01: A party profile includes identity/contact data, roles, terms, opening open items, transaction timeline and statements. Archiving keeps historical references. Any merge must preview references and preserve identities, audit and balances; do not silently merge because normalized names match.

OPEN-01: Opening setup imports ledger balances, AR/AP open items, stock quantity/value, banks, cash and outstanding cheques as one validated opening process. Reconcile every subledger to its control account before finalization. Protect against duplicate opening effects and provide an accountant-reviewable exception list.

Acceptance: Complete quotation to partial shipment/invoice to two payments to return/refund; and purchase order to two receipts to bill/credit/payment. Reopen after restart, print, export and trace all balances back to their events.

## 9 Inventory and valuation

INV-01: Items have stable identity, SKU, aliases, barcodes, category, stock/service type, base unit and exact conversion rules. Pack/carton/piece conversions are part of a wholesaler release. Archive referenced items without losing history. Price lists never overwrite historical document prices.

INV-02: Maintain quantity and value movement ledgers by company, item and warehouse, with source event and effective date. Distinguish on hand, reserved, available and in transit. Transfer between warehouses changes location without creating revenue or unexplained total-company value.

INV-03: Choose one valuation method first. Moving weighted average is a candidate, not a decision already made. Specify ordering of same-time events, price precision, cost rounding, receipt/bill variances and zero-balance behavior. Preserve enough cost provenance for exact returns and reproducible as-of valuation.

INV-04: Block negative stock initially unless Sina and the accountant explicitly approve a complete negative-stock costing model. Define whether backdating is blocked or handled by controlled recalculation. Recalculation must identify downstream affected entries and never silently alter a locked period.

INV-05: Sales returns use the attributable original issue cost where the approved policy requires it. Purchase returns reference original receipts and their cost adjustments. Quantities cannot exceed remaining eligible original quantities without an explicitly authorized exception path. Write-off, scrap and damaged returns have distinct dispositions.

INV-06: Provide receiving, delivery, warehouse transfer, count, count variance, adjustment, damage/write-off and reorder workflows. Counts record their cutoff and handle movements during counting. An adjustment requires a reason, permission and financial effect preview.

INV-07: Historical stock value must equal the inventory control balance at the same cutoff under the adopted policy, with categorized reconciliation exceptions. COGS must reconcile to the issue/cost ledger. Reports show quantity, unit, value and method; a margin report is invalid if cost coverage is incomplete.

INV-08: Batch/expiry and serial tracking are explicit extensions. Promote them to launch requirements if the chosen sector needs them. Lot selection, recall traceability, expiry warnings and serial uniqueness need separate fixtures; a generic text field is insufficient.

### Required stock fixture

Begin with opening quantity and value. Receive at two different prices, transfer between warehouses, reserve and partially deliver an order, invoice, receive a late supplier bill, sell more, return one original sale, return part of a purchase, count and write off damage. Compare the full movement ledger, cost layers/calculation record, COGS, stock report and GL. Repeat with a backdated receipt, an attempted locked-period correction, rounding at small quantities and process interruption.

Acceptance requires zero unexplained quantity or value difference. Do not certify trading accounting merely because the quantity screen and invoice total look plausible.

## 10 Treasury cheques and reconciliation

TRE-01: Model multiple cashboxes, bank accounts and petty cash with receipts, payments, transfers, charges and supporting references. One payment can settle several invoices; one invoice can have several payments. Handle split methods, advances, credits and refunds without losing open-item lineage.

TRE-02: A bank import first enters a staging area. Capture statement identity, account, date range, opening/ending balance and row origin. Preview mapping, unit, duplicates and errors. Reimporting or resuming a file cannot duplicate financial effects. Retain the original file checksum and import result.

TRE-03: Suggest matching to existing book entries before creating new ones. Support one-to-one, split and grouped matching with explicit fee treatment. A match cannot over-allocate either side. Importing a statement line is not itself a journal posting.

TRE-04: Separate unreviewed, matched, cleared and reconciled states. Reconcile to statement date and ending balance, explain outstanding items and differences, and retain a reconciliation report. Unreconciling needs permission, reason and history. Subsequent changes cannot silently invalidate an accepted reconciliation.

CHQ-01: First-class receivable/payable cheque records include direction, bank, amount/unit, issue/due dates, references, current holder/location, party, linked settlement and status history. Record only needed personal data; support restricted visibility of bank details.

CHQ-02: Define allowed lifecycles for received/issued, deposited, cleared, bounced, returned, cancelled, replaced and transferred/endorsed states where applicable. Each transition has an approved posting template, permissions and reversal behavior. Receiving/depositing never automatically means bank clearance.

CHQ-03: Due-cheque schedules, a clearly defined maturity-average calculation, overdue/bounced exceptions and near-term cash commitments drill into source records. A bounced receipt can reopen the related customer balance; replacement must not count as a second settlement. Bank fees use separate entries.

CHQ-04: Recording cheque details locally does not mean registration or verification in an official banking system. External integration, credentials, current rules and live actions require separate evidence and authority.

### Treasury acceptance scenario

Create a customer invoice, collect a partial cash receipt and a cheque, deposit and bounce the cheque, replace it, clear the replacement, and match the bank statement including a fee. Pay a supplier using a cheque and reconcile its clearance. Add a cash-to-bank transfer and petty-cash replenishment. Verify AR/AP, cheque controls, cash/bank GL and statements at every cutoff.

The cash forecast is a forecast based on known due items. Label assumptions and avoid presenting it as a formal cash-flow statement or a guarantee of available cash.

## 11 Reporting controls and settings

RPT-01: Ship journal, general ledger, subsidiary detail, trial balance, balance sheet, P&L, cash/bank movement, AR/AP aging and party statements. Add stock movement/value, gross margin, sales/purchases, cheque schedule and tax transaction registers with their prerequisite modules.

RPT-02: Every report discloses business, period/as-of date, currency and display unit, accounting basis, filters and included statuses. Re-running against the same snapshot produces the same result. Drill from totals to entries to source documents. Aging must state due-date basis and buckets.

RPT-03: Provide control reports for AR versus receivables control, AP versus payables control, stock versus inventory control, cheques versus cheque controls and bank reconciliation. Clearly identify incomplete cost, unmapped tax, unposted, import and clearing exceptions. Close is blocked by unresolved integrity failures.

RPT-04: Cash movement and forecast can ship first. A formal cash-flow statement needs an approved classification policy, noncash handling and reconciliation to cash and balance-sheet movements; do not relabel an account-balance list as that statement.

CTL-01: Define roles for owner/admin, accountant, sales, purchasing, warehouse, cashier and read-only. Permissions cover view, create, edit draft, post, reverse/refund, cost/margin access, backdate, unlock, export, restore and administration. Test the command boundary, not just hidden buttons. A local OS administrator can access local files unless separately protected; do not claim otherwise.

CTL-02: Approval is separate from posting and payment. Start with a clear single-user control model; add thresholds and dual approval only when identity, permissions and workflow are enforceable. Avoid fake multiuser assurance from a shared password.

CTL-03: Record actor/local identity, recorded-at time, financial date, operation, before/after or durable event, reason, source and links. Posted effects are never silently overwritten. Local append-only application history is not tamper-proof or legal non-repudiation against someone who controls the files.

SET-01: Settings cover business identity/print branding, fiscal years, chart mappings, currency/unit, dates/digits, numbering, terms, discounts/tax policies, warehouses, appearance/density, shortcuts, data location, backups and integrations. Distinguish presentation settings from policy changes that affect future documents.

SET-02: Every setting must save, survive restart and demonstrably change the relevant behavior. Show defaults, validation, unsaved changes and reset scope. Changes cannot retrospectively alter posted amounts or printed-document snapshots.

Acceptance: Reconcile a complete fixture, close and reopen with a reason, run denied-role commands directly, change each setting and restart. No report or permission is accepted solely because its navigation entry exists.

## 12 Data resilience security and migration

DATA-01: Import CSV and spreadsheet data through field mapping, normalized matching, preview, row-level errors, duplicate policy and a review queue. Preserve source row references and original values. Bulk replacement requires a specific confirmation and a usable recovery copy. Use transactional batches with resumable, idempotent checkpoints.

DATA-02: Export all user-owned business data, identifiers, relationships, policies, postings, allocations and audit history in documented machine-readable formats, plus human-readable accountant reports. Include attachments or a manifest with verified references. Test an independent reconstruction or round-trip sample; a PDF-only export is insufficient.

DATA-03: Backups include database, attachments, policy/configuration and a manifest with version, time and checksums. Support scheduled local copies and a user-chosen separate destination. A copy on the same disk is not disaster protection. Backup must use a consistent snapshot rather than an unsafe live file copy.

DATA-04: Restore into a fresh profile first, validate schema/integrity, counts, IDs, trial balance and attachment checksums, then make replacement an explicit action. Test interrupted backup, corrupt archive, missing attachment, old version and insufficient disk. Record last successful backup and last verified restore separately.

MIG-01: Before schema upgrade, create and verify a recovery snapshot. Migrations are versioned, repeatable in tests and transactional where possible. Prove old quotation totals, IDs, dates and print output remain stable. Where downgrade is unsupported, say so and supply a tested restore path; never promise automatic rollback without proof.

SEC-01: Keep the offline core functional without telemetry or mandatory sign-in. Any analytics, update checks or external integration has clear disclosure and consent. Apply least privilege, redact sensitive logs and document local data protection and threat assumptions.

SEC-02: Store secrets in an appropriate operating-system credential facility if and when approved integrations exist. Never embed tokens in source, test fixtures, screenshots, logs or exports. Key creation/configuration, live financial actions and statutory submissions require their own authorized workflow.

SEC-03: Review dependencies and font licenses, lock dependency versions, audit advisories and produce a software bill of materials for releases. Test hostile import sizes, malformed files, formulas/CSV injection, path traversal in archives and attachment handling. Do not run imported macros or arbitrary commands.

PLAT-01: Linux is the current application context, not a universal support claim. Declare supported distributions, GTK/runtime versions, X11/Wayland and packaging. Windows needs a separate build, native interaction, installer, upgrade, printing and recovery test matrix. Mobile, browser UI and cloud hosting require distinct scopes.

Acceptance: Upgrade a real baseline copy, run the complete fixture, simulate failures, restore to a clean installation and compare data and reports. Keep untouched user originals outside the experiment.

## 13 Persian localization and statutory boundaries

IR-01: Persian-first RTL must work throughout navigation, forms, tables, dialogs, search, print and PDF. Mixed Persian/Latin codes, account numbers, phone numbers and negative amounts use tested bidirectional isolation. Preserve logical reading order and copy/paste values; screen mirroring alone is insufficient.

IR-02: Accept Persian, Arabic and Latin digits where safe; normalize search variants such as Arabic/Persian yeh and kaf while retaining original display data. Apply normalization consistently to search and duplicate review, not as authority to merge records. Exact item codes remain distinguishable.

IR-03: Provide Jalali date entry/display with validated conversion to an unambiguous canonical date. Distinguish document date, posting date, due date and recorded-at timestamp. Define company timezone and fiscal boundaries; test leap years, midnight, year-end, invalid dates and CSV round trips. Display preferences must not shift a transaction into another accounting period.

IR-04: Use a readable Persian typeface with verified redistribution and embedding rights. A request or preference for IranBakh does not establish a license. Evaluate an appropriately licensed alternative such as Vazirmatn only after checking its official license and shaping/rendering in the actual app. Test dense numbers, punctuation, mixed scripts and PDF font embedding with native readers.

### Statutory capability is a separate release gate

Current Iranian tax-authority requirements were not established by this research. Vendor descriptions cannot substitute for official schemas, applicability, rates, deadlines or approval. No current rate, filing deadline or compliance certification is asserted in this blueprint.

IR-05: Before advertising statutory use or submission, obtain current primary technical documentation and review it with an Iranian accountant/tax specialist. Record source, version, effective date and applicability to the target entity and invoice types. Review identifiers, goods/services mappings, exemptions, rounding, books/retention, document relationships and signing/submission arrangements.

IR-06: Build a versioned adapter with local validation, immutable submission payload snapshot, queue, correlation key, status query, duplicate-safe retry and retained responses. Distinguish locally posted, queued, sent, accepted, rejected and uncertain. An interrupted request must query status before blind resubmission where the protocol allows it.

IR-07: Test original, correction, cancellation and return references, rejection recovery and locked-period interactions against official test fixtures/environment where available. Keep credential entry and live submissions behind approved flows. Offline bookkeeping must remain usable while a service is unavailable; external acceptance cannot be guaranteed offline.

IR-08: The same rule applies to official cheque/bank services, payroll/insurance outputs and any later sector-specific reporting. Required authorization, current law and external service behavior must be verified for that module. A configurable tax percentage field is not a tax-compliance engine.

## 14 Native UX acceptance contract

Navigation: Overview, Sales, Purchases, Inventory, Money and cheques, Accounting, Reports, Settings. Keep customers, suppliers and items globally reachable. Use libadwaita components where appropriate, a restrained visual hierarchy, consistent spacing and clear primary actions. Progressive disclosure must preserve expert speed.

| ID and screen | Required interaction | Observable acceptance |
|---|---|---|
| UX-05 Overview/sidebar | Due receipts/payments/cheques, exceptions, low stock and backup state; role-aware groups | Each card drills into its filtered source; current module/state obvious; no misleading zero on loading/error |
| UX-06 Party/item managers | Indexed search, filters, keyboard navigation, edit/archive and linked history | Find known/partial aliases in large fixtures; distinguish similar records; preserve selection and return focus |
| UX-07 Product and price picker | Search SKU/name/alias/barcode; show unit, current price, source/date, currency and availability | In a quote/invoice, choose correct product and price without leaving editor; stale/missing price explicit; keyboard-only path works |
| UX-08 Document editor | Sticky essential actions/totals, editable rows, predictable tab order, status and history | Add/edit/remove/reorder rows without lost focus or values; restart restores saved state; 1,000 lines remain reachable |
| UX-09 Review and posting | Readable totals, tax/discount breakdown, accounting preview and consequences | Double click does not duplicate; invalid fields focus correctly; clear posted/failed/uncertain outcome |
| UX-10 Import/matching | Staged preview, search, per-row issues, duplicate decisions and recoverable commit | Correct errors without retyping valid rows; replace scope explicit; cancellation leaves an explainable state |
| UX-11 Reports/print | Filters, drill-down, readable preview, export/print progress | UI and PDF amounts/units match; repeated page headings; no clipped Persian text or missing rows |
| UX-12 Settings/recovery | Searchable groups, Save/Revert, backup/restore status and safe previews | Every setting behavior tested; recovery procedure understandable without shell access |

UX-13: Preserve muscle memory: stable shortcuts and tab order, predictable Enter/Escape, visible focus, retained selection/scroll on return and sensible defaults. Publish shortcuts and avoid conflicts with Persian text entry. Destructive or irreversible actions need clear consequences; ordinary safe entry should not be buried in confirmations.

UX-14: Review real native screenshots at standard and narrow windows, dark/light appearance, high contrast and enlarged text. Test keyboard-only complete workflows and screen-reader labels/order. Record unsupported combinations honestly. A browser mockup is not proof of GTK rendering or native accessibility.

UX-15: Use native Persian readers and representative accountants/traders for task tests. Compare baseline and new flow on time, errors, steps and recovery. Better UX is established by evidence, not gradients, screenshots alone or the number of redesigned screens.

## 15 Performance and independent quality gates

The numbers below are proposed budgets to ratify and measure, not current Sarv results or competitor claims. Rust alone proves neither low latency nor correct accounting.

PERF-01: Reference profile: release build, supported Linux, four-core x86-64 CPU, 8 GB RAM and SSD. Record exact CPU, OS, filesystem, build, database settings and screen scale. Fixture: five fiscal years, 100,000 documents, 1,000,000 journal lines, 50,000 items and 10,000 parties; include a 1,000-line quotation. Publish generator/seed and data shape.

| Operation | Proposed target | Measurement boundary |
|---|---|---|
| Cold usable home | At most 3 seconds | Launch to interactive usable state; distinguish OS-cache cold and process cold |
| Loaded navigation | p95 at most 150 ms | User action to usable result, not just dispatch |
| Indexed party/product lookup | p95 at most 200 ms | Query to first correct selectable page |
| Filtered ledger | p95 at most 500 ms | First page including query and rendering |
| Typical 20-line invoice posting | p95 at most 250 ms | Command through durable commit and visible result |
| Annual trial balance and P&L | p95 at most 2 seconds | Same fiscal-year fixture and filters |
| 10,000-row CSV import | At most 30 seconds | Staging/validation/commit separately reported; errors included |
| Responsiveness and memory | No continuous main-thread block above 50 ms in tested heavy flows; initial RSS budget 250 MB | Trace imports, reports, PDF and scrolling; report steady/peak memory separately |

PERF-02: Use repeatable warm runs and genuinely defined cold runs; report p50/p95/p99, sample count and worst outliers. Do not combine unlike datasets or hide durability costs. Exports stream with bounded memory, progress and safe cancellation. Revise a budget only with measured evidence and an accepted tradeoff.

QA-01: Test layers: unit arithmetic/state machines; property-based totals and invariants; database integration and migrations; accountant-approved golden journals/reports; GUI workflows; rendered PDF; packaging; fault injection and restore. Use independent expected results, not the production calculation copied into tests.

QA-02: Fault scenarios include disk full, process kill around every commit boundary, corrupt input, duplicate/reordered commands, clock/date boundaries and interrupted migration. If hardware power-loss guarantees are claimed, test the actual storage/durability setup; process-kill tests alone are insufficient.

QA-03: Independent review reruns representative workflows and challenges claims. No unresolved severity-1/2 correctness, data-loss, security or core-workflow defect at release. Pilot users must complete opening-to-close, reconcile controls, restore a backup and export their data. Physical printing, screen readers and platform-specific behavior remain unaccepted until actually tested.

## 16 Advanced modules with explicit gates

These capabilities remain in the product vision. Each requires a separate approved scope, schema/API design, fixtures, UX review and release evidence. Industry packs must reuse the accounting kernel rather than fork its rules.

| ID and module | Prerequisites | Minimum acceptance before claiming support |
|---|---|---|
| ADV-01 Fixed assets | G2, account policies | Asset register, capitalization, effective depreciation, disposal/revaluation policy, GL reconciliation and period locks |
| ADV-02 Management accounting | G2/G4 | Cost centres, projects, budgets, allocations and variance reports reconcile to GL; no duplicate expense |
| ADV-03 Manufacturing | G3, reviewed cost model | Versioned BOM, materials issue, work in progress, output/byproducts, labor/overhead, scrap and cost variance fixture |
| ADV-04 POS and retail | G3, peripheral design | Barcode/scales, cashier shifts, receipts, discounts, cash count, refunds and terminal settlement; offline failure/duplicate sale tests |
| ADV-05 Payroll | G2, current jurisdiction review | Effective-dated pay/withholding/insurance rules, payslips, accrual/payment reconciliation, privacy roles and approved statutory outputs |
| ADV-06 Concurrent multiuser | Enforced identities/roles, server architecture | Transaction serialization, concurrency conflicts, authorization/audit, numbering, stock/settlement races, backup and operational security |
| ADV-07 Offline synchronization | ADV-06, stable IDs and outbox | Idempotent replay, conflict policies, tombstones/versioning and interrupted sync; no last-write-wins for money/stock |
| ADV-08 Multi currency | G2, money model extension | Transaction/base amounts, dated rates, settlement FX, realized/unrealized gains/losses, revaluation and per-currency reconciliation |
| ADV-09 Distribution and branches | G3, approved organization model | Pick/pack/dispatch, delivery proof, commissions, routes and branch transfers; consolidation eliminates intercompany effects where in scope |
| ADV-10 Restaurant and retail verticals | G3; ADV-03 as needed | Recipes/portion yields, table/kitchen/delivery flows; sector-specific batch/serial fixtures tied to stock and cost |
| ADV-11 Integrations and automation | Stable APIs, permissions | E-commerce, bank feeds, OCR, recurring documents and portals have staging, duplicate handling, human review and external-cost disclosures |
| ADV-12 Mobile and cloud clients | Approved platform/security model | Separate UX, identity, offline/sync behavior, hosting/privacy and release tests; not implied by Rust or desktop availability |

ADV-13: Apparel production needs size/color variants, roll units, cutting/sewing handoffs, waste and cost fixtures (IR11). Jewelry needs separate weight/purity/metal and money ledgers, workmanship and specialist tax validation (IR12). Contracting needs progress claims, retentions, guarantees and subcontractor reconciliation; import procurement needs shipment/customs and landed-cost/FX reconciliation. Each depends on G3 and relevant advanced modules, with its own approved acceptance fixture.

ADV-14: Consignment needs custody/ownership and settlement reconciliation. CRM, service/subscription billing and analytics remain explicit demand-led expansions. Manual journals do not establish automated module parity.

Free-core access, export and correctness must remain intact as extensions arrive. Funding, optional service pricing and open-source licensing require explicit decisions; do not let an implementation agent choose a business model by adding dependencies that require paid infrastructure.

## 17 Agent execution and release contract

EXEC-01: Read this specification and the paste-ready brief. Inspect the actual repository and active work before editing. Present a short baseline, gaps, dependencies and proposed first bounded milestone. Resolve market, accounting-policy or license decisions before dependent irreversible design; continue safe inspection and tests meanwhile.

EXEC-02: Research the exact job using current official sources, then trace it to a requirement ID and acceptance fixture. Maintain requirements/evidence.md, decisions.md, migrations.md and a test-fixture index in the working project. Keep product facts, assumptions, proposed choices and untested claims distinct.

EXEC-03: Parallelize by bounded ownership. Use up to ten agents only when runtime capacity and independent work justify it; fewer is fine. Assign domain/posting, persistence/migration, GTK UX, reports/import/export, localization/accessibility, performance and independent QA as useful. One owner controls shared schemas and interfaces; one integrator coordinates commits. Avoid multiple writers to the same files or database migrations.

EXEC-04: Use the loop research, baseline, requirement, test/fixture, implement, run/render, independent QA, fix and checkpoint. Build vertical slices rather than empty menus. Review native application screenshots and rendered financial documents. Keep working code and user data intact; never replace the app with an unintegrated demo.

EXEC-05: Each accepted requirement needs commit, commands and results, fixture/expected output, screenshot or report where relevant, reviewer and known limitations. Evidence must be rerunnable. Mark an untested condition untested; test counts, checkboxes and self-assessed polish do not establish acceptance.

EXEC-06: Stop the current milestone when its acceptance gate passes, the user changes scope, or a specific decision/authorization/environment blocker prevents further authorized progress. Do not stop at a first successful build, and do not run an unbounded perfection loop. Report material findings and blockers promptly. Offer the next bounded phase rather than silently expanding into the entire roadmap.

### Authority and delivery boundaries

EXEC-07: No paid services, live bank/payment actions, government submissions, credential creation/configuration, production-data sharing, destructive data operations, public deployment or publishing without the required explicit approval. Use synthetic/anonymized fixtures. Never put secrets in prompts, repository files, screenshots or evidence. Follow repository/environment instructions and preserve active UI work.

REL-01: A release checkpoint contains source commit, clean/dirty state, build/package, release notes, requirement coverage, accountant and QA review outcomes, performance report, license/SBOM report, migration/restore guide, screenshots, sample PDFs and precise remaining limitations. Do not claim Windows, tax compliance, multiuser or physical-print support from an untested build.

REL-02: Final demonstration: create/open business, import verified opening data, quote with catalog price, sale/receipt/return, purchase/payment, stock and cheque flows where in scope, reconcile controls, close, export, backup, restore and reopen. A reviewer must reproduce the amounts and trace every total.

## 18 Source register

Primary vendor/project sources checked on 7 October 2026. Product pages substantiate advertised capabilities, not independent quality, speed, universal plan entitlements or compliance. The repository and prior task reports provide baseline context only; Gate 0 must verify actual code and results.

- IR1 [Hesabfa accounting facilities](https://www.hesabfa.com/view/facilities/accounting)
- IR2 [Hesabfa pricing and plans](https://www.hesabfa.com/Prices)
- IR3 [Hesabfa official site and FAQ](https://www.hesabfa.com/)
- IR4 [Holoo APEX supermarket editions](https://holoo.co.ir/product/سوپر-مارکت/)
- IR5 [Holoo APEX restaurant editions](https://holoo.co.ir/product/رستوران/)
- IR6 [Sepidar commercial accounting package](https://www.sepidarsystem.com/products/sepidar/commercial-accounting-software/)
- IR7 [Sepidar treasury module](https://www.sepidarsystem.com/products/sepidar/receipt-and-payment-system/)
- IR8 [Sepidar inventory module](https://www.sepidarsystem.com/products/sepidar/warehouse-system/)
- IR9 [Sepidar customer and sales module](https://www.sepidarsystem.com/products/sepidar/customer-and-sales-system/)
- IR10 [Holoo APEX corporate manufacturing 10285](https://holoo.co.ir/product/ساده-کد-31/)
- IR11 [Holoo APEX apparel editions](https://holoo.co.ir/product/نرم-افزار-حسابداری-تولیدی-پوشاک/)
- IR12 [Holoo APEX jewelry editions](https://holoo.co.ir/product/نرم-افزار-حسابداری-طلا-و-جواهرفروشی/)
- GL1 [Zoho Books US feature catalog](https://www.zoho.com/us/books/accounting-software-features/)
- GL2 [Xero US feature catalog](https://www.xero.com/us/accounting-software/all-features/)
- GL3 [Xero bank reconciliation procedure](https://central.xero.com/0/article/Reconcile-your-bank-account)
- GL4 [QuickBooks Online audit log](https://quickbooks.intuit.com/learn-support/en-us/help-articles/learn-about-the-audit-log/00/262225)
- GL5 [QuickBooks bill approval and payment roles](https://quickbooks.intuit.com/learn-support/en-us/help-article/manage-workflows/set-use-bill-approval-payment-release-workflows/L1IOLL9hv_US_en_US)
- GL6 [QuickBooks Enterprise manufacturing](https://quickbooks.intuit.com/desktop/enterprise/industry-solutions/manufacturing/)
- GL7 [ERPNext immutable ledger](https://docs.frappe.io/erpnext/immutable-ledger-in-erpnext)
- GL8 [ERPNext inventory accounting](https://docs.frappe.io/erpnext/accounting-of-inventory-stock)
- GL9 [ERPNext period closing voucher](https://docs.frappe.io/erpnext/period-closing-voucher)
- GL10 [GnuCash features](https://www.gnucash.org/features.phtml?lang=en_US)
- GL11 [GnuCash statement reconciliation](https://wiki.gnucash.org/docs/C/gnucash-manual/gui-reconcile.html)

Tax-authority documentation is a pending verification dependency under IR-05, not a source for any asserted current rule in this document. Recheck applicable official material before statutory design or release. Proposed architecture, UX, accounting fixtures and performance targets are Sarv requirements requiring validation, not statements made by these vendors.
