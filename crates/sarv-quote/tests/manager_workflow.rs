use sarv_quote::{domain::*, export, imports::*, store::Store};

fn catalog(name: &str, price: i64) -> Catalog {
    catalog_csv(&format!("code,name,measure,price,currency,source_at,aliases\nSKU-۱,کاغذ,بسته,{price},rial,2020-01-01T00:00:00Z,كاغذ آ۴"), name).unwrap()
}
fn prepared() -> State {
    let mut state = State::default();
    accept_catalog(&mut state, catalog("مصنوعی", 1250)).unwrap();
    state.draft.customer = "مشتری نمایشی".into();
    state.draft.phone = "09120000000".into();
    state.draft.lines = order_csv("item,quantity\nSKU-1,2").unwrap();
    state.draft.stale_ack = true;
    state.draft.reference = "ORDER-۱۲۳".into();
    state.draft.notes = "یادداشت نمایشی".into();
    state
}
fn json(state: &State) -> Vec<u8> {
    serde_json::to_vec(state).unwrap()
}

#[test]
fn customer_save_is_idempotent_across_script_and_whitespace() {
    let mut state = State::default();
    let first = state.save_customer("  شركت ياس ", " ٠٩١٢٣ ").unwrap();
    assert_eq!(state.save_customer("شرکت یاس", "۰۹۱۲۳").unwrap(), first);
    assert_eq!(state.save_customer(" شركت ياس ", " 09123 ").unwrap(), first);
    assert_eq!(state.customers.len(), 1);
    assert_eq!(state.customers[0].phone, "09123");
    state.validate().unwrap();
}

#[test]
fn customer_edit_and_use_leave_approved_snapshot_unchanged() {
    let mut state = prepared();
    let approved = state.approve().unwrap();
    let archive = serde_json::to_vec(&state.quotes).unwrap();
    let customer = state.customers[0].id.clone();
    state
        .edit_customer(&customer, "نام تازه", "۰۹۳۵۰۰۰۰۰۰۰")
        .unwrap();
    assert_eq!(state.draft.customer, "مشتری نمایشی");
    let series = state.draft.series.clone();
    let catalog = state.draft.catalog_id.clone();
    let reference = state.draft.reference.clone();
    state.use_customer(&customer).unwrap();
    assert_eq!(state.draft.customer, "نام تازه");
    assert_eq!(state.draft.phone, "09350000000");
    assert_eq!(state.draft.series, series);
    assert_eq!(state.draft.catalog_id, catalog);
    assert_eq!(state.draft.reference, reference);
    assert_eq!(serde_json::to_vec(&state.quotes).unwrap(), archive);
    assert_eq!(state.quotes[0].id, approved);
    state.validate().unwrap();
}

#[test]
fn rejected_manager_actions_are_atomic() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    store
        .change(|state| {
            *state = prepared();
            state.approve()?;
            state.save_customer("دیگر", "123")?;
            Ok(())
        })
        .unwrap();
    let before = json(&store.state);
    let customer = store.state.customers[0].id.clone();
    assert!(
        store
            .change(|state| state.edit_customer(&customer, "دیگر", "١٢٣"))
            .is_err()
    );
    assert!(
        store
            .change(|state| state.edit_customer(&customer, "bad\u{202e}", "123"))
            .is_err()
    );
    assert!(store.change(|state| state.use_customer("missing")).is_err());
    assert!(
        store
            .change(|state| state.select_catalog("missing"))
            .is_err()
    );
    assert!(
        store
            .change(|state| state.duplicate_quote("missing"))
            .is_err()
    );
    assert_eq!(json(&store.state), before);
    assert_eq!(json(&Store::open(dir.path()).unwrap().state), before);
}

#[test]
fn catalog_selection_resets_only_when_source_changes() {
    let mut state = prepared();
    let first_catalog = state.draft.catalog_id.clone();
    state.draft.lines[0].chosen_code = Some("SKU-1".into());
    let before = json(&state);
    state.select_catalog(&first_catalog).unwrap();
    assert_eq!(json(&state), before);
    let other = catalog("قیمت تازه", 1350);
    let next_catalog = other.id.clone();
    state.catalogs.push(other);
    state.select_catalog(&next_catalog).unwrap();
    assert_eq!(state.draft.catalog_id, next_catalog);
    assert!(state.draft.lines[0].chosen_code.is_none());
    assert!(!state.draft.stale_ack);
    assert_eq!(state.draft.customer, "مشتری نمایشی");
    assert_eq!(state.draft.reference, "ORDER-۱۲۳");
    state.validate().unwrap();
}

