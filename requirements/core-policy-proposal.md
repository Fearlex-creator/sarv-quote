# Core policy version 1 and reconciled opening proposal

**Prepared 2026-10-07. Documentation proposal only; no financial implementation, schema migration or existing-book modification.** This records the user's latest delegation to choose practical defaults from the guide and to defer legal/tax work. It supersedes earlier *decision pending* language for these proposed engineering defaults; it does not rewrite historical acceptance evidence or claim professional approval. Root integrates the next bounded implementation after M2 native/date/sidebar/responsiveness acceptance.

The target is a usable trading/wholesaler/small-operator core. Requirements: ACC-01–06, MNY-01–04, POST-01–03, OPEN-01, INV-01–07, PUR-01–03, SAL-01–04, SET-01/02, DATA-02–04 and MIG-01. Operational treasury/cheques follow connected sales/purchases/inventory/returns in the user-directed sequence. Minimal opening registers for money and outstanding cheques are prerequisites for full OPEN-01; they do not establish an operational treasury module.

## 1. Visible defaults and compatibility

Create a company-specific, immutable `core-policy-v1` revision with effective financial date and a visible Persian settings summary. Every posting freezes policy revision, role-to-account mappings, unit conversions, source prices, discount allocations and relevant master-data versions. Revised settings apply prospectively to supported new commands; no retroactive recalculation of posted history. Policy revisions and snapshots must survive export/backup/restore. Drafts retain their working policy and require a visible recalculation preview when deliberately upgraded.

| Policy | Version 1 engineering choice | Compatibility and requirement |
| --- | --- | --- |
| Recognition | Accrual accounting; perpetual inventory; version 1 goods sale and delivery post together on the same financial date; goods receipt may precede supplier bill through GRNI | ACC/POST/INV. Quote/order/reservation has no GL/on-hand effect. A financial event is distinct from saving its draft. Delivered-but-unbilled sales and pre-delivery sales invoices remain blocked until an explicit accrual/fulfillment extension exists. |
| Currency | Canonical signed integer IRR; one toman is exactly ten rials; fractional-rial input rejected | Existing user decision and `Money` contract remain. Decimal toman display preserves residual rials. Units are explicit in input/export/print. |
| Quantity | Base-unit thousandths; positive source movement quantities; direction is a separate event type | Preserve existing quote precision. Stock balances use checked integer quantities; no binary floats. Zero/negative input cannot masquerade as a return. |
| Prices and amount limits | Integer IRR per selected sale/purchase unit; quantity per document line at most 1,000,000 selected units; source price/line/document amounts at most 9,000,000,000,000,000 IRR | These preserve existing quote limits. Checked `i128` intermediates; exact checked `i64` persistence/ledger sums. Reject overflow before any write. Aggregate stock quantities may exceed a line limit, up to checked `i64`; that is arithmetic validation, not a record quota. Existing larger valid manual-ledger balances are retained, not silently truncated or invalidated. |
| Rounding | Positive half-up to a whole rial at declared stages | Matches legacy positive quote line arithmetic. Negative journal amounts are not rounded; linked correction swaps exact debit/credit amounts or uses original allocations. |
| Discounts/returns | Stored line/document allocations, deterministic largest remainder; cumulative original-allocation return fractions with a final exact residual | MNY-04. Do not recompute old discounts from current prices or independently round each refund. |
| Valuation | Moving weighted average **per company/item/warehouse**, represented by remaining integer quantity and value | INV-03. This is the selected engineering default under user delegation, previously a guide candidate. It is not an approved industry-specific or statutory method. |
| Negative stock | Block a movement exceeding available on-hand quantity in the source warehouse | INV-04. Reservation remains separate. A transfer cannot borrow stock from another warehouse. |
| Ordering/backdating | Stock-changing postings use a durable company sequence and nondecreasing financial date; historical insertion before the latest posted company stock date is blocked | Same-date order is committed sequence, never UI row order, UUID ordering or wall-clock time. Reversals are new current/permitted-date events and still respect stock/locks. Opening must precede all normal activity. |
| Date/time | Persist Gregorian ISO civil dates; explicit Jalali entry/display; propose company timezone `Asia/Tehran` for Iran-oriented new companies | Timezone is visible and configurable before activity. No conversion of a stored civil date through UTC. Existing company dates/fiscal boundaries stay unchanged; a timezone change cannot move historical postings. Recorded-at timestamps remain distinct. |
| Tax | New core profile has tax calculation/submission **disabled**, with no automatic tax account/rate | Historical quotations retain their recorded user-configured tax and snapshots. “Disabled” does not assert exemption, compliance or correct tax treatment. Statutory/tax integrations are deferred. |

