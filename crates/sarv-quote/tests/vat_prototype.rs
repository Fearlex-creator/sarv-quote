//! Prototype golden calculations for Plan 028 (VAT on sales and purchases).
//! Pure functions live here in the test only; no production code under `crates/*/src` is touched.

const MAX_MONEY: i64 = 9_000_000_000_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TaxCategory {
    Taxable,
    ZeroRated,
    Exempt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PricingMode {
    Exclusive,
    Inclusive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LineInput {
    unit_price_rial: i64,
    quantity_thousandths: i64,
    line_discount_rial: i64,
    document_discount_rial: i64,
    category: TaxCategory,
    rate_bp: u32,
    pricing: PricingMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LineOutput {
    gross_rial: i64,
    net_rial: i64,
    tax_rial: i64,
    total_rial: i64,
    effective_rate_bp: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DocOutput {
    net_rial: i64,
    tax_rial: i64,
    total_rial: i64,
}

fn check_money(v: i64) -> Result<i64, &'static str> {
    if (0..=MAX_MONEY).contains(&v) {
        Ok(v)
    } else {
        Err("money out of bounds")
    }
}

/// Nonnegative exact rational rounded once, with positive half-up ties.
fn positive_half_up(numerator: i128, denominator: i128) -> Result<i64, &'static str> {
    if numerator < 0 || denominator <= 0 {
        return Err("invalid ratio");
    }
    let q = numerator / denominator;
    let r = numerator % denominator;
    let rounded = q.checked_add(i128::from(r >= denominator - r)).ok_or("rounding overflow")?;
    let val = i64::try_from(rounded).map_err(|_| "i64 overflow")?;
    check_money(val)
}

fn compute_line(input: LineInput) -> Result<LineOutput, &'static str> {
    check_money(input.unit_price_rial)?;
    check_money(input.line_discount_rial)?;
    check_money(input.document_discount_rial)?;
    if input.quantity_thousandths <= 0 {
        return Err("quantity must be positive");
    }
    if input.rate_bp > 10_000 {
        return Err("rate_bp out of range");
    }

    let effective_rate_bp = match input.category {
        TaxCategory::Taxable => input.rate_bp,
        TaxCategory::ZeroRated | TaxCategory::Exempt => 0,
    };

    let gross_num = i128::from(input.unit_price_rial)
        .checked_mul(i128::from(input.quantity_thousandths))
        .ok_or("gross multiply overflow")?;
    let gross_rial = positive_half_up(gross_num, 1000)?;

    let total_discount = input
        .line_discount_rial
        .checked_add(input.document_discount_rial)
        .ok_or("discount overflow")?;
    let subtotal = gross_rial.checked_sub(total_discount).ok_or("discount underflow")?;
    check_money(subtotal)?;

    match input.pricing {
        PricingMode::Exclusive => {
            let net_rial = subtotal;
            let tax_rial = if effective_rate_bp == 0 {
                0
            } else {
                let tax_num = i128::from(net_rial)
                    .checked_mul(i128::from(effective_rate_bp))
                    .ok_or("tax multiply overflow")?;
                positive_half_up(tax_num, 10_000)?
            };
            let total_rial = net_rial.checked_add(tax_rial).ok_or("total overflow")?;
            check_money(total_rial)?;
            Ok(LineOutput {
                gross_rial,
                net_rial,
                tax_rial,
                total_rial,
                effective_rate_bp,
            })
        }
        PricingMode::Inclusive => {
            let total_rial = subtotal;
            let tax_rial = if effective_rate_bp == 0 {
                0
            } else {
                let tax_num = i128::from(total_rial)
                    .checked_mul(i128::from(effective_rate_bp))
                    .ok_or("tax multiply overflow")?;
                let denom = 10_000i128 + i128::from(effective_rate_bp);
                positive_half_up(tax_num, denom)?
            };
            let net_rial = total_rial.checked_sub(tax_rial).ok_or("net underflow")?;
            check_money(net_rial)?;
            Ok(LineOutput {
                gross_rial,
                net_rial,
                tax_rial,
                total_rial,
                effective_rate_bp,
            })
        }
    }
}

fn sum_document(lines: &[LineOutput]) -> Result<DocOutput, &'static str> {
    let mut net = 0i64;
    let mut tax = 0i64;
    for l in lines {
        net = net.checked_add(l.net_rial).ok_or("doc net overflow")?;
        tax = tax.checked_add(l.tax_rial).ok_or("doc tax overflow")?;
    }
    check_money(net)?;
    check_money(tax)?;
    let total = net.checked_add(tax).ok_or("doc total overflow")?;
    check_money(total)?;
    Ok(DocOutput {
        net_rial: net,
        tax_rial: tax,
        total_rial: total,
    })
}

