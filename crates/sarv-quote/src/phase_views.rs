//! Coarse Phase 2–4 read/write models for the Tauri shell.

use serde::{Deserialize, Serialize};

use crate::accounting::{AccountClass, BalanceSide, JournalKind};
use crate::core::{ChequeDirection, OpeningChequeState};
use crate::domain::{id, now, AppMode, Settings, State, Unit};
use crate::procurement::{ProcurementCommand, ProcurementData};
use crate::sales::{SalesCommand, SalesData};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocRow {
    pub id: String,
    pub kind: String,
    pub reference: String,
    pub party: String,
    pub date: String,
    pub status: String,
    pub total_rial: i64,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductRow {
    pub id: String,
    pub code: String,
    pub name: String,
    pub measure: String,
    pub price_rial: i64,
    pub catalog: String,
    pub source_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StockRow {
    pub item_id: String,
    pub item_name: String,
    pub warehouse_id: String,
    pub warehouse_name: String,
    pub quantity_thousandths: i64,
    pub value_rial: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChequeRow {
    pub id: String,
    pub direction: String,
    pub party: String,
    pub bank: String,
    pub reference: String,
    pub due_date: String,
    pub amount_rial: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportRow {
    pub code: String,
    pub name: String,
    pub debit_rial: i64,
    pub credit_rial: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportsView {
    pub trial_balance: Vec<ReportRow>,
    pub total_debit_rial: i64,
    pub total_credit_rial: i64,
    pub income_rial: i64,
    pub expense_rial: i64,
    pub cash_rial: i64,
    pub as_of: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditRow {
    pub id: String,
    pub at: String,
    pub actor: String,
    pub operation: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WarehouseRow {
    pub id: String,
    pub name: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleHubView {
    pub invoices: Vec<DocRow>,
    pub bills: Vec<DocRow>,
    pub purchase_orders: Vec<DocRow>,
    pub products: Vec<ProductRow>,
    pub stock: Vec<StockRow>,
    pub cheques: Vec<ChequeRow>,
    pub warehouses: Vec<WarehouseRow>,
    pub audit: Vec<AuditRow>,
    pub reports: ReportsView,
    pub extras: AppExtras,
    pub company_ready: bool,
    pub onboarding_done: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppExtras {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recurring: Vec<RecurringItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub budgets: Vec<BudgetLine>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rates: Vec<FxRate>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub users: Vec<LocalUser>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub onboarding_done: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<AttachmentMeta>,
}

fn is_false(v: &bool) -> bool {
    !*v
}

impl AppExtras {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurringItem {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub amount_rial: i64,
    pub cadence: String,
    pub next_date: String,
    #[serde(default)]
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetLine {
    pub id: String,
    pub category: String,
    pub month: String,
    pub limit_rial: i64,
    pub spent_rial: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FxRate {
    pub id: String,
    pub currency: String,
    pub rate_to_rial: i64,
    pub as_of: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalUser {
    pub id: String,
    pub name: String,
    pub role: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub pin_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentMeta {
    pub id: String,
    pub name: String,
    pub linked_entry_id: String,
    pub stored_at: String,
}

pub fn module_hub(state: &State) -> ModuleHubView {
    ModuleHubView {
        invoices: sales_docs(state),
        bills: procurement_docs(state, "bill"),
        purchase_orders: procurement_docs(state, "order"),
        products: product_rows(state),
        stock: stock_rows(state),
        cheques: cheque_rows(state),
        warehouses: warehouse_rows(state),
        audit: audit_rows(state),
        reports: reports_view(state),
        extras: state.extras.clone(),
        company_ready: !state.accounting.companies.is_empty()
            || !state.accounting.core.is_empty(),
        onboarding_done: state.extras.onboarding_done
            || !state.settings.business_name.trim().is_empty()
            || !state.cashbook.entries.is_empty(),
    }
}

fn sales_docs(state: &State) -> Vec<DocRow> {
    let mut rows = Vec::new();
    for r in &state.accounting.sales.records {
        let (kind, party, date, total, status, reference, detail) = match &r.data {
            SalesData::Order(o) => (
                "سفارش فروش",
                o.customer.name.clone(),
                r.command.context().date.clone(),
                o.total.rials(),
                "ثبت‌شده",
                r.id.clone(),
                format!("{} قلم", o.lines.len()),
            ),
            SalesData::Fulfill(f) => (
                "فاکتور",
                f.customer.name.clone(),
                r.command.context().date.clone(),
                f.total.rials(),
                "صادر شده",
                f.invoice_id.clone(),
                format!("تحویل {}", f.delivery_id),
            ),
            SalesData::Return(ret) => (
                "برگشت از فروش",
                ret.customer_id.clone(),
                r.command.context().date.clone(),
                ret.total.rials(),
                "برگشت",
                ret.return_id.clone(),
                format!("اعتبار {}", ret.credit_id),
            ),
        };
        let _ = &r.command;
        let date = match &r.command {
            SalesCommand::Order(o) => o.document_date.clone(),
            SalesCommand::Fulfill(f) => f.document_date.clone(),
            SalesCommand::Return(x) => x.document_date.clone(),
        };
        rows.push(DocRow {
            id: r.id.clone(),
            kind: kind.into(),
            reference,
            party,
            date,
            status: status.into(),
            total_rial: total,
            detail,
        });
    }
    for d in &state.sales_drafts {
        rows.push(DocRow {
            id: d.id.clone(),
            kind: "پیش‌نویس فروش".into(),
            reference: d.id.clone(),
            party: d.customer_name.clone(),
            date: d.updated_at.clone(),
            status: "پیش‌نویس".into(),
            total_rial: 0,
            detail: format!("{} خط", d.lines.len()),
        });
    }
    rows.sort_by(|a, b| b.date.cmp(&a.date));
    rows
}

fn procurement_docs(state: &State, filter: &str) -> Vec<DocRow> {
    let mut rows = Vec::new();
    for r in &state.accounting.procurement.records {
        let include = match (&r.command, filter) {
            (ProcurementCommand::Order(_), "order") => true,
            (ProcurementCommand::Bill(_), "bill") => true,
            (ProcurementCommand::Receive(_), "bill") => true,
            (ProcurementCommand::Return(_), "bill") => true,
            _ if filter == "all" => true,
            _ => false,
        };
        if !include {
            continue;
        }
        let (kind, party, date, total, status, reference, detail) = match &r.data {
            ProcurementData::Order(o) => (
                "سفارش خرید",
                o.supplier.name.clone(),
                match &r.command {
                    ProcurementCommand::Order(c) => c.document_date.clone(),
                    _ => r.command.context().date.clone(),
                },
                o.total.rials(),
                "سفارش",
                match &r.command {
                    ProcurementCommand::Order(c) => c.reference.clone(),
                    _ => r.id.clone(),
                },
                format!("{} قلم", o.lines.len()),
            ),
            ProcurementData::Receive(rec) => (
                "رسید کالا",
                String::new(),
                match &r.command {
                    ProcurementCommand::Receive(c) => c.document_date.clone(),
                    _ => r.command.context().date.clone(),
                },
                rec.total.rials(),
                "رسید",
                match &r.command {
                    ProcurementCommand::Receive(c) => c.reference.clone(),
                    _ => r.id.clone(),
                },
                format!("{} حرکت", r.movements.len()),
            ),
            ProcurementData::Bill(b) => (
                "صورتحساب خرید",
                b.supplier.name.clone(),
                match &r.command {
                    ProcurementCommand::Bill(c) => c.document_date.clone(),
                    _ => r.command.context().date.clone(),
                },
                b.total.rials(),
                "بدهی",
                match &r.command {
                    ProcurementCommand::Bill(c) => c.reference.clone(),
                    _ => r.id.clone(),
                },
                format!("سررسید {}", match &r.command {
                    ProcurementCommand::Bill(c) => c.due_date.clone(),
                    _ => String::new(),
                }),
            ),
            ProcurementData::Return(ret) => (
                "برگشت خرید",
                ret.supplier_id.clone(),
                r.command.context().date.clone(),
                ret.total.rials(),
                "برگشت",
                ret.return_id.clone(),
                String::new(),
            ),
            ProcurementData::Resources(_) => (
                "منابع",
                String::new(),
                r.command.context().date.clone(),
                0,
                "ثبت",
                r.id.clone(),
                "ثبت کالا/انبار".into(),
            ),
            ProcurementData::Transfer(_) => (
                "انتقال انبار",
                String::new(),
                r.command.context().date.clone(),
                0,
                "انتقال",
                r.id.clone(),
                format!("{} حرکت", r.movements.len()),
            ),
        };
        let _ = date;
        rows.push(DocRow {
            id: r.id.clone(),
            kind: kind.into(),
            reference,
            party,
            date: r.command.context().date.clone(),
            status: status.into(),
            total_rial: total,
            detail,
        });
    }
    rows.sort_by(|a, b| b.date.cmp(&a.date));
    rows
}

fn product_rows(state: &State) -> Vec<ProductRow> {
    let mut rows = Vec::new();
    for cat in &state.catalogs {
        for p in &cat.products {
            rows.push(ProductRow {
                id: format!("{}:{}", cat.id, p.code),
                code: p.code.clone(),
                name: p.name.clone(),
                measure: p.measure.clone(),
                price_rial: p.price_rial,
                catalog: cat.name.clone(),
                source_at: p.source_at.clone(),
            });
        }
    }
    if let Some(setup) = state.accounting.core.setups.last() {
        for item in &setup.items {
            if rows.iter().any(|r| r.code == item.code || r.id == item.id) {
                continue;
            }
            rows.push(ProductRow {
                id: item.id.clone(),
                code: item.code.clone(),
                name: item.name.clone(),
                measure: item.unit_id.clone(),
                price_rial: 0,
                catalog: "هسته".into(),
                source_at: String::new(),
            });
        }
    }
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    rows
}

fn stock_rows(state: &State) -> Vec<StockRow> {
    use std::collections::BTreeMap;
    let mut pools: BTreeMap<(String, String), (i64, i64)> = BTreeMap::new();
    for r in &state.accounting.procurement.records {
        for m in &r.movements {
            let key = (m.item_id.clone(), m.warehouse_id.clone());
            let e = pools.entry(key).or_insert((0, 0));
            e.0 = m.after.quantity_thousandths;
            e.1 = m.after.value.rials();
        }
    }
    for r in &state.accounting.sales.records {
        for m in &r.movements {
            let key = (m.item_id.clone(), m.warehouse_id.clone());
            let e = pools.entry(key).or_insert((0, 0));
            e.0 = m.after.quantity_thousandths;
            e.1 = m.after.value.rials();
        }
    }
    let item_name = |id: &str| {
        state
            .accounting
            .core
            .setups
            .iter()
            .flat_map(|s| s.items.iter())
            .find(|i| i.id == id)
            .map(|i| i.name.clone())
            .unwrap_or_else(|| id.to_string())
    };
    let wh_name = |id: &str| {
        state
            .accounting
            .core
            .setups
            .iter()
            .flat_map(|s| s.warehouses.iter())
            .find(|w| w.id == id)
            .map(|w| w.name.clone())
            .unwrap_or_else(|| id.to_string())
    };
    pools
        .into_iter()
        .map(|((item_id, warehouse_id), (q, v))| StockRow {
            item_name: item_name(&item_id),
            warehouse_name: wh_name(&warehouse_id),
            item_id,
            warehouse_id,
            quantity_thousandths: q,
            value_rial: v,
        })
        .collect()
}

fn cheque_rows(state: &State) -> Vec<ChequeRow> {
    let mut rows = Vec::new();
    for setup in &state.accounting.core.setups {
        // Opening cheques live on opening registers linked to foundation; scan opening if present.
        let _ = setup;
    }
    if let Some(opening) = state.accounting.core.openings.last() {
        for c in &opening.cheques {
            let party = state
                .accounting
                .core
                .setups
                .iter()
                .flat_map(|s| s.parties.iter())
                .find(|p| p.id == c.party_id)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| c.party_id.clone());
            rows.push(ChequeRow {
                id: c.id.clone(),
                direction: match c.direction {
                    ChequeDirection::Receivable => "دریافتی".into(),
                    ChequeDirection::Payable => "پرداختی".into(),
                },
                party,
                bank: c.bank_reference.clone(),
                reference: c.cheque_reference.clone(),
                due_date: c.due_date.clone(),
                amount_rial: c.amount.rials(),
                status: match c.state {
                    OpeningChequeState::InHand => "نزد ما".into(),
                    OpeningChequeState::Deposited => "واگذار شده".into(),
                    OpeningChequeState::Cleared => "وصول".into(),
                    OpeningChequeState::Bounced => "برگشتی".into(),
                },
            });
        }
    }
    rows
}

fn warehouse_rows(state: &State) -> Vec<WarehouseRow> {
    let mut rows = Vec::new();
    for setup in &state.accounting.core.setups {
        for w in &setup.warehouses {
            rows.push(WarehouseRow {
                id: w.id.clone(),
                name: w.name.clone(),
                code: w.code.clone(),
            });
        }
    }
    rows
}

fn audit_rows(state: &State) -> Vec<AuditRow> {
    state
        .accounting
        .audit
        .iter()
        .rev()
        .take(200)
        .map(|a| AuditRow {
            id: a.id.clone(),
            at: a.at.clone(),
            actor: a.actor.clone(),
            operation: format!("{:?}", a.operation),
            detail: a.detail.clone(),
        })
        .collect()
}

fn reports_view(state: &State) -> ReportsView {
    let mut map = std::collections::BTreeMap::<String, (String, String, i64, i64)>::new();
    for j in &state.accounting.journals {
        for line in &j.lines {
            let e = map
                .entry(line.account.id.clone())
                .or_insert((
                    line.account.code.clone(),
                    line.account.name.clone(),
                    0,
                    0,
                ));
            e.2 += line.debit.rials();
            e.3 += line.credit.rials();
        }
    }
    let trial_balance: Vec<_> = map
        .into_iter()
        .map(|(_, (code, name, debit, credit))| ReportRow {
            code,
            name,
            debit_rial: debit,
            credit_rial: credit,
        })
        .collect();
    let total_debit_rial: i64 = trial_balance.iter().map(|r| r.debit_rial).sum();
    let total_credit_rial: i64 = trial_balance.iter().map(|r| r.credit_rial).sum();
    let mut income = state.cashbook.income_total();
    let mut expense = state.cashbook.expense_total();
    for j in &state.accounting.journals {
        for line in &j.lines {
            match line.account.classification {
                AccountClass::Revenue => {
                    income += line.credit.rials().saturating_sub(line.debit.rials())
                }
                AccountClass::Expense => {
                    expense += line.debit.rials().saturating_sub(line.credit.rials())
                }
                _ => {}
            }
        }
    }
    ReportsView {
        trial_balance,
        total_debit_rial,
        total_credit_rial,
        income_rial: income,
        expense_rial: expense,
        cash_rial: state.cashbook.net_cash(),
        as_of: chrono::Utc::now().format("%Y-%m-%d").to_string(),
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingInput {
    pub mode: String,
    pub business_name: String,
    pub currency: String,
    pub jalali: bool,
    #[serde(default)]
    pub sample_data: bool,
}

pub fn apply_onboarding(state: &mut State, input: OnboardingInput) -> Result<(), anyhow::Error> {
    use anyhow::ensure;
    ensure!(
        !input.business_name.trim().is_empty(),
        "نام را وارد کنید"
    );
    state.settings.app_mode = if input.mode == "business" || input.mode == "store" {
        AppMode::Business
    } else {
        AppMode::Personal
    };
    state.settings.business_name = input.business_name.trim().into();
    state.settings.default_currency = if input.currency == "rial" {
        Unit::Rial
    } else {
        Unit::Toman
    };
    state.settings.date_display = if input.jalali {
        crate::domain::DateDisplay::Jalali
    } else {
        crate::domain::DateDisplay::Iso
    };
    state.cashbook.ensure_default_wallets();
    if input.sample_data && state.cashbook.entries.is_empty() {
        let _ = state.cashbook.add(crate::cashbook::CashEntryInput {
            kind: crate::cashbook::CashKind::Income,
            amount_rial: 10_000_000,
            payee: "نمونه درآمد".into(),
            category: "فروش".into(),
            account: "صندوق".into(),
            to_account: String::new(),
            memo: "دادهٔ نمونه".into(),
            date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            splits: vec![],
            attachment_name: String::new(),
        })?;
        let _ = state.cashbook.add(crate::cashbook::CashEntryInput {
            kind: crate::cashbook::CashKind::Expense,
            amount_rial: 2_500_000,
            payee: "نمونه هزینه".into(),
            category: "اجاره".into(),
            account: "بانک".into(),
            to_account: String::new(),
            memo: "دادهٔ نمونه".into(),
            date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            splits: vec![],
            attachment_name: String::new(),
        })?;
    }
    state.extras.onboarding_done = true;
    Ok(())
}

pub fn save_extras(state: &mut State, extras: AppExtras) -> Result<AppExtras, anyhow::Error> {
    state.extras = extras;
    Ok(state.extras.clone())
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentInput {
    pub direction: String,
    pub amount_rial: i64,
    pub party: String,
    pub account: String,
    #[serde(default)]
    pub memo: String,
    #[serde(default)]
    pub date: String,
}

pub fn record_payment(
    state: &mut State,
    input: PaymentInput,
) -> Result<crate::cashbook::CashEntry, anyhow::Error> {
    let kind = if input.direction == "in" {
        crate::cashbook::CashKind::Income
    } else {
        crate::cashbook::CashKind::Expense
    };
    state.cashbook.add(crate::cashbook::CashEntryInput {
        kind,
        amount_rial: input.amount_rial,
        payee: input.party,
        category: if matches!(kind, crate::cashbook::CashKind::Income) {
            "دریافت".into()
        } else {
            "پرداخت".into()
        },
        account: input.account,
        to_account: String::new(),
        memo: input.memo,
        date: input.date,
        splits: vec![],
        attachment_name: String::new(),
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub path: String,
    pub generation: u64,
    pub at: String,
}

pub fn chart_rows(state: &State) -> Vec<crate::views::AccountRow> {
    crate::views::account_rows(&state.accounting)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalListRow {
    pub id: String,
    pub date: String,
    pub kind: String,
    pub memo: String,
    pub debit_rial: i64,
    pub credit_rial: i64,
    pub actor: String,
}

pub fn journal_list(state: &State) -> Vec<JournalListRow> {
    state
        .accounting
        .journals
        .iter()
        .rev()
        .map(|j| {
            let debit: i64 = j.lines.iter().map(|l| l.debit.rials()).sum();
            let credit: i64 = j.lines.iter().map(|l| l.credit.rials()).sum();
            JournalListRow {
                id: j.id.clone(),
                date: j.date.clone(),
                kind: match &j.kind {
                    JournalKind::Opening => "افتتاحیه".into(),
                    JournalKind::Manual => "دستی".into(),
                    JournalKind::Reversal { .. } => "برگشت".into(),
                },
                memo: if j.reason.is_empty() {
                    j.event.clone()
                } else {
                    j.reason.clone()
                },
                debit_rial: debit,
                credit_rial: credit,
                actor: j.actor.clone(),
            }
        })
        .collect()
}

// Silence unused import warnings for BalanceSide in some builds.
#[allow(dead_code)]
fn _side(s: BalanceSide) -> &'static str {
    match s {
        BalanceSide::Debit => "D",
        BalanceSide::Credit => "C",
    }
}

#[allow(dead_code)]
fn _settings_touch(s: &Settings) -> &str {
    &s.business_name
}

#[allow(dead_code)]
fn _ids() -> (String, String) {
    (id(), now())
}
