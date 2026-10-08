//! Explicit civil Gregorian/Jalali date input and display (IR-02/IR-03).
//!
//! Canonical persistence remains a Gregorian date, never a local timestamp.
//! Jalali conversion supports 1200/01/01 through 1499/12/29 inclusive
//! (1821-03-21 through 2121-03-20 Gregorian). Gregorian input supports positive
//! four-digit years independently; dates outside the Jalali interval cannot be
//! displayed as Jalali. No timezone, astronomical or statutory claim is made.
//!
//! The year-start calculation independently expresses the break-year/leap-count
//! mathematics of K. M. Borkowski, "The Persian calendar for 3000 years",
//! Earth, Moon, and Planets 74 (1996), 223–230:
//! <https://doi.org/10.1007/BF00055188> (publisher abstract checked 2026-10-07),
//! <https://www.astro.uni.torun.pl/~kb/Papers/EMP/PersianC-EMP.htm>.
//! The author's server was unavailable during implementation (2026-10-07).
//! The algorithm and its range were also checked against the jalaali-js project
//! documentation: <https://github.com/jalaali/jalaali-js>. Its stated agreement
//! with Intl is Gregorian 1800–2256; this module deliberately uses a narrower
//! interval. Existing chrono provides all Gregorian arithmetic. No third-party
//! source is vendored and no dependency or repository license is changed.

use chrono::{Datelike, Days, NaiveDate};
use serde::{Deserialize, Serialize};
use std::fmt;

pub const MIN_JALALI_YEAR: i32 = 1200;
pub const MAX_JALALI_YEAR: i32 = 1499;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Calendar {
    #[default]
    Gregorian,
    Jalali,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CalendarError {
    InvalidFormat,
    GregorianYearOutOfRange {
        year: i32,
    },
    JalaliYearOutOfRange {
        year: i32,
    },
    InvalidMonth {
        calendar: Calendar,
        month: u32,
    },
    InvalidDay {
        calendar: Calendar,
        year: i32,
        month: u32,
        day: u32,
        maximum: u32,
    },
    GregorianOutsideJalaliRange {
        date: NaiveDate,
    },
}

impl fmt::Display for CalendarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat => write!(
                f,
                "تاریخ را با سال چهاررقمی و ماه و روز دورقمی وارد کنید: YYYY-MM-DD یا YYYY/MM/DD"
            ),
            Self::GregorianYearOutOfRange { year } => {
                write!(f, "سال میلادی {year} خارج از محدودهٔ 1 تا 9999 است")
            }
            Self::JalaliYearOutOfRange { year } => write!(
                f,
                "سال شمسی {year} خارج از محدودهٔ پشتیبانی‌شدهٔ {MIN_JALALI_YEAR} تا {MAX_JALALI_YEAR} است"
            ),
            Self::InvalidMonth { month, .. } => {
                write!(f, "ماه {month} نامعتبر است؛ ماه باید از 1 تا 12 باشد")
            }
            Self::InvalidDay {
                calendar,
                year,
                month,
                day,
                maximum,
            } => {
                let name = match calendar {
                    Calendar::Gregorian => "میلادی",
                    Calendar::Jalali => "شمسی",
                };
                write!(
                    f,
                    "روز {day} برای ماه {month} سال {year} {name} نامعتبر است؛ روز باید از 1 تا {maximum} باشد"
                )
            }
            Self::GregorianOutsideJalaliRange { date } => write!(
                f,
                "تاریخ میلادی {date} خارج از محدودهٔ تبدیل شمسی است: 1821-03-21 تا 2121-03-20"
            ),
        }
    }
}

impl std::error::Error for CalendarError {}

/// A validated Jalali civil date. Private fields preserve conversion invariants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct JalaliDate {
    year: i32,
    month: u32,
    day: u32,
}

