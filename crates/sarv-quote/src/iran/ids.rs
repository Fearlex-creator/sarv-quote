//! Iranian identifier validation. These checks are advisory: stored contact data
//! stays free text because foreign parties have no Iranian identifiers.
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdError {
    Length,
    NotDigits,
    Repeated,
    Checksum,
}
impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            IdError::Length => "wrong length",
            IdError::NotDigits => "must contain digits only",
            IdError::Repeated => "all digits are identical",
            IdError::Checksum => "checksum does not match",
        })
    }
}
impl std::error::Error for IdError {}

/// Accepts Persian, Arabic-Indic and Latin digits and drops grouping noise
/// (spaces, hyphens, ZWNJ, thousands separators) so pasted values validate.
pub fn normalize_digits(input: &str) -> String {
    input
        .chars()
        .filter_map(|c| match c {
            '0'..='9' => Some(c),
            '\u{06F0}'..='\u{06F9}' => char::from_digit(c as u32 - 0x06F0, 10),
            '\u{0660}'..='\u{0669}' => char::from_digit(c as u32 - 0x0660, 10),
            ' ' | '-' | '\u{200C}' | '\u{200F}' | '\u{200E}' | ',' | '\u{066C}' | '\u{060C}' => None,
            other => Some(other),
        })
        .collect()
}

fn digits(input: &str, len: usize) -> Result<Vec<u32>, IdError> {
    let s = normalize_digits(input);
    if s.chars().count() != len {
        return Err(IdError::Length);
    }
    s.chars().map(|c| c.to_digit(10)).collect::<Option<Vec<_>>>().ok_or(IdError::NotDigits)
}

/// کد ملی: 10 digits, weighted mod-11 check digit.
pub fn national_id(input: &str) -> Result<(), IdError> {
    let d = digits(input, 10)?;
    if d.iter().all(|x| *x == d[0]) {
        return Err(IdError::Repeated);
    }
    let sum: u32 = d[..9].iter().enumerate().map(|(i, x)| x * (10 - i as u32)).sum();
    let r = sum % 11;
    let ok = if r < 2 { d[9] == r } else { d[9] == 11 - r };
    ok.then_some(()).ok_or(IdError::Checksum)
}

/// شناسه ملی (legal entity): 11 digits.
pub fn legal_id(input: &str) -> Result<(), IdError> {
    const COEF: [u32; 10] = [29, 27, 23, 19, 17, 29, 27, 23, 19, 17];
    let d = digits(input, 11)?;
    if d.iter().all(|x| *x == d[0]) {
        return Err(IdError::Repeated);
    }
    let c = d[9] + 2;
    let sum: u32 = d[..10].iter().zip(COEF).map(|(x, k)| (x + c) * k).sum();
    let mut r = sum % 11;
    if r == 10 {
        r = 0;
    }
    (d[10] == r).then_some(()).ok_or(IdError::Checksum)
}

/// Postal code: exactly 10 digits. Positional rules are not publicly specified,
/// so only the shape is checked.
pub fn postal_code(input: &str) -> Result<(), IdError> {
    digits(input, 10).map(|_| ())
}

/// Sayad cheque ID: 16 digits. No public checksum exists, so only the shape is checked.
pub fn sayad_id(input: &str) -> Result<(), IdError> {
    digits(input, 16).map(|_| ())
}
