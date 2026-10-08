# Independent Persian accounting copy review

Reviewed 2026-10-09 (Asia/Tehran) by the single research/review agent. **PASS for this bounded text/settings/navigation change; no material blocker found.** Root implemented and executed checks; this reviewer independently inspected source, artifacts, raw SQLite state and images without launching a GUI or build.

## Final attribution

- Reviewed final source aggregate: `18d6f0cf6691b4179f3a0b0667e73956f6c35e8a151cab2baa38253aee1e9905`.
- Reviewed immutable binary: `b09df74e6d5cba3cca56a4caaa4bf90ba4c0bc19b6d8c001fff9e59232c6eba2`.
- Independently rehashed all 36 production files, 35 Rust test files and 19 static compile inputs against [copy-final4 manifest](checks/copy-final4/manifest.json): exact matches. Binary matched too.
- Manifest records **240 ordinary tests passed, 0 failed, 5 ignored**, followed by all five explicit opt-ins, formatting, strict Clippy and release build. These are the integration lead's executed checks, independently read and attributed here.
- [Sidebar result](native/sidebar/sidebar-verification.json): **40/40 passed**, including actual keyboard navigation, visible invalid rates, exact settings save/discard and full-payload restart.
- Product native result: `/workspace/sarv-quote/artifacts/product-native/run-u2reyme_/product-verification.json`, **28/28 passed**, same binary. Covers actual customer/product/catalog/alias pickers, exact quote approval, native CSV/PDF export, revisions, preserved original/contact history, light/dark appearance and fresh-process restoration. The 900×560 interacted editor images were also inspected directly.
- Paired page render results: [before](native/before/copy-snapshots.json) and [after](native/after/copy-snapshots.json), **6/6 checks each**.

## Semantic and preservation findings

The official-source research supports normal Persian accounting terminology, while the actual source determines what each action does. Existing quotations are now پیش‌فاکتور; orders remain سفارش فروش/خرید; actual sales fulfillment and supplier bills are فاکتور فروش/خرید. Distinct warehouse documents are حواله خروج and رسید انبار. Return record/export titles distinguish receipt-only returns from billed purchase returns; billed returns keep their linked supplier credit and original cost. Sales returns use برگشت از فروش. No quotation/order is misleadingly presented as a posted invoice or a payment.

Final source fixes the material earlier findings: mechanically generated customer/audit grammar, normal quotation empty state, normal-form کالای هسته, order-policy headings, sales return captions, mismatched warehouse export names and exposed M5 report descriptions. The trial balance correctly describes connected purchase/sales journals. Necessary warnings about simultaneous invoice/warehouse posting, no receipt of cash, original-cost returns, immutable records and data replacement remain truthful.

The accounting/domain changes reviewed are display/error vocabulary, not posting equations or stored event/enum identifiers. Store and schema remain unchanged. Read-only SQLite `integrity_check` returned `ok` for the preserved synthetic source, paired profiles and sidebar profile. The source, before and after **raw persisted State bytes** independently matched exactly:

`4a1e652b579702cb6b03f5aa5ef59207941617540333aa7578b12ae6bd1d1c68`

That check includes all historical accounting records, quotations, custom text, settings and source links; navigation and screenshots did not rewrite history.

## Settings and focus

The percent entry is presentation only. Source inspection confirms localized digit/decimal normalization, at most two decimal places, integer-only conversion and bounds 0…10000 basis points; extra precision and overflow are rejected. The new Rust tests include all 10001 existing rate round trips. Native reports establish rejected `9.755` and `100.01` preserve the complete saved State and input focus, discard preserves State, and `۹٫۷۵` saves exactly 975 basis points.

The independently inspected final sidebar SQLite payload has rate **975**, appearance **Dark**, all other existing defaults intact and no accounting data. The observer's explicit materialization oracle matches `Settings::default()`: an empty legacy profile omits settings, while its first successful save materializes the existing defaults; the default ISO date remains omitted by its existing serde rule. This is an observer expectation correction, not a changed schema or a waived data-integrity assertion.

The intentional focus improvement captures whether current GTK focus lies beneath a sidebar ListBox before selection changes. Only then does it focus the newly selected destination. Body/form focus remains unaffected. Native checks confirm Accounting→Alt+9 focuses Inventory when already in navigation, body page shortcuts do not pull focus into navigation, arrow/Enter navigation works, F9 restores the current destination, narrow selection works, and close/Escape restores the toggle. The first observer attempt's prior focus assumption is preserved in [correction history](checks/corrections.md); its partial attempt is not counted as a pass.

## Actual visual inspection

Inspected all six paired before/after pages: sales, purchases, inventory, accounting, quotation and settings. The shell now leads with normal operation names, quotations have a separate clear section, developer paragraphs are reduced, and the accounting policy is in a disclosure. The preserved synthetic reference codes explain remaining `M5/M6` text in fixture rows; these are historical data, not developer captions.

Inspected sidebar standard dark, narrow dark focus, narrow accounting, settings error/saved images, and the actual light/dark quotation preview. Persian lettering and mixed-script identifiers are readable. Selected/focused destinations and the invalid rate have visible outlines. The complete invalid rate row and retained `100.01` value are visible; the saved localized 9.75 value is readable. Narrow accounting tabs wrap inside the window. The narrow quotation images show reachable product input and totals/discount with visible keyboard focus.

Inspected rendered sales invoice and supplier bill pages: titles are فاکتور فروش and فاکتور خرید, Persian wrapping and explicit money/quantity values remain readable. The actual quotation preview uses پیش‌فاکتور in generic headings/metadata; its custom historical header still says «فقط پیشنهاد قیمت». Preserving that user-authored text is correct, not a remaining generic-label regression. Sample exports retain their detailed source/ledger evidence; they do not assert a legal tax invoice.

## Limits and small remaining gaps

- A compact legacy phrase, `فاکتور‌نشده`, remains in the GRNI account label/error text and detailed procurement exports. Normal receipt/source-picker captions use بدون فاکتور خرید. This is a nonblocking wording refinement; the financial meaning and exact quantities/values are unchanged.
- These are synthetic private-X11 checks. Wayland, screen-reader speech, high contrast, enlarged text, physical printing and accountant/statutory review remain unverified.
- This copy review does not rerun performance benchmarks. The historical M6 budget misses remain open and attributed to their own build; there is no new performance, universal competitor-parity, production-readiness or tax-compliance claim.