impl JalaliDate {
    pub fn new(year: i32, month: u32, day: u32) -> Result<Self, CalendarError> {
        let maximum = jalali_month_length(year, month)?;
        if !(1..=maximum).contains(&day) {
            return Err(CalendarError::InvalidDay {
                calendar: Calendar::Jalali,
                year,
                month,
                day,
                maximum,
            });
        }
        Ok(Self { year, month, day })
    }

    pub fn year(self) -> i32 {
        self.year
    }
    pub fn month(self) -> u32 {
        self.month
    }
    pub fn day(self) -> u32 {
        self.day
    }
}

impl fmt::Display for JalaliDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}/{:02}/{:02}", self.year, self.month, self.day)
    }
}

fn validate_jalali_year(year: i32) -> Result<(), CalendarError> {
    if !(MIN_JALALI_YEAR..=MAX_JALALI_YEAR).contains(&year) {
        return Err(CalendarError::JalaliYearOutOfRange { year });
    }
    Ok(())
}

/// Borkowski's break years through the interval containing our entire range.
/// The last value bounds that interval; future break years are not needed here.
const BREAK_YEARS: [i32; 12] = [-61, 9, 38, 199, 426, 686, 756, 818, 1111, 1181, 1210, 1635];

/// Internal callers only use 1200..=1500. The extra year supplies the final
/// supported year's length without accepting that year as public input.
fn year_start(year: i32) -> NaiveDate {
    debug_assert!((MIN_JALALI_YEAR..=MAX_JALALI_YEAR + 1).contains(&year));
    let preceding: i32 = BREAK_YEARS
        .windows(2)
        .take_while(|pair| pair[1] <= year)
        .map(|pair| {
            let interval = pair[1] - pair[0];
            8 * (interval / 33) + (interval % 33) / 4
        })
        .sum();
    let pair = BREAK_YEARS
        .windows(2)
        .find(|pair| pair[0] <= year && year < pair[1])
        .expect("bounded year belongs to a break interval");
    let elapsed = year - pair[0];
    let interval = pair[1] - pair[0];
    let exceptional = i32::from(interval % 33 == 4 && interval - elapsed == 4);
    let persian_leaps =
        -14 + preceding + 8 * (elapsed / 33) + ((elapsed % 33) + 3) / 4 + exceptional;
    let gregorian_year = year + 621;
    let gregorian_leaps = gregorian_year / 4 - ((gregorian_year / 100 + 1) * 3) / 4 - 150;
    let march_day = 20 + persian_leaps - gregorian_leaps;
    NaiveDate::from_ymd_opt(gregorian_year, 3, march_day as u32)
        .expect("bounded year-start calculation produces a valid March date")
}

pub fn is_jalali_leap_year(year: i32) -> Result<bool, CalendarError> {
    validate_jalali_year(year)?;
    Ok((year_start(year + 1) - year_start(year)).num_days() == 366)
}

pub fn jalali_month_length(year: i32, month: u32) -> Result<u32, CalendarError> {
    validate_jalali_year(year)?;
    match month {
        1..=6 => Ok(31),
        7..=11 => Ok(30),
        12 => Ok(if is_jalali_leap_year(year)? { 30 } else { 29 }),
        _ => Err(CalendarError::InvalidMonth {
            calendar: Calendar::Jalali,
            month,
        }),
    }
}

pub fn to_gregorian(date: JalaliDate) -> NaiveDate {
    let prior_days = if date.month <= 6 {
        31 * (date.month - 1)
    } else {
        186 + 30 * (date.month - 7)
    };
    year_start(date.year)
        .checked_add_days(Days::new(u64::from(prior_days + date.day - 1)))
        .expect("validated bounded date arithmetic cannot overflow")
}

