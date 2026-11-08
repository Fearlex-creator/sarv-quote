//! Linked goods orders, same-date delivery/invoice, and original-allocation restock credits.
use crate::accounting::{
    AccountingState, AuditOperation, BalanceSide, JournalKind, JournalLineInput, Money, PostCommand,
};
use crate::core::{
    CoreItem, CoreParty, CoreRole, CoreSetup, MAX_SOURCE_QUANTITY, OpenItemSide, civil_date,
    positive_half_up,
};
use crate::domain::{MAX_MONEY, id, normalize, text};
use crate::procurement::{
    ProcurementContext, ProcurementReconciliation, StockDirection, StockMovement, StockPool,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
pub const SALES_EVENT_PREFIX: &str = "sales.";
pub type SalesContext = ProcurementContext;
pub type SalesOrderLine = crate::procurement::OrderLine;
pub type FrozenSalesOrderLine = crate::procurement::FrozenOrderLine;
macro_rules! record {($n:ident {$($f:ident:$t:ty),* $(,)?})=>{
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)] #[serde(deny_unknown_fields)] pub struct $n {$(pub $f:$t),*}
};}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SalesState {
    pub records: Vec<SalesRecord>,
    pub requests: Vec<SalesRequest>,
}
impl SalesState {
    pub fn is_empty(&self) -> bool {
        self.records.is_empty() && self.requests.is_empty()
    }
    pub fn validate_transition(&self, next: &Self) -> Result<()> {
        ensure!(
            next.records.starts_with(&self.records) && next.requests.starts_with(&self.requests),
            "سابقهٔ فروش قابل تغییر یا حذف نیست"
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
record!(QuotationLink{quote_id:String,quote_series:String,quote_revision:u32,quote_number:String,quote_sha256:String,source_line_indices:Vec<usize>});
record!(SalesOrder{context:SalesContext,customer_id:String,reference:String,document_date:String,lines:Vec<SalesOrderLine>,document_discount:Money,supplied_total:Money,quotation:Option<QuotationLink>});
record!(FulfillLine {
    id: String,
    order_line_id: String,
    warehouse_id: String,
    unit_id: String,
    quantity_thousandths: i64
});
record!(FulfillOrder{context:SalesContext,order_id:String,delivery_id:String,invoice_id:String,delivery_reference:String,invoice_reference:String,document_date:String,due_date:String,lines:Vec<FulfillLine>,supplied_total:Money});
record!(ReturnLine {
    id: String,
    fulfillment_line_id: String,
    quantity_thousandths: i64,
    supplied_credit: Money,
    supplied_cost: Money
});
record!(ReturnSale{context:SalesContext,fulfillment_id:String,return_id:String,credit_id:String,return_reference:String,credit_reference:String,document_date:String,lines:Vec<ReturnLine>,supplied_credit:Money,supplied_cost:Money});
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SalesCommand {
    Order(SalesOrder),
    Fulfill(FulfillOrder),
    Return(ReturnSale),
}
impl SalesCommand {
    pub fn context(&self) -> &SalesContext {
        match self {
            Self::Order(x) => &x.context,
            Self::Fulfill(x) => &x.context,
            Self::Return(x) => &x.context,
        }
    }
    fn context_mut(&mut self) -> &mut SalesContext {
        match self {
            Self::Order(x) => &mut x.context,
            Self::Fulfill(x) => &mut x.context,
            Self::Return(x) => &mut x.context,
        }
    }
    pub fn event(&self) -> &'static str {
        match self {
            Self::Order(_) => "sales.order.v1",
            Self::Fulfill(_) => "sales.fulfill.v1",
            Self::Return(_) => "sales.return.v1",
        }
    }
}
record!(PostedSalesOrder{customer:CoreParty,lines:Vec<FrozenSalesOrderLine>,total:Money});
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryState {
    Delivered,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvoiceState {
    Posted,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReturnState {
    Restocked,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CreditState {
    Posted,
}
record!(PostedFulfillmentLine {
    id: String,
    order_line_id: String,
    item: CoreItem,
    warehouse_id: String,
    quantity_thousandths: i64,
    gross: Money,
    line_discount: Money,
    document_discount: Money,
    net: Money,
    cost: Money
});
record!(PostedFulfillment{order_id:String,customer:CoreParty,delivery_id:String,invoice_id:String,delivery_state:DeliveryState,invoice_state:InvoiceState,lines:Vec<PostedFulfillmentLine>,total:Money,cost:Money});
record!(PostedSalesReturnLine {
    id: String,
    fulfillment_line_id: String,
    item_id: String,
    warehouse_id: String,
    quantity_thousandths: i64,
    gross: Money,
    line_discount: Money,
    document_discount: Money,
    net: Money,
    cost: Money
});
record!(PostedSalesReturn{fulfillment_id:String,customer_id:String,return_id:String,credit_id:String,return_state:ReturnState,credit_state:CreditState,lines:Vec<PostedSalesReturnLine>,total:Money,cost:Money});
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SalesData {
    Order(PostedSalesOrder),
    Fulfill(PostedFulfillment),
    Return(PostedSalesReturn),
}
record!(SalesRecord{id:String,command:SalesCommand,command_sha256:String,frozen_setup:CoreSetup,sequence:u64,journal_id:Option<String>,data:SalesData,movements:Vec<StockMovement>});
record!(SalesRequest {
    request_key: String,
    company_id: String,
    record_id: String,
    business_sha256: String
});
record!(SalesResult{record_id:String,journal_id:Option<String>,sequence:u64});
record!(SalesPreview{data:SalesData,movements:Vec<StockMovement>,total:Money,cost:Money,journal_lines:Vec<JournalLineInput>,nonfinancial:bool});
record!(SalesOrderRemainder {
    id: String,
    item: CoreItem,
    warehouse_id: String,
    original_quantity_thousandths: i64,
    fulfilled_quantity_thousandths: i64,
    remaining_quantity_thousandths: i64,
    original_net: Money,
    remaining_net: Money
});
record!(SaleReturnRemainder {
    id: String,
    item: CoreItem,
    warehouse_id: String,
    original_quantity_thousandths: i64,
    returned_quantity_thousandths: i64,
    quantity_thousandths: i64,
    gross: Money,
    line_discount: Money,
    document_discount: Money,
    net: Money,
    cost: Money
});
record!(ReceivableRemainder{id:String,source_id:String,party_id:String,reference:String,document_date:String,due_date:String,original:Money,remaining:Money,credit_ids:Vec<String>});
record!(SalesLedgerAdjustment {
    journal_id: String,
    date: String,
    role: CoreRole,
    amount: Money
});
record!(SalesReconciliation{company_id:String,as_of:String,controls:Vec<crate::procurement::ProcurementControl>,stock:Vec<StockPool>,receipts:Vec<crate::procurement::ReceiptRemainder>,payables:Vec<crate::procurement::PayableRemainder>,receivables:Vec<ReceivableRemainder>,typed_sales:Money,typed_sales_returns:Money,typed_cogs:Money,adjustments:Vec<SalesLedgerAdjustment>,sales:Money,sales_returns:Money,cogs:Money,profit:Money,total_debit:Money,total_credit:Money,reconciled:bool});
fn hash<T: Serialize>(v: &T) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(v)?)))
}
fn business_hash(c: &SalesCommand) -> Result<String> {
    let mut c = c.clone();
    c.context_mut().request_key.clear();
    hash(&c)
}
fn result(r: &SalesRecord) -> SalesResult {
    SalesResult {
        record_id: r.id.clone(),
        journal_id: r.journal_id.clone(),
        sequence: r.sequence,
    }
}
fn money(v: Money) -> Result<()> {
    ensure!(
        (0..=MAX_MONEY).contains(&v.rials()),
        "مبلغ فروش خارج از محدوده یا منفی است"
    );
    Ok(())
}
fn sum<I: IntoIterator<Item = Money>>(v: I) -> Result<Money> {
    v.into_iter().try_fold(Money::ZERO, |a, b| a.checked_add(b))
}
fn doc_date(d: &str, f: &str) -> Result<()> {
    civil_date(d)?;
    ensure!(d <= f, "تاریخ سند پس از تاریخ مالی است");
    Ok(())
}
fn claim(v: &str, ids: &mut HashSet<String>) -> Result<()> {
    text(v)?;
    ensure!(
        ids.insert(v.into()),
        "شناسهٔ سند یا ردیف تکراری یا متعلق به سابقه است"
    );
    Ok(())
}
fn base(i: &CoreItem, u: &str, q: i64) -> Result<i64> {
    ensure!(
        (1..=MAX_SOURCE_QUANTITY).contains(&q),
        "مقدار باید مثبت و در محدوده باشد"
    );
    let (n, d) = if u == i.base_unit_id {
        (1, 1)
    } else {
        let p = i
            .packs
            .iter()
            .find(|p| p.unit_id == u)
            .ok_or_else(|| anyhow::anyhow!("ضریب بسته‌بندی مشخص نیست"))?;
        (p.numerator, p.denominator)
    };
    ensure!(n > 0 && d > 0, "نسبت واحد نامعتبر است");
    let v = i128::from(q) * i128::from(n);
    ensure!(v % i128::from(d) == 0, "تبدیل در هزارم واحد پایه دقیق نیست");
    let v = i64::try_from(v / i128::from(d))?;
    ensure!(
        v > 0 && (!i.indivisible || v % 1000 == 0),
        "مقدار واحد پایه غیرقابل‌تقسیم است"
    );
    Ok(v)
}
pub(crate) fn allocation(
    v: Money,
    q: i64,
    priorq: i64,
    priorv: Money,
    nextq: i64,
) -> Result<Money> {
    ensure!(
        q > 0 && priorq >= 0 && nextq > 0 && v.rials() >= 0 && priorv.rials() >= 0,
        "مقدار تخصیص نامعتبر است"
    );
    let cumulative = priorq
        .checked_add(nextq)
        .ok_or_else(|| anyhow::anyhow!("سرریز مقدار تخصیص"))?;
    ensure!(cumulative <= q, "مقدار از ماندهٔ منبع بیشتر است");
    let target = if cumulative == q {
        v
    } else {
        Money::from_rials(positive_half_up(
            i128::from(v.rials()) * i128::from(cumulative),
            i128::from(q),
        )?)
    };
    let out = target.checked_sub(priorv)?;
    ensure!(out.rials() >= 0, "بهای تخصیص نامعتبر است");
    Ok(out)
}
fn discounts(lines: &[SalesOrderLine], weights: &[Money], d: Money) -> Result<Vec<Money>> {
    money(d)?;
    let total = sum(weights.iter().copied())?;
    ensure!(
        d.rials() <= total.rials(),
        "تخفیف سند از مبلغ کالا بیشتر است"
    );
    if total == Money::ZERO {
        return Ok(vec![Money::ZERO; weights.len()]);
    }
    let mut out = Vec::new();
    let mut remainder = Vec::new();
    let mut assigned = 0i64;
    for (i, w) in weights.iter().enumerate() {
        let p = i128::from(w.rials()) * i128::from(d.rials());
        let div = i128::from(total.rials());
        let v = i64::try_from(p / div)?;
        assigned = assigned
            .checked_add(v)
            .ok_or_else(|| anyhow::anyhow!("سرریز تخفیف"))?;
        out.push(Money::from_rials(v));
        remainder.push((p % div, lines[i].id.as_str(), i));
    }
    remainder.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(b.1)));
    for (_, _, i) in remainder
        .into_iter()
        .take(usize::try_from(d.rials() - assigned)?)
    {
        out[i] = out[i].checked_add(Money::from_rials(1))?;
    }
    Ok(out)
}
fn empty_pool(i: &str, w: &str) -> StockPool {
    StockPool {
        item_id: i.into(),
        warehouse_id: w.into(),
        quantity_thousandths: 0,
        value: Money::ZERO,
    }
}
fn movement(
    c: &SalesContext,
    l: &str,
    p: &mut StockPool,
    d: StockDirection,
    q: i64,
    v: Money,
) -> Result<StockMovement> {
    ensure!(q > 0 && v.rials() >= 0, "مقدار/بهای حرکت نامعتبر است");
    let before = p.clone();
    match d {
        StockDirection::In => {
            p.quantity_thousandths = p
                .quantity_thousandths
                .checked_add(q)
                .ok_or_else(|| anyhow::anyhow!("سرریز مقدار موجودی"))?;
            p.value = p.value.checked_add(v)?
        }
        StockDirection::Out => {
            p.quantity_thousandths = p
                .quantity_thousandths
                .checked_sub(q)
                .ok_or_else(|| anyhow::anyhow!("سرریز مقدار موجودی"))?;
            p.value = p.value.checked_sub(v)?
        }
    }
    ensure!(
        p.quantity_thousandths >= 0
            && p.value.rials() >= 0
            && (p.quantity_thousandths != 0 || p.value == Money::ZERO),
        "موجودی منفی یا بهای بدون مقدار ممنوع است"
    );
    Ok(StockMovement {
        id: format!(
            "sales-stock:{}",
            hash(&(c.company_id.as_str(), c.source_id.as_str(), l, d))?
        ),
        item_id: p.item_id.clone(),
        warehouse_id: p.warehouse_id.clone(),
        direction: d,
        quantity_thousandths: q,
        value: v,
        source_line_id: l.into(),
        before,
        after: p.clone(),
    })
}
fn pair(
    s: &CoreSetup,
    d: CoreRole,
    c: CoreRole,
    v: Money,
    r: &str,
) -> Result<Vec<JournalLineInput>> {
    if v == Money::ZERO {
        return Ok(Vec::new());
    }
    Ok(vec![
        JournalLineInput {
            account_id: s.account_id(d)?.into(),
            debit: v,
            credit: Money::ZERO,
            description: r.into(),

            dimension: String::new(),
        },
        JournalLineInput {
            account_id: s.account_id(c)?.into(),
            debit: Money::ZERO,
            credit: v,
            description: r.into(),

            dimension: String::new(),
        },
    ])
}
fn financial_command(c: &SalesCommand, lines: Vec<JournalLineInput>) -> PostCommand {
    let x = c.context();
    PostCommand {
        company_id: x.company_id.clone(),
        period_id: x.period_id.clone(),
        date: x.date.clone(),
        source_id: x.source_id.clone(),
        source_revision: x.source_revision,
        event: c.event().into(),
        request_key: x.request_key.clone(),
        actor: x.actor.clone(),
        reason: x.reason.clone(),
        kind: JournalKind::Manual,
        lines,
    }
}
fn record_journal_lines(r: &SalesRecord) -> Result<Vec<JournalLineInput>> {
    let mut out = Vec::new();
    match (&r.command, &r.data) {
        (SalesCommand::Fulfill(c), SalesData::Fulfill(v)) => {
            out.extend(pair(
                &r.frozen_setup,
                CoreRole::Receivables,
                CoreRole::Sales,
                v.total,
                &c.invoice_reference,
            )?);
            out.extend(pair(
                &r.frozen_setup,
                CoreRole::Cogs,
                CoreRole::Inventory,
                v.cost,
                &c.delivery_reference,
            )?);
        }
        (SalesCommand::Return(c), SalesData::Return(v)) => {
            out.extend(pair(
                &r.frozen_setup,
                CoreRole::SalesReturns,
                CoreRole::Receivables,
                v.total,
                &c.credit_reference,
            )?);
            out.extend(pair(
                &r.frozen_setup,
                CoreRole::Inventory,
                CoreRole::Cogs,
                v.cost,
                &c.return_reference,
            )?);
        }
        _ => anyhow::bail!("رویداد فروش فاقد سند مالی است"),
    };
    Ok(out)
}
fn data_ids(d: &SalesData) -> Vec<&str> {
    match d {
        SalesData::Order(x) => x.lines.iter().map(|l| l.input.id.as_str()).collect(),
        SalesData::Fulfill(x) => [x.delivery_id.as_str(), x.invoice_id.as_str()]
            .into_iter()
            .chain(x.lines.iter().map(|l| l.id.as_str()))
            .collect(),
        SalesData::Return(x) => [x.return_id.as_str(), x.credit_id.as_str()]
            .into_iter()
            .chain(x.lines.iter().map(|l| l.id.as_str()))
            .collect(),
    }
}
impl AccountingState {
    pub fn sales_document(&self, id: &str) -> Result<&SalesRecord> {
        self.sales
            .records
            .iter()
            .find(|r| r.id == id)
            .ok_or_else(|| anyhow::anyhow!("سند فروش پیدا نشد"))
    }
    pub(crate) fn sales_entity_ids(&self) -> Vec<&str> {
        let mut out = Vec::new();
        for r in &self.sales.records {
            out.push(r.id.as_str());
            out.extend(data_ids(&r.data));
            out.extend(r.movements.iter().map(|m| m.id.as_str()));
        }
        out
    }
    pub fn sales_order_remaining(&self, id: &str) -> Result<Vec<SalesOrderRemainder>> {
        let r = self.sales_document(id)?;
        let SalesData::Order(o) = &r.data else {
            anyhow::bail!("منبع سفارش فروش نیست")
        };
        let mut out = Vec::new();
        for l in &o.lines {
            let mut q = 0i64;
            let mut v = Money::ZERO;
            for r in &self.sales.records {
                if let SalesData::Fulfill(f) = &r.data
                    && f.order_id == id
                {
                    for x in f.lines.iter().filter(|x| x.order_line_id == l.input.id) {
                        q = q
                            .checked_add(x.quantity_thousandths)
                            .ok_or_else(|| anyhow::anyhow!("سرریز مقدار تحویل"))?;
                        v = v.checked_add(x.net)?;
                    }
                }
            }
            let remaining = l
                .base_quantity_thousandths
                .checked_sub(q)
                .ok_or_else(|| anyhow::anyhow!("سرریز ماندهٔ سفارش"))?;
            ensure!(remaining >= 0, "تحویل از سفارش بیشتر است");
            out.push(SalesOrderRemainder {
                id: l.input.id.clone(),
                item: l.item.clone(),
                warehouse_id: l.input.warehouse_id.clone(),
                original_quantity_thousandths: l.base_quantity_thousandths,
                fulfilled_quantity_thousandths: q,
                remaining_quantity_thousandths: remaining,
                original_net: l.net,
                remaining_net: l.net.checked_sub(v)?,
            });
        }
        Ok(out)
    }
    pub fn sales_invoice_returnable(&self, id: &str) -> Result<Vec<SaleReturnRemainder>> {
        let r = self.sales_document(id)?;
        let SalesData::Fulfill(f) = &r.data else {
            anyhow::bail!("منبع برگشت تحویل و فاکتور نیست")
        };
        let mut out = Vec::new();
        for l in &f.lines {
            let mut q = 0i64;
            let (mut gross, mut ld, mut dd, mut net, mut cost) = (
                Money::ZERO,
                Money::ZERO,
                Money::ZERO,
                Money::ZERO,
                Money::ZERO,
            );
            for r in &self.sales.records {
                if let SalesData::Return(v) = &r.data
                    && v.fulfillment_id == id
                {
                    for x in v.lines.iter().filter(|x| x.fulfillment_line_id == l.id) {
                        q = q
                            .checked_add(x.quantity_thousandths)
                            .ok_or_else(|| anyhow::anyhow!("سرریز مقدار برگشت"))?;
                        gross = gross.checked_add(x.gross)?;
                        ld = ld.checked_add(x.line_discount)?;
                        dd = dd.checked_add(x.document_discount)?;
                        net = net.checked_add(x.net)?;
                        cost = cost.checked_add(x.cost)?;
                    }
                }
            }
            let remaining = l
                .quantity_thousandths
                .checked_sub(q)
                .ok_or_else(|| anyhow::anyhow!("سرریز مقدار برگشت"))?;
            ensure!(remaining >= 0, "برگشت از تحویل بیشتر است");
            out.push(SaleReturnRemainder {
                id: l.id.clone(),
                item: l.item.clone(),
                warehouse_id: l.warehouse_id.clone(),
                original_quantity_thousandths: l.quantity_thousandths,
                returned_quantity_thousandths: q,
                quantity_thousandths: remaining,
                gross: l.gross.checked_sub(gross)?,
                line_discount: l.line_discount.checked_sub(ld)?,
                document_discount: l.document_discount.checked_sub(dd)?,
                net: l.net.checked_sub(net)?,
                cost: l.cost.checked_sub(cost)?,
            });
        }
        Ok(out)
    }
    pub fn sales_receivables(&self, company: &str, asof: &str) -> Result<Vec<ReceivableRemainder>> {
        civil_date(asof)?;
        let o = self
            .core
            .openings
            .iter()
            .find(|o| o.command.company_id == company)
            .ok_or_else(|| anyhow::anyhow!("افتتاحیه پیدا نشد"))?;
        ensure!(
            asof >= o.command.as_of.as_str(),
            "گزارش پیش از افتتاحیه پشتیبانی نمی‌شود"
        );
        let settled = self.payment_allocation_map(company, asof)?;
        let mut out: Vec<_> = o
            .command
            .open_items
            .iter()
            .filter(|x| x.side == OpenItemSide::Receivable)
            .map(|x| -> Result<ReceivableRemainder> {
                Ok(ReceivableRemainder {
                    id: x.id.clone(),
                    source_id: o.command.source_id.clone(),
                    party_id: x.party_id.clone(),
                    reference: x.document_reference.clone(),
                    document_date: x.document_date.clone(),
                    due_date: x.due_date.clone(),
                    original: x.remaining,
                    remaining: x
                        .remaining
                        .checked_add(settled.get(&x.id).copied().unwrap_or(Money::ZERO))?,
                    credit_ids: Vec::new(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        for r in self.sales.records.iter().filter(|r| {
            r.command.context().company_id == company && r.command.context().date.as_str() <= asof
        }) {
            if let (SalesCommand::Fulfill(c), SalesData::Fulfill(f)) = (&r.command, &r.data) {
                let mut remaining = f.total;
                let mut credit_ids = Vec::new();
                for other in self
                    .sales
                    .records
                    .iter()
                    .filter(|other| other.command.context().date.as_str() <= asof)
                {
                    if let SalesData::Return(ret) = &other.data
                        && ret.fulfillment_id == r.id
                    {
                        remaining = remaining.checked_sub(ret.total)?;
                        credit_ids.push(ret.credit_id.clone());
                    }
                }
                remaining = remaining.checked_add(
                    settled
                        .get(f.invoice_id.as_str())
                        .copied()
                        .unwrap_or(Money::ZERO),
                )?;
                ensure!(remaining.rials() >= 0, "اعتبار از دریافتنی بیشتر است");
                out.push(ReceivableRemainder {
                    id: f.invoice_id.clone(),
                    source_id: c.context.source_id.clone(),
                    party_id: f.customer.id.clone(),
                    reference: c.invoice_reference.clone(),
                    document_date: c.document_date.clone(),
                    due_date: c.due_date.clone(),
                    original: f.total,
                    remaining,
                    credit_ids,
                });
            }
        }
        Ok(out)
    }
    fn typed_sales_detail(&self, role: CoreRole, company: &str, asof: &str) -> Result<Money> {
        let mut total = Money::ZERO;
        for r in self.sales.records.iter().filter(|r| {
            r.command.context().company_id == company && r.command.context().date.as_str() <= asof
        }) {
            match (&r.data, role) {
                (SalesData::Fulfill(f), CoreRole::Sales) => total = total.checked_add(f.total)?,
                (SalesData::Return(f), CoreRole::SalesReturns) => {
                    total = total.checked_add(f.total)?
                }
                (SalesData::Fulfill(f), CoreRole::Cogs) => total = total.checked_add(f.cost)?,
                (SalesData::Return(f), CoreRole::Cogs) => total = total.checked_sub(f.cost)?,
                _ => {}
            }
        }
        Ok(total)
    }
    /// Ordinary journals retain their own provenance; they never become invoices.
    pub fn sales_ledger_adjustments(
        &self,
        company: &str,
        asof: &str,
    ) -> Result<Vec<SalesLedgerAdjustment>> {
        civil_date(asof)?;
        let opening = self
            .core
            .openings
            .iter()
            .find(|o| o.command.company_id == company)
            .ok_or_else(|| anyhow::anyhow!("افتتاحیه پیدا نشد"))?;
        ensure!(
            asof >= opening.command.as_of.as_str(),
            "گزارش پیش از افتتاحیه پشتیبانی نمی‌شود"
        );
        let mut out = Vec::new();
        for journal in self.journals.iter().filter(|j| {
            j.company_id == company
                && j.date.as_str() <= asof
                && !self.has_typed_journal_source(&j.id)
        }) {
            for role in [CoreRole::Sales, CoreRole::SalesReturns, CoreRole::Cogs] {
                let account = opening.frozen_setup.account_id(role)?;
                let amount = journal
                    .lines
                    .iter()
                    .filter(|l| l.account.id == account)
                    .try_fold(Money::ZERO, |total, line| {
                        let delta = if role.normal_balance() == BalanceSide::Debit {
                            line.debit.checked_sub(line.credit)?
                        } else {
                            line.credit.checked_sub(line.debit)?
                        };
                        total.checked_add(delta)
                    })?;
                if amount != Money::ZERO {
                    out.push(SalesLedgerAdjustment {
                        journal_id: journal.id.clone(),
                        date: journal.date.clone(),
                        role,
                        amount,
                    });
                }
            }
        }
        Ok(out)
    }
    pub fn sales_control_detail(&self, role: CoreRole, company: &str, asof: &str) -> Result<Money> {
        civil_date(asof)?;
        if role == CoreRole::Receivables {
            return sum(self
                .sales_receivables(company, asof)?
                .iter()
                .map(|r| r.remaining));
        }
        ensure!(
            matches!(
                role,
                CoreRole::Sales | CoreRole::SalesReturns | CoreRole::Cogs
            ),
            "نقش جزئیات فروش پشتیبانی نمی‌شود"
        );
        let typed = self.typed_sales_detail(role, company, asof)?;
        let adjustment = sum(self
            .sales_ledger_adjustments(company, asof)?
            .iter()
            .filter(|a| a.role == role)
            .map(|a| a.amount))?;
        typed.checked_add(adjustment)
    }
    pub fn sales_reconcile(&self, company: &str, asof: &str) -> Result<SalesReconciliation> {
        let ProcurementReconciliation {
            mut controls,
            stock,
            receipts,
            payables,
            total_debit,
            total_credit,
            reconciled,
            ..
        } = self.procurement_reconcile(company, asof)?;
        let sales = self.sales_control_detail(CoreRole::Sales, company, asof)?;
        let sales_returns = self.sales_control_detail(CoreRole::SalesReturns, company, asof)?;
        let cogs = self.sales_control_detail(CoreRole::Cogs, company, asof)?;
        let tb = self.trial_balance(company, asof)?;
        let setup = &self
            .core
            .openings
            .iter()
            .find(|o| o.command.company_id == company)
            .ok_or_else(|| anyhow::anyhow!("افتتاحیه پیدا نشد"))?
            .frozen_setup;
        for (role, detail) in [
            (CoreRole::Sales, sales),
            (CoreRole::SalesReturns, sales_returns),
            (CoreRole::Cogs, cogs),
        ] {
            let account = setup.account_id(role)?;
            let ledger = if let Some(row) = tb.rows.iter().find(|r| r.account_id == account) {
                if role.normal_balance() == BalanceSide::Debit {
                    row.debit.checked_sub(row.credit)?
                } else {
                    row.credit.checked_sub(row.debit)?
                }
            } else {
                Money::ZERO
            };
            let row = crate::procurement::ProcurementControl {
                role,
                account_id: account.into(),
                ledger,
                detail,
                difference: ledger.checked_sub(detail)?,
            };
            if let Some(existing) = controls.iter_mut().find(|r| r.role == role) {
                *existing = row;
            } else {
                controls.push(row);
            }
        }
        let reconciled = reconciled && controls.iter().all(|r| r.difference == Money::ZERO);
        Ok(SalesReconciliation {
            company_id: company.into(),
            as_of: asof.into(),
            controls,
            stock,
            receipts,
            payables,
            receivables: self.sales_receivables(company, asof)?,
            typed_sales: self.typed_sales_detail(CoreRole::Sales, company, asof)?,
            typed_sales_returns: self.typed_sales_detail(CoreRole::SalesReturns, company, asof)?,
            typed_cogs: self.typed_sales_detail(CoreRole::Cogs, company, asof)?,
            adjustments: self.sales_ledger_adjustments(company, asof)?,
            sales,
            sales_returns,
            cogs,
            profit: sales.checked_sub(sales_returns)?.checked_sub(cogs)?,
            total_debit,
            total_credit,
            reconciled,
        })
    }
    pub fn preview_sales(&self, command: &SalesCommand) -> Result<SalesPreview> {
        let c = command.context();
        let setup = self.validate_operational_context(c)?;
        let resources = self.procurement_resources(&c.company_id)?;
        let mut ids = self.operational_known_ids();
        let warehouse = |v: &str| -> Result<()> {
            ensure!(
                resources.warehouses.iter().any(|w| w.id == v),
                "انبار معتبر نیست"
            );
            Ok(())
        };
        let mut movements = Vec::new();
        let mut journal_lines = Vec::new();
        let (data, total, cost) = match command {
            SalesCommand::Order(x) => {
                text(&x.reference)?;
                doc_date(&x.document_date, &c.date)?;
                money(x.supplied_total)?;
                ensure!(
                    (1..=10_000).contains(&x.lines.len()),
                    "تعداد ردیف سفارش نامعتبر است"
                );
                let customer = resources
                    .parties
                    .iter()
                    .find(|p| p.id == x.customer_id && p.customer)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("شخص نقش مشتری ندارد"))?;
                ensure!(!self.sales.records.iter().any(|r|matches!(&r.command,SalesCommand::Order(p)if p.context.company_id==c.company_id&&p.customer_id==x.customer_id&&normalize(&p.reference)==normalize(&x.reference)&&p.document_date==x.document_date)),"مرجع سفارش این مشتری تکراری است");
                if let Some(q) = &x.quotation {
                    for v in [&q.quote_id, &q.quote_series, &q.quote_number] {
                        text(v)?;
                    }
                    ensure!(
                        q.quote_revision > 0
                            && q.quote_sha256.len() == 64
                            && q.quote_sha256.bytes().all(|b| b.is_ascii_hexdigit()),
                        "هویت تصویر پیش‌فاکتور نامعتبر است"
                    );
                    let mut ix = HashSet::new();
                    ensure!(
                        q.source_line_indices.len() == x.lines.len()
                            && q.source_line_indices.iter().all(|i| ix.insert(*i)),
                        "ردیف‌های پیوند پیش‌فاکتور نامعتبر است"
                    );
                    ensure!(!self.sales.records.iter().any(|r|matches!(&r.command,SalesCommand::Order(p)if p.context.company_id==c.company_id&&p.quotation.as_ref().is_some_and(|old|old.quote_id==q.quote_id))),"این پیش‌فاکتور قبلاً به سفارش تبدیل شده است");
                }
                let mut frozen = Vec::new();
                let mut weights = Vec::new();
                for l in &x.lines {
                    claim(&l.id, &mut ids)?;
                    warehouse(&l.warehouse_id)?;
                    text(&l.price_reference)?;
                    doc_date(&l.price_date, &c.date)?;
                    if let Some(r) = &l.override_reason {
                        text(r)?;
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
                    let net = gross.checked_sub(l.line_discount)?;
                    weights.push(net);
                    frozen.push(FrozenSalesOrderLine {
                        input: l.clone(),
                        item: item.clone(),
                        base_quantity_thousandths: q,
                        gross,
                        document_discount: Money::ZERO,
                        net,
                    });
                }
                for (l, d) in
                    frozen
                        .iter_mut()
                        .zip(discounts(&x.lines, &weights, x.document_discount)?)
                {
                    l.document_discount = d;
                    l.net = l.net.checked_sub(d)?;
                }
                let total = sum(frozen.iter().map(|l| l.net))?;
                ensure!(total == x.supplied_total, "جمع مستقل سفارش برابر نیست");
                (
                    SalesData::Order(PostedSalesOrder {
                        customer,
                        lines: frozen,
                        total,
                    }),
                    total,
                    Money::ZERO,
                )
            }
            SalesCommand::Fulfill(x) => {
                for v in [&x.delivery_reference, &x.invoice_reference] {
                    text(v)?;
                }
                claim(&x.delivery_id, &mut ids)?;
                claim(&x.invoice_id, &mut ids)?;
                doc_date(&x.document_date, &c.date)?;
                civil_date(&x.due_date)?;
                ensure!(
                    x.due_date >= x.document_date,
                    "سررسید پیش از تاریخ فاکتور است"
                );
                money(x.supplied_total)?;
                ensure!(
                    (1..=10_000).contains(&x.lines.len()),
                    "تعداد ردیف تحویل نامعتبر است"
                );
                ensure!(
                    c.date.as_str() >= self.operational_stock_date(&c.company_id)?,
                    "ثبت موجودی پیش از آخرین تاریخ موجودی شرکت ممنوع است"
                );
                let record = self.sales_document(&x.order_id)?;
                ensure!(
                    record.command.context().company_id == c.company_id
                        && c.date >= record.command.context().date,
                    "شرکت یا تاریخ تحویل با سفارش سازگار نیست"
                );
                let SalesData::Order(order) = &record.data else {
                    anyhow::bail!("منبع تحویل سفارش فروش نیست")
                };
                ensure!(
                    !self
                        .core
                        .openings
                        .iter()
                        .filter(|o| o.command.company_id == c.company_id)
                        .flat_map(|o| &o.command.open_items)
                        .any(|r| r.side == OpenItemSide::Receivable
                            && r.party_id == order.customer.id
                            && normalize(&r.document_reference) == normalize(&x.invoice_reference)
                            && r.document_date == x.document_date),
                    "این فاکتور قبلاً در افتتاحیه منظور شده است"
                );
                ensure!(!self.sales.records.iter().any(|r|matches!((&r.command,&r.data),(SalesCommand::Fulfill(p),SalesData::Fulfill(f))if p.context.company_id==c.company_id&&f.customer.id==order.customer.id&&p.document_date==x.document_date&&(normalize(&p.invoice_reference)==normalize(&x.invoice_reference)||normalize(&p.delivery_reference)==normalize(&x.delivery_reference)))),"مرجع تحویل یا فاکتور مشتری تکراری است");
                let remaining = self.sales_order_remaining(&x.order_id)?;
                let mut source_ids = HashSet::new();
                let mut pools = self.operational_stock_pools(&c.company_id, "9999-12-31")?;
                let mut lines = Vec::new();
                for l in &x.lines {
                    claim(&l.id, &mut ids)?;
                    warehouse(&l.warehouse_id)?;
                    ensure!(
                        source_ids.insert(&l.order_line_id),
                        "ردیف سفارش در تحویل تکراری است"
                    );
                    let orig = order
                        .lines
                        .iter()
                        .find(|a| a.input.id == l.order_line_id)
                        .ok_or_else(|| anyhow::anyhow!("ردیف سفارش پیدا نشد"))?;
                    let r = remaining.iter().find(|r| r.id == l.order_line_id).unwrap();
                    let q = base(&orig.item, &l.unit_id, l.quantity_thousandths)?;
                    let priorq = r.fulfilled_quantity_thousandths;
                    let (mut priorld, mut priordd) = (Money::ZERO, Money::ZERO);
                    for record in &self.sales.records {
                        if let SalesData::Fulfill(f) = &record.data
                            && f.order_id == x.order_id
                        {
                            for line in f
                                .lines
                                .iter()
                                .filter(|line| line.order_line_id == l.order_line_id)
                            {
                                priorld = priorld.checked_add(line.line_discount)?;
                                priordd = priordd.checked_add(line.document_discount)?;
                            }
                        }
                    }
                    let net = allocation(
                        orig.net,
                        orig.base_quantity_thousandths,
                        priorq,
                        orig.net.checked_sub(r.remaining_net)?,
                        q,
                    )?;
                    let ld = allocation(
                        orig.input.line_discount,
                        orig.base_quantity_thousandths,
                        priorq,
                        priorld,
                        q,
                    )?;
                    let dd = allocation(
                        orig.document_discount,
                        orig.base_quantity_thousandths,
                        priorq,
                        priordd,
                        q,
                    )?;
                    let gross = net.checked_add(ld)?.checked_add(dd)?;
                    let p = pools
                        .entry((orig.item.id.clone(), l.warehouse_id.clone()))
                        .or_insert_with(|| empty_pool(&orig.item.id, &l.warehouse_id));
                    ensure!(q <= p.quantity_thousandths, "موجودی انبار کافی نیست");
                    let cost = if q == p.quantity_thousandths {
                        p.value
                    } else {
                        Money::from_rials(positive_half_up(
                            i128::from(p.value.rials()) * i128::from(q),
                            i128::from(p.quantity_thousandths),
                        )?)
                    };
                    movements.push(movement(c, &l.id, p, StockDirection::Out, q, cost)?);
                    lines.push(PostedFulfillmentLine {
                        id: l.id.clone(),
                        order_line_id: l.order_line_id.clone(),
                        item: orig.item.clone(),
                        warehouse_id: l.warehouse_id.clone(),
                        quantity_thousandths: q,
                        gross,
                        line_discount: ld,
                        document_discount: dd,
                        net,
                        cost,
                    });
                }
                let total = sum(lines.iter().map(|l| l.net))?;
                let cost = sum(lines.iter().map(|l| l.cost))?;
                ensure!(
                    total == x.supplied_total,
                    "جمع مستقل فاکتور با تخصیص سفارش برابر نیست"
                );
                journal_lines.extend(pair(
                    setup,
                    CoreRole::Receivables,
                    CoreRole::Sales,
                    total,
                    &x.invoice_reference,
                )?);
                journal_lines.extend(pair(
                    setup,
                    CoreRole::Cogs,
                    CoreRole::Inventory,
                    cost,
                    &x.delivery_reference,
                )?);
                (
                    SalesData::Fulfill(PostedFulfillment {
                        order_id: x.order_id.clone(),
                        customer: order.customer.clone(),
                        delivery_id: x.delivery_id.clone(),
                        invoice_id: x.invoice_id.clone(),
                        delivery_state: DeliveryState::Delivered,
                        invoice_state: InvoiceState::Posted,
                        lines,
                        total,
                        cost,
                    }),
                    total,
                    cost,
                )
            }
            SalesCommand::Return(x) => {
                for v in [&x.return_reference, &x.credit_reference] {
                    text(v)?;
                }
                claim(&x.return_id, &mut ids)?;
                claim(&x.credit_id, &mut ids)?;
                doc_date(&x.document_date, &c.date)?;
                money(x.supplied_credit)?;
                money(x.supplied_cost)?;
                ensure!(
                    (1..=10_000).contains(&x.lines.len()),
                    "تعداد ردیف برگشت نامعتبر است"
                );
                ensure!(
                    c.date.as_str() >= self.operational_stock_date(&c.company_id)?,
                    "ثبت موجودی پیش از آخرین تاریخ موجودی شرکت ممنوع است"
                );
                let record = self.sales_document(&x.fulfillment_id)?;
                ensure!(
                    record.command.context().company_id == c.company_id
                        && c.date >= record.command.context().date,
                    "شرکت یا تاریخ برگشت با تحویل سازگار نیست"
                );
                let SalesData::Fulfill(f) = &record.data else {
                    anyhow::bail!("منبع برگشت تحویل و فاکتور نیست")
                };
                ensure!(!self.sales.records.iter().any(|r|matches!((&r.command,&r.data),(SalesCommand::Return(p),SalesData::Return(v))if p.context.company_id==c.company_id&&v.customer_id==f.customer.id&&p.document_date==x.document_date&&(normalize(&p.return_reference)==normalize(&x.return_reference)||normalize(&p.credit_reference)==normalize(&x.credit_reference)))),"مرجع برگشت یا اعتبار مشتری تکراری است");
                let remaining = self.sales_invoice_returnable(&x.fulfillment_id)?;
                let mut source_ids = HashSet::new();
                let mut pools = self.operational_stock_pools(&c.company_id, "9999-12-31")?;
                let mut lines = Vec::new();
                for l in &x.lines {
                    claim(&l.id, &mut ids)?;
                    ensure!(
                        source_ids.insert(&l.fulfillment_line_id),
                        "ردیف تحویل در برگشت تکراری است"
                    );
                    money(l.supplied_credit)?;
                    money(l.supplied_cost)?;
                    let orig = f
                        .lines
                        .iter()
                        .find(|orig| orig.id == l.fulfillment_line_id)
                        .ok_or_else(|| anyhow::anyhow!("ردیف تحویل پیدا نشد"))?;
                    ensure!(
                        l.quantity_thousandths > 0
                            && l.quantity_thousandths <= MAX_SOURCE_QUANTITY
                            && (!orig.item.indivisible || l.quantity_thousandths % 1000 == 0),
                        "مقدار برگشت واحد پایه نامعتبر است"
                    );
                    let r = remaining
                        .iter()
                        .find(|r| r.id == l.fulfillment_line_id)
                        .unwrap();
                    let q = l.quantity_thousandths;
                    let priorq = r.returned_quantity_thousandths;
                    let net = allocation(
                        orig.net,
                        orig.quantity_thousandths,
                        priorq,
                        orig.net.checked_sub(r.net)?,
                        q,
                    )?;
                    let ld = allocation(
                        orig.line_discount,
                        orig.quantity_thousandths,
                        priorq,
                        orig.line_discount.checked_sub(r.line_discount)?,
                        q,
                    )?;
                    let dd = allocation(
                        orig.document_discount,
                        orig.quantity_thousandths,
                        priorq,
                        orig.document_discount.checked_sub(r.document_discount)?,
                        q,
                    )?;
                    let cost = allocation(
                        orig.cost,
                        orig.quantity_thousandths,
                        priorq,
                        orig.cost.checked_sub(r.cost)?,
                        q,
                    )?;
                    ensure!(
                        net == l.supplied_credit && cost == l.supplied_cost,
                        "اعتبار یا بهای مستقل ردیف برگشت با تخصیص اصلی برابر نیست"
                    );
                    let gross = net.checked_add(ld)?.checked_add(dd)?;
                    let p = pools
                        .entry((orig.item.id.clone(), orig.warehouse_id.clone()))
                        .or_insert_with(|| empty_pool(&orig.item.id, &orig.warehouse_id));
                    movements.push(movement(c, &l.id, p, StockDirection::In, q, cost)?);
                    lines.push(PostedSalesReturnLine {
                        id: l.id.clone(),
                        fulfillment_line_id: l.fulfillment_line_id.clone(),
                        item_id: orig.item.id.clone(),
                        warehouse_id: orig.warehouse_id.clone(),
                        quantity_thousandths: q,
                        gross,
                        line_discount: ld,
                        document_discount: dd,
                        net,
                        cost,
                    });
                }
                let total = sum(lines.iter().map(|l| l.net))?;
                let cost = sum(lines.iter().map(|l| l.cost))?;
                ensure!(
                    total == x.supplied_credit && cost == x.supplied_cost,
                    "جمع مستقل برگشت با تخصیص اصلی برابر نیست"
                );
                journal_lines.extend(pair(
                    setup,
                    CoreRole::SalesReturns,
                    CoreRole::Receivables,
                    total,
                    &x.credit_reference,
                )?);
                journal_lines.extend(pair(
                    setup,
                    CoreRole::Inventory,
                    CoreRole::Cogs,
                    cost,
                    &x.return_reference,
                )?);
                (
                    SalesData::Return(PostedSalesReturn {
                        fulfillment_id: x.fulfillment_id.clone(),
                        customer_id: f.customer.id.clone(),
                        return_id: x.return_id.clone(),
                        credit_id: x.credit_id.clone(),
                        return_state: ReturnState::Restocked,
                        credit_state: CreditState::Posted,
                        lines,
                        total,
                        cost,
                    }),
                    total,
                    cost,
                )
            }
        };
        money(total)?;
        money(cost)?;
        for m in &movements {
            claim(&m.id, &mut ids)?;
        }
        let nonfinancial = journal_lines.is_empty();
        Ok(SalesPreview {
            data,
            movements,
            total,
            cost,
            journal_lines,
            nonfinancial,
        })
    }
    pub fn post_sales_command(&mut self, command: SalesCommand) -> Result<SalesResult> {
        let mut next = self.clone();
        let out = next.post_sales_inner(command)?;
        *self = next;
        Ok(out)
    }
    fn post_sales_inner(&mut self, command: SalesCommand) -> Result<SalesResult> {
        let c = command.context();
        text(&c.request_key)?;
        let business = business_hash(&command)?;
        ensure!(
            !self
                .core
                .zero_requests
                .iter()
                .any(|r| r.request_key == c.request_key)
                && !self
                    .procurement
                    .requests
                    .iter()
                    .any(|r| r.request_key == c.request_key),
            "کلید درخواست قبلاً برای شروع دفتر یا خرید استفاده شده است"
        );
        if let Some(alias) = self
            .sales
            .requests
            .iter()
            .find(|r| r.request_key == c.request_key)
        {
            ensure!(
                alias.company_id == c.company_id && alias.business_sha256 == business,
                "کلید درخواست با محتوای متفاوت استفاده شده است"
            );
            let prior = self.sales_document(&alias.record_id)?;
            ensure!(
                prior.command_sha256 == hash(&prior.command)?
                    && business_hash(&prior.command)? == business
                    && self.core.setups.contains(&prior.frozen_setup),
                "تصویر سند فروش نامعتبر است"
            );
            return Ok(result(prior));
        }
        if let Some(prior) = self
            .sales
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
                "رویداد فروش قبلاً با محتوای متفاوت یا نسخهٔ دیگر ثبت شده است"
            );
            if let Some(journal) = &prior.journal_id {
                ensure!(
                    self.post_sales(financial_command(&command, record_journal_lines(&prior)?))?
                        == *journal,
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
            self.sales.requests.push(SalesRequest {
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
        ensure!(
            !self
                .requests
                .iter()
                .any(|r| r.command.company_id == c.company_id
                    && r.command.source_id == c.source_id
                    && r.command.event == command.event()),
            "هویت رویداد قبلاً برای سند مالی دیگری استفاده شده است"
        );
        let preview = self.preview_sales(&command)?;
        let setup = self
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
            "شناسهٔ رویداد فروش با منابع تداخل دارد"
        );
        let sequence = self.operational_sequence(&c.company_id)?;
        let command_sha256 = hash(&command)?;
        let journal_id = if preview.nonfinancial {
            self.activity(
                &c.company_id,
                &record_id,
                AuditOperation::SalesRecorded,
                &c.actor,
                &c.reason,
            );
            None
        } else {
            Some(self.post_sales(financial_command(&command, preview.journal_lines))?)
        };
        let record = SalesRecord {
            id: record_id.clone(),
            command: command.clone(),
            command_sha256,
            frozen_setup: setup,
            sequence,
            journal_id,
            data: preview.data,
            movements: preview.movements,
        };
        let out = result(&record);
        self.sales.records.push(record);
        self.sales.requests.push(SalesRequest {
            request_key: c.request_key.clone(),
            company_id: c.company_id.clone(),
            record_id,
            business_sha256: business,
        });
        ensure!(
            self.sales_reconcile(&c.company_id, "9999-12-31")?
                .reconciled,
            "ثبت فروش باعث اختلاف حساب کنترلی می‌شود"
        );
        Ok(out)
    }
    pub(crate) fn replay_sales_record(&mut self, record: &SalesRecord) -> Result<Option<String>> {
        text(&record.id)?;
        ensure!(
            !self.operational_known_ids().contains(&record.id)
                && !data_ids(&record.data).contains(&record.id.as_str())
                && !record.movements.iter().any(|m| m.id == record.id),
            "شناسهٔ سوابق تغییرات فروش با منبع تداخل دارد"
        );
        let before = self.sales.records.len();
        let out = self.post_sales_command(record.command.clone())?;
        ensure!(
            self.sales.records.len() == before + 1,
            "سوابق تغییرات رویداد فروش تکراری است"
        );
        let created = self.sales.records.last_mut().unwrap();
        created.id = record.id.clone();
        self.sales.requests.last_mut().unwrap().record_id = record.id.clone();
        let mut expected = record.clone();
        expected.journal_id = out.journal_id.clone();
        ensure!(
            *created == expected && record.journal_id.is_some() == out.journal_id.is_some(),
            "محتوا یا محاسبات سوابق تغییرات فروش نامعتبر است"
        );
        if out.journal_id.is_none() {
            self.audit.last_mut().unwrap().entity_id = record.id.clone();
        }
        Ok(out.journal_id)
    }
    pub(crate) fn replay_sales_request(&mut self, request: &SalesRequest) -> Result<SalesResult> {
        let prior = self.sales_document(&request.record_id)?;
        ensure!(
            request.company_id == prior.command.context().company_id
                && request.business_sha256 == business_hash(&prior.command)?,
            "هویت یا هش درخواست فروش نامعتبر است"
        );
        let mut c = prior.command.clone();
        c.context_mut().request_key = request.request_key.clone();
        self.post_sales_command(c)
    }
}
