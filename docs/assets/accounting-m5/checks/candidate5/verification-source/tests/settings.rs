use chrono::{Duration, Utc};
use sarv_quote::{domain::*, imports::*, store::Store};

fn prepared() -> State {
    let mut state = State::default();
    let source = (Utc::now() - Duration::hours(1)).to_rfc3339();
    let csv = format!(
        "code,name,measure,price,currency,source_at,aliases\nA,نمونه,عدد,1001,rial,{source},\n"
    );
    accept_catalog(&mut state, catalog_csv(&csv, "نمایشی").unwrap()).unwrap();
    state.draft.customer = "مشتری".into();
    state.draft.lines = order_csv("item,quantity\nA,1\n").unwrap();
    state
}
fn fill(state: &mut State) {
    state.draft.customer = "مشتری".into();
    state.draft.lines = order_csv("item,quantity\nA,1\n").unwrap();
}
#[test]
fn defaults_are_real_new_draft_snapshots() {
    let mut state = prepared();
    let old = state.draft.clone();
    state.settings = Settings {
        business_name: "بازرگانی نمونه".into(),
        business_phone: "۱۲۳۴".into(),
        business_address: "خیابان اول\nپلاک دوم".into(),
        header: "پیشنهاد فروش".into(),
        footer: "با سپاس".into(),
        default_terms: "اعتبار هفت روز".into(),
        default_currency: Unit::Rial,
        quote_prefix: "SALE-".into(),
        next_number: 42,
        default_validity_days: 14,
        default_discount_rial: 1,
        tax_basis_points: 975,
        appearance: Appearance::Dark,
        density: Density::Compact,
        display_digits: DisplayDigits::Latin,
        date_display: DateDisplay::DayMonthYear,
    };
    assert_eq!(state.draft.issuer_name, old.issuer_name);
    assert_eq!(state.draft.display_unit, old.display_unit);
    state.new_draft().unwrap();
    assert_eq!(state.draft.catalog_id, old.catalog_id);
    assert_eq!(state.draft.display_unit, Unit::Rial);
    assert_eq!(state.draft.discount_rial, 1);
    assert_eq!(state.draft.issuer_name, "بازرگانی نمونه");
    assert_eq!(state.draft.issuer_phone, "۱۲۳۴");
    assert_eq!(state.draft.issuer_address, "خیابان اول\nپلاک دوم");
    assert_eq!(state.draft.header, "پیشنهاد فروش");
    assert_eq!(state.draft.footer, "با سپاس");
    assert_eq!(state.draft.terms, "اعتبار هفت روز");
    assert_eq!(state.draft.tax_basis_points, 975);
    assert_eq!(state.draft.display_digits, DisplayDigits::Latin);
    assert_eq!(state.draft.date_display, DateDisplay::DayMonthYear);
    assert_eq!(
        state.draft.valid_until,
        (Utc::now().date_naive() + Duration::days(14)).to_string()
    );
    fill(&mut state);
    let preview = state.preview().unwrap();
    assert_eq!(preview.number, "SALE-42");
    assert_eq!(preview.tax_rial, 98); // 1000 rial net * 9.75%, half-up.
    assert_eq!(preview.total_rial, 1098);
    state.approve().unwrap();
    state.validate().unwrap();
}
#[test]
fn numbers_survive_revision_and_failed_approval_never_consumes_number() {
    let mut state = prepared();
    let first = state.approve().unwrap();
    assert_eq!(state.quotes[0].number, "Q-1");
    assert_eq!(state.settings.next_number, 2);
    assert_eq!(state.approve().unwrap(), first);
    assert_eq!(state.settings.next_number, 2);
    state.settings.quote_prefix = "NEW-".into();
    state.settings.business_name = "نام تازه".into();
    state.settings.tax_basis_points = 10_000;
    state.settings.date_display = DateDisplay::DayMonthYear;
    state.revise(&first).unwrap();
    assert_eq!(state.approve().unwrap(), first);
    state.draft.notes = "اصلاح".into();
    state.approve().unwrap();
    assert_eq!(state.quotes[1].number, "Q-1");
    assert_eq!(state.quotes[1].issuer_name, "");
    assert_eq!(state.quotes[1].tax_rial, 0);
    assert_eq!(state.quotes[1].date_display, DateDisplay::Iso);
    assert_eq!(state.settings.next_number, 2);
    state.new_draft().unwrap();
    assert!(state.approve().is_err());
    assert_eq!(state.settings.next_number, 2);
    fill(&mut state);
    state.approve().unwrap();
    assert_eq!(state.quotes[2].number, "NEW-2");
    assert_eq!(state.quotes[2].tax_rial, 1001);
    assert_eq!(state.quotes[2].date_display, DateDisplay::DayMonthYear);
    assert_eq!(state.settings.next_number, 3);
    state.validate().unwrap();
}
#[test]
fn repeated_number_and_counter_exhaustion_fail_safely() {
    let mut state = prepared();
    state.approve().unwrap();
    state.new_draft().unwrap();
    fill(&mut state);
    state.settings.next_number = 1;
    assert!(state.approve().is_err());
    assert_eq!(state.settings.next_number, 1);
    assert_eq!(state.quotes.len(), 1);
    state.settings.next_number = u64::MAX - 1;
    assert!(state.approve().is_err());
    assert_eq!(state.settings.next_number, u64::MAX - 1);
    assert_eq!(state.quotes.len(), 1);
}
#[test]
fn configured_tax_is_exact_and_quote_tampering_is_rejected() {
    assert_eq!(tax_amount(1, 5000).unwrap(), 1);
    assert_eq!(tax_amount(1, 4999).unwrap(), 0);
    assert_eq!(tax_amount(0, 10_000).unwrap(), 0);
    assert_eq!(tax_amount(MAX_MONEY, 10_000).unwrap(), MAX_MONEY);
    assert!(tax_amount(-1, 100).is_err());
    assert!(tax_amount(1, 10_001).is_err());
    let mut state = prepared();
    state.settings.tax_basis_points = 333;
    state.settings.default_discount_rial = 101;
    state.new_draft().unwrap();
    fill(&mut state);
    state.approve().unwrap();
    assert_eq!(state.quotes[0].tax_rial, 30);
    assert_eq!(state.quotes[0].total_rial, 930);
    state.validate().unwrap();
    state.quotes[0].tax_rial += 1;
    state.quotes[0].total_rial += 1;
    assert!(state.validate().is_err());
    state.quotes[0].tax_rial -= 1;
    state.quotes[0].total_rial -= 1;
    state.draft.lines[0].quantity_milli = 1_000_000_000;
    state.catalogs[0].products[0].price_rial = MAX_MONEY;
    assert!(state.approve().is_err());
}
#[test]
fn every_setting_persists_and_backup_restore_keeps_quote_snapshots() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    store
        .change(|state| {
            *state = prepared();
            state.settings.business_name = "شرکت نمونه".into();
            state.settings.business_phone = "123".into();
            state.settings.business_address = "نشانی".into();
            state.settings.header = "سربرگ".into();
            state.settings.footer = "پایان".into();
            state.settings.default_terms = "شرایط".into();
            state.settings.default_currency = Unit::Rial;
            state.settings.quote_prefix = "S-".into();
            state.settings.next_number = 12;
            state.settings.default_validity_days = 30;
            state.settings.default_discount_rial = 101;
            state.settings.tax_basis_points = 1250;
            state.settings.appearance = Appearance::Dark;
            state.settings.density = Density::Compact;
            state.settings.display_digits = DisplayDigits::Latin;
            state.settings.date_display = DateDisplay::DayMonthYear;
            state.new_draft()?;
            fill(state);
            state.approve()?;
            Ok(())
        })
        .unwrap();
    let settings = store.state.settings.clone();
    let quote = serde_json::to_vec(&store.state.quotes[0]).unwrap();
    let backup = dir.path().join("settings-backup.json");
    store.backup(&backup).unwrap();
    drop(store);
    let reopened = Store::open(dir.path()).unwrap();
    assert_eq!(reopened.state.settings, settings);
    assert_eq!(
        serde_json::to_vec(&reopened.state.quotes[0]).unwrap(),
        quote
    );
    let restored = Store::restore_preview(&backup).unwrap();
    assert_eq!(restored.settings, settings);
    assert_eq!(serde_json::to_vec(&restored.quotes[0]).unwrap(), quote);
}
#[test]
fn legacy_settings_and_snapshot_defaults_are_omitted() {
    let state = State::default();
    let serialized = serde_json::to_value(&state).unwrap();
    assert!(serialized.get("settings").is_none());
    let settings = Settings {
        next_number: 2,
        ..Settings::default()
    };
    let settings_json = serde_json::to_value(&settings).unwrap();
    assert!(settings_json.get("date_display").is_none());
    let decoded: Settings = serde_json::from_value(settings_json).unwrap();
    assert_eq!(decoded.date_display, DateDisplay::Iso);
    let mut state = prepared();
    state.approve().unwrap();
    let mut legacy = serde_json::to_value(state).unwrap();
    legacy.as_object_mut().unwrap().remove("settings");
    for field in [
        "number",
        "tax_rial",
        "tax_basis_points",
        "display_digits",
        "date_display",
        "issuer_name",
        "issuer_phone",
        "issuer_address",
        "header",
        "footer",
        "terms",
    ] {
        legacy["quotes"][0].as_object_mut().unwrap().remove(field);
        legacy["draft"].as_object_mut().unwrap().remove(field);
    }
    let restored: State = serde_json::from_value(legacy).unwrap();
    restored.validate().unwrap();
    assert!(restored.settings.is_default());
    assert_eq!(restored.quotes[0].tax_rial, 0);
    assert_eq!(restored.quotes[0].date_display, DateDisplay::Iso);
    assert!(restored.quotes[0].number.is_empty());
    let bytes = serde_json::to_vec(&restored).unwrap();
    let again: State = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(serde_json::to_vec(&again).unwrap(), bytes);
}
#[test]
fn invalid_settings_and_digit_preferences_have_meaningful_behavior() {
    let settings = Settings::default();
    settings.validate().unwrap();
    let mut invalid = settings.clone();
    invalid.tax_basis_points = 10_001;
    assert!(invalid.validate().is_err());
    invalid = settings.clone();
    invalid.next_number = 0;
    assert!(invalid.validate().is_err());
    invalid = settings.clone();
    invalid.default_validity_days = 3651;
    assert!(invalid.validate().is_err());
    invalid = settings.clone();
    invalid.default_discount_rial = -1;
    assert!(invalid.validate().is_err());
    invalid = settings.clone();
    invalid.business_name = "x\u{202e}".into();
    assert!(invalid.validate().is_err());
    invalid = settings;
    invalid.default_terms = "x".repeat(4001);
    assert!(invalid.validate().is_err());
    assert_eq!(DisplayDigits::Persian.format("123.4"), "۱۲۳.۴");
    assert_eq!(DisplayDigits::Latin.format("۱۲۳٫۴"), "123.4");
    assert_eq!(DateDisplay::Iso.format("2026-10-07"), "2026-10-07");
    assert_eq!(DateDisplay::DayMonthYear.format("2026-10-07"), "07/10/2026");
    assert_eq!(
        DateDisplay::DayMonthYear.format("2024-02-29T08:03:02.123+03:30"),
        "29/02/2024T08:03:02.123+03:30"
    );
    assert_eq!(
        DisplayDigits::Persian.format(DateDisplay::DayMonthYear.format("2026-10-07")),
        "۰۷/۱۰/۲۰۲۶"
    );
    for invalid in [
        "",
        "2026-02-29",
        "2026-10-07junk",
        "2026-10-07T99:00:00Z",
        "نامشخص",
    ] {
        assert_eq!(DateDisplay::DayMonthYear.format(invalid), invalid);
    }
}

#[test]
fn tax_total_overflow_and_equivalent_document_numbers_are_rejected() {
    let mut state = prepared();
    state.catalogs[0].products[0].price_rial = MAX_MONEY;
    state.catalogs[0].fingerprint = product_fingerprint(&state.catalogs[0].products).unwrap();
    state.draft.tax_basis_points = 1;
    assert!(state.approve().is_err());
    assert_eq!(state.settings.next_number, 1);
    assert!(state.quotes.is_empty());
    state.draft.tax_basis_points = 0;
    state.approve().unwrap();
    assert_eq!(state.quotes[0].total_rial, MAX_MONEY);
    state.new_draft().unwrap();
    fill(&mut state);
    // Different glyphs for the prefix cannot evade visible document-number uniqueness.
    state.settings.quote_prefix = "q-".into();
    state.settings.next_number = 1;
    assert!(state.approve().is_err());
    assert_eq!(state.quotes.len(), 1);
}
