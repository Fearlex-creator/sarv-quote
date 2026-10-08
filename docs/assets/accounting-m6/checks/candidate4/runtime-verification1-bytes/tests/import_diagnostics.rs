use sarv_quote::imports::*;

#[test]
fn catalog_errors_identify_all_bad_columns_and_never_return_partial_catalog() {
    let input = "code,name,measure,price,currency,source_at,aliases\nA,درست,عدد,12.3,toman,2020-01-01T00:00:00Z,\nA,,عدد,-1,rial,invalid,\nB,نام,عدد,1,unknown,2020-01-01T00:00:00Z,";
    let report = catalog_csv_report(input, "آزمایش");
    assert!(report.value.is_none());
    for (row, column) in [
        (3, "code"),
        (3, "name"),
        (3, "price"),
        (3, "source_at"),
        (4, "currency"),
    ] {
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.row == row && d.column == column),
            "missing diagnostic {row}/{column}"
        );
    }
    assert!(!report.truncated);
    assert!(
        report
            .into_result()
            .unwrap_err()
            .to_string()
            .contains("ستون price")
    );
}

#[test]
fn order_diagnostics_are_bounded_and_repeated_valid_rows_remain_separate() {
    let input = format!("item,quantity\n{}", "A,0\n".repeat(1000));
    let report = order_csv_report(&input);
    assert!(report.value.is_none());
    assert_eq!(report.diagnostics.len(), MAX_DIAGNOSTICS);
    assert!(report.truncated);
    assert!(report.diagnostics.iter().all(|d| d.column == "quantity"));
    let report = order_csv_report("item,quantity\nA,۱٫۲۵\nA,2");
    assert!(report.diagnostics.is_empty());
    assert!(!report.truncated);
    let lines = report.value.unwrap();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].quantity_milli, 1250);
}

#[test]
fn malformed_structure_and_headers_have_explicit_diagnostics() {
    let report = order_csv_report("item,quantity\nA,1,extra\nB,0");
    assert!(report.value.is_none());
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.row == 2 && d.column == "row")
    );
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.row == 3 && d.column == "quantity")
    );
    assert_eq!(
        catalog_csv_report("wrong,header\na,b", "نام").diagnostics[0].column,
        "header"
    );
    let oversized = order_csv_report(&"x".repeat(MAX_IMPORT + 1));
    assert_eq!(oversized.diagnostics[0].column, "file");
}
