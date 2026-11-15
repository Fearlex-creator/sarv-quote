# Tauri accounting interface — verified implementation checkpoint

The current application uses one Tauri 2 / React interface and the existing Rust accounting backend. Legacy GTK4 applications, duplicate shells and archived source copies are removed; financial migration fixtures remain. It preserves the Rust accounting kernel, SQLite schema 7, posted history and stored-data compatibility. This is **not the full requested product or a production release**. Missing accounting services and platform release qualification remain explicit below. Native Linux IPC, financial posting, CSV saving and window lifecycle now pass bounded checks.

## Changes and actual workflows

The interface now uses title-only headers and concise controls, without visible descriptive subtext or workflow explanations. Financial review data and explicit confirmations remain visible. Settings has General and Agents tabs. Local agents can use every implemented Tauri operation through CLI/MCP with default-off grants, read-only/full access, revocation and native refresh after accepted writes. See [AGENT-ACCESS.md](AGENT-ACCESS.md).

| Screen | Connected behavior | Boundaries |
|---|---|---|
| Overview | Posted cash/bank, receivables, payables, source-linked recent journals, opening/setup guidance | No invented revenue trends or payment states |
| Company setup | Core chart and fiscal period creation; deliberate zero opening or validated opening CSV review/post | Fresh-company policy; custom chart editing is not yet connected to React |
| Customers and vendors | Create operational parties, inspect Rust-calculated outstanding balances and open sources | Separate from existing quotation contacts; no payment allocation |
| Sales | Create, save, reopen and review durable goods order drafts; record immutable orders; select partial deliveries with paired invoice; restock original-cost customer returns/credits; inspect source and journal | Goods in base units; no service invoices or payments |
| Purchases | Review exact order total; record order; select partial receipts and matched supplier bills; original-cost unbilled returns or billed returns with supplier credit | Temporary purchase editor; no durable purchase drafts or payments |
| Inventory | Create units, warehouses and products; inspect exact quantities and carried values; review/post warehouse transfers at carried cost | Single product/source/target transfer; no adjustment workflow |
| Accounting | Manual journal validation/review/post, journal inspection and reversal, chart inspection, fiscal lock/reopen reasons | Control-account restrictions stay enforced by Rust |
| Reports | Period P&L; cumulative trial balance; subledger control reconciliation; ledger drilldown; CSV export | Control reconciliation is not bank statement reconciliation; no balance-sheet/cash-flow report |
| Settings | Persist appearance, density, exact rial/toman display, digits and quotation issuer settings; local interface language/sidebar preference | Arbitrary backend validation messages may remain untranslated |
| Agents | Settings-controlled local CLI/MCP; 36 tools covering implemented desktop functions; read-only/full grant, revoke and refresh after writes | Running app and private local OS profile required; no remote model provider or multi-user roles |
| Banking | Clear unsupported state and link to available financial reports | No statement import, matching, reconciliation or payments |

## Architecture and shared components

`desktop/src-tauri` invokes `crates/sarv-quote/src/desktop_bridge.rs` through an asynchronous Tauri command. That dispatcher uses the original `Store`, transaction/generation guards and typed accounting/procurement/sales services. The workspace contains only the Rust accounting engine; `desktop/src-tauri` is the only native shell. GTK4/libadwaita and the earlier shells are removed. PDF rendering is opt-in; normal engine and CLI builds have no GUI dependencies. Tauri requires GTK3/WebKitGTK on Linux as platform libraries. There is no accounting schema migration.

Integer financial DTOs cross IPC as decimal strings. Rust computes journal totals, contact balances, exact purchase/sales totals, stock costs, reports and posting previews. JavaScript formats exact integers with BigInt; it does not replace posting or report calculations. Each posting review carries a book generation; stale reviews reject before writing. Request identities support repeated financial submissions, and derived receipt/bill/invoice/return retries reuse the immutable original command and reject changed selected quantities. Quantity selection is validated against the document and company. Partial costs use the original kernel allocation functions, whose only kernel change is crate-local visibility; cumulative allocation preserves the final rial remainder. No synthetic records or HTTP financial adapter are enabled by the production build.

