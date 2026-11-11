# Product design and reference evidence

Historical milestone record. The GTK, React, and Tauri applications have been removed. Current commands are in [the README](../README.md). Archived source copies are available in Git history; financial migration fixtures remain in this repository.

Reviewed on 2026-10-07 for the native Rust/GTK4/libadwaita product improvement. This report supersedes the component descriptions in earlier design reports; those screenshots remain historical evidence.

## What was actually viewed

- [Zoho Books quote help](https://www.zoho.com/us/books/help/quote/) and [price-list help](https://www.zoho.com/us/books/help/items/price-list.html): public documentation for customer selection, quotation fields, items, notes/terms, price lists, import mapping/preview and document actions. No authenticated Zoho account or customer screen was inspected.
- [GrandTotal](https://www.mediaatelier.com/en/GrandTotal/): public product information, plus its official [overview image](https://www.mediaatelier.com/Posts/Interactive-Estimates/en/imgs/overview.png) and [options image](https://www.mediaatelier.com/Posts/Interactive-Estimates/en/imgs/options.png). These show a focused desktop document workspace and compact grouped navigation. Some linked support pages returned HTTP 403; no installed GrandTotal session was used.
- Apple's public [Layout](https://developer.apple.com/design/human-interface-guidelines/layout), [Right to left](https://developer.apple.com/design/human-interface-guidelines/right-to-left), and [Lists and tables](https://developer.apple.com/design/human-interface-guidelines/lists-and-tables) guidance. The web pages require JavaScript; their public Apple DocC JSON content was retrieved and read instead. Local raw research is under `artifacts/product-pass/research/`.
- [Stable libadwaita 1.9](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/1.9/) documentation. The cloud runtime uses GTK 4.22.5 and libadwaita 1.9.4; the locked Rust binding enables stable `v1_9` APIs. No 1.11 alpha controls are used.

## Decisions and RTL adaptation

| Observed reference pattern | Sarv decision and user benefit | Persian RTL fit |
| --- | --- | --- |
| Zoho: choose a customer and a price source while entering a quotation | Searchable native customer picker and indexed product picker inside the document. Product result shows its exact code, source price and unit before deliberate selection. Reduces memory and navigation demands. | Customer names/contact/address search in Persian; normalized codes and approved aliases remain exact matching rules. Mixed Latin identifiers retain readable direction. |
| Zoho: customer/document fields precede a central item table; terms/totals follow it | Four compact core fields, optional document details, editable paged rows, visible currency and totals. Short quotes size their table to the actual rows. | Native reading order runs from the right; numeric columns remain aligned and explicit units prevent rial/toman ambiguity. |
| Zoho: price-list/import preview and separate document actions | Native catalog composition or documented CSV template, read-only approval preview, row/column diagnostics, immutable approved sources. Approval, preview and export remain separate. | Persian error messages identify the actual row/column. No arbitrary currency guessing, fuzzy substitution or invented price. |
| GrandTotal: restrained desktop workspace and grouped navigation with useful context | Compact icon/label navigation groups, counts, keyboard hints and native selected state; no decorative dashboard. | Sidebar sits on the leading/right side and collapses to a native navigation control below 1050 px. Alt shortcuts retain the same meaning when collapsed. |
| Apple layout: consistent alignment, hierarchy and stable related controls | Shared page headers, neutral native surfaces, one theme accent, compact manager rows and consistent dialogs/feedback. Primary action changes from approval to preview for an unchanged approved document. | Hierarchy follows Persian reading order. Optional explanations remain near the relevant field instead of occupying the workspace. |
| Apple RTL: mirror reading flow while preserving content meaning | GTK/Pango shaping, display-only directional isolation for Latin codes/dates, rejected user bidi overrides. | Persian text joins correctly; codes and numbers remain comparable rather than being manually reversed. |
| Apple lists/tables: structured rows aid scanning and comparison | Customer rows disclose details on demand; product lists page results; long quotation retains all canonical rows, totals and edits while rendering at most 100 rows/page. | Long names wrap; quantities/prices remain consistent columns, with horizontal scrolling at narrow widths. |
| libadwaita: native adaptive navigation, preferences and dialogs | HeaderBar, NavigationSplitView, native buttons/EntryRows/ComboRows, ExpanderRows, Dialogs and theme variables. | Native keyboard/focus and light/dark controls share behavior; RTL is set at the widget/layout level. |

These are adaptations of public patterns, not claims of feature parity. No reference logos, branding or proprietary assets were copied. Sarv provides one persistent active draft and immutable approved revisions; earlier displaced drafts are recovery backups, not a multi-draft library. Export records local output and never implies customer delivery or acceptance.

## Font choice

The requested Iran Bakh family was not available with verified redistribution permission. Sarv bundles unmodified **Vazirmatn v33.003** Regular, Medium, SemiBold and Bold under SIL OFL 1.1. See [font provenance, checksums and license](../assets/fonts/README.md). It is a Persian-focused, readable alternative, not a claim to be Iran Bakh.

Font bytes are embedded and registered with Fontconfig for this process only before GTK/Pango initialization. No system font installation or runtime download occurs. The same family is requested by the UI and PDF renderer; actual Fontconfig matching and a four-row A4 PDF regression were checked. Noto Sans Arabic remains a fallback. Font registration failure is reported rather than silently presented as success.

## Component contract

Use native theme colors, visible focus, persistent field names and restrained separators. App CSS is scoped to `.sarv-app`; no hardcoded RGB palette, gradients or emoji navigation. Search dialogs disclose exact results; cancellation preserves the draft. Import/restore requires explicit confirmation, and completion messages follow successful persistence. Unknown products remain in a visible review queue. Tables, manager lists and dialogs retain bounded widget counts; validation/totals/export operate on all canonical data.

Current rendered acceptance and measurement limits are recorded in [PRODUCT-ACCEPTANCE.md](PRODUCT-ACCEPTANCE.md) and the final product delivery report. Screenshots establish visual observations only; persistence, calculations and keyboard behavior require separate evidence.
