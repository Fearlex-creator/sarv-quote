# سرو — حسابداری محلی (Free Accounting App)

Local-first accounting for people and businesses: accounts/wallets, contacts, cash movements, and a single double-entry-backed ledger. Exact money lives in the Rust core (integer rials); the Tauri 2 + React UI formats for Toman/Rial, Jalali/Gregorian, Persian/Latin digits, and personal vs business language.

## Phase 1 MVP (shipped in the web shell)

Six weight 9–10 screens on one shared ledger:

1. **Transaction Ledger** — virtualized register, search/filters, detail drawer, CSV export, running balance  
2. **Quick Add** — expense / income / transfer sheet (`Ctrl+N`), splits, attachment name, save-and-add-another  
3. **Accounts and Wallets** — cash/bank/card/wallet/savings/loan, opening balance, archive, groups  
4. **Contacts** — customer/supplier roles, balance + aging buckets, CSV import, history link to ledger  
5. **Dashboard** — cash / income / expense / profit, receivables & payables, recent activity, date range chips  
6. **Settings** — profile, currency decimals, Jalali/Gregorian, RTL/LTR, theme, density, accent, personal/business mode  

Later phases (invoices, bills, payments, cheques, inventory, reports, …) add screens on **the same ledger** — Phase 1 work is not thrown away. Payroll, live bank feeds, and card processing stay out of scope.

### Double-entry model

Documents and cash entries post balanced sides. Journal lines (and Phase 1 cashbook entries that imply balanced sides) are the source of truth for dashboard, ledger, and future reports. Corrections reverse-and-repost; closed periods lock (accounting module).

### Local development

```bash
pnpm install
pnpm --dir web install
# Wayland WebKit issues: GDK_BACKEND=x11 WEBKIT_DISABLE_DMABUF_RENDERER=1
SARV_DATA_DIR="$PWD/data" pnpm dev
```

| Command | Purpose |
|---------|---------|
| `pnpm dev` | Tauri desktop + Vite |
| `pnpm web:dev` | Frontend only |
| `pnpm web:build` / `pnpm web:lint` / `pnpm web:typecheck` | Production web build & checks |
| `pnpm build` | Packaged Tauri app |
| `pnpm gtk` | Legacy GTK shell |

Requires Node `>=20.9.0` and a Rust toolchain. Data defaults to `data/sarv.sqlite3` (override with `SARV_DATA_DIR`).

Keyboard: `Ctrl/Cmd+N` Quick Add, `Ctrl/Cmd+K` command palette (navigate Phase 1 screens + Quick Add).

---

Current Persian terminology/settings pass: [bounded checklist and evidence](requirements/checkpoint-ui-copy.md) applies official-source accounting vocabulary, concise native forms and exact percentage settings. Final qualification passes 240 ordinary tests plus 5 separate opt-ins;40 sidebar/settings,28 customer/product/quote/export and12 paired native render/no-write checks pass with independent review. Posted history and schema7 remain unchanged. Prior performance misses remain open; this pass does not establish current performance, production readiness or statutory acceptance.

Previous C9 long-document follow-up: [full200 native checklist and evidence](requirements/checkpoint-M6-followup.md) completes all200 keyboard inputs, all400 preview details, settled standard/narrow validation, user-focus retention and exact3,000IRR posting/restart. The independent pilot financial/visual review passes. Final quiet n10/n20 completed with25 measured checks passing and14 strict misses; full performance acceptance remains open. Dark-mode observation and final independent measurement review are recorded in the follow-up. The existing Rust/GTK4/libadwaita shell and accounting history are preserved. Earlier C8 and other candidate notes below are historical execution scopes, not the latest200/performance status. No production/statutory acceptance is claimed.

Free, offline Persian quotation preparation, a manual accounting ledger, guided fresh-company setup, connected purchases, and linked goods sales with reconciled stock, supplier and customer balances. Application and business logic are written in Rust; the interface uses native GTK4/stable libadwaita, with Persian shaping/RTL through GTK/Pango. No web frontend, hosted services, AI matching, credentials or subscriptions.

