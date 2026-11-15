use sarv_quote::{
    desktop::{dispatch, seed_fixture},
    store::Store,
};
use serde_json::{Value, json};

fn call(dir: &std::path::Path, action: &str, p: Value) -> Value {
    dispatch(dir, action, p).unwrap()
}

fn fixture() -> (tempfile::TempDir, String) {
    let d = tempfile::tempdir().unwrap();
    seed_fixture(d.path()).unwrap();
    let s = Store::open(d.path()).unwrap();
    let c = s.state.accounting.companies[0].id.clone();
    (d, c)
}

#[test]
fn sheet_save_then_sheet_load_round_trips_formula() {
    let (d, c) = fixture();
    let workbook = json!({
        "sheets": [
            {
                "name": "Budget",
                "cells": {
                    "A1": "100",
                    "A2": "200",
                    "A3": "=SUM(A1:A2)"
                }
            }
        ]
    });
    let saved = call(d.path(), "sheet_save", json!({ "company_id": c, "workbook": workbook }));
    assert_eq!(saved["saved"], true);

    let loaded = call(d.path(), "sheet_load", json!({ "company_id": c }));
    assert_eq!(loaded["workbook"], workbook);
}

#[test]
fn second_company_sheet_is_isolated() {
    let (d, c1) = fixture();
    let c2 = "C2";

    let wb1 = json!({
        "sheets": [
            { "name": "C1 Sheet", "cells": { "A1": "111" } }
        ]
    });
    let wb2 = json!({
        "sheets": [
            { "name": "C2 Sheet", "cells": { "A1": "222" } }
        ]
    });

    call(d.path(), "sheet_save", json!({ "company_id": c1, "workbook": wb1 }));
    let empty_c2 = call(d.path(), "sheet_load", json!({ "company_id": c2 }));
    assert_eq!(empty_c2["workbook"], Value::Null);

    call(d.path(), "sheet_save", json!({ "company_id": c2, "workbook": wb2 }));
    assert_eq!(call(d.path(), "sheet_load", json!({ "company_id": c1 }))["workbook"], wb1);
    assert_eq!(call(d.path(), "sheet_load", json!({ "company_id": c2 }))["workbook"], wb2);
}

#[test]
fn oversized_workbook_payload_is_rejected() {
    let (d, c) = fixture();

    // Cell text > 500 chars is rejected.
    let too_long_cell = json!({
        "sheets": [
            { "name": "S1", "cells": { "A1": "x".repeat(501) } }
        ]
    });
    assert!(
        dispatch(d.path(), "sheet_save", json!({ "company_id": c, "workbook": too_long_cell })).is_err(),
        "cell text over 500 chars must be rejected"
    );

    // More than 12 sheets is rejected.
    let too_many_sheets: Vec<Value> = (1..=13).map(|i| json!({ "name": format!("S{i}"), "cells": {} })).collect();
    assert!(
        dispatch(
            d.path(),
            "sheet_save",
            json!({ "company_id": c, "workbook": { "sheets": too_many_sheets } })
        )
        .is_err(),
        "more than 12 sheets must be rejected"
    );

    // More than 4000 cells is rejected.
    let mut big_cells = serde_json::Map::new();
    for row in 1..=200 {
        for col in 0..=20 {
            let addr = format!("{}{}", (b'A' + col as u8) as char, row);
            big_cells.insert(addr, json!("1"));
            if big_cells.len() == 4001 {
                break;
            }
        }
        if big_cells.len() == 4001 {
            break;
        }
    }
    assert_eq!(big_cells.len(), 4001);
    assert!(
        dispatch(
            d.path(),
            "sheet_save",
            json!({ "company_id": c, "workbook": { "sheets": [{ "name": "Big", "cells": big_cells }] } })
        )
        .is_err(),
        "more than 4000 cells must be rejected"
    );
}