pub fn from_gregorian(date: NaiveDate) -> Result<JalaliDate, CalendarError> {
    if date < year_start(MIN_JALALI_YEAR) || date >= year_start(MAX_JALALI_YEAR + 1) {
        return Err(CalendarError::GregorianOutsideJalaliRange { date });
    }
    let candidate = (date.year() - 621).clamp(MIN_JALALI_YEAR, MAX_JALALI_YEAR);
    let year = if date < year_start(candidate) {
        candidate - 1
    } else {
        candidate
    };
    let ordinal = (date - year_start(year)).num_days() as u32;
    let (month, day) = if ordinal < 186 {
        (ordinal / 31 + 1, ordinal % 31 + 1)
    } else {
        ((ordinal - 186) / 30 + 7, (ordinal - 186) % 30 + 1)
    };
    JalaliDate::new(year, month, day)
}

/// An inclusive civil fiscal-year range; callers persist both dates as ISO.
/// This does not select the company's fiscal-year policy or timezone.
pub fn jalali_fiscal_year(year: i32) -> Result<(NaiveDate, NaiveDate), CalendarError> {
    validate_jalali_year(year)?;
    Ok((
        year_start(year),
        year_start(year + 1).pred_opt().expect("bounded date"),
    ))
}

/// Parse explicitly selected calendar input into the canonical Gregorian date.
/// Latin, Persian and Arabic-Indic digits may be mixed. Separators must be
/// uniformly '/' or '-'; no whitespace, shortened dates or timestamps accepted.
pub fn parse_date(input: &str, calendar: Calendar) -> Result<NaiveDate, CalendarError> {
    let mut normalized = [0u8; 10];
    let mut count = 0;
    for c in input.chars() {
        if count == normalized.len() {
            return Err(CalendarError::InvalidFormat);
        }
        normalized[count] = match c {
            '0'..='9' => c as u8,
            '۰'..='۹' => b'0' + (c as u32 - '۰' as u32) as u8,
            '٠'..='٩' => b'0' + (c as u32 - '٠' as u32) as u8,
            '-' | '/' => c as u8,
            _ => return Err(CalendarError::InvalidFormat),
        };
        count += 1;
    }
    if count != 10
        || !matches!(normalized[4], b'-' | b'/')
        || normalized[4] != normalized[7]
        || !normalized
            .iter()
            .enumerate()
            .all(|(i, digit)| matches!(i, 4 | 7) || digit.is_ascii_digit())
    {
        return Err(CalendarError::InvalidFormat);
    }
    let number = |slice: &[u8]| {
        slice
            .iter()
            .fold(0u32, |n, digit| n * 10 + u32::from(digit - b'0'))
    };
    let year = number(&normalized[..4]) as i32;
    let month = number(&normalized[5..7]);
    let day = number(&normalized[8..]);
    match calendar {
        Calendar::Jalali => Ok(to_gregorian(JalaliDate::new(year, month, day)?)),
        Calendar::Gregorian => {
            if !(1..=9999).contains(&year) {
                return Err(CalendarError::GregorianYearOutOfRange { year });
            }
            if !(1..=12).contains(&month) {
                return Err(CalendarError::InvalidMonth { calendar, month });
            }
            let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
            let maximum = match month {
                2 => {
                    if leap {
                        29
                    } else {
                        28
                    }
                }
                4 | 6 | 9 | 11 => 30,
                _ => 31,
            };
            NaiveDate::from_ymd_opt(year, month, day).ok_or(CalendarError::InvalidDay {
                calendar,
                year,
                month,
                day,
                maximum,
            })
        }
    }
}

/// Numeric Latin-digit output; the presentation layer may localize digits.
pub fn format_date(date: NaiveDate, calendar: Calendar) -> Result<String, CalendarError> {
    match calendar {
        Calendar::Jalali => Ok(from_gregorian(date)?.to_string()),
        Calendar::Gregorian => {
            if !(1..=9999).contains(&date.year()) {
                return Err(CalendarError::GregorianYearOutOfRange { year: date.year() });
            }
            Ok(date.format("%Y-%m-%d").to_string())
        }
    }
}
