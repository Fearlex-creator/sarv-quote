//! Language, digits, money, quantity and date formatting.
//!
//! Message keys are the English strings. Persian comes from [`crate::fa_messages::FA`].
//! Amounts from the engine are integer strings in rials. Quantities are thousandths.

use std::collections::HashMap;
use std::sync::OnceLock;

use sarv_quote::calendar::{Calendar, format_date, from_gregorian, parse_date as calendar_parse_date};

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
        Fmt {
            lang: Lang::Fa,
            persian_digits: true,
            unit: Unit::Toman,
            calendar: CalendarKind::Jalali,
        }
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

/// Convert ASCII digits `'0'..='9'` to Persian digits `'۰'..='۹'`, leaving all other characters unchanged.
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

/// Normalize Persian/Arabic text for case-, script-, diacritic-, and digit-insensitive matching.
pub fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in latin_digits(s).chars() {
        let mapped = match c {
            'ي' | 'ى' => 'ی',
            'ك' => 'ک',
            'ة' => 'ه',
            'أ' | 'إ' | 'آ' => 'ا',
            '\u{200c}' | '\u{640}' | '\u{64b}'..='\u{652}' => continue,
            other => other,
        };
        for lc in mapped.to_lowercase() {
            out.push(lc);
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoneyError {
    Empty,
    NotANumber,
    TooPrecise,
    TooLarge,
    Negative,
}

/// What the user typed, in the active display unit, as exact rials.
/// Accepts Persian/Arabic/Latin digits and grouping separators. In toman, one
/// decimal digit is allowed (0.5 toman = 5 rials). Anything else is an error.
pub fn parse_money(raw: &str, unit: Unit) -> Result<i64, MoneyError> {
    let norm = latin_digits(raw.trim()).replace([',', ' '], "");
    if norm.is_empty() {
        return Err(MoneyError::Empty);
    }
    if let Some(rest) = norm.strip_prefix('-') {
        if rest.is_empty() || !rest.chars().all(|c| c.is_ascii_digit() || c == '.') {
            return Err(MoneyError::NotANumber);
        }
        return Err(MoneyError::Negative);
    }
    if !norm.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return Err(MoneyError::NotANumber);
    }
    let mut parts = norm.split('.');
    let whole = parts.next().unwrap_or_default();
    let frac = parts.next();
    if parts.next().is_some() || whole.is_empty() {
        return Err(MoneyError::NotANumber);
    }
    let frac_val: i64 = match (unit, frac) {
        (_, None) => 0,
        (_, Some("")) => return Err(MoneyError::NotANumber),
        (Unit::Rial, Some(_)) => return Err(MoneyError::TooPrecise),
        (Unit::Toman, Some(f)) => {
            if f.len() > 1 {
                return Err(MoneyError::TooPrecise);
            }
            (f.as_bytes()[0] - b'0') as i64
        }
    };
    let whole_val: i64 = whole.parse().map_err(|_| MoneyError::TooLarge)?;
    let scale = match unit {
        Unit::Toman => 10,
        Unit::Rial => 1,
    };
    whole_val
        .checked_mul(scale)
        .and_then(|v| v.checked_add(frac_val))
        .ok_or(MoneyError::TooLarge)
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
            if i > 0 && (chars.len() - i).is_multiple_of(3) {
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

    /// Exact rial value formatted for an editable field: ungrouped, in active unit and digits,
    /// with one-digit toman fraction when `rials % 10 != 0` so `parse_money(&self.money_input(r), self.unit) == Ok(r)`.
    pub fn money_input(&self, rials: i64) -> String {
        let neg = rials < 0;
        let a = rials.unsigned_abs();
        let body = match self.unit {
            Unit::Rial => a.to_string(),
            Unit::Toman => {
                let whole = a / 10;
                let frac = a % 10;
                if frac == 0 {
                    whole.to_string()
                } else {
                    let dot = if self.persian_digits { '٫' } else { '.' };
                    format!("{whole}{dot}{frac}")
                }
            }
        };
        let body = self.digits(&body);
        if neg { format!("-{body}") } else { body }
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
        let text = if frac.is_empty() {
            format!("{}", a / 1000)
        } else {
            format!("{}{sep}{frac}", a / 1000)
        };
        let text = self.digits(&text);
        if n < 0 { format!("\u{2066}−{text}\u{2069}") } else { text }
    }

    /// Format a signed integer with locale thousands grouping, Unicode minus, and LTR bidi isolation.
    pub fn grouped_i64(&self, n: i64) -> String {
        let neg = n < 0;
        let grouped = self.group(&n.unsigned_abs().to_string());
        let body = self.digits(&grouped);
        let shown = if neg { format!("−{body}") } else { body };
        format!("\u{2066}{shown}\u{2069}")
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

    /// Parse a date typed in the active calendar (Jalali or Gregorian), accepting
    /// Persian/Arabic-Indic/Latin digits and `/`, `-`, or `.` separators.
    pub fn parse_date(&self, raw: &str) -> Result<String, String> {
        let norm = latin_digits(raw.trim()).replace('.', "/");
        let cal = match self.calendar {
            CalendarKind::Jalali => Calendar::Jalali,
            CalendarKind::Gregorian => Calendar::Gregorian,
        };
        calendar_parse_date(&norm, cal)
            .map(|d| d.format("%Y-%m-%d").to_string())
            .map_err(|_| self.t("Enter a valid date."))
    }

    /// Format an ISO `YYYY-MM-DD` date as the user would type it in the active calendar and digits.
    pub fn date_input(&self, iso: &str) -> String {
        self.date(iso)
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
        let f = Fmt {
            lang: Lang::En,
            persian_digits: false,
            unit: Unit::Toman,
            calendar: CalendarKind::Gregorian,
        };
        let mark = |s: &str| format!("\u{2066}{s}\u{2069}");
        assert_eq!(f.money("1234567890"), mark("123,456,789"));
        assert_eq!(f.money("-15"), mark("−1.5"));
        let fa = Fmt::default();
        assert_eq!(fa.money("1000000"), mark("۱۰۰٬۰۰۰"));
    }

    #[test]
    fn quantity_is_trimmed() {
        let f = Fmt {
            lang: Lang::En,
            persian_digits: false,
            ..Fmt::default()
        };
        assert_eq!(f.qty("2500"), "2.5");
        assert_eq!(f.qty("3000"), "3");
        assert_eq!(f.qty("-2500"), "\u{2066}−2.5\u{2069}");

        let fa = Fmt::default();
        assert_eq!(fa.qty("-1500"), "\u{2066}−۱٫۵\u{2069}");
    }

    #[test]
    fn grouped_i64_uses_locale_separator_and_bidi_isolation() {
        let en = Fmt {
            lang: Lang::En,
            persian_digits: false,
            ..Fmt::default()
        };
        assert_eq!(en.grouped_i64(0), "\u{2066}0\u{2069}");
        assert_eq!(en.grouped_i64(1234567), "\u{2066}1,234,567\u{2069}");
        assert_eq!(en.grouped_i64(-98765), "\u{2066}−98,765\u{2069}");

        let fa = Fmt::default();
        assert_eq!(fa.grouped_i64(1234567), "\u{2066}۱٬۲۳۴٬۵۶۷\u{2069}");
        assert_eq!(fa.grouped_i64(-98765), "\u{2066}−۹۸٬۷۶۵\u{2069}");
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

    #[test]
    fn parse_date_jalali_gregorian_and_round_trip() {
        let jalali = Fmt::default();
        let expected_iso = calendar_parse_date("1405/08/22", Calendar::Jalali)
            .unwrap()
            .format("%Y-%m-%d")
            .to_string();
        assert_eq!(jalali.parse_date("۱۴۰۵/۰۸/۲۲"), Ok(expected_iso.clone()));
        assert_eq!(jalali.parse_date("1405-08-22"), Ok(expected_iso.clone()));
        assert_eq!(jalali.parse_date("1405.08.22"), Ok(expected_iso.clone()));

        let greg = Fmt {
            lang: Lang::En,
            persian_digits: false,
            unit: Unit::Toman,
            calendar: CalendarKind::Gregorian,
        };
        assert_eq!(greg.parse_date("2026-11-13"), Ok("2026-11-13".to_string()));
        assert_eq!(greg.parse_date("2026/11/13"), Ok("2026-11-13".to_string()));

        for bad in ["", "not-a-date", "1405/13/01", "1405/08", "2026-02-30"] {
            assert!(jalali.parse_date(bad).is_err(), "expected Err for {bad:?}");
            assert!(greg.parse_date(bad).is_err(), "expected Err for {bad:?}");
        }

        for fmt in [jalali, greg] {
            for iso in ["2025-03-21", "2026-11-13", "2026-02-28"] {
                let typed = fmt.date_input(iso);
                assert_eq!(fmt.parse_date(&typed), Ok(iso.to_string()), "fmt={fmt:?} iso={iso}");
            }
        }
    }

    #[test]
    fn parse_money_table_and_round_trip() {
        let cases: &[(&str, Unit, Result<i64, MoneyError>)] = &[
            ("۱۲٬۳۴۵", Unit::Toman, Ok(123_450)),
            ("1234.5", Unit::Toman, Ok(12_345)),
            ("1234.56", Unit::Toman, Err(MoneyError::TooPrecise)),
            ("12.5", Unit::Rial, Err(MoneyError::TooPrecise)),
            ("100 تومان", Unit::Toman, Err(MoneyError::NotANumber)),
            ("", Unit::Toman, Err(MoneyError::Empty)),
            ("-5", Unit::Toman, Err(MoneyError::Negative)),
            ("1234567890123456789", Unit::Toman, Err(MoneyError::TooLarge)),
            ("9999999999999999999", Unit::Rial, Err(MoneyError::TooLarge)),
        ];
        for (raw, unit, expected) in cases {
            assert_eq!(parse_money(raw, *unit), *expected, "raw={raw:?} unit={unit:?}");
        }

        let configs = [
            Fmt::default(),
            Fmt {
                lang: Lang::En,
                persian_digits: false,
                unit: Unit::Toman,
                calendar: CalendarKind::Gregorian,
            },
            Fmt {
                lang: Lang::En,
                persian_digits: false,
                unit: Unit::Rial,
                calendar: CalendarKind::Gregorian,
            },
        ];
        for fmt in configs {
            for r in [12_345_i64, 12_340, 0] {
                let formatted = fmt.money_input(r);
                assert_eq!(
                    parse_money(&formatted, fmt.unit),
                    Ok(r),
                    "fmt={fmt:?} r={r} formatted={formatted:?}"
                );
            }
        }
    }

    #[test]
    fn every_ui_key_has_a_persian_entry() {
        use std::collections::HashSet;
        use std::path::Path;

        const ALLOW: &[&str] = &["English", "POS", "Sarv", "IRR"];
        let fa_keys: HashSet<&str> = FA.iter().map(|(k, _)| *k).collect();
        let allow: HashSet<&str> = ALLOW.iter().copied().collect();

        fn read_string_lit(s: &str) -> Option<(String, usize)> {
            let s = s.trim_start();
            if !s.starts_with('"') {
                return None;
            }
            let mut out = String::new();
            let mut chars = s[1..].char_indices();
            while let Some((i, c)) = chars.next() {
                match c {
                    '"' => return Some((out, i + 2)),
                    '\\' => {
                        let (_, esc) = chars.next()?;
                        match esc {
                            'n' => out.push('\n'),
                            'r' => out.push('\r'),
                            't' => out.push('\t'),
                            '\\' => out.push('\\'),
                            '"' => out.push('"'),
                            '\'' => out.push('\''),
                            other => out.push(other),
                        }
                    }
                    _ => out.push(c),
                }
            }
            None
        }

        fn skip_first_arg(s: &str) -> Option<&str> {
            let mut depth = 0usize;
            let mut in_str = false;
            let mut esc = false;
            for (i, c) in s.char_indices() {
                if in_str {
                    if esc {
                        esc = false;
                    } else if c == '\\' {
                        esc = true;
                    } else if c == '"' {
                        in_str = false;
                    }
                    continue;
                }
                match c {
                    '"' => in_str = true,
                    '(' | '[' | '{' => depth += 1,
                    ')' | ']' | '}' => {
                        if depth == 0 {
                            return None;
                        }
                        depth -= 1;
                    }
                    ',' if depth == 0 => return Some(&s[i + 1..]),
                    _ => {}
                }
            }
            None
        }

        fn collect_rs_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    if path.file_name().and_then(|n| n.to_str()) == Some("tests") {
                        continue;
                    }
                    collect_rs_files(&path, out);
                } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                    out.push(path);
                }
            }
        }

        let mut files = Vec::new();
        collect_rs_files(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src")), &mut files);
        files.sort();

        let mut missing = Vec::new();
        let mut check_key = |file: &Path, key: &str| {
            if !key.is_empty() && !allow.contains(key) && !fa_keys.contains(key) {
                missing.push(format!("{}: {key:?}", file.display()));
            }
        };

        for page in crate::app::Page::ALL {
            check_key(Path::new("app.rs"), page.label());
        }

        for file in &files {
            let src = std::fs::read_to_string(file).unwrap();
            for pat in [".t(\"", ".tf(\"", "ui.labeled(\"", "ui.hinted(\""] {
                let mut rest = src.as_str();
                while let Some(idx) = rest.find(pat) {
                    let after = &rest[idx + pat.len() - 1..];
                    if let Some((lit, used)) = read_string_lit(after) {
                        check_key(file, &lit);
                        rest = &after[used..];
                    } else {
                        rest = &rest[idx + pat.len()..];
                    }
                }
            }
            for pat in ["ui.btn(", "ui.btn_enabled(", "ui.wrap_btn(", "ui.compact_btn("] {
                let mut rest = src.as_str();
                while let Some(idx) = rest.find(pat) {
                    let after_paren = &rest[idx + pat.len()..];
                    if let Some(arg2) = skip_first_arg(after_paren)
                        && let Some((lit, _)) = read_string_lit(arg2)
                    {
                        check_key(file, &lit);
                    }
                    rest = after_paren;
                }
            }
            let mut rest = src.as_str();
            while let Some(idx) = rest.find("ui.empty(") {
                let after_paren = &rest[idx + "ui.empty(".len()..];
                if let Some(arg2) = skip_first_arg(after_paren) {
                    if let Some((lit, _)) = read_string_lit(arg2) {
                        check_key(file, &lit);
                    }
                    if let Some(arg3) = skip_first_arg(arg2)
                        && let Some((lit, _)) = read_string_lit(arg3)
                    {
                        check_key(file, &lit);
                    }
                }
                rest = after_paren;
            }
        }

        assert!(missing.is_empty(), "UI keys missing from FA:\n{}", missing.join("\n"));
    }

    #[test]
    fn fold_normalizes_arabic_persian_diacritics_zwnj_and_digits() {
        assert_eq!(fold("ياس"), fold("یاس"));
        assert_eq!(fold("ىاس"), fold("یاس"));
        assert_eq!(fold("كفش"), fold("کفش"));
        assert_eq!(fold("مدرسة"), fold("مدرسه"));
        assert_eq!(fold("أحمد إسلام آب"), fold("احمد اسلام اب"));
        assert_eq!(fold("می‌خواهم"), fold("میخواهم"));
        assert_eq!(fold("مُحَمَّدـی"), fold("محمدی"));
        assert_eq!(fold("۱۲۳۴AbC"), "1234abc");
    }
}
