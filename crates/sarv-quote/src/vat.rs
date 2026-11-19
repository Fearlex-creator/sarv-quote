//! Integer-rial VAT calculations for sales and purchase documents (ADR-0010).
//!
//! All VAT rates are user-entered (company settings or per-line). This module
//! never reads `crate::iran::law` (ADR-0004).

use crate::core::positive_half_up;
use crate::domain::MAX_MONEY;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaxCategory {
    #[default]
    Taxable,
    ZeroRated,
    Exempt,
}

impl TaxCategory {
    pub fn is_default(&self) -> bool {
        matches!(self, Self::Taxable)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PricingMode {
    #[default]
    Exclusive,
    Inclusive,
}

impl PricingMode {
    pub fn is_default(&self) -> bool {
        matches!(self, Self::Exclusive)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VatLineInput {
    pub unit_price_rial: i64,
    pub quantity_thousandths: i64,
    pub line_discount_rial: i64,
    pub document_discount_rial: i64,
    pub category: TaxCategory,
    pub rate_bp: u32,
    pub pricing: PricingMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VatLineOutput {
    pub gross_rial: i64,
    pub net_rial: i64,
    pub tax_rial: i64,
    pub total_rial: i64,
    pub effective_rate_bp: u32,
}

/// Compute exact integer-rial line gross, net, VAT, and total using `positive_half_up`.
pub fn compute_vat_line(input: VatLineInput) -> Result<VatLineOutput> {
    ensure!((0..=MAX_MONEY).contains(&input.unit_price_rial), "قیمت واحد خارج از بازه است");
    ensure!((0..=MAX_MONEY).contains(&input.line_discount_rial), "تخفیف سطر خارج از بازه است");
    ensure!(
        (0..=MAX_MONEY).contains(&input.document_discount_rial),
        "تخفیف سند خارج از بازه است"
    );
    ensure!(input.quantity_thousandths > 0, "مقدار باید مثبت باشد");
    ensure!(input.rate_bp <= 10_000, "نرخ مالیات بر ارزش افزوده باید بین صفر و صد درصد باشد");

    let effective_rate_bp = match input.category {
        TaxCategory::Taxable => input.rate_bp,
        TaxCategory::ZeroRated | TaxCategory::Exempt => 0,
    };

    let gross_num = i128::from(input.unit_price_rial)
        .checked_mul(i128::from(input.quantity_thousandths))
        .ok_or_else(|| anyhow::anyhow!("سرریز محاسبهٔ ناخالص سطر"))?;
    let gross_rial = positive_half_up(gross_num, 1_000)?;
    let total_discount = input
        .line_discount_rial
        .checked_add(input.document_discount_rial)
        .ok_or_else(|| anyhow::anyhow!("سرریز تخفیف سطر"))?;
    ensure!(total_discount <= gross_rial, "تخفیف از مبلغ سطر بیشتر است");
    let subtotal = gross_rial - total_discount;

    match input.pricing {
        PricingMode::Exclusive => {
            let net_rial = subtotal;
            let tax_rial = if effective_rate_bp == 0 {
                0
            } else {
                let num = i128::from(net_rial)
                    .checked_mul(i128::from(effective_rate_bp))
                    .ok_or_else(|| anyhow::anyhow!("سرریز محاسبهٔ مالیات"))?;
                positive_half_up(num, 10_000)?
            };
            let total_rial = net_rial.checked_add(tax_rial).ok_or_else(|| anyhow::anyhow!("سرریز جمع سطر"))?;
            ensure!(total_rial <= MAX_MONEY, "جمع سطر خارج از بازه است");
            Ok(VatLineOutput {
                gross_rial,
                net_rial,
                tax_rial,
                total_rial,
                effective_rate_bp,
            })
        }
        PricingMode::Inclusive => {
            let total_rial = subtotal;
            let tax_rial = if effective_rate_bp == 0 {
                0
            } else {
                let num = i128::from(total_rial)
                    .checked_mul(i128::from(effective_rate_bp))
                    .ok_or_else(|| anyhow::anyhow!("سرریز محاسبهٔ مالیات"))?;
                let den = 10_000i128 + i128::from(effective_rate_bp);
                positive_half_up(num, den)?
            };
            let net_rial = total_rial - tax_rial;
            Ok(VatLineOutput {
                gross_rial,
                net_rial,
                tax_rial,
                total_rial,
                effective_rate_bp,
            })
        }
    }
}

/// Compute tax on an already-computed net (Exclusive) or total (Inclusive) integer-rial amount.
pub fn compute_vat_on_subtotal(subtotal_rial: i64, category: TaxCategory, rate_bp: u32, pricing: PricingMode) -> Result<(i64, i64, i64)> {
    ensure!((0..=MAX_MONEY).contains(&subtotal_rial), "مبلغ پایه مالیات خارج از بازه است");
    ensure!(rate_bp <= 10_000, "نرخ مالیات بر ارزش افزوده باید بین صفر و صد درصد باشد");
    let effective = match category {
        TaxCategory::Taxable => rate_bp,
        TaxCategory::ZeroRated | TaxCategory::Exempt => 0,
    };
    if effective == 0 {
        return Ok((subtotal_rial, 0, subtotal_rial));
    }
    match pricing {
        PricingMode::Exclusive => {
            let num = i128::from(subtotal_rial)
                .checked_mul(i128::from(effective))
                .ok_or_else(|| anyhow::anyhow!("سرریز محاسبهٔ مالیات"))?;
            let tax = positive_half_up(num, 10_000)?;
            let total = subtotal_rial
                .checked_add(tax)
                .ok_or_else(|| anyhow::anyhow!("سرریز جمع با مالیات"))?;
            ensure!(total <= MAX_MONEY, "جمع با مالیات خارج از بازه است");
            Ok((subtotal_rial, tax, total))
        }
        PricingMode::Inclusive => {
            let num = i128::from(subtotal_rial)
                .checked_mul(i128::from(effective))
                .ok_or_else(|| anyhow::anyhow!("سرریز محاسبهٔ مالیات"))?;
            let den = 10_000i128 + i128::from(effective);
            let tax = positive_half_up(num, den)?;
            let net = subtotal_rial - tax;
            Ok((net, tax, subtotal_rial))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exclusive_worked_example_matches_adr_0010() {
        let out = compute_vat_line(VatLineInput {
            unit_price_rial: 10_003,
            quantity_thousandths: 1_250,
            line_discount_rial: 500,
            document_discount_rial: 100,
            category: TaxCategory::Taxable,
            rate_bp: 1_000,
            pricing: PricingMode::Exclusive,
        })
        .unwrap();
        assert_eq!(out.gross_rial, 12_504);
        assert_eq!(out.net_rial, 11_904);
        assert_eq!(out.tax_rial, 1_190);
        assert_eq!(out.total_rial, 13_094);
    }

    #[test]
    fn inclusive_worked_example_has_zero_drift() {
        let out = compute_vat_line(VatLineInput {
            unit_price_rial: 100_000,
            quantity_thousandths: 1_000,
            line_discount_rial: 0,
            document_discount_rial: 0,
            category: TaxCategory::Taxable,
            rate_bp: 1_000,
            pricing: PricingMode::Inclusive,
        })
        .unwrap();
        assert_eq!(out.total_rial, 100_000);
        assert_eq!(out.tax_rial, 9_091);
        assert_eq!(out.net_rial, 90_909);
        assert_eq!(out.net_rial + out.tax_rial, out.total_rial);
    }

    #[test]
    fn zero_rated_and_exempt_produce_zero_tax() {
        for cat in [TaxCategory::ZeroRated, TaxCategory::Exempt] {
            let out = compute_vat_line(VatLineInput {
                unit_price_rial: 50_000,
                quantity_thousandths: 2_000,
                line_discount_rial: 0,
                document_discount_rial: 0,
                category: cat,
                rate_bp: 1_000,
                pricing: PricingMode::Exclusive,
            })
            .unwrap();
            assert_eq!(out.tax_rial, 0);
            assert_eq!(out.effective_rate_bp, 0);
            assert_eq!(out.net_rial, 100_000);
            assert_eq!(out.total_rial, 100_000);
        }
    }

    #[test]
    fn schema_v7_demo_books_upgrade_with_vat_roles_and_post_vat_sales() {
        use crate::accounting::Money;
        use crate::core::CoreRole;
        use crate::procurement::ProcurementContext;
        use crate::sales::{FulfillLine, FulfillOrder, ReturnLine, ReturnSale, SalesCommand, SalesOrder, SalesOrderLine};
        use crate::store::Store;

        let dir = tempfile::tempdir().unwrap();
        crate::desktop::seed_fixture(dir.path()).unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        let company_id = store.state.accounting.companies[0].id.clone();
        let period_id = store.state.accounting.periods[0].id.clone();
        let setup = store
            .state
            .accounting
            .core
            .setups
            .iter()
            .find(|s| s.command.company_id == company_id)
            .unwrap()
            .clone();
        // Verify schema 7 -> 8 migration added both VAT roles
        let vat_payable_acct = setup.account_id(CoreRole::VatPayable).unwrap().to_string();
        let _vat_receivable_acct = setup.account_id(CoreRole::VatReceivable).unwrap().to_string();

        let resources = store.state.accounting.procurement_resources(&company_id).unwrap();
        let customer = resources.parties.iter().find(|p| p.customer).unwrap().clone();
        let item = resources.items[0].clone();
        let wh = resources.warehouses[0].clone();

        // Post a 10% Exclusive VAT sales order for 2 units @ 100_000 = 200_000 net + 20_000 VAT = 220_000 total
        let order_cmd = SalesCommand::Order(SalesOrder {
            context: ProcurementContext {
                company_id: company_id.clone(),
                period_id: period_id.clone(),
                setup_id: setup.id.clone(),
                source_id: "vat-sale-1".into(),
                source_revision: 1,
                date: "2026-10-08".into(),
                request_key: "vat-order-1".into(),
                actor: "tester".into(),
                reason: "VAT sale order".into(),
            },
            customer_id: customer.id.clone(),
            reference: "VAT-SO-1".into(),
            document_date: "2026-10-08".into(),
            lines: vec![SalesOrderLine {
                id: "vat-ol-1".into(),
                item_id: item.id.clone(),
                warehouse_id: wh.id.clone(),
                unit_id: item.base_unit_id.clone(),
                quantity_thousandths: 2_000,
                unit_price: Money::from_rials(100_000),
                line_discount: Money::ZERO,
                price_reference: "manual".into(),
                price_date: "2026-10-08".into(),
                override_reason: None,
            }],
            document_discount: Money::ZERO,
            supplied_total: Money::from_rials(220_000),
            quotation: None,
            vat_basis_points: 1_000,
            pricing_mode: PricingMode::Exclusive,
        });
        let order_res = store.state.accounting.post_sales_command(order_cmd).unwrap();

        // Fulfill all 2 units -> Receivables +220_000, Sales +200_000, VatPayable +20_000
        let fulfill_cmd = SalesCommand::Fulfill(FulfillOrder {
            context: ProcurementContext {
                company_id: company_id.clone(),
                period_id: period_id.clone(),
                setup_id: setup.id.clone(),
                source_id: "vat-fulfill-1".into(),
                source_revision: 1,
                date: "2026-10-08".into(),
                request_key: "vat-fulfill-1".into(),
                actor: "tester".into(),
                reason: "VAT fulfill".into(),
            },
            order_id: order_res.record_id.clone(),
            delivery_id: "vat-del-1".into(),
            invoice_id: "vat-inv-1".into(),
            delivery_reference: "VAT-DEL-1".into(),
            invoice_reference: "VAT-INV-1".into(),
            document_date: "2026-10-08".into(),
            due_date: "2026-10-15".into(),
            lines: vec![FulfillLine {
                id: "vat-fl-1".into(),
                order_line_id: "vat-ol-1".into(),
                warehouse_id: wh.id.clone(),
                unit_id: item.base_unit_id.clone(),
                quantity_thousandths: 2_000,
            }],
            supplied_total: Money::from_rials(220_000),
        });
        let fulfill_res = store.state.accounting.post_sales_command(fulfill_cmd).unwrap();
        let tb = store.state.accounting.trial_balance(&company_id, "2026-10-08").unwrap();
        assert_eq!(tb.total_debit, tb.total_credit);
        let vp_row = tb.rows.iter().find(|r| r.account_id == vat_payable_acct).unwrap();
        assert_eq!(vp_row.credit.rials() - vp_row.debit.rials(), 20_000);
        let rec = store.state.accounting.sales_reconcile(&company_id, "2026-10-08").unwrap();
        assert!(rec.reconciled, "sales must reconcile with VAT posted");

        // Partial return of 1 unit (50%) -> SalesReturns +100_000, VatPayable -10_000, Receivables -110_000
        let ret_lines = store.state.accounting.sales_invoice_returnable(&fulfill_res.record_id).unwrap();
        let half_cost = Money::from_rials(((i128::from(ret_lines[0].cost.rials()) + 1) / 2) as i64);
        let ret_cmd = SalesCommand::Return(ReturnSale {
            context: ProcurementContext {
                company_id: company_id.clone(),
                period_id,
                setup_id: setup.id,
                source_id: "vat-ret-1".into(),
                source_revision: 1,
                date: "2026-10-09".into(),
                request_key: "vat-ret-1".into(),
                actor: "tester".into(),
                reason: "VAT partial return".into(),
            },
            fulfillment_id: fulfill_res.record_id,
            return_id: "vat-sr-1".into(),
            credit_id: "vat-cr-1".into(),
            return_reference: "VAT-RET-1".into(),
            credit_reference: "VAT-CR-1".into(),
            document_date: "2026-10-09".into(),
            lines: vec![ReturnLine {
                id: "vat-rl-1".into(),
                fulfillment_line_id: "vat-fl-1".into(),
                quantity_thousandths: 1_000,
                supplied_credit: Money::from_rials(110_000),
                supplied_cost: half_cost,
            }],
            supplied_credit: Money::from_rials(110_000),
            supplied_cost: half_cost,
        });
        store.state.accounting.post_sales_command(ret_cmd).unwrap();
        let tb2 = store.state.accounting.trial_balance(&company_id, "2026-10-09").unwrap();
        assert_eq!(tb2.total_debit, tb2.total_credit);
        let vp_row2 = tb2.rows.iter().find(|r| r.account_id == vat_payable_acct).unwrap();
        assert_eq!(vp_row2.credit.rials() - vp_row2.debit.rials(), 10_000);
        let rec2 = store.state.accounting.sales_reconcile(&company_id, "2026-10-09").unwrap();
        assert!(rec2.reconciled, "sales must reconcile after partial VAT return");
        store.state.validate().unwrap();
    }
}
