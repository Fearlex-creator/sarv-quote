//! Simple money-in / money-out book that works without a full company chart.
//! Each entry implies a balanced pair of journal sides (personal mode language).

use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

use crate::domain::{id, now, MAX_MONEY};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum CashKind {
    #[default]
    Expense,
    Income,
    Transfer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum WalletKind {
    #[default]
    Cash,
    Bank,
    Card,
    Wallet,
    Savings,
    Loan,
}

impl WalletKind {
    pub fn label_fa(self) -> &'static str {
        match self {
            Self::Cash => "نقد",
            Self::Bank => "بانک",
            Self::Card => "کارت",
            Self::Wallet => "کیف پول",
            Self::Savings => "پس‌انداز",
            Self::Loan => "وام",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Wallet {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub kind: WalletKind,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub group: String,
    #[serde(default)]
    pub opening_balance_rial: i64,
    #[serde(default, skip_serializing_if = "is_false")]
    pub archived: bool,
    #[serde(default)]
    pub sort_order: i32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub currency: String,
}

fn is_false(v: &bool) -> bool {
    !*v
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CashSplit {
    pub category: String,
    pub amount_rial: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CashEntry {
    pub id: String,
    pub date: String,
    pub kind: CashKind,
    pub amount_rial: i64,
    pub payee: String,
    pub category: String,
    pub account: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub to_account: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub memo: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub splits: Vec<CashSplit>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub attachment_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub status: String,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiMemory {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub payees: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<String>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub payee_category: std::collections::BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub last_account: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub last_to_account: String,
    #[serde(default)]
    pub last_kind: CashKind,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cashbook {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entries: Vec<CashEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wallets: Vec<Wallet>,
    #[serde(default, skip_serializing_if = "UiMemory::is_default")]
    pub memory: UiMemory,
}

impl UiMemory {
    fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

impl Cashbook {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty() && self.wallets.is_empty() && self.memory == UiMemory::default()
    }

    pub fn ensure_default_wallets(&mut self) {
        if !self.wallets.is_empty() {
            return;
        }
        let defaults = [
            ("صندوق", WalletKind::Cash),
            ("بانک", WalletKind::Bank),
            ("کارت", WalletKind::Card),
            ("کیف پول", WalletKind::Wallet),
        ];
        for (i, (name, kind)) in defaults.into_iter().enumerate() {
            self.wallets.push(Wallet {
                id: id(),
                name: name.into(),
                kind,
                group: "اصلی".into(),
                opening_balance_rial: 0,
                archived: false,
                sort_order: i as i32,
                currency: String::new(),
            });
        }
    }

    pub fn add(&mut self, input: CashEntryInput) -> Result<CashEntry> {
        self.ensure_default_wallets();
        let amount_rial = input.amount_rial;
        ensure!(
            (1..=MAX_MONEY).contains(&amount_rial),
            "مبلغ باید بزرگ‌تر از صفر باشد"
        );
        let payee = input.payee.trim().to_string();
        ensure!(
            !payee.is_empty() && payee.chars().count() <= 160,
            "طرف حساب را وارد کنید"
        );
        let account = {
            let trimmed = input.account.trim();
            if trimmed.is_empty() {
                if self.memory.last_account.trim().is_empty() {
                    "صندوق".into()
                } else {
                    self.memory.last_account.clone()
                }
            } else {
                trimmed.to_string()
            }
        };
        let category = input.category.trim().to_string();
        let to_account = input.to_account.trim().to_string();
        if matches!(input.kind, CashKind::Transfer) {
            ensure!(!to_account.is_empty(), "حساب مقصد را انتخاب کنید");
            ensure!(to_account != account, "حساب مبدأ و مقصد باید متفاوت باشند");
        }
        let mut splits = input.splits;
        splits.retain(|s| s.amount_rial > 0 && !s.category.trim().is_empty());
        if !splits.is_empty() {
            let sum: i64 = splits.iter().map(|s| s.amount_rial).sum();
            ensure!(sum == amount_rial, "جمع تفکیک باید برابر مبلغ باشد");
        }
        let date = if input.date.trim().is_empty() {
            chrono::Utc::now().format("%Y-%m-%d").to_string()
        } else {
            input.date.trim().to_string()
        };
        ensure!(date.len() >= 10, "تاریخ نامعتبر است");
        let entry = CashEntry {
            id: id(),
            date,
            kind: input.kind,
            amount_rial,
            payee: payee.clone(),
            category: category.clone(),
            account: account.clone(),
            to_account: to_account.clone(),
            memo: input.memo.trim().to_string(),
            splits,
            attachment_name: input.attachment_name.trim().to_string(),
            status: "posted".into(),
            recorded_at: now(),
        };
        self.ensure_wallet_named(&entry.account);
        if !entry.to_account.is_empty() {
            self.ensure_wallet_named(&entry.to_account);
        }
        self.entries.push(entry.clone());
        remember(&mut self.memory, &entry);
        Ok(entry)
    }

    fn ensure_wallet_named(&mut self, name: &str) {
        if self.wallets.iter().any(|w| w.name == name) {
            return;
        }
        let order = self.wallets.len() as i32;
        self.wallets.push(Wallet {
            id: id(),
            name: name.into(),
            kind: WalletKind::Cash,
            group: "سایر".into(),
            opening_balance_rial: 0,
            archived: false,
            sort_order: order,
            currency: String::new(),
        });
    }

    pub fn upsert_wallet(&mut self, input: WalletInput) -> Result<Wallet> {
        self.ensure_default_wallets();
        let name = input.name.trim().to_string();
        ensure!(!name.is_empty() && name.chars().count() <= 80, "نام حساب الزامی است");
        ensure!(
            input.opening_balance_rial.abs() <= MAX_MONEY,
            "ماندهٔ افتتاحیه نامعتبر است"
        );
        if let Some(existing) = self.wallets.iter_mut().find(|w| w.id == input.id) {
            existing.name = name;
            existing.kind = input.kind;
            existing.group = input.group.trim().to_string();
            existing.opening_balance_rial = input.opening_balance_rial;
            existing.archived = input.archived;
            existing.sort_order = input.sort_order;
            existing.currency = input.currency.trim().to_string();
            return Ok(existing.clone());
        }
        let wallet = Wallet {
            id: if input.id.is_empty() {
                id()
            } else {
                input.id
            },
            name,
            kind: input.kind,
            group: input.group.trim().to_string(),
            opening_balance_rial: input.opening_balance_rial,
            archived: input.archived,
            sort_order: input.sort_order,
            currency: input.currency.trim().to_string(),
        };
        self.wallets.push(wallet.clone());
        Ok(wallet)
    }

    pub fn income_total(&self) -> i64 {
        self.entries
            .iter()
            .filter(|e| matches!(e.kind, CashKind::Income))
            .map(|e| e.amount_rial)
            .sum()
    }

    pub fn expense_total(&self) -> i64 {
        self.entries
            .iter()
            .filter(|e| matches!(e.kind, CashKind::Expense))
            .map(|e| e.amount_rial)
            .sum()
    }

    pub fn net_cash(&self) -> i64 {
        let opening: i64 = self
            .wallets
            .iter()
            .filter(|w| !w.archived)
            .map(|w| w.opening_balance_rial)
            .sum();
        opening
            .saturating_add(self.income_total())
            .saturating_sub(self.expense_total())
    }

    pub fn balance_for(&self, name: &str) -> i64 {
        let opening = self
            .wallets
            .iter()
            .find(|w| w.name == name)
            .map(|w| w.opening_balance_rial)
            .unwrap_or(0);
        let mut bal = opening;
        for e in &self.entries {
            match e.kind {
                CashKind::Income if e.account == name => bal += e.amount_rial,
                CashKind::Expense if e.account == name => bal -= e.amount_rial,
                CashKind::Transfer if e.account == name => bal -= e.amount_rial,
                CashKind::Transfer if e.to_account == name => bal += e.amount_rial,
                _ => {}
            }
        }
        bal
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CashEntryInput {
    pub kind: CashKind,
    pub amount_rial: i64,
    pub payee: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub account: String,
    #[serde(default)]
    pub to_account: String,
    #[serde(default)]
    pub memo: String,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub splits: Vec<CashSplit>,
    #[serde(default)]
    pub attachment_name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletInput {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub kind: WalletKind,
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub opening_balance_rial: i64,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub sort_order: i32,
    #[serde(default)]
    pub currency: String,
}

fn remember(memory: &mut UiMemory, entry: &CashEntry) {
    memory.last_kind = entry.kind;
    memory.last_account = entry.account.clone();
    if !entry.to_account.is_empty() {
        memory.last_to_account = entry.to_account.clone();
    }
    push_unique(&mut memory.payees, &entry.payee, 24);
    if !entry.category.is_empty() {
        push_unique(&mut memory.categories, &entry.category, 24);
        memory
            .payee_category
            .insert(entry.payee.clone(), entry.category.clone());
        while memory.payee_category.len() > 64 {
            if let Some(k) = memory.payee_category.keys().next().cloned() {
                memory.payee_category.remove(&k);
            } else {
                break;
            }
        }
    }
}

fn push_unique(list: &mut Vec<String>, value: &str, max: usize) {
    list.retain(|v| v != value);
    list.insert(0, value.to_string());
    list.truncate(max);
}