A new-company setup can offer the template below and its policy summary; users can customize names/codes and select validated role mappings. Existing custom books are opened unchanged. Template installation into an existing company is an explicit reviewed, additive setup command: no account renaming/remapping, no new financial balances and no silent second opening. Changing a referenced control mapping needs a migration/reconciliation design; version 1 blocks such changes after relevant postings instead of moving balances automatically.

## 2. Exact calculation, allocation and costing

**Order:** snapshot selected-unit quantity/price → round exact positive gross line value → subtract validated integer line discount → allocate document discount across eligible remaining line amounts → obtain line net/payable totals. Tax is disabled; any separately supported delivery charge is a visible, explicit charge line/account, not a hidden residual. Discounts cannot exceed the eligible subtotal. Zero-value stock still requires provenance; zero-price/free goods are an explicit supported operation, never introduced by relaxing the legacy quote price validator silently.

For a nonnegative rational amount `n/d`, calculate quotient and remainder with checked integers; add one rial exactly when `2 × remainder >= d`. A source selected-unit line computes `price_IRR × quantity_thousandths / 1000` once. Do not first round a carton price to a piece price: a 1,251-IRR carton of 12 pieces can retain its selected-unit price while stock receives exactly 12,000 base-unit thousandths. Its exact valuation ratio need not be stored as a rounded integer unit cost.

For document discount `D` and eligible rounded line weights `w_i`, assign `floor(D × w_i / sum(w))`; distribute the remaining rials by descending exact division remainder, breaking ties by immutable source line ID. Zero-weight lines receive zero. Persist these assignments. Fixture: weights 1,251 and 349, discount 101 → unrounded shares 78.969375 and 22.030625 → **79 and 22**, net **1,172 and 327**, total **1,499**. Equal weights 100/100/100 and discount 2 assign 1/1/0 by the recorded stable line-ID order.

For an original nonnegative allocation `A` over quantity `Q`, cumulative eligible returned quantity `q` receives cumulative credit `half_up(A × q / Q)`; the next return's amount is that cumulative target minus credits already committed. Final full return gets `A − already_credited`. Apply this independently to the stored original **net**, each original discount and attributable charges; reconstruct credited gross as credited net plus credited discounts, and payable as credited net plus credited charges. Do not round gross, discount and net independently or subtract independently rounded gross/discount credits into a negative incremental net credit. Every authoritative component has monotonic cumulative credit and exact final conservation; the derived gross preserves the original component relationship. Eligibility/over-return checks and remaining allocations commit atomically. Reject overlapping concurrent returns. Integer-amount limits and unit conversions do not waive this conservation rule.

**Inventory ratio:** each pool holds `Q` base-unit thousandths and `V` integer IRR. Incoming quantity `q` and attributable value `v` produce `(Q+q,V+v)`; do not persist a rounded average as the cost authority. For a partial issue, cost is `half_up(V × q / Q)` and the pool becomes `(Q−q,V−cost)`. When issuing all remaining quantity, consume **all remaining V** so no orphan value remains at zero quantity. Transfers issue that exact cost from the source and receive the identical cost at the target, atomically; total company value cannot change. A restockable sales return receives the original issue's stored attributable cost, allocated cumulatively/final-residual as above, rather than the current average. Damage/nonrestockable disposition has a distinct reason and expense command; never fabricate available stock.

