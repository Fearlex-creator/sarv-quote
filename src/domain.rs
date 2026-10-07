use anyhow::{Result, bail, ensure};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub const MAX_MONEY: i64 = 9_000_000_000_000_000;
pub const MAX_QUOTE_LINES: usize = 10_000;
const MAX_REFERENCE_CHARS: usize = 160;
const MAX_NOTES_CHARS: usize = 4000;
pub fn now() -> String {
    Utc::now().to_rfc3339()
}
pub fn id() -> String {
    Uuid::new_v4().to_string()
}
pub fn normalize(s: &str) -> String {
    digits(s)
        .chars()
        .map(|c| match c {
            'ي' | 'ى' => 'ی',
            'ك' => 'ک',
            '\u{200c}' => ' ',
            c => c,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
pub fn digits(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '۰'..='۹' => char::from_u32(c as u32 - '۰' as u32 + '0' as u32).unwrap(),
            '٠'..='٩' => char::from_u32(c as u32 - '٠' as u32 + '0' as u32).unwrap(),
            '٫' => '.',
            c => c,
        })
        .collect()
}
pub fn fa(s: impl ToString) -> String {
    s.to_string()
        .chars()
        .map(|c| {
            if c.is_ascii_digit() {
                char::from_u32(c as u32 - '0' as u32 + '۰' as u32).unwrap()
            } else {
                c
            }
        })
        .collect()
}
/// Directional isolation is display-only; untrusted input controls are rejected.
pub fn ltr(s: &str) -> String {
    format!("\u{2066}{s}\u{2069}")
}
pub fn grouped(n: i64) -> String {
    let s = n.to_string();
    let len = s.len();
    fa(s.chars()
        .enumerate()
        .flat_map(|(i, c)| {
            if i > 0 && (len - i).is_multiple_of(3) {
                vec!['٬', c]
            } else {
                vec![c]
            }
        })
        .collect::<String>())
}
pub fn text(s: &str) -> Result<()> {
    ensure!(
        !s.trim().is_empty()
            && s.len() <= 1024
            && !s
                .chars()
                .any(|c| c.is_control()
                    || matches!(c, '\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}')),
        "متن خالی، بسیار بلند یا دارای نویسهٔ کنترلی است"
    );
    Ok(())
}
fn forbidden_control(c: char) -> bool {
    c.is_control() || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}
fn phone(s: &str) -> Result<()> {
    ensure!(
        s.len() <= 100 && !s.chars().any(forbidden_control),
        "تلفن نامعتبر"
    );
    Ok(())
}
fn document_date(s: &str) -> Result<Option<NaiveDate>> {
    if s.is_empty() {
        return Ok(None);
    }
    let normalized = digits(s);
    ensure!(
        normalized.len() == 10
            && normalized
                .bytes()
                .enumerate()
                .all(|(i, c)| if matches!(i, 4 | 7) {
                    c == b'-'
                } else {
                    c.is_ascii_digit()
                })
            && !normalized.starts_with("0000"),
        "تاریخ میلادی باید به صورت YYYY-MM-DD و دارای سال مثبت باشد"
    );
    let parsed = NaiveDate::parse_from_str(&normalized, "%Y-%m-%d")?;
    ensure!(
        parsed.format("%Y-%m-%d").to_string() == normalized,
        "تاریخ باید به صورت YYYY-MM-DD باشد"
    );
    Ok(Some(parsed))
}
fn metadata(reference: &str, quote_date: &str, valid_until: &str, notes: &str) -> Result<()> {
    ensure!(
        reference.chars().count() <= MAX_REFERENCE_CHARS
            && !reference.chars().any(forbidden_control),
        "مرجع حداکثر ۱۶۰ نویسه و بدون نویسهٔ کنترلی است"
    );
    ensure!(
        notes.chars().count() <= MAX_NOTES_CHARS
            && !notes.chars().any(|c| c != '\n' && forbidden_control(c)),
        "یادداشت حداکثر ۴۰۰۰ نویسه است؛ فقط شکست خط مجاز است"
    );
    let date = document_date(quote_date)?;
    let expiry = document_date(valid_until)?;
    if let Some(expiry) = expiry {
        ensure!(
            date.is_some_and(|date| expiry >= date),
            "تاریخ اعتبار باید برابر یا پس از تاریخ پیشنهاد باشد"
        );
    }
    Ok(())
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum Density {
    #[default]
    Comfortable,
    Compact,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum DisplayDigits {
    #[default]
    Persian,
    Latin,
}
impl DisplayDigits {
    pub fn is_default(&self) -> bool {
        *self == Self::Persian
    }
    pub fn format(self, value: impl ToString) -> String {
        match self {
            Self::Persian => fa(value),
            Self::Latin => digits(&value.to_string()),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum DateDisplay {
    #[default]
    Iso,
    DayMonthYear,
}
impl DateDisplay {
    pub fn is_default(&self) -> bool {
        *self == Self::Iso
    }
    /// Gregorian display only. Storage and editable document dates remain ISO.
    pub fn format(self, value: &str) -> String {
        if self == Self::Iso {
            return value.into();
        }
        let Some(prefix) = value.get(..10) else {
            return value.into();
        };
        let Ok(Some(date)) = document_date(prefix) else {
            return value.into();
        };
        if value.len() > 10 && DateTime::parse_from_rfc3339(value).is_err() {
            return value.into();
        }
        format!("{}{}", date.format("%d/%m/%Y"), &value[10..])
    }
}
fn zero_u32(n: &u32) -> bool {
    *n == 0
}
fn zero_i64(n: &i64) -> bool {
    *n == 0
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Settings {
    pub business_name: String,
    pub business_phone: String,
    pub business_address: String,
    pub header: String,
    pub footer: String,
    pub default_terms: String,
    pub default_currency: Unit,
    pub quote_prefix: String,
    pub next_number: u64,
    pub default_validity_days: u32,
    pub default_discount_rial: i64,
    pub tax_basis_points: u32,
    pub appearance: Appearance,
    pub density: Density,
    pub display_digits: DisplayDigits,
    #[serde(default, skip_serializing_if = "DateDisplay::is_default")]
    pub date_display: DateDisplay,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            business_name: String::new(),
            business_phone: String::new(),
            business_address: String::new(),
            header: String::new(),
            footer: String::new(),
            default_terms: String::new(),
            default_currency: Unit::Toman,
            quote_prefix: "Q-".into(),
            next_number: 1,
            default_validity_days: 7,
            default_discount_rial: 0,
            tax_basis_points: 0,
            appearance: Appearance::System,
            density: Density::Comfortable,
            display_digits: DisplayDigits::Persian,
            date_display: DateDisplay::Iso,
        }
    }
}
impl Settings {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
    pub fn validate(&self) -> Result<()> {
        issuer_metadata(
            &self.business_name,
            &self.business_phone,
            &self.business_address,
            &self.header,
            &self.footer,
            &self.default_terms,
        )?;
        ensure!(
            self.quote_prefix.chars().count() <= 32
                && !self.quote_prefix.chars().any(forbidden_control),
            "پیشوند شماره نامعتبر است"
        );
        ensure!(
            self.next_number > 0 && self.next_number < u64::MAX,
            "شمارهٔ بعدی خارج از محدوده است"
        );
        ensure!(
            self.default_validity_days <= 3650,
            "اعتبار حداکثر ۳۶۵۰ روز است"
        );
        ensure!(
            (0..=MAX_MONEY).contains(&self.default_discount_rial),
            "تخفیف پیش‌فرض نامعتبر است"
        );
        ensure!(
            self.tax_basis_points <= 10_000,
            "نرخ مالیات تنظیم‌شده باید بین صفر و صد درصد باشد"
        );
        Ok(())
    }
}
fn issuer_metadata(
    name: &str,
    business_phone: &str,
    address: &str,
    header: &str,
    footer: &str,
    terms: &str,
) -> Result<()> {
    ensure!(
        name.chars().count() <= 160 && !name.chars().any(forbidden_control),
        "نام کسب‌وکار نامعتبر است"
    );
    phone(business_phone)?;
    for value in [address, header, footer, terms] {
        metadata("", "", "", value)?;
    }
    Ok(())
}
/// User-configured quotation tax; no statutory rate is inferred.
pub fn tax_amount(net_rial: i64, basis_points: u32) -> Result<i64> {
    ensure!(
        (0..=MAX_MONEY).contains(&net_rial) && basis_points <= 10_000,
        "مبلغ یا نرخ مالیات نامعتبر است"
    );
    let amount = (i128::from(net_rial) * i128::from(basis_points) + 5000) / 10_000;
    Ok(amount as i64)
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Unit {
    Rial,
    Toman,
}
impl Unit {
    pub fn label(self) -> &'static str {
        match self {
            Self::Rial => "ریال",
            Self::Toman => "تومان",
        }
    }
    pub fn parse(s: &str) -> Result<Self> {
        match normalize(s).as_str() {
            "rial" | "ریال" => Ok(Self::Rial),
            "toman" | "تومان" => Ok(Self::Toman),
            _ => bail!("واحد باید rial یا toman باشد"),
        }
    }
    pub fn amount(self, s: &str) -> Result<i64> {
        let n = self.nonnegative(s)?;
        ensure!(n > 0, "قیمت صفر پذیرفته نمی‌شود");
        Ok(n)
    }
    pub fn nonnegative(self, s: &str) -> Result<i64> {
        let s = digits(s);
        let parts = s.trim().split('.').collect::<Vec<_>>();
        ensure!(
            parts.len() <= 2
                && parts
                    .iter()
                    .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit())),
            "مبلغ باید عدد نامنفی بدون جداکننده باشد"
        );
        let fractional = parts.get(1).copied().unwrap_or("");
        ensure!(
            fractional.is_empty() || (self == Self::Toman && fractional.len() == 1),
            "ریال صحیح است؛ تومان حداکثر یک رقم اعشار دارد"
        );
        let n = parts[0]
            .parse::<i64>()?
            .checked_mul(if self == Self::Toman { 10 } else { 1 })
            .and_then(|n| {
                n.checked_add(if fractional.is_empty() {
                    0
                } else {
                    fractional.parse().ok()?
                })
            })
            .ok_or_else(|| anyhow::anyhow!("سرریز مبلغ"))?;
        ensure!(n <= MAX_MONEY, "مبلغ خارج از محدوده است");
        Ok(n)
    }
    pub fn number(self, rial: i64) -> String {
        if self == Self::Rial {
            fa(rial)
        } else {
            fa(if rial % 10 == 0 {
                (rial / 10).to_string()
            } else {
                format!("{}.{:01}", rial / 10, rial % 10)
            })
        }
    }
    pub fn display(self, rial: i64) -> String {
        if self == Self::Rial {
            format!("{} ریال", grouped(rial))
        } else {
            format!(
                "{}{} تومان",
                grouped(rial / 10),
                if rial % 10 == 0 {
                    String::new()
                } else {
                    format!(".{}", fa(rial % 10))
                }
            )
        }
    }
}
/// Quantities use thousandths. No binary floating point enters pricing.
pub fn quantity(s: &str) -> Result<i64> {
    let s = digits(s);
    let parts = s.trim().split('.').collect::<Vec<_>>();
    ensure!(
        parts.len() <= 2
            && !parts[0].is_empty()
            && parts
                .iter()
                .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit())),
        "تعداد نامعتبر است"
    );
    let fractional = parts.get(1).copied().unwrap_or("");
    ensure!(fractional.len() <= 3, "حداکثر سه رقم اعشار برای تعداد");
    let n = parts[0]
        .parse::<i64>()?
        .checked_mul(1000)
        .and_then(|n| n.checked_add(format!("{fractional:0<3}").parse::<i64>().ok()?))
        .ok_or_else(|| anyhow::anyhow!("سرریز تعداد"))?;
    ensure!(
        n > 0 && n <= 1_000_000_000,
        "تعداد باید مثبت و حداکثر یک میلیون باشد"
    );
    Ok(n)
}
pub fn qty_display(q: i64) -> String {
    fa(format!("{}.{:03}", q / 1000, q % 1000)
        .trim_end_matches('0')
        .trim_end_matches('.'))
}
pub fn line_total(price: i64, qty: i64) -> Result<i64> {
    ensure!(
        price > 0 && price <= MAX_MONEY && qty > 0 && qty <= 1_000_000_000,
        "قیمت یا تعداد نامعتبر"
    );
    let n = (i128::from(price) * i128::from(qty) + 500) / 1000;
    ensure!(n <= i128::from(MAX_MONEY), "سرریز جمع ردیف");
    Ok(n as i64)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Product {
    pub code: String,
    pub name: String,
    pub measure: String,
    pub price_rial: i64,
    pub source_unit: Unit,
    pub source_at: String,
    pub aliases: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Catalog {
    pub id: String,
    pub name: String,
    pub imported_at: String,
    pub fingerprint: String,
    pub products: Vec<Product>,
}
/// Borrowed, disposable lookup index. It is never serialized or persisted.
/// Build once per quotation preview or native table render, then reuse it.
pub struct CatalogIndex<'a> {
    catalog: &'a Catalog,
    codes: HashMap<String, Vec<usize>>,
    aliases: HashMap<String, Vec<usize>>,
    raw_codes: HashMap<&'a str, usize>,
}
impl<'a> CatalogIndex<'a> {
    pub fn matches(&self, query: &str) -> Vec<&'a Product> {
        let key = normalize(query);
        self.codes
            .get(&key)
            .or_else(|| self.aliases.get(&key))
            .map(|indices| {
                indices
                    .iter()
                    .map(|index| &self.catalog.products[*index])
                    .collect()
            })
            .unwrap_or_default()
    }
    /// Human choices refer to the exact stored code; case/script normalization
    /// is only for request matching, never for silently changing a chosen code.
    pub fn product_by_code(&self, code: &str) -> Option<&'a Product> {
        self.raw_codes
            .get(code)
            .map(|index| &self.catalog.products[*index])
    }
}
impl Catalog {
    pub fn index(&self) -> CatalogIndex<'_> {
        let mut codes: HashMap<String, Vec<usize>> = HashMap::with_capacity(self.products.len());
        let mut aliases: HashMap<String, Vec<usize>> = HashMap::new();
        let mut raw_codes = HashMap::with_capacity(self.products.len());
        for (index, product) in self.products.iter().enumerate() {
            codes
                .entry(normalize(&product.code))
                .or_default()
                .push(index);
            raw_codes.entry(product.code.as_str()).or_insert(index);
            for alias in &product.aliases {
                let candidates = aliases.entry(normalize(alias)).or_default();
                // Product iteration is ordered, so this prevents repeated aliases
                // on one product from creating false ambiguity without sorting.
                if candidates.last() != Some(&index) {
                    candidates.push(index);
                }
            }
        }
        CatalogIndex {
            catalog: self,
            codes,
            aliases,
            raw_codes,
        }
    }
    pub fn matches(&self, query: &str) -> Vec<&Product> {
        let key = normalize(query);
        let exact = self
            .products
            .iter()
            .filter(|p| normalize(&p.code) == key)
            .collect::<Vec<_>>();
        if !exact.is_empty() {
            exact
        } else {
            self.products
                .iter()
                .filter(|p| p.aliases.iter().any(|a| normalize(a) == key))
                .collect()
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Customer {
    pub id: String,
    pub name: String,
    pub phone: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderLine {
    pub query: String,
    pub quantity_milli: i64,
    pub chosen_code: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Draft {
    pub series: String,
    pub catalog_id: String,
    pub customer: String,
    pub phone: String,
    pub lines: Vec<OrderLine>,
    pub discount_rial: i64,
    pub display_unit: Unit,
    pub stale_ack: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reference: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub quote_date: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub valid_until: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub issuer_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub issuer_phone: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub issuer_address: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub header: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub footer: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub terms: String,
    #[serde(default, skip_serializing_if = "zero_u32")]
    pub tax_basis_points: u32,
    #[serde(default, skip_serializing_if = "DisplayDigits::is_default")]
    pub display_digits: DisplayDigits,
    #[serde(default, skip_serializing_if = "DateDisplay::is_default")]
    pub date_display: DateDisplay,
}
impl Default for Draft {
    fn default() -> Self {
        Self {
            series: id(),
            catalog_id: String::new(),
            customer: String::new(),
            phone: String::new(),
            lines: vec![],
            discount_rial: 0,
            display_unit: Unit::Toman,
            stale_ack: false,
            reference: String::new(),
            quote_date: Utc::now().format("%Y-%m-%d").to_string(),
            valid_until: String::new(),
            notes: String::new(),
            issuer_name: String::new(),
            issuer_phone: String::new(),
            issuer_address: String::new(),
            header: String::new(),
            footer: String::new(),
            terms: String::new(),
            tax_basis_points: 0,
            display_digits: DisplayDigits::Persian,
            date_display: DateDisplay::Iso,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuoteLine {
    pub query: String,
    pub code: String,
    pub name: String,
    pub measure: String,
    pub quantity_milli: i64,
    pub price_rial: i64,
    pub total_rial: i64,
    pub source_at: String,
    pub source_unit: Unit,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Quote {
    pub id: String,
    pub series: String,
    pub version: u32,
    pub customer: String,
    pub phone: String,
    pub approved_at: String,
    pub catalog_id: String,
    pub catalog_name: String,
    pub lines: Vec<QuoteLine>,
    pub discount_rial: i64,
    pub total_rial: i64,
    pub display_unit: Unit,
    pub stale_ack: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reference: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub quote_date: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub valid_until: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub issuer_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub issuer_phone: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub issuer_address: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub header: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub footer: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub terms: String,
    #[serde(default, skip_serializing_if = "zero_u32")]
    pub tax_basis_points: u32,
    #[serde(default, skip_serializing_if = "DisplayDigits::is_default")]
    pub display_digits: DisplayDigits,
    #[serde(default, skip_serializing_if = "DateDisplay::is_default")]
    pub date_display: DateDisplay,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub number: String,
    #[serde(default, skip_serializing_if = "zero_i64")]
    pub tax_rial: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    pub schema: u32,
    pub catalogs: Vec<Catalog>,
    pub customers: Vec<Customer>,
    pub quotes: Vec<Quote>,
    pub draft: Draft,
    #[serde(default, skip_serializing_if = "Settings::is_default")]
    pub settings: Settings,
}
impl Default for State {
    fn default() -> Self {
        Self {
            schema: 1,
            catalogs: vec![],
            customers: vec![],
            quotes: vec![],
            draft: Draft::default(),
            settings: Settings::default(),
        }
    }
}
pub fn stale(source: &str) -> bool {
    stale_at(source, &now())
}
/// Historical approvals are checked at their approval time, never at restore time.
pub fn stale_at(source: &str, at: &str) -> bool {
    match (
        DateTime::parse_from_rfc3339(source),
        DateTime::parse_from_rfc3339(at),
    ) {
        (Ok(source), Ok(at)) => at.signed_duration_since(source).num_days() >= 7,
        _ => true,
    }
}
pub fn timestamp(s: &str) -> Result<()> {
    let t = DateTime::parse_from_rfc3339(s)?;
    ensure!(t <= Utc::now(), "زمان منبع در آینده است");
    Ok(())
}
impl State {
    pub fn new_draft(&mut self) -> Result<()> {
        self.settings.validate()?;
        let date = Utc::now().date_naive();
        let expiry = date
            .checked_add_days(chrono::Days::new(u64::from(
                self.settings.default_validity_days,
            )))
            .ok_or_else(|| anyhow::anyhow!("تاریخ اعتبار خارج از محدوده است"))?;
        self.draft = Draft {
            catalog_id: self.draft.catalog_id.clone(),
            display_unit: self.settings.default_currency,
            discount_rial: self.settings.default_discount_rial,
            quote_date: date.format("%Y-%m-%d").to_string(),
            valid_until: expiry.format("%Y-%m-%d").to_string(),
            issuer_name: self.settings.business_name.clone(),
            issuer_phone: self.settings.business_phone.clone(),
            issuer_address: self.settings.business_address.clone(),
            header: self.settings.header.clone(),
            footer: self.settings.footer.clone(),
            terms: self.settings.default_terms.clone(),
            tax_basis_points: self.settings.tax_basis_points,
            display_digits: self.settings.display_digits,
            date_display: self.settings.date_display,
            ..Draft::default()
        };
        Ok(())
    }
    pub fn save_customer(&mut self, name: &str, customer_phone: &str) -> Result<String> {
        text(name)?;
        phone(customer_phone)?;
        if let Some(customer) = self.customers.iter().find(|c| {
            normalize(&c.name) == normalize(name)
                && digits(c.phone.trim()) == digits(customer_phone.trim())
        }) {
            return Ok(customer.id.clone());
        }
        ensure!(
            self.customers.len() < 100_000,
            "تعداد مشتریان بیش از حد مجاز است"
        );
        let customer_id = id();
        self.customers.push(Customer {
            id: customer_id.clone(),
            name: name.trim().into(),
            phone: digits(customer_phone.trim()),
        });
        Ok(customer_id)
    }
    pub fn edit_customer(
        &mut self,
        customer_id: &str,
        name: &str,
        customer_phone: &str,
    ) -> Result<()> {
        text(name)?;
        phone(customer_phone)?;
        ensure!(
            !self.customers.iter().any(|c| c.id != customer_id
                && normalize(&c.name) == normalize(name)
                && digits(c.phone.trim()) == digits(customer_phone.trim())),
            "این مشتری قبلاً ثبت شده است"
        );
        let customer = self
            .customers
            .iter_mut()
            .find(|c| c.id == customer_id)
            .ok_or_else(|| anyhow::anyhow!("مشتری یافت نشد"))?;
        customer.name = name.trim().into();
        customer.phone = digits(customer_phone.trim());
        Ok(())
    }
    pub fn use_customer(&mut self, customer_id: &str) -> Result<()> {
        let customer = self
            .customers
            .iter()
            .find(|c| c.id == customer_id)
            .ok_or_else(|| anyhow::anyhow!("مشتری یافت نشد"))?;
        self.draft.customer = customer.name.clone();
        self.draft.phone = customer.phone.clone();
        Ok(())
    }
    pub fn select_catalog(&mut self, catalog_id: &str) -> Result<()> {
        ensure!(
            self.catalogs.iter().any(|c| c.id == catalog_id),
            "فهرست یافت نشد"
        );
        if self.draft.catalog_id != catalog_id {
            self.draft.catalog_id = catalog_id.into();
            for line in &mut self.draft.lines {
                line.chosen_code = None;
            }
            self.draft.stale_ack = false;
        }
        Ok(())
    }
    pub fn duplicate_quote(&mut self, quote_id: &str) -> Result<()> {
        self.revise(quote_id)?;
        self.draft.series = id();
        self.draft.quote_date = Utc::now().format("%Y-%m-%d").to_string();
        if document_date(&self.draft.valid_until)?
            .is_some_and(|expiry| expiry < Utc::now().date_naive())
        {
            self.draft.valid_until.clear();
        }
        self.draft.stale_ack = false;
        Ok(())
    }
    pub fn catalog(&self) -> Result<&Catalog> {
        self.catalogs
            .iter()
            .find(|c| c.id == self.draft.catalog_id)
            .ok_or_else(|| anyhow::anyhow!("فهرست قیمت تأییدشده انتخاب نشده"))
    }
    pub fn preview(&self) -> Result<Quote> {
        self.settings.validate()?;
        Uuid::parse_str(&self.draft.series)?;
        text(&self.draft.customer)?;
        phone(&self.draft.phone)?;
        metadata(
            &self.draft.reference,
            &self.draft.quote_date,
            &self.draft.valid_until,
            &self.draft.notes,
        )?;
        ensure!(
            !self.draft.lines.is_empty() && self.draft.lines.len() <= MAX_QUOTE_LINES,
            "سفارش باید بین ۱ تا ۱۰۰۰۰ ردیف داشته باشد"
        );
        let cat = self.catalog()?;
        let index = cat.index();
        let mut lines = vec![];
        let mut subtotal = 0i64;
        for l in &self.draft.lines {
            text(&l.query)?;
            let p = if let Some(code) = &l.chosen_code {
                index
                    .product_by_code(code)
                    .ok_or_else(|| anyhow::anyhow!("کد انتخاب‌شده موجود نیست"))?
            } else {
                let matches = index.matches(&l.query);
                ensure!(
                    matches.len() == 1,
                    "ردیف «{}» نیاز به بررسی انسانی دارد",
                    l.query
                );
                matches[0]
            };
            timestamp(&p.source_at)?;
            ensure!(
                !stale(&p.source_at) || self.draft.stale_ack,
                "قیمت قدیمی است؛ هشدار را تأیید کنید"
            );
            let total = line_total(p.price_rial, l.quantity_milli)?;
            subtotal = subtotal
                .checked_add(total)
                .filter(|n| *n <= MAX_MONEY)
                .ok_or_else(|| anyhow::anyhow!("سرریز جمع سفارش"))?;
            lines.push(QuoteLine {
                query: l.query.clone(),
                code: p.code.clone(),
                name: p.name.clone(),
                measure: p.measure.clone(),
                quantity_milli: l.quantity_milli,
                price_rial: p.price_rial,
                total_rial: total,
                source_at: p.source_at.clone(),
                source_unit: p.source_unit,
            });
        }
        ensure!(
            self.draft.discount_rial >= 0 && self.draft.discount_rial <= subtotal,
            "تخفیف خارج از محدوده است"
        );
        issuer_metadata(
            &self.draft.issuer_name,
            &self.draft.issuer_phone,
            &self.draft.issuer_address,
            &self.draft.header,
            &self.draft.footer,
            &self.draft.terms,
        )?;
        let net = subtotal - self.draft.discount_rial;
        let tax = tax_amount(net, self.draft.tax_basis_points)?;
        let total = net
            .checked_add(tax)
            .filter(|n| *n <= MAX_MONEY)
            .ok_or_else(|| anyhow::anyhow!("سرریز جمع نهایی با مالیات"))?;
        Ok(Quote {
            id: String::new(),
            series: self.draft.series.clone(),
            version: 0,
            customer: self.draft.customer.clone(),
            phone: self.draft.phone.clone(),
            approved_at: String::new(),
            catalog_id: cat.id.clone(),
            catalog_name: cat.name.clone(),
            lines,
            discount_rial: self.draft.discount_rial,
            total_rial: total,
            display_unit: self.draft.display_unit,
            stale_ack: self.draft.stale_ack,
            reference: self.draft.reference.clone(),
            quote_date: digits(&self.draft.quote_date),
            valid_until: digits(&self.draft.valid_until),
            notes: self.draft.notes.clone(),
            issuer_name: self.draft.issuer_name.clone(),
            issuer_phone: self.draft.issuer_phone.clone(),
            issuer_address: self.draft.issuer_address.clone(),
            header: self.draft.header.clone(),
            footer: self.draft.footer.clone(),
            terms: self.draft.terms.clone(),
            tax_basis_points: self.draft.tax_basis_points,
            display_digits: self.draft.display_digits,
            date_display: self.draft.date_display,
            number: self
                .quotes
                .iter()
                .find(|q| q.series == self.draft.series)
                .map(|q| q.number.clone())
                .unwrap_or_else(|| {
                    format!(
                        "{}{}",
                        self.settings.quote_prefix, self.settings.next_number
                    )
                }),
            tax_rial: tax,
        })
    }
    pub fn approve(&mut self) -> Result<String> {
        let mut q = self.preview()?;
        if let Some(previous) = self.quotes.iter().rev().find(|x| x.series == q.series) {
            let mut a = serde_json::to_value(previous)?;
            let mut b = serde_json::to_value(&q)?;
            for key in ["id", "version", "approved_at"] {
                a.as_object_mut().unwrap().remove(key);
                b.as_object_mut().unwrap().remove(key);
            }
            if a == b {
                return Ok(previous.id.clone());
            }
            q.version = previous
                .version
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("سرریز نسخه"))?;
        } else {
            ensure!(
                !self
                    .quotes
                    .iter()
                    .any(|x| x.series != q.series && normalize(&x.number) == normalize(&q.number)),
                "شمارهٔ پیشنهاد قبلاً استفاده شده است"
            );
            let next = self
                .settings
                .next_number
                .checked_add(1)
                .filter(|n| *n < u64::MAX)
                .ok_or_else(|| anyhow::anyhow!("سرریز شمارهٔ پیشنهاد"))?;
            self.settings.next_number = next;
            q.version = 1;
        }
        q.id = id();
        q.approved_at = now();
        let result = q.id.clone();
        if !self.customers.iter().any(|c| {
            normalize(&c.name) == normalize(&q.customer)
                && digits(c.phone.trim()) == digits(q.phone.trim())
        }) {
            self.customers.push(Customer {
                id: id(),
                name: q.customer.clone(),
                phone: q.phone.clone(),
            });
        }
        self.quotes.push(q);
        Ok(result)
    }
    pub fn revise(&mut self, quote_id: &str) -> Result<()> {
        let q = self
            .quotes
            .iter()
            .find(|q| q.id == quote_id)
            .ok_or_else(|| anyhow::anyhow!("پیشنهاد یافت نشد"))?;
        self.draft = Draft {
            series: q.series.clone(),
            catalog_id: q.catalog_id.clone(),
            customer: q.customer.clone(),
            phone: q.phone.clone(),
            lines: q
                .lines
                .iter()
                .map(|l| OrderLine {
                    query: l.query.clone(),
                    quantity_milli: l.quantity_milli,
                    chosen_code: Some(l.code.clone()),
                })
                .collect(),
            discount_rial: q.discount_rial,
            display_unit: q.display_unit,
            stale_ack: false,
            reference: q.reference.clone(),
            quote_date: q.quote_date.clone(),
            valid_until: q.valid_until.clone(),
            notes: q.notes.clone(),
            issuer_name: q.issuer_name.clone(),
            issuer_phone: q.issuer_phone.clone(),
            issuer_address: q.issuer_address.clone(),
            header: q.header.clone(),
            footer: q.footer.clone(),
            terms: q.terms.clone(),
            tax_basis_points: q.tax_basis_points,
            display_digits: q.display_digits,
            date_display: q.date_display,
        };
        Ok(())
    }
    pub fn validate(&self) -> Result<()> {
        self.settings.validate()?;
        ensure!(self.schema == 1, "نسخهٔ داده پشتیبانی نمی‌شود");
        ensure!(
            self.catalogs.len() <= 1000
                && self.customers.len() <= 100_000
                && self.quotes.len() <= 100_000,
            "داده بیش از حد بزرگ است"
        );
        let mut ids = HashSet::new();
        let mut fingerprints = HashSet::new();
        for c in &self.catalogs {
            ensure!(
                ids.insert(&c.id) && fingerprints.insert(&c.fingerprint),
                "فهرست تکراری"
            );
            Uuid::parse_str(&c.id)?;
            text(&c.name)?;
            timestamp(&c.imported_at)?;
            ensure!(
                !c.products.is_empty() && c.products.len() <= 50_000,
                "اندازه فهرست نامعتبر"
            );
            ensure!(
                c.fingerprint == crate::imports::product_fingerprint(&c.products)?,
                "اثر انگشت فهرست نامعتبر"
            );
            let mut codes = HashSet::new();
            for p in &c.products {
                text(&p.code)?;
                text(&p.name)?;
                text(&p.measure)?;
                timestamp(&p.source_at)?;
                ensure!(
                    p.aliases.len() <= 1000,
                    "حداکثر ۱۰۰۰ نام جایگزین برای هر کالا"
                );
                ensure!(
                    p.price_rial > 0
                        && p.price_rial <= MAX_MONEY
                        && codes.insert(normalize(&p.code)),
                    "کد تکراری یا مبلغ نامعتبر"
                );
                for a in &p.aliases {
                    text(a)?;
                }
            }
        }
        for c in &self.customers {
            Uuid::parse_str(&c.id)?;
            text(&c.name)?;
            phone(&c.phone)?;
            ensure!(ids.insert(&c.id), "شناسهٔ تکراری");
        }
        let mut versions = HashSet::new();
        let mut series_versions: HashMap<&str, u32> = HashMap::new();
        let mut series_approval: HashMap<&str, DateTime<chrono::FixedOffset>> = HashMap::new();
        let mut series_numbers: HashMap<&str, &str> = HashMap::new();
        let mut used_numbers: HashMap<String, &str> = HashMap::new();
        for q in &self.quotes {
            Uuid::parse_str(&q.id)?;
            Uuid::parse_str(&q.series)?;
            timestamp(&q.approved_at)?;
            metadata(&q.reference, &q.quote_date, &q.valid_until, &q.notes)?;
            issuer_metadata(
                &q.issuer_name,
                &q.issuer_phone,
                &q.issuer_address,
                &q.header,
                &q.footer,
                &q.terms,
            )?;
            ensure!(
                q.number.chars().count() <= 64 && !q.number.chars().any(forbidden_control),
                "شمارهٔ پیشنهاد نامعتبر است"
            );
            if let Some(previous) = series_numbers.insert(&q.series, &q.number) {
                ensure!(
                    previous == q.number,
                    "شمارهٔ پیشنهاد در اصلاحیه تغییر کرده است"
                );
            }
            if !q.number.is_empty()
                && let Some(previous_series) = used_numbers.insert(normalize(&q.number), &q.series)
            {
                ensure!(previous_series == q.series, "شمارهٔ پیشنهاد تکراری است");
            }
            ensure!(
                !q.lines.is_empty() && q.lines.len() <= MAX_QUOTE_LINES,
                "تعداد ردیف‌های پیشنهاد نامعتبر است"
            );
            ensure!(
                ids.insert(&q.id) && q.version > 0 && versions.insert((&q.series, q.version)),
                "نسخهٔ تکراری"
            );
            let previous = series_versions.get(q.series.as_str()).copied().unwrap_or(0);
            ensure!(
                previous.checked_add(1) == Some(q.version),
                "ترتیب نسخه‌ها یا زنجیرهٔ پیشنهاد نامعتبر است"
            );
            series_versions.insert(&q.series, q.version);
            let approved_at = DateTime::parse_from_rfc3339(&q.approved_at)?;
            if let Some(previous) = series_approval.insert(&q.series, approved_at) {
                ensure!(approved_at >= previous, "زمان تأیید نسخه‌ها ناسازگار است");
            }
            let catalog = self
                .catalogs
                .iter()
                .find(|c| c.id == q.catalog_id)
                .ok_or_else(|| anyhow::anyhow!("فهرست پیشنهاد موجود نیست"))?;
            let mut check = State {
                catalogs: vec![catalog.clone()],
                ..State::default()
            };
            check.draft = Draft {
                series: q.series.clone(),
                catalog_id: q.catalog_id.clone(),
                customer: q.customer.clone(),
                phone: q.phone.clone(),
                lines: q
                    .lines
                    .iter()
                    .map(|l| OrderLine {
                        query: l.query.clone(),
                        quantity_milli: l.quantity_milli,
                        chosen_code: Some(l.code.clone()),
                    })
                    .collect(),
                discount_rial: q.discount_rial,
                display_unit: q.display_unit,
                stale_ack: true,
                reference: q.reference.clone(),
                quote_date: q.quote_date.clone(),
                valid_until: q.valid_until.clone(),
                notes: q.notes.clone(),
                issuer_name: q.issuer_name.clone(),
                issuer_phone: q.issuer_phone.clone(),
                issuer_address: q.issuer_address.clone(),
                header: q.header.clone(),
                footer: q.footer.clone(),
                terms: q.terms.clone(),
                tax_basis_points: q.tax_basis_points,
                display_digits: q.display_digits,
                date_display: q.date_display,
            };
            let expected = check.preview()?;
            phone(&q.phone)?;
            for l in &q.lines {
                text(&l.query)?;
                ensure!(
                    DateTime::parse_from_rfc3339(&l.source_at)? <= approved_at,
                    "زمان منبع پس از تأیید پیشنهاد است"
                );
                ensure!(
                    !stale_at(&l.source_at, &q.approved_at) || q.stale_ack,
                    "تأیید هشدار قیمت قدیمی در پیشنهاد ثبت نشده است"
                );
            }
            ensure!(
                serde_json::to_value(&expected.lines)? == serde_json::to_value(&q.lines)?
                    && expected.total_rial == q.total_rial
                    && expected.tax_rial == q.tax_rial
                    && expected.catalog_name == q.catalog_name,
                "مبالغ یا منبع پیشنهاد ناسازگار است"
            );
        }
        Uuid::parse_str(&self.draft.series)?;
        issuer_metadata(
            &self.draft.issuer_name,
            &self.draft.issuer_phone,
            &self.draft.issuer_address,
            &self.draft.header,
            &self.draft.footer,
            &self.draft.terms,
        )?;
        ensure!(
            self.draft.tax_basis_points <= 10_000,
            "نرخ مالیات نامعتبر است"
        );
        metadata(
            &self.draft.reference,
            &self.draft.quote_date,
            &self.draft.valid_until,
            &self.draft.notes,
        )?;
        if !self.draft.catalog_id.is_empty() {
            self.catalog()?;
        }
        ensure!(
            self.draft.customer.len() <= 1024
                && !self.draft.customer.chars().any(forbidden_control),
            "مشتری نامعتبر"
        );
        phone(&self.draft.phone)?;
        ensure!(
            self.draft.lines.len() <= MAX_QUOTE_LINES
                && self.draft.discount_rial >= 0
                && self.draft.discount_rial <= MAX_MONEY,
            "پیش‌نویس نامعتبر"
        );
        for l in &self.draft.lines {
            text(&l.query)?;
            ensure!(
                l.quantity_milli > 0 && l.quantity_milli <= 1_000_000_000,
                "تعداد نامعتبر"
            );
            if let Some(code) = &l.chosen_code {
                ensure!(
                    self.catalog()?.products.iter().any(|p| &p.code == code),
                    "انتخاب نامعتبر"
                );
            }
        }
        Ok(())
    }
}
