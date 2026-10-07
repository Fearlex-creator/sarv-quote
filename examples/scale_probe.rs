//! Bounded synthetic release-pipeline probe, deliberately independent of native GUI.
//! Each invocation owns a fresh output directory and monitors its own Linux /proc RSS.
use anyhow::{Context, Result, ensure};
use sarv_quote::{domain::*, export, imports, store::Store};
use serde::Serialize;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
const LIMIT_RSS_KIB: u64 = 512 * 1024;
const LIMIT_SECONDS: u64 = 60;
#[derive(Serialize)]
struct Measurement {
    phase: String,
    outcome: &'static str,
    elapsed_ms: f64,
    sampled_phase_peak_rss_kib: u64,
    kernel_process_peak_rss_kib: u64,
    ending_rss_kib: u64,
    error: Option<String>,
}
fn memory() -> (u64, u64) {
    let status = fs::read_to_string("/proc/self/status").unwrap_or_default();
    let field = |name: &str| {
        status
            .lines()
            .find_map(|line| {
                line.strip_prefix(name)?
                    .split_whitespace()
                    .next()?
                    .parse::<u64>()
                    .ok()
            })
            .unwrap_or(0)
    };
    (field("VmRSS:"), field("VmHWM:"))
}
fn append(path: &Path, value: &impl Serialize) -> Result<()> {
    let mut file = OpenOptions::new().append(true).create(true).open(path)?;
    serde_json::to_writer(&mut file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}
fn measure<T>(log: &Path, phase: &str, work: impl FnOnce() -> Result<T>) -> Result<T> {
    let start = Instant::now();
    let finished = Arc::new(AtomicBool::new(false));
    let sampled_peak = Arc::new(AtomicU64::new(memory().0));
    let worker_finished = finished.clone();
    let worker_peak = sampled_peak.clone();
    let watchdog_log = log.to_path_buf();
    let watchdog_phase = phase.to_string();
    let monitor = thread::spawn(move || {
        while !worker_finished.load(Ordering::Acquire) {
            let (rss, hwm) = memory();
            worker_peak.fetch_max(rss, Ordering::Relaxed);
            if start.elapsed() > Duration::from_secs(LIMIT_SECONDS)
                || hwm > LIMIT_RSS_KIB
                || rss > LIMIT_RSS_KIB
            {
                let failure = Measurement {
                    phase: watchdog_phase,
                    outcome: "bounded_stop",
                    elapsed_ms: start.elapsed().as_secs_f64() * 1000.0,
                    sampled_phase_peak_rss_kib: worker_peak.load(Ordering::Relaxed),
                    kernel_process_peak_rss_kib: hwm,
                    ending_rss_kib: rss,
                    error: Some(
                        "phase exceeded 60 seconds or process peak RSS exceeded 512 MiB".into(),
                    ),
                };
                let _ = append(&watchdog_log, &failure);
                eprintln!("bounded_stop: {}", failure.phase);
                std::process::exit(124);
            }
            thread::sleep(Duration::from_millis(10));
        }
    });
    let result = work();
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    let (rss, hwm) = memory();
    sampled_peak.fetch_max(rss, Ordering::Relaxed);
    finished.store(true, Ordering::Release);
    monitor
        .join()
        .map_err(|_| anyhow::anyhow!("RSS monitor panicked"))?;
    let measurement = Measurement {
        phase: phase.into(),
        outcome: if result.is_ok() { "passed" } else { "failed" },
        elapsed_ms,
        sampled_phase_peak_rss_kib: sampled_peak.load(Ordering::Relaxed),
        kernel_process_peak_rss_kib: hwm,
        ending_rss_kib: rss,
        error: result.as_ref().err().map(|error| format!("{error:#}")),
    };
    append(log, &measurement)?;
    println!(
        "{}: {} {:.2} ms; kernel lifetime peak {} KiB",
        phase, measurement.outcome, elapsed_ms, hwm
    );
    ensure!(
        elapsed_ms <= LIMIT_SECONDS as f64 * 1000.0 && hwm <= LIMIT_RSS_KIB,
        "phase exceeded resource bound"
    );
    result
}
fn fixture(products: usize, source: &str) -> Result<String> {
    let mut csv = csv::Writer::from_writer(Vec::new());
    csv.write_record([
        "code",
        "name",
        "measure",
        "price",
        "currency",
        "source_at",
        "aliases",
    ])?;
    for index in 0..products {
        let long = if index < 1000 {
            "محصول آزمایشی با شرح طولانی برای بررسی خوانایی جدول و صفحه‌بندی چاپ، بدون هیچ دادهٔ واقعی مشتری؛ کالا و قیمت فقط برای آزمون عملکرد هستند. "
        } else {
            "محصول آزمایشی "
        };
        csv.write_record([
            format!("SYN-{index:05}"),
            format!("{long}{}", fa(index)),
            "عدد".into(),
            fa(100_000 + index),
            "rial".into(),
            source.into(),
            format!("نام مستعار {}", fa(index)),
        ])?;
    }
    Ok(String::from_utf8(csv.into_inner()?)?)
}
fn order(count: usize) -> Result<String> {
    let mut csv = csv::Writer::from_writer(Vec::new());
    csv.write_record(["item", "quantity"])?;
    for index in 0..count {
        csv.write_record([format!("syn-{}", fa(format!("{index:05}"))), "۱٫۲۵۰".into()])?;
    }
    Ok(String::from_utf8(csv.into_inner()?)?)
}
fn expected_total(count: usize) -> i64 {
    (0..count)
        .map(|index| ((100_000 + index as i64) * 5 + 2) / 4)
        .sum()
}
fn run(products: usize, directory: &Path) -> Result<()> {
    ensure!(!cfg!(debug_assertions), "run this probe with --release");
    ensure!(
        [1000, 10_000, 50_000].contains(&products),
        "supported product tiers: 1000, 10000, 50000"
    );
    ensure!(
        !directory.exists(),
        "output directory already exists; use a fresh path"
    );
    fs::create_dir_all(directory)?;
    let log = directory.join("measurements.jsonl");
    let environment = serde_json::json!({
        "started_at": now(), "synthetic_only": true, "native_gui_measured": false,
        "products": products, "profile": "release", "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH, "pid": std::process::id(),
        "kernel": fs::read_to_string("/proc/sys/kernel/osrelease").ok().map(|value|value.trim().to_string()),
        "cpu": fs::read_to_string("/proc/cpuinfo").ok().and_then(|text|text.lines().find(|line|line.starts_with("model name")).map(str::to_owned)),
        "logical_parallelism": std::thread::available_parallelism().ok().map(|count|count.get()),
        "rss_sampling_interval_ms": 10, "phase_seconds_limit": LIMIT_SECONDS,
        "process_peak_rss_limit_kib": LIMIT_RSS_KIB,
        "peak_rss_note": "VmHWM is the actual kernel process-lifetime high-water RSS, cumulative across phases; per-phase RSS is sampled and may miss short peaks. Page-cache coldness is not controlled."
    });
    fs::write(
        directory.join("environment.json"),
        serde_json::to_vec_pretty(&environment)?,
    )?;
    let source = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
    let input = measure(&log, "generate_synthetic_catalog_csv", || {
        fixture(products, &source)
    })?;
    ensure!(
        input.len() <= imports::MAX_IMPORT,
        "synthetic CSV exceeds application import limit"
    );
    fs::write(directory.join("catalog.csv"), &input)?;
    let catalog = measure(&log, "catalog_csv_parse_and_fingerprint", || {
        imports::catalog_csv(&input, "فهرست مصنوعی سنجش عملکرد")
    })?;
    let mut state = State::default();
    state.settings.business_name = "پروندهٔ مصنوعی سنجش عملکرد".into();
    let catalog_id = measure(&log, "catalog_accept_import_and_validate", || {
        imports::accept_catalog(&mut state, catalog.clone())
    })?;
    ensure!(
        state.draft.catalog_id == catalog_id,
        "catalog not selected after import"
    );
    measure(&log, "catalog_state_validate", || state.validate())?;
    measure(&log, "duplicate_catalog_rejection", || {
        let before = state.catalogs.len();
        ensure!(
            imports::accept_catalog(&mut state, catalog.clone()).is_err(),
            "duplicate was accepted"
        );
        ensure!(
            state.catalogs.len() == before,
            "duplicate import mutated state"
        );
        Ok(())
    })?;
    let lookup = measure(&log, "compile_borrowed_catalog_index", || {
        Ok(catalog.index())
    })?;
    measure(&log, "indexed_exact_code_matches_100", || {
        for index in 0..100 {
            let index = index * (products / 100);
            let matches = lookup.matches(&format!("syn-{}", fa(format!("{index:05}"))));
            ensure!(
                matches.len() == 1 && matches[0].code == format!("SYN-{index:05}"),
                "indexed exact matching failed"
            );
        }
        Ok(())
    })?;
    measure(&log, "indexed_approved_alias_matches_100", || {
        for index in 0..100 {
            ensure!(
                lookup.matches(&format!("نام مستعار {}", fa(index))).len() == 1,
                "indexed alias matching failed"
            );
        }
        Ok(())
    })?;
    measure(&log, "exact_code_matches_100", || {
        for index in 0..100 {
            let index = index * (products / 100);
            let matches = catalog.matches(&format!("syn-{}", fa(format!("{index:05}"))));
            ensure!(
                matches.len() == 1 && matches[0].code == format!("SYN-{index:05}"),
                "exact matching failed"
            );
        }
        Ok(())
    })?;
    measure(&log, "approved_alias_matches_100", || {
        for index in 0..100 {
            ensure!(
                catalog.matches(&format!("نام مستعار {}", fa(index))).len() == 1,
                "approved alias matching failed"
            );
        }
        Ok(())
    })?;
    measure(&log, "search_equivalent_normalization_filters_10", || {
        let queries = [
            "SYN-000",
            "syn-۰۱۲",
            "محصول",
            "آزمايشي",
            "نام مستعار ۱۲",
            "syn-099",
            "synthetic-missing",
            "قیمت",
            "کالا",
            "پایان",
        ];
        for query in queries {
            let needle = normalize(query);
            let count = catalog
                .products
                .iter()
                .filter(|product| {
                    normalize(&product.code).contains(&needle)
                        || normalize(&product.name).contains(&needle)
                        || normalize(&product.measure).contains(&needle)
                        || normalize(&product.source_at).contains(&needle)
                        || product
                            .aliases
                            .iter()
                            .any(|alias| normalize(alias).contains(&needle))
                })
                .count();
            if query == "محصول" || query == "آزمايشي" {
                ensure!(count == products, "normalized name search omitted products");
            }
            if query == "synthetic-missing" {
                ensure!(count == 0, "missing search unexpectedly matched");
            }
            std::hint::black_box(count);
            // This measures normalization filters only, not creation/rendering of native rows.
        }
        Ok(())
    })?;
    drop(lookup);
    let profile = directory.join("profile");
    let mut store = measure(&log, "store_open_empty_profile", || Store::open(&profile))?;
    measure(&log, "store_persist_imported_catalog", || {
        store.change(|current| {
            *current = state.clone();
            Ok(())
        })
    })?;
    let mut approved = Vec::new();
    for lines in [100, 1000] {
        let text = order(lines)?;
        fs::write(directory.join(format!("order-{lines}.csv")), &text)?;
        let parsed = measure(&log, &format!("order_csv_parse_{lines}"), || {
            imports::order_csv(&text)
        })?;
        state.new_draft()?;
        state.draft.customer = format!("مشتری مصنوعی سنجش {lines} ردیف");
        state.draft.lines = parsed;
        let preview = measure(
            &log,
            &format!("quote_preview_exact_match_and_money_{lines}"),
            || state.preview(),
        )?;
        ensure!(
            preview.total_rial == expected_total(lines),
            "quote total differs from independent quarter-rial expectation"
        );
        let quote_id = measure(&log, &format!("quote_domain_approval_{lines}"), || {
            state.approve()
        })?;
        measure(
            &log,
            &format!("repeated_approval_idempotence_{lines}"),
            || {
                let count = state.quotes.len();
                ensure!(
                    state.approve()? == quote_id && state.quotes.len() == count,
                    "repeated approval was not idempotent"
                );
                Ok(())
            },
        )?;
        measure(&log, &format!("approved_state_validate_{lines}"), || {
            state.validate()
        })?;
        measure(
            &log,
            &format!("store_persist_approved_quote_{lines}"),
            || {
                store.change(|current| {
                    *current = state.clone();
                    Ok(())
                })
            },
        )?;
        let quote = state
            .quotes
            .iter()
            .find(|quote| quote.id == quote_id)
            .context("approved quote missing")?;
        measure(&log, &format!("quote_csv_export_{lines}"), || {
            export::csv(quote, &directory.join(format!("quote-{lines}.csv")))
        })?;
        measure(&log, &format!("quote_pdf_export_{lines}"), || {
            export::pdf(quote, &directory.join(format!("quote-{lines}.pdf")))
        })?;
        approved.push(quote_id);
    }
    measure(
        &log,
        "saved_customer_quote_search_equivalent_filters_10",
        || {
            for query in [
                "مشتری",
                "مصنوعي",
                "SYN-000",
                "syn-۰۱۲",
                "Q-1",
                "پیشنهاد",
                "synthetic-missing",
                "آزمایشی",
                "ردیف",
                "فهرست",
            ] {
                let needle = normalize(query);
                let customers = state
                    .customers
                    .iter()
                    .filter(|customer| {
                        normalize(&customer.name).contains(&needle)
                            || normalize(&customer.phone).contains(&needle)
                    })
                    .count();
                let quotes = state
                    .quotes
                    .iter()
                    .filter(|quote| {
                        [
                            &quote.customer,
                            &quote.phone,
                            &quote.id,
                            &quote.reference,
                            &quote.number,
                            &quote.catalog_name,
                            &quote.approved_at,
                        ]
                        .iter()
                        .any(|value| normalize(value).contains(&needle))
                            || quote.lines.iter().any(|line| {
                                normalize(&line.code).contains(&needle)
                                    || normalize(&line.name).contains(&needle)
                            })
                    })
                    .count();
                if query == "مشتری" {
                    ensure!(
                        customers == 2 && quotes == 2,
                        "saved-document name search failed"
                    );
                }
                if query == "synthetic-missing" {
                    ensure!(
                        customers == 0 && quotes == 0,
                        "saved-document missing search matched"
                    );
                }
                std::hint::black_box((customers, quotes));
            }
            Ok(())
        },
    )?;
    // Leave a usable native-UI profile with 100 editable order rows, preserving both archived exports.
    state.new_draft()?;
    state.draft.customer = "مشتری مصنوعی پروندهٔ بررسی رابط".into();
    state.draft.lines = imports::order_csv(&order(100)?)?;
    measure(&log, "persist_native_ui_fixture_100_order_rows", || {
        store.change(|current| {
            *current = state.clone();
            Ok(())
        })
    })?;
    let backup = directory.join("profile-backup.json");
    measure(&log, "committed_state_backup", || store.backup(&backup))?;
    let restored = measure(&log, "backup_restore_preview_checksum_and_validate", || {
        Store::restore_preview(&backup)
    })?;
    ensure!(
        restored.catalogs[0].products.len() == products && restored.draft.lines.len() == 100,
        "backup differs from fixture"
    );
    measure(&log, "safe_restore_with_safety_backup", || {
        store.restore(restored).map(|_| ())
    })?;
    drop(store);
    let reopened = measure(&log, "store_reopen_parse_and_validate", || {
        Store::open(&profile)
    })?;
    ensure!(
        reopened
            .state
            .quotes
            .iter()
            .map(|quote| quote.id.clone())
            .collect::<Vec<_>>()
            == approved,
        "reopen lost approved quotes"
    );
    ensure!(
        reopened.state.draft.lines.len() == 100
            && reopened.state.catalogs[0].products.len() == products,
        "reopen differs from fixture"
    );
    let summary = serde_json::json!({
        "outcome": "passed", "finished_at": now(), "products": products,
        "catalog_csv_bytes": input.len(), "quote_line_tiers": [100, 1000],
        "profile_directory": profile, "backup": backup,
        "kernel_process_peak_rss_kib": memory().1,
        "native_gui": "unavailable_in_this_probe; root owns native UI measurement",
        "scope": "one fresh-process synthetic run per tier; warm filesystem/font caches and concurrent machine load may affect results; no SLA established"
    });
    fs::write(
        directory.join("summary.json"),
        serde_json::to_vec_pretty(&summary)?,
    )?;
    Ok(())
}
fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let products = args
        .next()
        .context("usage: scale_probe <1000|10000|50000> <fresh-output-directory>")?
        .parse::<usize>()?;
    let directory = PathBuf::from(args.next().context("output directory required")?);
    run(products, &directory)
}