At whole-rial precision a rounded partial issue can slightly change the remaining `V/Q` ratio. Version 1 deliberately carries that residual in the remaining pool and consumes it on final issue; it does not claim identical unit-rate precision or rounding behavior to ERPNext or a particular accounting standard. The ratio and the movement's actual integer cost must both be explainable in reports.

**Units:** new stock items have a stable base unit and explicit pack ratios. Version 1 uses exact positive rational conversion factors; a converted quantity must fit base-unit thousandths exactly or input is rejected. Example `1 carton = 12 pieces`; `0.125 carton = 1.5 pieces`. Indivisible item rules can further require whole pieces. Unit-factor revisions do not reinterpret earlier documents/openings. Ambiguous or conflicting imported units cannot be guessed from display labels. New quantities and conversion products must be range/overflow checked before mutation.

**Unsupported variance boundary:** the first connected posting slice supports supplier bills matched to receipt quantities at their frozen receipt value. A bill with a different capitalizable cost is blocked with its unresolved variance shown until an explicit versioned current-date cost-adjustment command is implemented and independently proved for remaining versus already consumed stock. Do not post AP at a wrong value, hide the difference in GRNI, silently rewrite old COGS or label late-cost handling complete. Purchase returns likewise require original receipt/value provenance and an explicit permitted original-cost adjustment; if current pooled stock cannot safely support that attributable value/quantity, block the operation until that adjustment workflow exists. This is a declared prerequisite for the guide's complete late-bill/purchase-return fixture, not a claim those workflows are shipped.

## 3. Customizable chart template and role constraints

Suggested template `sarv-trading-chart-v1` uses the following illustrative codes/Persian labels. Codes and labels are not identifiers or legal classifications. Five class roots (Assets, Liabilities, Equity, Revenue, Expense) are nonpostable; listed leaves are postable only through their allowed command. Stable role IDs, company IDs and immutable account IDs carry the meaning. Users may add ordinary accounts, but cannot bypass subledger controls by removing a UI flag.

| Suggested code / label | Stable role | Class / normal side | Posting boundary |
| --- | --- | --- | --- |
| 1101 / صندوق | cash | Asset / debit | Named cashbox movements/opening; no generic direct control journal |
| 1102 / بانک | bank | Asset / debit | Named bank movements/opening; account identity required |
| 1201 / حساب‌های دریافتنی | receivables | Asset / debit | Customer open-item/allocation effects required |
| 1202 / اسناد دریافتنی | cheques_receivable | Asset / debit | Identified received-cheque state/value effects required |
| 1301 / موجودی کالا | inventory | Asset / debit | Item/warehouse quantity and value effects required; warehouse mapping to this role or distinct validated inventory leaves |
| 1401 / پیش‌پرداخت تأمین‌کنندگان | supplier_advances | Asset / debit | Identified supplier advance/remaining allocation required |
| 1501 / دارایی‌های ثابت | fixed_assets | Asset / debit | Simple explicitly classified capital acquisition/manual adjustment; depreciation automation deferred |
| 2101 / حساب‌های پرداختنی | payables | Liability / credit | Supplier open-item/allocation effects required |
| 2102 / اسناد پرداختنی | cheques_payable | Liability / credit | Identified issued-cheque state/value effects required |
| 2201 / کالای دریافت‌شده، صورتحساب‌نشده | grni | Liability / credit | Identified received/unbilled quantity/value required |
| 2301 / پیش‌دریافت مشتریان | customer_advances | Liability / credit | Identified customer advance/remaining allocation required |
| 2401 / وام‌ها | loans | Liability / credit | Ordinary reviewed journal/loan command; automated schedules deferred |
| 3101 / سرمایه | owner_capital | Equity / credit | Explicit documented opening/capital contribution |
| 3102 / برداشت مالک | owner_drawings | Equity / debit | Explicit owner withdrawal; never sales expense |
| 3201 / سود انباشته | retained_earnings | Equity / credit | Historical opening or future reviewed closing command; no automatic new-year opening |
| 4101 / فروش | sales | Revenue / credit | Posted sales invoice; frozen net revenue |
| 4102 / برگشت از فروش | sales_returns | Revenue / debit | Linked credit from original sale allocation |
| 5101 / بهای تمام‌شده کالای فروش‌رفته | cogs | Expense / debit | Stock issue/restock effects required |
| 5201 / هزینه‌های عملیاتی | operating_expense | Expense / debit | Explicit expense bill/journal |
| 5202 / کارمزد بانکی | bank_fees | Expense / debit | Explicit fee with bank movement |
| 5301 / ضایعات و کسری موجودی | inventory_loss | Expense / debit | Explicit stock-loss quantity/value effect required |