#[test]
fn revision_keeps_number_duplicate_gets_new_series_and_number() {
    let mut state = prepared();
    let original = state.approve().unwrap();
    let original_quote = state.quotes[0].clone();
    state.revise(&original).unwrap();
    state.draft.lines[0].quantity_milli = 3000;
    state.draft.stale_ack = true;
    let revised = state.approve().unwrap();
    assert_ne!(revised, original);
    assert_eq!(state.quotes[1].series, original_quote.series);
    assert_eq!(state.quotes[1].number, original_quote.number);
    assert_eq!(state.quotes[1].version, 2);
    let archive = serde_json::to_vec(&state.quotes).unwrap();
    let changed_price = catalog("امروز", 9999);
    accept_catalog(&mut state, changed_price).unwrap();
    state.duplicate_quote(&original).unwrap();
    assert_ne!(state.draft.series, original_quote.series);
    assert_eq!(state.draft.catalog_id, original_quote.catalog_id);
    assert_eq!(state.draft.lines[0].quantity_milli, 2000);
    assert_eq!(state.draft.notes, original_quote.notes);
    assert!(!state.draft.stale_ack);
    assert_eq!(serde_json::to_vec(&state.quotes).unwrap(), archive);
    assert!(state.approve().is_err());
    state.draft.stale_ack = true;
    let duplicated = state.approve().unwrap();
    assert_ne!(duplicated, original);
    assert_eq!(state.quotes[2].version, 1);
    assert_ne!(state.quotes[2].number, original_quote.number);
    assert_eq!(state.quotes[2].lines[0].price_rial, 1250);
    assert_eq!(state.approve().unwrap(), duplicated);
    assert_eq!(state.quotes.len(), 3);
    state.validate().unwrap();
}

#[test]
fn duplicate_clears_expired_validity_and_updates_date() {
    let mut state = prepared();
    state.draft.quote_date = "2020-01-01".into();
    state.draft.valid_until = "2020-01-03".into();
    let original = state.approve().unwrap();
    state.duplicate_quote(&original).unwrap();
    assert_eq!(
        state.draft.quote_date,
        chrono::Utc::now().format("%Y-%m-%d").to_string()
    );
    assert!(state.draft.valid_until.is_empty());
    assert_eq!(state.quotes[0].quote_date, "2020-01-01");
    assert_eq!(state.quotes[0].valid_until, "2020-01-03");
}

#[cfg(feature = "pdf-export")]
#[test]
fn managed_data_restarts_backs_up_and_exports_the_archived_quote() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let quote_id = store
        .change(|state| {
            *state = prepared();
            state.approve()
        })
        .unwrap();
    let customer = store.state.customers[0].id.clone();
    store
        .change(|state| state.edit_customer(&customer, "مشتری ویرایش‌شده", "555"))
        .unwrap();
    let backup = dir.path().join("manager-backup.json");
    store.backup(&backup).unwrap();
    let approved = store.state.quotes[0].clone();
    drop(store);
    let reopened = Store::open(dir.path()).unwrap();
    assert_eq!(reopened.state.customers[0].name, "مشتری ویرایش‌شده");
    assert_eq!(reopened.state.quotes[0].customer, "مشتری نمایشی");
    assert_eq!(reopened.state.quotes[0].id, quote_id);
    assert_eq!(
        json(&Store::restore_preview(&backup).unwrap()),
        json(&reopened.state)
    );
    let csv = dir.path().join("archive.csv");
    let pdf = dir.path().join("archive.pdf");
    export::csv(&approved, &csv).unwrap();
    export::pdf(&approved, &pdf).unwrap();
    assert!(std::fs::read(&pdf).unwrap().starts_with(b"%PDF"));
    assert!(
        String::from_utf8(std::fs::read(&csv).unwrap())
            .unwrap()
            .contains("مشتری نمایشی")
    );
    let previews = export::png_pages(&approved, dir.path()).unwrap();
    assert!(!previews.is_empty());
    for preview in previews {
        assert!(
            std::fs::read(preview)
                .unwrap()
                .starts_with(b"\x89PNG\r\n\x1a\n")
        );
    }
}