The guided setup/opening checkpoint is tracked in [requirements/milestone-M3.md](requirements/milestone-M3.md); its bounded functional checks pass with independent native import verification and recorded frozen-build measurements. Its historical strict timing misses remain recorded. Current C12 search and long-document navigation meet measured budgets, while quotation responsiveness and full release gates remain partial. The accepted native/Jalali checkpoint and its exact results remain in [requirements/milestone-M2.md](requirements/milestone-M2.md). The [126-ID baseline and current evidence map](requirements/evidence.md) and [full roadmap](requirements/roadmap.md) retain the broader product specification. The first accounting gate is still partial; no production or Iranian statutory acceptance is claimed.

The connected purchase/inventory checkpoint and its exact verification are recorded in [requirements/milestone-M4.md](requirements/milestone-M4.md). Purchase orders, partial goods receipts and matched supplier bills have distinct linked states. Internal warehouse transfers preserve company quantity/value without a financial journal. The [M5 goods sales checkpoint](requirements/milestone-M5.md) adds durable editable sales drafts, approved-quotation conversion, partial paired delivery/invoices and original-cost restock credits. Its verification is tracked separately. Payments, closing, unbilled delivery, service sales, statutory tax and durable editable purchase drafts remain dependencies.

Historical quotation product work, requirements, rendered screenshots and validation are recorded in [docs/PRODUCT-DELIVERY.md](docs/PRODUCT-DELIVERY.md), [requirements checklist](docs/PRODUCT-ACCEPTANCE.md) and [reference/design decisions](docs/PRODUCT-DESIGN.md). Earlier continuation evidence remains in [docs/CONTINUATION.md](docs/CONTINUATION.md).

The application has a native libadwaita shell, working customer/product/quote managers and persistent settings. Quotation baseline evidence is in [docs/PRODUCT-DELIVERY.md](docs/PRODUCT-DELIVERY.md); the [current workflow checklist](requirements/checkpoint-M5.md) links demonstration steps, exact tests, migration/restore, before/after native screenshots, sample PDFs and independent review. Physical printing and production-scale field use remain unverified.

Quotation approval creates an immutable local offer, **not** an invoice, posted accounting entry, inventory reservation, payment or tax filing. A saved or exported offer has not been sent to or accepted by a customer. It makes no statutory compliance claim.

## Run locally

**Tauri + React (current UI):**

```bash
cd sarv-quote
pnpm install
pnpm --dir web install
# If WebKit crashes on Wayland: GDK_BACKEND=x11 WEBKIT_DISABLE_DMABUF_RENDERER=1
SARV_DATA_DIR="$PWD/data" pnpm dev
```

Rust engine lives in `crates/sarv-quote` (SQLite, chrono, rust_decimal money helpers). The web shell is ObsidianUI `DashboardShell` + shadcn/TanStack Query. Legacy GTK binary: `pnpm gtk` (`sarv-quote-gtk`).

A new data profile starts empty; subsequent launches reopen its saved state. Normal launch uses `data/sarv.sqlite3` inside this project. Set `SARV_DATA_DIR=/absolute/path` to choose a different persistent location.

Machine-specific desktop launchers are retained locally and excluded from this repository. The native shell has a continuous sidebar with its own titlebar and bottom settings/backup actions. F9 or the content-titlebar navigation icon hides/shows it at wide widths; changing pages preserves a manually hidden sidebar. Below1050px navigation becomes an overlay, initially closed. Escape or its close button dismisses it and restores focus to the navigation toggle. The titlebar names the active workspace. Document bodies scroll beneath fixed action bars, and quotation tables can scroll horizontally in narrow windows. Current shell evidence and declared platform limits are in [the M6 checklist](requirements/checkpoint-M6.md).

Normal and synthetic-demo launchers are installed as `~/.local/share/applications/ir.sarv.quote.v1.desktop` and `ir.sarv.quote.demo.desktop`. The original workstation retains its local demo launcher and separate synthetic profile; these machine-specific files are excluded from publication. Existing launchers were not overwritten. These are local launch conveniences, not a packaged release for other machines.

