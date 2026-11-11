use anyhow::{Result, bail, ensure};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use uuid::Uuid;
use crate::contact_profile::PartyProfile;

pub const CURRENT_SCHEMA: u32 = 7;
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
pub fn phone(s: &str) -> Result<()> {
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
            "تاریخ اعتبار باید برابر یا پس از تاریخ پیش‌فاکتور باشد"
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
    Jalali,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum AppMode {
    #[default]
    Personal,
    Business,
}
impl AppMode {
    pub fn is_default(&self) -> bool {
        *self == Self::Personal
    }
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum TextDirection {
    #[default]
    Rtl,
    Ltr,
}
impl TextDirection {
    pub fn is_default(&self) -> bool {
        *self == Self::Rtl
    }
}
fn is_one(value: &u32) -> bool {
    *value == 1
}
pub fn default_true() -> bool {
    true
}
fn is_true(value: &bool) -> bool {
    *value
}
fn default_fiscal_month() -> u32 {
    1
}
impl DateDisplay {
    pub fn is_default(&self) -> bool {
        *self == Self::Iso
    }
    /// Presentation only; stored document and posting dates remain Gregorian ISO.
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
        let shown = match self {
            Self::Jalali => {
                match crate::calendar::format_date(date, crate::calendar::Calendar::Jalali) {
                    Ok(shown) => shown,
                    Err(_) => return format!("{value} • میلادی؛ خارج از دامنهٔ شمسی"),
                }
            }
            Self::DayMonthYear => date.format("%d/%m/%Y").to_string(),
            Self::Iso => unreachable!(),
        };
        format!("{shown}{}", &value[10..])
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
    #[serde(default, skip_serializing_if = "AppMode::is_default")]
    pub app_mode: AppMode,
    #[serde(default = "default_fiscal_month", skip_serializing_if = "is_one")]
    pub fiscal_year_start_month: u32,
    #[serde(default, skip_serializing_if = "TextDirection::is_default")]
    pub text_direction: TextDirection,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub accent: String,
    /// Counter page. Off by default so the daily sidebar stays focused on invoices and cash.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub show_pos: bool,
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
            app_mode: AppMode::Personal,
            fiscal_year_start_month: 1,
            text_direction: TextDirection::Rtl,
            accent: String::new(),
            show_pos: false,
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
        ensure!(
            (1..=12).contains(&self.fiscal_year_start_month),
            "ماه شروع سال مالی باید بین ۱ و ۱۲ باشد"
        );
        ensure!(
            self.accent.chars().count() <= 32 && !self.accent.chars().any(forbidden_control),
            "رنگ تأکید نامعتبر است"
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
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Product {
    pub code: String,
    pub name: String,
    pub measure: String,
    pub price_rial: i64,
    pub source_unit: Unit,
    pub source_at: String,
    pub aliases: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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
    pub fn match_count(&self, query: &str) -> usize {
        let key = normalize(query);
        self.codes
            .get(&key)
            .or_else(|| self.aliases.get(&key))
            .map_or(0, Vec::len)
    }
    pub fn matches_limited(&self, query: &str, limit: usize) -> Vec<&'a Product> {
        let key = normalize(query);
        self.codes
            .get(&key)
            .or_else(|| self.aliases.get(&key))
            .map(|indices| {
                indices
                    .iter()
                    .take(limit)
                    .map(|index| &self.catalog.products[*index])
                    .collect()
            })
            .unwrap_or_default()
    }
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
    pub fn search_index(&self) -> SearchIndex {
        SearchIndex::new(self.products.iter().map(|p| {
            format!(
                "{} {} {} {}",
                p.code,
                p.name,
                p.measure,
                p.aliases.join(" ")
            )
        }))
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
/// Owned normalized browsing cache. This never changes exact quote matching.
#[derive(Clone, Debug)]
pub struct SearchIndex {
    documents: Vec<String>,
}
impl SearchIndex {
    pub fn new(documents: impl IntoIterator<Item = String>) -> Self {
        Self {
            documents: documents.into_iter().map(|s| normalize(&s)).collect(),
        }
    }
    pub fn search(&self, query: &str, limit: usize) -> Vec<usize> {
        self.search_page(query, 0, limit).0
    }
    pub fn search_page(&self, query: &str, offset: usize, limit: usize) -> (Vec<usize>, usize) {
        let query = normalize(query);
        let tokens: Vec<_> = query.split_whitespace().collect();
        let mut results = Vec::new();
        let mut total = 0;
        for (index, document) in self.documents.iter().enumerate() {
            if !document.is_empty() && tokens.iter().all(|token| document.contains(token)) {
                if total >= offset && results.len() < limit {
                    results.push(index);
                }
                total += 1;
            }
        }
        (results, total)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Customer {
    pub id: String,
    pub name: String,
    pub phone: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub contact_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub email: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub address: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub archived: bool,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub is_customer: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_supplier: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tax_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}
impl Default for Customer {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            phone: String::new(),
            contact_name: String::new(),
            email: String::new(),
            address: String::new(),
            archived: false,
            is_customer: true,
            is_supplier: false,
            tax_id: String::new(),
            notes: String::new(),
        }
    }
}
fn is_false(value: &bool) -> bool {
    !value
}
/// Optional customer details are snapshots, not inferred accounting identity.
pub fn customer_details(contact: &str, email: &str, address: &str) -> Result<()> {
    ensure!(
        contact.chars().count() <= 160 && !contact.chars().any(forbidden_control),
        "نام مخاطب حداکثر ۱۶۰ نویسه و بدون نویسهٔ کنترلی است"
    );
    ensure!(
        email.len() <= 254
            && !email
                .chars()
                .any(|c| c.is_whitespace() || forbidden_control(c)),
        "ایمیل نامعتبر است"
    );
    if !email.is_empty() {
        let parts: Vec<_> = email.split('@').collect();
        ensure!(
            parts.len() == 2
                && !parts[0].is_empty()
                && !parts[0].starts_with('.')
                && !parts[0].ends_with('.')
                && !parts[0].contains("..")
                && parts[1].contains('.')
                && parts[1].split('.').all(|label| !label.is_empty()
                    && !label.starts_with('-')
                    && !label.ends_with('-')
                    && label.chars().all(|c| c.is_alphanumeric() || c == '-')),
            "ایمیل باید نشانی کامل با @ و دامنه باشد"
        );
    }
    ensure!(
        address.chars().count() <= 2000
            && !address.chars().any(|c| c != '\n' && forbidden_control(c)),
        "نشانی حداکثر ۲۰۰۰ نویسه است؛ فقط شکست خط مجاز است"
    );
    Ok(())
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OrderLine {
    pub query: String,
    pub quantity_milli: i64,
    pub chosen_code: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub customer_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub customer_contact: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub customer_email: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub customer_address: String,
}
impl Default for Draft {
    fn default() -> Self {
        Self {
            series: id(),
            catalog_id: String::new(),
            customer: String::new(),
            phone: String::new(),
            customer_id: String::new(),
            customer_contact: String::new(),
            customer_email: String::new(),
            customer_address: String::new(),
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
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub customer_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub customer_contact: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub customer_email: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub customer_address: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct BusinessProfile {
    pub phone: String,
    pub address: String,
    pub registration_id: String,
    pub tax_id: String,
    pub invoice_prefix: String,
    pub terms: String,
    pub footer: String,
    pub bank_details: String,
}
impl BusinessProfile {
    pub fn validate(&self) -> Result<()> {
        for value in [&self.phone, &self.registration_id, &self.tax_id, &self.invoice_prefix] {
            ensure!(value.chars().count() <= 160 && !value.chars().any(char::is_control), "Business identifier is too long or contains control characters");
        }
        for value in [&self.address, &self.terms, &self.footer, &self.bank_details] {
            ensure!(value.chars().count() <= 2000, "Business text is too long");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct State {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub product_profiles: BTreeMap<String, crate::product_profile::ProductProfile>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub party_profiles: BTreeMap<String, PartyProfile>,
    pub schema: u32,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub business_profiles: BTreeMap<String, BusinessProfile>,
    #[serde(default, skip_serializing_if = "crate::desktop::DesktopPreferences::is_empty")]
    pub desktop: crate::desktop::DesktopPreferences,
    pub catalogs: Vec<Catalog>,
    pub customers: Vec<Customer>,
    pub quotes: Vec<Quote>,
    pub draft: Draft,
    #[serde(default, skip_serializing_if = "Settings::is_default")]
    pub settings: Settings,
    #[serde(
        default,
        skip_serializing_if = "crate::accounting::AccountingState::is_empty"
    )]
    pub accounting: crate::accounting::AccountingState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sales_drafts: Vec<crate::sales_draft::RawSalesDraft>,
    #[serde(
        default,
        skip_serializing_if = "crate::cashbook::Cashbook::is_empty"
    )]
    pub cashbook: crate::cashbook::Cashbook,
    #[serde(default, skip_serializing_if = "crate::cheques::ChequeRegister::is_empty")]
    pub cheques: crate::cheques::ChequeRegister,
}
impl Default for State {
    fn default() -> Self {
        Self {
            party_profiles: BTreeMap::new(),
            product_profiles: BTreeMap::new(),
            schema: CURRENT_SCHEMA,
            business_profiles: BTreeMap::new(),
            desktop: crate::desktop::DesktopPreferences::default(),
            catalogs: vec![],
            customers: vec![],
            quotes: vec![],
            draft: Draft::default(),
            settings: Settings::default(),
            accounting: crate::accounting::AccountingState::default(),
            sales_drafts: vec![],
            cashbook: crate::cashbook::Cashbook::default(),
            cheques: crate::cheques::ChequeRegister::default(),
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
    pub fn customer_search_index(&self) -> SearchIndex {
        SearchIndex::new(self.customers.iter().map(|c| {
            if c.archived {
                String::new()
            } else {
                format!(
                    "{} {} {} {} {}",
                    c.name, c.phone, c.contact_name, c.email, c.address
                )
            }
        }))
    }
    pub fn new_draft(&mut self) -> Result<()> {
        self.desktop.validate()?;
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
        self.save_customer_details(name, customer_phone, "", "", "")
    }
    pub fn save_customer_details(
        &mut self,
        name: &str,
        customer_phone: &str,
        contact: &str,
        email: &str,
        address: &str,
    ) -> Result<String> {
        text(name)?;
        phone(customer_phone)?;
        customer_details(contact, email, address)?;
        if let Some(customer) = self.customers.iter().find(|c| {
            normalize(&c.name) == normalize(name)
                && digits(c.phone.trim()) == digits(customer_phone.trim())
        }) {
            ensure!(
                (contact.trim().is_empty() || contact.trim() == customer.contact_name)
                    && (email.trim().is_empty() || email.trim() == customer.email)
                    && (address.trim().is_empty() || address.trim() == customer.address),
                "این مشتری ثبت شده است؛ جزئیات را از ویرایش مشتری تغییر دهید"
            );
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
            contact_name: contact.trim().into(),
            email: email.trim().into(),
            address: address.trim().into(),
            archived: false,
            ..Customer::default()
        });
        Ok(customer_id)
    }
    pub fn edit_customer(
        &mut self,
        customer_id: &str,
        name: &str,
        customer_phone: &str,
    ) -> Result<()> {
        let c = self
            .customers
            .iter()
            .find(|c| c.id == customer_id)
            .ok_or_else(|| anyhow::anyhow!("مشتری یافت نشد"))?;
        let (contact, email, address) =
            (c.contact_name.clone(), c.email.clone(), c.address.clone());
        self.edit_customer_details(
            customer_id,
            name,
            customer_phone,
            &contact,
            &email,
            &address,
        )
    }
    pub fn edit_customer_details(
        &mut self,
        customer_id: &str,
        name: &str,
        customer_phone: &str,
        contact: &str,
        email: &str,
        address: &str,
    ) -> Result<()> {
        text(name)?;
        phone(customer_phone)?;
        customer_details(contact, email, address)?;
        ensure!(
            !self.customers.iter().any(|c| c.id != customer_id
                && normalize(&c.name) == normalize(name)
                && digits(c.phone.trim()) == digits(customer_phone.trim())),
            "این مشتری قبلاً ثبت شده است"
        );
        let c = self
            .customers
            .iter_mut()
            .find(|c| c.id == customer_id)
            .ok_or_else(|| anyhow::anyhow!("مشتری یافت نشد"))?;
        c.name = name.trim().into();
        c.phone = digits(customer_phone.trim());
        c.contact_name = contact.trim().into();
        c.email = email.trim().into();
        c.address = address.trim().into();
        Ok(())
    }
    pub fn set_customer_archived(&mut self, customer_id: &str, archived: bool) -> Result<()> {
        let c = self
            .customers
            .iter_mut()
            .find(|c| c.id == customer_id)
            .ok_or_else(|| anyhow::anyhow!("مشتری یافت نشد"))?;
        c.archived = archived;
        Ok(())
    }
    pub fn use_customer(&mut self, customer_id: &str) -> Result<()> {
        self.select_customer(customer_id)
    }
    pub fn select_customer(&mut self, customer_id: &str) -> Result<()> {
        let c = self
            .customers
            .iter()
            .find(|c| c.id == customer_id)
            .ok_or_else(|| anyhow::anyhow!("مشتری یافت نشد"))?;
        ensure!(
            !c.archived,
            "مشتری بایگانی شده است؛ ابتدا از بایگانی خارج کنید"
        );
        self.draft.customer = c.name.clone();
        self.draft.phone = c.phone.clone();
        self.draft.customer_id = c.id.clone();
        self.draft.customer_contact = c.contact_name.clone();
        self.draft.customer_email = c.email.clone();
        self.draft.customer_address = c.address.clone();
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
        customer_details(
            &self.draft.customer_contact,
            &self.draft.customer_email,
            &self.draft.customer_address,
        )?;
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
        let customer_id = if !self.draft.customer_id.is_empty() {
            self.draft.customer_id.clone()
        } else if let Some(previous) = self.quotes.iter().rev().find(|q| {
            q.series == self.draft.series
                && normalize(&q.customer) == normalize(&self.draft.customer)
                && digits(q.phone.trim()) == digits(self.draft.phone.trim())
        }) {
            previous.customer_id.clone()
        } else {
            self.customers
                .iter()
                .find(|c| {
                    normalize(&c.name) == normalize(&self.draft.customer)
                        && digits(c.phone.trim()) == digits(self.draft.phone.trim())
                })
                .map(|c| c.id.clone())
                .unwrap_or_default()
        };
        Ok(Quote {
            id: String::new(),
            series: self.draft.series.clone(),
            version: 0,
            customer: self.draft.customer.clone(),
            phone: self.draft.phone.clone(),
            customer_id,
            customer_contact: self.draft.customer_contact.clone(),
            customer_email: self.draft.customer_email.clone(),
            customer_address: self.draft.customer_address.clone(),
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
                "شمارهٔ پیش‌فاکتور قبلاً استفاده شده است"
            );
            let next = self
                .settings
                .next_number
                .checked_add(1)
                .filter(|n| *n < u64::MAX)
                .ok_or_else(|| anyhow::anyhow!("سرریز شمارهٔ پیش‌فاکتور"))?;
            self.settings.next_number = next;
            q.version = 1;
        }
        q.id = id();
        q.approved_at = now();
        let result = q.id.clone();
        if !self.customers.iter().any(|c| {
            (!q.customer_id.is_empty() && c.id == q.customer_id)
                || (normalize(&c.name) == normalize(&q.customer)
                    && digits(c.phone.trim()) == digits(q.phone.trim()))
        }) {
            self.customers.push(Customer {
                id: id(),
                name: q.customer.clone(),
                phone: q.phone.clone(),
                contact_name: q.customer_contact.clone(),
                email: q.customer_email.clone(),
                address: q.customer_address.clone(),
                archived: false,
                ..Customer::default()
            });
        }
        if q.customer_id.is_empty() && q.version == 1 {
            q.customer_id = self
                .customers
                .iter()
                .find(|c| {
                    normalize(&c.name) == normalize(&q.customer)
                        && digits(c.phone.trim()) == digits(q.phone.trim())
                })
                .map(|c| c.id.clone())
                .unwrap_or_default();
        }
        self.quotes.push(q);
        Ok(result)
    }
    pub fn revise(&mut self, quote_id: &str) -> Result<()> {
        let q = self
            .quotes
            .iter()
            .find(|q| q.id == quote_id)
            .ok_or_else(|| anyhow::anyhow!("پیش‌فاکتور یافت نشد"))?;
        self.draft = Draft {
            series: q.series.clone(),
            catalog_id: q.catalog_id.clone(),
            customer: q.customer.clone(),
            phone: q.phone.clone(),
            customer_id: q.customer_id.clone(),
            customer_contact: q.customer_contact.clone(),
            customer_email: q.customer_email.clone(),
            customer_address: q.customer_address.clone(),
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
    /// Full validation of every invariant. Used on open, restore, import, backup and
    /// period close; `validate_changes` is the fast path for ordinary saves.
    pub fn validate(&self) -> Result<()> {
        self.validate_accounting_group()?;
        self.validate_approved_quotation_data()?;
        self.validate_draft()
    }
    /// Same verdict as `validate()` for a state that differs from an already fully
    /// validated `base`, doing only the work whose inputs changed. Each part is a pure
    /// function of the fields compared below (see ADR-0007):
    /// - accounting group: accounting, cheques, profiles, desktop, settings, schema, sales drafts
    ///   and quotes (sales orders link to approved quotes); below schema 3 it also reads
    ///   the draft, so it always runs there;
    /// - approved quotation data: catalogs, customers, quotes;
    /// - draft: draft, customers, catalogs.
    pub fn validate_changes(&self, base: &State) -> Result<()> {
        let accounting_changed = self.schema < 3
            || self.schema != base.schema
            || self.accounting != base.accounting
            || self.product_profiles != base.product_profiles
            || self.party_profiles != base.party_profiles
            || self.business_profiles != base.business_profiles
            || self.desktop != base.desktop
            || self.settings != base.settings
            || self.sales_drafts != base.sales_drafts
            || self.cheques != base.cheques
            || self.quotes != base.quotes;
        let quotation_changed = self.catalogs != base.catalogs
            || self.customers != base.customers
            || self.quotes != base.quotes;
        let draft_changed = self.draft != base.draft
            || self.customers != base.customers
            || self.catalogs != base.catalogs;
        if accounting_changed {
            self.validate_accounting_group()?;
        }
        if quotation_changed {
            self.validate_approved_quotation_data()?;
        }
        if draft_changed {
            self.validate_draft()?;
        }
        Ok(())
    }
    fn validate_accounting_group(&self) -> Result<()> {
        for (item_id, profile) in &self.product_profiles {
            profile.validate()?;
            let resources=self.accounting.procurement_resources(&profile.company_id)?;
            ensure!(resources.items.iter().any(|i| &i.id == item_id), "Product profile does not belong to this business");
            ensure!(profile.preferred_supplier_id.is_empty() || resources.parties.iter().any(|p| p.id == profile.preferred_supplier_id && p.supplier), "Preferred supplier does not belong to this business");
        }
        for (party_id, profile) in &self.party_profiles {
            profile.validate()?;
            ensure!(self.accounting.procurement_resources(&profile.company_id)?.parties.iter().any(|p| &p.id == party_id), "Contact profile does not belong to a registered party");
        }
        self.desktop.validate()?;
        ensure!(self.desktop.active_company.is_empty() || self.accounting.companies.iter().any(|c|c.id==self.desktop.active_company), "فضای کاری فعال نامعتبر است");
        self.settings.validate()?;
        for (company, profile) in &self.business_profiles {
            ensure!(self.accounting.companies.iter().any(|c| &c.id == company), "Business profile company does not exist");
            profile.validate()?;
        }
        ensure!(
            (1..=CURRENT_SCHEMA).contains(&self.schema),
            "نسخهٔ داده پشتیبانی نمی‌شود"
        );
        ensure!(
            self.schema >= 7
                || !self.accounting.procurement.records.iter().any(|record| {
                    matches!(
                        record.command,
                        crate::procurement::ProcurementCommand::Return(_)
                    ) || matches!(record.data, crate::procurement::ProcurementData::Return(_))
                }),
            "برگشت خرید در نسخهٔ قدیمی داده پشتیبانی نمی‌شود"
        );
        ensure!(
            self.schema >= 5 || self.accounting.procurement.is_empty(),
            "گردش خرید و موجودی نیازمند نسخهٔ دادهٔ ۵ است"
        );
        ensure!(
            self.schema >= 4 || self.accounting.core.is_empty(),
            "تنظیمات و افتتاحیهٔ کنترلی نیازمند نسخهٔ دادهٔ ۴ است"
        );
        ensure!(
            self.schema >= 2 || self.accounting.is_empty(),
            "دفتر حسابداری نیازمند نسخهٔ دادهٔ ۲ است"
        );
        ensure!(
            self.schema >= 3
                || (self.settings.date_display != DateDisplay::Jalali
                    && self.draft.date_display != DateDisplay::Jalali
                    && self
                        .quotes
                        .iter()
                        .all(|q| q.date_display != DateDisplay::Jalali)),
            "تاریخ شمسی نیازمند نسخهٔ دادهٔ ۳ است"
        );
        ensure!(
            self.schema >= 6 || (self.accounting.sales.is_empty() && self.sales_drafts.is_empty()),
            "گردش فروش نیازمند نسخهٔ دادهٔ ۶ است"
        );
        self.validate_sales_drafts_and_links()?;
        self.accounting.validate()?;
        self.validate_cheques()?;
        Ok(())
    }
    fn validate_approved_quotation_data(&self) -> Result<()> {
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
            customer_details(&c.contact_name, &c.email, &c.address)?;
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
            customer_details(&q.customer_contact, &q.customer_email, &q.customer_address)?;
            if !q.customer_id.is_empty() {
                Uuid::parse_str(&q.customer_id)?;
                ensure!(
                    self.customers.iter().any(|c| c.id == q.customer_id),
                    "مشتری مرتبط پیش‌فاکتور موجود نیست"
                );
            }
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
                "شمارهٔ پیش‌فاکتور نامعتبر است"
            );
            if let Some(previous) = series_numbers.insert(&q.series, &q.number) {
                ensure!(
                    previous == q.number,
                    "شمارهٔ پیش‌فاکتور در اصلاحیه تغییر کرده است"
                );
            }
            if !q.number.is_empty()
                && let Some(previous_series) = used_numbers.insert(normalize(&q.number), &q.series)
            {
                ensure!(previous_series == q.series, "شمارهٔ پیش‌فاکتور تکراری است");
            }
            ensure!(
                !q.lines.is_empty() && q.lines.len() <= MAX_QUOTE_LINES,
                "تعداد ردیف‌های پیش‌فاکتور نامعتبر است"
            );
            ensure!(
                ids.insert(&q.id) && q.version > 0 && versions.insert((&q.series, q.version)),
                "نسخهٔ تکراری"
            );
            let previous = series_versions.get(q.series.as_str()).copied().unwrap_or(0);
            ensure!(
                previous.checked_add(1) == Some(q.version),
                "ترتیب نسخه‌ها یا زنجیرهٔ پیش‌فاکتور نامعتبر است"
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
                .ok_or_else(|| anyhow::anyhow!("فهرست پیش‌فاکتور موجود نیست"))?;
            let mut check = State {
                catalogs: vec![catalog.clone()],
                ..State::default()
            };
            check.draft = Draft {
                series: q.series.clone(),
                catalog_id: q.catalog_id.clone(),
                customer: q.customer.clone(),
                phone: q.phone.clone(),
                customer_id: q.customer_id.clone(),
                customer_contact: q.customer_contact.clone(),
                customer_email: q.customer_email.clone(),
                customer_address: q.customer_address.clone(),
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
                    "زمان منبع پس از تأیید پیش‌فاکتور است"
                );
                ensure!(
                    !stale_at(&l.source_at, &q.approved_at) || q.stale_ack,
                    "تأیید هشدار قیمت قدیمی در پیش‌فاکتور ثبت نشده است"
                );
            }
            ensure!(
                serde_json::to_value(&expected.lines)? == serde_json::to_value(&q.lines)?
                    && expected.total_rial == q.total_rial
                    && expected.tax_rial == q.tax_rial
                    && expected.catalog_name == q.catalog_name,
                "مبالغ یا منبع پیش‌فاکتور ناسازگار است"
            );
        }
        Ok(())
    }
    fn validate_draft(&self) -> Result<()> {
        Uuid::parse_str(&self.draft.series)?;
        customer_details(
            &self.draft.customer_contact,
            &self.draft.customer_email,
            &self.draft.customer_address,
        )?;
        if !self.draft.customer_id.is_empty() {
            Uuid::parse_str(&self.draft.customer_id)?;
            ensure!(
                self.customers
                    .iter()
                    .any(|c| c.id == self.draft.customer_id),
                "مشتری مرتبط پیش‌نویس موجود نیست"
            );
        }
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
