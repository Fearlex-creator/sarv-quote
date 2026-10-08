# Authoritative M4 API — procurement.rs (supersedes old trade.rs design)

Root owns AccountingState.procurement/schema/audit/posting capability. All following types in pub mod procurement; serde strict/equality. Money is existing integerIRR. Commands execute on AccountingState; clone failureatomic; Store adds durableCAS. Transfers ALWAYS nonfinancial paired stock movements, no Journal. All zeroamount operational effects likewise nonfinancial. Prefix const PROCUREMENT_EVENT_PREFIX = "procurement.".

```rust
ProcurementState { records: Vec<ProcurementRecord>, requests: Vec<ProcurementRequest> }
// Default, is_empty(), validate_transition(&self,next:&Self)->Result<()> immutable prefixes.
ProcurementContext { company_id:String, period_id:String, setup_id:String,
 source_id:String, source_revision:u64, date:String, request_key:String, actor:String, reason:String }
RegisterResources { context:ProcurementContext, parties:Vec<CoreParty>, units:Vec<CoreUnit>,
 items:Vec<CoreItem>, warehouses:Vec<CoreWarehouse> }
PurchaseOrder { context:ProcurementContext, supplier_id:String, reference:String,
 document_date:String, lines:Vec<OrderLine>, document_discount:Money, supplied_total:Money }
OrderLine { id:String, item_id:String, warehouse_id:String, unit_id:String,
 quantity_thousandths:i64, unit_price:Money, line_discount:Money,
 price_reference:String, price_date:String, override_reason:Option<String> }
ReceiveOrder { context:ProcurementContext, order_id:String, reference:String,
 document_date:String, lines:Vec<ReceiveLine>, supplied_total:Money }
ReceiveLine { id:String, order_line_id:String, warehouse_id:String, unit_id:String,
 quantity_thousandths:i64 } // exact conversion using frozen order item ratios; cumulative original PO net allocation
MatchBill { context:ProcurementContext, supplier_id:String, reference:String,
 document_date:String, due_date:String, lines:Vec<BillLine>, supplied_total:Money }
BillLine { id:String, receipt_line_id:String, quantity_thousandths:i64, supplied_value:Money }
// BillLine quantity is BASE-unit thousandths, source either receipt line or openingGRNI id.
TransferStock { context:ProcurementContext, item_id:String, source_warehouse_id:String,
 target_warehouse_id:String, unit_id:String, quantity_thousandths:i64 }
enum ProcurementCommand { Resources(RegisterResources), Order(PurchaseOrder),
 Receive(ReceiveOrder), Bill(MatchBill), Transfer(TransferStock) }
// ProcurementCommand.context()->&ProcurementContext public helper.

FrozenOrderLine { input:OrderLine, item:CoreItem, base_quantity_thousandths:i64,
 gross:Money, document_discount:Money, net:Money }
PostedOrder { supplier:CoreParty, lines:Vec<FrozenOrderLine>, total:Money }
PostedReceiptLine { id:String, order_line_id:String, item_id:String, warehouse_id:String,
 quantity_thousandths:i64, value:Money } // BASE quantity, frozen proportional PO cost
PostedReceipt { order_id:String, supplier_id:String, lines:Vec<PostedReceiptLine>, total:Money }
PostedBillLine { id:String, receipt_line_id:String, quantity_thousandths:i64, value:Money }
PostedBill { supplier_id:String, lines:Vec<PostedBillLine>, total:Money }
StockPool { item_id:String, warehouse_id:String, quantity_thousandths:i64, value:Money }
StockDirection { In, Out }
StockMovement { id:String, item_id:String, warehouse_id:String, direction:StockDirection,
 quantity_thousandths:i64, value:Money, source_line_id:String,
 before:StockPool, after:StockPool }
PostedTransfer { item_id:String, quantity_thousandths:i64, value:Money }
enum ProcurementData { Resources(RegisterResources), Order(PostedOrder),
 Receive(PostedReceipt), Bill(PostedBill), Transfer(PostedTransfer) }
ProcurementRecord { id:String, command:ProcurementCommand, command_sha256:String,
 frozen_setup:CoreSetup, sequence:u64, journal_id:Option<String>,
 data:ProcurementData, movements:Vec<StockMovement> }
// record.uses_nonfinancial_key()->bool = journal_id.is_none().
// id is stable source/event record ID (distinct from JID); nonfinancial audit entity_id=id.
ProcurementRequest { request_key:String, company_id:String, record_id:String,
 business_sha256:String } // full command digest except cleared request key; all aliases, financial or nonfinancial
ProcurementResult { record_id:String, journal_id:Option<String>, sequence:u64 }
ProcurementPreview { data:ProcurementData, movements:Vec<StockMovement>,
 total:Money, journal_lines:Vec<JournalLineInput>, nonfinancial:bool }

ReceiptRemainder { id:String, source_id:String, supplier_id:String, item_id:String,
 warehouse_id:String, quantity_thousandths:i64, value:Money,
 original_quantity_thousandths:i64, original_value:Money, date:String, reference:String }
PayableRemainder { id:String, source_id:String, party_id:String, reference:String,
 document_date:String, due_date:String, remaining:Money }
ProcurementControl { role:CoreRole, account_id:String, ledger:Money, detail:Money, difference:Money }
ProcurementReconciliation { company_id:String, as_of:String, controls:Vec<ProcurementControl>,
 stock:Vec<StockPool>, receipts:Vec<ReceiptRemainder>, payables:Vec<PayableRemainder>,
 total_debit:Money, total_credit:Money, reconciled:bool }

AccountingState::preview_procurement(&ProcurementCommand)->Result<ProcurementPreview>
AccountingState::post_procurement_command(ProcurementCommand)->Result<ProcurementResult>
AccountingState::procurement_resources(company_id:&str)->Result<RegisterResources>
AccountingState::procurement_document(record_id:&str)->Result<&ProcurementRecord>
AccountingState::procurement_reconcile(company_id:&str,as_of:&str)->Result<ProcurementReconciliation>
pub(crate) AccountingState::replay_procurement_record(&ProcurementRecord)->Result<Option<String>>
pub(crate) AccountingState::replay_procurement_request(&ProcurementRequest)->Result<ProcurementResult>
```