The accepted M3 cloud checkpoint passed **171 ordinary Rust tests**, with graphical lifecycle and bundled-font PDF opt-ins run separately, strict Clippy and a release build. Its native core passed 27 checks, independent real CSV import 38, and preserved quotation/manual/Jalali/sidebar/save-close workflows 122. Those counts describe M3. The final M4 freeze6 passes 194 ordinary tests, three separately run opt-ins, strict Clippy and a release build. [M4 verification](requirements/milestone-M4.md) identifies exact native, independent, performance and preserved-regression evidence, including earlier separately attributed freezes. These are named evidence suites, not full-product acceptance. Large synthetic responsiveness measurements and production limits remain explicit. See [software risk results](docs/SOFTWARE-RISKS.md).

Current M5 C12 passes220 ordinary tests, four separately run opt-ins, strict Clippy,83 native sales checks,37 immutable-ID checks,13 quotation callback checks and seven preserved workflows. Its final once-only n10 startup/n20 search/39 long-page measurements pass those budgets on the declared cloud executor. Fiveof22 quotation action scopes and original broad50ms responsiveness probes still miss; [all samples and limits](requirements/performance-M5.md) remain visible. The native detail pager, narrow Sales page and quotation focus handoff were repaired and tested without material GTK diagnostics.

## Build and verify

Installed native libraries on Sina's Arch/Omarchy machine are GTK **4.22.4**, libadwaita **1.9.3**, Cairo 1.18.4 and Pango 1.58.2; the earlier baseline used Rust 1.98.1. The locked Rust binding is stable `libadwaita` **0.9.2**, with `v1_9` APIs and gtk-rs **0.11**/GLib **0.22**. The app does not use libadwaita's 1.11 alpha API documentation.

Build requires a current Rust stable toolchain, **native libadwaita ≥1.9**, a compatible GTK4 development library, Cairo/Pango, `pkg-config` and a C compiler for bundled SQLite. The installed libadwaita 1.9.3 package requires **GTK ≥4.21.1** in its `pkg-config` metadata; GTK ≥4.10 alone is no longer sufficient even though the Rust GTK feature is named `v4_10`. Use the distribution's matching libadwaita/GTK packages and check both versions. Vazirmatn is bundled under SIL OFL 1.1 and registered only for the running process; Noto Sans Arabic can supply fallback glyphs. Fontconfig must be available. See [font provenance/license](assets/fonts/README.md).

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

The existing repository declares MIT ([future distribution decision remains pending](decisions.md)); no license choice was changed during this checkpoint. The application is MIT licensed ([LICENSE](LICENSE)); the Rust `libadwaita`/`libadwaita-sys` bindings also declare MIT. The native C libadwaita library is [LGPL-2.1-or-later](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/1.9/). It remains a dynamically linked system dependency; do not describe it as MIT or statically bundle it into an MIT-only executable. Other dependencies retain their own licenses. No paid library or service is required.

## Accounting workflows

Open **دفتر حسابداری** with Alt+7. Create a company and explicit Gregorian or Jalali fiscal period (stored as Gregorian ISO), define a custom chart with deliberate classifications/normal sides, post its initial balanced opening, then manual journals, linked reversals and period locks/reopen reasons. Trial balance includes all posted history through the cutoff, supports exact rial/toman display and date-labeled PDF/CSV export. Fresh companies can instead use guided `core-policy-v1` setup with editable chart codes/names, explicit timezone/calendar, typed opening registers and strict CSV staging. The independently supplied ledger must balance and match twelve controls; stock and GRNI share the same quantity/value pool. Entirely zero monetary openings use a nonfinancial initialization marker. Existing custom books retain their manual workflow. Connected purchase orders, partial receipts, exact-cost supplier bills and internal warehouse transfers are implemented in the M4 local checkpoint. Linked goods sales and original-cost returns are implemented in M5; supplier payments and payment allocation remain dependent work. Declaring a policy does not implement missing workflows. Actual legal/tax integrations are deferred. Control-account postings requiring an absent subledger are blocked. These are separate books from quotation approval. See the [complete synthetic demonstration](requirements/milestone-M1.md).

