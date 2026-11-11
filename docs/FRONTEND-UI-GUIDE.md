# Sarv desktop

The desktop is `crates/sarv-desktop`, a Rune and rune-kit application. It calls `sarv_quote::desktop::dispatch` in-process. Ledger rules stay in `crates/sarv-quote`.

```bash
cargo run -p sarv-desktop
cargo test -p sarv-desktop --bin sarv-desktop
```

The window uses rune-kit’s neutral theme: sidebar, toolbar, tables, dialogs, and fields. Persian is the default, with Jalali dates as a display of Gregorian ISO values stored in the books. First run is a welcome, then the company and fiscal year, then opening cash and bank. The daily shell appears after the books are open.

There is no React, Tauri, or browser shell in this repository.
