use super::harness::{blow, click, make_test_runtime};
use crate::app::{Page, Tone};

#[test]
fn test_pos_charges_the_catalog_and_keeps_the_books() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    rt.state
        .run(
            "product_profile",
            serde_json::json!({
                "item_id": "ITEM-A",
                "profile": {
                    "barcode": "6260001",
                    "category": "",
                    "notes": "",
                    "sale_price": "50000",
                    "minimum_stock": "0"
                }
            }),
            None,
        )
        .expect("price");
    rt.update(|s, _| {
        s.show_pos = true;
        s.nav(Page::Pos);
    });
    rt.frame(1.0 / 60.0);
    assert!(rt.find(Page::Pos.key()).is_some(), "POS nav row mounts after the feature flag");
    assert_eq!(rt.state.page, Page::Pos);
    assert!(rt.find("pos-screen").is_some());
    assert!(rt.find("pos-ticket").is_some());
    assert!(
        rt.state.forms.view("pos-search").focused,
        "POS opens with catalog search focused for barcode entry"
    );
    assert!(rt.find("field-pos-search").is_some());
    assert!(crate::ui::views::pos::product_matches(
        "کالای الف",
        "SKU-A",
        "ITEM-A",
        "6260001",
        "sku-a"
    ));
    assert!(crate::ui::views::pos::product_matches(
        "کالای الف",
        "SKU-A",
        "ITEM-A",
        "6260001",
        "۶۲۶"
    ));
    assert!(!crate::ui::views::pos::product_matches(
        "کالای الف",
        "SKU-A",
        "ITEM-A",
        "6260001",
        "no-such-sku"
    ));

    click(&mut rt, "pos-item-ITEM-A");
    click(&mut rt, "btn-pos-account");
    assert_eq!(rt.state.pos.lines.len(), 1, "on account waits for a customer");
    click(&mut rt, "pos-item-ITEM-A");
    assert_eq!(rt.state.pos.lines[0].qty, 2);
    click(&mut rt, "pos-minus-ITEM-A");
    assert_eq!(rt.state.pos.lines[0].qty, 1);
    rt.state.forms.set("pos-received", "1000");
    click(&mut rt, "btn-pos-cash");
    assert_eq!(rt.state.pos.lines.len(), 1, "short cash does not post");

    rt.state.forms.set("pos-received", "50000");
    let before = rt.state.books.count("sales_count");
    click(&mut rt, "btn-pos-cash");
    assert!(rt.state.pos.lines.is_empty(), "{}", blow(&rt, "pos cash"));
    assert!(rt.state.books.count("sales_count") > before);
    let dumped = serde_json::to_string(&rt.state.books.list("sales")[0]).unwrap_or_default();
    assert!(dumped.contains("50000"), "catalog price posted once: {dumped}");
    assert!(rt.state.toasts.iter().any(|t| t.tone == Tone::Success));
}