Replay contract: financial calls complete command, produces one JournalPosted audit; nonfinancial one ProcurementRecorded audit. Helper returns generated JID Option, preserves expectedrecord id by replacing generated record id and corresponding typed request.record_id; normalfinancial request JID + record.journal_id root replaces tohistorical. Root replaces generated audit metadata. Replay of typed aliases clones immutable full original command with aliaskey, validates business hash/global namespace, returns same record/effect without second audit; original request order restored afterreplay likeM3.

Resource/query snapshots seed immutable opening CoreParty/CoreUnit/CoreItem/CoreWarehouse definitions plus appended validated Resources records; no modifying opening. Frozen order item ratios authoritative for partial receipt conversion. Parties dualroles preserved. Alias collision against M3zero/normalrequests rejected except paired normalfinancial+typed alias for same validated procurement effect.

Natural business identity(company,source_id,event) one effect across ALL revisions; source_revision doesn't authorize another posted effect. Same/diff key exactfullcommand one effect; changedfullcontent evenquantity/due/provenance-only rejects. Supplied line/doc costs must equal frozen cumulative PO/receipt allocation; variance rejected per-line, never guessed. Duplicate external supplier invoice references additionally strict-reject at samecompany/supplier/reference/docdate until explicit duplicate-review route later; distinct source IDs don't evade duplicate detection.

PO immutable nonfinancial snapshot. Order originalQ,V -> partial receipt cost half_up(V*cumulativeReceivedQ/Q)-alreadyReceivedValue, final consumesresidual. Receipt originalQ,V -> partial bill same cumulative/final formula. OpeningGRNI billable only from opening date; no receipt/stock recreation. AP+stock+GRNI asof end-of-day seededopening+dated posted records; opening report unchanged. Stockchanging date >=latestcompany stockdate; billdate >=sourceavailability andlatestallocationdate for every source. Supplier/stock master creations append-only. Generic operational reversal/correction blocked; minimum safe linked cancel deferred until separately evidenced, drafts managed native/app layer initially.

Literalmain: M3Inv300 AP149 GRNI50; PO12pieces@15=180 (unitqty12000). R-A5pieces/75 => Inv375 GRNI125; R-B7/105 =>Inv480 GRNI230, W136/420 W26/60. BillA2ofR-A/30 =>AP179 GRNI200. BillB3ofR-A/45 +7ofR-B/105 total150 =>AP329 GRNI50. Transfer6W1→W2 cost70 =>W130/350 W212/130; total42/480, noJournal. Optional billopeningR1 five/50 =>AP379 GRNI0 stockunchanged. Cash251 bank300 AR200 CR200 CP100 capital952 untouched; P&L/COGS0. Totals1431/1431 afterbothreceipts andthroughmatching/transfer. PureStockfreezero events neveremptyJ.

Scope only resources/PO/partialreceipts/partialexact-costbills/transfer/asofcontrols/native/export/migration/retry. Directexpense/asset routes, actual purchase returns/costvariance corrections, sales/returns, treasury/cheques and closing remain explicit nextscope; no completePUR/INV/SAL gateclaim. Schema5 root-owned. Newdependencies/livebooks/license/externalservices excluded.
