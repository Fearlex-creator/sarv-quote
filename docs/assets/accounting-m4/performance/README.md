# Frozen M4 freeze3 performance observations

This bundle retains the original, single repeated measurement and the unchanged single full quotation scale run. It has not been regenerated to hide strict misses. Performance acceptance is partial; see [the report](../../../../requirements/performance-M4.md).

- `performance.json`: all n10 startup and n20 real-key search samples, exact native arithmetic proofs, declared hardware, operation phase checks and ordered source/observer provenance before/after.
- `scale-performance.json` and `native-scale-verification.json`: unchanged full scale harness result, 15 functional checks, all heartbeat gaps, page-click samples, named durations and main-thread traces. Functional PASS does not imply strict budget PASS.
- `source-provenance.json`: authoritative UTF-8 path aggregate, historical helper order separately, executable/observer hashes and sequential run scheduling limits.
- `core-performance-phases.jsonl`, `procurement-stage-phases.jsonl`, `procurement-full-phases.jsonl`: every measured phase/gap/tick/result, including a rejected receipt preview and the missed 50.761-ms resource post.
- `application-warning-check.json`, `logs/`, `observer-*.log`: original application logs and observer notices, distinguished explicitly. Application warning/critical count is zero.
- `M4-synthetic-prices.csv`, stage/verification JSON: catalogue bytes and native fixture evidence. No SQLite data is included.
- `screenshots/`: selected native observation endpoints. Screenshots establish neither financial correctness nor visual acceptance on their own.
- `latency-samples.svg`/`.png`, `render-plots.py`: standalone sample plots, generated from retained raw values.

Executable: `sarv-M4-freeze3`, SHA-256 `6eed73ccede51feadd3353393315667329f8136205ad46c973c9e9ece6da44b4`. Production aggregate: `ec50e593693bff141271b490cf2cce74005c3b3117324d7285fa9e6f631b56b9`.

Reproduce with `scripts/native-performance-M4.py` and unchanged `scripts/native-performance-M2.py` after environment activation, private `dbus-run-session` and a fresh ignored output directory. The repeated startup observer requires the preserved synthetic M3 scale seed, which is excluded here along with executables, runtime caches and SQLite files. Source and observer hash assertions prevent silently attributing changed code to this run.

No physical/cache-cold, dedicated SSD/reference hardware, production scale, Windows/Wayland, tax compliance or production readiness claim follows. Nearest-rank p99 equals worst at n10/n20; n1 phase/workflow points are not tail estimates. An elapsed asynchronous navigation duration is not proof of blocking. Global/PDF-busy gaps include synchronous probe screenshots and current-stage attribution after transitions; exact causal isolation remains incomplete.