Open **خریدها** with Alt+8 to add a supplier, unit, item or warehouse after opening; enter a purchase order with explicit catalog/manual price provenance; review its exact total before posting. Receive against its remaining quantities, then match supplier bills against received-unbilled quantities at the frozen cost. A bill creates a payable without receiving stock again. Native previews run outside the GTK main thread and show financial lines or an explicit nonfinancial effect. Posted sources are immutable. Unposted editor rows are temporary: cancelling or closing discards them; durable editable purchase drafts are still pending. Select an eligible original receipt or bill to return quantity at its frozen attributable cost. Billed returns post a distinct linked supplier credit; unbilled returns reduce GRNI without a credit. Review physical and financial effects before posting and export each document separately. Payments/refunds remain future work. See the [M6 demonstration](requirements/checkpoint-M6.md).

Open **موجودی و انتقال** with Alt+9 to inspect end-of-day stock, unbilled receipts and supplier payables, then transfer quantity between warehouses at the source pool's exact carried cost. Company inventory value stays unchanged. Ledger receipt/bill rows link to their source details. Reports support explicit rial/toman, readable Persian PDF and canonical CSV; original paper dates are separate from financial availability. See the [M4 demonstration and remaining gaps](requirements/milestone-M4.md).

## Workflow / روش کار

1. **فهرست قیمت**: create a price list with the native catalog composer (code, description, unit, price, source date and approved aliases), clone a saved source as a new immutable snapshot, or import a UTF-8 supplier CSV, inspect the validation/price preview, then explicitly confirm the source, units and aliases. The filename supplies an editable catalog name. Existing catalogs are retained as immutable snapshots. Duplicate content is rejected even with a different filename/name or row order.
2. **پیش‌فاکتور**: select an approved catalog, reuse a saved customer or type a customer name/optional phone. Search the indexed product picker by Persian name, code or approved alias; inspect price/unit and explicitly choose the product. Add requested codes/aliases and quantities in the native item table; source prices remain read-only. Enter on the new quantity field adds the row; Enter on an edited row saves and refreshes review while retaining keyboard focus; Ctrl+↑/↓ moves within the same column. CSV import and the advanced CSV expander are secondary entry paths. Import asks for confirmation before replacing the current order and preserves the prior valid draft in an automatic recovery backup. Row removal can be undone without discarding subsequent edits/additions. Valid edits autosave after 500 ms; Ctrl+S saves immediately. Pending edits are labeled unsaved. Invalid input remains visible and blocks approval, a new draft, revision, restore and closing until corrected.
3. Only exact normalized codes and supplier-approved aliases match automatically. Product names/fuzzy similarity do not match implicitly. Several matches or no matches require explicit human selection. Unknown lines show catalog choices only for the user to identify the correct item after checking the customer request. There is no automatic substitution or invented price.
4. Review quantity, unit price, source unit and source timestamp on every row. Prices aged seven days or more block approval until the user acknowledges the stale-price warning. Changing the catalog/requested price source clears the acknowledgment. Old catalogs are not silently refreshed. Enter an optional whole-quote discount in the selected explicit currency. Switching the currency preserves the rial amount. Optional customer reference, Gregorian quote date/expiry and customer notes are saved in the quote snapshot. Editable/storage dates use `YYYY-MM-DD`, accept Persian/Arabic digits and require expiry on/after the quote date. Settings support Gregorian ISO/`DD/MM/YYYY` and exact Jalali display; fiscal and core opening date fields accept the explicitly selected calendar while storage remains Gregorian ISO. Reference is limited to 160 characters and notes to 4,000.
5. **تأیید نسخه** / Ctrl+Enter creates an immutable approved quote. If table edits still need visual review, the first approval refreshes the rows and asks you to inspect the updated prices/descriptions; approve again after review. Repeated approval with unchanged content returns the same quote. Changed content in the same series produces a new version; previous versions retain their customer, product, metadata and price snapshots.
6. **بایگانی پیش‌فاکتورها**: search stored offers and filter latest, previous or all approved revisions. Expand an offer to inspect details, export PDF/CSV, preview, revise, or **کپی در پیش‌فاکتور تازه**. Revisions preserve the document number and historical snapshots; a copy creates a new series and receives a new number on approval. **پیش‌نمایش چاپ** opens a native preview using the same document renderer, with 75/100/125% zoom. A draft preview is explicitly marked unapproved; approved previews show their saved number/version. Print an exported PDF with the desktop PDF viewer. Export never overwrites an existing file; choose a fresh filename. Local approval and export do not send a message or record customer acceptance.
7. **مشتری‌ها / فهرست قیمت**: search, create/edit a saved customer with contact name, telephone, email and address through a native dialog, or select the customer inside the quotation. Customer history is linked by ID; archive/unarchive keeps referenced records and immutable documents safe. Customer edits never change historical quotes. Approval also deduplicates and saves the customer. Catalog managers expose approved products, aliases, source dates and explicit prices, with search and catalog selection; prices remain immutable import snapshots. Managers load additional results with **نمایش بیشتر** rather than rendering every saved record at once.
8. **پشتیبان و راهنما**: create a JSON backup. Restore shows validated catalog/customer/quote counts and requires explicit confirmation. The current valid widget edits are saved before an automatic `before-restore-<uuid>.json` safety backup is written in the data directory. Invalid current input blocks replacement. Malformed, oversized, unsupported, checksum-invalid or inconsistent backups are rejected before replacement. Restore replaces the whole state, including draft.
9. **تنظیمات**: explicitly save business name/phone/address, header/footer/default terms, amount unit, number prefix/next counter, default expiry/discount, an optional quotation-only percentage addition (9.75% stores exactly975 basis points;0.01% steps), appearance, density, digits and date display. System/light/dark use native Adwaita appearance; comfortable/compact changes table geometry. Issuer/document/default calculation preferences are copied into fresh drafts and approved snapshots, so changing settings does not rewrite an existing offer. Appearance and current manager digit preferences update after save. Save/Revert controls and validation feedback sit at the top of Settings. Closing with unsaved settings offers Save, Revert or Continue editing. All settings persist locally and are included in backups.

