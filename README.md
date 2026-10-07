# سرو — پیشنهاد قیمت v1

Free, offline distributor quotation preparation. Application and business logic are written in Rust; the interface uses native GTK4/libadwaita, with Persian shaping/RTL through GTK/Pango. No web frontend, hosted services, AI matching, credentials or subscriptions.

Current interface and workflow continuation, rendered screenshots and final validation are recorded in [docs/CONTINUATION.md](docs/CONTINUATION.md).

The application has a native libadwaita shell, working customer/product/quote managers and persistent settings. Current native release evidence is in [docs/VERIFICATION.md](docs/VERIFICATION.md); physical printing and production-scale field use remain unverified.

This is a quotation v1. Approval creates an immutable local offer, **not** an invoice, posted accounting entry, inventory reservation, payment or tax filing. A saved or exported offer has not been sent to or accepted by a customer. It makes no statutory compliance claim.

## Run locally

```bash
cd sarv-quote
./target/release/sarv-quote
```

The release binary is built locally. A new data profile starts empty; subsequent launches reopen its saved state. Normal launch uses `data/sarv.sqlite3` inside this project. Set `SARV_DATA_DIR=/absolute/path` to choose a different persistent location. Do not run two different data profiles simultaneously under the same desktop session/application ID: GTK forwards a second launch to the existing window.

Machine-specific desktop launchers are retained locally and excluded from this repository. Native navigation collapses below 1050 px; its Persian **فهرست** header button switches between navigation and content. The 900×940 px native capture shows all quotation columns. The document body scrolls beneath a fixed action bar; metadata adapts to two columns with collapsed navigation. Native keyboard and rendering checks cover 900×940, 900×560 and 700×560 windows. The quotation table can scroll horizontally when needed.

Normal and synthetic-demo launchers are installed as `~/.local/share/applications/ir.sarv.quote.v1.desktop` and `ir.sarv.quote.demo.desktop`. The original workstation retains its local demo launcher and separate synthetic profile; these machine-specific files are excluded from publication. Existing launchers were not overwritten. These are local launch conveniences, not a packaged release for other machines.

Current snapshot checks: format, warning-free Clippy, **86 passing ordinary Rust tests**, plus the graphical lifecycle test run separately and release build. The new indexed matcher, bounded import preview and background export changes pass their automated tests; native keyboard, small-window and restart checks pass in the continuation report. Large synthetic responsiveness measurements and production limits are recorded there. See [software risk results](docs/SOFTWARE-RISKS.md).

## Build and verify

Installed native libraries on Sina's Arch/Omarchy machine are GTK **4.22.4**, libadwaita **1.9.3**, Cairo 1.18.4 and Pango 1.58.2; the earlier baseline used Rust 1.98.1. The locked Rust binding is stable `libadwaita` **0.9.2**, with `v1_9` APIs and gtk-rs **0.11**/GLib **0.22**. The app does not use libadwaita's 1.11 alpha API documentation.

Build requires a current Rust stable toolchain, **native libadwaita ≥1.9**, a compatible GTK4 development library, Cairo/Pango, `pkg-config` and a C compiler for bundled SQLite. The installed libadwaita 1.9.3 package requires **GTK ≥4.21.1** in its `pkg-config` metadata; GTK ≥4.10 alone is no longer sufficient even though the Rust GTK feature is named `v4_10`. Use the distribution's matching libadwaita/GTK packages and check both versions. Install a Persian-capable font such as Noto Sans Arabic for UI/PDF shaping.

On a clean Linux computer, copy the complete source including `Cargo.lock`, install those build dependencies through the distribution package manager, then build from the project directory. Arch package names include `rust`, `base-devel`, `gtk4`, `libadwaita` and `noto-fonts`; Debian/Ubuntu development packages include `build-essential`, `pkg-config`, `libgtk-4-dev`, `libadwaita-1-dev`, `libcairo2-dev`, `libpango1.0-dev` and `fonts-noto-core`. These names do not guarantee sufficiently recent versions on an older distribution. The first Cargo build downloads locked crates unless cached. A graphical Wayland or X11 session is required to run the app.

