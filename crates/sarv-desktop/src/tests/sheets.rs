use super::harness::{blow, click, make_test_runtime};
use crate::app::Page;

#[test]
fn test_sheet_formulas_save_beside_the_books() {
    let mut rt = make_test_runtime();
    let generation = sarv_quote::store::Store::open(&rt.state.engine.dir).unwrap().local_generation();
    click(&mut rt, Page::Sheets.key());
    assert!(rt.find("sheet-card").is_some(), "{}", blow(&rt, "sheet"));
    click(&mut rt, "cell-B1");
    assert_eq!(rt.state.forms.get("sheet-formula"), "=BOOKS(\"cash\")");

    click(&mut rt, "cell-C1");
    rt.state.forms.set("sheet-formula", "10");
    click(&mut rt, "btn-sheet-apply");
    click(&mut rt, "cell-C2");
    rt.state.forms.set("sheet-formula", "25");
    click(&mut rt, "btn-sheet-apply");
    click(&mut rt, "cell-C3");
    rt.state.forms.set("sheet-formula", "=C1+C2");
    click(&mut rt, "btn-sheet-apply");
    click(&mut rt, "btn-Add sheet");
    assert_eq!(rt.state.sheets.book.sheets.len(), 2);
    click(&mut rt, "btn-sheet-tab-0");
    assert!(rt.state.sheets.dirty, "edits leave the workbook dirty before save");
    click(&mut rt, "btn-Save sheet");
    assert!(!rt.state.sheets.dirty, "successful save must clear the dirty flag");

    let loaded = rt
        .state
        .engine
        .call("sheet_load", serde_json::json!({ "company_id": rt.state.books.company_id }))
        .unwrap();
    assert_eq!(loaded["workbook"]["sheets"][0]["cells"]["C3"], "=C1+C2");
    assert_eq!(loaded["workbook"]["sheets"][0]["cells"]["B1"], "=BOOKS(\"cash\")");
    assert_eq!(
        sarv_quote::store::Store::open(&rt.state.engine.dir).unwrap().local_generation(),
        generation
    );
    assert!(
        rt.state
            .toasts
            .iter()
            .any(|t| t.message.contains("برگه") || t.message.contains("Sheet"))
    );
}
