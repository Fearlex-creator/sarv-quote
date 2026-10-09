use sarv_quote::{
    accounting::{Money, MoneyUnit, TrialBalance, TrialBalanceRow},
    accounting_export::{trial_csv, trial_pdf},
};
use std::process::Command;

#[test]
fn exact_report_units_pagination_and_no_overwrite_are_visible_in_exports() {
    let mut rows = vec![];
    for n in 0..60 {
        rows.push(TrialBalanceRow {
            account_id: format!("ID-{n}"),
            code: format!("TB-{n:03}"),
            name: format!("حساب نمونه {n} آمیخته TEST-{n:03}"),
            debit: Money::from_rials(if n % 2 == 0 { 1251 } else { 0 }),
            credit: Money::from_rials(if n % 2 == 1 { 1251 } else { 0 }),
        });
    }
    rows[0].code = "=DANGEROUS()".into();
    let balance = TrialBalance {
        company_id: "COMPANY-SYNTHETIC".into(),
        as_of: "2026-10-07".into(),
        rows,
        total_debit: Money::from_rials(37530),
        total_credit: Money::from_rials(37530),
    };
    let dir = tempfile::tempdir().unwrap();
    let csv = dir.path().join("trial.csv");
    let pdf = dir.path().join("trial.pdf");
    trial_csv(
        "شرکت آزمایشی TEST-COMPANY",
        &balance,
        MoneyUnit::Toman,
        &csv,
    )
    .unwrap();
    let bytes = std::fs::read(&csv).unwrap();
    let mut reader = csv::Reader::from_reader(&bytes[3..]);
    let headers = reader.headers().unwrap().clone();
    let first = reader.records().next().unwrap().unwrap();
    for (field, value) in [
        ("debit_irr", "1251"),
        ("display_debit", "125.1"),
        ("display_unit", "toman"),
        ("base_currency", "IRR"),
        ("account_code", "'=DANGEROUS()"),
        ("included_states", "posted_including_reversals"),
    ] {
        assert_eq!(
            &first[headers.iter().position(|h| h == field).unwrap()],
            value
        );
    }
    trial_pdf(
        "شرکت آزمایشی TEST-COMPANY",
        &balance,
        MoneyUnit::Toman,
        &pdf,
    )
    .unwrap();
    assert!(std::fs::read(&pdf).unwrap().starts_with(b"%PDF"));
    let output = Command::new("pdftotext")
        .arg(&pdf)
        .arg("-")
        .output()
        .expect("independent PDF text extractor required for this report check");
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("125.1") && text.contains("3753"));
    assert!(
        text.matches("TEST-COMPANY").count() >= 2,
        "company header repeats across report pages"
    );
    for n in 0..60 {
        assert!(
            text.contains(&format!("TEST-{n:03}")),
            "missing report account {n}"
        );
    }
    assert!(trial_pdf("شرکت آزمایشی", &balance, MoneyUnit::Rial, &pdf).is_err());
    assert!(trial_csv("شرکت آزمایشی", &balance, MoneyUnit::Rial, &csv).is_err());
    let mut bad = balance.clone();
    bad.total_debit = Money::from_rials(1);
    assert!(
        trial_pdf(
            "شرکت آزمایشی",
            &bad,
            MoneyUnit::Rial,
            &dir.path().join("invalid.pdf")
        )
        .is_err()
    );
    assert!(!dir.path().join("invalid.pdf").exists());
    if let Ok(destination) = std::env::var("SARV_ACCOUNTING_EXPORT_ARTIFACT_DIR") {
        std::fs::create_dir_all(&destination).unwrap();
        std::fs::copy(
            &pdf,
            std::path::Path::new(&destination).join("sample-trial-balance.pdf"),
        )
        .unwrap();
        std::fs::copy(
            &csv,
            std::path::Path::new(&destination).join("sample-trial-balance.csv"),
        )
        .unwrap();
    }
}
