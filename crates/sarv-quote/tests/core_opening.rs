use sarv_quote::accounting::*;
use sarv_quote::core::*;
fn irr(n: i64) -> Money {
    Money::from_rials(n)
}
fn fixture() -> (AccountingState, OpeningCommand) {
    let mut state = AccountingState::default();
    let setup = state
        .setup_core_company(NewCoreCompany {
            name: "F2-core-v1".into(),
            fiscal_name: "1405".into(),
            start_date: "2026-03-21".into(),
            end_date: "2027-03-20".into(),
            timezone: CompanyTimezone::AsiaTehran,
            chart: default_chart(),
            actor: "owner".into(),
        })
        .unwrap();
    let co = setup.company_id;
    let balances = CoreRole::ALL
        .into_iter()
        .filter(|r| r.is_control() || *r == CoreRole::OwnerCapital)
        .map(|role| {
            let (debit, credit) = match role {
                CoreRole::Cash => (251, 0),
                CoreRole::Bank => (300, 0),
                CoreRole::Receivables => (200, 0),
                CoreRole::Inventory => (300, 0),
                CoreRole::ChequesReceivable => (200, 0),
                CoreRole::Payables => (0, 149),
                CoreRole::ChequesPayable => (0, 100),
                CoreRole::Grni => (0, 50),
                CoreRole::OwnerCapital => (0, 952),
                _ => (0, 0),
            };
            OpeningBalance {
                account_id: state.core.setups[0].account_id(role).unwrap().into(),
                debit: irr(debit),
                credit: irr(credit),
                source_reference: format!("GL-{}", role.key()),
            }
        })
        .collect();
    let parties = ["A", "B"]
        .into_iter()
        .map(|n| CoreParty {
            id: n.into(),
            company_id: co.clone(),
            name: n.into(),
            customer: true,
            supplier: true,
        })
        .collect();
    let units = ["piece", "carton"]
        .into_iter()
        .map(|n| CoreUnit {
            id: n.into(),
            company_id: co.clone(),
            name: n.into(),
        })
        .collect();
    let warehouses = ["W1", "W2"]
        .into_iter()
        .map(|n| CoreWarehouse {
            id: n.into(),
            company_id: co.clone(),
            name: n.into(),
        })
        .collect();
    let open_items = [
        ("AR1", "A", OpenItemSide::Receivable, 125),
        ("AR2", "B", OpenItemSide::Receivable, 75),
        ("AP1", "A", OpenItemSide::Payable, 100),
        ("AP2", "B", OpenItemSide::Payable, 49),
    ]
    .into_iter()
    .map(|(id, party, side, n)| OpeningOpenItem {
        id: id.into(),
        company_id: co.clone(),
        party_id: party.into(),
        side,
        document_reference: id.into(),
        document_date: "2026-03-01".into(),
        due_date: "2026-04-01".into(),
        remaining: irr(n),
    })
    .collect();
    let cheques = [
        (
            "CR1",
            ChequeDirection::Receivable,
            OpeningChequeState::Held,
            200,
        ),
        (
            "CP1",
            ChequeDirection::Payable,
            OpeningChequeState::Issued,
            100,
        ),
    ]
    .into_iter()
    .map(|(id, direction, status, n)| OpeningCheque {
        id: id.into(),
        company_id: co.clone(),
        party_id: "A".into(),
        direction,
        state: status,
        bank_reference: "bank-cheque".into(),
        holder: "owner".into(),
        cheque_reference: id.into(),
        settlement_reference: format!("settled-{id}"),
        issue_date: "2026-03-01".into(),
        due_date: "2026-05-01".into(),
        amount: irr(n),
    })
    .collect();
    let stock = [
        ("S1", "W1", "carton", 2000, 240),
        ("S2", "W2", "piece", 6000, 60),
    ]
    .into_iter()
    .map(|(id, w, u, q, v)| OpeningStock {
        id: id.into(),
        company_id: co.clone(),
        item_id: "item-A".into(),
        warehouse_id: w.into(),
        unit_id: u.into(),
        quantity_thousandths: q,
        value: irr(v),
        source_reference: id.into(),
    })
    .collect();
    let c = OpeningCommand {
        company_id: co.clone(),
        period_id: setup.period_id,
        setup_id: setup.setup_id,
        as_of: "2026-03-21".into(),
        source_id: "supplied-opening".into(),
        request_key: "opening-key".into(),
        actor: "owner".into(),
        reason: "independent fixture".into(),
        balances,
        parties,
        units,
        items: vec![CoreItem {
            id: "item-A".into(),
            company_id: co.clone(),
            sku: "SKU-A".into(),
            name: "A".into(),
            base_unit_id: "piece".into(),
            indivisible: true,
            packs: vec![PackRatio {
                unit_id: "carton".into(),
                numerator: 12,
                denominator: 1,
            }],
        }],
        warehouses,
        cash: vec![OpeningCash {
            id: "C1".into(),
            company_id: co.clone(),
            name: "counted till".into(),
            balance: irr(251),
            source_reference: "counted-C1".into(),
        }],
        banks: vec![OpeningBank {
            id: "B1".into(),
            company_id: co.clone(),
            name: "book bank".into(),
            bank_reference: "B1-ref".into(),
            balance: irr(300),
            statement_balance: irr(300),
            source_reference: "statement-B1".into(),
        }],
        open_items,
        advances: vec![],
        cheques,
        stock,
        grni: vec![OpeningGrni {
            id: "R1".into(),
            company_id: co,
            party_id: "A".into(),
            item_id: "item-A".into(),
            warehouse_id: "W2".into(),
            unit_id: "piece".into(),
            quantity_thousandths: 5000,
            value: irr(50),
            receipt_reference: "received-R1".into(),
            received_date: "2026-03-10".into(),
        }],
        import_source: None,
    };
    (state, c)
}
#[test]
fn literal_1251_opening_reconciles_and_survives_replay() {
    let (mut s, c) = fixture();
    assert!(s.reconcile_opening(&c.company_id, &c.as_of).is_err());
    let p = s.opening_preview(&c).unwrap();
    assert_eq!(
        (p.total_debit.rials(), p.total_credit.rials()),
        (1251, 1251)
    );
    assert!(p.balanced && p.reconciled);
    assert_eq!(p.controls.len(), 12);
    assert_eq!(c.base_quantity("item-A", "carton", 2000).unwrap(), 24000);
    assert_eq!(c.base_quantity("item-A", "piece", 6000).unwrap(), 6000);
    let journal = s.post_opening(c.clone()).unwrap();
    let report = s.reconcile_opening(&c.company_id, &c.as_of).unwrap();
    let literal = [
        (CoreRole::Cash, 251),
        (CoreRole::Bank, 300),
        (CoreRole::Receivables, 200),
        (CoreRole::ChequesReceivable, 200),
        (CoreRole::Inventory, 300),
        (CoreRole::SupplierAdvances, 0),
        (CoreRole::Payables, 149),
        (CoreRole::ChequesPayable, 100),
        (CoreRole::Grni, 50),
        (CoreRole::CustomerAdvances, 0),
        (CoreRole::Cogs, 0),
        (CoreRole::InventoryLoss, 0),
    ];
    for (role, expected) in literal {
        let r = report.controls.iter().find(|r| r.role == role).unwrap();
        assert_eq!(
            (r.ledger.rials(), r.detail.rials(), r.difference.rials()),
            (expected, expected, 0)
        );
    }
    assert_eq!(
        s.journals
            .iter()
            .find(|j| j.id == journal)
            .unwrap()
            .lines
            .len(),
        9
    );
    assert_eq!(
        s.trial_balance(&c.company_id, &c.as_of)
            .unwrap()
            .total_debit,
        irr(1251)
    );
    assert!(s.reconcile_opening(&c.company_id, "2026-03-20").is_err());
    assert!(s.reconcile_opening(&c.company_id, "2026-03-22").is_err());
    s.validate().unwrap();
    let restored: AccountingState =
        serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(s, restored);
}
#[test]
fn errors_and_mismatches_leave_direct_command_state_unchanged() {
    let (mut s, c) = fixture();
    let original = s.clone();
    let mut mismatch = c.clone();
    mismatch.open_items[0].remaining = irr(124);
    let p = s.opening_preview(&mismatch).unwrap();
    assert!(p.balanced);
    assert!(!p.reconciled);
    assert_eq!(
        p.controls
            .iter()
            .find(|r| r.role == CoreRole::Receivables)
            .unwrap()
            .difference,
        irr(1)
    );
    assert!(s.post_opening(mismatch).is_err());
    assert_eq!(s, original);
    type OpeningMutation = Box<dyn Fn(&mut OpeningCommand)>;
    let changes: Vec<OpeningMutation> = vec![
        Box::new(|c| {
            c.balances.remove(5);
        }),
        Box::new(|c| c.open_items.push(c.open_items[0].clone())),
        Box::new(|c| c.banks[0].statement_balance = irr(299)),
        Box::new(|c| c.stock[0].company_id = "foreign".into()),
        Box::new(|c| c.stock[0].value = irr(-1)),
        Box::new(|c| c.stock[0].unit_id = "unknown".into()),
        Box::new(|c| c.grni[0].quantity_thousandths = 7000),
        Box::new(|c| c.cheques[0].state = OpeningChequeState::Issued),
        Box::new(|c| c.cheques[0].settlement_reference = "AR1".into()),
        Box::new(|c| c.open_items[0].document_date = "2026-03-22".into()),
        Box::new(|c| c.as_of = "2026-03-22".into()),
        Box::new(|c| c.items[0].packs[0].denominator = 7),
    ];
    for (i, change) in changes.into_iter().enumerate() {
        let mut bad = c.clone();
        change(&mut bad);
        assert!(s.post_opening(bad).is_err(), "case {i}");
        assert_eq!(s, original, "case {i}");
    }
    s.set_period_lock(&c.period_id, true, "owner", "period lock")
        .unwrap();
    let locked = s.clone();
    assert!(s.post_opening(c).is_err());
    assert_eq!(s, locked);
}
#[test]
fn retries_freeze_detail_and_do_not_second_initialize() {
    let (mut s, c) = fixture();
    let j = s.post_opening(c.clone()).unwrap();
    let once = s.clone();
    assert_eq!(s.post_opening(c.clone()).unwrap(), j);
    assert_eq!(s, once);
    let mut alias = c.clone();
    alias.request_key = "second-key".into();
    assert_eq!(s.post_opening(alias.clone()).unwrap(), j);
    assert_eq!(s.journals.len(), 1);
    assert_eq!(s.core.openings.len(), 1);
    assert_eq!(s.requests.len(), 2);
    s.validate().unwrap();
    let snapshot = s.clone();
    alias.open_items[0].due_date = "2026-04-02".into();
    assert!(s.post_opening(alias).is_err());
    assert_eq!(s, snapshot);
    let mut second = c.clone();
    second.source_id = "different-source".into();
    second.request_key = "third-key".into();
    assert!(s.post_opening(second).is_err());
    assert_eq!(s, snapshot);
    let mut tampered = s.clone();
    tampered.core.setups[0].command.policy.timezone = CompanyTimezone::Utc;
    assert!(tampered.validate().is_err());
    assert!(s.validate_transition(&tampered).is_err());
    let mut tampered = s.clone();
    tampered.core.openings[0].command.stock[0].value = irr(239);
    assert!(tampered.validate().is_err());
    let mut tampered = s.clone();
    tampered.core.openings[0].command.stock[0].quantity_thousandths = 3000;
    assert!(tampered.validate().is_err());
    let mut tampered = s.clone();
    tampered.core.openings[0].command.open_items[0].due_date = "2026-04-02".into();
    assert!(tampered.validate().is_err());
    let mut tampered = s.clone();
    tampered.core.openings[0]
        .frozen_setup
        .command
        .policy
        .timezone = CompanyTimezone::Utc;
    assert!(tampered.validate().is_err());
    assert!(
        s.reverse(
            &j,
            &c.period_id,
            "2026-03-22",
            "reverse-opening",
            "owner",
            "not a supported correction"
        )
        .is_err()
    );
    assert_eq!(s, snapshot);
    s.set_period_lock(&c.period_id, true, "owner", "lock after posted")
        .unwrap();
    assert_eq!(s.post_opening(c.clone()).unwrap(), j);
    s.validate().unwrap();
}
#[test]
fn nonzero_advances_reconcile_separately_for_dual_role_party() {
    let (mut s, mut c) = fixture();
    c.advances = vec![
        OpeningAdvance {
            id: "SA".into(),
            company_id: c.company_id.clone(),
            party_id: "A".into(),
            side: AdvanceSide::Supplier,
            source_reference: "paid-advance".into(),
            document_date: "2026-03-20".into(),
            remaining: irr(11),
        },
        OpeningAdvance {
            id: "CA".into(),
            company_id: c.company_id.clone(),
            party_id: "A".into(),
            side: AdvanceSide::Customer,
            source_reference: "received-advance".into(),
            document_date: "2026-03-20".into(),
            remaining: irr(7),
        },
    ];
    for b in &mut c.balances {
        if b.account_id
            == s.core.setups[0]
                .account_id(CoreRole::SupplierAdvances)
                .unwrap()
        {
            b.debit = irr(11);
        }
        if b.account_id
            == s.core.setups[0]
                .account_id(CoreRole::CustomerAdvances)
                .unwrap()
        {
            b.credit = irr(7);
        }
        if b.account_id == s.core.setups[0].account_id(CoreRole::OwnerCapital).unwrap() {
            b.credit = irr(956);
        }
    }
    assert!(s.opening_preview(&c).unwrap().reconciled);
    s.post_opening(c).unwrap();
    s.validate().unwrap();
}
#[test]
fn guided_setup_is_atomic_customizable_and_preserves_other_books() {
    let mut s = AccountingState::default();
    let other = s.add_company("existing manual books", "owner").unwrap();
    let period = s
        .add_period(&other, "legacy year", "2026-03-21", "2027-03-20", "owner")
        .unwrap();
    let legacy_cash = s
        .add_account(
            NewAccount {
                company_id: other.clone(),
                parent_id: None,
                code: "legacy-cash".into(),
                name: "unchanged ordinary cash".into(),
                classification: AccountClass::Asset,
                normal_balance: BalanceSide::Debit,
                postable: true,
                control: false,
            },
            "owner",
        )
        .unwrap();
    let legacy_capital = s
        .add_account(
            NewAccount {
                company_id: other.clone(),
                parent_id: None,
                code: "legacy-capital".into(),
                name: "unchanged equity".into(),
                classification: AccountClass::Equity,
                normal_balance: BalanceSide::Credit,
                postable: true,
                control: false,
            },
            "owner",
        )
        .unwrap();
    s.post(PostCommand {
        company_id: other.clone(),
        period_id: period,
        date: "2026-03-21".into(),
        source_id: "legacy-source".into(),
        source_revision: 1,
        event: "legacy-opening".into(),
        request_key: "legacy-key".into(),
        actor: "owner".into(),
        reason: "existing approved manual fixture".into(),
        kind: JournalKind::Opening,
        lines: vec![
            JournalLineInput {
                account_id: legacy_cash,
                debit: irr(1251),
                credit: irr(0),
                description: "independent legacy funds".into(),

                dimension: String::new(),
            },
            JournalLineInput {
                account_id: legacy_capital,
                debit: irr(0),
                credit: irr(1251),
                description: "independent supplied capital".into(),

                dimension: String::new(),
            },
        ],
    })
    .unwrap();
    let prior = s.clone();
    let mut chart = default_chart();
    chart[0].code = "CUSTOM-CASH".into();
    chart[0].name = "My cashbox".into();
    let new = NewCoreCompany {
        name: "fresh".into(),
        fiscal_name: "1405".into(),
        start_date: "2026-03-21".into(),
        end_date: "2027-03-20".into(),
        timezone: CompanyTimezone::Utc,
        chart,
        actor: "owner".into(),
    };
    let mut invalid = new.clone();
    invalid.chart[0].code = "1".into();
    assert!(s.setup_core_company(invalid).is_err());
    assert_eq!(s, prior);
    s.setup_core_company(new).unwrap();
    assert_eq!(s.companies[0].id, other);
    assert_eq!(s.journals, prior.journals);
    assert!(s.accounts.starts_with(&prior.accounts));
    assert_eq!(
        s.trial_balance(&other, "2026-03-21").unwrap().total_debit,
        irr(1251)
    );
    assert!(s.companies.starts_with(&prior.companies));
    assert!(s.audit.starts_with(&prior.audit));
    assert_eq!(s.accounts.iter().filter(|a| !a.postable).count(), 5);
    assert!(
        s.accounts
            .iter()
            .any(|a| a.code == "CUSTOM-CASH" && a.name == "My cashbox")
    );
    s.validate().unwrap();
    let mut forbidden = s.core.setups[0].command.clone();
    forbidden.company_id = other;
    let before = s.clone();
    assert!(s.configure_core(forbidden).is_err());
    assert_eq!(s, before);
}
#[test]
fn rational_quantity_and_half_up_are_exact_checked() {
    let (_, mut c) = fixture();
    c.items[0].indivisible = false;
    assert_eq!(c.base_quantity("item-A", "carton", 125).unwrap(), 1500);
    c.items[0].packs[0].numerator = 1;
    c.items[0].packs[0].denominator = 3;
    assert!(c.base_quantity("item-A", "carton", 1000).is_err());
    assert_eq!(c.base_quantity("item-A", "carton", 3000).unwrap(), 1000);
    c.items[0].packs[0].numerator = i64::MAX;
    c.items[0].packs[0].denominator = 1;
    assert!(c.base_quantity("item-A", "carton", 1000).is_err());
    assert_eq!(positive_half_up(350 * 5, 30).unwrap(), 58);
    assert_eq!(positive_half_up(58 * 2, 5).unwrap(), 23);
    assert_eq!(positive_half_up(5, 2).unwrap(), 3);
    assert!(positive_half_up(i128::MAX, 1).is_err());
}

