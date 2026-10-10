use sarv_quote::{domain::*, imports, store::Store};

fn state() -> State {
    let mut state = State::default();
    let catalog = imports::catalog_csv("code,name,measure,price,currency,source_at,aliases\nA,کالای ساختگی,عدد,12500.3,toman,2020-01-01T00:00:00Z,", "فهرست ساختگی").unwrap();
    imports::accept_catalog(&mut state, catalog).unwrap();
    state.draft.lines = vec![OrderLine {
        query: "A".into(),
        quantity_milli: 1250,
        chosen_code: None,
    }];
    state.draft.stale_ack = true;
    state
}

#[test]
fn optional_customer_details_keep_legacy_serialized_bytes() {
    let original =
        r#"{"id":"01875931-c4e0-4982-a206-6c037ec91f4f","name":"مشتری","phone":"09120000000"}"#;
    let customer: Customer = serde_json::from_str(original).unwrap();
    assert_eq!(serde_json::to_string(&customer).unwrap(), original);
    assert!(!customer.archived);
    let draft = Draft::default();
    let value = serde_json::to_value(&draft).unwrap();
    for field in [
        "customer_id",
        "customer_contact",
        "customer_email",
        "customer_address",
    ] {
        assert!(value.get(field).is_none());
    }
    let old: Draft = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(old).unwrap(), value);
}

#[test]
fn selection_archival_and_editing_preserve_historical_customer_snapshots() {
    let mut state = state();
    let customer = state
        .save_customer_details(
            "مشتری ساختگی",
            "۰۹۱۲۰۰۰۰۰۰۰",
            "مخاطب ساختگی",
            "first@example.test",
            "نشانی اول\nطبقه ۲",
        )
        .unwrap();
    state.select_customer(&customer).unwrap();
    assert_eq!(state.draft.phone, "09120000000");
    let first = state.approve().unwrap();
    assert_eq!(state.approve().unwrap(), first);
    let historical = serde_json::to_value(&state.quotes[0]).unwrap();
    assert_eq!(state.quotes[0].customer_id, customer);
    assert_eq!(state.quotes[0].total_rial, 156254);
    state
        .edit_customer_details(
            &customer,
            "نام تازه",
            "09121111111",
            "مخاطب تازه",
            "next@example.test",
            "نشانی تازه",
        )
        .unwrap();
    assert_eq!(serde_json::to_value(&state.quotes[0]).unwrap(), historical);
    state.set_customer_archived(&customer, true).unwrap();
    let before = serde_json::to_value(&state.draft).unwrap();
    assert!(state.select_customer(&customer).is_err());
    assert_eq!(serde_json::to_value(&state.draft).unwrap(), before);
    state.revise(&first).unwrap();
    assert_eq!(state.draft.customer_contact, "مخاطب ساختگی");
    assert_eq!(state.draft.customer_email, "first@example.test");
    state.draft.stale_ack = true;
    assert_eq!(state.approve().unwrap(), first);
    state.set_customer_archived(&customer, false).unwrap();
    state.select_customer(&customer).unwrap();
    assert_eq!(state.draft.customer, "نام تازه");
    state.validate().unwrap();
}

#[test]
fn invalid_customer_details_are_atomic_and_all_fields_survive_restart_and_backup() {
    let directory = tempfile::tempdir().unwrap();
    let mut store = Store::open(directory.path()).unwrap();
    store
        .change(|s| {
            *s = state();
            s.save_customer_details(
                "ساختگی",
                "09120000000",
                "مخاطب",
                "contact@example.test",
                "نشانی\nآزمایشی",
            )?;
            Ok(())
        })
        .unwrap();
    let customer = store.state.customers[0].id.clone();
    let before = serde_json::to_value(&store.state).unwrap();
    for email in [
        "no-at",
        "a@@example.test",
        "a@.test",
        "a@example..test",
        "a b@example.test",
    ] {
        assert!(
            store
                .change(|s| s.edit_customer_details(
                    &customer,
                    "تغییر",
                    "0912",
                    "مخاطب",
                    email,
                    "نشانی"
                ))
                .is_err()
        );
        assert_eq!(serde_json::to_value(&store.state).unwrap(), before);
    }
    assert!(customer_details("bad\u{202e}", "", "address").is_err());
    assert!(customer_details("", "", &"x".repeat(2001)).is_err());
    store
        .change(|s| {
            s.select_customer(&customer)?;
            s.approve()?;
            s.set_customer_archived(&customer, true)
        })
        .unwrap();
    let saved = serde_json::to_value(&store.state).unwrap();
    let backup = directory.path().join("details.json");
    store.backup(&backup).unwrap();
    assert_eq!(
        serde_json::to_value(Store::restore_preview(&backup).unwrap()).unwrap(),
        saved
    );
    drop(store);
    assert_eq!(
        serde_json::to_value(Store::open(directory.path()).unwrap().state).unwrap(),
        saved
    );
}

