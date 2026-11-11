# Sarv handoff

The desktop is `crates/sarv-desktop`, a Rune application. The accounting engine and `sarv-agent` CLI are `crates/sarv-quote`. The React, Tauri, and GTK shells have been removed.

```bash
cargo run -p sarv-desktop
cargo test -p sarv-desktop --bin sarv-desktop
cargo test -p sarv-quote
cargo test -p sarv-quote --features pdf-export
```

`SARV_DATA_DIR` selects the books. `SARV_DEMO=1` opens a throwaway fixture. Agents are configured from the CLI. Law data is a read-only notice.

See [README](../README.md) and [AGENTS.md](../AGENTS.md).
