use sarv_quote::{domain::*, imports::*};

fn catalog() -> Catalog {
    let source = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
    let products = vec![
        Product {
            code: "K-۱".into(),
            name: "کالای اول".into(),
            measure: "عدد".into(),
            price_rial: 101,
            source_unit: Unit::Rial,
            source_at: source.clone(),
            aliases: vec![
                "نام\u{200c}فارسی".into(),
                "نام فارسی".into(),
                "نام  فارسي".into(),
                "مشترک".into(),
                "X".into(),
            ],
        },
        Product {
            code: "X".into(),
            name: "کالای دوم".into(),
            measure: "عدد".into(),
            price_rial: 205,
            source_unit: Unit::Rial,
            source_at: source.clone(),
            aliases: vec!["k-1".into(), "مشترک".into()],
        },
        Product {
            code: "ك-ي٢".into(),
            name: "کالای سوم".into(),
            measure: "عدد".into(),
            price_rial: 307,
            source_unit: Unit::Rial,
            source_at: source,
            aliases: vec!["مشترک".into()],
        },
    ];
    Catalog {
        id: id(),
        name: "آزمون نمایه".into(),
        imported_at: now(),
        fingerprint: product_fingerprint(&products).unwrap(),
        products,
    }
}
#[test]
fn index_matches_legacy_semantics_across_scripts_and_exact_priority() {
    let catalog = catalog();
    let index = catalog.index();
    for query in [
        "K-1",
        "k-١",
        "K-۱",
        "نام فارسی",
        "نام\u{200c}فارسي",
        "نام   فارسی",
        "X",
        "x",
        "مشترک",
        "ک-ی۲",
        "ك-ي٢",
        "ک-ی٢",
        "missing",
        "",
    ] {
        let legacy = catalog
            .matches(query)
            .iter()
            .map(|p| p.code.as_str())
            .collect::<Vec<_>>();
        let indexed = index
            .matches(query)
            .iter()
            .map(|p| p.code.as_str())
            .collect::<Vec<_>>();
        assert_eq!(indexed, legacy, "query {query}");
    }
    assert_eq!(
        index
            .matches("x")
            .iter()
            .map(|p| p.code.as_str())
            .collect::<Vec<_>>(),
        ["X"]
    );
    assert_eq!(index.matches("نام فارسی").len(), 1); // Three equivalent aliases on one product are one candidate.
    assert_eq!(
        index
            .matches("مشترک")
            .iter()
            .map(|p| p.code.as_str())
            .collect::<Vec<_>>(),
        ["K-۱", "X", "ك-ي٢"]
    );
    assert_eq!(index.product_by_code("X").unwrap().price_rial, 205);
    assert!(index.product_by_code("x").is_none());
    assert!(index.product_by_code("K-1").is_none());
}
#[test]
fn indexed_preview_still_requires_human_choice_and_exact_selected_code() {
    let catalog = catalog();
    let mut state = State::default();
    accept_catalog(&mut state, catalog).unwrap();
    state.draft.customer = "مشتری آزمون".into();
    state.draft.lines = vec![OrderLine {
        query: "مشترک".into(),
        quantity_milli: 1500,
        chosen_code: None,
    }];
    assert!(state.preview().is_err());
    assert!(state.approve().is_err());
    assert!(state.quotes.is_empty());
    state.draft.lines[0].chosen_code = Some("x".into());
    assert!(state.preview().is_err());
    state.draft.lines[0].chosen_code = Some("X".into());
    let preview = state.preview().unwrap();
    assert_eq!(preview.lines[0].code, "X");
    assert_eq!(preview.lines[0].query, "مشترک");
    assert_eq!(preview.total_rial, 308);
    state.approve().unwrap();
    state.validate().unwrap();
    state.draft.lines[0].chosen_code = None;
    state.draft.lines[0].query = "K-١".into();
    assert_eq!(state.preview().unwrap().lines[0].code, "K-۱");
    assert_eq!(state.preview().unwrap().total_rial, 152);
}
#[test]
fn normalization_code_collisions_preserve_multiple_candidates_in_order() {
    // Validated imports reject this catalog, but the index must never conceal its ambiguity.
    let mut catalog = catalog();
    let mut duplicate = catalog.products[0].clone();
    duplicate.code = "k-١".into();
    duplicate.name = "نام متفاوت".into();
    catalog.products.push(duplicate);
    let indexed = catalog.index().matches("k-1");
    assert_eq!(
        indexed.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
        ["کالای اول", "نام متفاوت"]
    );
    assert_eq!(catalog.matches("k-1").len(), 2);
}