```bash
pkg-config --modversion gtk4 libadwaita-1
cargo build --release --locked
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
./target/release/sarv-quote
```

After dependencies have been fetched, Cargo commands also work with `--offline`. The installed binary does not access the network. The lockfile pins dependency versions. The binary uses native system libraries; it is not a portable static executable.

The application is MIT licensed ([LICENSE](LICENSE)); the Rust `libadwaita`/`libadwaita-sys` bindings also declare MIT. The native C libadwaita library is [LGPL-2.1-or-later](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/1.9/). It remains a dynamically linked system dependency; do not describe it as MIT or statically bundle it into an MIT-only executable. Other dependencies retain their own licenses. No paid library or service is required.

## Workflow / روش کار

1. **فهرست قیمت**: import a UTF-8 supplier CSV, inspect the validation/price preview, then explicitly confirm the source, units and aliases. The filename supplies an editable catalog name. Existing catalogs are retained as immutable snapshots. Duplicate content is rejected even with a different filename/name or row order.
2. **پیشنهاد قیمت**: select an approved catalog, reuse a saved customer or type a customer name/optional phone. Add requested codes/aliases and quantities in the native item table; source prices remain read-only. Enter on the new quantity field adds the row; Enter on an edited row saves and refreshes review while retaining keyboard focus; Ctrl+↑/↓ moves within the same column. CSV import and the advanced CSV expander are secondary entry paths. Import asks for confirmation before replacing the current order and preserves the prior valid draft in an automatic recovery backup. Row removal can be undone without discarding subsequent edits/additions. Valid edits autosave after 500 ms; Ctrl+S saves immediately. Pending edits are labeled unsaved. Invalid input remains visible and blocks approval, a new draft, revision, restore and closing until corrected.
3. Only exact normalized codes and supplier-approved aliases match automatically. Product names/fuzzy similarity do not match implicitly. Several matches or no matches require explicit human selection. Unknown lines show catalog choices only for the user to identify the correct item after checking the customer request. There is no automatic substitution or invented price.
4. Review quantity, unit price, source unit and source timestamp on every row. Prices aged seven days or more block approval until the user acknowledges the stale-price warning. Changing the catalog/requested price source clears the acknowledgment. Old catalogs are not silently refreshed. Enter an optional whole-quote discount in the selected explicit currency. Switching the currency preserves the rial amount. Optional customer reference, Gregorian quote date/expiry and customer notes are saved in the quote snapshot. Editable/storage dates use `YYYY-MM-DD`, accept Persian/Arabic digits and require expiry on/after the quote date. Settings can display Gregorian dates as ISO or `DD/MM/YYYY`; this is not a Jalali calendar. Reference is limited to 160 characters and notes to 4,000.
5. **تأیید نسخه** / Ctrl+Enter creates an immutable approved quote. If table edits still need visual review, the first approval refreshes the rows and asks you to inspect the updated prices/descriptions; approve again after review. Repeated approval with unchanged content returns the same quote. Changed content in the same series produces a new version; previous versions retain their customer, product, metadata and price snapshots.
6. **بایگانی پیشنهادها**: search stored offers and filter latest, previous or all approved revisions. Expand an offer to inspect details, export PDF/CSV, preview, revise, or **کپی در پیشنهاد تازه**. Revisions preserve the document number and historical snapshots; a copy creates a new series and receives a new number on approval. **پیش‌نمایش چاپ** opens a native preview using the same document renderer, with 75/100/125% zoom. A draft preview is explicitly marked unapproved; approved previews show their saved number/version. Print an exported PDF with the desktop PDF viewer. Export never overwrites an existing file; choose a fresh filename. Local approval and export do not send a message or record customer acceptance.
7. **مشتری‌ها / فهرست قیمت**: search, create/edit a saved customer through a native dialog, or reuse that customer's name/phone in the draft. Customer edits never change historical quotes. Approval also deduplicates and saves the customer. Catalog managers expose approved products, aliases, source dates and explicit prices, with search and catalog selection; prices remain immutable import snapshots. Managers load additional results with **نمایش بیشتر** rather than rendering every saved record at once.
8. **پشتیبان و راهنما**: create a JSON backup. Restore shows validated catalog/customer/quote counts and requires explicit confirmation. The current valid widget edits are saved before an automatic `before-restore-<uuid>.json` safety backup is written in the data directory. Invalid current input blocks replacement. Malformed, oversized, unsupported, checksum-invalid or inconsistent backups are rejected before replacement. Restore replaces the whole state, including draft.
9. **تنظیمات**: explicitly save business name/phone/address, header/footer/default terms, amount unit, number prefix/next counter, default expiry/discount, an optional contract-rate addition, appearance, density, digits and date display. System/light/dark use native Adwaita appearance; comfortable/compact changes table geometry. Issuer/document/default calculation preferences are copied into fresh drafts and approved snapshots, so changing settings does not rewrite an existing offer. Appearance and current manager digit preferences update after save. Save/Revert controls and validation feedback sit at the top of Settings. Closing with unsaved settings offers Save, Revert or Continue editing. All settings persist locally and are included in backups.

