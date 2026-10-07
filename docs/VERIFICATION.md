# Verification evidence

The native screenshots and 39-check GUI acceptance below describe the preceding local v1 build. The current snapshot includes subsequent software-risk fixes; see [SOFTWARE-RISKS.md](SOFTWARE-RISKS.md) for current checks and remaining native verification.
All verification uses synthetic data. Current release evidence lives in `artifacts/delivery-complete/`; older `delivery-final`, `final-native`, `redesign-final` and `native-release-proof` artifacts are retained historical comparison runs. Evidence applies to the source/build that produced it, and no tax, accounting or accessibility compliance is claimed.

## Native release workflow

The final release ran in a fresh isolated GTK4/libadwaita Wayland profile. These machine-readable reports all say **passed**:

| Report | Result and scope |
| --- | --- |
| Quotation workflow (local-only evidence) | **22 named checks**: approved catalog preview/import; Persian/Arabic input; ambiguous/unmatched human selection; unresolved/stale/invalid approval gates; unit-preserving discount; repeated-click idempotency; revision/history; search; PDF/CSV; backup/restore/safety backup; SQLite reopen; invalid widget retention; unsaved status; delete/edit/add/undo; changed-source review and latest-widget-edit preservation before restore. |
| Settings workflow (local-only evidence) | **14 named checks**: all issuer/header/footer/terms fields; canonical discount conversion; new-draft currency/discount/expiry; exact configured addition; numbering idempotency; immutable issuer/terms/digits; actual Adwaita light appearance; compact geometry; Latin/Persian display; native customer creation; SQLite reopen; document exports; settings backup; Gregorian date display/snapshot. |
| Approved preview (local-only evidence) | **3 named checks**: saved approved quote opens a native dialog; 75%/125% zoom changes picture geometry; 100% restored. |

The settings workflow additionally asserts that a draft with customer/notes and **no item rows** receives a valid recovery backup when New replaces it. That assertion executes in `src/ui/smoke.rs`; it is not an extra named entry in the 14-check JSON array.

The ordinary quotation run saved two immutable revisions totaling **3,995,000** and **5,245,000 rials**. The configured synthetic offer **DEMO-42** totals **5,516,700 rials**, including **262,700 rials** of the explicit user-set 5% addition. Number consumption remained stable on repeated approval; three quote snapshots existed after the settings flow.

Outputs: [ordinary PDF](assets/delivery-complete-sample-quote.pdf), [ordinary CSV](assets/delivery-complete-sample-quote.csv), [configured PDF](assets/delivery-complete-business-quote.pdf), [configured CSV](assets/delivery-complete-business-quote.csv), ordinary backup (local-only evidence), settings backup (local-only evidence).

## Native captures inspected

The following supplied final captures were opened as local images and inspected. The reviews establish what is visibly rendered; workflow assertions establish the exercised behavior.

1. [Quotation](assets/delivery-complete-native-after-quotation.png): four dense editable rows, native selected navigation, persistent Persian captions, explicit toman units, stable discount/addition/total and read-only catalog prices.
2. [Products](assets/delivery-complete-native-products.png): approved catalog expansion, aliases, source timestamps, stale-price indication and explicit units.
3. [Customers](assets/delivery-complete-native-customers.png): searchable stored customers, New/Edit/Use controls.
4. [Settings](assets/delivery-complete-native-settings.png): native preference groups and business/document content. Further settings continue below the visible scroll area; their behavior is covered by the settings workflow.
5. [Quote archive](assets/delivery-complete-native-quotes.png): latest-revision filter, saved numbers/versions and preview/export/revise/copy actions; no sent/paid status.
6. [Approved preview](assets/delivery-complete-native-approved-preview.png): explicit approved version, number DEMO-42, issuer/address/header, Gregorian DD/MM/YYYY dates, Latin document digits, joined Persian text and distinct quantity/price/total columns. The document scrolls beyond the visible viewport.
7. [Narrow quotation](assets/delivery-complete-native-narrow.png): 900×940 px capture with collapsed sidebar, readable Persian **فهرست** navigation control, all four rows and all quotation columns visible. Horizontal scrolling remains available. This does not establish usability in a short 508 px-high tile; use a tall/maximized window for the main controls.

[Import preview](assets/delivery-complete-native-catalog-preview.png), [review](assets/delivery-complete-native-review.png), [human review](assets/delivery-complete-native-human-review.png) and [draft print preview](assets/delivery-complete-native-print-preview.png) are also recorded by the native harness. [DESIGN.md](DESIGN.md) contains before/after rationale. [QA-REVIEW.md](QA-REVIEW.md) records independent findings and regressions.

## Rust and independent regression checks

