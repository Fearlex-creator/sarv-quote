//! Money crosses IPC as strings and is stored as integer currency minor units.
use anyhow::{Result, bail, ensure};
use rust_decimal::{Decimal, prelude::ToPrimitive};
use std::str::FromStr;

pub const LIMIT: i64 = 9_000_000_000_000_000;
pub const CURRENCIES: &[(&str, u32)] = &[
    ("USD", 2),
    ("EUR", 2),
    ("GBP", 2),
    ("IRR", 0),
    ("JPY", 0),
    ("AED", 2),
    ("KWD", 3),
];

pub fn decimals(currency: &str) -> Result<u32> {
    CURRENCIES
        .iter()
        .find(|c| c.0 == currency)
        .map(|c| c.1)
        .ok_or_else(|| anyhow::anyhow!("Unsupported currency: {currency}"))
}

pub fn parse(text: &str, currency: &str) -> Result<i64> {
    let text: String = text
        .trim()
        .chars()
        .map(|c| match c {
            '۰'..='۹' => char::from_u32('0' as u32 + c as u32 - '۰' as u32).unwrap(),
            '٠'..='٩' => char::from_u32('0' as u32 + c as u32 - '٠' as u32).unwrap(),
            '٫' => '.',
            _ => c,
        })
        .collect();
    ensure!(!text.is_empty() && text.len() <= 40, "Enter a valid amount");
    // Group separators are intentionally rejected: 1,234 must never become 1.234.
    ensure!(
        text.chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == '-'),
        "Use digits and a decimal point, without grouping separators"
    );
    let value = Decimal::from_str(&text)?;
    let factor = Decimal::from(10_i64.pow(decimals(currency)?));
    let minor = value
        .checked_mul(factor)
        .ok_or_else(|| anyhow::anyhow!("Amount overflow"))?;
    ensure!(
        minor.fract().is_zero(),
        "{} allows {} decimal places",
        currency,
        decimals(currency)?
    );
    let result = minor
        .to_i64()
        .ok_or_else(|| anyhow::anyhow!("Amount overflow"))?;
    if !(-LIMIT..=LIMIT).contains(&result) {
        bail!("Amount is too large");
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_currency_precision() {
        assert_eq!(
            parse("90071992547409.91", "USD").unwrap_err().to_string(),
            "Amount is too large"
        );
        assert_eq!(parse("۱۲۳۴۵۶", "IRR").unwrap(), 123456);
        assert_eq!(parse("0.29", "USD").unwrap(), 29);
        assert_eq!(parse("1.234", "KWD").unwrap(), 1234);
        assert!(parse("1.1", "IRR").is_err());
        assert!(parse("1.001", "USD").is_err());
        assert!(parse("1,234", "USD").is_err());
    }
}
