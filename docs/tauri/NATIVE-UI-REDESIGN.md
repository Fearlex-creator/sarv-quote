# Native UI redesign

The UI keeps its existing Button, Modal, Field, DataGrid, Status, Sidebar, and loading composition. The Persian follow-up adds official shadcn source primitives for Input, Textarea, NativeSelect, Table, Skeleton, and Badge, adapted to the shared native theme without adding dependencies. The final instruction was used for ordering and width: Settings first, then the shell and data views, with a `max-w-4xl` equivalent of 896 px throughout.

## Current-screen review, before editing

Current screenshots were captured from the running Rust-backed browser QA app and inspected before implementation. Initial loading captures were rejected and retaken. No older screenshots were used as audit evidence.

1. **Settings / General — needs hierarchy work.** Labels and controls alternate across a two-column form, and large textareas push Save below the fold. The accessible labels are already connected to the fields. Reference: [General](../../artifacts/tauri-native-redesign/before/settings-general.png).
2. **Settings / Agents — functional, inconsistent layout.** Access, connection status, copy actions, and revoke occupy different patterns. Revoke is styled as an ordinary action. The switch has an explicit accessible name and state. Reference: [Agents](../../artifacts/tauri-native-redesign/before/settings-agents.png).
3. **Accounting / Journals — weak scanning hierarchy.** A breadcrumb repeats the page heading; the primary action sits below the top bar. Transactions occupy a large bordered card, amounts are left-aligned, and dates use raw ISO text. No refresh timestamp or sticky table header is present. Status is already represented by both a label and a dot. Reference: [Journals](../../artifacts/tauri-native-redesign/before/accounting-journals.png).
4. **Reports / Trial balance — totals detached from columns.** Totals sit below the table in a separate summary block. Type sizes differ between headings, table labels, amounts, and controls. Units sit in the global context rather than the amount headers. Reference: [Trial balance](../../artifacts/tauri-native-redesign/before/reports-trial-balance.png).

Data-heavy screens: Accounting, Sales, Purchases, Inventory, Reports, and Overview's recent entries. Settings-style surfaces: General, Agents, journal/record/contact details, company setup, and editors. Forms retain their existing fields and validation while sharing the smaller type scale, radius, spacing, and action hierarchy.

## Implemented sequence and health

1. **Settings and Agents — verified.** Grouped lists have labels left, controls right, one container border, and divided rows. Section spacing is 32 px and label-to-group spacing is 8 px. Save lives in the top bar and targets the existing form. Revoke is separated and destructive. Local CLI/MCP and read-only/full permissions retain their existing behavior.
2. **App shell — verified.** Grouped muted sidebar, one page title and its actions in the top bar, neutral surfaces, one blue accent, 24 px page titles, and 14 px remaining text. Shared rounding comes from `--radius`. No heavy shadows. Mobile uses bottom navigation and a More dialog for all pages and the organization selector.
3. **Accounting, Reports, Sales, Purchases, Inventory, and Overview — verified.** Dense tables replace transaction card frames. Headers stick inside the scrolling table area. Monetary columns use tabular digits and right alignment, with currency in column headers. Existing exact `BigInt` formatting is retained; display minus signs are consistent. Toman displays one decimal place so whole and fractional amounts align. Dates use `Intl.DateTimeFormat`, and date input values remain ISO. Backend-supplied trial, profit, and journal-detail totals appear in table footers.
4. **Details and forms — verified.** Details use divided grouped rows. Existing posting/reversal dialogs display a short record-specific consequence during review. Editing a reviewed input still invalidates its existing preview. Shaped table/form skeletons replace loading spinners. Empty financial views use text. Hover, focus-visible rings, and color transitions are retained.

## Accounting and data gaps left unchanged

- Journal, ledger, sales, purchase, and stock list responses do not supply financial aggregates scoped to their displayed rows. No new frontend financial sums were introduced and unrelated report totals were not substituted. Those lists therefore do not have monetary totals rows; only existing backend-supplied totals are displayed.
- Paginated Accounting, Sales, and Purchases rows remain cached after a general workspace refresh. This behavior predates the redesign. It was not changed. The displayed page's timestamp records when that page was fetched, so a general refresh does not falsely make cached rows appear newly loaded.
- Overview's existing reconciliation-difference badge uses the neutral kind. The reconciliation result and classification were preserved rather than changing accounting behavior during the visual pass.
- Stock previously displayed an empty table during report loading. This presentation issue was flagged before replacing that interim view with a skeleton. The request, response, and stock calculation are unchanged.

The Rust accounting kernel, SQLite schema, bridge command names, payloads, calculations, exports, preview-generation guards, and agent service were not modified during this redesign.

## Verification

TypeScript, ESLint, frontend production build, native Tauri release build, Rust-backed browser workflows, and a real WebKitGTK/X11 window are checked. Browser tests include complete procurement/sales/journal workflows, partial goods and original-cost returns, warehouse transfers, opening CSV review, local CLI/MCP grant and revoke, and the focused native UI regression. The focused test renders `9,007,199,254,740,993` without losing precision, verifies the backend's footer totals, and confirms cancelling a validated journal review leaves the journal count and balances unchanged.