Final Rust log (local-only evidence) records **69 passing ordinary tests, 0 failures**, with one graphical test intentionally ignored by that ordinary run. The separate graphical log (local-only evidence) records that test passing on Wayland. The ordinary suite comprises domain **8**, export **13**, manager **7**, independent QA **10**, settings **8**, storage **13**, and workflow **10** tests. Format checking, Clippy with warnings denied and the release build passed on the final source.

Coverage includes exact rial/toman round-trips, half-up fractional arithmetic, overflow, Persian normalization, exact-code precedence, approved-alias ambiguity, CSV formula neutralization, equivalent duplicate imports, immutable revisions, numbering/settings/date snapshots, historical stale-price validation, failure-atomic commands, optimistic-writer conflicts, bounded backups, malicious/corrupt restore, symlink/non-overwrite exports, multipage/long-Persian PDF layout and independently compared numeric raster placement.

The separate graphical test checks native widget lifecycle: destruction/initialization must not synthesize New or empty Edit actions. Native no-loss regressions additionally cover invalid inputs, undo after intervening edits and pre-replacement recovery backups.

| Final check | Status |
| --- | --- |
| Format, Clippy with warnings denied, aggregate tests, release build | Passed; final Rust log linked above |
| Ordinary tests | 69 passed, 0 failed |
| Separate graphical widget-lifecycle regression | Passed on Wayland; separate log linked above |
| Real native quotation/settings/approved preview | Passed: 22 / 14 / 3 named checks |
| Actual injected keyboard events | Passed: 8 checks; keyboard report linked below |
| Full process close/reopen with settings/quotes retained | Passed; restart report linked below |

Keyboard verification (local-only evidence) records real Wayland `wtype` events after the compositor confirmed focus on the isolated app: Tab, Shift+Tab, Ctrl+S, Ctrl+Enter, Alt+6, Settings Ctrl+S and Alt+1. All **8 checks** passed: expected row focus, navigation, contextual save, idempotent approval, retained draft and unchanged persisted payload.

Restart verification (local-only evidence) records a controlled native window close, original process exit and a new process using the exact release binary/profile. State SHA-256 and generation **32** remained identical, retaining **3 quotes**, **4 draft lines** and next counter **43**. Verified release SHA-256: `058e3818a1a7bf2bc7d53790ae8c1fc4a6087da66c8fed52a15bd2a4b925fcc9`.

## Reproduce

The installed launcher report (local-only evidence) confirms the demo desktop launcher started the exact final release binary. This final launch used the isolated application ID to preserve an existing normal window; the final synthetic demo remains open. Automatic approval review rejected closing the normal window because it could contain unsaved edits. Both desktop launchers point to this checkout's release binary through the normal launcher or demo script.

The Library result (local-only evidence) records unavailable screenshot upload support in this executor. No Library file IDs were returned; all screenshots are retained locally.

Installed native libraries are GTK **4.22.4** and libadwaita **1.9.3**, confirmed using `pkg-config --modversion gtk4 libadwaita-1`. The stable locked Rust libadwaita binding is **0.9.2/v1_9**. Installed libadwaita requires GTK ≥4.21.1. See [README](../README.md) for clean-machine prerequisites and license boundaries.

```bash
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
```

Use `--offline` for Cargo after dependencies are cached. Run the graphical lifecycle regression separately in a graphical session:

```bash
cargo test --locked --test qa_invariants -- --ignored
```

Close other Sarv instances before exercising the actual native widgets in a fresh disposable profile:

```bash
SARV_DATA_DIR="$(mktemp -d /tmp/sarv-native-test.XXXXXX)" \
SARV_UI_SMOKE=full ./target/release/sarv-quote
```

The explicit harness imports synthetic data, uses native controls, writes reports/captures and leaves the window available for inspection. Normal launch does not seed data. Inspect the reports; startup alone is not workflow verification.

## Limits

Clean-machine dependency instructions are documented; a separate clean operating-system installation has not been exercised. No physical printer, screen-reader session, alternative distribution/display scaling, enormous editable order, power-loss fault injection, long-running field operation or signed distribution package was verified. The 900×940 px and wide desktop captures do not establish usable operation in very short tiles; approximately 860 px height or a maximized window is recommended. Manager paging does not make the eager editable order table virtualized. SQLite tests establish logical failure atomicity and reopen behavior, not hardware durability guarantees. Backup checksums detect inconsistency rather than authenticate or encrypt data. Native libraries remain dynamically linked system dependencies. No live bank/tax/SMS/AI integration, real customer message or external publication was performed.

Screenshots are delivered as local files. The optional Library upload helper reported `prepare_uploads` unavailable, so there are no Library file IDs or cloud screenshot links.
