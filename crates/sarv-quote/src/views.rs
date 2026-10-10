//! Coarse-grained read models for the Tauri/React Phase-1 screens.

use serde::Serialize;

use crate::accounting::{AccountClass, AccountingState, BalanceSide, JournalKind};
use crate::cashbook::{CashKind, CashSplit, WalletKind};
use crate::domain::{AppMode, Customer, Settings, State, Unit};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgingBucket {
    pub label: String,
    pub amount_rial: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartyBalance {
    pub name: String,
    pub amount_rial: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardView {
    pub business_name: String,
    pub mode_label: String,
    pub app_mode: String,
    pub cash_on_hand_rial: i64,
    pub income_rial: i64,
    pub expense_rial: i64,
    pub profit_rial: i64,
    pub receivable_count: usize,
    pub payable_hint: String,
    pub receivables: Vec<PartyBalance>,
    pub payables: Vec<PartyBalance>,
    pub account_count: usize,
    pub journal_count: usize,
    pub contact_count: usize,
    pub catalog_count: usize,
    pub quote_count: usize,
    pub company_ready: bool,
    pub currency_label: String,
    pub recent: Vec<LedgerRow>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitView {
    pub category: String,
    pub amount_rial: i64,
    pub debit_rial: i64,
    pub credit_rial: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerRow {
    pub id: String,
    pub date: String,
    pub memo: String,
    pub kind: String,
    pub actor: String,
    pub payee: String,
    pub category: String,
    pub account: String,
    pub amount_rial: i64,
    pub signed_amount_rial: i64,
    pub running_balance_rial: i64,
    pub debit_rial: i64,
    pub credit_rial: i64,
    pub account_summary: String,
    pub status: String,
    pub attachment_name: String,
    pub splits: Vec<SplitView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountRow {
    pub id: String,
    pub code: String,
    pub name: String,
    pub classification: String,
    pub account_type: String,
    pub group: String,
    pub normal_balance: String,
    pub postable: bool,
    pub archived: bool,
    pub opening_balance_rial: i64,
    pub balance_rial: i64,
    pub currency: String,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactRow {
    pub id: String,
    pub name: String,
    pub phone: String,
    pub email: String,
    pub address: String,
    pub contact_name: String,
    pub archived: bool,
    pub roles: Vec<String>,
    pub is_customer: bool,
    pub is_supplier: bool,
    pub tax_id: String,
    pub notes: String,
    pub balance_rial: i64,
    pub aging: Vec<AgingBucket>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    pub settings: Settings,
    pub currency: String,
    pub date_display: String,
    pub appearance: String,
    pub digits: String,
    pub app_mode: String,
    pub text_direction: String,
}

pub fn dashboard(state: &State) -> DashboardView {
    let acc = &state.accounting;
    let recent = all_ledger_rows(state).into_iter().take(12).collect();
    let (journal_income, journal_expense, journal_cash) = flow_totals(acc);
    let income = journal_income.saturating_add(state.cashbook.income_total());
    let expense = journal_expense.saturating_add(state.cashbook.expense_total());
    let cash = if acc.journals.is_empty() {
        state.cashbook.net_cash()
    } else {
        journal_cash.saturating_add(state.cashbook.net_cash())
    };
    let (receivables, payables) = party_balances(state);
    let mode = match state.settings.app_mode {
        AppMode::Personal => "شخصی",
        AppMode::Business => "کسب‌وکار",
    };
    DashboardView {
        business_name: if state.settings.business_name.trim().is_empty() {
            "سرو".into()
        } else {
            state.settings.business_name.clone()
        },
        mode_label: mode.into(),
        app_mode: match state.settings.app_mode {
            AppMode::Personal => "personal".into(),
            AppMode::Business => "business".into(),
        },
        cash_on_hand_rial: cash,
        income_rial: income,
        expense_rial: expense,
        profit_rial: income.saturating_sub(expense),
        receivable_count: receivables.len(),
        payable_hint: if state.cashbook.entries.is_empty() && acc.journals.is_empty() {
            "هنوز تراکنشی نیست — Ctrl+N".into()
        } else {
            format!("{} تراکنش", state.cashbook.entries.len() + acc.journals.len())
        },
        receivables,
        payables,
        account_count: wallet_account_rows(state)
            .into_iter()
            .filter(|a| !a.archived)
            .count()
            .max(
                acc.accounts
                    .iter()
                    .filter(|a| !a.archived)
                    .count(),
            ),
        journal_count: acc.journals.len() + state.cashbook.entries.len(),
        contact_count: state.customers.iter().filter(|c| !c.archived).count(),
        catalog_count: state.catalogs.len(),
        quote_count: state.quotes.len(),
        company_ready: true,
        currency_label: match state.settings.default_currency {
            Unit::Rial => "ریال".into(),
            Unit::Toman => "تومان".into(),
        },
        recent,
    }
}

pub fn all_ledger_rows(state: &State) -> Vec<LedgerRow> {
    let mut rows = journal_ledger_rows(&state.accounting);
    rows.extend(cash_ledger_rows(state));
    rows.sort_by(|a, b| a.date.cmp(&b.date).then(a.id.cmp(&b.id)));
    let mut running = 0i64;
    for row in &mut rows {
        running = running.saturating_add(row.signed_amount_rial);
        row.running_balance_rial = running;
    }
    rows.reverse();
    rows
}

fn cash_ledger_rows(state: &State) -> Vec<LedgerRow> {
    state
        .cashbook
        .entries
        .iter()
        .map(|e| {
            let kind = match e.kind {
                CashKind::Expense => "هزینه",
                CashKind::Income => "درآمد",
                CashKind::Transfer => "انتقال",
            };
            let memo = if e.memo.trim().is_empty() {
                format!("{} — {}", kind, e.payee)
            } else {
                e.memo.clone()
            };
            let (debit, credit, signed) = match e.kind {
                CashKind::Expense => (e.amount_rial, 0, -e.amount_rial),
                CashKind::Income => (0, e.amount_rial, e.amount_rial),
                CashKind::Transfer => (e.amount_rial, e.amount_rial, 0),
            };
            let splits = if e.splits.is_empty() {
                vec![SplitView {
                    category: e.category.clone(),
                    amount_rial: e.amount_rial,
                    debit_rial: debit,
                    credit_rial: credit,
                }]
            } else {
                e.splits
                    .iter()
                    .map(|s| split_view(e.kind, s))
                    .collect()
            };
            LedgerRow {
                id: e.id.clone(),
                date: e.date.clone(),
                memo,
                kind: kind.into(),
                actor: e.payee.clone(),
                payee: e.payee.clone(),
                category: e.category.clone(),
                account: e.account.clone(),
                amount_rial: e.amount_rial,
                signed_amount_rial: signed,
                running_balance_rial: 0,
                debit_rial: debit,
                credit_rial: credit,
                account_summary: if e.to_account.is_empty() {
                    format!("{} · {}", e.account, e.category)
                } else {
                    format!("{} → {}", e.account, e.to_account)
                },
                status: if e.status.is_empty() {
                    "posted".into()
                } else {
                    e.status.clone()
                },
                attachment_name: e.attachment_name.clone(),
                splits,
            }
        })
        .collect()
}

fn split_view(kind: CashKind, s: &CashSplit) -> SplitView {
    let (debit, credit) = match kind {
        CashKind::Expense | CashKind::Transfer => (s.amount_rial, 0),
        CashKind::Income => (0, s.amount_rial),
    };
    SplitView {
        category: s.category.clone(),
        amount_rial: s.amount_rial,
        debit_rial: debit,
        credit_rial: credit,
    }
}

fn journal_ledger_rows(acc: &AccountingState) -> Vec<LedgerRow> {
    acc.journals
        .iter()
        .map(|j| {
            let debit: i64 = j.lines.iter().map(|l| l.debit.rials()).sum();
            let credit: i64 = j.lines.iter().map(|l| l.credit.rials()).sum();
            let account_summary = j
                .lines
                .iter()
                .map(|l| l.account.name.as_str())
                .take(3)
                .collect::<Vec<_>>()
                .join(" · ");
            let splits = j
                .lines
                .iter()
                .map(|l| SplitView {
                    category: l.account.name.clone(),
                    amount_rial: l.debit.rials().max(l.credit.rials()),
                    debit_rial: l.debit.rials(),
                    credit_rial: l.credit.rials(),
                })
                .collect();
            LedgerRow {
                id: j.id.clone(),
                date: j.date.clone(),
                memo: if j.reason.trim().is_empty() {
                    j.event.clone()
                } else {
                    j.reason.clone()
                },
                kind: match &j.kind {
                    JournalKind::Opening => "افتتاحیه".into(),
                    JournalKind::Manual => "دستی".into(),
                    JournalKind::Reversal { .. } => "برگشت".into(),
                },
                actor: j.actor.clone(),
                payee: j.actor.clone(),
                category: String::new(),
                account: account_summary.clone(),
                amount_rial: debit.max(credit),
                signed_amount_rial: 0,
                running_balance_rial: 0,
                debit_rial: debit,
                credit_rial: credit,
                account_summary,
                status: "posted".into(),
                attachment_name: String::new(),
                splits,
            }
        })
        .collect()
}

fn wallet_names(state: &State) -> Vec<String> {
    let mut book = state.cashbook.clone();
    book.ensure_default_wallets();
    let mut names: Vec<_> = book
        .wallets
        .iter()
        .filter(|w| !w.archived)
        .map(|w| w.name.clone())
        .collect();
    for e in &state.cashbook.entries {
        if !names.iter().any(|n| n == &e.account) {
            names.push(e.account.clone());
        }
        if !e.to_account.is_empty() && !names.iter().any(|n| n == &e.to_account) {
            names.push(e.to_account.clone());
        }
    }
    names
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestView {
    pub payees: Vec<String>,
    pub categories: Vec<String>,
    pub accounts: Vec<String>,
    pub payee_category: std::collections::BTreeMap<String, String>,
    pub last_account: String,
    pub last_to_account: String,
    pub last_kind: String,
}

pub fn suggestions(state: &State) -> SuggestView {
    let mut payees = state.cashbook.memory.payees.clone();
    for c in state.customers.iter().filter(|c| !c.archived) {
        if !payees.iter().any(|p| p == &c.name) {
            payees.push(c.name.clone());
        }
    }
    let mut categories = state.cashbook.memory.categories.clone();
    for preset in ["خوراک", "حمل‌ونقل", "اجاره", "حقوق", "فروش", "سایر"] {
        if !categories.iter().any(|c| c == preset) {
            categories.push(preset.into());
        }
    }
    SuggestView {
        payees,
        categories,
        accounts: wallet_names(state),
        payee_category: state.cashbook.memory.payee_category.clone(),
        last_account: state.cashbook.memory.last_account.clone(),
        last_to_account: state.cashbook.memory.last_to_account.clone(),
        last_kind: match state.cashbook.memory.last_kind {
            CashKind::Expense => "expense".into(),
            CashKind::Income => "income".into(),
            CashKind::Transfer => "transfer".into(),
        },
    }
}

pub fn wallet_account_rows(state: &State) -> Vec<AccountRow> {
    let mut book = state.cashbook.clone();
    book.ensure_default_wallets();
    let mut rows: Vec<_> = book
        .wallets
        .iter()
        .map(|w| AccountRow {
            id: w.id.clone(),
            code: format!("{:02}", w.sort_order + 1),
            name: w.name.clone(),
            classification: match w.kind {
                WalletKind::Loan => "بدهی".into(),
                _ => "دارایی".into(),
            },
            account_type: w.kind.label_fa().into(),
            group: if w.group.is_empty() {
                "اصلی".into()
            } else {
                w.group.clone()
            },
            normal_balance: match w.kind {
                WalletKind::Loan => "بستانکار".into(),
                _ => "بدهکار".into(),
            },
            postable: true,
            archived: w.archived,
            opening_balance_rial: w.opening_balance_rial,
            balance_rial: state.cashbook.balance_for(&w.name),
            currency: w.currency.clone(),
            sort_order: w.sort_order,
        })
        .collect();
    rows.sort_by(|a, b| a.sort_order.cmp(&b.sort_order).then(a.name.cmp(&b.name)));
    rows
}

pub fn account_rows(acc: &AccountingState) -> Vec<AccountRow> {
    let balances = account_balances(acc);
    let mut rows: Vec<_> = acc
        .accounts
        .iter()
        .map(|a| AccountRow {
            id: a.id.clone(),
            code: a.code.clone(),
            name: a.name.clone(),
            classification: class_fa(a.classification),
            account_type: class_fa(a.classification),
            group: class_fa(a.classification),
            normal_balance: match a.normal_balance {
                BalanceSide::Debit => "بدهکار".into(),
                BalanceSide::Credit => "بستانکار".into(),
            },
            postable: a.postable,
            archived: a.archived,
            opening_balance_rial: 0,
            balance_rial: *balances.get(&a.id).unwrap_or(&0),
            currency: String::new(),
            sort_order: 0,
        })
        .collect();
    rows.sort_by(|a, b| a.code.cmp(&b.code));
    rows
}

pub fn contact_rows(state: &State) -> Vec<ContactRow> {
    let mut rows: Vec<_> = state
        .customers
        .iter()
        .map(|c| {
            let balance = contact_balance(state, &c.name);
            let mut roles = Vec::new();
            if c.is_customer {
                roles.push("مشتری".into());
            }
            if c.is_supplier {
                roles.push("تأمین‌کننده".into());
            }
            if roles.is_empty() {
                roles.push("مخاطب".into());
            }
            ContactRow {
                id: c.id.clone(),
                name: c.name.clone(),
                phone: c.phone.clone(),
                email: c.email.clone(),
                address: c.address.clone(),
                contact_name: c.contact_name.clone(),
                archived: c.archived,
                roles,
                is_customer: c.is_customer,
                is_supplier: c.is_supplier,
                tax_id: c.tax_id.clone(),
                notes: c.notes.clone(),
                balance_rial: balance,
                aging: aging_for(balance),
            }
        })
        .collect();
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    rows
}

fn contact_balance(state: &State, name: &str) -> i64 {
    let mut bal = 0i64;
    for e in &state.cashbook.entries {
        if e.payee != name {
            continue;
        }
        match e.kind {
            // Expense to contact → we may owe them (negative = payable)
            CashKind::Expense => bal -= e.amount_rial,
            // Income from contact → they may owe us
            CashKind::Income => bal += e.amount_rial,
            CashKind::Transfer => {}
        }
    }
    bal
}

fn aging_for(balance: i64) -> Vec<AgingBucket> {
    let abs = balance.abs();
    vec![
        AgingBucket {
            label: "جاری".into(),
            amount_rial: abs,
        },
        AgingBucket {
            label: "۳۰+".into(),
            amount_rial: 0,
        },
        AgingBucket {
            label: "۶۰+".into(),
            amount_rial: 0,
        },
        AgingBucket {
            label: "۹۰+".into(),
            amount_rial: 0,
        },
    ]
}

fn party_balances(state: &State) -> (Vec<PartyBalance>, Vec<PartyBalance>) {
    let mut map = std::collections::BTreeMap::<String, i64>::new();
    for e in &state.cashbook.entries {
        let entry = map.entry(e.payee.clone()).or_default();
        match e.kind {
            CashKind::Income => *entry += e.amount_rial,
            CashKind::Expense => *entry -= e.amount_rial,
            CashKind::Transfer => {}
        }
    }
    let mut receivables = Vec::new();
    let mut payables = Vec::new();
    for (name, amount) in map {
        if amount > 0 {
            receivables.push(PartyBalance {
                name,
                amount_rial: amount,
            });
        } else if amount < 0 {
            payables.push(PartyBalance {
                name,
                amount_rial: -amount,
            });
        }
    }
    receivables.sort_by(|a, b| b.amount_rial.cmp(&a.amount_rial));
    payables.sort_by(|a, b| b.amount_rial.cmp(&a.amount_rial));
    receivables.truncate(8);
    payables.truncate(8);
    (receivables, payables)
}

pub fn settings_view(settings: &Settings) -> SettingsView {
    use crate::domain::{Appearance, DateDisplay, DisplayDigits, TextDirection};
    SettingsView {
        currency: match settings.default_currency {
            Unit::Rial => "rial".into(),
            Unit::Toman => "toman".into(),
        },
        date_display: match settings.date_display {
            DateDisplay::Iso => "iso".into(),
            DateDisplay::DayMonthYear => "dmy".into(),
            DateDisplay::Jalali => "jalali".into(),
        },
        appearance: match settings.appearance {
            Appearance::System => "system".into(),
            Appearance::Light => "light".into(),
            Appearance::Dark => "dark".into(),
        },
        digits: match settings.display_digits {
            DisplayDigits::Persian => "persian".into(),
            DisplayDigits::Latin => "latin".into(),
        },
        app_mode: match settings.app_mode {
            AppMode::Personal => "personal".into(),
            AppMode::Business => "business".into(),
        },
        text_direction: match settings.text_direction {
            TextDirection::Rtl => "rtl".into(),
            TextDirection::Ltr => "ltr".into(),
        },
        settings: settings.clone(),
    }
}

fn flow_totals(acc: &AccountingState) -> (i64, i64, i64) {
    let mut income = 0i64;
    let mut expense = 0i64;
    let mut cash = 0i64;
    for j in &acc.journals {
        for line in &j.lines {
            match line.account.classification {
                AccountClass::Revenue => {
                    income += line.credit.rials().saturating_sub(line.debit.rials())
                }
                AccountClass::Expense => {
                    expense += line.debit.rials().saturating_sub(line.credit.rials())
                }
                AccountClass::Asset if looks_like_cash(&line.account.name, &line.account.code) => {
                    cash += line.debit.rials().saturating_sub(line.credit.rials());
                }
                _ => {}
            }
        }
    }
    (income, expense, cash)
}

fn account_balances(acc: &AccountingState) -> std::collections::HashMap<String, i64> {
    let mut map = std::collections::HashMap::new();
    for j in &acc.journals {
        for line in &j.lines {
            let entry = map.entry(line.account.id.clone()).or_insert(0i64);
            match line.account.normal_balance {
                BalanceSide::Debit => {
                    *entry += line.debit.rials();
                    *entry -= line.credit.rials();
                }
                BalanceSide::Credit => {
                    *entry += line.credit.rials();
                    *entry -= line.debit.rials();
                }
            }
        }
    }
    map
}

fn looks_like_cash(name: &str, code: &str) -> bool {
    let n = name.to_lowercase();
    code.starts_with('1')
        && (n.contains("نقد")
            || n.contains("بانک")
            || n.contains("صندوق")
            || n.contains("cash")
            || n.contains("bank")
            || n.contains("wallet"))
}

fn class_fa(c: AccountClass) -> String {
    match c {
        AccountClass::Asset => "دارایی",
        AccountClass::Liability => "بدهی",
        AccountClass::Equity => "حقوق صاحبان",
        AccountClass::Revenue => "درآمد",
        AccountClass::Expense => "هزینه",
    }
    .into()
}

pub fn upsert_contact(state: &mut State, contact: ContactInput) -> Result<Customer, anyhow::Error> {
    use crate::domain::{customer_details, id, phone};
    use anyhow::ensure;
    phone(&contact.phone)?;
    customer_details(&contact.contact_name, &contact.email, &contact.address)?;
    ensure!(
        !contact.name.trim().is_empty() && contact.name.chars().count() <= 160,
        "نام مخاطب الزامی است"
    );
    ensure!(
        contact.is_customer || contact.is_supplier,
        "حداقل یک نقش مشتری یا تأمین‌کننده را انتخاب کنید"
    );
    ensure!(
        contact.tax_id.chars().count() <= 32,
        "شناسهٔ مالیاتی حداکثر ۳۲ نویسه است"
    );
    ensure!(
        contact.notes.chars().count() <= 4000,
        "یادداشت بیش از حد بلند است"
    );
    if let Some(existing) = state.customers.iter_mut().find(|c| c.id == contact.id) {
        existing.name = contact.name;
        existing.phone = contact.phone;
        existing.email = contact.email;
        existing.address = contact.address;
        existing.contact_name = contact.contact_name;
        existing.archived = contact.archived;
        existing.is_customer = contact.is_customer;
        existing.is_supplier = contact.is_supplier;
        existing.tax_id = contact.tax_id;
        existing.notes = contact.notes;
        return Ok(existing.clone());
    }
    let customer = Customer {
        id: if contact.id.is_empty() {
            id()
        } else {
            contact.id
        },
        name: contact.name,
        phone: contact.phone,
        contact_name: contact.contact_name,
        email: contact.email,
        address: contact.address,
        archived: contact.archived,
        is_customer: contact.is_customer,
        is_supplier: contact.is_supplier,
        tax_id: contact.tax_id,
        notes: contact.notes,
    };
    state.customers.push(customer.clone());
    Ok(customer)
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactInput {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub contact_name: String,
    #[serde(default)]
    pub archived: bool,
    #[serde(default = "crate::domain::default_true")]
    pub is_customer: bool,
    #[serde(default)]
    pub is_supplier: bool,
    #[serde(default)]
    pub tax_id: String,
    #[serde(default)]
    pub notes: String,
}