#[test]
fn browsing_cache_normalizes_scripts_and_tokens_but_never_changes_exact_matching() {
    let mut state = state();
    let first = state
        .save_customer_details(
            "فروشگاه كاغذ",
            "۰۹۱۲",
            "مخاطب علي",
            "paper@example.test",
            "تهران خیابان آبان",
        )
        .unwrap();
    state
        .save_customer_details(
            "فروشگاه قلم",
            "0913",
            "مخاطب دوم",
            "pen@example.test",
            "شیراز",
        )
        .unwrap();
    let index = state.customer_search_index();
    assert_eq!(index.search("کاغذ علي تهران 0912", 10), vec![0]);
    assert_eq!(index.search("example.test", 10), vec![0, 1]);
    assert_eq!(index.search_page("فروشگاه", 1, 1), (vec![1], 2));
    state.set_customer_archived(&first, true).unwrap();
    assert_eq!(state.customer_search_index().search("", 10), vec![1]);
    let catalog = &state.catalogs[0];
    assert_eq!(catalog.search_index().search("کالای عدد", 10), vec![0]);
    assert!(catalog.index().matches("کالای").is_empty());
    assert_eq!(catalog.index().match_count("a"), 1);
    assert!(catalog.index().matches_limited("a", 0).is_empty());
}

#[test]
fn typed_customer_auto_linking_is_idempotent_and_legacy_history_is_not_rewritten() {
    let mut state = state();
    state.draft.customer = "ساختگی".into();
    state.draft.phone = "09120000000".into();
    let first = state.approve().unwrap();
    assert!(!state.quotes[0].customer_id.is_empty());
    assert_eq!(state.approve().unwrap(), first);
    // An existing old approved offer has no optional identity field. Reopening
    // and approving unchanged content must retain that old offer byte-for-byte.
    state.quotes[0].customer_id.clear();
    let legacy = serde_json::to_value(&state.quotes[0]).unwrap();
    state.revise(&first).unwrap();
    state.draft.stale_ack = true;
    assert_eq!(state.approve().unwrap(), first);
    assert_eq!(serde_json::to_value(&state.quotes[0]).unwrap(), legacy);
    state.validate().unwrap();
}

#[test]
fn conflicting_customer_creation_is_rejected_without_silently_discarding_details() {
    let mut state = State::default();
    let customer = state
        .save_customer_details(
            "مشتری ساختگی",
            "09120000000",
            "مخاطب اصلی",
            "first@example.test",
            "نشانی اصلی",
        )
        .unwrap();
    let before = serde_json::to_value(&state).unwrap();
    assert_eq!(
        state.save_customer("مشتری ساختگی", "۰۹۱۲۰۰۰۰۰۰۰").unwrap(),
        customer
    );
    assert_eq!(
        state
            .save_customer_details(
                "مشتری ساختگی",
                "09120000000",
                "مخاطب اصلی",
                "first@example.test",
                "نشانی اصلی"
            )
            .unwrap(),
        customer
    );
    for (contact, email, address) in [
        ("مخاطب جدید", "first@example.test", "نشانی اصلی"),
        ("مخاطب اصلی", "next@example.test", "نشانی اصلی"),
        ("مخاطب اصلی", "first@example.test", "نشانی تازه"),
    ] {
        assert!(
            state
                .save_customer_details("مشتری ساختگی", "09120000000", contact, email, address)
                .is_err()
        );
        assert_eq!(serde_json::to_value(&state).unwrap(), before);
    }
}

#[test]
fn revising_after_master_customer_edits_keeps_identity_without_recreating_old_customer() {
    let mut state = state();
    let customer = state
        .save_customer_details(
            "نام قبلی",
            "09120000000",
            "مخاطب قبلی",
            "old@example.test",
            "نشانی قبلی",
        )
        .unwrap();
    state.select_customer(&customer).unwrap();
    let first = state.approve().unwrap();
    let original = serde_json::to_value(&state.quotes[0]).unwrap();
    state
        .edit_customer_details(
            &customer,
            "نام تازه",
            "09121111111",
            "مخاطب تازه",
            "new@example.test",
            "نشانی تازه",
        )
        .unwrap();
    state.set_customer_archived(&customer, true).unwrap();
    state.revise(&first).unwrap();
    state.draft.lines[0].quantity_milli = 2000;
    state.draft.stale_ack = true;
    state.approve().unwrap();
    state.validate().unwrap();
    assert_eq!(
        state.customers.len(),
        1,
        "a historical snapshot must not recreate the old master name"
    );
    assert_eq!(state.customers[0].id, customer);
    assert_eq!(state.customers[0].name, "نام تازه");
    assert!(state.customers[0].archived);
    assert_eq!(state.quotes[1].customer_id, customer);
    assert_eq!(state.quotes[1].customer, "نام قبلی");
    assert_eq!(state.quotes[1].customer_contact, "مخاطب قبلی");
    assert_eq!(state.quotes[1].customer_email, "old@example.test");
    assert_eq!(state.quotes[1].customer_address, "نشانی قبلی");
    assert_eq!(serde_json::to_value(&state.quotes[0]).unwrap(), original);
}