Keyboard: Tab/Shift+Tab navigate; Enter/Space activate controls; Enter in the new quantity field adds a row; Enter in edited table entries saves/reviews; Ctrl+S saves the draft or the current Settings page; Ctrl+Enter approves on the quotation page; Ctrl+N starts a fresh quote; Alt+1…9 switches quotation/catalog/customers/archive/backup/settings/accounting/purchases/inventory pages. Alt+0 opens Sales. F9 or the titlebar navigation button shows/hides the sidebar. At narrow widths, navigation opens as an overlay; Escape or its close button dismisses it and restores focus to the navigation button. Ctrl+K opens indexed product selection; Ctrl+F opens customer selection in the quote or focuses the active manager, accounting or purchase search. Enter chooses a selected result; Escape cancels. Plain Escape also closes the current catalog editor without saving; nested confirmations and open popup choices keep their own cancellation behavior. Use GTK dropdown search for explicit catalog selections. Before New, Revision or Copy replaces a draft containing customer/contact/reference/notes or item rows, the app saves a recovery backup (`before-new-draft-…json` or `before-revision-…json`) in the data directory. Restore it through the normal validated backup flow if needed. The table scrolls horizontally in narrow windows.

## CSV formats

Supplier CSV (exact header order):

```csv
code,name,measure,price,currency,source_at,aliases
P-101,کاغذ نمایشی,بسته,۱۲۵۰۰۰,toman,2026-10-01T08:00:00Z,کاغذ سفید|كاغذ آ۴
```

- Currency is `rial`/`ریال` or `toman`/`تومان`. No currency guessing.
- Prices must be positive. Rials are integers; tomans support at most one decimal digit (one rial). No grouping separators, scientific notation, negatives or floating point.
- `source_at` is an RFC3339 timestamp with timezone; future sources are rejected. Managers show the source date using the chosen date/digit preference, with the full supplied timestamp available as a tooltip; storage and CSV keep the canonical timestamp. Catalog import time is stored separately.
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