No tax, automatic balancing/suspense, valuation-variance or close-transfer account is silently added. An unresolved opening imbalance is an error to resolve, not a generated equity plug. `owner_capital=952` below is a supplied synthetic fixture fact. Ordinary loan/asset/manual accounts remain customizable; role mappings validate company, classification, normal side, postability and archive state. Referenced accounts/parties/items/warehouses are archived, never erased. A party can have separate customer and supplier balances; do not net AR and AP merely because the same identity is used.

## 4. Posting matrix with tax disabled

All examples are artificial IRR values. The posting boundary commits source lifecycle, exact journal, stock/open-item effects, numbering, audit and request/event identity together. Validation/cancellation before commit leaves all unchanged; retries reproduce one economic result even with a different request key. Locks apply to every command and import.

| Event | Version 1 effect | Required distinct state/control |
| --- | --- | --- |
| Quote, order, reservation | No GL; no on-hand change | Reserved quantity is separate from on-hand |
| Stock receipt valued 1,500, not billed | Dr Inventory 1,500 / Cr GRNI 1,500 | Receipt item/warehouse/cost provenance; no AP yet |
| Bill matched to that receipt | Dr GRNI 1,500 / Cr AP 1,500 | Clear matched receipt value once; supplier open item 1,500; inventory unchanged |
| Direct stock purchase with combined receipt/bill | Dr Inventory / Cr AP | One command creates both receipt and supplier open item; cannot post a second receipt effect |
| Nonstock expense bill 349 | Dr Operating expense 349 / Cr AP 349 | Supplier open item; no stock |
| Capital acquisition bill | Dr Fixed assets / Cr AP | Explicit acquisition classification; no inventory/expense guess |
| Delivery with original cost 1,200 | Dr COGS 1,200 / Cr Inventory 1,200 | Quantity issue; exactly one issue event, paired atomically with same-date sale below |
| Sales invoice net 2,000 | Dr AR 2,000 / Cr Sales 2,000 | Customer open item; paired goods delivery cannot create duplicate COGS |
| Goods invoice with delivery, including partial order fulfillment | Both preceding sale and issue effects atomically on one financial date | Version 1 revenue trigger is completed delivery in this command; invoice-before-delivery and unbilled delivery remain unsupported rather than falsely recognized |
| Eligible half credit/restock | Dr Sales returns 1,000 / Cr AR or validated customer advance/credit 1,000; Dr Inventory 600 / Cr COGS 600 | Original quantities/net/issue costs; financial credit and physical restock are distinct supported effects |
| Later cash/bank receipt 1,000 | Dr Cash/Bank 1,000 / Cr AR 1,000 | Allocate existing customer open item; no new sales revenue |
| Later supplier payment 1,500 | Dr AP 1,500 / Cr Cash/Bank 1,500 | Allocate supplier open item; no second purchase |
| Warehouse transfer | Source inventory credit and target debit at identical issue cost | Total company quantity/value unchanged; same control account may aggregate both sides, movement links remain visible |

Cheque receipt/issue/clearance/bounce/replacement operational templates remain for the subsequent treasury milestone. Opening cheque evidence below supplies as-of states only, not external registration/clearance or authorization to perform live banking.

## 5. Independent opening fixture: exact 1,251 IRR

