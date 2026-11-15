# Plan 025: Every report and document can be exported, and the user can find the file

> **Executor instructions**: Follow this plan step by step. Run every verification command. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-desktop/src/ui/documents.rs crates/sarv-desktop/src/ui/views/reports.rs crates/sarv-desktop/src/ui/views/purchases.rs crates/sarv-desktop/src/ui/views/sales.rs crates/sarv-desktop/src/ui/shell.rs crates/sarv-desktop/src/app.rs`

## Status

- **Priority**: P2
- **Effort**: M
- **Risk**: LOW
- **Depends on**: 003 (documents use the held store), 011 (tone-aware toasts) recommended
- **Category**: direction (feature)
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

Owners hand printed or exported reports to their accountant, the bank and the tax office. The engine can already produce PDFs and CSVs for sales documents, purchase documents, the trial balance, opening balances and reconciliations — with embedded Persian fonts and tested output. The desktop wires up only two of them: sales/quotation PDFs, and a trial-balance + P&L CSV. The balance sheet, aging, ledger and all purchase documents cannot be exported at all. And when an export does succeed, the only feedback is a toast containing a bare file path inside the app's data folder — the user is not offered a way to open the file or its folder.

## Current state

- `crates/sarv-desktop/src/ui/documents.rs` (45 lines) — writes into `<books dir>/exports/` and returns the `PathBuf`:
  ```rust
  pub fn write_sales_pdf(st: &State, document_id: &str) -> Result<PathBuf, String> {
      …
      write(st, "invoice", document_id, move |state, path| {
          sarv_quote::sales_export::printable_document_pdf(state, &company, &id, language, path).map_err(|e| e.to_string())
      })
  }
  ```
  File names: `{stem}-{safe_name(id)}.pdf`. The engine's writers refuse to overwrite an existing file (`write_new`), so exporting the same document twice fails with "file exists" — check by calling it twice in a test.
- Engine export functions (all `pub`; the desktop builds with the `pdf-export` feature, `crates/sarv-desktop/Cargo.toml`):
  - `sales_export.rs`: `document_csv`, `document_pdf`, `reconciliation_csv`, `reconciliation_pdf` (lines 1372-1429), plus `printable_document_pdf`, `printable_quotation_pdf`.
  - `procurement_export.rs`: `document_csv` (1805), `document_pdf` (1824), `reconciliation_csv` (1843), `reconciliation_pdf` (1862).
  - `accounting_export.rs`: `trial_csv` (39), `trial_csv_with_dates` (48), `trial_pdf` (145), `trial_pdf_with_dates` (154).
  - `core_export.rs`: `opening_csv` (323), `opening_pdf` (534).
  Read each signature before calling (`sed -n '<line>,+18p'`); they differ in parameters (state, company id, record id / date range, language or preferences, output path).
- Reports CSV today — `crates/sarv-desktop/src/ui/views/reports.rs:311-341` `export_csv`: calls actions `report_csv` and `profit_csv` (which return `{filename, content}`) and writes them under `<books dir>/exports/`. Report tabs: trial balance, P&L, parties, balance sheet, aging.
- CSV cells must be made safe against spreadsheet formula injection; the engine has `safe_cell` (`crates/sarv-quote/src/export.rs:7`) and a duplicate `csv_safe` in `desktop_bridge.rs` (~1875). Any new CSV must use one of them.
- Toasts: `State::toast(tone, message)`; `Toast` struct is `{id, tone, message}` (`crates/sarv-desktop/src/app.rs:124-129`); rendered in `ui/shell.rs:397-411` with rune-kit `Toast::new(key, message).lead(icon).on_close(..)`. rune-kit `Toast` also has `.action(...)` (`rune-kit/src/toast.rs:35`) — read its signature.
- Opening a file or folder with the OS: no helper exists. Use `std::process::Command` with `xdg-open` (Linux), `open` (macOS), `cmd /C start "" <path>` (Windows) — spawn and do not wait; ignore failure with a toast.
- Sales print action: `crates/sarv-desktop/src/ui/views/sales.rs:805-819` (`print_selected`).

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |
| Engine export tests | `cargo test -p sarv-quote --features pdf-export` | all pass (needs `poppler-utils`) |

## Scope

**In scope**: `crates/sarv-desktop/src/ui/documents.rs`, `src/app.rs` (toast action), `src/ui/shell.rs`, `src/ui/views/{reports,purchases,sales,accounting,settings}.rs`, `crates/sarv-quote/src/desktop_bridge.rs` (new CSV actions for balance sheet, aging, ledger), `crates/sarv-quote/tests/desktop_bridge.rs`, `src/fa_messages.rs`, `src/main.rs` (tests).

**Out of scope**: new PDF layouts (balance-sheet/aging PDFs do not exist in the engine; CSV only for those); a native "Save as…" file dialog unless the pinned Rune exposes one (check `grep -rn "save_dialog\|file_dialog\|pick_file" ~/.cargo/git/checkouts/rune-*/b556a38/crates/rune/src | head`); direct printing to a printer.

## Git workflow

- Branch `advisor/025-exports-and-printing`; commits `feat(desktop): open exported files from the toast`, `feat(desktop): purchase and trial-balance PDFs`, `feat(engine): CSV for balance sheet, aging and ledger`, `feat(desktop): export every report tab`.

## Steps

### Step 1: Export toasts the user can act on

- Extend `Toast` with `pub open_path: Option<PathBuf>`; add `State::toast_file(&mut self, path: PathBuf)` which toasts `"{Saved} {file name}"` with `Tone::Success` and sets `open_path`.
- In `mount_toasts`, when `open_path` is set add two actions through the kit's `.action`: "Open" (opens the file) and "Show in folder" (opens the parent directory). Implement `pub fn reveal(path: &Path, folder: bool)` in `documents.rs` with the per-OS commands above.
- Unique names: append a timestamp when the target exists — change `documents::write` to try `{stem}-{id}.pdf`, then `{stem}-{id}-{HHMMSS}.pdf`.
- Replace `st.toast(Tone::Success, path.display().to_string())` in `sales.rs:816`, the backup filename toast in `settings.rs:206` and the CSV export toast in `reports.rs` with `toast_file`.

If a native save dialog exists in Rune, add a Settings preference "Ask where to save exports" (default off) and use it; otherwise add a Settings row "Exports folder" showing `<books dir>/exports` with a "Show in folder" button.

### Step 2: Documents

Add to `documents.rs`, following the existing two functions: `write_purchase_pdf(st, record_id)` → `procurement_export::document_pdf`; `write_trial_balance_pdf(st, from, to)` → `accounting_export::trial_pdf_with_dates`; `write_sales_document_csv`, `write_purchase_document_csv`. Pass language/preferences the same way the sales call does.

Wire: Purchases header action "Print" (enabled with a selection); Sales keeps "Print"; if plan 019's detail sheet exists, add Print there too.

### Step 3: Report CSVs in the engine

Following the `report_csv` / `profit_csv` arms (`desktop_bridge.rs` ~1822-1862: build with `csv::Writer`, return `{"filename", "content"}`), add:

- `balance_sheet_csv` — from `report(&store.state, &p)?["balance_sheet"]` rows.
- `aging_csv` — receivable and payable aging rows with their buckets (the engine computes buckets in `statements.rs`; include them).
- `ledger_csv` — the `ledger` rows for `{account_id, from, to}` **without** the 100-row cap, with a running balance if plan 019 landed.

Every text cell through `csv_safe`/`safe_cell`. Headers in English like the existing two (they are data files; keep consistent). Tests in `tests/desktop_bridge.rs`: each action returns non-empty content whose header row matches, row count equals the report's row count, and a party named `=HYPERLINK("x")` is neutralised (look at how `tests/export_hardening.rs` asserts that).

### Step 4: Export from every report tab

Replace the single "Export" behaviour in `reports.rs` with a per-tab export: the active tab decides which action(s) run (trial → `report_csv` + trial-balance PDF; P&L → `profit_csv`; balance sheet → `balance_sheet_csv`; aging → `aging_csv`). The ledger dialog (Accounting) gets an "Export CSV" footer button → `ledger_csv`. Write files with a shared helper `fn save_text_export(st, filename, content) -> Result<PathBuf, String>` (extract from `export_csv`) and finish with `toast_file`.

### Step 5: Strings and tests

Persian: `("Open", "باز کردن")`, `("Show in folder", "نمایش در پوشه")`, `("Print", "چاپ")` (check existing), `("Export CSV", "خروجی CSV")`, `("Exports folder", "پوشهٔ خروجی‌ها")`.

Tests (`main.rs`; pattern `test_export_writes_csv_and_language_persists` at line 660):

1. `test_export_balance_sheet_and_aging_csv` — files exist under `exports/` and are non-empty.
2. `test_exporting_the_same_invoice_twice_succeeds` — two PDFs with different names.
3. `test_purchase_pdf_is_written` — after a purchase flow.
4. `test_export_toast_carries_the_path` — `rt.state.toasts.last().unwrap().open_path.is_some()`.

Do not spawn `xdg-open` in tests: guard `reveal` so it is only called from the click handlers.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass; `cargo test -p sarv-quote` → all pass.

## Done criteria

- [ ] `grep -rn "path.display().to_string()" crates/sarv-desktop/src/ui/views` returns nothing
- [ ] `grep -n '"balance_sheet_csv"\|"aging_csv"\|"ledger_csv"' crates/sarv-quote/src/desktop_bridge.rs` → three arms
- [ ] `grep -rn "procurement_export::document_pdf\|trial_pdf" crates/sarv-desktop/src` matches
- [ ] Both suites pass with the new tests
- [ ] `plans/README.md` updated

## STOP conditions

- An engine export function needs inputs the desktop cannot supply without re-deriving accounting data (e.g. a pre-built reconciliation struct). Skip that export and report its signature.
- PDF generation takes long enough to freeze the UI noticeably (> ~1 s for an ordinary invoice). Report timings; moving exports to a worker thread is a separate plan.

## Maintenance notes

- Exports accumulate in `<books dir>/exports`; they are not part of backups. Say so next to the "Exports folder" row.
- When new report tabs are added, add their CSV action at the same time — the per-tab `match` should have no silent default.