/// Cumulative proportional allocation matching `sales::allocation` / `procurement::allocation`.
fn allocate_cumulative(
    orig_value: i64,
    orig_qty_thousandths: i64,
    prior_qty_thousandths: i64,
    prior_value: i64,
    next_qty_thousandths: i64,
) -> Result<i64, &'static str> {
    if orig_qty_thousandths <= 0 || prior_qty_thousandths < 0 || next_qty_thousandths <= 0 || orig_value < 0 || prior_value < 0 {
        return Err("invalid allocation args");
    }
    let cumulative = prior_qty_thousandths
        .checked_add(next_qty_thousandths)
        .ok_or("cumulative qty overflow")?;
    if cumulative > orig_qty_thousandths {
        return Err("cumulative exceeds original");
    }
    let target = if cumulative == orig_qty_thousandths {
        orig_value
    } else {
        let num = i128::from(orig_value)
            .checked_mul(i128::from(cumulative))
            .ok_or("allocation mul overflow")?;
        positive_half_up(num, i128::from(orig_qty_thousandths))?
    };
    target.checked_sub(prior_value).ok_or("allocation underflow")
}

#[test]
fn golden_vat_calculations() {
    struct Case {
        name: &'static str,
        lines: Vec<LineInput>,
        expected_lines: Vec<(i64, i64, i64, u32)>, // (net, tax, total, effective_bp)
        expected_doc: (i64, i64, i64),             // (net, tax, total)
    }

    let cases = vec![
        Case {
            name: "01_single_line_exclusive_exact_10pct",
            lines: vec![LineInput {
                unit_price_rial: 1_000_000,
                quantity_thousandths: 2_000,
                line_discount_rial: 0,
                document_discount_rial: 0,
                category: TaxCategory::Taxable,
                rate_bp: 1000,
                pricing: PricingMode::Exclusive,
            }],
            expected_lines: vec![(2_000_000, 200_000, 2_200_000, 1000)],
            expected_doc: (2_000_000, 200_000, 2_200_000),
        },
        Case {
            name: "02_single_line_exclusive_half_up_tie",
            lines: vec![LineInput {
                unit_price_rial: 15,
                quantity_thousandths: 1_000,
                line_discount_rial: 0,
                document_discount_rial: 0,
                category: TaxCategory::Taxable,
                rate_bp: 1000,
                pricing: PricingMode::Exclusive,
            }],
            expected_lines: vec![(15, 2, 17, 1000)],
            expected_doc: (15, 2, 17),
        },
        Case {
            name: "03_single_line_exclusive_below_half_rounds_down",
            lines: vec![LineInput {
                unit_price_rial: 14,
                quantity_thousandths: 1_000,
                line_discount_rial: 0,
                document_discount_rial: 0,
                category: TaxCategory::Taxable,
                rate_bp: 1000,
                pricing: PricingMode::Exclusive,
            }],
            expected_lines: vec![(14, 1, 15, 1000)],
            expected_doc: (14, 1, 15),
        },
        Case {
            name: "04_thousandths_qty_and_discounts_exclusive",
            lines: vec![LineInput {
                unit_price_rial: 10_003,
                quantity_thousandths: 1_250,
                line_discount_rial: 500,
                document_discount_rial: 100,
                category: TaxCategory::Taxable,
                rate_bp: 1000,
                pricing: PricingMode::Exclusive,
            }],
            // gross = round(10003 * 1250 / 1000) = 12504; net = 12504 - 600 = 11904; tax = round(11904 * 0.1) = 1190
            expected_lines: vec![(11_904, 1_190, 13_094, 1000)],
            expected_doc: (11_904, 1_190, 13_094),
        },
        Case {
            name: "05_multiline_per_line_rounding_and_override",
            lines: vec![
                LineInput {
                    unit_price_rial: 15,
                    quantity_thousandths: 1_000,
                    line_discount_rial: 0,
                    document_discount_rial: 0,
                    category: TaxCategory::Taxable,
                    rate_bp: 1000,
                    pricing: PricingMode::Exclusive,
                },
                LineInput {
                    unit_price_rial: 15,
                    quantity_thousandths: 1_000,
                    line_discount_rial: 0,
                    document_discount_rial: 0,
                    category: TaxCategory::Taxable,
                    rate_bp: 1000,
                    pricing: PricingMode::Exclusive,
                },
                LineInput {
                    unit_price_rial: 100,
                    quantity_thousandths: 1_000,
                    line_discount_rial: 0,
                    document_discount_rial: 0,
                    category: TaxCategory::Taxable,
                    rate_bp: 900,
                    pricing: PricingMode::Exclusive,
                },
            ],
            expected_lines: vec![(15, 2, 17, 1000), (15, 2, 17, 1000), (100, 9, 109, 900)],
            expected_doc: (130, 13, 143),
        },
        Case {
            name: "06_zero_rated_line_ignores_rate",
            lines: vec![LineInput {
                unit_price_rial: 500_000,
                quantity_thousandths: 3_000,
                line_discount_rial: 0,
                document_discount_rial: 0,
                category: TaxCategory::ZeroRated,
                rate_bp: 1000,
                pricing: PricingMode::Exclusive,
            }],
            expected_lines: vec![(1_500_000, 0, 1_500_000, 0)],
            expected_doc: (1_500_000, 0, 1_500_000),
        },
        Case {
            name: "07_exempt_line_ignores_rate",
            lines: vec![LineInput {
                unit_price_rial: 750_000,
                quantity_thousandths: 2_000,
                line_discount_rial: 0,
                document_discount_rial: 0,
                category: TaxCategory::Exempt,
                rate_bp: 1000,
                pricing: PricingMode::Exclusive,
            }],
            expected_lines: vec![(1_500_000, 0, 1_500_000, 0)],
            expected_doc: (1_500_000, 0, 1_500_000),
        },
        Case {
            name: "08_mixed_taxable_zerorated_exempt",
            lines: vec![
                LineInput {
                    unit_price_rial: 1_000_000,
                    quantity_thousandths: 1_000,
                    line_discount_rial: 0,
                    document_discount_rial: 0,
                    category: TaxCategory::Taxable,
                    rate_bp: 1000,
                    pricing: PricingMode::Exclusive,
                },
                LineInput {
                    unit_price_rial: 500_000,
                    quantity_thousandths: 1_000,
                    line_discount_rial: 0,
                    document_discount_rial: 0,
                    category: TaxCategory::ZeroRated,
                    rate_bp: 1000,
                    pricing: PricingMode::Exclusive,
                },
                LineInput {
                    unit_price_rial: 250_000,
                    quantity_thousandths: 1_000,
                    line_discount_rial: 0,
                    document_discount_rial: 0,
                    category: TaxCategory::Exempt,
                    rate_bp: 1000,
                    pricing: PricingMode::Exclusive,
                },
            ],
            expected_lines: vec![
                (1_000_000, 100_000, 1_100_000, 1000),
                (500_000, 0, 500_000, 0),
                (250_000, 0, 250_000, 0),
            ],
            expected_doc: (1_750_000, 100_000, 1_850_000),
        },
        Case {
            name: "09_inclusive_exact_split_10pct",
            lines: vec![LineInput {
                unit_price_rial: 1_100_000,
                quantity_thousandths: 1_000,
                line_discount_rial: 0,
                document_discount_rial: 0,
                category: TaxCategory::Taxable,
                rate_bp: 1000,
                pricing: PricingMode::Inclusive,
            }],
            expected_lines: vec![(1_000_000, 100_000, 1_100_000, 1000)],
            expected_doc: (1_000_000, 100_000, 1_100_000),
        },
        Case {
            name: "10_inclusive_non_exact_split_zero_drift",
            lines: vec![LineInput {
                unit_price_rial: 100_000,
                quantity_thousandths: 1_000,
                line_discount_rial: 0,
                document_discount_rial: 0,
                category: TaxCategory::Taxable,
                rate_bp: 1000,
                pricing: PricingMode::Inclusive,
            }],
            // 100_000 * 1000 / 11_000 = 9090.909... -> 9091; net = 100_000 - 9091 = 90_909
            expected_lines: vec![(90_909, 9_091, 100_000, 1000)],
            expected_doc: (90_909, 9_091, 100_000),
        },
        Case {
            name: "11_inclusive_half_up_tie_20pct",
            lines: vec![LineInput {
                unit_price_rial: 3,
                quantity_thousandths: 1_000,
                line_discount_rial: 0,
                document_discount_rial: 0,
                category: TaxCategory::Taxable,
                rate_bp: 2000,
                pricing: PricingMode::Inclusive,
            }],
            // 3 * 2000 / 12_000 = 0.5 -> 1; net = 3 - 1 = 2
            expected_lines: vec![(2, 1, 3, 2000)],
            expected_doc: (2, 1, 3),
        },
        Case {
            name: "12_inclusive_exempt_line",
            lines: vec![LineInput {
                unit_price_rial: 500_000,
                quantity_thousandths: 1_000,
                line_discount_rial: 0,
                document_discount_rial: 0,
                category: TaxCategory::Exempt,
                rate_bp: 1000,
                pricing: PricingMode::Inclusive,
            }],
            expected_lines: vec![(500_000, 0, 500_000, 0)],
            expected_doc: (500_000, 0, 500_000),
        },
        Case {
            name: "13_large_amount_within_max_money_checked",
            lines: vec![LineInput {
                unit_price_rial: 8_000_000_000_000_000,
                quantity_thousandths: 1_000,
                line_discount_rial: 0,
                document_discount_rial: 0,
                category: TaxCategory::Taxable,
                rate_bp: 1000,
                pricing: PricingMode::Exclusive,
            }],
            expected_lines: vec![(8_000_000_000_000_000, 800_000_000_000_000, 8_800_000_000_000_000, 1000)],
            expected_doc: (8_000_000_000_000_000, 800_000_000_000_000, 8_800_000_000_000_000),
        },
    ];

    for case in cases {
        let outputs: Vec<LineOutput> = case
            .lines
            .iter()
            .map(|&l| compute_line(l).unwrap_or_else(|e| panic!("{}: unexpected error {e}", case.name)))
            .collect();
        assert_eq!(outputs.len(), case.expected_lines.len(), "{}", case.name);
        for (i, (out, &(exp_net, exp_tax, exp_total, exp_bp))) in outputs.iter().zip(&case.expected_lines).enumerate() {
            assert_eq!(
                (out.net_rial, out.tax_rial, out.total_rial, out.effective_rate_bp),
                (exp_net, exp_tax, exp_total, exp_bp),
                "{} line {i}",
                case.name
            );
            assert_eq!(out.net_rial + out.tax_rial, out.total_rial, "{} line {i} sum invariant", case.name);
        }
        let doc = sum_document(&outputs).unwrap_or_else(|e| panic!("{}: doc sum error {e}", case.name));
        assert_eq!((doc.net_rial, doc.tax_rial, doc.total_rial), case.expected_doc, "{}", case.name);
    }
}