Keyboard: Tab/Shift+Tab navigate; Enter/Space activate controls; Enter in the new quantity field adds a row; Enter in edited table entries saves/reviews; Ctrl+S saves the draft or the current Settings page; Ctrl+Enter approves on the quotation page; Ctrl+N starts a fresh quote; Alt+1…6 switches quotation/catalog/customers/archive/backup/settings pages. Use GTK dropdown search for explicit catalog/product selections. Before New, Revision or Copy replaces a draft containing customer/contact/reference/notes or item rows, the app saves a recovery backup (`before-new-draft-…json` or `before-revision-…json`) in the data directory. Restore it through the normal validated backup flow if needed. The table scrolls horizontally in narrow windows.

## CSV formats

Supplier CSV (exact header order):

```csv
code,name,measure,price,currency,source_at,aliases
P-101,کاغذ نمایشی,بسته,۱۲۵۰۰۰,toman,2026-10-01T08:00:00Z,کاغذ سفید|كاغذ آ۴
```

- Currency is `rial`/`ریال` or `toman`/`تومان`. No currency guessing.
- Prices must be positive. Rials are integers; tomans support at most one decimal digit (one rial). No grouping separators, scientific notation, negatives or floating point.
- `source_at` is an RFC3339 timestamp with timezone; future sources are rejected. Display preserves the supplied timestamp, isolated for correct bidi rendering. Catalog import time is stored separately.
- Aliases use `|`. Ambiguous aliases are permitted and require review. Duplicate normalized product codes, malformed rows, empty required fields and control/bidi-override input are rejected.
- ASCII, Persian and Arabic digits work; Arabic decimal separator `٫` works. Arabic kaf/yeh, Persian kaf/yeh, digit forms, whitespace and ZWNJ are normalized for matching.
- Files are limited to 10 MiB, 50,000 catalog products and 10,000 order lines. Managers page their results, but the editable order table renders at most 100 rows per page while totals and exports retain the full order. These validation limits are not performance guarantees; very large saved states still need incremental storage and realistic field benchmarks before production-scale use.

Order CSV:

```csv
item,quantity
P-101,۲
قلم آبی,١٢
```

Quantities are positive, up to three decimal places, maximum 1,000,000 per line. Repeated customer rows remain separate requested lines. Monetary calculation uses checked integer rials and quantity thousandths, with a wider integer intermediate. Each line rounds half-up to the nearest rial before summing. The whole-quote discount is subtracted once; an explicitly configured contract-rate addition is then calculated on that net amount and rounded half-up to one rial. Rates use integer basis points (100 = 1%) and default to zero. This optional quoted tax/addition is user configured, with no assertion of a legally applicable rate or tax compliance. No delivery fee, inferred exchange rate or inventory check is added.

Document numbering consumes the next counter only on successful first approval of a new series. Failed approval and repeated clicks do not consume another number. Revisions retain their series number; approved snapshots retain issuer text, terms, display unit/digits/date format and configured tax amounts.

