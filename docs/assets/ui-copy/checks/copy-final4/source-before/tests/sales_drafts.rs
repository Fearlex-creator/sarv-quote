use sarv_quote::{
    accounting::{Money, MoneyUnit},
    domain::{CURRENT_SCHEMA, DateDisplay, State},
    imports::{accept_catalog, catalog_csv},
    sales::SalesCommand,
    sales_draft::{RawSalesDraft, RawSalesLine},
    store::Store,
};
fn base() -> State {
    let envelope: serde_json::Value = serde_json::from_str(include_str!(
        "../docs/assets/accounting-m3/restore/core-v4.json"
    ))
    .unwrap();
    let mut state: State = serde_json::from_value(envelope["state"].clone()).unwrap();
    state.schema = CURRENT_SCHEMA;
    state.validate().unwrap();
    state
}
fn draft(s: &State) -> RawSalesDraft {
    let o = &s.accounting.core.openings[0].command;
    RawSalesDraft {
        id: "raw-sales-draft-test".into(),
        revision: 0,
        company_id: o.company_id.clone(),
        period_id: o.period_id.clone(),
        setup_id: o.setup_id.clone(),
        customer_id: "CU-A".into(),
        reference: "synthetic-order".into(),
        document_date: "2026-10-08".into(),
        financial_date: "2026-10-08".into(),
        date_display: DateDisplay::Iso,
        display_unit: MoneyUnit::Rial,
        document_discount: "1".into(),
        supplied_total: "199".into(),
        reason: "synthetic durable editor test".into(),
        quotation: None,
        lines: vec![RawSalesLine {
            id: "raw-sales-line-test".into(),
            item_id: "ITEM-A".into(),
            warehouse_id: "W1".into(),
            unit_id: "PIECE".into(),
            quantity: "10".into(),
            unit_price: "20".into(),
            line_discount: "0".into(),
            price_reference: "synthetic-price".into(),
            price_date: "2026-10-08".into(),
            override_reason: String::new(),
        }],
    }
}
#[test]
fn incomplete_raw_input_survives_restart_backup_and_failed_post_unchanged() {
    let profile = tempfile::tempdir().unwrap();
    let mut st = Store::open(profile.path()).unwrap();
    st.change(|s| {
        *s = base();
        Ok(())
    })
    .unwrap();
    let mut d = draft(&st.state);
    d.lines[0].unit_price = "۱۲٫".into();
    d.financial_date = "not a date".into();
    st.change(|s| {
        s.save_sales_draft(d.clone(), None)?;
        Ok(())
    })
    .unwrap();
    let before = serde_json::to_vec(&st.state).unwrap();
    assert!(
        st.change(|s| {
            s.post_sales_from_draft(&d.id, 1, "failed-key", "tester")?;
            Ok(())
        })
        .is_err()
    );
    assert_eq!(serde_json::to_vec(&st.state).unwrap(), before);
    let backup = profile.path().join("raw-draft-v6.json");
    st.backup(&backup).unwrap();
    let restored = Store::restore_preview(&backup).unwrap();
    assert_eq!(restored.sales_drafts[0].lines[0].unit_price, "۱۲٫");
    drop(st);
    let st = Store::open(profile.path()).unwrap();
    assert_eq!(serde_json::to_vec(&st.state).unwrap(), before);
}
#[test]
fn draft_revision_cas_noop_atomic_submit_and_lost_receipt_retries_after_lock() {
    let profile = tempfile::tempdir().unwrap();
    let mut st = Store::open(profile.path()).unwrap();
    st.change(|s| {
        *s = base();
        Ok(())
    })
    .unwrap();
    let d = draft(&st.state);
    st.change(|s| {
        assert_eq!(s.save_sales_draft(d.clone(), None)?, 1);
        Ok(())
    })
    .unwrap();
    let mut updated = st.state.sales_drafts[0].clone();
    assert_eq!(
        st.state.save_sales_draft(updated.clone(), Some(1)).unwrap(),
        1
    );
    updated.reason = "new synthetic review reason".into();
    st.change(|s| {
        assert_eq!(s.save_sales_draft(updated.clone(), Some(1))?, 2);
        Ok(())
    })
    .unwrap();
    let before = serde_json::to_vec(&st.state).unwrap();
    assert!(
        st.change(|s| {
            s.save_sales_draft(updated.clone(), Some(1))?;
            Ok(())
        })
        .is_err()
    );
    assert_eq!(serde_json::to_vec(&st.state).unwrap(), before);
    let mut stale = Store::open(profile.path()).unwrap();
    let result = st
        .change(|s| s.post_sales_from_draft(&d.id, 2, "submit-key", "tester"))
        .unwrap();
    assert!(st.state.sales_drafts.is_empty());
    assert!(result.journal_id.is_none());
    assert!(
        stale
            .change(|s| s.post_sales_from_draft(&d.id, 2, "other-key", "tester"))
            .is_err()
    );
    let order_count = st.state.accounting.sales.records.len();
    let audit_count = st.state.accounting.audit.len();
    let period = st.state.accounting.core.openings[0]
        .command
        .period_id
        .clone();
    st.change(|s| {
        s.accounting
            .set_period_lock(&period, true, "tester", "synthetic lock")
    })
    .unwrap();
    let locked_audit = st.state.accounting.audit.len();
    assert!(locked_audit > audit_count);
    drop(st);
    let mut st = Store::open(profile.path()).unwrap();
    assert_eq!(
        st.change(|s| s.post_sales_from_draft(&d.id, 2, "submit-key", "tester"))
            .unwrap(),
        result
    );
    assert_eq!(
        st.change(|s| s.post_sales_from_draft(&d.id, 2, "retry-new-key", "tester"))
            .unwrap(),
        result
    );
    assert_eq!(st.state.accounting.sales.records.len(), order_count);
    assert_eq!(st.state.accounting.audit.len(), locked_audit);
    assert!(
        st.change(|s| s.post_sales_from_draft(&d.id, 2, "bad-actor", "other"))
            .is_err()
    );
    st.state.validate().unwrap();
}
fn approved_quote() -> State {
    let mut s = base();
    let at = chrono::Utc::now().format("%Y-%m-%dT00:00:00Z").to_string();
    let cat = catalog_csv(
        &format!(
            "code,name,measure,price,currency,source_at,aliases\nSKU-A,کالای الف,عدد,20,rial,{at},"
        ),
        "synthetic conversion",
    )
    .unwrap();
    accept_catalog(&mut s, cat).unwrap();
    s.draft.customer = "مشتری الف".into();
    s.draft.lines = vec![sarv_quote::domain::OrderLine {
        query: "SKU-A".into(),
        quantity_milli: 10_000,
        chosen_code: None,
    }];
    s.draft.discount_rial = 1;
    s.draft.quote_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    s.approve().unwrap();
    s.validate().unwrap();
    s
}
#[test]
fn authentic_quote_conversion_preserves_full_snapshot_and_rejects_tax_or_any_changed_source() {
    let mut s = approved_quote();
    let q = s.quotes[0].clone();
    let company = s.accounting.core.openings[0].command.company_id.clone();
    let mut d = s.sales_draft_from_quote(&q.id, &company).unwrap();
    assert!(d.lines[0].warehouse_id.is_empty());
    d.lines[0].warehouse_id = "W1".into();
    let id = d.id.clone();
    s.save_sales_draft(d, None).unwrap();
    let order = s
        .sales_order_from_draft(&id, 1, "quote-convert", "tester")
        .unwrap();
    assert_eq!(order.supplied_total, Money::from_rials(199));
    let before = serde_json::to_vec(&s).unwrap();
    for mutation in 0..5 {
        let mut bad = order.clone();
        match mutation {
            0 => bad.lines[0].unit_price = Money::from_rials(21),
            1 => bad.lines[0].quantity_thousandths = 9000,
            2 => bad.lines[0].price_reference = "invented catalog".into(),
            3 => bad.quotation.as_mut().unwrap().source_line_indices[0] = 1,
            _ => bad.document_discount = Money::ZERO,
        };
        assert!(s.validate_sales_quotation(&bad).is_err());
    }
    assert_eq!(serde_json::to_vec(&s).unwrap(), before);
    s.post_sales_from_draft(&id, 1, "quote-convert", "tester")
        .unwrap();
    assert_eq!(
        serde_json::to_vec(&s.quotes[0]).unwrap(),
        serde_json::to_vec(&q).unwrap()
    );
    for mutation in 0..4 {
        let mut bad = s.clone();
        match mutation {
            0 => bad.quotes[0].notes = "changed immutable note".into(),
            1 => bad.quotes[0].customer_email = "changed@example.test".into(),
            2 => bad.quotes[0].terms = "changed terms".into(),
            _ => bad.quotes.clear(),
        };
        assert!(bad.validate().is_err());
    }
    let mut taxed = approved_quote();
    taxed.draft.tax_basis_points = 100;
    taxed.approve().unwrap();
    let taxed_q = taxed.quotes.last().unwrap();
    assert!(taxed.sales_draft_from_quote(&taxed_q.id, &company).is_err());
    // The lower-level ledger cannot confer quotation authority: whole-State validation rejects a forged link.
    let mut forged = approved_quote();
    let mut bad = order.clone();
    bad.context.source_id = "fabricated-linked-order".into();
    bad.context.request_key = "fabricated-key".into();
    bad.quotation.as_mut().unwrap().quote_sha256 = "0".repeat(64);
    forged
        .accounting
        .post_sales_command(SalesCommand::Order(bad))
        .unwrap();
    assert!(forged.validate().is_err());
}
#[test]
fn raw_jalali_dates_exact_toman_conversion_and_fractional_rial_rejection() {
    let mut s = base();
    let mut d = draft(&s);
    d.date_display = DateDisplay::Jalali;
    d.document_date = "۱۴۰۵/۰۷/۱۶".into();
    d.financial_date = d.document_date.clone();
    d.lines[0].price_date = d.document_date.clone();
    d.display_unit = MoneyUnit::Toman;
    d.lines[0].unit_price = "۲".into();
    d.document_discount = "۰٫۱".into();
    d.supplied_total = "۱۹٫۹".into();
    let id = d.id.clone();
    s.save_sales_draft(d, None).unwrap();
    let order = s
        .sales_order_from_draft(&id, 1, "jalali-key", "tester")
        .unwrap();
    assert_eq!(order.context.date, "2026-10-08");
    assert_eq!(order.supplied_total.rials(), 199);
    let mut d = s.sales_drafts[0].clone();
    d.lines[0].unit_price = "۲٫۰۱".into();
    s.save_sales_draft(d, Some(1)).unwrap();
    assert!(
        s.sales_order_from_draft(&id, 2, "fractional-key", "tester")
            .is_err()
    );
}

