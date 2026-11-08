//! Exact, linked procurement and stock transfers. Tax/cost variance/treasury are unsupported.
use crate::accounting::{
    AccountingState, AuditOperation, BalanceSide, JournalKind, JournalLineInput, Money, PostCommand,
};
use crate::core::{
    CoreItem, CoreParty, CoreRole, CoreSetup, CoreUnit, CoreWarehouse, MAX_SOURCE_QUANTITY,
    civil_date, positive_half_up,
};
use crate::domain::{MAX_MONEY, id, normalize, text};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
pub const PROCUREMENT_EVENT_PREFIX: &str = "procurement.";
macro_rules! record {($n:ident {$($f:ident:$t:ty),* $(,)?})=>{
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)] #[serde(deny_unknown_fields)] pub struct $n {$(pub $f:$t),*}
};}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcurementState {
    pub records: Vec<ProcurementRecord>,
    pub requests: Vec<ProcurementRequest>,
}
impl ProcurementState {
    pub fn is_empty(&self) -> bool {
        self.records.is_empty() && self.requests.is_empty()
    }
    pub fn validate_transition(&self, next: &Self) -> Result<()> {
        ensure!(
            next.records.starts_with(&self.records) && next.requests.starts_with(&self.requests),
            "سابقهٔ خرید و انبار قابل تغییر یا حذف نیست"
        );
        Ok(())
    }
    pub fn uses_nonfinancial_key(&self, key: &str) -> bool {
        self.requests.iter().any(|r| {
            r.request_key == key
                && self
                    .records
                    .iter()
                    .any(|p| p.id == r.record_id && p.journal_id.is_none())
        })
    }
}
record!(ProcurementContext {
    company_id: String,
    period_id: String,
    setup_id: String,
    source_id: String,
    source_revision: u64,
    date: String,
    request_key: String,
    actor: String,
    reason: String
});
record!(RegisterResources{context:ProcurementContext,parties:Vec<CoreParty>,units:Vec<CoreUnit>,items:Vec<CoreItem>,warehouses:Vec<CoreWarehouse>});
record!(OrderLine{id:String,item_id:String,warehouse_id:String,unit_id:String,quantity_thousandths:i64,unit_price:Money,line_discount:Money,price_reference:String,price_date:String,override_reason:Option<String>});
record!(PurchaseOrder{context:ProcurementContext,supplier_id:String,reference:String,document_date:String,lines:Vec<OrderLine>,document_discount:Money,supplied_total:Money});
record!(ReceiveLine {
    id: String,
    order_line_id: String,
    warehouse_id: String,
    unit_id: String,
    quantity_thousandths: i64
});
record!(ReceiveOrder{context:ProcurementContext,order_id:String,reference:String,document_date:String,lines:Vec<ReceiveLine>,supplied_total:Money});
record!(BillLine {
    id: String,
    receipt_line_id: String,
    quantity_thousandths: i64,
    supplied_value: Money
});
record!(MatchBill{context:ProcurementContext,supplier_id:String,reference:String,document_date:String,due_date:String,lines:Vec<BillLine>,supplied_total:Money});
record!(PurchaseReturnLine { id:String, receipt_line_id:String, bill_line_id:Option<String>, quantity_thousandths:i64, supplied_value:Money });
record!(ReturnPurchase { context:ProcurementContext, supplier_id:String, return_id:String, return_reference:String, credit_id:Option<String>, credit_reference:Option<String>, document_date:String, lines:Vec<PurchaseReturnLine>, supplied_total:Money });
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PurchaseReturnState {
    Returned,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupplierCreditState {
    Posted,
}
record!(TransferStock {
    context: ProcurementContext,
    item_id: String,
    source_warehouse_id: String,
    target_warehouse_id: String,
    unit_id: String,
    quantity_thousandths: i64
});
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcurementCommand {
    Resources(RegisterResources),
    Order(PurchaseOrder),
    Receive(ReceiveOrder),
    Bill(MatchBill),
    Transfer(TransferStock),
    Return(ReturnPurchase),
}
impl ProcurementCommand {
    pub fn context(&self) -> &ProcurementContext {
        match self {
            Self::Resources(c) => &c.context,
            Self::Order(c) => &c.context,
            Self::Receive(c) => &c.context,
            Self::Bill(c) => &c.context,
            Self::Transfer(c) => &c.context,
            Self::Return(c) => &c.context,
        }
    }
    fn context_mut(&mut self) -> &mut ProcurementContext {
        match self {
            Self::Resources(c) => &mut c.context,
            Self::Order(c) => &mut c.context,
            Self::Receive(c) => &mut c.context,
            Self::Bill(c) => &mut c.context,
            Self::Transfer(c) => &mut c.context,
            Self::Return(c) => &mut c.context,
        }
    }
    pub fn event(&self) -> &'static str {
        match self {
            Self::Resources(_) => "procurement.resources.v1",
            Self::Order(_) => "procurement.order.v1",
            Self::Receive(_) => "procurement.receive.v1",
            Self::Bill(_) => "procurement.bill.v1",
            Self::Transfer(_) => "procurement.transfer.v1",
            Self::Return(_) => "procurement.return.v1",
        }
    }
}
record!(FrozenOrderLine {
    input: OrderLine,
    item: CoreItem,
    base_quantity_thousandths: i64,
    gross: Money,
    document_discount: Money,
    net: Money
});
record!(PostedOrder{supplier:CoreParty,lines:Vec<FrozenOrderLine>,total:Money});
record!(PostedReceiptLine {
    id: String,
    order_line_id: String,
    item_id: String,
    warehouse_id: String,
    quantity_thousandths: i64,
    value: Money
});
record!(PostedReceipt{order_id:String,supplier_id:String,lines:Vec<PostedReceiptLine>,total:Money});
record!(PostedBillLine {
    id: String,
    receipt_line_id: String,
    quantity_thousandths: i64,
    value: Money
});
record!(PostedBill{supplier_id:String,lines:Vec<PostedBillLine>,total:Money});
record!(PostedPurchaseReturnLine { id:String, receipt_line_id:String, bill_line_id:Option<String>, item_id:String, warehouse_id:String, quantity_thousandths:i64, value:Money });
record!(PostedPurchaseReturn { supplier_id:String, return_id:String, return_state:PurchaseReturnState, credit_id:Option<String>, credit_state:Option<SupplierCreditState>, receipt_id:Option<String>, bill_id:Option<String>, lines:Vec<PostedPurchaseReturnLine>, total:Money });
record!(PurchaseReturnableLine { receipt_line_id:String, bill_line_id:Option<String>, bill_id:Option<String>, supplier_id:String, item_id:String, warehouse_id:String, original_quantity_thousandths:i64, original_value:Money, remaining_quantity_thousandths:i64, remaining_value:Money, date:String, reference:String });
record!(StockPool {
    item_id: String,
    warehouse_id: String,
    quantity_thousandths: i64,
    value: Money
});
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StockDirection {
    In,
    Out,
}
record!(StockMovement {
    id: String,
    item_id: String,
    warehouse_id: String,
    direction: StockDirection,
    quantity_thousandths: i64,
    value: Money,
    source_line_id: String,
    before: StockPool,
    after: StockPool
});
record!(PostedTransfer {
    item_id: String,
    quantity_thousandths: i64,
    value: Money
});
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcurementData {
    Resources(RegisterResources),
    Order(PostedOrder),
    Receive(PostedReceipt),
    Bill(PostedBill),
    Transfer(PostedTransfer),
    Return(PostedPurchaseReturn),
}
record!(ProcurementRecord{id:String,command:ProcurementCommand,command_sha256:String,frozen_setup:CoreSetup,sequence:u64,journal_id:Option<String>,data:ProcurementData,movements:Vec<StockMovement>});
impl ProcurementRecord {
    pub fn uses_nonfinancial_key(&self) -> bool {
        self.journal_id.is_none()
    }
}
record!(ProcurementRequest {
    request_key: String,
    company_id: String,
    record_id: String,
    business_sha256: String
});
record!(ProcurementResult{record_id:String,journal_id:Option<String>,sequence:u64});
record!(ProcurementPreview{data:ProcurementData,movements:Vec<StockMovement>,total:Money,journal_lines:Vec<JournalLineInput>,nonfinancial:bool});
record!(ReceiptRemainder {
    id: String,
    source_id: String,
    supplier_id: String,
    item_id: String,
    warehouse_id: String,
    quantity_thousandths: i64,
    value: Money,
    original_quantity_thousandths: i64,
    original_value: Money,
    date: String,
    reference: String
});
record!(PayableRemainder {
    id: String,
    source_id: String,
    party_id: String,
    reference: String,
    document_date: String,
    due_date: String,
    remaining: Money
});
record!(ProcurementControl {
    role: CoreRole,
    account_id: String,
    ledger: Money,
    detail: Money,
    difference: Money
});
record!(ProcurementReconciliation{company_id:String,as_of:String,controls:Vec<ProcurementControl>,stock:Vec<StockPool>,receipts:Vec<ReceiptRemainder>,payables:Vec<PayableRemainder>,total_debit:Money,total_credit:Money,reconciled:bool});

