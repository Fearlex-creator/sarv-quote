use super::harness::{click, make_test_runtime};
use crate::app::Page;

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
