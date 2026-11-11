//! Language, digits, money, quantity and date formatting.
//!
//! Message keys are the English strings. Persian comes from [`crate::fa_messages::FA`].
//! Amounts from the engine are integer strings in rials. Quantities are thousandths.

use std::collections::HashMap;
use std::sync::OnceLock;

use sarv_quote::calendar::{Calendar, format_date, from_gregorian};

use crate::fa_messages::FA;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    Fa,
    En,
}

impl Lang {
    pub fn is_rtl(self) -> bool {
        self == Lang::Fa
    }
    pub fn code(self) -> &'static str {
        match self {
            Lang::Fa => "fa",
            Lang::En => "en",
        }
    }
    pub fn other(self) -> Lang {
        match self {
            Lang::Fa => Lang::En,
            Lang::En => Lang::Fa,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Unit {
    #[default]
    Toman,
    Rial,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CalendarKind {
    #[default]
    Jalali,
    Gregorian,
}

/// How numbers and dates are shown, independent of the language (as in Settings).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fmt {
    pub lang: Lang,
    pub persian_digits: bool,
    pub unit: Unit,
    pub calendar: CalendarKind,
}

impl Default for Fmt {
    fn default() -> Self {
        Fmt { lang: Lang::Fa, persian_digits: true, unit: Unit::Toman, calendar: CalendarKind::Jalali }
    }
}

fn table() -> &'static HashMap<&'static str, &'static str> {
    static T: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    T.get_or_init(|| FA.iter().copied().collect())
}

/// Translate an English message key. Unknown keys fall back to the English text.
pub fn tr(lang: Lang, key: &str) -> String {
    match lang {
        Lang::En => key.to_string(),
        Lang::Fa => table().get(key).copied().unwrap_or(key).to_string(),
    }
}

/// Translate and fill `{name}` placeholders (values are isolated so they keep their own direction).
pub fn trf(lang: Lang, key: &str, values: &[(&str, &str)]) -> String {
    let mut out = tr(lang, key);
    for (name, value) in values {
        let v = if lang == Lang::Fa { isolate(value) } else { (*value).to_string() };
        out = out.replace(&format!("{{{name}}}"), &v);
    }
    out
}

/// First-strong isolate: keeps record names and references independent of surrounding RTL text.
pub fn isolate(value: &str) -> String {
    format!("\u{2068}{value}\u{2069}")
}

/// Latin digits to Persian digits (also `.` to the decimal separator and `,` to the thousands one).
pub fn persian_digits(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '0'..='9' => char::from_u32('۰' as u32 + (c as u32 - '0' as u32)).unwrap_or(c),
            _ => c,
        })
        .collect()
}

/// Persian or Arabic-Indic digits back to Latin, for parsing what the user typed.
pub fn latin_digits(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '۰'..='۹' => char::from_u32('0' as u32 + (c as u32 - '۰' as u32)).unwrap_or(c),
            '٠'..='٩' => char::from_u32('0' as u32 + (c as u32 - '٠' as u32)).unwrap_or(c),
            '٫' => '.',
            '٬' | '،' => ',',
            _ => c,
        })
        .collect()
}

impl Fmt {
    pub fn t(&self, key: &str) -> String {
        tr(self.lang, key)
    }
    pub fn tf(&self, key: &str, values: &[(&str, &str)]) -> String {
        trf(self.lang, key, values)
    }

    /// Localise the digits of an already formatted number.
    pub fn digits(&self, s: &str) -> String {
        if self.persian_digits { persian_digits(s) } else { s.to_string() }
    }

    fn group(&self, whole: &str) -> String {
        let sep = if self.persian_digits { '٬' } else { ',' };
        let chars: Vec<char> = whole.chars().collect();
        let mut out = String::new();
        for (i, c) in chars.iter().enumerate() {
            if i > 0 && (chars.len() - i) % 3 == 0 {
                out.push(sep);
            }
            out.push(*c);
        }
        out
    }