fn hash<T: Serialize>(v: &T) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(v)?)))
}
fn business_hash(c: &ProcurementCommand) -> Result<String> {
    let mut c = c.clone();
    c.context_mut().request_key.clear();
    hash(&c)
}
fn result(r: &ProcurementRecord) -> ProcurementResult {
    ProcurementResult {
        record_id: r.id.clone(),
        journal_id: r.journal_id.clone(),
        sequence: r.sequence,
    }
}
fn money(v: Money) -> Result<()> {
    ensure!(
        (0..=MAX_MONEY).contains(&v.rials()),
        "مبلغ خرید خارج از محدوده یا منفی است"
    );
    Ok(())
}
fn doc_date(date: &str, financial: &str) -> Result<()> {
    civil_date(date)?;
    ensure!(date <= financial, "تاریخ سند پس از تاریخ مالی است");
    Ok(())
}
fn base(item: &CoreItem, unit: &str, q: i64) -> Result<i64> {
    ensure!(
        (1..=MAX_SOURCE_QUANTITY).contains(&q),
        "مقدار باید مثبت و در محدوده باشد"
    );
    let (n, d) = if unit == item.base_unit_id {
        (1, 1)
    } else {
        let p = item
            .packs
            .iter()
            .find(|p| p.unit_id == unit)
            .ok_or_else(|| anyhow::anyhow!("ضریب بسته‌بندی مشخص نیست"))?;
        (p.numerator, p.denominator)
    };
    ensure!(n > 0 && d > 0, "نسبت واحد نامعتبر است");
    let v = i128::from(q) * i128::from(n);
    ensure!(v % i128::from(d) == 0, "تبدیل در هزارم واحد پایه دقیق نیست");
    let v = i64::try_from(v / i128::from(d))?;
    ensure!(
        v > 0 && (!item.indivisible || v % 1000 == 0),
        "مقدار واحد پایه غیرقابل‌تقسیم است"
    );
    Ok(v)
}
fn sum<I: IntoIterator<Item = Money>>(values: I) -> Result<Money> {
    values
        .into_iter()
        .try_fold(Money::ZERO, |a, b| a.checked_add(b))
}
pub(crate) fn allocation(
    v: Money,
    original_q: i64,
    previous_q: i64,
    previous_v: Money,
    new_q: i64,
) -> Result<Money> {
    ensure!(
        original_q > 0 && new_q > 0 && previous_q >= 0,
        "مقدار تخصیص نامعتبر است"
    );
    let cumulative = previous_q
        .checked_add(new_q)
        .ok_or_else(|| anyhow::anyhow!("سرریز مقدار تخصیص"))?;
    ensure!(cumulative <= original_q, "مقدار از ماندهٔ منبع بیشتر است");
    let target = if cumulative == original_q {
        v
    } else {
        Money::from_rials(positive_half_up(
            i128::from(v.rials()) * i128::from(cumulative),
            i128::from(original_q),
        )?)
    };
    let next = target.checked_sub(previous_v)?;
    ensure!(next.rials() >= 0, "بهای تخصیص نامعتبر است");
    Ok(next)
}
fn discount_alloc(lines: &[OrderLine], weights: &[Money], discount: Money) -> Result<Vec<Money>> {
    money(discount)?;
    let total = sum(weights.iter().copied())?;
    ensure!(
        discount.rials() <= total.rials(),
        "تخفیف سند از مبلغ کالا بیشتر است"
    );
    if total == Money::ZERO {
        return Ok(vec![Money::ZERO; weights.len()]);
    }
    let mut values = Vec::new();
    let mut residuals = Vec::new();
    let mut assigned = 0i64;
    for (i, w) in weights.iter().enumerate() {
        let p = i128::from(w.rials()) * i128::from(discount.rials());
        let d = i128::from(total.rials());
        let v = i64::try_from(p / d)?;
        assigned = assigned
            .checked_add(v)
            .ok_or_else(|| anyhow::anyhow!("سرریز تخصیص تخفیف"))?;
        values.push(Money::from_rials(v));
        residuals.push((p % d, lines[i].id.as_str(), i));
    }
    residuals.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(b.1)));
    let remaining = usize::try_from(discount.rials() - assigned)?;
    for (_, _, i) in residuals.into_iter().take(remaining) {
        values[i] = values[i].checked_add(Money::from_rials(1))?;
    }
    Ok(values)
}
impl AccountingState {
    fn procurement_opening(&self, company: &str) -> Result<&crate::core::OpeningRecord> {
        self.core
            .openings
            .iter()
            .find(|o| o.command.company_id == company)
            .ok_or_else(|| anyhow::anyhow!("ابتدا افتتاحیهٔ تطبیق‌شده را نهایی کنید"))
    }
    pub fn procurement_resources(&self, company: &str) -> Result<RegisterResources> {
        let o = self.procurement_opening(company)?;
        let s = &o.frozen_setup;
        let mut out = RegisterResources {
            context: ProcurementContext {
                company_id: company.into(),
                period_id: s.command.period_id.clone(),
                setup_id: s.id.clone(),
                source_id: "resource-view".into(),
                source_revision: 1,
                date: o.command.as_of.clone(),
                request_key: "resource-view".into(),
                actor: s.command.actor.clone(),
                reason: "نمایش منابع".into(),
            },
            parties: o.command.parties.clone(),
            units: o.command.units.clone(),
            items: o.command.items.clone(),
            warehouses: o.command.warehouses.clone(),
        };
        for r in self
            .procurement
            .records
            .iter()
            .filter(|r| r.command.context().company_id == company)
        {
            if let ProcurementData::Resources(x) = &r.data {
                out.parties.extend(x.parties.clone());
                out.units.extend(x.units.clone());
                out.items.extend(x.items.clone());
                out.warehouses.extend(x.warehouses.clone());
            }
        }
        Ok(out)
    }
    pub fn procurement_document(&self, id: &str) -> Result<&ProcurementRecord> {
        self.procurement
            .records
            .iter()
            .find(|r| r.id == id)
            .ok_or_else(|| anyhow::anyhow!("سند خرید پیدا نشد"))
    }
    pub(crate) fn operational_stock_pools(
        &self,
        company: &str,
        asof: &str,
    ) -> Result<BTreeMap<(String, String), StockPool>> {
        let o = self.procurement_opening(company)?;
        ensure!(
            asof >= o.command.as_of.as_str(),
            "گزارش پیش از افتتاحیه پشتیبانی نمی‌شود"
        );
        let mut pools = BTreeMap::new();
        for s in &o.command.stock {
            let q = o
                .command
                .base_quantity(&s.item_id, &s.unit_id, s.quantity_thousandths)?;
            let pool = pools
                .entry((s.item_id.clone(), s.warehouse_id.clone()))
                .or_insert_with(|| empty_pool(&s.item_id, &s.warehouse_id));
            pool.quantity_thousandths = pool
                .quantity_thousandths
                .checked_add(q)
                .ok_or_else(|| anyhow::anyhow!("سرریز موجودی"))?;
            pool.value = pool.value.checked_add(s.value)?;
        }
        // A single persisted audit order governs both modules, including same-day moves.
        let mut movements: Vec<_> = self
            .procurement
            .records
            .iter()
            .map(|r| {
                (
                    &r.id,
                    r.journal_id.as_deref(),
                    r.command.context(),
                    &r.movements,
                )
            })
            .chain(self.sales.records.iter().map(|r| {
                (
                    &r.id,
                    r.journal_id.as_deref(),
                    r.command.context(),
                    &r.movements,
                )
            }))
            .filter(|(_, _, c, _)| c.company_id == company && c.date.as_str() <= asof)
            .map(|(id, journal, c, moves)| {
                let position = self
                    .audit
                    .iter()
                    .position(|a| {
                        a.company_id == company
                            && (a.entity_id == *id || journal == Some(a.entity_id.as_str()))
                    })
                    .ok_or_else(|| anyhow::anyhow!("حرکت انبار بدون سوابق تغییرات معتبر است"))?;
                Ok((position, c, moves))
            })
            .collect::<Result<Vec<_>>>()?;
        movements.sort_by_key(|(position, _, _)| *position);
        for (_, _, moves) in movements {
            for m in moves {
                let pool = pools
                    .entry((m.item_id.clone(), m.warehouse_id.clone()))
                    .or_insert_with(|| empty_pool(&m.item_id, &m.warehouse_id));
                ensure!(*pool == m.before, "توالی موجودی یا بهای حرکت نامعتبر است");
                let after = move_pool(pool, m.direction, m.quantity_thousandths, m.value)?;
                ensure!(after == m.after, "ماندهٔ حرکت موجودی نامعتبر است");
                *pool = after;
            }
        }
        Ok(pools)
    }
    fn procurement_receipts(&self, company: &str, asof: &str) -> Result<Vec<ReceiptRemainder>> {
        let o = self.procurement_opening(company)?;
        let mut receipts = BTreeMap::new();
        for x in &o.command.grni {
            let q = o
                .command
                .base_quantity(&x.item_id, &x.unit_id, x.quantity_thousandths)?;
            receipts.insert(
                x.id.clone(),
                ReceiptRemainder {
                    id: x.id.clone(),
                    source_id: o.command.source_id.clone(),
                    supplier_id: x.party_id.clone(),
                    item_id: x.item_id.clone(),
                    warehouse_id: x.warehouse_id.clone(),
                    quantity_thousandths: q,
                    value: x.value,
                    original_quantity_thousandths: q,
                    original_value: x.value,
                    date: o.command.as_of.clone(),
                    reference: x.receipt_reference.clone(),
                },
            );
        }
        for r in self.procurement.records.iter().filter(|r| {
            r.command.context().company_id == company && r.command.context().date.as_str() <= asof
        }) {
            match &r.data {
                ProcurementData::Receive(receive) => {
                    let ProcurementCommand::Receive(input) = &r.command else {
                        anyhow::bail!("نوع رسید نامعتبر است")
                    };
                    for x in &receive.lines {
                        ensure!(
                            receipts
                                .insert(
                                    x.id.clone(),
                                    ReceiptRemainder {
                                        id: x.id.clone(),
                                        source_id: input.context.source_id.clone(),
                                        supplier_id: receive.supplier_id.clone(),
                                        item_id: x.item_id.clone(),
                                        warehouse_id: x.warehouse_id.clone(),
                                        quantity_thousandths: x.quantity_thousandths,
                                        value: x.value,
                                        original_quantity_thousandths: x.quantity_thousandths,
                                        original_value: x.value,
                                        date: input.context.date.clone(),
                                        reference: input.reference.clone()
                                    }
                                )
                                .is_none(),
                            "شناسهٔ رسید تکراری است"
                        );
                    }
                }
                ProcurementData::Bill(b) => {
                    for x in &b.lines {
                        let receipt = receipts
                            .get_mut(&x.receipt_line_id)
                            .ok_or_else(|| anyhow::anyhow!("رسید فاکتور پیدا نشد"))?;
                        receipt.quantity_thousandths = receipt
                            .quantity_thousandths
                            .checked_sub(x.quantity_thousandths)
                            .ok_or_else(|| anyhow::anyhow!("سرریز مقدار فاکتور"))?;
                        receipt.value = receipt.value.checked_sub(x.value)?;
                        ensure!(
                            receipt.quantity_thousandths >= 0 && receipt.value.rials() >= 0,
                            "تخصیص فاکتور از رسید بیشتر است"
                        );
                    }
                }
                ProcurementData::Return(v) if v.bill_id.is_none() => {
                    for x in &v.lines {
                        let receipt = receipts
                            .get_mut(&x.receipt_line_id)
                            .ok_or_else(|| anyhow::anyhow!("رسید برگشت پیدا نشد"))?;
                        receipt.quantity_thousandths = receipt
                            .quantity_thousandths
                            .checked_sub(x.quantity_thousandths)
                            .ok_or_else(|| anyhow::anyhow!("سرریز مقدار برگشت"))?;
                        receipt.value = receipt.value.checked_sub(x.value)?;
                        ensure!(
                            receipt.quantity_thousandths >= 0
                                && receipt.value.rials() >= 0
                                && (receipt.quantity_thousandths != 0
                                    || receipt.value == Money::ZERO),
                            "برگشت از ماندهٔ رسید بیشتر است"
                        );
                    }
                }
                _ => {}
            }
        }
        Ok(receipts.into_values().collect())
    }
    /// Open supplier payables as-of a date, net of posted payments.
    pub fn procurement_open_payables(
        &self,
        company: &str,
        asof: &str,
    ) -> Result<Vec<PayableRemainder>> {
        self.procurement_payables(company, asof)
    }
    fn procurement_payables(&self, company: &str, asof: &str) -> Result<Vec<PayableRemainder>> {
        let o = self.procurement_opening(company)?;
        let settled = self.payment_allocation_map(company, asof)?;
        let mut out: Vec<_> = o
            .command
            .open_items
            .iter()
            .filter(|x| x.side == crate::core::OpenItemSide::Payable)
            .map(|x| -> Result<PayableRemainder> {
                Ok(PayableRemainder {
                    id: x.id.clone(),
                    source_id: o.command.source_id.clone(),
                    party_id: x.party_id.clone(),
                    reference: x.document_reference.clone(),
                    document_date: x.document_date.clone(),
                    due_date: x.due_date.clone(),
                    remaining: x
                        .remaining
                        .checked_add(settled.get(x.id.as_str()).copied().unwrap_or(Money::ZERO))?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        for r in self.procurement.records.iter().filter(|r| {
            r.command.context().company_id == company && r.command.context().date.as_str() <= asof
        }) {
            if let (ProcurementData::Bill(b), ProcurementCommand::Bill(input)) =
                (&r.data, &r.command)
                && b.total.rials() > 0
            {
                out.push(PayableRemainder {
                    id: r.id.clone(),
                    source_id: input.context.source_id.clone(),
                    party_id: b.supplier_id.clone(),
                    reference: input.reference.clone(),
                    document_date: input.document_date.clone(),
                    due_date: input.due_date.clone(),
                    remaining: b
                        .total
                        .checked_add(settled.get(r.id.as_str()).copied().unwrap_or(Money::ZERO))?,
                });
            }
        }
        for r in self.procurement.records.iter().filter(|r| {
            r.command.context().company_id == company && r.command.context().date.as_str() <= asof
        }) {
            if let ProcurementData::Return(v) = &r.data
                && let Some(bill_id) = &v.bill_id
            {
                if v.total == Money::ZERO {
                    continue;
                }
                let payable = out
                    .iter_mut()
                    .find(|p| &p.id == bill_id)
                    .ok_or_else(|| anyhow::anyhow!("ماندهٔ فاکتور برگشت پیدا نشد"))?;
                payable.remaining = payable.remaining.checked_sub(v.total)?;
                ensure!(
                    payable.remaining.rials() >= 0,
                    "اعتبار برگشت از ماندهٔ فاکتور بیشتر است"
                );
            }
        }
        Ok(out)
    }
    fn operational_receipt_record(&self, company: &str, line: &str) -> Result<&ProcurementRecord> {
        self.procurement.records.iter().find(|r| r.command.context().company_id == company
            && matches!(&r.data, ProcurementData::Receive(v) if v.lines.iter().any(|l| l.id == line)))
            .ok_or_else(|| anyhow::anyhow!("برگشت فقط از رسید عملیاتی دارای سابقهٔ اصلی مجاز است؛ رسید افتتاحیه قابل برگشت نیست"))
    }
    fn receipt_allocation_date<'a>(
        &'a self,
        company: &str,
        line: &str,
        original: &'a str,
    ) -> &'a str {
        self.procurement
            .records
            .iter()
            .filter(|r| r.command.context().company_id == company)
            .filter(|r| match &r.data {
                ProcurementData::Bill(v) => v.lines.iter().any(|l| l.receipt_line_id == line),
                ProcurementData::Return(v) => v.lines.iter().any(|l| l.receipt_line_id == line),
                _ => false,
            })
            .map(|r| r.command.context().date.as_str())
            .chain(std::iter::once(original))
            .max()
            .unwrap()
    }
    pub fn procurement_receipt_returnable(
        &self,
        company: &str,
        line: &str,
    ) -> Result<PurchaseReturnableLine> {
        let source = self.operational_receipt_record(company, line)?;
        let r = self
            .procurement_receipts(company, "9999-12-31")?
            .into_iter()
            .find(|r| r.id == line)
            .ok_or_else(|| anyhow::anyhow!("ماندهٔ رسید پیدا نشد"))?;
        Ok(PurchaseReturnableLine {
            receipt_line_id: r.id,
            bill_line_id: None,
            bill_id: None,
            supplier_id: r.supplier_id,
            item_id: r.item_id,
            warehouse_id: r.warehouse_id,
            original_quantity_thousandths: r.original_quantity_thousandths,
            original_value: r.original_value,
            remaining_quantity_thousandths: r.quantity_thousandths,
            remaining_value: r.value,
            date: self
                .receipt_allocation_date(company, line, &source.command.context().date)
                .into(),
            reference: r.reference,
        })
    }
    pub fn procurement_bill_returnable(
        &self,
        company: &str,
        line: &str,
    ) -> Result<PurchaseReturnableLine> {
        let source = self.procurement.records.iter().find(|r| r.command.context().company_id == company
            && matches!(&r.data, ProcurementData::Bill(v) if v.lines.iter().any(|l| l.id == line)))
            .ok_or_else(|| anyhow::anyhow!("ردیف فاکتور عملیاتی پیدا نشد"))?;
        let (ProcurementData::Bill(b), ProcurementCommand::Bill(c)) =
            (&source.data, &source.command)
        else {
            anyhow::bail!("منبع فاکتور نامعتبر است")
        };
        let orig = b.lines.iter().find(|l| l.id == line).unwrap();
        let mut out = self.procurement_receipt_returnable(company, &orig.receipt_line_id)?;
        // Credits consume the frozen bill-line cursor without reopening GRNI.
        let mut q = 0i64;
        let mut v = Money::ZERO;
        for r in self
            .procurement
            .records
            .iter()
            .filter(|r| r.command.context().company_id == company)
        {
            if let ProcurementData::Return(ret) = &r.data {
                for l in ret
                    .lines
                    .iter()
                    .filter(|l| l.bill_line_id.as_deref() == Some(line))
                {
                    q = q
                        .checked_add(l.quantity_thousandths)
                        .ok_or_else(|| anyhow::anyhow!("سرریز مقدار برگشت"))?;
                    v = v.checked_add(l.value)?;
                }
            }
        }
        out.bill_line_id = Some(line.into());
        out.bill_id = Some(source.id.clone());
        out.original_quantity_thousandths = orig.quantity_thousandths;
        out.original_value = orig.value;
        out.remaining_quantity_thousandths = orig
            .quantity_thousandths
            .checked_sub(q)
            .ok_or_else(|| anyhow::anyhow!("سرریز ماندهٔ برگشت"))?;
        out.remaining_value = orig.value.checked_sub(v)?;
        ensure!(
            out.remaining_quantity_thousandths >= 0
                && out.remaining_value.rials() >= 0
                && (out.remaining_quantity_thousandths != 0 || out.remaining_value == Money::ZERO),
            "برگشت از ردیف فاکتور بیشتر است"
        );
        out.date = self
            .receipt_allocation_date(company, &orig.receipt_line_id, &c.context.date)
            .into();
        out.reference = c.reference.clone();
        Ok(out)
    }
    pub fn procurement_reconcile(
        &self,
        company: &str,
        asof: &str,
    ) -> Result<ProcurementReconciliation> {
        civil_date(asof)?;
        let o = self.procurement_opening(company)?;
        ensure!(
            asof >= o.command.as_of.as_str(),
            "گزارش پیش از افتتاحیه پشتیبانی نمی‌شود"
        );
        let opening = self.reconcile_opening(company, &o.command.as_of)?;
        ensure!(opening.reconciled, "افتتاحیه تطبیق ندارد");
        let stock: Vec<_> = self
            .operational_stock_pools(company, asof)?
            .into_values()
            .collect();
        let receipts = self.procurement_receipts(company, asof)?;
        let payables = self.procurement_payables(company, asof)?;
        let tb = self.trial_balance(company, asof)?;
        let mut controls = Vec::new();
        for r in opening.controls {
            let detail = match r.role {
                CoreRole::Cash | CoreRole::Bank => r
                    .detail
                    .checked_add(self.banking_delta(company, &r.account_id, asof)?)?
                    .checked_add(self.payment_fund_delta(company, &r.account_id, asof)?)?,
                // Cheque notes accounts move through cheque-funded payments and cheque clearing.
                // Both deltas are debit-minus-credit; the payable is credit-normal, so flip it.
                CoreRole::ChequesReceivable | CoreRole::ChequesPayable => {
                    let moved = self
                        .banking_delta(company, &r.account_id, asof)?
                        .checked_add(self.payment_fund_delta(company, &r.account_id, asof)?)?;
                    let moved = if r.role == CoreRole::ChequesPayable {
                        Money::ZERO.checked_sub(moved)?
                    } else {
                        moved
                    };
                    r.detail.checked_add(moved)?
                }
                CoreRole::Inventory => sum(stock.iter().map(|x| x.value))?,
                CoreRole::Grni => sum(receipts.iter().map(|x| x.value))?,
                CoreRole::Payables => sum(payables.iter().map(|x| x.remaining))?,
                CoreRole::Receivables | CoreRole::Cogs => {
                    self.sales_control_detail(r.role, company, asof)?
                }
                CoreRole::CustomerAdvances | CoreRole::SupplierAdvances => r
                    .detail
                    .checked_add(self.payment_advance_delta(company, r.role, asof)?)?,
                _ => r.detail,
            };
            let ledger = if let Some(t) = tb.rows.iter().find(|t| t.account_id == r.account_id) {
                if r.role.normal_balance() == BalanceSide::Debit {
                    t.debit.checked_sub(t.credit)?
                } else {
                    t.credit.checked_sub(t.debit)?
                }
            } else {
                Money::ZERO
            };
            controls.push(ProcurementControl {
                role: r.role,
                account_id: r.account_id,
                ledger,
                detail,
                difference: ledger.checked_sub(detail)?,
            });
        }
        let reconciled = controls.iter().all(|r| r.difference == Money::ZERO);
        Ok(ProcurementReconciliation {
            company_id: company.into(),
            as_of: asof.into(),
            controls,
            stock,
            receipts,
            payables,
            total_debit: tb.total_debit,
            total_credit: tb.total_credit,
            reconciled,
        })
    }
    pub(crate) fn validate_operational_context(
        &self,
        c: &ProcurementContext,
    ) -> Result<&CoreSetup> {
        for t in [
            &c.company_id,
            &c.period_id,
            &c.setup_id,
            &c.source_id,
            &c.request_key,
            &c.actor,
            &c.reason,
        ] {
            text(t)?;
        }
        ensure!(c.source_revision > 0, "نسخهٔ منبع باید مثبت باشد");
        civil_date(&c.date)?;
        let o = self.procurement_opening(&c.company_id)?;
        ensure!(c.date >= o.command.as_of, "تاریخ خرید پیش از افتتاحیه است");
        self.reconcile_opening(&c.company_id, &o.command.as_of)?;
        let s = self
            .core
            .setups
            .iter()
            .find(|s| s.id == c.setup_id && s.command.company_id == c.company_id)
            .ok_or_else(|| anyhow::anyhow!("سیاست خرید متعلق به کسب‌وکار نیست"))?;
        ensure!(
            *s == o.frozen_setup && hash(&s.command)? == s.command_sha256,
            "تصویر سیاست خرید نامعتبر است"
        );
        let p = self
            .periods
            .iter()
            .find(|p| p.id == c.period_id && p.company_id == c.company_id)
            .ok_or_else(|| anyhow::anyhow!("سال مالی خرید متعلق به کسب‌وکار نیست"))?;
        ensure!(
            !p.locked && c.date >= p.start_date && c.date <= p.end_date,
            "سال مالی قفل یا تاریخ خارج از دوره است"
        );
        for m in &s.command.mappings {
            ensure!(
                self.accounts
                    .iter()
                    .any(|a| a.id == m.account_id && !a.archived && a.company_id == c.company_id),
                "حساب نگاشت فعال نیست"
            );
        }
        Ok(s)
    }
    pub(crate) fn procurement_entity_ids(&self) -> Vec<&str> {
        let mut ids = Vec::new();
        for record in &self.procurement.records {
            ids.push(record.id.as_str());
            ids.extend(data_ids(&record.data));
            ids.extend(record.movements.iter().map(|m| m.id.as_str()));
        }
        ids
    }
    pub(crate) fn operational_known_ids(&self) -> HashSet<String> {
        let mut out: HashSet<String> = self
            .companies
            .iter()
            .map(|x| x.id.clone())
            .chain(self.periods.iter().map(|x| x.id.clone()))
            .chain(self.accounts.iter().map(|x| x.id.clone()))
            .chain(self.journals.iter().map(|x| x.id.clone()))
            .chain(self.audit.iter().map(|x| x.id.clone()))
            .chain(self.core.setups.iter().map(|x| x.id.clone()))
            .chain(self.core.openings.iter().map(|x| x.journal_id.clone()))
            .collect();
        for o in &self.core.openings {
            out.extend(o.command.parties.iter().map(|x| x.id.clone()));
            out.extend(o.command.units.iter().map(|x| x.id.clone()));
            out.extend(o.command.items.iter().map(|x| x.id.clone()));
            out.extend(o.command.warehouses.iter().map(|x| x.id.clone()));
            out.extend(o.command.cash.iter().map(|x| x.id.clone()));
            out.extend(o.command.banks.iter().map(|x| x.id.clone()));
            out.extend(o.command.open_items.iter().map(|x| x.id.clone()));
            out.extend(o.command.advances.iter().map(|x| x.id.clone()));
            out.extend(o.command.cheques.iter().map(|x| x.id.clone()));
            out.extend(o.command.stock.iter().map(|x| x.id.clone()));
            out.extend(o.command.grni.iter().map(|x| x.id.clone()));
        }
        for r in &self.procurement.records {
            out.insert(r.id.clone());
            out.extend(r.movements.iter().map(|m| m.id.clone()));
            match &r.data {
                ProcurementData::Resources(x) => {
                    out.extend(x.parties.iter().map(|x| x.id.clone()));
                    out.extend(x.units.iter().map(|x| x.id.clone()));
                    out.extend(x.items.iter().map(|x| x.id.clone()));
                    out.extend(x.warehouses.iter().map(|x| x.id.clone()));
                }
                ProcurementData::Order(x) => out.extend(x.lines.iter().map(|x| x.input.id.clone())),
                ProcurementData::Receive(x) => out.extend(x.lines.iter().map(|x| x.id.clone())),
                ProcurementData::Bill(x) => out.extend(x.lines.iter().map(|x| x.id.clone())),
                ProcurementData::Return(x) => {
                    out.insert(x.return_id.clone());
                    out.extend(x.credit_id.clone());
                    out.extend(x.lines.iter().map(|l| l.id.clone()));
                }
                _ => {}
            }
        }
        out.extend(self.sales_entity_ids().into_iter().map(str::to_owned));
        for r in &self.payments.records {
            out.insert(r.id.clone());
        }
        out
    }
    pub(crate) fn operational_stock_date(&self, company: &str) -> Result<&str> {
        let o = self.procurement_opening(company)?;
        Ok(self
            .procurement
            .records
            .iter()
            .filter(|r| r.command.context().company_id == company && !r.movements.is_empty())
            .map(|r| r.command.context().date.as_str())
            .chain(
                self.sales
                    .records
                    .iter()
                    .filter(|r| {
                        r.command.context().company_id == company && !r.movements.is_empty()
                    })
                    .map(|r| r.command.context().date.as_str()),
            )
            .max()
            .unwrap_or(&o.command.as_of))
    }
    pub(crate) fn operational_sequence(&self, company: &str) -> Result<u64> {
        u64::try_from(
            self.audit
                .iter()
                .filter(|a| {
                    a.company_id == company
                        && matches!(
                            a.operation,
                            AuditOperation::JournalPosted
                                | AuditOperation::CoreOpeningInitialized
                                | AuditOperation::ProcurementRecorded
                                | AuditOperation::SalesRecorded
                        )
                })
                .count(),
        )?
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("سرریز توالی شرکت"))
    }
    pub fn preview_procurement(&self, command: &ProcurementCommand) -> Result<ProcurementPreview> {
        let c = command.context();
        let setup = self.validate_operational_context(c)?;
        let resources = self.procurement_resources(&c.company_id)?;
        let mut ids = self.operational_known_ids();
        let claim = |id: &str, ids: &mut HashSet<String>| -> Result<()> {
            text(id)?;
            ensure!(
                ids.insert(id.into()),
                "شناسهٔ منبع/ردیف تکراری یا متعلق به سابقه است"
            );
            Ok(())
        };
        let warehouse = |id: &str| -> Result<()> {
            ensure!(
                resources.warehouses.iter().any(|w| w.id == id),
                "انبار معتبر نیست"
            );
            Ok(())
        };
        let supplier = |id: &str| -> Result<CoreParty> {
            resources
                .parties
                .iter()
                .find(|p| p.id == id && p.supplier)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("شخص نقش تأمین‌کننده ندارد"))
        };
        let mut movements = Vec::new();
        let mut journal_lines = Vec::new();
        let (data, total) = match command {
            ProcurementCommand::Resources(x) => {
                ensure!(
                    !x.parties.is_empty()
                        || !x.units.is_empty()
                        || !x.items.is_empty()
                        || !x.warehouses.is_empty(),
                    "حداقل یک منبع جدید لازم است"
                );
                for p in &x.parties {
                    claim(&p.id, &mut ids)?;
                    ensure!(
                        p.company_id == c.company_id && (p.customer || p.supplier),
                        "کسب‌وکار/نقش شخص نامعتبر است"
                    );
                    text(&p.name)?;
                }
                let mut unit_names: HashSet<_> =
                    resources.units.iter().map(|u| normalize(&u.name)).collect();
                for u in &x.units {
                    claim(&u.id, &mut ids)?;
                    ensure!(
                        u.company_id == c.company_id && unit_names.insert(normalize(&u.name)),
                        "کسب‌وکار یا نام واحد تکراری است"
                    );
                    text(&u.name)?;
                }
                let mut skus: HashSet<_> =
                    resources.items.iter().map(|i| normalize(&i.sku)).collect();
                let knownunit =
                    |id: &str| resources.units.iter().chain(&x.units).any(|u| u.id == id);
                for i in &x.items {
                    claim(&i.id, &mut ids)?;
                    ensure!(
                        i.company_id == c.company_id
                            && skus.insert(normalize(&i.sku))
                            && knownunit(&i.base_unit_id),
                        "کسب‌وکار/کد کالا/واحد پایه نامعتبر است"
                    );
                    text(&i.sku)?;
                    text(&i.name)?;
                    let mut packs = HashSet::new();
                    for p in &i.packs {
                        ensure!(
                            p.unit_id != i.base_unit_id
                                && knownunit(&p.unit_id)
                                && packs.insert(&p.unit_id)
                                && p.numerator > 0
                                && p.denominator > 0,
                            "بسته‌بندی نامعتبر یا تکراری است"
                        );
                    }
                }
                for w in &x.warehouses {
                    claim(&w.id, &mut ids)?;
                    ensure!(w.company_id == c.company_id, "کسب‌وکار انبار نامعتبر است");
                    text(&w.name)?;
                }
                (ProcurementData::Resources(x.clone()), Money::ZERO)
            }
            ProcurementCommand::Order(x) => {
                text(&x.reference)?;
                doc_date(&x.document_date, &c.date)?;
                money(x.supplied_total)?;
                ensure!(
                    (1..=10_000).contains(&x.lines.len()),
                    "تعداد ردیف سفارش نامعتبر است"
                );
                let supplier = supplier(&x.supplier_id)?;
                ensure!(!self.procurement.records.iter().any(|r|matches!(&r.command,ProcurementCommand::Order(p) if p.context.company_id==c.company_id&&p.supplier_id==x.supplier_id&&normalize(&p.reference)==normalize(&x.reference)&&p.document_date==x.document_date)),"مرجع سفارش این تأمین‌کننده تکراری است");
                let mut frozen = Vec::new();
                let mut weights = Vec::new();
                for l in &x.lines {
                    claim(&l.id, &mut ids)?;
                    warehouse(&l.warehouse_id)?;
                    text(&l.price_reference)?;
                    doc_date(&l.price_date, &c.date)?;
                    if let Some(reason) = &l.override_reason {
                        text(reason)?;
                    }
                    money(l.unit_price)?;
                    money(l.line_discount)?;
                    let item = resources
                        .items
                        .iter()
                        .find(|i| i.id == l.item_id)
                        .ok_or_else(|| anyhow::anyhow!("کالای سفارش پیدا نشد"))?;
                    let q = base(item, &l.unit_id, l.quantity_thousandths)?;
                    let gross = Money::from_rials(positive_half_up(
                        i128::from(l.unit_price.rials()) * i128::from(l.quantity_thousandths),
                        1000,
                    )?);
                    money(gross)?;
                    ensure!(
                        l.line_discount.rials() <= gross.rials(),
                        "تخفیف ردیف از مبلغ بیشتر است"
                    );
                    let weight = gross.checked_sub(l.line_discount)?;
                    weights.push(weight);
                    frozen.push(FrozenOrderLine {
                        input: l.clone(),
                        item: item.clone(),
                        base_quantity_thousandths: q,
                        gross,
                        document_discount: Money::ZERO,
                        net: weight,
                    });
                }
                for (l, d) in
                    frozen
                        .iter_mut()
                        .zip(discount_alloc(&x.lines, &weights, x.document_discount)?)
                {
                    l.document_discount = d;
                    l.net = l.net.checked_sub(d)?;
                }
                let total = sum(frozen.iter().map(|l| l.net))?;
                money(total)?;
                ensure!(
                    total == x.supplied_total,
                    "جمع مستقل سفارش با خالص محاسبه‌شده برابر نیست"
                );
                (
                    ProcurementData::Order(PostedOrder {
                        supplier,
                        lines: frozen,
                        total,
                    }),
                    total,
                )
            }
            ProcurementCommand::Receive(x) => {
                text(&x.reference)?;
                doc_date(&x.document_date, &c.date)?;
                money(x.supplied_total)?;
                ensure!(
                    (1..=10_000).contains(&x.lines.len()),
                    "تعداد ردیف رسید نامعتبر است"
                );
                ensure!(
                    c.date.as_str() >= self.operational_stock_date(&c.company_id)?,
                    "ثبت موجودی پیش از آخرین تاریخ موجودی شرکت ممنوع است"
                );
                let order_record = self.procurement_document(&x.order_id)?;
                ensure!(
                    order_record.command.context().company_id == c.company_id
                        && c.date >= order_record.command.context().date,
                    "سفارش متعلق به شرکت یا تاریخ رسید نیست"
                );
                let ProcurementData::Order(order) = &order_record.data else {
                    anyhow::bail!("منبع رسید سفارش خرید نیست")
                };
                ensure!(
                    !self
                        .procurement_opening(&c.company_id)?
                        .command
                        .grni
                        .iter()
                        .any(|r| r.party_id == order.supplier.id
                            && normalize(&r.receipt_reference) == normalize(&x.reference)
                            && r.received_date == x.document_date),
                    "این رسید قبلاً در افتتاحیه منظور شده است"
                );
                ensure!(!self.procurement.records.iter().any(|r|matches!((&r.data,&r.command),(ProcurementData::Receive(v),ProcurementCommand::Receive(p)) if p.context.company_id==c.company_id&&v.supplier_id==order.supplier.id&&normalize(&p.reference)==normalize(&x.reference)&&p.document_date==x.document_date)),"مرجع رسید تأمین‌کننده تکراری است");
                let mut lines = Vec::new();
                let mut source_ids = HashSet::new();
                let mut pools = self.operational_stock_pools(&c.company_id, "9999-12-31")?;
                for l in &x.lines {
                    claim(&l.id, &mut ids)?;
                    warehouse(&l.warehouse_id)?;
                    ensure!(
                        source_ids.insert(&l.order_line_id),
                        "ردیف سفارش در یک رسید تکراری است"
                    );
                    let orig = order
                        .lines
                        .iter()
                        .find(|o| o.input.id == l.order_line_id)
                        .ok_or_else(|| anyhow::anyhow!("ردیف سفارش پیدا نشد"))?;
                    let q = base(&orig.item, &l.unit_id, l.quantity_thousandths)?;
                    let mut priorq = 0i64;
                    let mut priorv = Money::ZERO;
                    for r in &self.procurement.records {
                        if let ProcurementData::Receive(receipt) = &r.data
                            && receipt.order_id == x.order_id
                        {
                            for old in receipt
                                .lines
                                .iter()
                                .filter(|old| old.order_line_id == l.order_line_id)
                            {
                                priorq = priorq
                                    .checked_add(old.quantity_thousandths)
                                    .ok_or_else(|| anyhow::anyhow!("سرریز مقدار دریافت"))?;
                                priorv = priorv.checked_add(old.value)?;
                            }
                        }
                    }
                    let value =
                        allocation(orig.net, orig.base_quantity_thousandths, priorq, priorv, q)?;
                    let pool = pools
                        .entry((orig.item.id.clone(), l.warehouse_id.clone()))
                        .or_insert_with(|| empty_pool(&orig.item.id, &l.warehouse_id));
                    let before = pool.clone();
                    let after = move_pool(pool, StockDirection::In, q, value)?;
                    movements.push(StockMovement {
                        id: movement_id(c, &l.id, "in")?,
                        item_id: orig.item.id.clone(),
                        warehouse_id: l.warehouse_id.clone(),
                        direction: StockDirection::In,
                        quantity_thousandths: q,
                        value,
                        source_line_id: l.id.clone(),
                        before,
                        after: after.clone(),
                    });
                    *pool = after;
                    lines.push(PostedReceiptLine {
                        id: l.id.clone(),
                        order_line_id: l.order_line_id.clone(),
                        item_id: orig.item.id.clone(),
                        warehouse_id: l.warehouse_id.clone(),
                        quantity_thousandths: q,
                        value,
                    });
                }
                let total = sum(lines.iter().map(|l| l.value))?;
                ensure!(
                    total == x.supplied_total,
                    "جمع رسید با بهای تخصیص‌یافتهٔ سفارش برابر نیست"
                );
                if total.rials() > 0 {
                    journal_lines = pair(
                        setup,
                        CoreRole::Inventory,
                        CoreRole::Grni,
                        total,
                        &x.reference,
                    )?;
                }
                (
                    ProcurementData::Receive(PostedReceipt {
                        order_id: x.order_id.clone(),
                        supplier_id: order.supplier.id.clone(),
                        lines,
                        total,
                    }),
                    total,
                )
            }
            ProcurementCommand::Bill(x) => {
                text(&x.reference)?;
                doc_date(&x.document_date, &c.date)?;
                civil_date(&x.due_date)?;
                ensure!(
                    x.due_date >= x.document_date,
                    "سررسید پیش از تاریخ فاکتور است"
                );
                supplier(&x.supplier_id)?;
                money(x.supplied_total)?;
                ensure!(
                    (1..=10_000).contains(&x.lines.len()),
                    "تعداد ردیف فاکتور نامعتبر است"
                );
                ensure!(
                    !self
                        .procurement_opening(&c.company_id)?
                        .command
                        .open_items
                        .iter()
                        .any(|r| r.side == crate::core::OpenItemSide::Payable
                            && r.party_id == x.supplier_id
                            && normalize(&r.document_reference) == normalize(&x.reference)
                            && r.document_date == x.document_date),
                    "این فاکتور پرداختنی قبلاً در افتتاحیه منظور شده است"
                );
                ensure!(!self.procurement.records.iter().any(|r|matches!(&r.command,ProcurementCommand::Bill(b) if b.context.company_id==c.company_id&&b.supplier_id==x.supplier_id&&normalize(&b.reference)==normalize(&x.reference)&&b.document_date==x.document_date)),"مرجع فاکتور این تأمین‌کننده قبلاً ثبت شده است؛ بازبینی مستقل لازم است");
                let receipts = self.procurement_receipts(&c.company_id, "9999-12-31")?;
                let mut lines = Vec::new();
                let mut source_ids = HashSet::new();
                for l in &x.lines {
                    claim(&l.id, &mut ids)?;
                    money(l.supplied_value)?;
                    ensure!(
                        source_ids.insert(&l.receipt_line_id),
                        "رسید در یک فاکتور تکراری است"
                    );
                    let r = receipts
                        .iter()
                        .find(|r| r.id == l.receipt_line_id)
                        .ok_or_else(|| anyhow::anyhow!("رسید فاکتور پیدا نشد"))?;
                    ensure!(
                        r.supplier_id == x.supplier_id && c.date >= r.date,
                        "شخص یا تاریخ فاکتور با رسید سازگار نیست"
                    );
                    let resources_item = resources
                        .items
                        .iter()
                        .find(|i| i.id == r.item_id)
                        .ok_or_else(|| anyhow::anyhow!("کالای رسید پیدا نشد"))?;
                    ensure!(
                        l.quantity_thousandths > 0
                            && (!resources_item.indivisible || l.quantity_thousandths % 1000 == 0),
                        "مقدار فاکتور واحد پایه نامعتبر است"
                    );
                    ensure!(
                        c.date.as_str()
                            >= self.receipt_allocation_date(
                                &c.company_id,
                                &l.receipt_line_id,
                                &r.date
                            ),
                        "تاریخ فاکتور پیش از آخرین تخصیص یا برگشت همین رسید است"
                    );
                    // Bills and unbilled returns consume one original receipt cursor.
                    let value = allocation(
                        r.original_value,
                        r.original_quantity_thousandths,
                        r.original_quantity_thousandths - r.quantity_thousandths,
                        r.original_value.checked_sub(r.value)?,
                        l.quantity_thousandths,
                    )?;
                    ensure!(
                        value == l.supplied_value,
                        "اختلاف بهای فاکتور: انتظار {} ریال، منبع {} ریال؛ اصلاح بهای دیرهنگام پشتیبانی نمی‌شود",
                        value.rials(),
                        l.supplied_value.rials()
                    );
                    lines.push(PostedBillLine {
                        id: l.id.clone(),
                        receipt_line_id: l.receipt_line_id.clone(),
                        quantity_thousandths: l.quantity_thousandths,
                        value,
                    });
                }
                let total = sum(lines.iter().map(|l| l.value))?;
                ensure!(total == x.supplied_total, "جمع مستقل فاکتور برابر نیست");
                if total.rials() > 0 {
                    journal_lines = pair(
                        setup,
                        CoreRole::Grni,
                        CoreRole::Payables,
                        total,
                        &x.reference,
                    )?;
                }
                (
                    ProcurementData::Bill(PostedBill {
                        supplier_id: x.supplier_id.clone(),
                        lines,
                        total,
                    }),
                    total,
                )
            }
            ProcurementCommand::Return(x) => {
                supplier(&x.supplier_id)?;
                text(&x.return_reference)?;
                doc_date(&x.document_date, &c.date)?;
                money(x.supplied_total)?;
                ensure!(
                    (1..=10_000).contains(&x.lines.len()),
                    "تعداد ردیف برگشت نامعتبر است"
                );
                let billed = x.lines[0].bill_line_id.is_some();
                ensure!(
                    x.lines.iter().all(|l| l.bill_line_id.is_some() == billed)
                        && x.credit_id.is_some() == billed
                        && x.credit_reference.is_some() == billed,
                    "برگشت فاکتور‌شده و فاکتور‌نشده قابل ترکیب نیست؛ شناسه و مرجع اعتبار الزامی است"
                );
                claim(&x.return_id, &mut ids)?;
                if let Some(credit) = &x.credit_id {
                    claim(credit, &mut ids)?;
                }
                if let Some(reference) = &x.credit_reference {
                    text(reference)?;
                }
                ensure!(!self.procurement.records.iter().any(|r| matches!(&r.command, ProcurementCommand::Return(old)
                    if old.context.company_id == c.company_id && old.supplier_id == x.supplier_id
                    && old.document_date == x.document_date && normalize(&old.return_reference) == normalize(&x.return_reference))),
                    "مرجع برگشت این تأمین‌کننده قبلاً ثبت شده است");
                if let Some(reference) = &x.credit_reference {
                    ensure!(!self.procurement.records.iter().any(|r| matches!(&r.command, ProcurementCommand::Return(old)
                        if old.context.company_id == c.company_id && old.supplier_id == x.supplier_id
                        && old.document_date == x.document_date && old.credit_reference.as_ref().is_some_and(|v| normalize(v) == normalize(reference)))),
                        "مرجع اعتبار این تأمین‌کننده قبلاً ثبت شده است");
                }
                ensure!(
                    c.date.as_str() >= self.operational_stock_date(&c.company_id)?,
                    "ثبت برگشت پیش از آخرین تاریخ موجودی شرکت ممنوع است"
                );
                let mut pools = self.operational_stock_pools(&c.company_id, "9999-12-31")?;
                let mut source_ids = HashSet::new();
                let mut receipt_id: Option<String> = None;
                let mut bill_id: Option<String> = None;
                let mut lines = Vec::new();
                for l in &x.lines {
                    claim(&l.id, &mut ids)?;
                    money(l.supplied_value)?;
                    ensure!(
                        source_ids.insert(&l.receipt_line_id),
                        "ردیف رسید در برگشت تکراری است"
                    );
                    let source =
                        self.operational_receipt_record(&c.company_id, &l.receipt_line_id)?;
                    let eligible = if let Some(bill_line) = &l.bill_line_id {
                        let e = self.procurement_bill_returnable(&c.company_id, bill_line)?;
                        ensure!(
                            e.receipt_line_id == l.receipt_line_id,
                            "ردیف فاکتور به این رسید تعلق ندارد"
                        );
                        if let Some(prior) = &bill_id {
                            ensure!(
                                e.bill_id.as_ref() == Some(prior),
                                "برگشت باید به یک فاکتور تعلق داشته باشد"
                            );
                        }
                        bill_id = e.bill_id.clone();
                        e
                    } else {
                        if let Some(prior) = &receipt_id {
                            ensure!(
                                *prior == source.id,
                                "برگشت فاکتور‌نشده باید به یک رسید تعلق داشته باشد"
                            );
                        }
                        receipt_id = Some(source.id.clone());
                        self.procurement_receipt_returnable(&c.company_id, &l.receipt_line_id)?
                    };
                    ensure!(
                        eligible.supplier_id == x.supplier_id && c.date >= eligible.date,
                        "شخص یا تاریخ برگشت با منبع و آخرین تخصیص سازگار نیست"
                    );
                    let item = resources
                        .items
                        .iter()
                        .find(|i| i.id == eligible.item_id)
                        .ok_or_else(|| anyhow::anyhow!("کالای رسید برگشت پیدا نشد"))?;
                    ensure!(
                        l.quantity_thousandths > 0
                            && (!item.indivisible || l.quantity_thousandths % 1000 == 0),
                        "مقدار برگشت واحد پایه نامعتبر است"
                    );
                    let value = allocation(
                        eligible.original_value,
                        eligible.original_quantity_thousandths,
                        eligible.original_quantity_thousandths
                            - eligible.remaining_quantity_thousandths,
                        eligible
                            .original_value
                            .checked_sub(eligible.remaining_value)?,
                        l.quantity_thousandths,
                    )?;
                    ensure!(
                        value == l.supplied_value,
                        "بهای اصلی برگشت برابر نیست: انتظار {} ریال، ارائه {} ریال",
                        value.rials(),
                        l.supplied_value.rials()
                    );
                    let pool = pools
                        .entry((eligible.item_id.clone(), eligible.warehouse_id.clone()))
                        .or_insert_with(|| empty_pool(&eligible.item_id, &eligible.warehouse_id));
                    let before = pool.clone();
                    let after = move_pool(pool, StockDirection::Out, l.quantity_thousandths, value)
                        .map_err(|error| anyhow::anyhow!("برگشت با بهای اصلی از موجودی فعلی ممکن نیست؛ اصلاح بهای جداگانه لازم است: {error}"))?;
                    let movement_id = movement_id(c, &l.id, "return-out")?;
                    movements.push(StockMovement {
                        id: movement_id,
                        item_id: eligible.item_id.clone(),
                        warehouse_id: eligible.warehouse_id.clone(),
                        direction: StockDirection::Out,
                        quantity_thousandths: l.quantity_thousandths,
                        value,
                        source_line_id: l.id.clone(),
                        before,
                        after: after.clone(),
                    });
                    *pool = after;
                    lines.push(PostedPurchaseReturnLine {
                        id: l.id.clone(),
                        receipt_line_id: l.receipt_line_id.clone(),
                        bill_line_id: l.bill_line_id.clone(),
                        item_id: eligible.item_id,
                        warehouse_id: eligible.warehouse_id,
                        quantity_thousandths: l.quantity_thousandths,
                        value,
                    });
                }
                let total = sum(lines.iter().map(|l| l.value))?;
                money(total)?;
                ensure!(total == x.supplied_total, "جمع مستقل برگشت برابر نیست");
                if let Some(bill) = &bill_id
                    && total != Money::ZERO
                {
                    let payables = self.procurement_payables(&c.company_id, "9999-12-31")?;
                    let payable = payables
                        .iter()
                        .find(|p| &p.id == bill)
                        .ok_or_else(|| anyhow::anyhow!("ماندهٔ پرداختنی فاکتور پیدا نشد"))?;
                    ensure!(
                        total.rials() <= payable.remaining.rials(),
                        "اعتبار از ماندهٔ پرداخت‌نشدهٔ فاکتور بیشتر است"
                    );
                }
                if total != Money::ZERO {
                    journal_lines = pair(
                        setup,
                        if billed {
                            CoreRole::Payables
                        } else {
                            CoreRole::Grni
                        },
                        CoreRole::Inventory,
                        total,
                        &x.return_reference,
                    )?;
                }
                (
                    ProcurementData::Return(PostedPurchaseReturn {
                        supplier_id: x.supplier_id.clone(),
                        return_id: x.return_id.clone(),
                        return_state: PurchaseReturnState::Returned,
                        credit_id: x.credit_id.clone(),
                        credit_state: if billed {
                            Some(SupplierCreditState::Posted)
                        } else {
                            None
                        },
                        receipt_id,
                        bill_id,
                        lines,
                        total,
                    }),
                    total,
                )
            }
            ProcurementCommand::Transfer(x) => {
                ensure!(
                    x.source_warehouse_id != x.target_warehouse_id,
                    "انبار مبدأ و مقصد یکسان است"
                );
                warehouse(&x.source_warehouse_id)?;
                warehouse(&x.target_warehouse_id)?;
                ensure!(
                    c.date.as_str() >= self.operational_stock_date(&c.company_id)?,
                    "ثبت موجودی پیش از آخرین تاریخ موجودی شرکت ممنوع است"
                );
                let item = resources
                    .items
                    .iter()
                    .find(|i| i.id == x.item_id)
                    .ok_or_else(|| anyhow::anyhow!("کالای انتقال پیدا نشد"))?;
                let q = base(item, &x.unit_id, x.quantity_thousandths)?;
                let pools = self.operational_stock_pools(&c.company_id, "9999-12-31")?;
                let source = pools
                    .get(&(x.item_id.clone(), x.source_warehouse_id.clone()))
                    .cloned()
                    .unwrap_or_else(|| empty_pool(&x.item_id, &x.source_warehouse_id));
                let target = pools
                    .get(&(x.item_id.clone(), x.target_warehouse_id.clone()))
                    .cloned()
                    .unwrap_or_else(|| empty_pool(&x.item_id, &x.target_warehouse_id));
                ensure!(q <= source.quantity_thousandths, "موجودی مبدأ کافی نیست");
                let value = if q == source.quantity_thousandths {
                    source.value
                } else {
                    Money::from_rials(positive_half_up(
                        i128::from(source.value.rials()) * i128::from(q),
                        i128::from(source.quantity_thousandths),
                    )?)
                };
                let sourceafter = move_pool(&source, StockDirection::Out, q, value)?;
                let targetafter = move_pool(&target, StockDirection::In, q, value)?;
                movements.push(StockMovement {
                    id: movement_id(c, &c.source_id, "out")?,
                    item_id: x.item_id.clone(),
                    warehouse_id: x.source_warehouse_id.clone(),
                    direction: StockDirection::Out,
                    quantity_thousandths: q,
                    value,
                    source_line_id: c.source_id.clone(),
                    before: source,
                    after: sourceafter,
                });
                movements.push(StockMovement {
                    id: movement_id(c, &c.source_id, "in")?,
                    item_id: x.item_id.clone(),
                    warehouse_id: x.target_warehouse_id.clone(),
                    direction: StockDirection::In,
                    quantity_thousandths: q,
                    value,
                    source_line_id: c.source_id.clone(),
                    before: target,
                    after: targetafter,
                });
                (
                    ProcurementData::Transfer(PostedTransfer {
                        item_id: x.item_id.clone(),
                        quantity_thousandths: q,
                        value,
                    }),
                    value,
                )
            }
        };
        money(total)?;
        for m in &movements {
            claim(&m.id, &mut ids)?;
        }
        let nonfinancial = journal_lines.is_empty();
        Ok(ProcurementPreview {
            data,
            movements,
            total,
            journal_lines,
            nonfinancial,
        })
    }
    pub fn post_procurement_command(
        &mut self,
        command: ProcurementCommand,
    ) -> Result<ProcurementResult> {
        let mut next = self.clone();
        let out = next.post_procurement_inner(command)?;
        *self = next;
        Ok(out)
    }
    fn post_procurement_inner(&mut self, command: ProcurementCommand) -> Result<ProcurementResult> {
        let c = command.context();
        ensure!(
            !self
                .sales
                .requests
                .iter()
                .any(|r| r.request_key == c.request_key),
            "کلید درخواست قبلاً برای فروش استفاده شده است"
        );
        text(&c.request_key)?;
        let business = business_hash(&command)?;
        ensure!(
            !self
                .core
                .zero_requests
                .iter()
                .any(|r| r.request_key == c.request_key),
            "کلید درخواست برای شروع دفتر بدون مانده استفاده شده است"
        );
        if let Some(alias) = self
            .procurement
            .requests
            .iter()
            .find(|r| r.request_key == c.request_key)
        {
            ensure!(
                alias.company_id == c.company_id && alias.business_sha256 == business,
                "کلید درخواست با محتوای متفاوت استفاده شده است"
            );
            let prior = self.procurement_document(&alias.record_id)?;
            ensure!(
                prior.command_sha256 == hash(&prior.command)?
                    && business_hash(&prior.command)? == business
                    && self.core.setups.contains(&prior.frozen_setup),
                "تصویر سند خرید نامعتبر است"
            );
            return Ok(result(prior));
        }
        if let Some(prior) = self
            .procurement
            .records
            .iter()
            .find(|r| {
                r.command.context().company_id == c.company_id
                    && r.command.context().source_id == c.source_id
                    && r.command.event() == command.event()
            })
            .cloned()
        {
            ensure!(
                prior.command_sha256 == hash(&prior.command)?
                    && business_hash(&prior.command)? == business
                    && self.core.setups.contains(&prior.frozen_setup),
                "رویداد خرید قبلاً با محتوای متفاوت یا نسخهٔ دیگر ثبت شده است"
            );
            if prior.journal_id.is_some() {
                let lines = record_journal_lines(&prior)?;
                ensure!(
                    self.post_procurement(financial_command(&command, lines))?
                        == prior.journal_id.clone().unwrap(),
                    "درخواست به سند دیگری اشاره می‌کند"
                );
            } else {
                ensure!(
                    !self
                        .requests
                        .iter()
                        .any(|r| r.command.request_key == c.request_key),
                    "کلید درخواست قبلاً برای عملیات مالی استفاده شده است"
                );
            }
            self.procurement.requests.push(ProcurementRequest {
                request_key: c.request_key.clone(),
                company_id: c.company_id.clone(),
                record_id: prior.id.clone(),
                business_sha256: business,
            });
            return Ok(result(&prior));
        }
        ensure!(
            !self
                .requests
                .iter()
                .any(|r| r.command.request_key == c.request_key),
            "کلید درخواست قبلاً برای عملیات مالی دیگری استفاده شده است"
        );
        let preview = self.preview_procurement(&command)?;
        let frozen_setup = self
            .core
            .setups
            .iter()
            .find(|s| s.id == c.setup_id)
            .unwrap()
            .clone();
        let record_id = id();
        ensure!(
            !self.operational_known_ids().contains(&record_id)
                && !data_ids(&preview.data).contains(&record_id.as_str())
                && !preview.movements.iter().any(|m| m.id == record_id),
            "شناسهٔ رویداد خرید با منابع تداخل دارد"
        );
        let sequence = self.operational_sequence(&c.company_id)?;
        let command_sha256 = hash(&command)?;
        let journal_id = if preview.nonfinancial {
            self.activity(
                &c.company_id,
                &record_id,
                AuditOperation::ProcurementRecorded,
                &c.actor,
                &c.reason,
            );
            None
        } else {
            Some(self.post_procurement(financial_command(&command, preview.journal_lines))?)
        };
        let record = ProcurementRecord {
            id: record_id.clone(),
            command: command.clone(),
            command_sha256,
            frozen_setup,
            sequence,
            journal_id,
            data: preview.data,
            movements: preview.movements,
        };
        let out = result(&record);
        self.procurement.records.push(record);
        self.procurement.requests.push(ProcurementRequest {
            request_key: c.request_key.clone(),
            company_id: c.company_id.clone(),
            record_id,
            business_sha256: business,
        });
        let report = self.procurement_reconcile(&c.company_id, "9999-12-31")?;
        ensure!(report.reconciled, "ثبت خرید باعث اختلاف حساب کنترلی می‌شود");
        Ok(out)
    }
    pub(crate) fn replay_procurement_record(
        &mut self,
        record: &ProcurementRecord,
    ) -> Result<Option<String>> {
        text(&record.id)?;
        ensure!(
            !self.operational_known_ids().contains(&record.id)
                && !data_ids(&record.data).contains(&record.id.as_str())
                && !record.movements.iter().any(|m| m.id == record.id),
            "شناسهٔ سوابق تغییرات خرید با شناسهٔ منبع تداخل دارد"
        );
        let before = self.procurement.records.len();
        let out = self.post_procurement_command(record.command.clone())?;
        ensure!(
            self.procurement.records.len() == before + 1,
            "سوابق تغییرات رویداد خرید تکراری است"
        );
        let created = self.procurement.records.last_mut().unwrap();
        created.id = record.id.clone();
        self.procurement.requests.last_mut().unwrap().record_id = record.id.clone();
        let mut expected = record.clone();
        expected.journal_id = out.journal_id.clone();
        ensure!(
            *created == expected && record.journal_id.is_some() == out.journal_id.is_some(),
            "محتوا یا محاسبات سوابق تغییرات خرید نامعتبر است"
        );
        if out.journal_id.is_none() {
            self.audit.last_mut().unwrap().entity_id = record.id.clone();
        }
        Ok(out.journal_id)
    }
    pub(crate) fn replay_procurement_request(
        &mut self,
        request: &ProcurementRequest,
    ) -> Result<ProcurementResult> {
        let prior = self.procurement_document(&request.record_id)?;
        ensure!(
            request.company_id == prior.command.context().company_id
                && request.business_sha256 == business_hash(&prior.command)?,
            "هویت یا هش درخواست خرید نامعتبر است"
        );
        let mut c = prior.command.clone();
        c.context_mut().request_key = request.request_key.clone();
        self.post_procurement_command(c)
    }
}
fn empty_pool(item: &str, warehouse: &str) -> StockPool {
    StockPool {
        item_id: item.into(),
        warehouse_id: warehouse.into(),
        quantity_thousandths: 0,
        value: Money::ZERO,
    }
}
fn move_pool(before: &StockPool, direction: StockDirection, q: i64, v: Money) -> Result<StockPool> {
    ensure!(
        q > 0 && v.rials() >= 0,
        "مقدار/بهای حرکت موجودی نامعتبر است"
    );
    let mut after = before.clone();
    match direction {
        StockDirection::In => {
            after.quantity_thousandths = after
                .quantity_thousandths
                .checked_add(q)
                .ok_or_else(|| anyhow::anyhow!("سرریز مقدار موجودی"))?;
            after.value = after.value.checked_add(v)?;
        }
        StockDirection::Out => {
            after.quantity_thousandths = after
                .quantity_thousandths
                .checked_sub(q)
                .ok_or_else(|| anyhow::anyhow!("سرریز مقدار موجودی"))?;
            after.value = after.value.checked_sub(v)?;
        }
    }
    ensure!(
        after.quantity_thousandths >= 0
            && after.value.rials() >= 0
            && (after.quantity_thousandths != 0 || after.value == Money::ZERO),
        "موجودی منفی یا بهای باقی‌مانده بدون مقدار ممنوع است"
    );
    Ok(after)
}
fn movement_id(context: &ProcurementContext, line: &str, direction: &str) -> Result<String> {
    Ok(format!(
        "stock:{}",
        hash(&(
            context.company_id.as_str(),
            context.source_id.as_str(),
            line,
            direction
        ))?
    ))
}
fn pair(
    setup: &CoreSetup,
    debit: CoreRole,
    credit: CoreRole,
    value: Money,
    reference: &str,
) -> Result<Vec<JournalLineInput>> {
    Ok(vec![
        JournalLineInput {
            account_id: setup.account_id(debit)?.into(),
            debit: value,
            credit: Money::ZERO,
            description: reference.into(),

            dimension: String::new(),
        },
        JournalLineInput {
            account_id: setup.account_id(credit)?.into(),
            debit: Money::ZERO,
            credit: value,
            description: reference.into(),

            dimension: String::new(),
        },
    ])
}
fn financial_command(command: &ProcurementCommand, lines: Vec<JournalLineInput>) -> PostCommand {
    let c = command.context();
    PostCommand {
        company_id: c.company_id.clone(),
        period_id: c.period_id.clone(),
        date: c.date.clone(),
        source_id: c.source_id.clone(),
        source_revision: c.source_revision,
        event: command.event().into(),
        request_key: c.request_key.clone(),
        actor: c.actor.clone(),
        reason: c.reason.clone(),
        kind: JournalKind::Manual,
        lines,
    }
}
fn record_journal_lines(record: &ProcurementRecord) -> Result<Vec<JournalLineInput>> {
    match (&record.command, &record.data) {
        (ProcurementCommand::Receive(c), ProcurementData::Receive(r)) => pair(
            &record.frozen_setup,
            CoreRole::Inventory,
            CoreRole::Grni,
            r.total,
            &c.reference,
        ),
        (ProcurementCommand::Bill(c), ProcurementData::Bill(b)) => pair(
            &record.frozen_setup,
            CoreRole::Grni,
            CoreRole::Payables,
            b.total,
            &c.reference,
        ),
        (ProcurementCommand::Return(c), ProcurementData::Return(r)) => pair(
            &record.frozen_setup,
            if r.bill_id.is_some() {
                CoreRole::Payables
            } else {
                CoreRole::Grni
            },
            CoreRole::Inventory,
            r.total,
            &c.return_reference,
        ),
        _ => anyhow::bail!("رویداد خرید فاقد سند مالی است"),
    }
}

fn data_ids(data: &ProcurementData) -> Vec<&str> {
    match data {
        ProcurementData::Resources(x) => x
            .parties
            .iter()
            .map(|x| x.id.as_str())
            .chain(x.units.iter().map(|x| x.id.as_str()))
            .chain(x.items.iter().map(|x| x.id.as_str()))
            .chain(x.warehouses.iter().map(|x| x.id.as_str()))
            .collect(),
        ProcurementData::Order(x) => x.lines.iter().map(|x| x.input.id.as_str()).collect(),
        ProcurementData::Receive(x) => x.lines.iter().map(|x| x.id.as_str()).collect(),
        ProcurementData::Bill(x) => x.lines.iter().map(|x| x.id.as_str()).collect(),
        ProcurementData::Return(x) => std::iter::once(x.return_id.as_str())
            .chain(x.credit_id.as_deref())
            .chain(x.lines.iter().map(|l| l.id.as_str()))
            .collect(),
        ProcurementData::Transfer(_) => Vec::new(),
    }
}