#[test]
fn day_month_year_raw_draft_and_quote_conversion_keep_explicit_date_semantics() {
    let mut s = base();
    let mut d = draft(&s);
    d.date_display = DateDisplay::DayMonthYear;
    d.document_date = "08/10/2026".into();
    d.financial_date = d.document_date.clone();
    d.lines[0].price_date = d.document_date.clone();
    let id = d.id.clone();
    s.save_sales_draft(d, None).unwrap();
    let order = s
        .sales_order_from_draft(&id, 1, "dmy-key", "tester")
        .unwrap();
    assert_eq!(order.context.date, "2026-10-08");
    assert_eq!(order.document_date, "2026-10-08");
    let mut s = approved_quote();
    s.draft.date_display = DateDisplay::DayMonthYear;
    s.draft.notes = "explicit DMY quotation revision".into();
    let quote_id = s.approve().unwrap();
    let company = s.accounting.core.openings[0].command.company_id.clone();
    let mut d = s.sales_draft_from_quote(&quote_id, &company).unwrap();
    assert_eq!(d.date_display, DateDisplay::DayMonthYear);
    d.lines[0].warehouse_id = "W1".into();
    let id = d.id.clone();
    s.save_sales_draft(d, None).unwrap();
    let order = s
        .sales_order_from_draft(&id, 1, "dmy-quote-key", "tester")
        .unwrap();
    assert_eq!(order.document_date, s.quotes.last().unwrap().quote_date);
}
