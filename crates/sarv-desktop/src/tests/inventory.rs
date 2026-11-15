use super::harness::{assert_no_danger, blow, books_validate, click, make_test_runtime, type_into};
use crate::app::Page;
use crate::data;

#[test]
fn test_new_product_dialog_and_save_headlessly() {
    let mut rt = make_test_runtime();
    click(&mut rt, Page::Inventory.key());
    assert!(rt.find("inventory-table").is_some());
    click(&mut rt, "btn-New product");
    assert!(rt.state.inventory.modal_open);
    assert!(rt.find("field-item-name").is_some());
    assert!(rt.find("field-item-sku").is_some());
    assert!(rt.find("field-item-price").is_some());
    assert!(rt.find("item-unit").is_some(), "unit Select present");

    rt.state.forms.set("item-name", "Test Tea");
    rt.state.forms.set("item-sku", "TEA-1");
    rt.state.inventory.unit_id = "PIECE".into();
    rt.state.forms.set("item-price", "25000");
    rt.state.forms.set("item-barcode", "123");
    rt.state.forms.set("item-notes", "Green tea");
    click(&mut rt, "btn-Save");
    assert!(!rt.state.inventory.modal_open, "dialog closes after save");
    let items = rt.state.books.workspace["resources"]["items"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(
        items.iter().any(|i| i["name"] == "Test Tea" || i["sku"] == "TEA-1"),
        "product persisted in resources.items"
    );
}

fn warehouse_qty(rt: &mut rune::Runtime<crate::app::State>, item_id: &str, warehouse_id: &str) -> i64 {
    let detail = rt
        .state
        .query("product_detail", serde_json::json!({ "item_id": item_id, "as_of": data::today() }))
        .unwrap_or_else(|e| panic!("{e}"));
    detail["stock"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|s| data::s(&s["warehouse_id"]) == warehouse_id)
        .and_then(|s| data::s(&s["quantity_thousandths"]).parse::<i64>().ok())
        .unwrap_or(0)
}

fn total_qty(rt: &mut rune::Runtime<crate::app::State>, item_id: &str) -> i64 {
    let detail = rt
        .state
        .query("product_detail", serde_json::json!({ "item_id": item_id, "as_of": data::today() }))
        .unwrap_or_else(|e| panic!("{e}"));
    data::s(&detail["stats"]["stock_quantity"]).parse::<i64>().unwrap_or(0)
}

#[test]
fn stock_transfer_moves_quantity_between_warehouses() {
    let mut rt = make_test_runtime();
    rt.state
        .run(
            "resource",
            serde_json::json!({
                "kind": "warehouse",
                "id": "WH-2",
                "name": "Branch Warehouse",
                "reason": "Add second warehouse"
            }),
            None,
        )
        .unwrap_or_else(|e| panic!("{e}"));

    let src = crate::ui::pickers::first_warehouse(&rt.state);
    let dst = "WH-2".to_string();
    let src_before = warehouse_qty(&mut rt, "ITEM-A", &src);
    let dst_before = warehouse_qty(&mut rt, "ITEM-A", &dst);
    let total_before = total_qty(&mut rt, "ITEM-A");
    assert!(src_before >= 1000, "demo item needs stock to transfer: {src_before}");

    click(&mut rt, Page::Inventory.key());
    click(&mut rt, "btn-Transfer stock");
    // Select pickers cannot be driven headlessly via dropdown item keys; set IDs directly.
    rt.state.inventory.transfer_item = "ITEM-A".into();
    rt.state.inventory.transfer_from = src.clone();
    rt.state.inventory.transfer_to = dst.clone();
    type_into(&mut rt, "transfer-qty", "2");
    click(&mut rt, "btn-Save");

    assert!(!rt.state.inventory.transfer_open, "{}", blow(&rt, "stock transfer"));
    assert_eq!(warehouse_qty(&mut rt, "ITEM-A", &src), src_before - 2000);
    assert_eq!(warehouse_qty(&mut rt, "ITEM-A", &dst), dst_before + 2000);
    assert_eq!(total_qty(&mut rt, "ITEM-A"), total_before);
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn product_profile_save_round_trips_every_field() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;

    click(&mut rt, Page::Inventory.key());
    click(&mut rt, "item-ITEM-A");
    click(&mut rt, "btn-Product details");
    assert!(rt.state.inventory.profile_open, "{}", blow(&rt, "open profile"));

    type_into(&mut rt, "prod-brand", "SarvBrand");
    type_into(&mut rt, "prod-category", "Beverages");
    type_into(&mut rt, "prod-barcode", "6261234567890");
    type_into(&mut rt, "prod-cost", "80000");
    type_into(&mut rt, "prod-price", "120000");
    type_into(&mut rt, "prod-min-stock", "5");
    type_into(&mut rt, "prod-notes", "Keep dry");
    click(&mut rt, "btn-Save");
    assert!(!rt.state.inventory.profile_open, "{}", blow(&rt, "save profile 1"));

    click(&mut rt, "item-ITEM-A");
    click(&mut rt, "btn-Product details");
    assert!(rt.state.inventory.profile_open, "{}", blow(&rt, "reopen profile 1"));
    let snapshot: Vec<(&str, String)> = [
        "prod-brand",
        "prod-category",
        "prod-barcode",
        "prod-cost",
        "prod-price",
        "prod-min-stock",
        "prod-notes",
    ]
    .into_iter()
    .map(|k| (k, rt.state.forms.get(k)))
    .collect();

    click(&mut rt, "btn-Save");
    assert!(!rt.state.inventory.profile_open, "{}", blow(&rt, "save profile 2"));

    click(&mut rt, "item-ITEM-A");
    click(&mut rt, "btn-Product details");
    assert!(rt.state.inventory.profile_open, "{}", blow(&rt, "reopen profile 2"));

    for (k, expected) in &snapshot {
        assert_eq!(rt.state.forms.get(k), *expected, "field `{k}` changed on round-trip");
    }
    assert_eq!(rt.state.forms.get("prod-min-stock"), "5");
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn test_transfer_double_click_posts_once() {
    let mut rt = make_test_runtime();
    rt.state
        .run(
            "resource",
            serde_json::json!({ "kind": "warehouse", "id": "WH-DBL", "name": "Double Click Depot", "reason": "Test" }),
            None,
        )
        .unwrap_or_else(|e| panic!("{e}"));

    let src = crate::ui::pickers::first_warehouse(&rt.state);
    let dst = "WH-DBL".to_string();
    let src_before = warehouse_qty(&mut rt, "ITEM-A", &src);
    let dst_before = warehouse_qty(&mut rt, "ITEM-A", &dst);

    click(&mut rt, Page::Inventory.key());
    click(&mut rt, "btn-Transfer stock");
    rt.state.inventory.transfer_item = "ITEM-A".into();
    rt.state.inventory.transfer_from = src.clone();
    rt.state.inventory.transfer_to = dst.clone();
    type_into(&mut rt, "transfer-qty", "2");
    crate::tests::walkthrough::double_click(&mut rt, "btn-Save");

    assert!(!rt.state.inventory.transfer_open, "{}", blow(&rt, "double click transfer"));
    assert_eq!(warehouse_qty(&mut rt, "ITEM-A", &src), src_before - 2000);
    assert_eq!(warehouse_qty(&mut rt, "ITEM-A", &dst), dst_before + 2000);
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn test_product_profile_roundtrip_keeps_price() {
    let mut rt = make_test_runtime();
    assert_eq!(rt.state.fmt.unit, crate::i18n::Unit::Toman);
    rt.state
        .run(
            "product_profile",
            serde_json::json!({
                "item_id": "ITEM-A",
                "profile": {
                    "barcode": "6260001",
                    "category": "",
                    "notes": "",
                    "sale_price": "12345",
                    "minimum_stock": "0"
                }
            }),
            None,
        )
        .expect("seed price");

    click(&mut rt, Page::Inventory.key());
    click(&mut rt, "item-ITEM-A");
    click(&mut rt, "btn-Product details");
    assert!(rt.state.inventory.profile_open, "{}", blow(&rt, "open profile"));

    click(&mut rt, "btn-Save");
    assert!(!rt.state.inventory.profile_open, "{}", blow(&rt, "save profile"));

    let detail = rt
        .state
        .query("product_detail", serde_json::json!({ "item_id": "ITEM-A", "as_of": data::today() }))
        .expect("product_detail");
    assert_eq!(data::s(&detail["profile"]["sale_price"]), "12345");
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn test_product_profile_keeps_minimum_stock() {
    let mut rt = make_test_runtime();
    rt.state
        .run(
            "product_profile",
            serde_json::json!({
                "item_id": "ITEM-A",
                "profile": {
                    "barcode": "",
                    "category": "",
                    "notes": "",
                    "minimum_stock": "5"
                }
            }),
            None,
        )
        .expect("seed minimum_stock");

    click(&mut rt, Page::Inventory.key());
    click(&mut rt, "item-ITEM-A");
    click(&mut rt, "btn-Product details");
    assert!(rt.state.inventory.profile_open, "{}", blow(&rt, "open profile"));
    assert_eq!(rt.state.forms.get("prod-min-stock"), "5");

    click(&mut rt, "btn-Save");
    assert!(!rt.state.inventory.profile_open, "{}", blow(&rt, "save profile"));

    let detail = rt
        .state
        .query("product_detail", serde_json::json!({ "item_id": "ITEM-A", "as_of": data::today() }))
        .expect("product_detail");
    assert_eq!(data::s(&detail["profile"]["minimum_stock_thousandths"]), "5000");
    assert_no_danger(&rt);
    books_validate(&rt);
}