fn zero_fixture() -> (AccountingState, OpeningCommand) {
    let (s, mut c) = fixture();
    for b in &mut c.balances {
        b.debit = Money::ZERO;
        b.credit = Money::ZERO;
    }
    c.parties.clear();
    c.units.clear();
    c.items.clear();
    c.warehouses.clear();
    c.cash.clear();
    c.banks.clear();
    c.open_items.clear();
    c.advances.clear();
    c.cheques.clear();
    c.stock.clear();
    c.grni.clear();
    (s, c)
}
#[test]
fn zero_opening_initializes_without_financial_journal_and_replays_aliases() {
    let (mut s, c) = zero_fixture();
    let p = s.opening_preview(&c).unwrap();
    assert!(p.balanced && p.reconciled);
    assert_eq!((p.total_debit, p.total_credit), (Money::ZERO, Money::ZERO));
    let audit_before = s.audit.len();
    let initialization = s.post_opening(c.clone()).unwrap();
    assert_eq!(s.audit.len(), audit_before + 1);
    assert!(s.journals.is_empty());
    assert!(s.requests.is_empty());
    assert_eq!(
        s.audit.last().unwrap().operation,
        AuditOperation::CoreOpeningInitialized
    );
    assert_eq!(s.audit.last().unwrap().entity_id, initialization);
    assert_eq!(
        s.core.openings[0].effect,
        OpeningEffect::ZeroBalanceInitialization
    );
    assert_eq!(s.core.zero_requests.len(), 1);
    assert_eq!(s.core.zero_requests[0].initialization_id, initialization);
    assert!(
        s.reconcile_opening(&c.company_id, &c.as_of)
            .unwrap()
            .reconciled
    );
    let once = s.clone();
    assert_eq!(s.post_opening(c.clone()).unwrap(), initialization);
    assert_eq!(s, once);
    let mut alias = c.clone();
    alias.request_key = "zero-alias-before-second".into();
    assert_eq!(s.post_opening(alias.clone()).unwrap(), initialization);
    assert_eq!(s.core.zero_requests.len(), 2);
    let second = s
        .setup_core_company(NewCoreCompany {
            name: "second fresh empty business".into(),
            fiscal_name: "1405".into(),
            start_date: "2026-03-21".into(),
            end_date: "2027-03-20".into(),
            timezone: CompanyTimezone::Utc,
            chart: default_chart(),
            actor: "owner".into(),
        })
        .unwrap();
    let mut second_c = c.clone();
    second_c.company_id = second.company_id;
    second_c.period_id = second.period_id;
    second_c.setup_id = second.setup_id;
    second_c.request_key = "second-zero-company".into();
    second_c.source_id = "second-independent-empty-opening".into();
    let old_setup = s.core.setups[0].clone();
    let second_setup = s.core.setups[1].clone();
    for b in &mut second_c.balances {
        let role = old_setup
            .command
            .mappings
            .iter()
            .find(|m| m.account_id == b.account_id)
            .unwrap()
            .role;
        b.account_id = second_setup.account_id(role).unwrap().into();
    }
    let mut marker_collision = second_c.clone();
    marker_collision.parties.push(CoreParty {
        id: initialization.clone(),
        company_id: second_c.company_id.clone(),
        name: "invalid reuse of another initialization identity".into(),
        customer: true,
        supplier: false,
    });
    let unchanged = s.clone();
    assert!(s.post_opening(marker_collision).is_err());
    assert_eq!(s, unchanged);
    let second_id = s.post_opening(second_c.clone()).unwrap();
    assert_ne!(initialization, second_id);
    alias.request_key = "zero-alias-after-second".into();
    assert_eq!(s.post_opening(alias.clone()).unwrap(), initialization);
    assert_eq!(s.core.zero_requests.len(), 4);
    assert!(s.journals.is_empty());
    assert!(s.requests.is_empty());
    s.validate().unwrap();
    let restored: AccountingState =
        serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(s, restored);
    let before = s.clone();
    let mut changed = alias.clone();
    changed.reason = "changed detailed initialization".into();
    assert!(s.post_opening(changed).is_err());
    assert_eq!(s, before);
    let mut second_source = c.clone();
    second_source.request_key = "different-empty-source".into();
    second_source.source_id = "replacement-empty-opening".into();
    assert!(s.post_opening(second_source).is_err());
    assert_eq!(s, before);
    let mut cross_key = second_c.clone();
    cross_key.request_key = c.request_key.clone();
    assert!(s.post_opening(cross_key).is_err());
    assert_eq!(s, before);
    let mut tampered = s.clone();
    tampered.core.zero_requests[3].command_sha256 = "0".repeat(64);
    assert!(tampered.validate().is_err());
    assert!(s.validate_transition(&tampered).is_err());
    let mut tampered = s.clone();
    tampered.core.openings[0].effect = OpeningEffect::Journal;
    assert!(tampered.validate().is_err());
    let mut tampered = s.clone();
    tampered.core.openings[0].command.balances[0].debit = irr(1);
    assert!(tampered.validate().is_err());
    s.set_period_lock(&c.period_id, true, "owner", "lock zero initialized period")
        .unwrap();
    assert_eq!(s.post_opening(c).unwrap(), initialization);
    s.validate().unwrap();
}
#[test]
fn zero_and_financial_request_keys_share_one_collision_boundary() {
    let (mut s, c) = zero_fixture();
    let marker = s.post_opening(c.clone()).unwrap();
    let fixed = s.core.setups[0]
        .account_id(CoreRole::FixedAssets)
        .unwrap()
        .to_string();
    let capital = s.core.setups[0]
        .account_id(CoreRole::OwnerCapital)
        .unwrap()
        .to_string();
    let command = PostCommand {
        company_id: c.company_id.clone(),
        period_id: c.period_id.clone(),
        date: "2026-03-22".into(),
        source_id: "ordinary-adjustment".into(),
        source_revision: 1,
        event: "manual".into(),
        request_key: "ordinary-financial-key".into(),
        actor: "owner".into(),
        reason: "explicit ordinary acquisition with capital".into(),
        kind: JournalKind::Manual,
        lines: vec![
            JournalLineInput {
                account_id: fixed,
                debit: irr(1),
                credit: Money::ZERO,
                description: "explicit asset".into(),

                dimension: String::new(),
            },
            JournalLineInput {
                account_id: capital,
                debit: Money::ZERO,
                credit: irr(1),
                description: "explicit supplied capital".into(),

                dimension: String::new(),
            },
        ],
    };
    let before = s.clone();
    let mut bad = command.clone();
    bad.request_key = c.request_key.clone();
    assert!(s.post(bad).is_err());
    assert_eq!(s, before);
    let financial = s.post(command.clone()).unwrap();
    assert_ne!(financial, marker);
    let before = s.clone();
    let mut bad = c.clone();
    bad.request_key = command.request_key.clone();
    assert!(s.post_opening(bad).is_err());
    assert_eq!(s, before);
    assert_eq!(s.post_opening(c.clone()).unwrap(), marker);
    assert_eq!(s, before);
    let mut second_opening = command;
    second_opening.kind = JournalKind::Opening;
    second_opening.date = c.as_of.clone();
    second_opening.request_key = "second-generic-opening".into();
    second_opening.source_id = "generic-second-opening".into();
    assert!(s.post(second_opening).is_err());
    assert_eq!(s, before);
    let p = s.reconcile_opening(&c.company_id, &c.as_of).unwrap();
    assert_eq!(p.total_debit, Money::ZERO);
    assert!(p.reconciled);
    s.validate().unwrap();
}

