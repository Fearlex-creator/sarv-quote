//! Sheba (Iranian IBAN): `IR` + 2 check digits + 3-digit bank code + 19-digit account.
use super::ids::{IdError, normalize_digits};

/// Bank codes seen in Sheba numbers. The table is data, not law: `verify` it
/// against the Central Bank list before relying on a name for anything but display.
pub const BANKS_VERIFY: &[(&str, &str)] = &[
    ("010", "بانک مرکزی"), ("011", "بانک صنعت و معدن"), ("012", "بانک ملت"),
    ("013", "بانک رفاه کارگران"), ("014", "بانک مسکن"), ("015", "بانک سپه"),
    ("016", "بانک کشاورزی"), ("017", "بانک ملی ایران"), ("018", "بانک تجارت"),
    ("019", "بانک صادرات ایران"), ("020", "بانک توسعه صادرات"), ("021", "پست بانک ایران"),
    ("022", "بانک توسعه تعاون"), ("053", "بانک کارآفرین"), ("054", "بانک پارسیان"),
    ("055", "بانک اقتصاد نوین"), ("056", "بانک سامان"), ("057", "بانک پاسارگاد"),
    ("058", "بانک سرمایه"), ("059", "بانک سینا"), ("060", "بانک مهر ایران"),
    ("061", "بانک شهر"), ("062", "بانک آینده"), ("063", "بانک انصار"),
    ("064", "بانک گردشگری"), ("065", "بانک حکمت ایرانیان"), ("066", "بانک دی"),
    ("069", "بانک ایران زمین"), ("070", "بانک رسالت"), ("073", "مؤسسه اعتباری کوثر"),
    ("075", "مؤسسه اعتباری ملل"), ("078", "بانک خاورمیانه"),
];

fn mod97(s: &str) -> u32 {
    s.chars().fold(0u32, |r, c| match c.to_digit(10) {
        Some(d) => (r * 10 + d) % 97,
        // Letters expand to two digits (A=10 … Z=35).
        None => (r * 100 + (c.to_ascii_uppercase() as u32 - 'A' as u32 + 10)) % 97,
    })
}

/// Canonical form: upper-case, no spaces, Latin digits.
pub fn canonical(input: &str) -> String {
    normalize_digits(input).to_ascii_uppercase()
}

pub fn validate(input: &str) -> Result<(), IdError> {
    let s = canonical(input);
    if s.chars().count() != 26 {
        return Err(IdError::Length);
    }
    let (head, tail) = s.split_at(2);
    if head != "IR" || !tail.chars().all(|c| c.is_ascii_digit()) {
        return Err(IdError::NotDigits);
    }
    let rearranged = format!("{}{}", &s[4..], &s[..4]);
    (mod97(&rearranged) == 1).then_some(()).ok_or(IdError::Checksum)
}

/// The two check digits for a 22-digit BBAN (bank code + account).
pub fn check_digits(bban: &str) -> Option<String> {
    let bban = normalize_digits(bban);
    (bban.len() == 22 && bban.chars().all(|c| c.is_ascii_digit()))
        .then(|| format!("{:02}", 98 - mod97(&format!("{bban}IR00"))))
}

/// Bank name for a valid Sheba, for display only.
pub fn bank_name(input: &str) -> Option<&'static str> {
    validate(input).ok()?;
    let s = canonical(input);
    BANKS_VERIFY.iter().find(|(code, _)| *code == &s[4..7]).map(|(_, name)| *name)
}