Shared components include Button variants, Sidebar primitives, Radix Dialog/confirmation, Field, PageHeader, Loading/Error/Note states, exact Money formatting, Status, TanStack DataGrid, command palette and a root error boundary. The table supports stable row identities, column sorting, search and explicit empty results. Source/journal lists and ledger details page at 100 records. Search acts on the loaded page; resources and full report aggregates are not yet scalable server-side searches.

Native CSV saving runs entirely in Rust through the scoped native dialog and writes a new file. Production CSP restricts resources to the application and IPC. Native dialog/window-destroy capability is scoped to the main window; no general shell or JavaScript filesystem plugin is added. A local audit actor is not authentication.

## Accessibility, interaction and performance

Implemented: associated field labels and hints, accessible modal titles/descriptions and Radix focus handling, semantic tables and buttons, visible focus, error/loading retry, text alongside financial states, Ctrl/Cmd+K commands, F9 sidebar preference, F1 help, tab arrow/Home/End handling including RTL, content scrolling at the minimum window size and unsaved-editor close handling. Browser navigation resets the content scroll to show each new page's heading. Saved drafts remain durable after reopening. Modal search/entry autofocus and unique description IDs are implemented. Long financial reviews scroll independently while posting actions remain visible; customer return reviews show both credit and original inventory cost.

Known accessibility work: complete WCAG audit, screen-reader testing, native assistive-technology testing, roving-tab semantics, table `aria-sort` announcements and complete localized validation. The UI primitives are installed from the official shadcn registry under preset `b6TqdhPcaf` (Radix Luma). The old primitive styling has been removed. Financial grid/formatting and layout adapters compose those defaults, preserving localized labels and financial review behavior.

Commands run off Tauri's UI thread. Feature screens, editors and command search load on demand; navigation hover/focus preloads modules. Stable callbacks and a memoized books context avoid unrelated feature rendering. Tables use bounded source/ledger pages, cached searchable labels and deferred filtering. International amount/date formatters are reused. Fonts and app resources are bundled; no runtime CDN is required. The main entry chunk is 494.87 kB / 155.04 kB gzip, down from 527.52 kB / 156.64 kB gzip. Other feature chunks load separately; this does not measure the total downloaded payload. Startup latency, memory, long-document input latency and thousand-record responsiveness have **not** been measured for this interface; no performance budget or native responsiveness acceptance is claimed.

The first-run welcome and company/fiscal/opening wizard use the existing shadcn primitives, English/Persian labels and mirrored RTL controls. Back navigation preserves input and opening posting remains explicitly confirmed. Brief CSS transitions animate pages, dialogs, setup stages and controls; financial rows and numbers remain stationary. Reduced-motion disables animation and transitions. Dependencies are the official preset's requirements; the older Lucide/Inter and direct Radix dependencies were removed.

## Verification and evidence

Current cleanup/performance checks are listed below. Older native goods/CSV/agent evidence remains identified as prior-checkpoint evidence; its historical Rust suite counts do not supersede the current upstream serialization failures.

| Check | Result and evidence |
|---|---|
| Default headless Rust suite | 201 passed, 3 inherited serialization/migration failures, 3 ignored; `/tmp/sarv-motion-core-final.log` |
| Optional PDF suite | 243 passed, the same 3 inherited failures, 3 ignored; `/tmp/sarv-cleanup-pdf-tests.log` |
| Bridge integrity tests | 11 tests: original seven gates plus partial allocation/returns, transfer conservation, quantity/retry/source isolation and malformed-input rejection |
| Rust Clippy | Core command completed with two inherited sort warnings; `/tmp/sarv-cleanup-clippy.log` |
| TypeScript, ESLint and production frontend build | Passed; `/tmp/sarv-preset-native-final.log` |
| Tauri release build, official CLI | Built with `--no-bundle`; `/tmp/sarv-preset-native-final.log`; binaries `target/release/sarv-agent` and `desktop/src-tauri/target/release/sarv-desktop` |
| Browser end-to-end against the real Rust dispatcher | Nine tests passed, including English/Persian first run, reduced motion, exact amounts, RTL, local agents and complete financial workflows; `/tmp/sarv-preset-browser-final.log`, `artifacts/tauri-ui/results.json` |
| Prior native IPC, goods posting, CSV and window lifecycle | Passed on X11/WebKitGTK: partial receipt, matched bill, original-cost supplier return, reconciled controls, native CSV 1,251 / 1,251 IRR, command focus/navigation, unsaved keep/discard and restart; `artifacts/tauri-native-evidence.json`, `artifacts/tauri-native-window-checks.log` |
| Local agents | Six Rust integration tests; native Settings/clipboard/CLI/MCP posting and reports, read-only/revocation and automatic refresh passed; `artifacts/tauri-agents-tests.log`, `artifacts/tauri-agents-native-evidence.json` |
| Native preset/first run | English/Persian welcome, setup stage navigation/cancel and deferred command search checked on WebKitGTK/X11; serialized books unchanged; `/tmp/sarv-preset-native/` |
| Windows/macOS packaging, signing and installation | Not validated |

