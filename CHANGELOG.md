# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-11-20

### Added
- **Year-End Closing (ADR-0009)**: Preview-then-post fiscal year closing journal (`JournalKind::Closing`) that zeroes Revenue and Expense accounts to Retained Earnings, locks the closed fiscal period, and excludes closing journals from Profit & Loss reports.
- **Bank Statement Reconciliation (ADR-0011)**: Append-only reconciliation sessions (`ReconciliationRegister`) with statement date/balance, line matching, live difference calculation, zero-difference completion guard, and reopen support in Banking.
- **VAT on Sales & Purchases (ADR-0010)**: `VatReceivable` (`1402`) and `VatPayable` (`2501`) core roles, schema 7→8 additive migration, user-entered VAT settings (`vat_registered` + default VAT rate % — never wired to read-only `iran::law`), integer-rial Exclusive/Inclusive line math, VAT journal posting on sales/returns and deductible/non-deductible purchase bills/returns, and period VAT totals in Reports.
- **SQL Server Books Migration**: Preview-and-import contacts and products from Holoo, Sepidar, Hesabfa, and generic `.sql`/`.csv` exports (UTF-8/UTF-16 auto-detection, idempotent duplicate skipping).
- **Minimal Rune Desktop UI Redesign**: Redesigned sidebar (3 focused groups, 2-row footer), 4-tab Settings, hero KPI cards, inline contextual row actions without layout shift, and calm visual hierarchy across all views.

### Security
- Upgraded app-lock key derivation from PBKDF2-HMAC-SHA256 to **Argon2id** (`LockRecord` v3) with transparent v1/v2 migration preserving the workspace DEK.
- Replaced hand-rolled encryption with audited **ChaCha20-Poly1305 AEAD** (`SARVENC2`) binding canonical filenames as AAD (retaining `SARVENC1` read support for seamless vault migration).
- Switched constant-time comparisons to `subtle::ConstantTimeEq`, wrapped active DEKs and derived keys in `zeroize::Zeroizing`, raised minimum password length to 8 characters, hardened atomic file permissions (`0o600` + `sync_all`), added SQLite WAL truncation checkpointing before sealing, and added full in-memory wipe on lock and auto-seal on drop.

### Fixed
- Fixed 10× Toman/Rial sales draft display/edit mismatch, `editing_revision` propagation, payment error propagation, Purchases Receive&Bill retry resume, dual-role contact balance, overlay z-order vs Escape priority, dirty-sheet navigation guards, and RTL bidi isolation for negative numbers and Latin tokens.
- Split Windows release workflow builds (`sarv-desktop` + `sarv-agent`), added Windows CI check job, upgraded workspace MSRV to Rust 1.88, updated dependencies, and pruned 571 dead legacy localization keys.

### Notes before general production rollout
- Windows release binaries are currently unsigned (SmartScreen warning expected until code-signing certificate is configured).
- Moadian tax-invoice submission is out of scope for `0.3.0`.
- Iranian statutory VAT/closing treatment should be reviewed by a qualified local accountant before relying on output for tax filings.

## [0.2.0] - 2026-11-15

### Added
- Native Rune + `rune-kit` desktop UI with Sarv teal identity, RTL/LTR layout, Jalali/Gregorian calendars, and Persian/English localization.
- Long-lived SQLite store with external-write refresh and backup recovery screen when books fail to load.
- OS-appropriate data directory resolution (`SARV_DATA_DIR`, XDG / Application Support / AppData) with private permissions and atomic writes.
- Multi-company setup and switcher, fiscal year creation and switching, and chart of accounts management.
- Full document lifecycle (quotations, sales, purchases, partial deliveries/receipts, returns), multi-line journals, cheque register with endorsements, and payment reversals.
- History paging, record detail drawers, full account ledger, sortable tables, keyboard shortcuts, and command palette.
- PDF and CSV exports for documents and reports with toast open/reveal actions, Persian amount-in-words, advisory Iranian ID hints, and read-only law data notice.
- Local append-only logging (`sarv.log`), crash report files on panic with unseen-crash toast, `--version` flag, and Settings About section.

### Fixed
- Strict money input parsing, deterministic request keys per dialog open, confirmation guards for irreversible actions, and engine idempotency/panic hardening.

## [0.1.0]

- Initial tagged release (`v0.1.0`).