#[test]
fn golden_partial_return_allocation_and_overflow_guards() {
    // Case 14: Partial return of a line in two steps (1.000 then 2.000 of 3.000)
    let orig_qty = 3_000;
    let orig_net = 100_001;
    let orig_tax = 10_000;

    let net_1 = allocate_cumulative(orig_net, orig_qty, 0, 0, 1_000).unwrap();
    let tax_1 = allocate_cumulative(orig_tax, orig_qty, 0, 0, 1_000).unwrap();
    assert_eq!((net_1, tax_1), (33_334, 3_333));

    let net_2 = allocate_cumulative(orig_net, orig_qty, 1_000, net_1, 2_000).unwrap();
    let tax_2 = allocate_cumulative(orig_tax, orig_qty, 1_000, tax_1, 2_000).unwrap();
    assert_eq!((net_2, tax_2), (66_667, 6_667));
    assert_eq!(net_1 + net_2, orig_net);
    assert_eq!(tax_1 + tax_2, orig_tax);

    // Case 15: Overflow guards near MAX_MONEY and i64::MAX
    let over_max_money = compute_line(LineInput {
        unit_price_rial: 8_500_000_000_000_000,
        quantity_thousandths: 1_000,
        line_discount_rial: 0,
        document_discount_rial: 0,
        category: TaxCategory::Taxable,
        rate_bp: 1000,
        pricing: PricingMode::Exclusive,
    });
    assert!(over_max_money.is_err());

    let at_i64_max = compute_line(LineInput {
        unit_price_rial: i64::MAX,
        quantity_thousandths: 1_000,
        line_discount_rial: 0,
        document_discount_rial: 0,
        category: TaxCategory::Taxable,
        rate_bp: 1000,
        pricing: PricingMode::Exclusive,
    });
    assert!(at_i64_max.is_err());
}
