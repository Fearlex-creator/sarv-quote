//! Durable raw sales editing is distinct from immutable, postable sales sources.
use crate::{
    accounting::{Money, MoneyUnit},
    calendar::{self, Calendar},
    domain::{DateDisplay, State, digits, id, normalize, quantity},
    imports::hash,
    procurement::ProcurementContext,
    sales::{QuotationLink, SalesCommand, SalesOrder, SalesOrderLine, SalesResult},
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawSalesLine {
    pub id: String,
    pub item_id: String,
    pub warehouse_id: String,
    pub unit_id: String,
    pub quantity: String,
    pub unit_price: String,
    pub line_discount: String,
    pub price_reference: String,
    pub price_date: String,
    pub override_reason: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawSalesDraft {
    pub id: String,
    pub revision: u64,
    pub company_id: String,
    pub period_id: String,
    pub setup_id: String,
    pub customer_id: String,
    pub reference: String,
    pub document_date: String,
    pub financial_date: String,
    pub date_display: DateDisplay,
    pub display_unit: MoneyUnit,
    pub document_discount: String,
    pub supplied_total: String,
    pub reason: String,
    pub quotation: Option<QuotationLink>,
    pub lines: Vec<RawSalesLine>,
}
fn bounded(s: &str) -> Result<()> {
    ensure!(
        s.len() <= 16_384 && !s.contains('\0'),
        "متن پیش‌نویس بیش از حد بزرگ یا نامعتبر است"
    );
    Ok(())
}
impl RawSalesDraft {
    pub fn validate_raw(&self) -> Result<()> {
        ensure!(
            !self.id.trim().is_empty() && self.id.len() <= 128,
            "شناسهٔ پیش‌نویس نامعتبر است"
        );
        ensure!(
            self.lines.len() <= 10_000,
            "حداکثر ده هزار ردیف پیش‌نویس مجاز است"
        );
        for s in [
            &self.id,
            &self.company_id,
            &self.period_id,
            &self.setup_id,
            &self.customer_id,
            &self.reference,
            &self.document_date,
            &self.financial_date,
            &self.document_discount,
            &self.supplied_total,
            &self.reason,
        ] {
            bounded(s)?;
        }
        let mut ids = HashSet::new();
        for l in &self.lines {
            ensure!(
                !l.id.trim().is_empty() && l.id.len() <= 128 && ids.insert(&l.id),
                "شناسهٔ ردیف پیش‌نویس نامعتبر یا تکراری است"
            );
            for s in [
                &l.id,
                &l.item_id,
                &l.warehouse_id,
                &l.unit_id,
                &l.quantity,
                &l.unit_price,
                &l.line_discount,
                &l.price_reference,
                &l.price_date,
                &l.override_reason,
            ] {
                bounded(s)?;
            }
        }
        if let Some(q) = &self.quotation {
            for s in [
                &q.quote_id,
                &q.quote_series,
                &q.quote_number,
                &q.quote_sha256,
            ] {
                bounded(s)?;
            }
            ensure!(
                q.source_line_indices.len() <= 10_000,
                "پیوند ردیف‌های پیش‌فاکتور بیش از حد بزرگ است"
            );
        }
        ensure!(
            serde_json::to_vec(self)?.len() <= 8 * 1024 * 1024,
            "پیش‌نویس بیش از هشت مگابایت است"
        );
        Ok(())
    }
}
fn canonical_date(value: &str, display: DateDisplay) -> Result<String> {
    let value = digits(value.trim());
    let date = match display {
        DateDisplay::Jalali => calendar::parse_date(&value, Calendar::Jalali)?,
        DateDisplay::Iso => calendar::parse_date(&value, Calendar::Gregorian)?,
        DateDisplay::DayMonthYear => {
            let date = chrono::NaiveDate::parse_from_str(&value, "%d/%m/%Y")?;
            ensure!(
                date.format("%d/%m/%Y").to_string() == value,
                "تاریخ روز/ماه/سال نامعتبر است"
            );
            date
        }
    };
    Ok(date.format("%Y-%m-%d").to_string())
}
fn quantity_text(q: i64) -> String {
    format!("{}.{:03}", q / 1000, q % 1000)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
}
impl State {
    pub(crate) fn validate_sales_drafts_and_links(&self) -> Result<()> {
        ensure!(
            self.sales_drafts.len() <= 1000,
            "حداکثر هزار پیش‌نویس فروش مجاز است"
        );
        let mut ids = HashSet::new();
        for d in &self.sales_drafts {
            d.validate_raw()?;
            ensure!(
                d.revision > 0 && ids.insert(&d.id),
                "نسخه یا شناسهٔ پیش‌نویس نامعتبر است"
            );
            ensure!(
                !self
                    .accounting
                    .sales
                    .records
                    .iter()
                    .any(|r| r.command.context().source_id == d.id),
                "منبع ثبت‌شده نباید پیش‌نویس فعال باشد"
            );
        }
        for r in &self.accounting.sales.records {
            if let SalesCommand::Order(order) = &r.command {
                self.validate_sales_quotation(order)?;
            }
        }
        Ok(())
    }
    pub fn save_sales_draft(
        &mut self,
        mut draft: RawSalesDraft,
        expected_revision: Option<u64>,
    ) -> Result<u64> {
        draft.validate_raw()?;
        ensure!(
            self.schema == crate::domain::CURRENT_SCHEMA,
            "ابتدا داده را ارتقا دهید"
        );
        ensure!(
            !self
                .accounting
                .sales
                .records
                .iter()
                .any(|r| r.command.context().source_id == draft.id),
            "منبع ثبت‌شده دوباره پیش‌نویس نمی‌شود"
        );
        if let Some(index) = self.sales_drafts.iter().position(|d| d.id == draft.id) {
            let current = &self.sales_drafts[index];
            ensure!(
                expected_revision == Some(current.revision) && draft.revision == current.revision,
                "پیش‌نویس در پنجرهٔ دیگری تغییر کرده است"
            );
            if current == &draft {
                return Ok(current.revision);
            }
            draft.revision = current
                .revision
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("سرریز نسخهٔ پیش‌نویس"))?;
            let revision = draft.revision;
            self.sales_drafts[index] = draft;
            Ok(revision)
        } else {
            ensure!(
                expected_revision.is_none()
                    && draft.revision == 0
                    && self.sales_drafts.len() < 1000,
                "پیش‌نویس جدید یا نسخهٔ آن نامعتبر است"
            );
            draft.revision = 1;
            self.sales_drafts.push(draft);
            Ok(1)
        }
    }
    pub fn remove_sales_draft(&mut self, id: &str, expected_revision: u64) -> Result<()> {
        let index = self
            .sales_drafts
            .iter()
            .position(|d| d.id == id && d.revision == expected_revision)
            .ok_or_else(|| anyhow::anyhow!("پیش‌نویس حذف یا تغییر کرده است"))?;
        self.sales_drafts.remove(index);
        Ok(())
    }
    pub fn sales_order_from_draft(
        &self,
        id: &str,
        expected_revision: u64,
        request_key: &str,
        actor: &str,
    ) -> Result<SalesOrder> {
        let d = self
            .sales_drafts
            .iter()
            .find(|d| d.id == id && d.revision == expected_revision)
            .ok_or_else(|| anyhow::anyhow!("ابتدا نسخهٔ جاری پیش‌نویس را ذخیره کنید"))?;
        let lines = d
            .lines
            .iter()
            .map(|l| {
                Ok(SalesOrderLine {
                    id: l.id.clone(),
                    item_id: l.item_id.clone(),
                    warehouse_id: l.warehouse_id.clone(),
                    unit_id: l.unit_id.clone(),
                    quantity_thousandths: quantity(&l.quantity)?,
                    unit_price: Money::parse(&l.unit_price, d.display_unit)?,
                    line_discount: Money::parse(&l.line_discount, d.display_unit)?,
                    price_reference: l.price_reference.clone(),
                    price_date: canonical_date(&l.price_date, d.date_display)?,
                    override_reason: (!l.override_reason.trim().is_empty())
                        .then(|| l.override_reason.clone()),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let order = SalesOrder {
            context: ProcurementContext {
                company_id: d.company_id.clone(),
                period_id: d.period_id.clone(),
                setup_id: d.setup_id.clone(),
                source_id: d.id.clone(),
                source_revision: d.revision,
                date: canonical_date(&d.financial_date, d.date_display)?,
                request_key: request_key.to_owned(),
                actor: actor.to_owned(),
                reason: d.reason.clone(),
            },
            customer_id: d.customer_id.clone(),
            reference: d.reference.clone(),
            document_date: canonical_date(&d.document_date, d.date_display)?,
            lines,
            document_discount: Money::parse(&d.document_discount, d.display_unit)?,
            supplied_total: Money::parse(&d.supplied_total, d.display_unit)?,
            quotation: d.quotation.clone(),
        };
        self.validate_sales_quotation(&order)?;
        self.accounting
            .preview_sales(&SalesCommand::Order(order.clone()))?;
        Ok(order)
    }
    pub fn post_sales_from_draft(
        &mut self,
        id: &str,
        expected_revision: u64,
        request_key: &str,
        actor: &str,
    ) -> Result<SalesResult> {
        if !self.sales_drafts.iter().any(|d| d.id == id) {
            let original = self
                .accounting
                .sales
                .records
                .iter()
                .find_map(|r| match &r.command {
                    SalesCommand::Order(order)
                        if order.context.source_id == id
                            && order.context.source_revision == expected_revision =>
                    {
                        Some(order)
                    }
                    _ => None,
                })
                .ok_or_else(|| anyhow::anyhow!("پیش‌نویس یا منبع ثبت‌شده با این نسخه پیدا نشد"))?;
            ensure!(
                original.context.actor == actor,
                "عامل تلاش مجدد با ثبت اصلی متفاوت است"
            );
            let mut command = original.clone();
            command.context.request_key = request_key.into();
            self.validate_sales_quotation(&command)?;
            return self
                .accounting
                .post_sales_command(SalesCommand::Order(command));
        }
        let order = self.sales_order_from_draft(id, expected_revision, request_key, actor)?;
        let mut next = self.clone();
        let result = next
            .accounting
            .post_sales_command(SalesCommand::Order(order))?;
        next.remove_sales_draft(id, expected_revision)?;
        next.validate()?;
        *self = next;
        Ok(result)
    }
    /// Validate the entire retained approved quotation and explicit source-line map.
    /// No fuzzy catalog/customer match can authorize a financial document.
    pub fn validate_sales_quotation(&self, order: &SalesOrder) -> Result<()> {
        let Some(link) = &order.quotation else {
            return Ok(());
        };
        let q = self
            .quotes
            .iter()
            .find(|q| q.id == link.quote_id)
            .ok_or_else(|| anyhow::anyhow!("پیش‌فاکتور تأییدشدهٔ پیوندشده پیدا نشد"))?;
        ensure!(
            hash(&serde_json::to_vec(q)?) == link.quote_sha256
                && q.series == link.quote_series
                && q.version == link.quote_revision
                && q.number == link.quote_number,
            "تصویر کامل پیش‌فاکتور پیوندشده تغییر کرده است"
        );
        ensure!(
            q.tax_rial == 0 && q.tax_basis_points == 0,
            "تبدیل پیش‌فاکتور دارای مالیات هنوز فعال نیست؛ مالیات حذف نمی‌شود"
        );
        ensure!(
            order.lines.len() == q.lines.len()
                && link.source_line_indices.len() == q.lines.len()
                && order.document_discount.rials() == q.discount_rial
                && order.supplied_total.rials() == q.total_rial,
            "تبدیل باید تمام ردیف‌ها و مبلغ دقیق پیش‌فاکتور را حفظ کند"
        );
        let resources = self
            .accounting
            .procurement_resources(&order.context.company_id)?;
        let party = resources
            .parties
            .iter()
            .find(|p| p.id == order.customer_id && p.customer)
            .ok_or_else(|| anyhow::anyhow!("مشتری حسابداری معتبر نیست"))?;
        let mapping_reason = format!("quote-customer-map:{}:{}", q.id, party.id);
        ensure!(
            normalize(&party.name) == normalize(&q.customer)
                || order.context.reason.contains(&mapping_reason),
            "نگاشت مشتری متفاوت به دلیل صریح quote-customer-map نیاز دارد"
        );
        let mut indices = HashSet::new();
        for (l, source_index) in order.lines.iter().zip(&link.source_line_indices) {
            ensure!(
                indices.insert(*source_index),
                "ردیف پیش‌فاکتور دوباره استفاده شده است"
            );
            let source = q
                .lines
                .get(*source_index)
                .ok_or_else(|| anyhow::anyhow!("شمارهٔ ردیف پیش‌فاکتور نامعتبر است"))?;
            let item = resources
                .items
                .iter()
                .find(|i| i.id == l.item_id)
                .ok_or_else(|| anyhow::anyhow!("کالای نگاشت‌شده معتبر نیست"))?;
            let unit = resources
                .units
                .iter()
                .find(|u| u.id == l.unit_id)
                .ok_or_else(|| anyhow::anyhow!("واحد نگاشت‌شده معتبر نیست"))?;
            let exact_items = resources
                .items
                .iter()
                .filter(|i| normalize(&i.sku) == normalize(&source.code))
                .count();
            ensure!(
                exact_items == 1
                    && normalize(&item.sku) == normalize(&source.code)
                    && normalize(&unit.name) == normalize(&source.measure),
                "کد و واحد کالا باید نگاشت دقیق و یکتا داشته باشد"
            );
            let matching_units: Vec<_> = resources
                .units
                .iter()
                .filter(|u| {
                    normalize(&u.name) == normalize(&source.measure)
                        && (item.base_unit_id == u.id
                            || item.packs.iter().any(|p| p.unit_id == u.id))
                })
                .collect();
            ensure!(
                matching_units.len() == 1 && matching_units[0].id == unit.id,
                "نگاشت واحد کالا مبهم است"
            );
            ensure!(
                l.quantity_thousandths == source.quantity_milli
                    && l.unit_price.rials() == source.price_rial
                    && l.line_discount == Money::ZERO,
                "تعداد یا قیمت منبع پیش‌فاکتور تغییر کرده است"
            );
            ensure!(
                l.price_reference
                    == format!(
                        "quotation:{}:line:{}:catalog:{}",
                        q.id, source_index, q.catalog_id
                    )
                    && l.price_date == source.source_at.get(..10).unwrap_or_default(),
                "منبع و تاریخ قیمت پیش‌فاکتور معتبر نیست"
            );
        }
        Ok(())
    }
    pub fn sales_draft_from_quote(
        &self,
        quote_id: &str,
        company_id: &str,
    ) -> Result<RawSalesDraft> {
        let q = self
            .quotes
            .iter()
            .find(|q| q.id == quote_id)
            .ok_or_else(|| anyhow::anyhow!("پیش‌فاکتور پیدا نشد"))?;
        ensure!(
            q.tax_rial == 0 && q.tax_basis_points == 0,
            "پیش‌فاکتور دارای مالیات قابل تبدیل نیست"
        );
        let resources = self.accounting.procurement_resources(company_id)?;
        let parties: Vec<_> = resources
            .parties
            .iter()
            .filter(|p| p.customer && normalize(&p.name) == normalize(&q.customer))
            .collect();
        ensure!(
            parties.len() == 1,
            "ابتدا مشتری دقیق و یکتای پیش‌فاکتور را در منابع حسابداری تعریف کنید"
        );
        let opening = self
            .accounting
            .core
            .openings
            .iter()
            .find(|o| o.command.company_id == company_id)
            .ok_or_else(|| anyhow::anyhow!("افتتاحیهٔ شرکت پیدا نشد"))?;
        let display_unit = match q.display_unit {
            crate::domain::Unit::Rial => MoneyUnit::Rial,
            crate::domain::Unit::Toman => MoneyUnit::Toman,
        };
        let warehouse_id = if resources.warehouses.len() == 1 {
            resources.warehouses[0].id.clone()
        } else {
            String::new()
        };
        let mut lines = Vec::new();
        for (index, source) in q.lines.iter().enumerate() {
            let items: Vec<_> = resources
                .items
                .iter()
                .filter(|i| normalize(&i.sku) == normalize(&source.code))
                .collect();
            ensure!(
                items.len() == 1,
                "ابتدا کد دقیق و یکتای کالای پیش‌فاکتور را تعریف کنید"
            );
            let item = items[0];
            let units: Vec<_> = resources
                .units
                .iter()
                .filter(|u| {
                    normalize(&u.name) == normalize(&source.measure)
                        && (item.base_unit_id == u.id
                            || item.packs.iter().any(|p| p.unit_id == u.id))
                })
                .collect();
            ensure!(
                units.len() == 1,
                "ابتدا واحد دقیق و یکتای کالای پیش‌فاکتور را تعریف کنید"
            );
            lines.push(RawSalesLine {
                id: id(),
                item_id: item.id.clone(),
                warehouse_id: warehouse_id.clone(),
                unit_id: units[0].id.clone(),
                quantity: quantity_text(source.quantity_milli),
                unit_price: Money::from_rials(source.price_rial).display(display_unit),
                line_discount: "0".into(),
                price_reference: format!(
                    "quotation:{}:line:{}:catalog:{}",
                    q.id, index, q.catalog_id
                ),
                price_date: q
                    .date_display
                    .format(source.source_at.get(..10).unwrap_or_default()),
                override_reason: String::new(),
            });
        }
        let document_date = if q.quote_date.is_empty() {
            q.approved_at.get(..10).unwrap_or_default()
        } else {
            &q.quote_date
        };
        let financial_date = crate::domain::now();
        let financial_date = &financial_date[..10];
        let period = self
            .accounting
            .periods
            .iter()
            .find(|p| {
                p.company_id == company_id
                    && !p.locked
                    && p.start_date.as_str() <= financial_date
                    && p.end_date.as_str() >= financial_date
            })
            .ok_or_else(|| anyhow::anyhow!("سال مالی باز برای تاریخ امروز پیدا نشد"))?;
        Ok(RawSalesDraft {
            id: id(),
            revision: 0,
            company_id: company_id.into(),
            period_id: period.id.clone(),
            setup_id: opening.frozen_setup.id.clone(),
            customer_id: parties[0].id.clone(),
            reference: q.number.clone(),
            document_date: q.date_display.format(document_date),
            financial_date: q.date_display.format(financial_date),
            date_display: q.date_display,
            display_unit,
            document_discount: Money::from_rials(q.discount_rial).display(display_unit),
            supplied_total: Money::from_rials(q.total_rial).display(display_unit),
            reason: "تبدیل پیش‌فاکتور تأییدشده با قیمت و ردیف‌های اصلی".into(),
            quotation: Some(QuotationLink {
                quote_id: q.id.clone(),
                quote_series: q.series.clone(),
                quote_revision: q.version,
                quote_number: q.number.clone(),
                quote_sha256: hash(&serde_json::to_vec(q)?),
                source_line_indices: (0..q.lines.len()).collect(),
            }),
            lines,
        })
    }
}
