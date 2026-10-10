use sarv_quote::{
    accounting::{AccountingState, Money},
    core::*,
    core_import::*,
};
fn fixture() -> (AccountingState, String, String) {
    let mut state = AccountingState::default();
    let setup = state
        .setup_core_company(NewCoreCompany {
            name: "شرکت مصنوعی F2".into(),
            fiscal_name: "سال ۱۴۰۵".into(),
            start_date: "2026-03-21".into(),
            end_date: "2027-03-20".into(),
            timezone: CompanyTimezone::AsiaTehran,
            chart: default_chart(),
            actor: "آزمایشگر".into(),
        })
        .unwrap();
    let csv = include_str!("../../../docs/assets/accounting-m3/opening.csv")
        .replace("F2-core-v1", &setup.company_id);
    (state, setup.company_id, csv)
}
fn change(csv: &str, id: &str, col: &str, value: &str) -> String {
    let mut r = csv::Reader::from_reader(csv.as_bytes());
    let h = r.headers().unwrap().clone();
    let id_index = h.iter().position(|x| x == "id").unwrap();
    let col_index = h.iter().position(|x| x == col).unwrap();
    let mut w = csv::Writer::from_writer(vec![]);
    w.write_record(&h).unwrap();
    for row in r.records() {
        let row = row.unwrap();
        let mut v = row.iter().map(String::from).collect::<Vec<_>>();
        if row[id_index] == *id {
            v[col_index] = value.into();
        }
        w.write_record(v).unwrap();
    }
    String::from_utf8(w.into_inner().unwrap()).unwrap()
}
#[test]
fn independent_literal_opening_stages_without_mutation_and_preserves_provenance() {
    let (mut state, co, input) = fixture();
    let before = state.clone();
    let command = opening_csv(&state, &co, &input, "آزمایشگر", "import-1").unwrap();
    assert_eq!(state, before);
    let p = state.opening_preview(&command).unwrap();
    assert_eq!(p.total_debit.rials(), 1251);
    assert_eq!(p.total_credit.rials(), 1251);
    assert!(p.balanced && p.reconciled);
    assert!(p.controls.len() >= 10);
    assert!(p.controls.iter().all(|c| c.difference == Money::ZERO));
    assert_eq!(
        command.base_quantity("ITEM-A", "CARTON", 2000).unwrap(),
        24000
    );
    assert_eq!(
        command.stock.iter().map(|s| s.value.rials()).sum::<i64>(),
        300
    );
    assert_eq!(
        command
            .open_items
            .iter()
            .filter(|x| x.side == OpenItemSide::Receivable)
            .map(|x| x.remaining.rials())
            .sum::<i64>(),
        200
    );
    assert_eq!(
        command.import_source.as_ref().unwrap().sha256,
        sarv_quote::imports::hash(input.as_bytes())
    );
    let source = command.import_source.as_ref().unwrap();
    assert!(
        source
            .rows
            .iter()
            .any(|r| r.id == "R1" && r.original_values[20] == "RECEIPT-R1")
    );
    state.post_opening(command).unwrap();
    assert!(
        state
            .reconcile_opening(&co, "2026-03-21")
            .unwrap()
            .reconciled
    );
}
#[test]
fn invalid_content_reports_row_and_column_and_never_writes() {
    let (state, co, input) = fixture();
    let before = state.clone();
    for (id, col, value) in [
        ("ST-W1", "quantity", "-2"),
        ("ST-W1", "unit_id", "UNKNOWN"),
        ("PACK-A-CARTON", "numerator", "0"),
        ("AR-125", "amount", "0.1"),
        ("GL-cash", "amount", "9223372036854775808"),
        ("C1", "unit", "USD"),
        ("CR1", "state", "cleared"),
        ("R1", "company_id", "OTHER-COMPANY"),
        ("CU-A", "customer", "yes"),
        ("AR-125", "due_date", "1405/01/01"),
    ] {
        let bad = change(&input, id, col, value);
        let r = opening_csv_report(&state, &co, &bad, "آزمایشگر", "invalid");
        assert!(r.value.is_none(), "accepted {id}.{col}");
        assert!(!r.diagnostics.is_empty());
        assert!(
            r.diagnostics.iter().any(|d| d.row > 1),
            "no row-specific diagnostic {id}.{col}: {:?}",
            r.diagnostics
        );
        assert_eq!(state, before);
    }
}
#[test]
fn one_rial_difference_remains_reviewable_and_post_rejects() {
    let (mut state, co, input) = fixture();
    let input = change(&input, "AR-125", "amount", "126");
    let c = opening_csv(&state, &co, &input, "آزمایشگر", "mismatch").unwrap();
    let p = state.opening_preview(&c).unwrap();
    assert_eq!(
        p.controls
            .iter()
            .find(|x| x.role == CoreRole::Receivables)
            .unwrap()
            .difference
            .rials(),
        -1
    );
    assert!(!p.reconciled);
    let before = state.clone();
    assert!(state.post_opening(c).is_err());
    assert_eq!(state, before);
}
#[test]
fn explicit_units_safe_digits_duplicates_unknown_columns_and_empty_files() {
    let (state, co, input) = fixture();
    let bad = change(&input, "GL-bank", "id", "GL-cash");
    assert!(
        opening_csv_report(&state, &co, &bad, "actor", "key")
            .value
            .is_none()
    );
    let header = input.replacen("statement_amount", "unknown", 1);
    assert_eq!(
        opening_csv_report(&state, &co, &header, "actor", "key").diagnostics[0].column,
        "header"
    );
    assert!(opening_csv(&state, &co, "", "actor", "key").is_err());
    let persian = change(&input, "GL-cash", "amount", "۲۵۱");
    let persian = change(&persian, "C1", "amount", "٢٥١");
    assert!(
        opening_csv(&state, &co, &persian, "actor", "key")
            .unwrap()
            .cash[0]
            .balance
            .rials()
            == 251
    );
    let toman = change(&input, "C1", "unit", "toman");
    let toman = change(&toman, "C1", "amount", "25.1");
    assert_eq!(
        opening_csv(&state, &co, &toman, "actor", "key")
            .unwrap()
            .cash[0]
            .balance
            .rials(),
        251
    );
    let bad = change(&input, "ST-W1", "name", "ignored data");
    assert!(
        opening_csv_report(&state, &co, &bad, "actor", "key")
            .diagnostics
            .iter()
            .any(|e| e.column == "name")
    );
}
#[test]
fn bounded_read_and_exact_original_values() {
    let (state, co, input) = fixture();
    let input = change(&input, "C1", "amount", " ۲۵۱ ");
    let c = opening_csv(&state, &co, &input, "actor", "key").unwrap();
    assert_eq!(
        c.import_source
            .as_ref()
            .unwrap()
            .rows
            .iter()
            .find(|r| r.id == "C1")
            .unwrap()
            .original_values[6],
        " ۲۵۱ "
    );
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("opening.csv");
    std::fs::write(&p, input).unwrap();
    assert!(
        opening_file_report(&state, &co, &p, "actor", "key")
            .value
            .is_some()
    );
    std::fs::write(&p, vec![b'x'; sarv_quote::imports::MAX_IMPORT + 1]).unwrap();
    let r = opening_file_report(&state, &co, &p, "actor", "key");
    assert!(r.value.is_none());
    assert_eq!(r.diagnostics[0].column, "file");
}

#[test]
fn crlf_source_diagnostics_and_original_refs_point_to_actual_data_row() {
    let (state, co, input) = fixture();
    let input = change(&input, "OPEN-F2-2026", "company_id", "OTHER").replace('\n', "\r\n");
    let r = opening_csv_report(&state, &co, &input, "actor", "key");
    let d = r
        .diagnostics
        .iter()
        .find(|d| d.column == "company_id")
        .unwrap();
    assert_eq!(d.row, 2);
}

#[test]
fn reference_pack_role_and_historical_errors_identify_their_source_columns() {
    let (state, co, input) = fixture();
    for (id, col, value) in [
        ("AR-75", "party_id", "SU-A"),
        ("AR-75", "document_date", "2026-04-01"),
        ("AR-75", "due_date", "2026-02-01"),
        ("B1", "statement_amount", "299"),
        ("CR1", "settlement_reference", "DOC-AR-125"),
    ] {
        let r = opening_csv_report(&state, &co, &change(&input, id, col, value), "actor", "key");
        assert!(r.value.is_none());
        assert!(
            r.diagnostics.iter().any(|d| d.row > 1 && d.column == col),
            "{id}.{col}: {:?}",
            r.diagnostics
        );
    }
}
