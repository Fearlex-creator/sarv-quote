//! Moadian unique tax ID (شماره منحصربه‌فرد مالیاتی), 22 characters:
//! 6-char fiscal memory ID + 5 hex digits of days since 1970-01-01 + 10 hex digits of
//! serial + 1 Verhoeff check digit. `⚠ verify` the character-to-number mapping for the
//! check digit against the current official technical guide before production use.
use chrono::NaiveDate;

const D: [[u8; 10]; 10] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9], [1, 2, 3, 4, 0, 6, 7, 8, 9, 5],
    [2, 3, 4, 0, 1, 7, 8, 9, 5, 6], [3, 4, 0, 1, 2, 8, 9, 5, 6, 7],
    [4, 0, 1, 2, 3, 9, 5, 6, 7, 8], [5, 9, 8, 7, 6, 0, 4, 3, 2, 1],
    [6, 5, 9, 8, 7, 1, 0, 4, 3, 2], [7, 6, 5, 9, 8, 2, 1, 0, 4, 3],
    [8, 7, 6, 5, 9, 3, 2, 1, 0, 4], [9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
];
const P: [[u8; 10]; 8] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9], [1, 5, 7, 6, 2, 8, 3, 0, 9, 4],
    [5, 8, 0, 3, 7, 9, 6, 1, 4, 2], [8, 9, 1, 6, 0, 4, 3, 5, 2, 7],
    [9, 4, 5, 3, 1, 2, 6, 8, 7, 0], [4, 2, 8, 6, 5, 7, 3, 9, 0, 1],
    [2, 7, 9, 3, 8, 0, 6, 4, 1, 5], [7, 0, 4, 6, 9, 1, 3, 2, 5, 8],
];
const INV: [u8; 10] = [0, 4, 3, 2, 1, 5, 6, 7, 8, 9];

fn digit_values(s: &str) -> Option<Vec<u8>> {
    s.chars().map(|c| c.to_digit(10).map(|d| d as u8)).collect()
}

/// Verhoeff check digit for a string of decimal digits.
pub fn verhoeff_check_digit(number: &str) -> Option<u8> {
    let digits = digit_values(number)?;
    let c = digits.iter().rev().enumerate().fold(0u8, |c, (i, d)| {
        D[c as usize][P[(i + 1) % 8][*d as usize] as usize]
    });
    Some(INV[c as usize])
}

pub fn verhoeff_valid(number_with_check: &str) -> bool {
    digit_values(number_with_check).is_some_and(|digits| {
        digits
            .iter()
            .rev()
            .enumerate()
            .fold(0u8, |c, (i, d)| D[c as usize][P[i % 8][*d as usize] as usize])
            == 0
    })
}

/// Digits stay as they are; letters become their ASCII code (A = 65 …).
fn to_numeric(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_digit() { c.to_string() } else { (c.to_ascii_uppercase() as u32).to_string() })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaxIdError {
    MemoryId,
    Date,
    Serial,
}

pub fn generate(memory_id: &str, issued: NaiveDate, serial: u64) -> Result<String, TaxIdError> {
    if memory_id.chars().count() != 6 || !memory_id.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(TaxIdError::MemoryId);
    }
    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).ok_or(TaxIdError::Date)?;
    let days = u64::try_from((issued - epoch).num_days()).map_err(|_| TaxIdError::Date)?;
    if days > 0xF_FFFF {
        return Err(TaxIdError::Date);
    }
    if serial > 0xFF_FFFF_FFFF {
        return Err(TaxIdError::Serial);
    }
    let body = format!("{}{days:05X}{serial:010X}", memory_id.to_ascii_uppercase());
    let check = verhoeff_check_digit(&to_numeric(&body)).ok_or(TaxIdError::MemoryId)?;
    Ok(format!("{body}{check}"))
}

pub fn validate(tax_id: &str) -> bool {
    tax_id.chars().count() == 22
        && tax_id.chars().all(|c| c.is_ascii_alphanumeric())
        && tax_id.is_char_boundary(21)
        && verhoeff_check_digit(&to_numeric(&tax_id[..21]))
            .is_some_and(|d| tax_id[21..].chars().next().and_then(|c| c.to_digit(10)) == Some(u32::from(d)))
}