#[test]
fn zero_valued_stock_keeps_exact_quantity_and_provenance_without_fake_gl() {
    let (mut s, mut c) = fixture();
    for b in &mut c.balances {
        b.debit = Money::ZERO;
        b.credit = Money::ZERO;
    }
    c.cash[0].balance = Money::ZERO;
    c.banks[0].balance = Money::ZERO;
    c.banks[0].statement_balance = Money::ZERO;
    c.open_items.clear();
    c.advances.clear();
    c.cheques.clear();
    for stock in &mut c.stock {
        stock.value = Money::ZERO;
    }
    c.grni[0].value = Money::ZERO;
    let marker = s.post_opening(c.clone()).unwrap();
    assert_eq!(
        s.core.openings[0].effect,
        OpeningEffect::ZeroBalanceInitialization
    );
    assert_eq!(s.core.openings[0].journal_id, marker);
    assert!(s.journals.is_empty());
    assert!(s.requests.is_empty());
    let saved = &s.core.openings[0].command;
    assert_eq!(
        saved
            .base_quantity("item-A", "carton", saved.stock[0].quantity_thousandths)
            .unwrap(),
        24000
    );
    assert_eq!(
        saved
            .base_quantity("item-A", "piece", saved.stock[1].quantity_thousandths)
            .unwrap(),
        6000
    );
    assert_eq!(saved.stock[0].source_reference, "S1");
    assert_eq!(saved.grni[0].quantity_thousandths, 5000);
    assert_eq!(saved.grni[0].value, Money::ZERO);
    let p = s.reconcile_opening(&c.company_id, &c.as_of).unwrap();
    assert!(p.balanced && p.reconciled);
    assert_eq!(p.total_debit, Money::ZERO);
    assert!(s.opening_preview(&c).is_err());
    s.validate().unwrap();
    let mut tampered = s.clone();
    tampered.core.openings[0].command.stock[0].quantity_thousandths = 3000;
    assert!(tampered.validate().is_err());
}
