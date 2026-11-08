//! Guided, immutable trading defaults and a reconciled fresh-company opening.
//! This is an opening boundary, not operational trade, treasury or statutory software.
use crate::accounting::{
    AccountClass, AccountingState, AuditOperation, BalanceSide, JournalKind, JournalLineInput,
    Money, NewAccount, PostCommand,
};
use crate::domain::{id, normalize, text};
use anyhow::{Result, ensure};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};

pub const CORE_POLICY_VERSION: &str = "core-policy-v1";
pub const CORE_OPENING_EVENT: &str = "core.opening.v1";
pub const QUANTITY_SCALE: i64 = 1000;
pub const MAX_SOURCE_QUANTITY: i64 = 1_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompanyTimezone {
    Utc,
    AsiaTehran,
}
impl CompanyTimezone {
    pub fn label(self) -> &'static str {
        match self {
            Self::Utc => "UTC",
            Self::AsiaTehran => "Asia/Tehran",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecognitionPolicy {
    AccrualPerpetualCombinedDelivery,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValuationPolicy {
    MovingWeightedAveragePerItemWarehouse,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoundingPolicy {
    PositiveHalfUpWholeRial,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReturnPolicy {
    OriginalAllocationCumulativeFinalResidual,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorePolicy {
    pub version: String,
    pub effective_date: String,
    pub timezone: CompanyTimezone,
    pub recognition: RecognitionPolicy,
    pub valuation: ValuationPolicy,
    pub rounding: RoundingPolicy,
    pub returns: ReturnPolicy,
    pub currency: String,
    pub quantity_scale: i64,
    pub tax_enabled: bool,
    pub allow_negative_stock: bool,
    pub allow_stock_backdating: bool,
}
impl CorePolicy {
    pub fn v1(effective_date: &str, timezone: CompanyTimezone) -> Self {
        Self {
            version: CORE_POLICY_VERSION.into(),
            effective_date: effective_date.into(),
            timezone,
            recognition: RecognitionPolicy::AccrualPerpetualCombinedDelivery,
            valuation: ValuationPolicy::MovingWeightedAveragePerItemWarehouse,
            rounding: RoundingPolicy::PositiveHalfUpWholeRial,
            returns: ReturnPolicy::OriginalAllocationCumulativeFinalResidual,
            currency: "IRR".into(),
            quantity_scale: QUANTITY_SCALE,
            tax_enabled: false,
            allow_negative_stock: false,
            allow_stock_backdating: false,
        }
    }
    pub fn summary(&self) -> String {
        format!(
            "{} · تعهدی / موجودی دائمی · ریال صحیح · مقدار هزارم · گردکردن نیم‌به‌بالا · میانگین موزون هر کالا/انبار · برگشت با بهای اصلی · موجودی منفی و ثبت قدیمی ممنوع · مالیات غیرفعال · تاریخ مدنی مستقل / {}",
            self.version,
            self.timezone.label()
        )
    }
    fn validate(&self) -> Result<()> {
        civil_date(&self.effective_date)?;
        ensure!(
            *self == Self::v1(&self.effective_date, self.timezone),
            "سیاست نسخهٔ یک نامعتبر یا تغییر داده شده است"
        );
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CoreRole {
    Cash,
    Bank,
    Receivables,
    ChequesReceivable,
    Inventory,
    SupplierAdvances,
    FixedAssets,
    Payables,
    ChequesPayable,
    Grni,
    CustomerAdvances,
    Loans,
    OwnerCapital,
    OwnerDrawings,
    RetainedEarnings,
    Sales,
    SalesReturns,
    Cogs,
    OperatingExpense,
    BankFees,
    InventoryLoss,
}
impl CoreRole {
    pub const ALL: [Self; 21] = [
        Self::Cash,
        Self::Bank,
        Self::Receivables,
        Self::ChequesReceivable,
        Self::Inventory,
        Self::SupplierAdvances,
        Self::FixedAssets,
        Self::Payables,
        Self::ChequesPayable,
        Self::Grni,
        Self::CustomerAdvances,
        Self::Loans,
        Self::OwnerCapital,
        Self::OwnerDrawings,
        Self::RetainedEarnings,
        Self::Sales,
        Self::SalesReturns,
        Self::Cogs,
        Self::OperatingExpense,
        Self::BankFees,
        Self::InventoryLoss,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Self::Cash => "cash",
            Self::Bank => "bank",
            Self::Receivables => "receivables",
            Self::ChequesReceivable => "cheques_receivable",
            Self::Inventory => "inventory",
            Self::SupplierAdvances => "supplier_advances",
            Self::FixedAssets => "fixed_assets",
            Self::Payables => "payables",
            Self::ChequesPayable => "cheques_payable",
            Self::Grni => "grni",
            Self::CustomerAdvances => "customer_advances",
            Self::Loans => "loans",
            Self::OwnerCapital => "owner_capital",
            Self::OwnerDrawings => "owner_drawings",
            Self::RetainedEarnings => "retained_earnings",
            Self::Sales => "sales",
            Self::SalesReturns => "sales_returns",
            Self::Cogs => "cogs",
            Self::OperatingExpense => "operating_expense",
            Self::BankFees => "bank_fees",
            Self::InventoryLoss => "inventory_loss",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Cash => "صندوق",
            Self::Bank => "بانک",
            Self::Receivables => "حساب‌های دریافتنی",
            Self::ChequesReceivable => "اسناد دریافتنی",
            Self::Inventory => "موجودی کالا",
            Self::SupplierAdvances => "پیش‌پرداخت تأمین‌کنندگان",
            Self::FixedAssets => "دارایی‌های ثابت",
            Self::Payables => "حساب‌های پرداختنی",
            Self::ChequesPayable => "اسناد پرداختنی",
            Self::Grni => "کالای دریافت‌شده، فاکتور‌نشده",
            Self::CustomerAdvances => "پیش‌دریافت مشتریان",
            Self::Loans => "وام‌ها",
            Self::OwnerCapital => "سرمایه",
            Self::OwnerDrawings => "برداشت مالک",
            Self::RetainedEarnings => "سود انباشته",
            Self::Sales => "فروش",
            Self::SalesReturns => "برگشت از فروش",
            Self::Cogs => "بهای تمام‌شده کالای فروش‌رفته",
            Self::OperatingExpense => "هزینه‌های عملیاتی",
            Self::BankFees => "کارمزد بانکی",
            Self::InventoryLoss => "ضایعات و کسری موجودی",
        }
    }
    pub fn classification(self) -> AccountClass {
        match self {
            Self::Cash
            | Self::Bank
            | Self::Receivables
            | Self::ChequesReceivable
            | Self::Inventory
            | Self::SupplierAdvances
            | Self::FixedAssets => AccountClass::Asset,
            Self::Payables
            | Self::ChequesPayable
            | Self::Grni
            | Self::CustomerAdvances
            | Self::Loans => AccountClass::Liability,
            Self::OwnerCapital | Self::OwnerDrawings | Self::RetainedEarnings => {
                AccountClass::Equity
            }
            Self::Sales | Self::SalesReturns => AccountClass::Revenue,
            _ => AccountClass::Expense,
        }
    }
    pub fn normal_balance(self) -> BalanceSide {
        match self {
            Self::OwnerDrawings | Self::SalesReturns => BalanceSide::Debit,
            _ => match self.classification() {
                AccountClass::Asset | AccountClass::Expense => BalanceSide::Debit,
                _ => BalanceSide::Credit,
            },
        }
    }
    pub fn is_control(self) -> bool {
        matches!(
            self,
            Self::Cash
                | Self::Bank
                | Self::Receivables
                | Self::ChequesReceivable
                | Self::Inventory
                | Self::SupplierAdvances
                | Self::Payables
                | Self::ChequesPayable
                | Self::Grni
                | Self::CustomerAdvances
                | Self::Cogs
                | Self::InventoryLoss
        )
    }
    /// Opening registers cover balance-sheet controls; COGS/loss have zero opening only.
    pub fn has_opening_register(self) -> bool {
        self.is_control() && !matches!(self, Self::Cogs | Self::InventoryLoss)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChartLeaf {
    pub role: CoreRole,
    pub code: String,
    pub name: String,
}
pub fn default_chart() -> Vec<ChartLeaf> {
    let codes = [
        "1101", "1102", "1201", "1202", "1301", "1401", "1501", "2101", "2102", "2201", "2301",
        "2401", "3101", "3102", "3201", "4101", "4102", "5101", "5201", "5202", "5301",
    ];
    CoreRole::ALL
        .into_iter()
        .zip(codes)
        .map(|(role, code)| ChartLeaf {
            role,
            code: code.into(),
            name: role.label().into(),
        })
        .collect()
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewCoreCompany {
    pub name: String,
    pub fiscal_name: String,
    pub start_date: String,
    pub end_date: String,
    pub timezone: CompanyTimezone,
    pub chart: Vec<ChartLeaf>,
    pub actor: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoreCompanyResult {
    pub company_id: String,
    pub period_id: String,
    pub setup_id: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleMapping {
    pub role: CoreRole,
    pub account_id: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigureCore {
    pub company_id: String,
    pub period_id: String,
    pub policy: CorePolicy,
    pub mappings: Vec<RoleMapping>,
    pub actor: String,
    pub reason: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoreSetup {
    pub id: String,
    pub command: ConfigureCore,
    pub command_sha256: String,
}
impl CoreSetup {
    pub fn account_id(&self, role: CoreRole) -> Result<&str> {
        self.command
            .mappings
            .iter()
            .find(|m| m.role == role)
            .map(|m| m.account_id.as_str())
            .ok_or_else(|| anyhow::anyhow!("نگاشت نقش حساب پیدا نشد"))
    }
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoreFoundation {
    pub setups: Vec<CoreSetup>,
    pub openings: Vec<OpeningRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub zero_requests: Vec<ZeroOpeningRequest>,
}
impl CoreFoundation {
    pub fn is_empty(&self) -> bool {
        self.setups.is_empty() && self.openings.is_empty() && self.zero_requests.is_empty()
    }
    pub fn validate_transition(&self, next: &Self) -> Result<()> {
        ensure!(
            next.setups.starts_with(&self.setups)
                && next.openings.starts_with(&self.openings)
                && next.zero_requests.starts_with(&self.zero_requests),
            "تنظیمات یا جزئیات افتتاحیهٔ ثبت‌شده قابل تغییر یا حذف نیست"
        );
        Ok(())
    }
}

macro_rules! record { ($name:ident { $($(#[$meta:meta])* $field:ident : $ty:ty),* $(,)? }) => {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct $name { $($(#[$meta])* pub $field: $ty),* }
}; }
record!(OpeningBalance {
    account_id: String,
    debit: Money,
    credit: Money,
    source_reference: String
});
record!(CoreParty {
    id: String,
    company_id: String,
    name: String,
    customer: bool,
    supplier: bool
});
record!(CoreUnit {
    id: String,
    company_id: String,
    name: String
});
record!(PackRatio {
    unit_id: String,
    numerator: i64,
    denominator: i64
});
record!(CoreItem { id: String, company_id: String, sku: String, name: String, base_unit_id: String, indivisible: bool, packs: Vec<PackRatio> });
record!(CoreWarehouse {
    id: String,
    company_id: String,
    name: String
});
record!(OpeningCash {
    id: String,
    company_id: String,
    name: String,
    balance: Money,
    source_reference: String
});
record!(OpeningBank {
    id: String,
    company_id: String,
    name: String,
    bank_reference: String,
    balance: Money,
    statement_balance: Money,
    source_reference: String
});
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpenItemSide {
    Receivable,
    Payable,
}
record!(OpeningOpenItem {
    id: String,
    company_id: String,
    party_id: String,
    side: OpenItemSide,
    document_reference: String,
    document_date: String,
    due_date: String,
    remaining: Money
});
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdvanceSide {
    Supplier,
    Customer,
}
record!(OpeningAdvance {
    id: String,
    company_id: String,
    party_id: String,
    side: AdvanceSide,
    source_reference: String,
    document_date: String,
    remaining: Money
});
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChequeDirection {
    Receivable,
    Payable,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpeningChequeState {
    Held,
    Issued,
}
record!(OpeningCheque {
    id: String,
    company_id: String,
    party_id: String,
    direction: ChequeDirection,
    state: OpeningChequeState,
    bank_reference: String,
    holder: String,
    cheque_reference: String,
    settlement_reference: String,
    issue_date: String,
    due_date: String,
    amount: Money
});
record!(OpeningStock {
    id: String,
    company_id: String,
    item_id: String,
    warehouse_id: String,
    unit_id: String,
    quantity_thousandths: i64,
    value: Money,
    source_reference: String
});
record!(OpeningGrni {
    id: String,
    company_id: String,
    party_id: String,
    item_id: String,
    warehouse_id: String,
    unit_id: String,
    quantity_thousandths: i64,
    value: Money,
    receipt_reference: String,
    received_date: String
});
record!(OpeningSourceRow { row: usize, id: String, kind: String, original_values: Vec<String> });
record!(OpeningImportSource { sha256: String, rows: Vec<OpeningSourceRow> });
record!(OpeningCommand {
    company_id: String, period_id: String, setup_id: String, as_of: String, source_id: String,
    request_key: String, actor: String, reason: String,
    balances: Vec<OpeningBalance>, parties: Vec<CoreParty>, units: Vec<CoreUnit>, items: Vec<CoreItem>, warehouses: Vec<CoreWarehouse>, cash: Vec<OpeningCash>, banks: Vec<OpeningBank>, open_items: Vec<OpeningOpenItem>, advances: Vec<OpeningAdvance>, cheques: Vec<OpeningCheque>, stock: Vec<OpeningStock>, grni: Vec<OpeningGrni>, #[serde(default, skip_serializing_if = "Option::is_none")] import_source: Option<OpeningImportSource>
});
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpeningEffect {
    #[default]
    Journal,
    ZeroBalanceInitialization,
}
record!(ZeroOpeningRequest {
    request_key: String,
    company_id: String,
    command_sha256: String,
    initialization_id: String
});
record!(OpeningRecord {
    command: OpeningCommand,
    journal_id: String,
    frozen_setup: CoreSetup,
    command_sha256: String,
    #[serde(default)]
    effect: OpeningEffect
});
record!(ControlDifference {
    role: CoreRole,
    account_id: String,
    ledger: Money,
    detail: Money,
    difference: Money
});
record!(OpeningPreview { company_id: String, as_of: String, controls: Vec<ControlDifference>, total_debit: Money, total_credit: Money, balanced: bool, reconciled: bool });

pub fn civil_date(value: &str) -> Result<NaiveDate> {
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| anyhow::anyhow!("تاریخ مدنی معتبر YYYY-MM-DD وارد کنید"))?;
    ensure!(
        date.format("%Y-%m-%d").to_string() == value,
        "قالب تاریخ مدنی نامعتبر است"
    );
    Ok(date)
}
/// Nonnegative exact rational rounded once, with positive half-up ties.
pub fn positive_half_up(numerator: i128, denominator: i128) -> Result<i64> {
    ensure!(numerator >= 0 && denominator > 0, "نسبت مبلغ نامعتبر است");
    let q = numerator / denominator;
    let r = numerator % denominator;
    let rounded = q
        .checked_add(i128::from(r >= denominator - r))
        .ok_or_else(|| anyhow::anyhow!("سرریز گردکردن مبلغ"))?;
    i64::try_from(rounded).map_err(|_| anyhow::anyhow!("سرریز مبلغ ریالی"))
}
impl OpeningCommand {
    pub fn base_quantity(
        &self,
        item_id: &str,
        unit_id: &str,
        selected_thousandths: i64,
    ) -> Result<i64> {
        ensure!(
            (1..=MAX_SOURCE_QUANTITY).contains(&selected_thousandths),
            "مقدار ورودی باید مثبت و در محدودهٔ منبع باشد"
        );
        let item = self
            .items
            .iter()
            .find(|i| i.id == item_id)
            .ok_or_else(|| anyhow::anyhow!("کالای مقدار پیدا نشد"))?;
        let (n, d) = if item.base_unit_id == unit_id {
            (1, 1)
        } else {
            let pack = item
                .packs
                .iter()
                .find(|p| p.unit_id == unit_id)
                .ok_or_else(|| anyhow::anyhow!("ضریب بسته‌بندی مشخص نیست"))?;
            (pack.numerator, pack.denominator)
        };
        ensure!(n > 0 && d > 0, "نسبت بسته‌بندی باید مثبت باشد");
        let product = i128::from(selected_thousandths)
            .checked_mul(i128::from(n))
            .ok_or_else(|| anyhow::anyhow!("سرریز تبدیل واحد"))?;
        ensure!(
            product % i128::from(d) == 0,
            "مقدار تبدیل‌شده در هزارم واحد پایه دقیق نیست"
        );
        let base = i64::try_from(product / i128::from(d))
            .map_err(|_| anyhow::anyhow!("سرریز مقدار پایه"))?;
        ensure!(
            base > 0 && (!item.indivisible || base % QUANTITY_SCALE == 0),
            "کالا به مقدار غیرقابل‌تقسیم نیاز دارد"
        );
        Ok(base)
    }
}

impl AccountingState {
    /// Company, fiscal year, groups, chart and policy are created together or unchanged.
    pub fn setup_core_company(&mut self, command: NewCoreCompany) -> Result<CoreCompanyResult> {
        let mut next = self.clone();
        let mut roles = HashSet::new();
        let mut codes = HashSet::new();
        ensure!(
            command.chart.len() == CoreRole::ALL.len(),
            "تمام نقش‌های نمودار حساب لازم است"
        );
        for leaf in &command.chart {
            text(&leaf.code)?;
            text(&leaf.name)?;
            ensure!(
                roles.insert(leaf.role) && codes.insert(normalize(&leaf.code)),
                "نقش یا کد نمودار تکراری است"
            );
        }
        let company_id = next.add_company(&command.name, &command.actor)?;
        let period_id = next.add_period(
            &company_id,
            &command.fiscal_name,
            &command.start_date,
            &command.end_date,
            &command.actor,
        )?;
        let mut groups = Vec::new();
        for (code, name, class, normal) in [
            ("1", "دارایی‌ها", AccountClass::Asset, BalanceSide::Debit),
            ("2", "بدهی‌ها", AccountClass::Liability, BalanceSide::Credit),
            (
                "3",
                "حقوق مالکانه",
                AccountClass::Equity,
                BalanceSide::Credit,
            ),
            ("4", "درآمدها", AccountClass::Revenue, BalanceSide::Credit),
            ("5", "هزینه‌ها", AccountClass::Expense, BalanceSide::Debit),
        ] {
            let group = next.add_account(
                NewAccount {
                    company_id: company_id.clone(),
                    parent_id: None,
                    code: code.into(),
                    name: name.into(),
                    classification: class,
                    normal_balance: normal,
                    postable: false,
                    control: false,
                },
                &command.actor,
            )?;
            groups.push((class, group));
        }
        let mut mappings = Vec::new();
        for leaf in &command.chart {
            let role = leaf.role;
            let parent = groups
                .iter()
                .find(|(c, _)| *c == role.classification())
                .unwrap()
                .1
                .clone();
            let account_id = next.add_account(
                NewAccount {
                    company_id: company_id.clone(),
                    parent_id: Some(parent),
                    code: leaf.code.clone(),
                    name: leaf.name.clone(),
                    classification: role.classification(),
                    normal_balance: role.normal_balance(),
                    postable: true,
                    control: role.is_control(),
                },
                &command.actor,
            )?;
            mappings.push(RoleMapping { role, account_id });
        }
        let setup_id = next.configure_core(ConfigureCore {
            company_id: company_id.clone(),
            period_id: period_id.clone(),
            policy: CorePolicy::v1(&command.start_date, command.timezone),
            mappings,
            actor: command.actor,
            reason: "راه‌اندازی هدایت‌شدهٔ هستهٔ بازرگانی".into(),
        })?;
        *self = next;
        Ok(CoreCompanyResult {
            company_id,
            period_id,
            setup_id,
        })
    }
    fn validate_core_setup(&self, command: &ConfigureCore) -> Result<()> {
        text(&command.company_id)?;
        text(&command.actor)?;
        text(&command.reason)?;
        command.policy.validate()?;
        ensure!(
            self.companies.iter().any(|c| c.id == command.company_id),
            "کسب‌وکار راه‌اندازی پیدا نشد"
        );
        let period = self
            .periods
            .iter()
            .find(|p| p.id == command.period_id && p.company_id == command.company_id)
            .ok_or_else(|| anyhow::anyhow!("سال مالی راه‌اندازی متعلق به کسب‌وکار نیست"))?;
        ensure!(!period.locked, "سال مالی قفل است");
        ensure!(
            command.policy.effective_date == period.start_date,
            "سیاست افتتاحیه باید از شروع سال مالی معتبر باشد"
        );
        ensure!(
            !self
                .journals
                .iter()
                .any(|j| j.company_id == command.company_id),
            "راه‌اندازی هسته فقط برای کسب‌وکار بدون سابقهٔ مالی است؛ اتصال ماندهٔ موجود پشتیبانی نمی‌شود"
        );
        ensure!(
            !self
                .core
                .setups
                .iter()
                .any(|s| s.command.company_id == command.company_id),
            "تنظیمات هستهٔ کسب‌وکار قبلاً ثبت شده است"
        );
        self.validate_role_mappings(command, true)
    }
    fn validate_role_mappings(&self, command: &ConfigureCore, active: bool) -> Result<()> {
        ensure!(
            command.mappings.len() == CoreRole::ALL.len(),
            "نگاشت تمام نقش‌های هسته لازم است"
        );
        let mut roles = HashSet::new();
        let mut accounts = HashSet::new();
        for m in &command.mappings {
            ensure!(
                roles.insert(m.role) && accounts.insert(&m.account_id),
                "نقش یا حساب نگاشت تکراری است"
            );
            let a = self
                .accounts
                .iter()
                .find(|a| a.id == m.account_id)
                .ok_or_else(|| anyhow::anyhow!("حساب نگاشت پیدا نشد"))?;
            ensure!(
                a.company_id == command.company_id
                    && a.classification == m.role.classification()
                    && a.normal_balance == m.role.normal_balance()
                    && a.postable
                    && a.control == m.role.is_control()
                    && (!active || !a.archived),
                "طبقه، ماهیت، کنترل یا وضعیت حساب نگاشت نامعتبر است"
            );
            let parent = self
                .accounts
                .iter()
                .find(|p| Some(p.id.as_str()) == a.parent_id.as_deref())
                .ok_or_else(|| anyhow::anyhow!("حساب نگاشت باید گروه والد داشته باشد"))?;
            ensure!(
                parent.company_id == command.company_id
                    && parent.classification == a.classification
                    && !parent.postable
                    && !parent.control
                    && parent.parent_id.is_none()
                    && (!active || !parent.archived),
                "گروه والد نگاشت نامعتبر است"
            );
        }
        Ok(())
    }
    pub fn configure_core(&mut self, command: ConfigureCore) -> Result<String> {
        self.validate_core_setup(&command)?;
        let command_sha256 = command_hash(&command)?;
        let setup_id = id();
        self.activity(
            &command.company_id,
            &setup_id,
            AuditOperation::CoreConfigured,
            &command.actor,
            &command.reason,
        );
        self.core.setups.push(CoreSetup {
            id: setup_id.clone(),
            command,
            command_sha256,
        });
        Ok(setup_id)
    }
    pub(crate) fn replay_core_setup(&mut self, setup: &CoreSetup) -> Result<()> {
        text(&setup.id)?;
        self.validate_core_setup(&setup.command)?;
        ensure!(
            setup.command_sha256 == command_hash(&setup.command)?,
            "محتوای تصویر سیاست با هش ثبت‌شده برابر نیست"
        );
        ensure!(
            !self.core.setups.iter().any(|s| s.id == setup.id),
            "شناسهٔ تنظیمات تکراری است"
        );
        self.activity(
            &setup.command.company_id,
            &setup.id,
            AuditOperation::CoreConfigured,
            &setup.command.actor,
            &setup.command.reason,
        );
        self.core.setups.push(setup.clone());
        Ok(())
    }
    pub fn opening_preview(&self, command: &OpeningCommand) -> Result<OpeningPreview> {
        self.validate_opening(command, true)
    }
    pub fn post_opening(&mut self, command: OpeningCommand) -> Result<String> {
        // Public calls also have failure atomicity; persistence adds durable CAS.
        let mut next = self.clone();
        let journal = next.post_opening_inner(command)?;
        *self = next;
        Ok(journal)
    }
    fn post_opening_inner(&mut self, command: OpeningCommand) -> Result<String> {
        text(&command.request_key)?;
        if let Some(prior) = self
            .core
            .openings
            .iter()
            .find(|o| o.command.company_id == command.company_id)
        {
            ensure!(
                prior.command_sha256 == opening_hash(&prior.command)?
                    && self.core.setups.contains(&prior.frozen_setup),
                "تصویر افتتاحیهٔ موجود نامعتبر است"
            );
            let mut old = prior.command.clone();
            let mut new = command.clone();
            old.request_key.clear();
            new.request_key.clear();
            ensure!(
                old == new,
                "افتتاحیهٔ کسب‌وکار قبلاً ثبت شده؛ محتوای متفاوت یا افتتاحیهٔ اصلاحی پشتیبانی نمی‌شود"
            );
            if prior.effect == OpeningEffect::ZeroBalanceInitialization {
                let initialization_id = prior.journal_id.clone();
                return self.record_zero_request(&command, &initialization_id);
            }
            let post = opening_post(&command);
            return self.post_core_opening(post);
        }
        let preview = self.validate_opening(&command, true)?;
        ensure!(
            !self
                .journals
                .iter()
                .any(|j| j.company_id == command.company_id),
            "افتتاحیه باید پیش از تمام سابقهٔ مالی باشد؛ اتصال به ماندهٔ موجود پشتیبانی نمی‌شود"
        );
        ensure!(
            preview.balanced,
            "جمع مستقل افتتاحیه برابر نیست؛ سرمایه را صریحاً اصلاح کنید"
        );
        ensure!(
            preview.reconciled,
            "جزئیات افتتاحیه با تمام حساب‌های کنترلی برابر نیست"
        );
        let (journal_id, effect) =
            if preview.total_debit == Money::ZERO && preview.total_credit == Money::ZERO {
                let initialization_id = id();
                self.record_zero_request(&command, &initialization_id)?;
                self.activity(
                    &command.company_id,
                    &initialization_id,
                    AuditOperation::CoreOpeningInitialized,
                    &command.actor,
                    &command.reason,
                );
                (initialization_id, OpeningEffect::ZeroBalanceInitialization)
            } else {
                (
                    self.post_core_opening(opening_post(&command))?,
                    OpeningEffect::Journal,
                )
            };
        let frozen_setup = self
            .core
            .setups
            .iter()
            .find(|s| s.id == command.setup_id)
            .unwrap()
            .clone();
        let command_sha256 = opening_hash(&command)?;
        self.core.openings.push(OpeningRecord {
            command,
            journal_id: journal_id.clone(),
            frozen_setup,
            command_sha256,
            effect,
        });
        Ok(journal_id)
    }
    pub(crate) fn replay_core_opening(&mut self, command: OpeningCommand) -> Result<String> {
        self.post_opening(command)
    }
    fn record_zero_request(
        &mut self,
        command: &OpeningCommand,
        initialization_id: &str,
    ) -> Result<String> {
        ensure!(
            !self
                .requests
                .iter()
                .any(|r| r.command.request_key == command.request_key),
            "کلید درخواست قبلاً برای عملیات مالی استفاده شده است"
        );
        ensure!(
            !self.procurement.uses_nonfinancial_key(&command.request_key),
            "کلید درخواست قبلاً برای رویداد خرید یا انبار استفاده شده است"
        );
        ensure!(
            !self.sales.uses_nonfinancial_key(&command.request_key),
            "کلید درخواست قبلاً برای رویداد فروش استفاده شده است"
        );
        let record = ZeroOpeningRequest {
            request_key: command.request_key.clone(),
            company_id: command.company_id.clone(),
            command_sha256: opening_business_hash(command)?,
            initialization_id: initialization_id.into(),
        };
        if let Some(prior) = self
            .core
            .zero_requests
            .iter()
            .find(|r| r.request_key == record.request_key)
        {
            ensure!(
                *prior == record,
                "کلید درخواست با محتوای متفاوت استفاده شده است"
            );
        } else {
            self.core.zero_requests.push(record);
        }
        Ok(initialization_id.into())
    }
    pub(crate) fn replay_zero_request(&mut self, request: &ZeroOpeningRequest) -> Result<String> {
        let opening = self
            .core
            .openings
            .iter()
            .find(|o| {
                o.command.company_id == request.company_id
                    && o.journal_id == request.initialization_id
                    && o.effect == OpeningEffect::ZeroBalanceInitialization
            })
            .ok_or_else(|| {
                anyhow::anyhow!("درخواست راه‌اندازی صفر به افتتاحیهٔ معتبر اشاره نمی‌کند")
            })?;
        ensure!(
            request.command_sha256 == opening_business_hash(&opening.command)?,
            "هش درخواست راه‌اندازی صفر نامعتبر است"
        );
        let mut command = opening.command.clone();
        command.request_key = request.request_key.clone();
        self.post_opening(command)
    }
    /// Only the supported opening boundary is certified; absence is never a verified zero.
    pub fn reconcile_opening(&self, company_id: &str, as_of: &str) -> Result<OpeningPreview> {
        civil_date(as_of)?;
        let opening = self
            .core
            .openings
            .iter()
            .find(|o| o.command.company_id == company_id)
            .ok_or_else(|| anyhow::anyhow!("افتتاحیهٔ تطبیق‌شده وجود ندارد؛ صفر تأییدشده نیست"))?;
        ensure!(
            as_of == opening.command.as_of,
            "این گزارش فقط مرز آغاز روز افتتاحیه را تأیید می‌کند"
        );
        ensure!(
            opening.command_sha256 == opening_hash(&opening.command)?,
            "محتوای جزئیات افتتاحیه با هش ثبت‌شده برابر نیست"
        );
        ensure!(
            self.core.setups.contains(&opening.frozen_setup),
            "تصویر سیاست افتتاحیه با سیاست ثبت‌شده سازگار نیست"
        );
        let mut preview = self.validate_opening(&opening.command, false)?;
        if opening.effect == OpeningEffect::ZeroBalanceInitialization {
            ensure!(
                preview.total_debit == Money::ZERO
                    && preview.total_credit == Money::ZERO
                    && preview.reconciled,
                "راه‌اندازی بدون مانده نمی‌تواند مبلغ مالی داشته باشد"
            );
            ensure!(
                self.audit.iter().any(|a| a.id != opening.journal_id
                    && a.entity_id == opening.journal_id
                    && a.company_id == company_id
                    && a.operation == AuditOperation::CoreOpeningInitialized
                    && a.actor == opening.command.actor
                    && a.reason == opening.command.reason),
                "سوابق تغییرات راه‌اندازی صفر پیدا نشد"
            );
            ensure!(
                !self.journals.iter().any(|j| j.id == opening.journal_id),
                "راه‌اندازی صفر نباید سند مالی داشته باشد"
            );
            return Ok(preview);
        }
        let journal = self
            .journals
            .iter()
            .find(|j| j.id == opening.journal_id)
            .ok_or_else(|| anyhow::anyhow!("سند افتتاحیه پیدا نشد"))?;
        ensure!(
            journal.event == CORE_OPENING_EVENT
                && journal.kind == JournalKind::Opening
                && journal.company_id == company_id
                && journal.date == as_of,
            "سند و مرز افتتاحیه سازگار نیست"
        );
        for row in &mut preview.controls {
            let mut debit = Money::ZERO;
            let mut credit = Money::ZERO;
            for line in journal
                .lines
                .iter()
                .filter(|l| l.account.id == row.account_id)
            {
                debit = debit.checked_add(line.debit)?;
                credit = credit.checked_add(line.credit)?;
            }
            row.ledger = if row.role.normal_balance() == BalanceSide::Debit {
                debit.checked_sub(credit)?
            } else {
                credit.checked_sub(debit)?
            };
            row.difference = row.ledger.checked_sub(row.detail)?;
        }
        preview.reconciled = preview.controls.iter().all(|r| r.difference == Money::ZERO);
        Ok(preview)
    }
    fn validate_opening(&self, c: &OpeningCommand, active: bool) -> Result<OpeningPreview> {
        for value in [
            &c.company_id,
            &c.period_id,
            &c.setup_id,
            &c.source_id,
            &c.request_key,
            &c.actor,
            &c.reason,
        ] {
            text(value)?;
        }
        civil_date(&c.as_of)?;
        let setup = self
            .core
            .setups
            .iter()
            .find(|s| {
                s.id == c.setup_id
                    && s.command.company_id == c.company_id
                    && s.command.period_id == c.period_id
            })
            .ok_or_else(|| {
                anyhow::anyhow!("تنظیمات افتتاحیه متعلق به این کسب‌وکار و سال مالی نیست")
            })?;
        setup.command.policy.validate()?;
        ensure!(
            setup.command_sha256 == command_hash(&setup.command)?,
            "تصویر سیاست افتتاحیه نامعتبر است"
        );
        self.validate_role_mappings(&setup.command, active)?;
        let period = self
            .periods
            .iter()
            .find(|p| p.id == c.period_id && p.company_id == c.company_id)
            .ok_or_else(|| anyhow::anyhow!("سال مالی افتتاحیه پیدا نشد"))?;
        ensure!(!active || !period.locked, "سال مالی قفل است");
        ensure!(
            c.as_of == period.start_date && c.as_of == setup.command.policy.effective_date,
            "افتتاحیه باید در آغاز روز شروع سال مالی باشد"
        );
        if active {
            ensure!(
                !self
                    .core
                    .openings
                    .iter()
                    .any(|o| o.command.company_id == c.company_id),
                "افتتاحیهٔ این کسب‌وکار قبلاً نهایی شده است؛ پیش‌نمایش جدید مجاز نیست"
            );
            ensure!(
                !self.journals.iter().any(|j| j.company_id == c.company_id),
                "پیش‌نمایش افتتاحیه فقط برای کسب‌وکار بدون سابقهٔ مالی است"
            );
        }
        let mut ids: HashSet<&str> = self
            .companies
            .iter()
            .map(|x| x.id.as_str())
            .chain(self.periods.iter().map(|x| x.id.as_str()))
            .chain(self.accounts.iter().map(|x| x.id.as_str()))
            .chain(self.journals.iter().map(|x| x.id.as_str()))
            .chain(self.audit.iter().map(|x| x.id.as_str()))
            .chain(self.core.setups.iter().map(|x| x.id.as_str()))
            .chain(self.core.openings.iter().map(|o| o.journal_id.as_str()))
            .collect();
        ids.extend(self.procurement_entity_ids());
        for opening in self
            .core
            .openings
            .iter()
            .filter(|o| o.command.company_id != c.company_id)
        {
            for (entity, _) in opening_entities(&opening.command) {
                ids.insert(entity);
            }
        }
        for (entity, company) in opening_entities(c) {
            text(entity)?;
            ensure!(
                company == c.company_id && ids.insert(entity),
                "کسب‌وکار یا شناسهٔ جزئیات افتتاحیه نامعتبر/تکراری است"
            );
        }
        let mut unit_names = HashSet::new();
        for u in &c.units {
            text(&u.name)?;
            ensure!(unit_names.insert(normalize(&u.name)), "نام واحد تکراری است");
        }
        for p in &c.parties {
            text(&p.name)?;
            ensure!(
                p.customer || p.supplier,
                "شخص باید نقش مشتری یا تأمین‌کننده داشته باشد"
            );
        }
        let mut skus = HashSet::new();
        for item in &c.items {
            text(&item.sku)?;
            text(&item.name)?;
            ensure!(skus.insert(normalize(&item.sku)), "کد کالا تکراری است");
            ensure!(
                c.units.iter().any(|u| u.id == item.base_unit_id),
                "واحد پایهٔ کالا مشخص نیست"
            );
            let mut packs = HashSet::new();
            for pack in &item.packs {
                ensure!(
                    pack.unit_id != item.base_unit_id
                        && packs.insert(&pack.unit_id)
                        && c.units.iter().any(|u| u.id == pack.unit_id)
                        && pack.numerator > 0
                        && pack.denominator > 0,
                    "نسبت یا واحد بسته‌بندی نامعتبر/تکراری است"
                );
            }
        }
        for w in &c.warehouses {
            text(&w.name)?;
        }
        let mut detail: BTreeMap<CoreRole, Money> = CoreRole::ALL
            .into_iter()
            .filter(|r| r.is_control())
            .map(|r| (r, Money::ZERO))
            .collect();
        let mut sources = HashSet::new();
        for cash in &c.cash {
            text(&cash.name)?;
            text(&cash.source_reference)?;
            amount(cash.balance, false)?;
            ensure!(
                sources.insert((CoreRole::Cash, normalize(&cash.source_reference))),
                "مرجع صندوق تکراری است"
            );
            add_detail(&mut detail, CoreRole::Cash, cash.balance)?;
        }
        let mut bank_ids = HashSet::new();
        for bank in &c.banks {
            text(&bank.name)?;
            text(&bank.bank_reference)?;
            text(&bank.source_reference)?;
            amount(bank.balance, false)?;
            amount(bank.statement_balance, false)?;
            ensure!(
                bank.balance == bank.statement_balance,
                "ماندهٔ بانک و صورت‌حساب مستقل برابر نیست"
            );
            ensure!(
                bank_ids.insert(normalize(&bank.bank_reference))
                    && sources.insert((CoreRole::Bank, normalize(&bank.source_reference))),
                "حساب بانک یا مرجع آن تکراری است"
            );
            add_detail(&mut detail, CoreRole::Bank, bank.balance)?;
        }
        let mut documents = HashSet::new();
        for open in &c.open_items {
            let customer = open.side == OpenItemSide::Receivable;
            party(c, &open.party_id, customer)?;
            text(&open.document_reference)?;
            historical_date(&open.document_date, &c.as_of)?;
            civil_date(&open.due_date)?;
            ensure!(
                open.due_date >= open.document_date,
                "سررسید پیش از تاریخ سند است"
            );
            amount(open.remaining, true)?;
            let role = if customer {
                CoreRole::Receivables
            } else {
                CoreRole::Payables
            };
            ensure!(
                documents.insert((role, &open.party_id, normalize(&open.document_reference))),
                "فاکتور باز تکراری است"
            );
            add_detail(&mut detail, role, open.remaining)?;
        }
        for advance in &c.advances {
            let customer = advance.side == AdvanceSide::Customer;
            party(c, &advance.party_id, customer)?;
            text(&advance.source_reference)?;
            historical_date(&advance.document_date, &c.as_of)?;
            amount(advance.remaining, true)?;
            let role = if customer {
                CoreRole::CustomerAdvances
            } else {
                CoreRole::SupplierAdvances
            };
            ensure!(
                documents.insert((
                    role,
                    &advance.party_id,
                    normalize(&advance.source_reference)
                )),
                "مرجع پیش‌پرداخت/پیش‌دریافت تکراری است"
            );
            add_detail(&mut detail, role, advance.remaining)?;
        }
        let mut cheques = HashSet::new();
        for cheque in &c.cheques {
            let customer = cheque.direction == ChequeDirection::Receivable;
            party(c, &cheque.party_id, customer)?;
            for value in [
                &cheque.bank_reference,
                &cheque.holder,
                &cheque.cheque_reference,
                &cheque.settlement_reference,
            ] {
                text(value)?;
            }
            historical_date(&cheque.issue_date, &c.as_of)?;
            civil_date(&cheque.due_date)?;
            ensure!(
                cheque.due_date >= cheque.issue_date,
                "سررسید چک پیش از تاریخ صدور است"
            );
            ensure!(
                cheque.state
                    == if customer {
                        OpeningChequeState::Held
                    } else {
                        OpeningChequeState::Issued
                    },
                "وضعیت چک افتتاحیه باید نگهداری‌شده/صادرشده و وصول‌نشده باشد"
            );
            amount(cheque.amount, true)?;
            ensure!(
                !c.open_items.iter().any(|o| o.party_id == cheque.party_id
                    && (o.side == OpenItemSide::Receivable) == customer
                    && normalize(&o.document_reference) == normalize(&cheque.settlement_reference)),
                "مرجع تسویهٔ چک با فاکتور باز هم‌پوشانی دارد؛ افتتاحیهٔ جزئی ترکیبی پشتیبانی نمی‌شود"
            );
            ensure!(
                cheques.insert((
                    normalize(&cheque.bank_reference),
                    normalize(&cheque.cheque_reference)
                )),
                "شناسهٔ بانکی چک تکراری است"
            );
            let role = if customer {
                CoreRole::ChequesReceivable
            } else {
                CoreRole::ChequesPayable
            };
            add_detail(&mut detail, role, cheque.amount)?;
        }
        let mut pools: BTreeMap<(&str, &str), (i64, Money)> = BTreeMap::new();
        let mut stock_sources = HashSet::new();
        for stock in &c.stock {
            warehouse(c, &stock.warehouse_id)?;
            text(&stock.source_reference)?;
            amount(stock.value, false)?;
            ensure!(
                stock_sources.insert(normalize(&stock.source_reference)),
                "مرجع موجودی افتتاحیه تکراری است"
            );
            let qty =
                c.base_quantity(&stock.item_id, &stock.unit_id, stock.quantity_thousandths)?;
            let pool = pools
                .entry((&stock.item_id, &stock.warehouse_id))
                .or_insert((0, Money::ZERO));
            pool.0 = pool
                .0
                .checked_add(qty)
                .ok_or_else(|| anyhow::anyhow!("سرریز مقدار موجودی"))?;
            pool.1 = pool.1.checked_add(stock.value)?;
            add_detail(&mut detail, CoreRole::Inventory, stock.value)?;
        }
        let mut receipt_refs = HashSet::new();
        let mut unbilled: BTreeMap<(&str, &str), (i64, Money)> = BTreeMap::new();
        for receipt in &c.grni {
            party(c, &receipt.party_id, false)?;
            warehouse(c, &receipt.warehouse_id)?;
            text(&receipt.receipt_reference)?;
            historical_date(&receipt.received_date, &c.as_of)?;
            amount(receipt.value, false)?;
            ensure!(
                receipt_refs.insert((&receipt.party_id, normalize(&receipt.receipt_reference))),
                "رسید فاکتور‌نشده تکراری است"
            );
            let qty = c.base_quantity(
                &receipt.item_id,
                &receipt.unit_id,
                receipt.quantity_thousandths,
            )?;
            let pool = unbilled
                .entry((&receipt.item_id, &receipt.warehouse_id))
                .or_insert((0, Money::ZERO));
            pool.0 = pool
                .0
                .checked_add(qty)
                .ok_or_else(|| anyhow::anyhow!("سرریز مقدار کالای فاکتور‌نشده"))?;
            pool.1 = pool.1.checked_add(receipt.value)?;
            add_detail(&mut detail, CoreRole::Grni, receipt.value)?;
        }
        for (key, (qty, value)) in unbilled {
            let pool = pools.get(&key).ok_or_else(|| {
                anyhow::anyhow!("کالای فاکتور‌نشده باید در همان موجودی کالا/انبار منظور شده باشد")
            })?;
            ensure!(
                qty <= pool.0 && value.rials() <= pool.1.rials(),
                "کالای فاکتور‌نشده از زیرمجموعهٔ موجودی همان کالا/انبار بیشتر است"
            );
        }
        let mut balances = BTreeMap::new();
        let mut debit = Money::ZERO;
        let mut credit = Money::ZERO;
        for balance in &c.balances {
            text(&balance.source_reference)?;
            ensure!(
                balance.debit.rials() >= 0
                    && balance.credit.rials() >= 0
                    && (balance.debit == Money::ZERO || balance.credit == Money::ZERO),
                "ماندهٔ افتتاحیه باید غیرمنفی و یک‌طرفه باشد"
            );
            let a = self
                .accounts
                .iter()
                .find(|a| a.id == balance.account_id && a.company_id == c.company_id)
                .ok_or_else(|| anyhow::anyhow!("حساب مانده متعلق به کسب‌وکار نیست"))?;
            ensure!(
                a.postable && (!active || !a.archived),
                "حساب مانده بایگانی یا گروه غیرقابل ثبت است"
            );
            ensure!(
                !matches!(
                    a.classification,
                    AccountClass::Revenue | AccountClass::Expense
                ) || (balance.debit == Money::ZERO && balance.credit == Money::ZERO),
                "افتتاحیهٔ تازه نباید درآمد یا هزینهٔ تاریخی ایجاد کند"
            );
            if a.control {
                ensure!(
                    setup
                        .command
                        .mappings
                        .iter()
                        .any(|m| m.account_id == a.id && m.role.is_control()),
                    "حساب کنترلی مانده نگاشت معتبر ندارد"
                );
            }
            ensure!(
                balances
                    .insert(balance.account_id.as_str(), balance)
                    .is_none(),
                "ماندهٔ حساب افتتاحیه تکراری است"
            );
            debit = debit.checked_add(balance.debit)?;
            credit = credit.checked_add(balance.credit)?;
        }
        let mut controls = Vec::new();
        for role in CoreRole::ALL.into_iter().filter(|r| r.is_control()) {
            let account_id = setup.account_id(role)?;
            let balance = balances.get(account_id).ok_or_else(|| {
                anyhow::anyhow!(
                    "ماندهٔ مستقل {} باید حتی در حالت صفر صریحاً اعلام شود",
                    role.label()
                )
            })?;
            let ledger = if role.normal_balance() == BalanceSide::Debit {
                balance.debit.checked_sub(balance.credit)?
            } else {
                balance.credit.checked_sub(balance.debit)?
            };
            let value = detail[&role];
            controls.push(ControlDifference {
                role,
                account_id: account_id.into(),
                ledger,
                detail: value,
                difference: ledger.checked_sub(value)?,
            });
        }
        if let Some(source) = &c.import_source {
            ensure!(
                source.sha256.len() == 64 && source.sha256.bytes().all(|b| b.is_ascii_hexdigit()),
                "هش منبع افتتاحیه نامعتبر است"
            );
            let mut rows = HashSet::new();
            let mut source_ids = HashSet::new();
            for row in &source.rows {
                text(&row.id)?;
                text(&row.kind)?;
                ensure!(
                    row.row > 0
                        && rows.insert(row.row)
                        && source_ids.insert((&row.kind, &row.id))
                        && row.original_values.len() <= 100
                        && row.original_values.iter().all(|v| v.len() <= 4000),
                    "ردیف منبع افتتاحیه نامعتبر یا بیش از محدوده است"
                );
            }
        }
        let reconciled = controls.iter().all(|r| r.difference == Money::ZERO);
        Ok(OpeningPreview {
            company_id: c.company_id.clone(),
            as_of: c.as_of.clone(),
            controls,
            total_debit: debit,
            total_credit: credit,
            balanced: debit == credit,
            reconciled,
        })
    }
}
fn add_detail(values: &mut BTreeMap<CoreRole, Money>, role: CoreRole, value: Money) -> Result<()> {
    let current = values.get_mut(&role).unwrap();
    *current = current.checked_add(value)?;
    Ok(())
}
fn amount(value: Money, positive: bool) -> Result<()> {
    ensure!(
        value.rials() >= i64::from(positive) && value.rials() <= crate::domain::MAX_MONEY,
        "مبلغ جزئیات افتتاحیه منفی/صفر یا خارج از محدودهٔ منبع است"
    );
    Ok(())
}
fn historical_date(value: &str, as_of: &str) -> Result<()> {
    civil_date(value)?;
    ensure!(value <= as_of, "تاریخ سابقه پس از مرز افتتاحیه است");
    Ok(())
}
fn party(c: &OpeningCommand, id: &str, customer: bool) -> Result<()> {
    ensure!(
        c.parties
            .iter()
            .any(|p| p.id == id && if customer { p.customer } else { p.supplier }),
        "شخص یا نقش مشتری/تأمین‌کننده معتبر نیست"
    );
    Ok(())
}
fn warehouse(c: &OpeningCommand, id: &str) -> Result<()> {
    ensure!(
        c.warehouses.iter().any(|w| w.id == id),
        "انبار مقدار پیدا نشد"
    );
    Ok(())
}
fn opening_entities(c: &OpeningCommand) -> Vec<(&str, &str)> {
    c.parties
        .iter()
        .map(|x| (x.id.as_str(), x.company_id.as_str()))
        .chain(
            c.units
                .iter()
                .map(|x| (x.id.as_str(), x.company_id.as_str())),
        )
        .chain(
            c.items
                .iter()
                .map(|x| (x.id.as_str(), x.company_id.as_str())),
        )
        .chain(
            c.warehouses
                .iter()
                .map(|x| (x.id.as_str(), x.company_id.as_str())),
        )
        .chain(
            c.cash
                .iter()
                .map(|x| (x.id.as_str(), x.company_id.as_str())),
        )
        .chain(
            c.banks
                .iter()
                .map(|x| (x.id.as_str(), x.company_id.as_str())),
        )
        .chain(
            c.open_items
                .iter()
                .map(|x| (x.id.as_str(), x.company_id.as_str())),
        )
        .chain(
            c.advances
                .iter()
                .map(|x| (x.id.as_str(), x.company_id.as_str())),
        )
        .chain(
            c.cheques
                .iter()
                .map(|x| (x.id.as_str(), x.company_id.as_str())),
        )
        .chain(
            c.stock
                .iter()
                .map(|x| (x.id.as_str(), x.company_id.as_str())),
        )
        .chain(
            c.grni
                .iter()
                .map(|x| (x.id.as_str(), x.company_id.as_str())),
        )
        .collect()
}
fn opening_post(c: &OpeningCommand) -> PostCommand {
    PostCommand {
        company_id: c.company_id.clone(),
        period_id: c.period_id.clone(),
        date: c.as_of.clone(),
        source_id: format!("core-opening:{}", c.company_id),
        source_revision: 1,
        event: CORE_OPENING_EVENT.into(),
        request_key: c.request_key.clone(),
        actor: c.actor.clone(),
        reason: c.reason.clone(),
        kind: JournalKind::Opening,
        lines: c
            .balances
            .iter()
            .filter(|b| b.debit != Money::ZERO || b.credit != Money::ZERO)
            .map(|b| JournalLineInput {
                account_id: b.account_id.clone(),
                debit: b.debit,
                credit: b.credit,
                description: b.source_reference.clone(),

                dimension: String::new(),
            })
            .collect(),
    }
}

fn command_hash(command: &ConfigureCore) -> Result<String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(command)?)
    ))
}

fn opening_hash(command: &OpeningCommand) -> Result<String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(command)?)
    ))
}

fn opening_business_hash(command: &OpeningCommand) -> Result<String> {
    let mut command = command.clone();
    command.request_key.clear();
    opening_hash(&command)
}
