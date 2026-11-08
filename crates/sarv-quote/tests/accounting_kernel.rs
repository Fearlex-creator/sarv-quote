use sarv_quote::accounting::*;

fn account(
    state: &mut AccountingState,
    company: &str,
    code: &str,
    class: AccountClass,
    side: BalanceSide,
) -> String {
    state
        .add_account(
            NewAccount {
                company_id: company.into(),
                parent_id: None,
                code: code.into(),
                name: format!("حساب {code}"),
                classification: class,
                normal_balance: side,
                postable: true,
                control: false,
            },
            "آزمایشگر محلی",
        )
        .unwrap()
}
fn fixture() -> (AccountingState, String, String, String, String) {
    let mut state = AccountingState::default();
    let company = state
        .add_company("کسب‌وکار مصنوعی", "آزمایشگر محلی")
        .unwrap();
    let period = state
        .add_period(
            &company,
            "دورهٔ مصنوعی",
            "2026-01-01",
            "2026-12-31",
            "آزمایشگر محلی",
        )
        .unwrap();
    let asset = account(
        &mut state,
        &company,
        "1101",
        AccountClass::Asset,
        BalanceSide::Debit,
    );
    let equity = account(
        &mut state,
        &company,
        "3101",
        AccountClass::Equity,
        BalanceSide::Credit,
    );
    (state, company, period, asset, equity)
}
fn command(company: &str, period: &str, asset: &str, equity: &str, amount: i64) -> PostCommand {
    PostCommand {
        company_id: company.into(),
        period_id: period.into(),
        date: "2026-01-01".into(),
        source_id: "opening-fixture".into(),
        source_revision: 1,
        event: "opening".into(),
        request_key: "request-a".into(),
        actor: "آزمایشگر محلی".into(),
        reason: "مانده‌های مصنوعی مستقل".into(),
        kind: JournalKind::Opening,
        lines: vec![
            JournalLineInput {
                account_id: asset.into(),
                debit: Money::from_rials(amount),
                credit: Money::ZERO,
                description: "دارایی مصنوعی".into(),

                dimension: String::new(),
            },
            JournalLineInput {
                account_id: equity.into(),
                debit: Money::ZERO,
                credit: Money::from_rials(amount),
                description: "سرمایهٔ مصنوعی".into(),

                dimension: String::new(),
            },
        ],
    }
}
#[test]
fn exact_money_units_digits_range_and_serialization() {
    assert_eq!(
        Money::parse("۱۲۵.۱", MoneyUnit::Toman).unwrap().rials(),
        1251
    );
    assert_eq!(Money::parse("١٢٥١", MoneyUnit::Rial).unwrap().rials(), 1251);
    assert_eq!(Money::parse("-0.1", MoneyUnit::Toman).unwrap().rials(), -1);
    assert_eq!(
        Money::parse("1.1000", MoneyUnit::Toman).unwrap().rials(),
        11
    );
    assert!(Money::parse("1.01", MoneyUnit::Toman).is_err());
    assert!(Money::parse("1.1", MoneyUnit::Rial).is_err());
    for bad in [
        "NaN",
        "1e3",
        "1,000",
        ".1",
        "1.",
        "1.2.3",
        "--1",
        "+1",
        "۱٬۰۰۰",
        "",
    ] {
        assert!(Money::parse(bad, MoneyUnit::Rial).is_err(), "{bad}");
    }
    let min = Money::parse("-9223372036854775808", MoneyUnit::Rial).unwrap();
    assert_eq!(min.rials(), i64::MIN);
    assert_eq!(min.display(MoneyUnit::Toman), "-922337203685477580.8");
    assert!(min.checked_sub(Money::from_rials(1)).is_err());
    assert!(
        Money::from_rials(i64::MAX)
            .checked_add(Money::from_rials(1))
            .is_err()
    );
    assert!(Money::parse("9223372036854775808", MoneyUnit::Rial).is_err());
    assert!(Money::parse("922337203685477580.8", MoneyUnit::Toman).is_err());
    assert_eq!(
        serde_json::to_string(&Money::from_rials(1251)).unwrap(),
        "1251"
    );
    assert!(serde_json::from_str::<Money>("1251.0").is_err());
    for rials in [-1251, -10, -1, 0, 1, 10, 1251, i64::MIN, i64::MAX] {
        let money = Money::from_rials(rials);
        assert_eq!(
            Money::parse(&money.display(MoneyUnit::Toman), MoneyUnit::Toman).unwrap(),
            money
        );
    }
}
#[test]
fn independent_opening_adjustment_and_reversal_fixture() {
    let (mut s, c, p, a, e) = fixture();
    let opening = command(&c, &p, &a, &e, 1251);
    let id = s.post(opening).unwrap();
    let tb = s.trial_balance(&c, "2026-01-01").unwrap();
    assert_eq!(tb.total_debit.rials(), 1251);
    assert_eq!(tb.total_credit.rials(), 1251);
    assert_eq!(
        tb.rows
            .iter()
            .find(|r| r.account_id == a)
            .unwrap()
            .debit
            .rials(),
        1251
    );
    assert_eq!(
        tb.rows
            .iter()
            .find(|r| r.account_id == e)
            .unwrap()
            .credit
            .rials(),
        1251
    );
    let mut adjustment = command(&c, &p, &a, &e, 349);
    adjustment.kind = JournalKind::Manual;
    adjustment.source_id = "manual-fixture".into();
    adjustment.event = "adjustment".into();
    adjustment.request_key = "manual-key".into();
    adjustment.date = "2026-02-01".into();
    let adjustment_id = s.post(adjustment).unwrap();
    assert_eq!(
        s.trial_balance(&c, "2026-02-01")
            .unwrap()
            .total_debit
            .rials(),
        1600
    );
    let reverse_id = s
        .reverse(
            &adjustment_id,
            &p,
            "2026-03-01",
            "reverse-key",
            "آزمایشگر محلی",
            "اصلاح آزمون مصنوعی",
        )
        .unwrap();
    assert_ne!(id, reverse_id);
    assert_eq!(
        s.trial_balance(&c, "2026-02-28")
            .unwrap()
            .total_debit
            .rials(),
        1600
    );
    assert_eq!(
        s.trial_balance(&c, "2026-03-01")
            .unwrap()
            .total_debit
            .rials(),
        1251
    );
    assert!(
        s.reverse(
            &adjustment_id,
            &p,
            "2026-03-02",
            "reverse-duplicate",
            "آزمایشگر محلی",
            "اصلاح دوم"
        )
        .is_err()
    );
    assert!(
        s.reverse(
            &reverse_id,
            &p,
            "2026-03-02",
            "reverse-reverse",
            "آزمایشگر محلی",
            "اصلاح دوم"
        )
        .is_err()
    );
    s.validate().unwrap();
    let restored: AccountingState =
        serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored, s);
}
#[test]
fn retry_identity_and_business_identity_survive_restore_and_lock() {
    let (mut s, c, p, a, e) = fixture();
    let cmd = command(&c, &p, &a, &e, 1251);
    let id = s.post(cmd.clone()).unwrap();
    assert_eq!(s.post(cmd.clone()).unwrap(), id);
    let mut second = cmd.clone();
    second.request_key = "different-key".into();
    assert_eq!(s.post(second.clone()).unwrap(), id);
    let before = s.clone();
    let mut conflicting = cmd.clone();
    conflicting.reason = "محتوای دیگر".into();
    assert!(s.post(conflicting.clone()).is_err());
    conflicting.request_key = "third-key".into();
    assert!(s.post(conflicting).is_err());
    assert_eq!(s, before);
    assert_eq!(s.journals.len(), 1);
    assert_eq!(s.requests.len(), 2);
    s.set_period_lock(&p, true, "آزمایشگر محلی", "قفل آزمون")
        .unwrap();
    assert_eq!(s.post(cmd).unwrap(), id);
    let mut alias = second;
    alias.request_key = "locked-retry".into();
    assert_eq!(s.post(alias).unwrap(), id);
    s.validate().unwrap();
    let mut restored: AccountingState =
        serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
    let retry = restored.requests[0].command.clone();
    assert_eq!(restored.post(retry).unwrap(), id);
    restored.validate().unwrap();
}
#[test]
fn failed_post_is_atomic_for_invalid_lines_unbalance_and_overflow() {
    let (s, c, p, a, e) = fixture();
    let cmd = command(&c, &p, &a, &e, 1251);
    let mut variants = vec![];
    let mut bad = cmd.clone();
    bad.lines[1].credit = Money::from_rials(1250);
    variants.push(bad);
    let mut bad = cmd.clone();
    bad.lines[0].debit = Money::ZERO;
    variants.push(bad);
    let mut bad = cmd.clone();
    bad.lines[0].credit = Money::from_rials(1);
    variants.push(bad);
    let mut bad = cmd.clone();
    bad.lines[0].debit = Money::from_rials(-1);
    variants.push(bad);
    let mut bad = cmd.clone();
    bad.lines.truncate(1);
    variants.push(bad);
    let mut bad = cmd.clone();
    bad.lines[0].account_id = "absent".into();
    variants.push(bad);
    let mut bad = command(&c, &p, &a, &e, i64::MAX);
    bad.lines.push(JournalLineInput {
        account_id: a.clone(),
        debit: Money::from_rials(1),
        credit: Money::ZERO,
        description: String::new(),

        dimension: String::new(),
    });
    variants.push(bad);
    for bad in variants {
        let mut candidate = s.clone();
        assert!(candidate.post(bad).is_err());
        assert_eq!(candidate, s);
    }
}
#[test]
fn company_dates_hierarchy_and_control_restrictions() {
    let (mut s, c, p, a, e) = fixture();
    assert!(
        s.add_period(&c, "هم‌پوشان", "2026-12-31", "2027-12-31", "آزمایشگر")
            .is_err()
    );
    assert!(
        s.add_period(&c, "نامعتبر", "2026-02-29", "2026-12-31", "آزمایشگر")
            .is_err()
    );
    assert!(
        s.add_period(&c, "وارونه", "2027-12-31", "2027-01-01", "آزمایشگر")
            .is_err()
    );
    assert!(
        s.add_period(&c, "قالب", "2027-1-01", "2027-12-31", "آزمایشگر")
            .is_err()
    );
    let c2 = s.add_company("دوم", "آزمایشگر").unwrap();
    let p2 = s
        .add_period(&c2, "دوم", "2026-01-01", "2026-12-31", "آزمایشگر")
        .unwrap();
    let a2 = account(&mut s, &c2, "1101", AccountClass::Asset, BalanceSide::Debit);
    let mut cmd = command(&c, &p, &a2, &e, 1);
    assert!(s.post(cmd.clone()).is_err());
    cmd.lines[0].account_id = a.clone();
    cmd.period_id = p2;
    assert!(s.post(cmd).is_err());
    let mut cmd = command(&c, &p, &a, &e, 1);
    cmd.date = "2027-01-01".into();
    assert!(s.post(cmd).is_err());
    let control = s
        .add_account(
            NewAccount {
                company_id: c.clone(),
                parent_id: None,
                code: "1200".into(),
                name: "کنترل مصنوعی".into(),
                classification: AccountClass::Asset,
                normal_balance: BalanceSide::Debit,
                postable: true,
                control: true,
            },
            "آزمایشگر",
        )
        .unwrap();
    assert!(s.post(command(&c, &p, &control, &e, 1)).is_err());
    assert!(
        s.add_account(
            NewAccount {
                company_id: c,
                parent_id: Some(a2),
                code: "child".into(),
                name: "فرزند".into(),
                classification: AccountClass::Asset,
                normal_balance: BalanceSide::Debit,
                postable: true,
                control: false
            },
            "آزمایشگر"
        )
        .is_err()
    );
    s.validate().unwrap();
}
#[test]
fn archived_accounts_allow_only_exact_original_reversal_snapshots() {
    let (mut s, c, p, a, e) = fixture();
    let journal_id = s.post(command(&c, &p, &a, &e, 1251)).unwrap();
    let original = s.journals[0].clone();
    s.archive_account(&a, "آزمایشگر", "پایان استفاده").unwrap();
    let mut manual = command(&c, &p, &a, &e, 1);
    manual.kind = JournalKind::Manual;
    manual.source_id = "new".into();
    manual.event = "manual".into();
    manual.request_key = "new".into();
    assert!(s.post(manual).is_err());
    let reversal = s
        .reverse(
            &journal_id,
            &p,
            "2026-02-01",
            "reverse",
            "آزمایشگر",
            "برگشت دقیق",
        )
        .unwrap();
    assert_eq!(s.journals[0], original);
    assert_eq!(
        s.journals.iter().find(|j| j.id == reversal).unwrap().lines[0].account,
        original.lines[0].account
    );
    assert_eq!(
        s.trial_balance(&c, "2026-02-01").unwrap().total_debit,
        Money::ZERO
    );
    s.validate().unwrap();
}
#[test]
fn locks_reopen_and_reversal_cross_period_preserve_history() {
    let (mut s, c, p, a, e) = fixture();
    let journal = s.post(command(&c, &p, &a, &e, 1251)).unwrap();
    let next = s
        .add_period(&c, "بعدی", "2027-01-01", "2027-12-31", "آزمایشگر")
        .unwrap();
    s.set_period_lock(&p, true, "آزمایشگر", "بررسی تمام شد")
        .unwrap();
    assert!(
        s.reverse(&journal, &next, "2027-01-01", "reverse", "آزمایشگر", "اصلاح")
            .is_err()
    );
    assert!(s.set_period_lock(&p, false, "آزمایشگر", "").is_err());
    s.set_period_lock(&p, false, "آزمایشگر", "بازگشایی با علت")
        .unwrap();
    s.reverse(&journal, &next, "2027-01-01", "reverse", "آزمایشگر", "اصلاح")
        .unwrap();
    s.set_period_lock(&p, true, "آزمایشگر", "قفل دوباره")
        .unwrap();
    s.validate().unwrap();
}
#[test]
fn cumulative_books_reject_reimported_opening_in_later_year() {
    let (mut s, c, p, a, e) = fixture();
    s.post(command(&c, &p, &a, &e, 1251)).unwrap();
    let next = s
        .add_period(&c, "بعدی", "2027-01-01", "2027-12-31", "آزمایشگر")
        .unwrap();
    let mut opening = command(&c, &next, &a, &e, 1251);
    opening.date = "2027-01-01".into();
    opening.request_key = "next-opening".into();
    opening.source_id = "opening-2027".into();
    assert!(s.post(opening).is_err());
    assert_eq!(
        s.trial_balance(&c, "2027-12-31")
            .unwrap()
            .total_debit
            .rials(),
        1251
    );
    s.validate().unwrap();
}
#[test]
fn restored_state_and_generic_store_transition_reject_history_tampering() {
    let (mut s, c, p, a, e) = fixture();
    s.post(command(&c, &p, &a, &e, 1251)).unwrap();
    let mut next = s.clone();
    next.journals[0].reason = "سند بازنویسی‌شده".into();
    assert!(s.validate_transition(&next).is_err());
    let mut next = s.clone();
    next.journals.clear();
    assert!(next.validate().is_err());
    let mut next = s.clone();
    next.requests.push(next.requests[0].clone());
    assert!(next.validate().is_err());
    let mut next = s.clone();
    next.accounts[0].name = "نام بازنویسی‌شده".into();
    assert!(s.validate_transition(&next).is_err());
    let mut next = s.clone();
    next.periods[0].locked = true;
    assert!(s.validate_transition(&next).is_err());
    let mut next = s.clone();
    next.audit.clear();
    assert!(next.validate().is_err());
    let mut next = s.clone();
    next.journals[0].lines[0].account.version = 2;
    assert!(next.validate().is_err());
    let mut next = s.clone();
    next.audit[0].company_id = "absent".into();
    assert!(next.validate().is_err());
    let mut next = s.clone();
    next.set_period_lock(&p, true, "آزمایشگر", "قفل معتبر")
        .unwrap();
    s.validate_transition(&next).unwrap();
    next.archive_account(&a, "آزمایشگر", "بایگانی معتبر")
        .unwrap();
    s.validate_transition(&next).unwrap();
}
#[test]
fn many_exact_synthetic_postings_have_independent_expected_totals() {
    let (mut s, c, p, a, e) = fixture();
    let mut expected = 0;
    for n in 1..=100 {
        let amount = n * 37;
        expected += amount;
        let mut cmd = command(&c, &p, &a, &e, amount);
        cmd.kind = JournalKind::Manual;
        cmd.source_id = format!("manual-{n}");
        cmd.event = "manual".into();
        cmd.request_key = format!("request-{n}");
        s.post(cmd).unwrap();
    }
    let tb = s.trial_balance(&c, "2026-12-31").unwrap();
    assert_eq!(expected, 186850);
    assert_eq!(tb.total_debit.rials(), 186850);
    assert_eq!(tb.total_credit.rials(), 186850);
    s.validate().unwrap();
}

