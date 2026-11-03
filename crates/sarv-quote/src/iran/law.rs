//! Law as data: effective-dated values with an explicit `verify` flag. Nothing here
//! guesses outside the years it lists; an unknown year returns `None`.
use crate::calendar::from_gregorian;
use chrono::NaiveDate;

/// A legal value valid for Jalali years `from_year..=to_year`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LawValue<T> {
    pub value: T,
    pub from_year: i32,
    pub to_year: i32,
    /// `true` when the spec could only confirm this from secondary sources.
    pub verify: bool,
    pub note: &'static str,
}
impl<T: Copy> LawValue<T> {
    fn for_year(table: &[LawValue<T>], year: i32) -> Option<LawValue<T>> {
        table.iter().copied().find(|v| (v.from_year..=v.to_year).contains(&year))
    }
}
fn jalali_year(on: NaiveDate) -> Option<i32> {
    from_gregorian(on).ok().map(|d| d.year())
}

const VAT: &[LawValue<u32>] = &[LawValue {
    value: 1000,
    from_year: 1403,
    to_year: 1405,
    verify: false,
    note: "9% (VAT Law art. 7) + 1 point from annual budget laws; the 1405 bill proposed 12% but the final law kept 10%",
}];
/// General VAT rate in basis points on a given date (1000 = 10%).
pub fn standard_vat(on: NaiveDate) -> Option<LawValue<u32>> {
    LawValue::for_year(VAT, jalali_year(on)?)
}

/// Salary-tax bracket: marginal `rate_bp` applies to monthly income up to `up_to_rial`
/// (`None` = no upper bound). The first bracket has a 0% rate: the monthly exemption.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bracket {
    pub up_to_rial: Option<i64>,
    pub rate_bp: u32,
}
const T: i64 = 10; // 1 toman = 10 rial
const M: i64 = 1_000_000;
const SALARY_1405: [Bracket; 6] = [
    Bracket { up_to_rial: Some(40 * M * T), rate_bp: 0 },
    Bracket { up_to_rial: Some(80 * M * T), rate_bp: 1000 },
    Bracket { up_to_rial: Some(100 * M * T), rate_bp: 1500 },
    Bracket { up_to_rial: Some(120 * M * T), rate_bp: 2000 },
    Bracket { up_to_rial: Some(140 * M * T), rate_bp: 2500 },
    Bracket { up_to_rial: None, rate_bp: 3000 },
];
/// Monthly salary-tax brackets. `verify`: interpretation of the thresholds as cumulative
/// monthly income (not income after the exemption) must be confirmed against the 1405
/// budget law and circular.
pub fn salary_brackets(year: i32) -> Option<LawValue<&'static [Bracket]>> {
    (year == 1405).then_some(LawValue {
        value: &SALARY_1405[..],
        from_year: 1405,
        to_year: 1405,
        verify: true,
        note: "monthly exemption 40M toman; thresholds read as cumulative monthly income",
    })
}

/// Progressive monthly salary tax in whole rial (half-up).
pub fn salary_tax(year: i32, monthly_taxable_rial: i64) -> Option<i64> {
    let brackets = salary_brackets(year)?.value;
    let mut tax: i128 = 0;
    let mut lower: i64 = 0;
    for b in brackets {
        let upper = b.up_to_rial.unwrap_or(i64::MAX);
        if monthly_taxable_rial > lower {
            let slice = i128::from(monthly_taxable_rial.min(upper) - lower);
            tax += slice * i128::from(b.rate_bp);
        }
        lower = upper;
        if monthly_taxable_rial <= upper {
            break;
        }
    }
    Some(((tax + 5_000) / 10_000) as i64)
}

/// Social-insurance contribution split.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Insurance {
    pub employee_bp: u32,
    pub employer_bp: u32,
    pub unemployment_bp: u32,
}
/// Standard 7% / 20% / 3%. `verify`: special workshops, hard/harmful jobs and exemptions
/// differ, and the monthly ceiling/floor is a yearly figure that is not shipped here.
pub const INSURANCE_STANDARD_VERIFY: Insurance =
    Insurance { employee_bp: 700, employer_bp: 2000, unemployment_bp: 300 };

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InsuranceAmounts {
    pub base_rial: i64,
    pub employee_rial: i64,
    pub employer_rial: i64,
    pub unemployment_rial: i64,
}
fn share(base: i64, bp: u32) -> i64 {
    ((i128::from(base) * i128::from(bp) + 5_000) / 10_000) as i64
}
/// Contributions on `insurable_rial`, capped at `ceiling_rial` when one is known.
pub fn insurance(rate: Insurance, insurable_rial: i64, ceiling_rial: Option<i64>) -> InsuranceAmounts {
    let base = ceiling_rial.map_or(insurable_rial, |c| insurable_rial.min(c)).max(0);
    InsuranceAmounts {
        base_rial: base,
        employee_rial: share(base, rate.employee_bp),
        employer_rial: share(base, rate.employer_bp),
        unemployment_rial: share(base, rate.unemployment_bp),
    }
}

/// One employee, one month. The caller states the insurable and taxable bases
/// explicitly: which allowances are exempt is a yearly decision, not a code constant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PayrollInput {
    pub year: i32,
    pub gross_rial: i64,
    pub insurable_rial: i64,
    pub taxable_rial: i64,
    pub insurance_ceiling_rial: Option<i64>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PayrollResult {
    pub insurance: InsuranceAmounts,
    pub salary_tax_rial: i64,
    pub net_rial: i64,
    /// Employer cost on top of gross: employer + unemployment shares.
    pub employer_extra_rial: i64,
}
pub fn payroll(input: PayrollInput) -> Option<PayrollResult> {
    let ins = insurance(INSURANCE_STANDARD_VERIFY, input.insurable_rial, input.insurance_ceiling_rial);
    let tax = salary_tax(input.year, input.taxable_rial)?;
    Some(PayrollResult {
        insurance: ins,
        salary_tax_rial: tax,
        net_rial: input.gross_rial - ins.employee_rial - tax,
        employer_extra_rial: ins.employer_rial + ins.unemployment_rial,
    })
}