Use a fresh synthetic company `F2-core-v1`, fiscal start **1405/01/01 = 2026-03-21**, with opening as the beginning-of-day state before ordinary activity. Opening command records an as-of boundary plus sequence; it must not ambiguously overlap same-day later commands. Figures are manually chosen and summed here, independently of production calculations. These tiny values expose residual rials (`1,251 IRR = 125.1 toman`); they are not realistic business valuations.

### Supplied balances and detail

| Opening account | Debit IRR | Credit IRR | Independent detail required |
| --- | ---: | ---: | --- |
| Cash | 251 | 0 | Cashbox C1 counted funds 251 |
| Bank | 300 | 0 | Bank B1 book/statement ending balance 300; no invented reconciling difference |
| AR | 200 | 0 | Customer A invoice remainder 125; customer B invoice remainder 75 |
| Inventory | 300 | 0 | Warehouse W1 240 + W2 60, below |
| Cheques receivable | 200 | 0 | CR1 held/uncleared, amount 200; identified customer/source; no bank increase |
| AP | 0 | 149 | Supplier A bill remainder 100; supplier B bill remainder 49 |
| Cheques payable | 0 | 100 | CP1 issued/uncleared, amount 100; supplier/source; no bank decrease yet under this explicit cheque-control convention |
| GRNI | 0 | 50 | Receipt R1: five pieces in W2 received/unbilled, frozen value 50; included in inventory 60, never additional stock |
| Owner capital | 0 | 952 | Supplied owner opening capital; never an automatically calculated balancing account |
| **Totals** | **1,251** | **1,251** | Debits `251+300+200+300+200`; credits `149+100+50+952` |

CR1's historical 200 customer invoice was settled into the receivable-cheque control before cutoff; it is **not** another 200 open AR. CP1's historical 100 supplier bill was settled into payable-cheque control before cutoff; it is **not** another 100 open AP. Preserve external/original settlement references as opening provenance; do not recreate their old revenue, expense or stock effects. Bank B1 stays 300 until a later actual supported clearance event. Due dates, bank/holder/direction and current state are required for both cheque records; these balances are commitments/assets, not claimed bank clearance.

| Item/unit and warehouse | Input quantity | Base quantity thousandths | Value IRR | Exact starting ratio |
| --- | ---: | ---: | ---: | --- |
| A, base piece, W1 | 2 cartons × 12 pieces | 24,000 | 240 | 10 IRR/piece |
| A, base piece, W2 | 6 pieces | 6,000 | 60 | 10 IRR/piece |
| **Total** | **30 pieces** | **30,000** | **300** | W2 includes R1's five unbilled pieces/value 50 |

Required control equalities: `125+75 = AR 200`; `100+49 = AP 149`; `240+60 = inventory 300`; `CR1 200 = receivable cheques 200`; `CP1 100 = payable cheques 100`; `R1 50 = GRNI 50`; cashbox `251 = cash GL 251`; bank account `300 = bank GL 300 = stated fixture statement 300`. Every difference must be **zero**, individually and at the shared cutoff. P&L revenue/expense remains zero; net assets `1,251−(149+100+50)=952`, matching supplied equity. No additional opening revenue or COGS is manufactured.

Supplier/customer advance controls are explicitly zero in this fixture, with no advance records. Their eventual nonzero opening must supply party/provenance/remaining allocations and independently reconcile; an absent unsupported register cannot be displayed as a verified zero for real imported books.

### Independent 349 adjustment and linked reversal

Keep the existing single-cash-account 1,251/349 manual-kernel fixture unchanged in its separate company. For this richer opening fixture, on **2026-03-22** add documented owner funds Dr Cash **349** / Cr Owner capital **349**. Expected cash **600**, capital **1,301**, total debit/credit balances **1,600**; other controls unchanged. On **2026-03-23** reverse that exact journal with reason/link: cash returns to **251**, capital to **952**, total balances **1,251**. A report cutoff on March 22 still shows **1,600** after the reversal is recorded; March 23 shows **1,251**. Reversal is not an opening reset.

### Additional independent moving-average/return acceptance fixture