CSV exports contain explicit rial prices/totals/discount/tax, source currencies/timestamps, catalog, requested identifier, approved product/version/number, customer phone, document metadata, issuer text and terms. Dates remain canonical ISO regardless of the saved display preference. Text beginning with spreadsheet formula markers (`= + - @`, including whitespace/control/BOM prefixes) is prefixed with an apostrophe. A UTF-8 BOM aids Persian spreadsheet readers. There is no formula interpolation.

## Demo and evidence

`demo/prices.csv` and `demo/order.csv` are entirely synthetic, including phone/customer values in UI tests. They intentionally demonstrate an ambiguous alias, unknown code and stale price. Source timestamps stay fixed so the stale warning can be observed.

Current build, rendered screenshots, independent keyboard checks, restart and synthetic scale measurements are recorded in [docs/CONTINUATION.md](docs/CONTINUATION.md), with selected evidence committed under `docs/assets/`. Earlier `delivery-complete`, `delivery-final`, `redesign-final` and `final-native` artifacts remain historical comparison evidence. The configured synthetic offer `DEMO-42` totals **5,516,700 rials**, including **262,700 rials** of the user-set addition. Screenshots and reports are local files.

Reproduce the native widget smoke run in a **fresh, separate** data directory with a desktop display available:

```bash
SARV_DATA_DIR="$(mktemp -d /tmp/sarv-native-test.XXXXXX)" \
SARV_UI_SMOKE=full ./target/release/sarv-quote
```

This explicit test mode imports synthetic data, presses real GTK buttons/dropdowns, writes verification artifacts, and leaves the window open for inspection. Normal startup never seeds demo data. To inspect the saved verification profile after closing another instance:

```bash
SARV_DATA_DIR="$PWD/artifacts/delivery-complete" ./target/release/sarv-quote
```

## Storage and limits

SQLite WAL transactions with full synchronization persist the state; optimistic generation checks prevent one stale instance overwriting another. Failed changes never replace the in-memory/disk state. Do not copy just the SQLite database while the app is running: use the app's backup command so journal state is included logically. Backups use a versioned JSON format with a SHA-256 integrity checksum and full business/reference validation. The checksum detects corruption; it is not an authenticated signature or encryption.

Current v1 stores an aggregate JSON document within SQLite. This keeps snapshot/recovery semantics simple for a small local pilot; incremental normalized storage and fully virtualized tables remain future work. A synthetic 50,000-product/1,000-line probe verifies bounded widget rendering and responsiveness during background PDF export; changed saves and initial setup still pause the main thread. Hardware print output, long-run production use, hardware power loss, alternative desktops and accessibility screen-reader use have not been verified. See the continuation report for measured limits.

## Architecture

- `src/domain.rs`: normalization, units, deterministic calculation, review/approval rules, revisions and state validation.
- `src/imports.rs`: bounded CSV parsing, source validation, catalog fingerprints and duplicate guards.
- `src/store.rs`: SQLite transactions, generation checks, backups, bounded restore validation and safe new-file writes.
- `src/export.rs`: spreadsheet-safe CSV and Persian Pango/Cairo PDF.
- `src/ui.rs`, `src/ui/components.rs`, `src/ui/editor.rs`, `src/style.css`: native RTL desktop interface, shared components, editable quotation table and layout styles.
- `src/ui/settings.rs`, `src/ui/managers.rs`: native staged preferences and customer/catalog/quote managers.
- `src/ui/smoke.rs`: opt-in native widget workflow harness.
- `tests/workflow.rs`, `tests/domain_hardening.rs`, `tests/storage_hardening.rs`, `tests/export_hardening.rs`, `tests/qa_invariants.rs`, `tests/settings.rs`, `tests/manager_workflow.rs`: workflow, exact money, source validation, storage/restore, document layout, settings/managers and independent regressions.

See `CONTEXT.md` for domain language and `docs/ROADMAP.md` for the accounting foundation. The source repository is private. No deployment, release tag, security setting or live business integration is included.