    /// An integer string of rials, shown in the chosen unit with grouping (no currency label).
    pub fn money(&self, rials: &str) -> String {
        let n: i128 = rials.trim().parse().unwrap_or(0);
        let neg = n < 0;
        let a = n.unsigned_abs();
        let body = match self.unit {
            Unit::Rial => self.group(&a.to_string()),
            Unit::Toman => {
                let whole = self.group(&(a / 10).to_string());
                let frac = a % 10;
                if frac == 0 {
                    whole
                } else {
                    let dot = if self.persian_digits { '٫' } else { '.' };
                    format!("{whole}{dot}{frac}")
                }
            }
        };
        let body = self.digits(&body);
        let shown = if neg { format!("−{body}") } else { body };
        format!("\u{2066}{shown}\u{2069}")
    }

    pub fn money_i64(&self, rials: i64) -> String {
        self.money(&rials.to_string())
    }

    pub fn unit_label(&self) -> String {
        match (self.unit, self.lang) {
            (Unit::Toman, Lang::Fa) => "تومان".into(),
            (Unit::Toman, Lang::En) => "toman".into(),
            (Unit::Rial, Lang::Fa) => "ریال".into(),
            (Unit::Rial, Lang::En) => "IRR".into(),
        }
    }

    pub fn money_with_unit(&self, rials: &str) -> String {
        format!("{} {}", self.money(rials), self.unit_label())
    }

    /// Quantity in thousandths, trimmed like `formatQuantity`.
    pub fn qty(&self, thousandths: &str) -> String {
        let n: i128 = thousandths.trim().parse().unwrap_or(0);
        let a = n.unsigned_abs();
        let frac = format!("{:03}", a % 1000);
        let frac = frac.trim_end_matches('0');
        let sep = if self.persian_digits { '٫' } else { '.' };
        let text = if frac.is_empty() { format!("{}", a / 1000) } else { format!("{}{sep}{frac}", a / 1000) };
        let text = self.digits(&text);
        if n < 0 { format!("−{text}") } else { text }
    }

    pub fn number(&self, n: impl std::fmt::Display) -> String {
        self.digits(&n.to_string())
    }

    /// An ISO `YYYY-MM-DD` date in the chosen calendar, with the chosen digits.
    pub fn date(&self, iso: &str) -> String {
        let day = iso.get(..10).unwrap_or(iso);
        let Ok(d) = chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d") else {
            return self.digits(iso);
        };
        let cal = match self.calendar {
            CalendarKind::Jalali => Calendar::Jalali,
            CalendarKind::Gregorian => Calendar::Gregorian,
        };
        let s = format_date(d, cal).unwrap_or_else(|_| day.to_string());
        self.digits(&s.replace('-', "/"))
    }

    /// Jalali (or Gregorian) year of today's date, for the fiscal period badge.
    pub fn year_of(&self, iso: &str) -> String {
        let day = iso.get(..10).unwrap_or(iso);
        let Ok(d) = chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d") else {
            return String::new();
        };
        match self.calendar {
            CalendarKind::Jalali => from_gregorian(d).map(|j| self.digits(&j.year().to_string())).unwrap_or_default(),
            CalendarKind::Gregorian => self.digits(&format!("{}", d.format("%Y"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_groups_and_converts_to_toman() {
        let f = Fmt { lang: Lang::En, persian_digits: false, unit: Unit::Toman, calendar: CalendarKind::Gregorian };
        let mark = |s: &str| format!("\u{2066}{s}\u{2069}");
        assert_eq!(f.money("1234567890"), mark("123,456,789"));
        assert_eq!(f.money("-15"), mark("−1.5"));
        let fa = Fmt::default();
        assert_eq!(fa.money("1000000"), mark("۱۰۰٬۰۰۰"));
    }

    #[test]
    fn quantity_is_trimmed() {
        let f = Fmt { lang: Lang::En, persian_digits: false, ..Fmt::default() };
        assert_eq!(f.qty("2500"), "2.5");
        assert_eq!(f.qty("3000"), "3");
    }

    #[test]
    fn translations_resolve_and_fall_back() {
        assert_eq!(tr(Lang::Fa, "Sales"), "فروش");
        assert_eq!(tr(Lang::Fa, "not a key"), "not a key");
        assert_eq!(latin_digits("۱۲٬۳۴۵٫۶"), "12,345.6");
    }

    #[test]
    fn jalali_dates() {
        let f = Fmt::default();
        assert_eq!(f.date("2025-03-21"), "۱۴۰۴/۰۱/۰۱");
    }
}