In a separate fresh fixture branch starting from the opening, receive **one carton = 12 pieces** into W1 for **180 IRR**, unbilled: W1 becomes **36 pieces / 420 IRR**, W2 remains **6 / 60**, total inventory **480**, GRNI **230**. A matched supplier bill **180** moves GRNI to **50**, AP to **329**, with stock still **480**. These are later connected-workflow acceptance values, not part of the opening journal or the 349 adjustment fixture.

Transfer **6 pieces** W1→W2: source cost `420 × 6 / 36 = 70`; W1 **30 / 350**, W2 **12 / 130**, total still **42 / 480**. Issue **5 pieces** from W1: exact ratio `350 × 5 / 30 = 58⅓`, positive half-up cost **58**; W1 **25 / 292**, W2 **12 / 130**, inventory **422**, COGS **58**. Return the original issue in two restockable parts: **2 pieces** gives cumulative `58 × 2 / 5 = 23.2` → **23**, W1 **27 / 315**, inventory **445**, net COGS **35**; final **3 pieces** restores residual **35**, W1 **30 / 350**, total inventory **480**, net COGS **0**. Exact issue/return history remains visible; current averages are not used to recost the return.

## 6. Opening transaction, existing books and acceptance dependencies

Opening stages independent ledger balances and related records in a preview, with original row/reference, normalized explicit unit and per-row errors. One owner-issued command validates company, fiscal boundary, policy/mappings, unique IDs, source lineage, balances and all control equalities before committing **one atomic opening** with its open items/stock/GRNI/cash/bank/cheque records and audit. Retrying identical content returns the existing opening; changed content with the same key fails; different keys cannot create a second company initialization. Cancelled/invalid staging leaves no financial effects.

Existing cumulative books never receive another opening because a new fiscal year or policy revision begins. Preserve current posted opening/history and custom mappings. If old manual-ledger books later gain subledgers, use a separately designed **attach/reconcile existing balance** migration: an as-of analysis, explicit mapping and provenance, zero additional net GL posting, tested backups and reviewable difference report. Do not use the fresh-company opening command to fabricate missing history or silently double balances. Conflicting/corrective openings require linked correction policy and a complete reconciliation; version 1 must reject them until that workflow is implemented.

The next bounded implementation is policy types/settings/snapshots, role-validated chart setup, staged exact opening and minimal immutable balance registers; root owns schema/migrations. Required dependencies before full opening acceptance:

1. Stable company/party/item/unit/warehouse/cashbox/bank/cheque/receipt identities and original opening provenance; independent AR/AP roles for dual-role parties.
2. Typed immutable policy/mapping versions, checked arithmetic/allocation, current account classification/normal-side constraints and archive behavior.
3. Atomic opening command and supported control effects. Current `control: bool` blocks manual control postings but does **not** implement their subledgers; adding menu items or setting a flag cannot close this gap.
4. Exact as-of control reports, persisted uniqueness/idempotency, fiscal locks, preserved source/reversal history and concurrent same/different-key retry proof.
5. Versioned transactional upgrade with verified pre-upgrade backup; old quote IDs/totals/tax snapshots/output unchanged; fresh-profile restore/export reconstructs role/policy/open-item/quantity/value/cheque details and checksums.
6. Native staging/preview/errors/commit/restart proof, followed by independent technical review against literal expected figures above. Import mismatch of one rial, unknown pack factor, duplicated invoice/cheque/receipt, wrong company, locked period and ambiguous cutoff must reject without partial state.

Complete late-cost variance, purchase returns, fulfillment/recognition timing, advances, operational cheque lifecycles and closing remain separate connected-workflow dependencies. Do not claim all G1/G3/OPEN-01 accepted merely because the chart or this proposal exists. The guide explicitly calls for professional review before production use; no such review has occurred. The user's delegation permits implementing visible engineering defaults and synthetic tests now without repeatedly asking for policy selections; it does not create a professional or legal acceptance claim. External services, new licenses, credentials, payments, live books and public deployment remain outside this proposal.

## 7. Research and guide ambiguities resolved