#[test]
fn post_rejects_cumulative_account_and_report_overflow_atomically() {
    let (mut s, c, p, a, e) = fixture();
    s.post(command(&c, &p, &a, &e, i64::MAX)).unwrap();
    let before = s.clone();
    let mut extra = command(&c, &p, &a, &e, 1);
    extra.kind = JournalKind::Manual;
    extra.source_id = "extra".into();
    extra.event = "manual".into();
    extra.request_key = "extra".into();
    assert!(s.post(extra.clone()).is_err());
    assert_eq!(s, before);
    // Every individual account still fits; the aggregate report would overflow.
    let a2 = account(&mut s, &c, "1102", AccountClass::Asset, BalanceSide::Debit);
    let e2 = account(
        &mut s,
        &c,
        "3102",
        AccountClass::Equity,
        BalanceSide::Credit,
    );
    extra.lines[0].account_id = a2;
    extra.lines[1].account_id = e2;
    let before = s.clone();
    assert!(s.post(extra).is_err());
    assert_eq!(s, before);
    assert_eq!(
        s.trial_balance(&c, "2026-12-31")
            .unwrap()
            .total_debit
            .rials(),
        i64::MAX
    );
    s.validate().unwrap();
}

#[test]
fn backdated_post_checks_every_affected_cutoff_not_only_current_balance() {
    let (mut s, c, p, a, e) = fixture();
    s.post(command(&c, &p, &a, &e, i64::MAX)).unwrap();
    let mut reduction = command(&c, &p, &e, &a, 1);
    reduction.kind = JournalKind::Manual;
    reduction.date = "2026-02-01".into();
    reduction.source_id = "reduction".into();
    reduction.event = "manual".into();
    reduction.request_key = "reduction".into();
    s.post(reduction).unwrap();
    assert_eq!(
        s.trial_balance(&c, "2026-02-01")
            .unwrap()
            .total_debit
            .rials(),
        i64::MAX - 1
    );
    let mut backdated = command(&c, &p, &a, &e, 1);
    backdated.kind = JournalKind::Manual;
    backdated.date = "2026-01-02".into();
    backdated.source_id = "backdated".into();
    backdated.event = "manual".into();
    backdated.request_key = "backdated".into();
    let before = s.clone();
    assert!(s.post(backdated).is_err());
    assert_eq!(s, before);
    s.validate().unwrap();
}