The browser suite covers light/dark, English/Persian, keyboard navigation, 1440×900, 1920×1080, 1100×720, 900×640, and 390×844. Browser financial fixtures and native window fixtures use separate synthetic profiles under artifacts. Native navigation was checked against the serialized SQLite accounting state before/after and did not modify it.

Screen-reader behavior and Windows/macOS OS integration were not comprehensively tested. Screenshots alone do not establish full accessibility compliance.

## Persian and shadcn follow-up

Persian covers navigation, grouped settings, local-agent access, table headers and empty states, forms, status badges, toasts, accessible labels, command search, and record-specific confirmation consequences. Language changes persist on the device. A fresh installation follows a Persian system locale; an existing language choice takes precedence. Persian commands retain English search aliases. The command input also supplies cmdk's generated accessible label.

Interface language, display digits, and calendar are independent. Dates use `Intl.DateTimeFormat` with the selected numbering system and Gregorian/Jalali calendar; quantities retain exact thousandths and money retains `BigInt` formatting. Native date-entry controls keep their existing Gregorian ISO values. No Persian calendar picker library was added. Stored dates, quantities, amounts, reasons, command payloads, and accounting calculations retain their existing behavior. The existing Rust parsers already accept Persian digits and the Persian decimal separator.

RTL mirrors sidebar placement, select chevrons, directional icons, tab keys, and toast placement. Text inputs use automatic direction for names and explicit LTR direction for numbers, phones, emails, and ISO dates. Mixed-script confirmation values are isolated; financial cells keep their right alignment. All tab groups use roving focus.

The shadcn configuration now has `rtl: true` and a working `@/*` alias. The registry's source imports use the existing `cn`, Radix Slot, and CVA dependencies. The installed component directory is `desktop/src/components/ui`; `npx shadcn@latest info` recognizes Badge, Button, Dialog, Input, NativeSelect, Sidebar, Skeleton, Table, and Textarea. Existing sidebar, dialog, and field composition is preserved; financial grids and editors use the shared Table primitives.

`tabular-nums` was checked against the installed Tailwind generator and the rendered computed style. The Persian browser regression measures all ten digit widths after Vazirmatn loads, checks right alignment and LTR numeric isolation, and verifies `9,007,199,254,740,993` remains exact. The RTL regression also covers Persian decimal goods input, Jalali dates with Persian and Latin digits, keyboard navigation, a posting review that is cancelled, mobile navigation, and persisted language. It compares all journal/source/resource records and financial report values before and after the UI interactions; the expected workspace revision change after saving preferences is excluded from report comparison.

The rebuilt native WebKitGTK window was checked with Persian settings, local-agent settings, Gregorian/Latin display, and Jalali/Persian display. After restoring the original display preferences, the complete serialized SQLite state matches the starting snapshot. Native proof and screenshots are in `/tmp/sarv-persian-rtl`; browser tests are in `desktop/tests/persian-rtl.spec.ts`. Screen-reader and Windows/macOS platform integration remain unverified.

## After captures

- [Settings](../../artifacts/tauri-native-redesign/after/07-settings.png)
- [Agents](../../artifacts/tauri-native-redesign/after/08-agents.png)
- [Journals, exact large amount](../../artifacts/tauri-native-redesign/after/03-journals-exact-amounts.png)
- [Posting review](../../artifacts/tauri-native-redesign/after/04-posting-review.png)
- [Trial balance totals](../../artifacts/tauri-native-redesign/after/05-trial-totals.png)
- [Ledger](../../artifacts/tauri-native-redesign/after/06-ledger.png)
- [Mobile Agents](../../artifacts/tauri-native-redesign/after/09-agents-mobile.png)
- [Mobile Journals](../../artifacts/tauri-native-redesign/after/10-journals-mobile.png)
- [Native Settings](../../artifacts/tauri-native-redesign/after/12-native-settings.png)
- [Native Agents](../../artifacts/tauri-native-redesign/after/13-native-agents.png)
- [Native Journals](../../artifacts/tauri-native-redesign/after/14-native-journals.png)

Browser test results: `artifacts/tauri-ui/results.json`. Native CLI/MCP clipboard copying and toggle/revoke were also verified in the real window, with agent access off afterward and accounting state unchanged. Native state evidence: `artifacts/tauri-native-redesign/native-evidence.json`.


## Visual comparison

Settings before:

![Settings before](/workspace/sarv-quote/artifacts/tauri-native-redesign/before/settings-general.png)

Settings after, in the native window:

![Settings after](/workspace/sarv-quote/artifacts/tauri-native-redesign/after/12-native-settings.png)

Journals before:

![Journals before](/workspace/sarv-quote/artifacts/tauri-native-redesign/before/accounting-journals.png)

Journals after, in the native window:

![Journals after](/workspace/sarv-quote/artifacts/tauri-native-redesign/after/14-native-journals.png)