Sources checked through read-only unauthenticated HTTPS on **2026-10-07**. These are workflow references, not copied implementation, a license selection or Iranian legal/accounting authority:

| Source | Refreshed observation | Application and limit |
| --- | --- | --- |
| [Sarv blueprint](Sarv-Accounting-Requirements.md), §§6–12 | Exact checked amounts, reviewed event matrix, complete reconciled opening, original-cost returns, negative-stock default constraint and candidate moving average | Normative product contract. It does not specify warehouse pooling, concrete chart codes, tie-breaks, pack precision or a full late-cost adjustment algorithm. Choices above are visible engineering decisions. |
| [Sarv execution brief](Sarv-Agent-Execution-Brief.md) | Bounded workflow, preserve quotations, independent expected fixtures, one schema owner, no silent license/funding choice | No code/schema changes in this document task. Latest user direction takes precedence over earlier pending-choice language; acceptance gaps stay honest. |
| [ERPNext perpetual inventory help](https://docs.frappe.io/erpnext/perpetual-inventory) — HTTP 200 | “accounting entry is done for every stock transaction”; receipt debits inventory and credits Stock Received But Not Billed; bill clears that account; delivery debits COGS and credits inventory; warehouse/account links documented | E1 public project workflow, version not pinned by live page. Supports separate receipt/bill/issue effects and inventory accounts; its taxes/FIFO examples are not Sarv defaults. |
| [ERPNext opening stock help](https://docs.frappe.io/erpnext/opening-stock) — HTTP 200 | Opening stock describes both amount and value; warehouses/accounting links are prerequisites; opening entry uses item, quantity and rate | E1 workflow supports quantity/value and warehouse evidence. Its closing-to-opening description is not permission to duplicate Sarv cumulative GL balances. |
| [ERPNext FIFO/moving average help](https://docs.frappe.io/erpnext/fifo-and-moving-average) — HTTP 200 | Moving average uses a blended ratio per item/warehouse; receipts change that ratio; historical insertions can change subsequent valuations and require forward reposting | E1 mathematical/workflow reference supports the pooling choice and conservative backdate restriction. Sarv explicitly specifies its own whole-rial residual convention; vendor legal/standard-acceptance statements are not adopted. |
| [ERPNext opening balance help](https://docs.frappe.io/erpnext/opening-balance) and [opening invoice tool](https://docs.frappe.io/erpnext/opening-invoice-creation-tool) — HTTP 200 each | Opening reproduces a cutover position; invoice-level unpaid balances preserve due dates/allocations; exclude controls already created by opening documents from any additional opening journal | E1 reference supports exact detailed control reconciliation and no double count. Sarv stages and commits the complete fresh opening atomically rather than copying the vendor's separate temporary-opening workflow. |
| [ERPNext immutable ledger help](https://docs.frappe.io/erpnext/immutable-ledger-in-erpnext) — HTTP 200 | Original/reversal rows and follow-up corrections remain traceable; current builds can allow backdating with controlled reposting and period validation | E1 reference supports retained history and explicit boundaries. Sarv version 1 deliberately blocks earlier stock dates; the source does not establish that every ERPNext backdate is prohibited. |
| Existing local quotation/kernel contracts | `src/domain.rs`: 1,000ths, source limits, positive half-up; `src/accounting.rs`: checked integer IRR/custom chart/control restrictions; `tests/accounting_kernel.rs`: 1,251/349/cutoff/reversal/second-opening guard | Observed source behavior, not proof of future policy/subledgers. This proposal preserves existing snapshots and extends fixture expectations independently. |

Older guessed ERPNext stock-valuation/opening-invoice/immutable-ledger paths returned **404**; corrected paths discovered through official documentation links succeeded as above. Zoho US opening-balance help returned **403** during this bounded refresh, establishing no new facts. Moving-average pooling, rounding/residuals, no-negative stock, date sequencing, tax-disabled defaults and role codes are Sarv choices explicitly recorded here, not assertions that all vendors use them. No paid or authenticated service was used.
