use sarv_quote::{
    accounting::{AccountingState, MoneyUnit},
    core::*,
    core_export::*,
    core_import::opening_csv as import_opening_csv,
    domain::{DateDisplay, DisplayDigits, State},
    store::{Store, backup_state},
};
use std::{fs, process::Command};
fn fixture() -> (AccountingState, String) {
    let mut state = AccountingState::default();
    let setup = state
        .setup_core_company(NewCoreCompany {
            name: "شرکت مصنوعی TEST-OPENING".into(),
            fiscal_name: "سال ۱۴۰۵".into(),
            start_date: "2026-03-21".into(),
            end_date: "2027-03-20".into(),
            timezone: CompanyTimezone::AsiaTehran,
            chart: default_chart(),
            actor: "آزمایشگر".into(),
        })
        .unwrap();
    let input = include_str!("../docs/assets/accounting-m3/opening.csv")
        .replace("F2-core-v1", &setup.company_id);
    let mut command = import_opening_csv(
        &state,
        &setup.company_id,
        &input,
        "آزمایشگر",
        "export-fixture",
    )
    .unwrap();
    command.source_id = "=DANGEROUS()".into();
    command.cash[0].name = "=CASH-FORMULA()".into();
    state.post_opening(command).unwrap();
    (state, setup.company_id)
}
#[test]
fn exact_csv_controls_details_snapshots_and_formula_guards() {
    let (state, co) = fixture();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("report.csv");
    opening_csv(&state, &co, MoneyUnit::Toman, DateDisplay::Jalali, &path).unwrap();
    let bytes = fs::read(&path).unwrap();
    assert_eq!(&bytes[..3], &[0xef, 0xbb, 0xbf]);
    let mut r = csv::Reader::from_reader(&bytes[3..]);
    let h = r.headers().unwrap().clone();
    let index = |name: &str| h.iter().position(|x| x == name).unwrap();
    let rows = r.records().map(|r| r.unwrap()).collect::<Vec<_>>();
    let controls = rows
        .iter()
        .filter(|r| &r[index("row_kind")] == "control")
        .collect::<Vec<_>>();
    assert!(controls.len() >= 10);
    for r in controls {
        assert_eq!(&r[index("difference_irr")], "0");
        assert_eq!(&r[index("source_reference")], "'=DANGEROUS()");
        assert_eq!(&r[index("policy_version")], "core-policy-v1");
        assert_eq!(&r[index("as_of_gregorian")], "2026-03-21");
        assert_eq!(&r[index("display_as_of")], "1405/01/01");
        assert_eq!(&r[index("base_currency")], "IRR");
        assert_eq!(&r[index("display_unit")], "toman");
    }
    for k in ["total_debit", "total_credit"] {
        let row = rows.iter().find(|r| &r[index("row_kind")] == k).unwrap();
        assert_eq!(&row[index("amount_irr")], "1251");
        assert_eq!(&row[index("display_amount")], "125.1");
    }
    let stock = rows.iter().find(|r| &r[index("id")] == "ST-W1").unwrap();
    assert_eq!(&stock[index("quantity_thousandths")], "2000");
    assert_eq!(&stock[index("base_quantity_thousandths")], "24000");
    let capital = rows
        .iter()
        .find(|r| r[index("row_kind")] == *"ledger" && r[index("label")].contains("3101"))
        .unwrap();
    assert!(capital[index("label")].contains("سرمایه"));
    assert_eq!(
        &capital[index("id")],
        state.core.setups[0]
            .account_id(CoreRole::OwnerCapital)
            .unwrap()
    );
    let cash = rows.iter().find(|r| &r[index("id")] == "C1").unwrap();
    assert_eq!(&cash[index("label")], "'=CASH-FORMULA()");
    let full = rows
        .iter()
        .find(|r| &r[index("row_kind")] == "opening_snapshot")
        .unwrap();
    let reconstructed: OpeningRecord = serde_json::from_str(&full[index("snapshot_json")]).unwrap();
    assert_eq!(reconstructed, state.core.openings[0]);
    assert!(opening_csv(&state, &co, MoneyUnit::Rial, DateDisplay::Iso, &path).is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);
}
#[test]
fn persian_pdf_repeats_headers_includes_every_detail_exact_units_dates_and_pages() {
    let (state, co) = fixture();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("report.pdf");
    opening_pdf_with_preferences(
        &state,
        &co,
        MoneyUnit::Toman,
        DateDisplay::Jalali,
        DisplayDigits::Latin,
        &path,
    )
    .unwrap();
    assert!(fs::read(&path).unwrap().starts_with(b"%PDF"));
    let output = Command::new("pdftotext")
        .arg(&path)
        .arg("-")
        .output()
        .expect("pdftotext required for independent PDF check");
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    for value in [
        "125.1",
        "1251",
        "IRR",
        "toman",
        "2026-03-21",
        "1405/01/01",
        "core-policy-v1",
        "ST-W1",
        "ST-W2",
        "R1",
        "CR1",
        "CP1",
        "AR-125",
        "AR-75",
        "AP-100",
        "AP-49",
        "HISTORIC-SETTLED-INVOICE-CR1",
        "HISTORIC-SETTLED-BILL-CP1",
        "24.000",
        "3101",
        "1101",
    ] {
        assert!(text.contains(value), "PDF missing {value}: {text}");
    }
    // Pango's shaped Persian glyph runs can be extracted with intra-word spaces.
    let logical_text: String = text
        .chars()
        .filter(|c| {
            !c.is_whitespace() && !matches!(c,'\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}')
        })
        .collect();
    assert!(
        logical_text.contains("سرمایه") && logical_text.contains("صندوق"),
        "human account names absent"
    );
    assert!(
        text.matches("TEST-OPENING").count() >= 2,
        "repeated headers absent"
    );
    let info = Command::new("pdfinfo").arg(&path).output().unwrap();
    assert!(info.status.success());
    let info = String::from_utf8(info.stdout).unwrap();
    let pages = info
        .lines()
        .find(|l| l.starts_with("Pages:"))
        .unwrap()
        .split_whitespace()
        .last()
        .unwrap()
        .parse::<usize>()
        .unwrap();
    assert!(pages >= 2);
    assert_eq!(text.matches("TEST-OPENING").count(), pages);
    let before = fs::read(&path).unwrap();
    assert!(opening_pdf(&state, &co, MoneyUnit::Rial, DateDisplay::Iso, &path).is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
    if let Ok(dir) = std::env::var("SARV_CORE_EXPORT_ARTIFACT_DIR") {
        fs::create_dir_all(&dir).unwrap();
        fs::copy(
            &path,
            std::path::Path::new(&dir).join("opening-reconciliation.pdf"),
        )
        .unwrap();
        fs::write(
            std::path::Path::new(&dir).join("opening-pdf-text.txt"),
            text,
        )
        .unwrap();
    }
}
#[test]
fn uninitialized_books_never_export_verified_zero() {
    let mut s = AccountingState::default();
    let c = s
        .setup_core_company(NewCoreCompany {
            name: "بدون افتتاحیه".into(),
            fiscal_name: "۱۴۰۵".into(),
            start_date: "2026-03-21".into(),
            end_date: "2027-03-20".into(),
            timezone: CompanyTimezone::AsiaTehran,
            chart: default_chart(),
            actor: "آزمایشگر".into(),
        })
        .unwrap();
    let d = tempfile::tempdir().unwrap();
    let csv = d.path().join("empty.csv");
    let pdf = d.path().join("empty.pdf");
    assert!(opening_csv(&s, &c.company_id, MoneyUnit::Rial, DateDisplay::Iso, &csv).is_err());
    assert!(opening_pdf(&s, &c.company_id, MoneyUnit::Rial, DateDisplay::Iso, &pdf).is_err());
    assert!(!csv.exists() && !pdf.exists());
}
#[test]
fn backup_restores_identical_usable_opening_exports() {
    let (accounting, co) = fixture();
    let state = State {
        accounting,
        ..State::default()
    };
    let dir = tempfile::tempdir().unwrap();
    let backup = dir.path().join("opening.sarv");
    backup_state(&state, &backup).unwrap();
    let restored = Store::restore_preview(&backup).unwrap();
    assert_eq!(restored.accounting.core, state.accounting.core);
    let a = dir.path().join("before.csv");
    let b = dir.path().join("restored.csv");
    opening_csv(
        &state.accounting,
        &co,
        MoneyUnit::Toman,
        DateDisplay::Jalali,
        &a,
    )
    .unwrap();
    opening_csv(
        &restored.accounting,
        &co,
        MoneyUnit::Toman,
        DateDisplay::Jalali,
        &b,
    )
    .unwrap();
    assert_eq!(fs::read(a).unwrap(), fs::read(b).unwrap());
    opening_pdf(
        &restored.accounting,
        &co,
        MoneyUnit::Toman,
        DateDisplay::Jalali,
        &dir.path().join("restored.pdf"),
    )
    .unwrap();
}

#[test]
fn gregorian_outside_jalali_range_exports_without_false_conversion() {
    let mut state = AccountingState::default();
    let setup = state
        .setup_core_company(NewCoreCompany {
            name: "شرکت تاریخی TEST-HISTORY".into(),
            fiscal_name: "۱۸۰۰".into(),
            start_date: "1800-03-21".into(),
            end_date: "1801-03-20".into(),
            timezone: CompanyTimezone::Utc,
            chart: default_chart(),
            actor: "آزمایشگر".into(),
        })
        .unwrap();
    let input = include_str!("../docs/assets/accounting-m3/opening.csv")
        .replace("F2-core-v1", &setup.company_id)
        .replace("2026-", "1800-");
    let command = import_opening_csv(
        &state,
        &setup.company_id,
        &input,
        "آزمایشگر",
        "historical-export",
    )
    .unwrap();
    state.post_opening(command).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let pdf = dir.path().join("historical.pdf");
    opening_pdf_with_preferences(
        &state,
        &setup.company_id,
        MoneyUnit::Rial,
        DateDisplay::Iso,
        DisplayDigits::Latin,
        &pdf,
    )
    .unwrap();
    let result = Command::new("pdftotext")
        .arg(&pdf)
        .arg("-")
        .output()
        .unwrap();
    let text = String::from_utf8(result.stdout).unwrap();
    assert!(text.contains("1800-03-21"));
    let denied = dir.path().join("jalali.pdf");
    assert!(
        opening_pdf(
            &state,
            &setup.company_id,
            MoneyUnit::Rial,
            DateDisplay::Jalali,
            &denied
        )
        .is_err()
    );
    assert!(!denied.exists());
}

#[test]
fn reviewed_explicit_zero_initialization_exports_without_fake_financial_journal() {
    let mut state = AccountingState::default();
    let setup = state
        .setup_core_company(NewCoreCompany {
            name: "شرکت تازه TEST-ZERO".into(),
            fiscal_name: "۱۴۰۵".into(),
            start_date: "2026-03-21".into(),
            end_date: "2027-03-20".into(),
            timezone: CompanyTimezone::AsiaTehran,
            chart: default_chart(),
            actor: "آزمایشگر".into(),
        })
        .unwrap();
    let command = OpeningCommand {
        company_id: setup.company_id.clone(),
        period_id: setup.period_id.clone(),
        setup_id: setup.setup_id.clone(),
        as_of: "2026-03-21".into(),
        source_id: "ZERO-REVIEWED".into(),
        request_key: "zeroexport".into(),
        actor: "آزمایشگر".into(),
        reason: "تأیید آغاز شرکت بدون مانده".into(),
        balances: state.core.setups[0]
            .command
            .mappings
            .iter()
            .filter(|m| m.role.is_control())
            .map(|m| OpeningBalance {
                account_id: m.account_id.clone(),
                debit: sarv_quote::accounting::Money::ZERO,
                credit: sarv_quote::accounting::Money::ZERO,
                source_reference: format!("zero-{}", m.role.key()),
            })
            .collect(),
        parties: vec![],
        units: vec![],
        items: vec![],
        warehouses: vec![],
        cash: vec![],
        banks: vec![],
        open_items: vec![],
        advances: vec![],
        cheques: vec![],
        stock: vec![],
        grni: vec![],
        import_source: None,
    };
    state.post_opening(command).unwrap();
    assert!(state.journals.is_empty());
    assert_eq!(
        state.core.openings[0].effect,
        OpeningEffect::ZeroBalanceInitialization
    );
    let d = tempfile::tempdir().unwrap();
    let csv = d.path().join("zero.csv");
    opening_csv(
        &state,
        &setup.company_id,
        MoneyUnit::Rial,
        DateDisplay::Iso,
        &csv,
    )
    .unwrap();
    let bytes = fs::read(csv).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.contains("zero_balance_initialization"));
    assert!(text.contains("initialized_reconciled_zero_opening"));
    let pdf = d.path().join("zero.pdf");
    opening_pdf(
        &state,
        &setup.company_id,
        MoneyUnit::Rial,
        DateDisplay::Iso,
        &pdf,
    )
    .unwrap();
    let output = Command::new("pdftotext")
        .arg(&pdf)
        .arg("-")
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("TEST-ZERO"));
}
