# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
