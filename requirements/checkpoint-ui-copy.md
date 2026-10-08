# Persian accounting copy checkpoint

Baseline: clean `main` at `5cf903914f4d1db8808272b6746aa1e9f1190811`, schema7. One research/review agent owns only its source-linked research document; root owns implementation and integration. Existing M6 agents remain finished/HOLD. The user's scope is Persian terminology and bounded UI/settings logic, not new accounting modules or legal rules.

[Official-source research](../docs/PERSIAN-ACCOUNTING-COPY-RESEARCH.md) distinguishes پیش‌فاکتور, سفارش فروش/خرید, فاکتور فروش/خرید, رسید انبار, حواله خروج and linked returns. Presentation labels may change; persisted IDs, states, calculation policy, postings, numbering and user-authored historical document text must remain exact.

Acceptance defined before edits:

- IR-01 / UX-05–09: consistent Persian navigation, forms, record states and exports. No quotation/order/receipt relabelled as a posted invoice. Existing customer/product/price selection and links remain functional.
- UX-12/13: concise scoped settings, exact percent-to-integer-basis-point input, visible invalid input, unchanged saved settings on rejection/discard, save and restart persistence. Appearance, digits, calendar and numbering continue to work.
- UX-14 / QA-03: inspect and interact with actual GTK normal/narrow/light/dark pages, settings and an affected document workflow; independent source/semantic/image review. Preserve meaningful posting, return, backup-replacement and unsaved-data consequences.
- DATA-03/04 / MIG-01: no schema migration or historical data rewrite. Synthetic before/after States establish that text/navigation alone writes nothing; settings changes affect only intended preferences and new pre-invoices.

Current status: implemented, natively verified and independently reviewed. Git integration recorded by the pull request. The prior M6 performance run remains attributed to its frozen build, including14 misses; this copy pass does not waive those misses or establish full performance/production acceptance.


Final implementation:

- App title سرو | حسابداری; daily sales/purchase/inventory/accounting operations precede pre-invoice and records. New companies start at accounting setup; initialized businesses start at sales. Existing page IDs and shortcuts remain stable. Sidebar-focused shortcuts keep focus on the new current destination; content focus remains in content.
- Quotes remain پیش‌فاکتور, actual paired invoices become فاکتور فروش / حواله خروج, matched supplier bills become فاکتور خرید. Receipt-only returns say برگشت رسید انبار; billed purchase returns say برگشت از خرید. Source links, states, IDs, postings and balances remain distinct.
- Concise forms/statuses, ordinary item names, consistent warehouse/export terms and scoped quote settings replace old quotation/developer vocabulary. Significant posting/return/unsaved/restore warnings remain. Stored custom headers, footers and historical text are preserved.
- نرخ افزودهٔ پیش‌فاکتور (%) accepts exact localized percentages in0.01% steps,0–100%, without float conversion. Existing integer basis points are unchanged. No statutory rate or accounting policy was selected by this change.

Evidence and acceptance results:

| Requirement / acceptance | Result | Evidence |
| --- | --- | --- |
| IR-01 / UX-05–09 vocabulary and document distinction | Complete for this bounded copy pass | Official-source glossary, independent review, paired six-page native renders, invoice/bill/pre-invoice PDFs |
| UX-12/13 exact percent and settings | Complete | Exhaustive10001-rate round-trip plus localized/rejected input tests; native invalid-input/no-write, discard,975bp save,Dark save and restart |
| UX-14 / QA-03 native interaction and review | Complete within declared platform |40sidebar/settings checks,28customer/product/price/quote/revision/export/theme/narrow checks,12paired render/no-write checks; actual images independently inspected |
| DATA-03/04 / MIG-01 preservation | Complete within this change | Schema7 unchanged; byte-exact source/before/after State; immutable quote/contact history checks; migration/refusal and restore tests |

Final frozen qualification: [manifest](../docs/assets/ui-copy/checks/copy-final4/manifest.json), **240 ordinary Rust tests passed,0failed,5ignored**, and the **5ignored checks separately passed** (font layout, actual M3/M4/M5 schema refusal and GTK lifecycle). `cargo fmt --all -- --check`, strict all-target Clippy and offline locked release build all passed. Two existing PDF tests were rerun specifically to retain synthetic sample exports; they are already included in the240 ordinary tests, not additional unique tests. Final release SHA256: `b09df74e6d5cba3cca56a4caaa4bf90ba4c0bc19b6d8c001fff9e59232c6eba2`.

Native evidence: [before](../docs/assets/ui-copy/native/before/), [after](../docs/assets/ui-copy/native/after/), [40-check sidebar/settings](../docs/assets/ui-copy/native/sidebar/sidebar-verification.json), [28-check product workflow](../docs/assets/ui-copy/native/product/product-verification.json), [independent review](../docs/assets/ui-copy/review.md), [sample PDFs](../docs/assets/ui-copy/pdfs/). All tests use synthetic profiles on private X11/Xvfb/Openbox/DBus with real XTest keys and AT-SPI observation; no live data. Observer sources/hashes and raw receipts are included. Corrections and intermediate qualification summaries are retained separately from final acceptance.

Demonstration:

1. Start an initialized synthetic company: فروش is the daily landing page; navigate فروش، خرید، انبار، حسابداری. Orders remain orders, invoice and warehouse identities are visible separately in details.
2. Open پیش‌فاکتور, select a customer and catalog product/price, enter quantity, confirm the quote, preview/export, then create a revision. Verify the original stored revision remains unchanged.
3. Open تنظیمات. Enter100.01 or9.755 in نرخ افزودهٔ پیش‌فاکتور (%), press Ctrl+S: input/error/focus remain and saved State is unchanged. Discard; enter۹٫۷۵ and save. Restart:975basis points persist. Existing documents remain unchanged.
4. Switch تم برنامه to تیره and save. Resize to650×760, open/collapse the sidebar, use arrow/Enter and Escape; focus remains visible. Product editor additionally verified at900×560.
5. Inspect [pre-invoice](../docs/assets/ui-copy/pdfs/pre-invoice.pdf), [sales invoice](../docs/assets/ui-copy/pdfs/sales/invoice.pdf) and [purchase invoice](../docs/assets/ui-copy/pdfs/purchases/bill-a.pdf). Synthetic pre-invoice custom header retains its historical wording intentionally; generic labels use the new terms.

Remaining gaps: this is a copy/settings/desktop navigation pass, not completion of treasury, closing, permissions, advanced modules or statutory integration. Detailed accounting exports remain trace reports rather than a redesigned customer invoice template. Wayland, screen-reader speech, high contrast and enlarged text were not tested in this pass. The prior M6 performance run remains attributed to its frozen build, including14misses; no new startup/search/RSS/long-document benchmark was run or claimed. Cloud-local checks were used; no hosted Actions allowance/billing changes, deployment, credentials or live financial data.