The browser workflow creates an isolated synthetic company, explicitly initializes zero balances, creates the five operational resources, purchases 5 × 10 = 50, receives and bills once, saves/reopens a sales draft, posts 2 × 20 − 1 = 39 with 20 COGS, and checks 19 profit. An unbalanced manual journal is rejected; a valid expense of 5 produces 14 profit. It checks balanced trial/control reports, ledger access, CSV download, preferences, dark/RTL, command navigation, tab keyboard handling, sidebar persistence and absence of page errors. A second test imports the repository's independent synthetic opening CSV into a separate company with unique resource IDs, inspects the 1,251 IRR balanced totals and control comparison, verifies that preview writes nothing, then confirms posting and restart persistence. A fourth agent test grants CLI/MCP access from Settings, uses the copied configuration to create books and read them, checks read-only denial and revocation, and confirms no visible helper subtext. A third test selects partial receipt/bill/delivery quantities, rejects over-receiving, posts customer and supplier returns, moves the final inventory to another warehouse, and independently checks 19 IRR receivables, zero payables, 15 IRR profit and 2 IRR carried stock with reconciled controls. Browser QA disables live file watching so native build artifacts cannot interrupt a financial review. Restart the adapter and Vite after source changes before rerunning QA. All tests use the same Rust dispatcher as Tauri through an explicitly enabled loopback QA adapter. They verify the presentation and accounting integration, not the native webview transport or OS dialogs.

Native compilation and execution use authenticated distribution libraries extracted into a workspace sysroot. `scripts/run-tauri-cloud.sh` resolves WebKit's compiled-in helper paths through a temporary read-only library view. System files remain unchanged, WebKit's sandbox remains enabled, and writable application state is restricted to an isolated synthetic profile. A normal Linux desktop with the Tauri runtime packages does not need this cloud workaround. `scripts/qa-tauri-window.py` reproduces keyboard, native CSV and unsaved-close checks on the private 1440×900 X11 display. Financial posting was exercised with real X11 input and the resulting exact source records/journals verified in SQLite. The native receipt/bill/return restored Inventory 300, GRNI 50 and Payables 149, with every control difference zero. The evidence records the final binary hash. Wayland, Windows/macOS and installers/signing remain untested.

## Screenshot review and design fidelity

The current curated capture manifest contains 59 browser captures and 5 native agent captures under `artifacts/tauri-ui/`. They contain clearly named QA records produced by the real dispatcher. Overview, invoices, purchases, inventory, accounting, reports, customers and settings are captured at 1440×900, 1920×1080, 1100×720 and 900×640. The matrix checks no document-level horizontal overflow, and the full workflow reports no page errors. Workflow captures include onboarding, opening balances and CSV control review, goods order editor, invoice posting review/detail, supplier bill detail, manual journal editor, P&L, ledger, control reconciliation, dark mode and Persian RTL. Current captures cover partial receipts/bills/invoices, original-cost customer and supplier returns, warehouse transfer review/result, minimal Settings agent access states and native writes appearing automatically. Earlier native financial/save/window-close captures remain in the artifacts directory as prior-checkpoint evidence. `reviewed-screenshots.json` identifies current/selected captures; other native images preserve earlier diagnostic iterations. Unsupported expense attachment and bank reconciliation flows cannot be demonstrated.

The concept image informed the overview composition. Final implementation follows the user's exact design tokens where the image differs. Reviewed comparisons:

| Point | Actual implementation and refinement |
|---|---|
| Shell geometry | 232px normal sidebar, 56px toolbar; compact/collapsed navigation fits 900px minimum width |
| Palette and elevation | Warm-neutral prescribed backgrounds, the preset's teal primary action, semantic positive/error states, restrained borders |
| Typography | Preset IBM Plex Sans/Space Grotesk with bundled Vazirmatn for Persian; tabular financial digits |
| Content fidelity | Rust-backed financial position and real source records replace concept placeholders; unsupported payment language removed |
| Table/form rhythm | Shared table/search/form components; invoice review separates financial lines from final commit |
| Rendering defects corrected | Stable table column IDs, distinct accessible field labels, display-digit preference separate from locale, retained-scroll heading clipping, posting-review generation invalidation, command autofocus, native close permission and visible review actions |

The generated concept has different empty-state content from initialized QA books; the final interface is not claimed to be pixel-identical. Screenshots were inspected and recaptured after corrections. Small dataset screenshots do not establish high-volume ergonomics.

## Evidence-based review rubric

Scale: 0 absent, 1 initial, 2 partial/unvalidated, 3 works in bounded checks, 4 consistently supported by implemented behavior and checks, 5 full release qualification. These are checkpoint judgments, not user-study results.

| Area | Score / 5 | Evidence or remaining limit |
|---|---:|---|
| Visual consistency | 4 | Shared tokens/components and inspected main screens |
| Typography | 4 | Bundled Latin/Persian fonts and consistent scales |
| Layout precision | 4 | Four-size browser captures and genuine native chrome/dialog inspection |
| Information hierarchy | 4 | Position, sources, financial review and commit separated |
| Navigation | 4 | Shared shell, palette, source/journal links |
| Keyboard efficiency | 3 | Commands, tabs and sidebar checks; advanced entry shortcuts incomplete |
| Table quality | 3 | Sort/search/paging; no virtualized large datasets or bulk operations |
| Form quality | 3 | Durable sales drafts and validation; purchase drafts absent |
| Financial clarity | 4 | Exact Rust amounts, posted/delivered states and linked journal effects |
| Responsive behavior | 3 | Main screens at four sizes; all editor combinations not covered |
| Native desktop feel | 3 | Genuine IPC/financial posting, CSV dialogs, keyboard focus and window lifecycle; other platforms pending |
| Accessibility | 3 | Labels/focus/modal/keyboard work; full WCAG/assistive testing pending |
| Dark mode | 3 | Implemented and inspected; exhaustive contrast audit pending |
| Localization | 2 | RTL and digit handling; copy and calendar parity incomplete |
| Performance | 2 | Bounded lists/off-thread commands; runtime budgets unmeasured |
| Reliability | 3 | Regression/integrity/persistence checks; native crash/recovery qualification pending |
| Workflow completeness | 2 | Connected goods accounting; payments/banking/services/attachments absent |
| Rust integration | 4 | Existing kernel, Store guards and checked totals, real end-to-end effects |
| Maintainability | 4 | Typed shared boundary/components, preserved kernel/schema, reproducible checks |
| Professional trust | 3 | Honest states and traceability; roles, compliance and release gates incomplete |

Weakest areas are workflow breadth, platform release qualification, localization and measured performance. Work focused first on real posting, financial precision, persistence and stale/duplicate protection; visual improvements do not substitute for these gates.

## Prioritized remaining work

1. Broaden native qualification to supported Wayland/macOS/Windows desktops, CSV cancel/existing-file/error behavior and crash recovery. Package/sign/test installers on target OSes.
2. Define and implement typed payment/allocation/refund and bank statement/reconciliation services with invariant tests before enabling those screens. Add service invoicing and tax/legal policy only with explicit business rules.
3. Port preserved quotation/PDF; add durable purchase drafts, richer bill-line/source navigation and attachment storage/recovery. Partial goods selection, returns and transfers now work in the Tauri interface.
4. Complete Persian copy, Jalali date editing/display parity, WCAG/screen-reader/contrast audit and keyboard entry shortcuts.
5. Measure startup/memory/input/search with representative 1k/10k records; add indexed backend search, resource paging and table virtualization where measurements justify it.
6. Add authorization/roles, backups/recovery UI, approvals/projects and statutory integrations under separately specified policies.

Run instructions are in `desktop/README.md`; architecture inventory is in `AUDIT.md`; bounded public reference research is in `RESEARCH.md`. Do not enable the development QA adapter on real books or treat this checkpoint as production acceptance.
