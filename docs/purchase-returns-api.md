# Authoritative M6 purchase-return API

Prepared2026-10-08 against M5 eb8f8c29648fd4a206a26d65ae4eaca5cf945a1b. Root owns shared schema7/Store/migration/integration; kernel owns procurement.rs. Native/export owners use this contract. Existing core-policy-v1 permits original-cost purchase returns only when attributable quantity/value can safely leave the current pool. No variance account, tax, settlement/refund or historical recosting is selected.

## Commands and frozen records

Strict serde/equality types in procurement, using existing Money/ProcurementContext:

    PurchaseReturnLine { id:String, receipt_line_id:String, bill_line_id:Option<String>,
        quantity_thousandths:i64, supplied_value:Money }
    ReturnPurchase { context:ProcurementContext, supplier_id:String,
        return_id:String, return_reference:String, credit_id:Option<String>,
        credit_reference:Option<String>, document_date:String,
        lines:Vec<PurchaseReturnLine>, supplied_total:Money }
    ProcurementCommand::Return(ReturnPurchase) // procurement.return.v1
    PurchaseReturnState::Returned
    SupplierCreditState::Posted
    PostedPurchaseReturnLine { id:String, receipt_line_id:String,
        bill_line_id:Option<String>, item_id:String, warehouse_id:String,
        quantity_thousandths:i64, value:Money }
    PostedPurchaseReturn { supplier_id:String, return_id:String,
        return_state:PurchaseReturnState, credit_id:Option<String>,
        credit_state:Option<SupplierCreditState>, receipt_id:Option<String>,
        bill_id:Option<String>, lines:Vec<PostedPurchaseReturnLine>, total:Money }
    ProcurementData::Return(PostedPurchaseReturn)
    PurchaseReturnableLine { receipt_line_id:String, bill_line_id:Option<String>,
        bill_id:Option<String>, supplier_id:String, item_id:String,
        warehouse_id:String, original_quantity_thousandths:i64,
        original_value:Money, remaining_quantity_thousandths:i64,
        remaining_value:Money, date:String, reference:String }
    AccountingState::procurement_receipt_returnable(company_id:&str,
        receipt_line_id:&str) -> Result<PurchaseReturnableLine>
    AccountingState::procurement_bill_returnable(company_id:&str,
        bill_line_id:&str) -> Result<PurchaseReturnableLine>

Quantity input is positive BASE-unit thousandths. Existing preview/post/document/reconcile APIs accept Return. Return/credit identities are distinct from record/journal/line/movement IDs and globally unique. Billed zero-value returns still retain a credit identity/state without an empty journal. Return/credit posting states are persisted and replay-validated; preview is never posting. receipt_id/bill_id are original operational record IDs, present exclusively for their mode.

## Eligibility and allocation

One command uses one mode. Unbilled lines have no bill_line_id and no credit metadata and all belong to one operational receipt. Billed lines all belong to one bill, retain exact receipt correspondence and require both credit ID/reference. Mixed modes, mixed original bills/receipts, inferred mappings and opening-only GRNI/AP physical returns reject. A bill may contain multiple operational receipts; each return keeps the original warehouse.

Unbilled returns and bills share ONE cumulative original receipt quantity/value cursor. Receipt remainder excludes both. Allocate positive half-up(original_value × cumulative_consumed_quantity / original_quantity) minus already_consumed_value, with final exact residual. Bill matching must use this same cursor after an unbilled return. Billed returns consume only their specific frozen bill-line cursor and never restore GRNI eligibility. Neither mode reopens received PO quantity. Queries expose original and remaining cursor parameters; UI computes supplied amounts exactly, showing original cost rather than current average.

Tiny example receipt3pieces/value1: bill1/value0 → unbilled return1/value1 → final bill1/value0. Returning the first bill credits0 rather than another share of the receipt's1rial. Operational quantity/identity remains even with no monetary journal.

Outgoing original-cost movements require available quantity AND value in the original warehouse. Removing all pool quantity must consume all pool value. Insufficient value or orphan value rejects atomically with a useful unresolved-adjustment message. No balancing difference account, warehouse borrowing, backdating or earlier COGS rewrite. This remains partial INV/PUR pending a separately proved adjustment workflow.

## Financial and integrity boundary

Unbilled: Dr GRNI / Cr Inventory. Billed: Dr AP / Cr Inventory, reducing only the linked original bill remainder. Credit cannot exceed unpaid eligible bill remainder; no supplier advance/unapplied credit is invented. Preserve source snapshots and sales revenue/returns/COGS. Omit zero monetary lines; retain nonfinancial operational effects. Inventory/AP/GRNI and historical cutoffs reconcile individually; unavailable detail is an error rather than verified zero.

Reuse typed procurement all-revision natural identity (company,source,event), full command hash except request key, global requests/IDs, actor/reason, CAS and immutable replay. Identical same/different-key retry returns the original effect after lock; changed content/revision, reused IDs or generic typed reversal rejects. Exact normalized supplier return-reference/date and credit-reference/date checks prevent a new source ID bypassing duplicate paper references; no fuzzy merging.

Financial date must respect company/period/opening, latest company stock date, source receipt/bill availability and latest allocation on the affected streams. Paper document_date is separate and no later than financial date. Return/credit commit on one financial date. One detached worker atomically commits source/journal/movement/requests/numbering/audit; lost receipts cannot create a second effect.

## Main literal fixture

M5 March31: W1A25pieces/292IRR,W2A12/130; Inv422,AR299,AP329,GRNI50,Sales199,salesReturns100,COGS58,earnings41,TB1630/1630. Original R-B7/105 is billed on bill150 alongside another45 line.

| New event after March31 | W1 quantity/value | Inventory | AP | Linked bill remainder | TB debit/credit |
| --- | --- | ---: | ---: | ---: | --- |
| Billed return3/original credit45 |22/247|377|284|105|1585/1585|
| Billed return final4/credit60 |18/187|317|224|45|1525/1525|

W2,AR,GRNI,Sales,salesReturns,COGS and earnings are unchanged. Independent reviewer supplies unbilled/interleaved/tiny/fault oracles. Native proof covers source selection/search, exact preview/commit/retry/error retention, drill-down, rial/toman/Jalali, narrow/standard light/dark, keyboard focus, restart/export/restored equality. Preserve M5 quotes/sales and M4 purchases. Evidence must name its actual frozen source/binary. Treasury, purchase drafts, price variance, closing/permissions, full performance/platform/professional/statutory gates remain partial/missing/untested.
