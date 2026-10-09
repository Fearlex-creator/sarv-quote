use chrono::{Days, NaiveDate};
use sarv_quote::calendar::{
    Calendar, CalendarError, JalaliDate, MAX_JALALI_YEAR, MIN_JALALI_YEAR, format_date,
    from_gregorian, is_jalali_leap_year, jalali_fiscal_year, jalali_month_length, parse_date,
    to_gregorian,
};

fn gregorian(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

#[test]
fn known_nowruz_and_independent_esfand_fixtures() {
    // Independent fixtures checked with Node 24.19.0 / ICU 78.3's Persian
    // calendar, using UTC noon, not the conversion implementation under test.
    // 1395/01/23 is also published by the jalaali-js project's README.
    let fixtures = [
        ((1390, 1, 1), (2011, 3, 21)),
        ((1391, 1, 1), (2012, 3, 20)),
        ((1395, 1, 1), (2016, 3, 20)),
        ((1395, 1, 23), (2016, 4, 11)),
        ((1395, 12, 30), (2017, 3, 20)),
        ((1399, 1, 1), (2020, 3, 20)),
        ((1399, 12, 30), (2021, 3, 20)),
        ((1400, 1, 1), (2021, 3, 21)),
        ((1400, 12, 29), (2022, 3, 20)),
        ((1403, 1, 1), (2024, 3, 20)),
        ((1403, 12, 30), (2025, 3, 20)),
        ((1404, 1, 1), (2025, 3, 21)),
        ((1405, 1, 1), (2026, 3, 21)),
    ];
    for ((jy, jm, jd), (gy, gm, gd)) in fixtures {
        let jalali = JalaliDate::new(jy, jm, jd).unwrap();
        let iso = gregorian(gy, gm, gd);
        assert_eq!(to_gregorian(jalali), iso, "Jalali fixture {jalali}");
        assert_eq!(from_gregorian(iso).unwrap(), jalali);
    }
}

#[test]
fn range_endpoints_and_outside_dates_are_explicit() {
    let first = JalaliDate::new(MIN_JALALI_YEAR, 1, 1).unwrap();
    let last = JalaliDate::new(MAX_JALALI_YEAR, 12, 29).unwrap();
    assert_eq!(to_gregorian(first), gregorian(1821, 3, 21));
    assert_eq!(to_gregorian(last), gregorian(2121, 3, 20));
    for date in [
        gregorian(1821, 3, 20),
        gregorian(2121, 3, 21),
        NaiveDate::MIN,
        NaiveDate::MAX,
    ] {
        assert_eq!(
            from_gregorian(date),
            Err(CalendarError::GregorianOutsideJalaliRange { date })
        );
    }
    for year in [i32::MIN, 0, 1199, 1500, i32::MAX] {
        assert_eq!(
            JalaliDate::new(year, 1, 1),
            Err(CalendarError::JalaliYearOutOfRange { year })
        );
        assert!(jalali_fiscal_year(year).is_err());
        assert!(is_jalali_leap_year(year).is_err());
    }
}

#[test]
fn jalali_leap_and_every_month_validate_without_rollover() {
    for year in [1391, 1395, 1399, 1403] {
        assert!(is_jalali_leap_year(year).unwrap());
        assert_eq!(jalali_month_length(year, 12).unwrap(), 30);
        assert!(JalaliDate::new(year, 12, 30).is_ok());
    }
    for year in [1394, 1398, 1400, 1402, 1404] {
        assert!(!is_jalali_leap_year(year).unwrap());
        assert_eq!(jalali_month_length(year, 12).unwrap(), 29);
        assert!(matches!(
            JalaliDate::new(year, 12, 30),
            Err(CalendarError::InvalidDay { maximum: 29, .. })
        ));
    }
    for month in 1..=12 {
        let maximum = if month <= 6 {
            31
        } else if month <= 11 {
            30
        } else {
            29
        };
        assert_eq!(jalali_month_length(1404, month).unwrap(), maximum);
        assert!(JalaliDate::new(1404, month, maximum).is_ok());
        assert!(JalaliDate::new(1404, month, maximum + 1).is_err());
        assert!(JalaliDate::new(1404, month, 0).is_err());
    }
    for month in [0, 13, u32::MAX] {
        assert!(JalaliDate::new(1404, month, 1).is_err());
    }
    assert!(JalaliDate::new(1404, 1, u32::MAX).is_err());
}

#[test]
fn explicit_input_calendar_never_guesses_the_year() {
    assert_eq!(
        parse_date("1404-01-01", Calendar::Gregorian).unwrap(),
        gregorian(1404, 1, 1)
    );
    assert_eq!(
        parse_date("1404-01-01", Calendar::Jalali).unwrap(),
        gregorian(2025, 3, 21)
    );
    assert!(parse_date("2025-03-21", Calendar::Jalali).is_err());
    assert_eq!(
        parse_date("0001-01-01", Calendar::Gregorian).unwrap(),
        gregorian(1, 1, 1)
    );
    assert_eq!(
        parse_date("9999/12/31", Calendar::Gregorian).unwrap(),
        gregorian(9999, 12, 31)
    );
    assert!(parse_date("0000-01-01", Calendar::Gregorian).is_err());
}

#[test]
fn persian_arabic_and_mixed_digits_produce_identical_canonical_dates() {
    for input in [
        "1404/01/01",
        "۱۴۰۴/۰۱/۰۱",
        "١٤٠٤/٠١/٠١",
        "۱4٠۴/0۱/٠1",
        "۱۴۰۴-۰۱-۰۱",
    ] {
        let date = parse_date(input, Calendar::Jalali).unwrap();
        assert_eq!(
            format_date(date, Calendar::Gregorian).unwrap(),
            "2025-03-21"
        );
        assert_eq!(format_date(date, Calendar::Jalali).unwrap(), "1404/01/01");
    }
    assert_eq!(
        parse_date("۲۰۲۴-۰۲-۲۹", Calendar::Gregorian).unwrap(),
        gregorian(2024, 2, 29)
    );
}

#[test]
fn malformed_inputs_and_gregorian_leap_centuries_reject_cleanly() {
    for input in [
        "",
        "1404/1/1",
        "1404/01-01",
        "1404.01.01",
        " 1404/01/01",
        "1404/01/01 ",
        "1404/01/01T12:00:00Z",
        "1404/01/01\n",
        "\u{2066}1404/01/01\u{2069}",
        "+1404/01/01",
        "۱۴۰۴/۰۱/۰x",
        "14040/01/01",
        "1404//1/01",
    ] {
        assert_eq!(
            parse_date(input, Calendar::Jalali),
            Err(CalendarError::InvalidFormat),
            "{input:?}"
        );
    }
    for input in [
        "1900-02-29",
        "2100-02-29",
        "2025-02-29",
        "2025-04-31",
        "2025-00-01",
        "2025-01-00",
    ] {
        assert!(parse_date(input, Calendar::Gregorian).is_err(), "{input}");
    }
    assert_eq!(
        parse_date("2000-02-29", Calendar::Gregorian).unwrap(),
        gregorian(2000, 2, 29)
    );
    assert!(format_date(NaiveDate::MIN, Calendar::Gregorian).is_err());
    assert!(format_date(NaiveDate::MAX, Calendar::Gregorian).is_err());
}

#[test]
fn fiscal_boundaries_are_inclusive_and_preserve_chronology() {
    assert_eq!(
        jalali_fiscal_year(1403).unwrap(),
        (gregorian(2024, 3, 20), gregorian(2025, 3, 20))
    );
    assert_eq!(
        jalali_fiscal_year(1404).unwrap(),
        (gregorian(2025, 3, 21), gregorian(2026, 3, 20))
    );
    for year in MIN_JALALI_YEAR..=MAX_JALALI_YEAR {
        let (start, end) = jalali_fiscal_year(year).unwrap();
        assert_eq!(
            (end - start).num_days() + 1,
            if is_jalali_leap_year(year).unwrap() {
                366
            } else {
                365
            }
        );
        assert_eq!(
            from_gregorian(start).unwrap(),
            JalaliDate::new(year, 1, 1).unwrap()
        );
        assert_eq!(
            from_gregorian(end).unwrap(),
            JalaliDate::new(year, 12, jalali_month_length(year, 12).unwrap()).unwrap()
        );
        if year < MAX_JALALI_YEAR {
            assert_eq!(
                end.succ_opt().unwrap(),
                jalali_fiscal_year(year + 1).unwrap().0
            );
        }
    }
}

#[test]
fn all_supported_days_roundtrip_and_calendar_switch_keeps_canonical_date() {
    let start = gregorian(1821, 3, 21);
    let end = gregorian(2121, 3, 20);
    let mut date = start;
    let mut count = 0u64;
    while date <= end {
        let jalali = from_gregorian(date).unwrap();
        assert_eq!(to_gregorian(jalali), date);
        assert_eq!(
            parse_date(&jalali.to_string(), Calendar::Jalali).unwrap(),
            date
        );
        assert_eq!(
            parse_date(
                &format_date(date, Calendar::Gregorian).unwrap(),
                Calendar::Gregorian
            )
            .unwrap(),
            date
        );
        count += 1;
        date = date.checked_add_days(Days::new(1)).unwrap();
    }
    assert_eq!(count, 109_573);
}

#[test]
fn presentation_mode_serializes_explicitly_and_errors_explain_limits() {
    assert_eq!(Calendar::default(), Calendar::Gregorian);
    assert_eq!(
        serde_json::to_string(&Calendar::Jalali).unwrap(),
        "\"Jalali\""
    );
    assert_eq!(
        serde_json::from_str::<Calendar>("\"Gregorian\"").unwrap(),
        Calendar::Gregorian
    );
    assert!(serde_json::from_str::<Calendar>("\"automatic\"").is_err());
    let error = JalaliDate::new(1404, 12, 30).unwrap_err().to_string();
    assert!(error.contains("29") && error.contains("1404") && error.contains("شمسی"));
    let error = from_gregorian(gregorian(2121, 3, 21))
        .unwrap_err()
        .to_string();
    assert!(error.contains("1821-03-21") && error.contains("2121-03-20"));
}