CSV exports contain customer ID/contact/email/address and explicit rial prices/totals/discount/tax, source currencies/timestamps, catalog, requested identifier, approved product/version/number, customer phone, document metadata, issuer text and terms. Dates remain canonical ISO regardless of the saved display preference. Text beginning with spreadsheet formula markers (`= + - @`, including whitespace/control/BOM prefixes) is prefixed with an apostrophe. A UTF-8 BOM aids Persian spreadsheet readers. There is no formula interpolation.

## Demo and evidence

`demo/prices.csv` and `demo/order.csv` are entirely synthetic, including phone/customer values in UI tests. They intentionally demonstrate an ambiguous alias, unknown code and stale price. Source timestamps stay fixed so the stale warning can be observed.

The quotation baseline, rendered screenshots, independent product/keyboard checks, restart and earlier synthetic scale measurements are recorded in [docs/PRODUCT-DELIVERY.md](docs/PRODUCT-DELIVERY.md); current connected-accounting checks and limits are recorded in [M4 delivery](docs/assets/accounting-m4/README.md), with selected evidence committed under `docs/assets/`. Earlier `delivery-complete`, `delivery-final`, `redesign-final` and `final-native` artifacts remain historical comparison evidence. The configured synthetic offer `DEMO-42` totals **5,516,700 rials**, including **262,700 rials** of the user-set addition. Screenshots and reports are local files.

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

Schema 7 stores the quotation/accounting aggregate and raw sales drafts within SQLite. Schemas 1–6 upgrade only after validation and a verified recovery snapshot; see [migration and recovery](migrations.md). This keeps snapshot/recovery semantics simple for a small local pilot; incremental normalized storage and fully virtualized tables remain future work. A synthetic 50,000-product/1,000-line probe measures import preparation, bounded indexed pickers, catalog composition, paging and background PDF export. Imports, restore preparation and backups use workers with generation checks. Opening a copied catalog with over 1,000 products also builds its search index on a worker with explicit loading, close/discard and reopen states. Draft Save/Ctrl+S/autosave, stored customer/product selection and accounting commands validate, serialize and commit on workers with generation guards. Stored picker choices and the current form use one atomic commit; stale selections reject without a separate draft write. State cloning, widget rebuilds and some dependent commands still run on the main thread; the current performance report records remaining budget failures. Hardware print output, long-run production use, hardware power loss, alternative desktops and accessibility screen-reader use have not been verified. See the product delivery report for measured limits.

## Architecture

- `src/domain.rs`: normalization, units, deterministic calculation, review/approval rules, revisions and state validation.
- `src/imports.rs`: bounded CSV parsing, source validation, catalog fingerprints and duplicate guards.
- `src/store.rs`: SQLite transactions, generation checks, backups, bounded restore validation and safe new-file writes.
- `src/export.rs`: spreadsheet-safe CSV and Persian Pango/Cairo PDF.
- `src/ui.rs`, `src/ui/components.rs`, `src/ui/workspace.rs`, `src/ui/editor.rs`, `src/ui/pickers.rs`, `src/style.css`: native RTL desktop interface, embedded fonts, shared components, indexed pickers, editable quotation table and layout styles.
- `src/ui/settings.rs`, `src/ui/managers.rs`, `src/ui/catalog_editor.rs`: native staged preferences, customer/history/archive tools and immutable price-list composition.
- `src/ui/smoke.rs`: opt-in native widget workflow harness.
- `tests/workflow.rs`, `tests/domain_hardening.rs`, `tests/storage_hardening.rs`, `tests/export_hardening.rs`, `tests/qa_invariants.rs`, `tests/settings.rs`, `tests/manager_workflow.rs`: workflow, exact money, source validation, storage/restore, document layout, settings/managers and independent regressions.

See `CONTEXT.md` for domain language and `docs/ROADMAP.md` for the accounting foundation. The source repository is private. No deployment, release tag, security setting or live business integration is included.