#[test]
fn optional_dimension_stays_out_of_old_books_and_does_not_unbalance() {
    let (mut state, company, period, asset, equity) = fixture();
    let mut posted = command(&company, &period, &asset, &equity, 40);
    posted.kind = JournalKind::Manual;
    posted.event = "manual".into();
    posted.request_key = "dimension-post".into();
    posted.source_id = "dimension-post".into();
    posted.lines[0].dimension = "party:sample".into();
    state.post(posted).unwrap();
    let journal = state.journals.last().unwrap();
    assert_eq!(journal.lines[0].dimension, "party:sample");
    assert!(journal.lines[1].dimension.is_empty());
    assert_eq!(journal.lines[0].debit, journal.lines[1].credit);
    let encoded = serde_json::to_string(journal).unwrap();
    assert!(encoded.contains("party:sample"));
    assert_eq!(encoded.matches("\"dimension\"").count(), 1);
    let bare = JournalLineInput {
        account_id: "legacy".into(),
        debit: Money::ZERO,
        credit: Money::ZERO,
        description: "legacy".into(),
        dimension: String::new(),
    };
    let value = serde_json::to_value(&bare).unwrap();
    assert!(value.get("dimension").is_none());
    let restored: JournalLineInput = serde_json::from_value(value).unwrap();
    assert!(restored.dimension.is_empty());
    state.validate().unwrap();
    let id = state.journals.last().unwrap().id.clone();
    state
        .reverse(
            &id,
            &period,
            "2026-01-02",
            "dimension-reversal",
            "آزمایشگر محلی",
            "برگرداندن تفصیلی",
        )
        .unwrap();
    assert_eq!(
        state.journals.last().unwrap().lines[0].dimension,
        "party:sample"
    );
    state.validate().unwrap();
    state.journals[0].lines[0].dimension = "changed".into();
    assert!(state.validate().is_err());
}
