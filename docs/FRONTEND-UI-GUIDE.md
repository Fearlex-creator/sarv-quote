# Daftar — Frontend UI Build Guide
### Tauri v2 + React + shadcn/ui · RTL-first · Persian (fa) + English (en)

> Companion to **[ACCOUNTING-SPEC.md](./ACCOUNTING-SPEC.md)**. The spec defines the domain, API and backend milestones. This guide defines **every screen, interaction, component and convention** of the UI. The same React app runs inside Tauri (desktop) and in the browser (web build).

| § | Section |
|---|---|
| 1 | UI principles & tech stack |
| 2 | First-run onboarding: screen by screen |
| 3 | App shell |
| 4 | Dashboard |
| 5 | Module screens (list / detail / form) |
| 6 | Managing the books |
| 7 | Sharing the books |
| 8 | Design system |
| 9 | Component inventory |
| 10 | State & data layer |
| 11 | Offline & sync UX, states, accessibility |
| 12 | Keyboard shortcut map |
| 13 | Folder structure |
| 14 | UI features checklist |
| 15 | Frontend-only Claude Code prompt |

---

## 1. UI principles & tech stack

**Principles**
1. **RTL-first, never RTL-afterwards.** Build every screen in Persian first, then verify in English. Use only logical CSS (`ms-*`, `pe-*`, `start-*`, `text-start`). shadcn/ui's CLI generates these when `rtl: true` is set in `components.json` [shadcn RTL docs: https://ui.shadcn.com/docs/rtl].
2. **Keyboard-first.** An accountant enters hundreds of lines a day. Every screen is fully operable without a mouse. Every action has a command-palette entry.
3. **Numbers are the content.** Tabular figures, right-aligned in LTR / left-aligned in RTL **for numeric columns, aligned by the decimal**, and consistent separators. Money is never rendered from JS `number`.
4. **Show state, not surprises.** Every document shows its status (draft / posted / reversed / provisional / Moadian state / locked) with an explanation of what is allowed next and why.
5. **Progressive disclosure by tier.** Solo users see a lean sidebar. Features appear as the company's tier or flags enable them.
6. **Fast perceived speed.** Skeletons < 100 ms, virtualised lists, prefetch on hover/focus, optimistic UI only for drafts.
7. **Calm visual design.** Neutral surfaces, one accent colour, semantic colours only for meaning (debit/credit are **not** red/green; red means error or overdue).

**Tech stack (fixed)**
| Concern | Choice |
|---|---|
| Build | Vite + React + TypeScript (strict) |
| UI kit | shadcn/ui (`rtl: true`, `DirectionProvider`), Radix/Base UI primitives, Tailwind CSS v4, `lucide-react` icons, `tw-animate-css` |
| Routing | TanStack Router (file-based, typed search params for filters) |
| Server state | TanStack Query (cache, prefetch, invalidation by domain events) |
| Tables | TanStack Table + TanStack Virtual |
| Forms | react-hook-form + zod (`@hookform/resolvers`) |
| Command palette | `cmdk` (shadcn `Command`) |
| i18n | i18next + react-i18next (ICU plurals), namespaces per module |
| Dates | Jalali: `date-fns-jalali` or `@internationalized/date` Persian calendar (`⚠ verify` choice in a spike; must match Rust `kernel::jalali` golden table) |
| Decimals | `decimal.js` or `big.js` (pick one; wrap in `lib/money.ts`) |
| Charts | Recharts (via shadcn `chart`) |
| Desktop bridge | `@tauri-apps/api` + generated `bindings.ts` (tauri-specta) |
| HTTP | generated OpenAPI client (`openapi-typescript` + `openapi-fetch`) |
| Testing | Vitest + Testing Library, Playwright (web), `tauri-driver` WebDriver (desktop), Storybook (component catalogue + visual tests in fa/en, light/dark) |
| Font | Vazirmatn (variable, self-hosted, includes Latin), `font-feature-settings: "tnum"` for numbers. Optional "Vazirmatn FD" variant (Farsi digits baked in) is **not** used. We control digits in code (§8.3) |

---

## 2. First-run onboarding: screen by screen

The wizard is full-screen with a progress rail at the inline-start. Every step can be skipped and finished later from **Settings → Setup checklist**. State persists after each step, so quitting resumes where the user left off. It must take under 3 minutes to reach "first invoice" with defaults.

| # | Screen | Fields / content | Behaviour & validation |
|---|---|---|---|
| 0 | **Welcome / language** | Large toggle **فارسی / English**, theme (light/dark/system), calendar (Jalali default / Gregorian), digit style (Persian/Latin) | Switches `dir` + locale instantly, without a reload. Choices are saved to the local profile |
| 1 | **Mode** | Cards: *Work on this computer (offline)* · *Connect to a server* (URL + login) · *Restore from backup* · *Open sample company* | Connect validates the URL, TLS and server version. Restore opens the file picker (Tauri dialog) |
| 2 | **Business type (tier)** | Cards: Solo/freelancer · Small business · Mid-sized · Enterprise group. Each card lists what will be switched on | Sets `tier` feature preset; editable later. Enterprise suggests server mode |
| 3 | **Company info** | Legal name (fa/en), type (person/legal), national ID or legal entity ID, economic code, registration no., address with province/city pickers, postal code, phone, logo upload, branches (optional) | Live checksum validation of IDs and postal code. The logo is cropped in-app. Fields are optional for Solo |
| 4 | **Fiscal year** | Start date (Jalali picker, default 1 Farvardin of the current year), length (12 months), period type (monthly), first year to record | Shows the computed end date and periods preview. Warns if the start date is mid-year ("you'll enter opening balances as of …") |
| 5 | **Currency & money display** | Base currency (IRR fixed for Iranian companies; others allowed), display unit **Rial / Toman**, other currencies used (USD, EUR, AED, CNY, TRY…), rate types | Explains "amounts are stored in Rial; you can view Toman". Preview of formatted sample amounts |
| 6 | **Chart of accounts template** | Cards: Trading, Services, Manufacturing, Contracting, Solo, Import from Excel. Preview tree on the side, with code length settings (kol/moein digits) and the tafsili levels toggle | Searchable preview. The user can edit the tree after creation. Templates are fa/en |
| 7 | **Tax & Moadian** | Registered for VAT? Economic code, fiscal memory ID(s), connection method (Direct with certificate / Trusted provider / Later), default VAT code | Certificate upload goes to secure storage. "Test connection" runs against the sandbox/mock. Can skip |
| 8 | **Banks, cash & cheques** | Add bank accounts (bank picker with logos, Sheba with validation, account no.), cash boxes, petty-cash holders, cheque books (serial range) | Sheba auto-detects the bank. Each becomes a tafsili automatically |
| 9 | **Opening balances** | Options: *Start fresh* · *Enter balances* (grid by account/tafsili with running Dr/Cr difference) · *Import from Excel* (template download) · inventory opening quantities and costs · open AR/AP invoices · cheques on hand | The difference bar must reach 0 to post the opening entry, or difference → "Opening balance equity" with a warning |
| 10 | **Import from other software** | Sources: Excel (generic), Hesabfa export, Sepidar, Holoo. Step flow: choose source → upload → **map columns** (remembered) → **validate preview** (errors per row, fix in-grid) → import → summary | Runs in the background with progress. Idempotent re-run. Errors can be downloaded as Excel |
| 11 | **Invite team** (SB+) | Email/phone, role, companies; *Invite my accountant* shortcut with an "External accountant" role | Hidden in Solo-local. Shows "requires sync/server" with an upgrade path |
| 12 | **Sample data** | Toggle "Add sample customers, items and invoices so I can explore". Option to wipe sample data later in one click | Sample data is tagged `is_sample` and excluded from legal reports, with a watermark |
| 13 | **Done** | Setup checklist card: create first invoice, connect bank statement, set invoice template, back up settings | Lands on the Dashboard with a "next steps" panel |

Onboarding UX details:
- Inline help is a "?" popover per field with a 1–2 sentence explanation in plain Persian (e.g. what a "تفصیلی" is).
- Keyboard: Enter moves to the next field, Ctrl+Enter moves to the next step, Alt+← / Alt+→ go back/forward (mirrored in RTL).
- Sample data and import can be combined; import never touches sample-tagged rows.

---

## 3. App shell

```
┌───────────────────────────────────────────────────────────────────────────┐
│ Top bar: [Company switcher ▾] [FY 1405 ▾]  [🔍 Search… Ctrl K]  [+ New ▾]  │
│          [Sync ●] [🔔 3] [?] [Avatar ▾]                                     │
├──────────┬────────────────────────────────────────────────────────────────┤
│ Sidebar  │ Breadcrumbs · Page title · Page actions                        │
│ (start)  │ ┌──────────────────────────────────────────────────────────┐   │
│          │ │ Content (list / detail / form / report)                   │   │
│          │ └──────────────────────────────────────────────────────────┘   │
│          │ Optional end-side panel: details / comments / activity        │
└──────────┴────────────────────────────────────────────────────────────────┘
```
(In RTL the sidebar is on the right. Everything is mirrored by logical properties.)

- **Sidebar** (shadcn `Sidebar`, collapsible to icons with `Ctrl+B`). Groups: Dashboard · Sales (invoices, quotes, receipts, customers) · Purchases (bills, expenses, payments, suppliers) · Banking (accounts, reconciliation, cheques, cash) · Inventory (items, stock, warehouses, counts) · Payroll · Fixed assets · Accounting (journal, COA, tafsili, periods) · Tax (Moadian, VAT, seasonal) · Reports · Settings. Items appear by tier and permission. Users can pin favourites and reorder within groups. Badges show counts (e.g. Moadian failed 2, cheques due today 4).
- **Command palette** (`Ctrl/⌘ K`, shadcn `Command`): navigation ("Go to aging report"), creation ("New invoice for Ali…"), actions on the current record ("Post", "Reverse", "Send to Moadian"), settings toggles (theme, language, density), recent records. Fuzzy search with Persian normalisation (ی/ي, ک/ك, ZWNJ‑insensitive, Persian/Latin digits). It shows shortcut hints. Prefixes: `>` actions, `#` document numbers, `@` parties, `/` reports.
- **Global search** (`/` or `Ctrl+Shift+F`): full-text over parties, items, documents (by number, amount "12,500,000", Sayad ID, Sheba, national ID), and accounts. Results are grouped by type and keyboard-navigable, with a preview pane.
- **Quick-create** (`+ New` menu or `N` then a letter): `N I` invoice, `N B` bill, `N R` receipt, `N P` payment, `N J` journal entry, `N C` customer, `N Q` cheque, `N E` expense. Each opens a **sheet** (side drawer) for fast entry or a full page.
- **Notifications** (bell, shadcn `Popover` + list): approvals waiting, mentions, Moadian failures/rejections, cheques due, sync conflicts, backup status, data-pack updates. Mark read, filter, deep links. Desktop native notifications via the Tauri plugin (opt-in).
- **Company switcher**: search companies, show role and tier per company, "Add company". Switching keeps the route if the target company supports it. A coloured company badge is optional, to avoid posting to the wrong company.
- **Fiscal-year selector**: affects default date ranges. Closed years show a lock icon.
- **Sync indicator**: ● green synced / ◐ syncing / ● amber offline (n queued) / ● red conflicts. Click opens the Sync panel (§11).
- **User menu**: profile, language, theme, density, digits, shortcuts sheet (`?`), about (app + data-pack versions), sign out/lock.
- **Window chrome (desktop)**: native title bar or custom (Tauri `decorations`). Remember window state. Single-instance. Deep links `daftar://invoice/…`.

---

## 4. Dashboard

Layout: responsive grid of cards (12-column). The user can rearrange and hide cards (persisted per user and company). The time range defaults to the current Jalali month, with comparison to the previous period.

| Card | Content | Interaction |
|---|---|---|
| Cash position | Total cash + bank balances, by account, sparkline 30 days | Click → bank account register |
| Receivables | AR total, overdue, aging bar (0–30/31–60/61–90/90+) | Click a bucket → filtered aging |
| Payables | AP total, due this week | → AP aging |
| Cheques | Received due in 7 days (count/amount), issued due in 7 days, bounced this month | → Cheque calendar |
| Sales & gross margin | Month vs previous, chart by day/week | → Sales report |
| Expenses by category | Top 6 + other | → P&L drill |
| Moadian health | Sent / accepted / pending buyer / failed / approaching deadline | → Moadian queue filtered |
| Bank reconciliation | Unreconciled lines per account, last reconciled date | → Reconcile |
| Inventory alerts (SB+) | Below reorder, expiring batches (M) | → Items filtered |
| Payroll (SB+) | Next run date, last run total, DISK file status | → Payroll |
| Approvals (SB+) | Waiting for me | → Approvals inbox |
| Setup checklist | Remaining onboarding tasks | Dismissible |
| Cash-flow forecast (M) | 30/60/180-day projection | → Forecast |

All figures show the "as of" timestamp and a provisional-data badge if offline postings are included.

---

## 5. Module screens

### 5.0 Shared patterns (apply to every module)

**List screen**
- Header: title, count, primary action (`New …`), secondary menu (import, export Excel/PDF/CSV, print, saved views).
- **Filter bar**: quick search, date range (Jalali presets: today, this week, this month, this season (فصل), this FY, custom), status chips, party/account/tafsili pickers, amount range, "more filters" sheet. Filters live in typed URL search params (TanStack Router), so they are shareable and bookmarkable.
- **Saved views**: name, columns, sort, filters, density; personal or shared (SB+).
- **Data table** (`DataTable` §9): virtualised, server-side sorting/pagination (cursor), column resize/reorder/hide, pinned first column, totals row for numeric columns (sums of filtered set from the server), row selection with bulk action bar (post, send to Moadian, export, delete drafts), keyboard navigation (↑↓, Enter opens, Space selects, `x` toggles).
- Row status badges plus a hover/focus preview popover (key fields) without leaving the list.
- Empty state with a CTA and an import link (§11.2).

**Detail screen**
- Header: document number, status badge(s), party, date (Jalali + tooltip Gregorian), totals. Actions follow the state machine (Edit draft, Post, Reverse, Duplicate, Print/PDF, Share link, Send to Moadian, Record payment…). Disabled actions show a tooltip with the **reason** from the API.
- Tabs: *Overview* (printable layout preview) · *Accounting* (the journal entry it produced with links to ledger) · *Payments/Allocations* · *Moadian* (status timeline) · *Attachments* · *Activity* (audit + comments).
- End-side panel toggle for comments (§7.6).
- Prev/next record navigation (`J`/`K`) within the current list's filter.

**Form screen**
- Header form (party, dates, refs) in a 2–3-column grid. A line grid (`EntryGrid` §9) below. Totals panel at the inline-end. A footer with notes/terms/attachments.
- **Autosave of drafts** every few seconds (debounced) with a "Saved" indicator. Explicit **Post** (`Ctrl+Enter`). `Esc` asks before discarding unsaved changes.
- zod schema mirrors the server DTO. Server validation errors map to fields by `pointer`.
- Defaults are remembered per user (last warehouse, last cash box, VAT code by item).

### 5.1 Sales: invoices (فاکتور فروش), quotes, receipts

- **Invoice list:** columns: number, date, customer, due date, amount, VAT, total, paid, balance, status (draft/posted/partially paid/paid/overdue/void), Moadian status icon, currency. Quick filters: overdue, unpaid, not sent to Moadian, this month.
- **Invoice form:**
  - Header: customer (`PartyPicker` with create-inline; shows balance, credit limit, colour badge if a cheque bounced), invoice type for Moadian (type 1/2, pattern) auto-suggested from customer data, date, due date/terms, price list, warehouse, currency + rate (when FX), sales person/marketer (M), project/dimensions.
  - Lines (`EntryGrid`): item/service (`ItemPicker`: name, code, barcode scan), description, qty, unit (multi-UoM M), unit price, discount (%/amount), VAT code (auto), VAT amount, line total. Live stock-on-hand hint and margin hint (permission-gated).
  - Footer: notes, terms, attachments. Totals: subtotal, discounts, VAT, additions (freight), total in base and FX, amount in words (Persian "به حروف") with the toggle Rial/Toman.
  - Actions: Save draft, Post, Post & new, Post & print, Post & send (link/SMS/email), Receive payment now.
- **Invoice detail:** print preview (template chosen per company/branch), payments tab, Moadian tab with timeline (queued → sent → pending buyer → accepted/rejected), error panel with Persian explanation and fix action (e.g. "product ID missing on item X → open item").
- **Corrective / cancel / return wizards:** a stepper that explains what the law allows (e.g. "Buyer identity can't be corrected; cancel and reissue"), shows a diff of original vs corrective, and produces the linked documents.
- **Quote (پیش‌فاکتور):** same form; actions Convert to invoice/order, Send, Mark accepted/lost. Validity date.
- **Receipt (دریافت):** party, date, method rows (cash / bank transfer / card-POS / cheque (opens `ChequeFields`)), allocation grid to open invoices (auto-allocate oldest first, editable), unapplied remainder. One receipt can mix methods.
- **Customer detail:** profile, IDs, balance, statement (صورتحساب) with Jalali date range and PDF/Excel/share, open invoices, cheques, activity, contacts, portal access (SB+).

### 5.2 Purchases & expenses
- **Bills (فاکتور خرید)** list/form mirror sales, with supplier reference number, purchase VAT (input), "seller registered in Moadian?" flag (affects seasonal report), and link to the incoming Moadian invoice (SB+: an inbox of invoices issued to us, with Accept/Reject inline and "Create bill from this").
- **Quick expense:** a minimal sheet: amount, paid from (cash/bank/petty cash/person), category (account), VAT, receipt photo/drag-drop, OCR prefill (SB+) with field confidence highlights.
- **Purchase orders → goods receipt → bill** (SB+): conversion actions and a 3-way match indicator (qty/price differences highlighted).
- **Payments (پرداخت):** mirror receipts; cheque issue opens `ChequeFields` with cheque-book leaf picker.
- **Supplier detail:** like customer detail.

### 5.3 Inventory
- **Items list:** code, name, category, unit, on-hand (per warehouse via expandable row), avg cost, sale price, Moadian product ID present ✓/⚠, active. Bulk edit prices/VAT codes.
- **Item form:** type (goods/service), names fa/en, codes, barcode(s), units + conversions (M), category, accounts (inventory/COGS/revenue defaults from category), VAT code, Moadian product ID (13-digit field with lookup help link), tracking (none/batch/serial, M), reorder point, images.
- **Item detail:** stock card (کاردکس) with running qty and value, movements, price history, batches/serials (M).
- **Stock moves:** receipt, issue, transfer, adjustment forms with grid entry. **Stock count (انبارگردانی):** create count sheet (filter by warehouse/category), print/export, enter counted qty with a scanner-friendly mode, review variances, post adjustment.
- **Warehouses:** list/form; bins (M).

### 5.4 Banking, cash & reconciliation
- **Accounts overview:** cards per bank/cash/petty cash with balance, unreconciled count, last statement date.
- **Account register:** transactions list with running balance, filters, and "reconciled" checkmarks.
- **Statement import:** upload Excel/CSV → bank mapping auto-detected (data pack) or manual column mapper (date (Jalali/Gregorian parse), description, debit, credit, balance, reference) → preview → import. Duplicate detection by hash.
- **Reconciliation workspace** (the most important screen for daily work):
  - Two panes: **statement lines** (inline-start) and **ledger candidates** (inline-end). In RTL the mirrored order is fine; the spatial meaning is "bank ↔ books".
  - Each statement line shows suggested matches with a confidence badge and reason ("same amount, ref 4471, 1 day apart").
  - Actions per line: `Enter` accept suggestion, `M` manual match (multi-select to match one-to-many), `C` create transaction (prefilled from rule/OCR), `T` transfer between accounts, `S` split, `X` exclude (with reason, never delete).
  - **Rules:** "Always book lines containing 'کارمزد' to Bank fees" with conditions/preview of affected lines.
  - Running difference bar: statement closing balance vs reconciled balance; "Finish reconciliation" locks the matched lines and produces a reconciliation report PDF.
- **Transfers** between accounts; **cash count** (close of day) with denomination grid (optional).

### 5.5 Cheques (چک‌ها)
- **Cheque list** with tabs Received / Issued, status chips (in hand, in collection, cleared, bounced, spent, returned, issued, voided), due-date range, bank, party. Columns: Sayad ID (formatted 4-4-4-4), serial, bank, due date (with "in 3 days" relative), amount, party, status, colour status (if captured).
- **Calendar view**: month grid (Jalali) with daily totals in/out; click a day → list.
- **Cheque detail:** front/back images, fields, **lifecycle timeline** with the journal entry per step, available next actions (Deposit, Clear, Bounce, Spend to supplier, Return to customer) via state machine.
- **Batch actions:** select many → Deposit to bank X / Clear on date Y. A confirmation sheet shows the resulting postings summary.
- **Sayad steps helper:** for transfer/issue, a checklist reminding the user to register/confirm in the bank app, with a field to record confirmation date/reference.
- **Cheque books:** list of books with leaf grid (used/void/available).

### 5.6 Payroll (SB+)
- **Employees** list/form: personal (national ID, birth date Jalali, insurance no.), contract (start/end, base wage, job code for insurance `⚠ verify` code list), pay elements, bank Sheba, workshop.
- **Payroll run wizard:** choose month → import/enter attendance (days worked, overtime hours, leave) in a grid → calculate → review grid (employee rows × elements; differences vs last month highlighted) → approve → post → outputs.
- **Outputs page:** payslips (bulk PDF, share individually), bank file, insurance DISK (DSKKAR00/DSKWOR00 download as zip + summary), salary-tax list, payroll journal entry link.
- **Settings:** pay elements editor (taxable/insurable flags), yearly law data review (shows data-pack values with ⚠ verify badges).

### 5.7 Accounting: journal, COA, tafsili
- **Journal list** (اسناد حسابداری): number, date, description, source (manual/invoice/…), total, status, created by. Filter by source and status.
- **Journal entry form:** `EntryGrid` with columns: account (`AccountPicker`), tafsili 1–4 (`TafsiliPicker`, enabled per account rules), description (inherits header), debit, credit, (FX: currency, rate, FC amount), dimensions. A live **balance bar** shows Δ = debit − credit; `=` on a line fills it with the remaining difference. Templates and recurring setup.
- **Chart of accounts:** tree table (group → kol → moein) with code, name fa/en, nature, balance, tafsili rules column. Inline add child, drag to reorder (codes are re-generated only for unused accounts), deactivate.
- **Tafsili:** list by type with balances; detail shows all accounts it appears in.
- **Periods & locks:** see §6.

### 5.8 Tax: Moadian, VAT, seasonal
- **Moadian queue:** tabs All / Needs action / Pending buyer / Accepted / Failed / Cancelled; deadline column ("8 days left", amber at ≤ 3, red overdue). Bulk send/retry. Detail drawer shows request/response JSON (for advanced users), Tax ID, reference number, and an error explainer.
- **Incoming (cartable):** invoices issued to us; accept/reject with reason; link to bill.
- **VAT return worksheet:** period selector (season), output VAT, input VAT, adjustments, payable, drill-down, export.
- **Seasonal report:** list of transactions not covered by Moadian, missing-data warnings (national ID/postal code), export TTMS file.
- **Settings:** fiscal memories, certificates (expiry warnings), provider selection, test connection.

### 5.9 Reports
- **Report gallery:** categories (Financial statements, Ledgers, Receivables, Payables, Inventory, Tax, Cheques, Payroll, Custom). Search, favourites, recently used.
- **Report viewer:** parameter bar (date range, level kol/moein/tafsili, comparison periods, company/branch, dimensions, currency/unit Rial/Toman, include provisional?), result as a virtualised tree-table with expand/collapse levels, drill-down on any amount → ledger → entry → source document (breadcrumb trail, back with `Backspace`). Export Excel (with formulas for totals), PDF (fa/en print layout with header/footer, page numbers in Persian digits), CSV. Schedule (SB+), save as view.
- **Trial balance:** 2/4/6/8-column toggles.
- **Statement designer** (M): rows mapped to account ranges, preview.

---

## 6. Managing the books

### 6.1 Period close
- **Close center** (Accounting → Periods): a timeline of periods for the FY with status (open / soft-locked / hard-locked) and a per-period **close checklist**: bank accounts reconciled, Moadian queue empty or explained, cheques due reviewed, depreciation run, payroll posted, FX revaluation (M), accruals, inventory count variances posted, trial balance reviewed. Items auto-check from data where possible; manual items need a tick and an optional note. M: owners and due dates per task.
- **Lock period** dialog: choose soft/hard. Shows what will be blocked and who can override. Requires a confirmation phrase for hard lock (type the period name).
- **Year-end close wizard**: (1) pre-checks (unbalanced subledgers, unposted drafts, open Moadian errors), (2) preview of the closing entries (temporary accounts → summary → retained earnings), (3) post closing entry (اختتامیه), (4) create next FY and opening entry (افتتاحیه), (5) optionally hard-lock the old year. Every step is reversible until the final lock. Progress is persisted.

### 6.2 Locks & immutability in the UI
- Posted documents show a 🔒 badge with the tooltip "Posted documents can't be edited. Use Reverse or Amend." The **Amend** action creates the reversal + new draft and highlights changes.
- Locked-period edits are blocked with an inline explanation and (if permitted) "Request override" → approval.

### 6.3 Audit trail
- **Activity tab** on every record: who/when/device/what changed (field diff, old → new, with amounts formatted), and system events (posted, sent to Moadian, synced).
- **Audit log screen** (Settings → Audit): global filterable log (user, entity, action, date). Export. Admin/auditor only.
- **Integrity check** page: run "Verify ledger" (hash chain, balances, subledger vs control accounts) with a green/red result and details.

### 6.4 Approvals (SB+)
- **Approvals inbox:** list of items waiting for me, with document preview, amount, requester, rule that triggered it, history. Approve / Reject (reason required) / Request changes. Keyboard: `A` approve, `R` reject.
- Documents in approval show a stepper (Requested → Manager → Finance → Posted).
- **Rules editor** (M): condition builder (document type, amount ≥, account, project, party), approver (role/user/manager), steps, SoD check, escalation after n days.

### 6.5 Backups & restore
- Settings → Backup: automatic backups (frequency, keep N, folder via Tauri dialog), last backup status, "Back up now", "Restore…" (choose file → preview of company, date, version → restore into a new company or replace with typed confirmation). Encryption password option. For server mode: shows the server's backup status (admin) and "Download company export".
- A reminder banner appears if no backup was made in 7 days (Solo-local).

### 6.6 Multi-company
- Company switcher (§3). Settings → Companies: list, create (from scratch / copy settings and COA from an existing company), archive.
- Group view (E): consolidation group tree, ownership %, intercompany mapping, consolidation run with elimination preview.
- Cross-company search is permission-gated and shows a company badge on each result.

---

## 7. Sharing the books

### 7.1 Invite accountant/auditor & roles
- Settings → Team: members list (name, role, companies, last active, 2FA ✓), **Invite** dialog (email or phone, role, companies, expiry date for external roles, message). Pending invites can be resent or revoked.
- **Roles screen:** predefined roles (Owner, Admin, Accountant, Bookkeeper, Sales, Purchasing, Storekeeper, Cashier, Payroll officer, Auditor (read-only), External accountant). A **permission matrix** (resources × actions view/create/edit/post/approve/delete-draft/export) with scope (all / branch / own) for custom roles (M). There is a "Preview as role" mode.
- Auditor role: read-only to everything incl. audit log, can comment, can export; banner "Auditor view — read only".

### 7.2 Read-only links
- On reports and statements: **Share → Create link**: expiry (1/7/30 days), password optional, allow download PDF/Excel, watermark with recipient name. The link list shows views and lets you revoke. Server/sync required; disabled in Solo-local with explanation.

### 7.3 Export
- Every list and report: Excel (.xlsx, RTL sheet direction for fa, numbers as numbers, Jalali dates as text + hidden Gregorian column), PDF (print template), CSV (UTF-8 BOM for Excel compatibility). Full-company export in Settings → Data.

### 7.4 Share invoices with customers
- **Share link** for invoices/quotes/statements: public page (web) showing the invoice in fa/en with download PDF, payment instructions (bank accounts/Sheba, card number), and an online-pay button when a generic payment gateway adapter is configured. Send via SMS/email/copy link/QR code. Track "viewed" events.
- **Customer portal** (SB+): customers log in by phone OTP to see invoices, statements, payments, cheques.

### 7.5 Accountant portal
- For external accountants managing many clients: a **client list** (companies across organisations they're invited to) with health indicators (unreconciled lines, Moadian failures, days since close, overdue tasks), bulk open, and per-client notes. Uses the same app; just a dedicated landing page when the user has the External accountant role in ≥ 2 companies.

### 7.6 Comments & mentions
- Comment thread on any record (end-side panel, `Ctrl+Shift+M`): rich text (bold, list, link), `@mention` users (notifies), attach files, **resolve** threads, mark a comment as "question for accountant". Comments are not part of the legal record but are kept in the audit log.
- Line-level comments on journal lines and reconciliation lines (icon in the row).

### 7.7 Activity log
- Company-wide **Activity feed** (sidebar → Activity): a human-readable stream ("Sara posted invoice 1405-0231 for 125,000,000 ریال", "Moadian accepted invoice …") with filters by user/module and links.

---

## 8. Design system

### 8.1 Tokens (CSS variables, Tailwind v4 `@theme`)
Use shadcn's variable convention (`--background`, `--foreground`, `--primary`, `--muted`, `--accent`, `--destructive`, `--border`, `--ring`, `--chart-1..5`, `--sidebar-*`) in OKLCH, plus domain tokens:

| Token | Purpose | Light (example) | Dark (example) |
|---|---|---|---|
| `--primary` | Brand accent (teal-blue) | `oklch(0.55 0.12 230)` | `oklch(0.72 0.11 230)` |
| `--success` | Posted, reconciled, accepted | `oklch(0.62 0.14 150)` | `oklch(0.72 0.13 150)` |
| `--warning` | Due soon, provisional, pending | `oklch(0.78 0.15 80)` | `oklch(0.80 0.14 80)` |
| `--destructive` | Errors, overdue, bounced | `oklch(0.58 0.20 27)` | `oklch(0.68 0.18 27)` |
| `--info` | Draft, informational | `oklch(0.65 0.10 250)` | `oklch(0.75 0.09 250)` |
| `--amount-negative` | Negative numbers (parentheses + this colour, optional) | muted red | muted red |
| `--row-hover`, `--row-selected`, `--cell-focus` | Grid states | | |
| `--radius` | 0.5rem (compact: 0.375rem) | | |
| Spacing scale | 4px base; density multiplier `--density: 1 / 0.85 / 0.75` | | |

Rules: debit/credit columns use **neutral** text colour. Colour is for status only. Every colour pair meets WCAG AA contrast (checked in Storybook with axe).

### 8.2 Typography (Persian)
- **Vazirmatn** variable (weights 100–900), self-hosted woff2 subset. Base size 14px (comfortable) / 13px (compact). Line-height 1.6 for Persian text, 1.4 in grids.
- Numbers: `font-variant-numeric: tabular-nums` everywhere amounts appear.
- English uses the same Vazirmatn Latin glyphs for visual consistency. Optionally Inter for `en` (a token switch).
- Text normalisation on input: Arabic ي/ك → Persian ی/ک, trim ZWNJ duplicates, keep ZWNJ (نیم‌فاصله) intact. Provide `Shift+Space` → ZWNJ helper in text inputs.
- Headings scale: 24/20/16/14. No all-caps. Avoid italics in Persian.

### 8.3 Digits, numbers and money
- `lib/format.ts` exposes `formatAmount(decimalString, {currency, unit: 'rial'|'toman', digits: 'fa'|'latn', fraction})`, `formatNumber`, `formatPercent`, `parseAmount(input)`.
- Implemented with `Intl.NumberFormat('fa-IR', { numberingSystem: 'arabext' })` for Persian digits (۰-۹) **or** `latn`. Grouping separator follows locale (`٬` in fa-IR) or user override `,`. **Never format via JS `number` for amounts.** Format decimal strings by splitting integer/fraction and grouping manually or via BigInt for the integer part.
- Negative amounts: `(۱۲٬۵۰۰)` in reports; `−۱۲٬۵۰۰` in inputs. Use LRM/RLM or `<bdi>`/`dir="ltr"` on number spans to prevent bidi reordering of signs, separators and currency codes.
- Currency display: `۱۲٬۵۰۰٬۰۰۰ ریال` or `۱٬۲۵۰٬۰۰۰ تومان`; in en `IRR 12,500,000`. FX: `USD 1,250.00`.
- **Money input parsing:** accepts Persian/Arabic-Indic/Latin digits, separators, `k`/`m`/`b` and Persian `هزار/میلیون/میلیارد` suffixes, simple arithmetic (`=1200*3`), paste of formatted numbers. Shows the formatted value and amount in words (Persian) on focus.
- Amount in words: `numberToPersianWords(decimalString)` with tests up to 10^18.

### 8.4 Jalali date picker
- `JalaliDatePicker` built on shadcn `Popover` + a custom calendar grid (shadcn's `Calendar`/react-day-picker supports custom calendars `⚠ verify` Persian support; fallback: own grid). Features: month/year dropdowns with Persian month names (فروردین…اسفند), week starting **Saturday**, Friday highlighted as weekend, holidays from the data pack marked, today button, Gregorian secondary labels (small) toggle.
- **Typed entry**: `1405/08/12`, `۱۴۰۵/۸/۱۲`, `8/12` (current year), `12` (current month), `t`/`امروز` today, `+3`/`-1` relative days, `Ctrl+↑/↓` day ±1 in field. Invalid dates are flagged inline.
- Range picker with presets: today, yesterday, this week, this month, last month, this season, last season, this FY, last FY, custom.
- Display format setting: `1405/08/12` vs `۱۲ آبان ۱۴۰۵`. Tooltip shows Gregorian.

### 8.5 Dark mode & density
- Theme: light/dark/system (`class="dark"` strategy). Charts and status colours have tuned dark variants. PDF/print always light.
- Density: Comfortable (default for Solo), Compact (default for Accountant role), Spacious. Affects row height (40/32/48px), paddings, and font size.

### 8.6 Iconography & motion
- lucide icons; directional icons flip in RTL (`rtl:rotate-180`) *except* those with fixed meaning (media play, clocks, charts). Motion: 150–200 ms, respect `prefers-reduced-motion`. Use logical slide animations (`slide-in-from-end`) and pass `dir` to portalled components (known tw-animate-css issue noted in shadcn RTL docs).

---

## 9. Component inventory

### 9.1 shadcn/ui components used
`accordion, alert, alert-dialog, avatar, badge, breadcrumb, button, calendar (base for Jalali), card, chart, checkbox, collapsible, command, context-menu, dialog, direction, drawer, dropdown-menu, form, hover-card, input, input-otp, label, menubar, navigation-menu, pagination, popover, progress, radio-group, resizable, scroll-area, select, separator, sheet, sidebar, skeleton, slider, sonner (toasts), switch, table, tabs, textarea, toggle, toggle-group, tooltip`. Install all with `rtl: true` and audit `calendar`, `pagination` and `sidebar` manually as the shadcn RTL docs recommend.

### 9.2 Custom components

| Component | Responsibility | Key behaviour |
|---|---|---|
| `EntryGrid` | Keyboard data-entry grid for journal lines, invoice lines, attendance, opening balances, stock counts | Cell editors per column type; Enter/Tab/Shift+Tab navigation (direction-aware); `Ins`/`Ctrl+Enter` new row; `Ctrl+D` duplicate row above; `Ctrl+Del` delete row; `=` fill balancing amount; F2 edit; Esc cancel cell; copy/paste ranges from Excel (TSV); per-row validation with error icons; virtualised for 1,000+ lines; footer totals; undo/redo (Ctrl+Z/Y) within the form |
| `MoneyInput` | Amount entry | Parsing rules §8.3; Rial/Toman toggle; FX hint; amount-in-words on focus; never emits number, emits decimal string |
| `AccountPicker` | Choose a postable account | Search by code prefix ("1101") or name, tree context (group › kol › moein), recent/favourites, shows tafsili requirement icons, disabled non-postable accounts with reason, create-inline (permission) |
| `TafsiliPicker` | Choose tafsili per level | Filtered by the account's allowed tafsili types at that level; search by name/code/national ID/phone; shows balance; create-inline party |
| `PartyPicker` | Customer/supplier | Search name/phone/national ID/economic code; shows balance, credit status, Moadian completeness |
| `ItemPicker` | Goods/services | Search name/code/barcode; scanner input; on-hand per warehouse; last price for this party |
| `JalaliDatePicker`, `JalaliRangePicker` | Dates | §8.4 |
| `AmountCell`, `AmountText` | Display amounts | Tabular, bidi-safe, negative style, unit, copy raw value on click |
| `StatusBadge` | Document/cheque/Moadian states | Colour + icon + label; tooltip with next allowed actions |
| `StateTimeline` | Lifecycle history (cheques, Moadian, approvals) | Steps with timestamps (Jalali), actor, linked journal entries |
| `DataTable` | Lists | TanStack Table + Virtual; server pagination; saved views; column menu; totals row; bulk bar |
| `TreeTable` | COA, reports | Expand levels, keyboard tree navigation, level toggles (kol/moein/tafsili) |
| `FilterBar` | Filters | URL-synced, chips, presets |
| `ReportViewer` | Reports | Params, tree-table, drill-down breadcrumb, export |
| `BalanceBar` | Δ debit/credit | Sticky footer; green when balanced; click jumps to first unbalanced line |
| `DocumentHeader` | Detail headers | Number, status, actions with disabled reasons |
| `ActionGuard` | Permission/state guard | Renders children disabled with reason tooltip from API `allowed_actions` |
| `CommentsPanel` | Comments & mentions | Threads, mentions autocomplete, resolve |
| `ActivityList` | Audit/activity | Diffs with formatted values |
| `ImportWizard` | Excel/other imports | Upload → mapping → validation grid → run → summary |
| `ChequeFields` | Cheque sub-form | Sayad ID mask 4-4-4-4 with checksum/length validation, bank picker, due date, images |
| `ShebaInput`, `NationalIdInput`, `PostalCodeInput` | IDs | Masks, live checksum, bank detection from Sheba |
| `PrintPreview` | PDF preview | Renders server-generated PDF (Typst) in-app; print via Tauri/OS |
| `SyncStatus`, `SyncInbox` | Sync UX | §11.1 |
| `EmptyState`, `ErrorState`, `PageSkeleton` | States | §11.2 |
| `ShortcutHint`, `ShortcutsDialog` | Keyboard | Shows platform-specific keys (⌘ vs Ctrl) |
| `CompanySwitcher`, `FiscalYearSelect` | Shell | §3 |
| `AmountInWords` | Persian words | Rial/Toman |
| `FileDrop` | Attachments | Drag-drop, paste screenshot, camera on web mobile |

---

## 10. State & data layer

### 10.1 Layers
```
UI components ──▶ feature hooks (useInvoices, usePostInvoice) ──▶ TanStack Query ──▶ api client (typed)
                                                                                     │
                                                         Transport interface ◀──────┘
                                              ┌───────────────┴───────────────┐
                                     TauriTransport (invoke)          HttpTransport (fetch)
                                     bindings.ts (tauri-specta)        openapi client (utoipa)
```
- `src/lib/transport.ts` defines `interface Transport { query<T>(op, input): Promise<T>; command<T>(op, input, opts?: {idempotencyKey}): Promise<T>; subscribe(event, cb): Unsubscribe }`. Selected at boot: Tauri when `window.__TAURI_INTERNALS__` is present and the company is in local mode; HTTP otherwise (web build, team mode).
- **Operation names are shared** (`sales.invoices.list`, `sales.invoices.post`…). A generated map ties each op to a Tauri command and an HTTP route. A CI contract test asserts both transports return identical DTOs for fixtures.
- Errors: both transports throw `ApiError {code, message (localized), fieldErrors[], allowedActions?}`; one `toast`/form mapper.

### 10.2 TanStack Query conventions
- Query keys: `[companyId, module, entity, params]`, e.g. `[cid, 'sales', 'invoices', {status:'unpaid', page}]`. The factory lives in `features/<module>/queries.ts`.
- `staleTime` 30 s for lists, 5 min for reference data (COA, tax codes, currencies). Prefetch detail on row hover/focus.
- **Invalidation by domain events:** the backend emits events (`InvoicePosted`, `PaymentAllocated`…) through Tauri events or SSE/WebSocket (server). `lib/events.ts` maps events to query keys to invalidate. No manual "refresh" button needed (still offered).
- Mutations: optimistic only for drafts (autosave). Posting waits for the server and then invalidates.
- Pagination: `useInfiniteQuery` with cursors for virtualised lists.

### 10.3 TanStack Router
- File-based routes under `src/routes/`: `/$companyId/sales/invoices`, `/$companyId/sales/invoices/$id`, `/$companyId/sales/invoices/new`, etc. The company id is in the URL so deep links and multi-window (desktop) work.
- Typed `validateSearch` with zod for filters, so URLs are the state of list screens.
- Route `loader`s prefetch queries. `pendingComponent` = skeleton. `errorComponent` = `ErrorState`.
- Permission guards in `beforeLoad` (redirect to "No access" page with the role needed).

### 10.4 TanStack Table
- Column definitions per entity in `features/<module>/columns.tsx`, with meta: `{align:'amount', total:true, format:'money', width}`. Server-side sorting/filtering via `manualSorting`/`manualFiltering`. Column visibility/order persisted per saved view.

### 10.5 Forms: react-hook-form + zod
- Schemas in `features/<module>/schemas.ts`, derived from generated DTO types (zod schemas generated from OpenAPI where possible, e.g. `openapi-zod-client` `⚠ verify` tool choice, plus hand-written refinements).
- Money fields are `z.string().regex(decimalRegex)`; dates are ISO strings `YYYY-MM-DD` (Gregorian) internally, converted only by the date picker.
- `useFieldArray` for lines, connected to `EntryGrid`. Validation on blur for cells and on submit for the document. Server errors map with `setError`.
- Draft autosave: `watch` + debounce 1.5 s → `saveDraft` mutation with `version` (ETag) to detect concurrent edits.

### 10.6 Tauri bridge specifics
- Commands generated by tauri-specta into `src/bindings.ts`, never hand-written. Long tasks (imports, payroll calc, backups) return a job id. Progress comes via Tauri events (`job://progress`).
- File I/O only through Tauri dialog/fs plugins with scoped permissions (capabilities file). The web build uses `<input type=file>` and downloads.
- Desktop-only features (native notifications, auto-update, deep links, OS keychain) live behind `platform.ts` capability checks so the web build degrades gracefully.

### 10.7 Local UI state
- Zustand (or React context) only for UI-ephemeral state: sidebar collapsed, density, command palette open, current company. Persist preferences to Tauri `store` plugin / `localStorage` on web. Server data never goes in client stores.

---

## 11. Offline & sync UX, states, accessibility

### 11.1 Offline & sync UX
- **Indicator** in the top bar (§3). Offline banner (dismissible): "You're offline. Changes are saved on this computer and will sync when you reconnect."
- **Provisional postings:** documents posted offline show a `Provisional` amber badge, a `P-…` number, and the tooltip "Will receive its final number when synced". Reports have an "Include provisional" toggle (on by default with a footnote).
- **Sync panel** (sheet): last sync time, queued commands (count + list), progress, errors. **Sync inbox** for conflicts: side-by-side "Server / This device" values per field, choose per field or "Keep server" / "Re-apply mine". Rejected postings show the reason (e.g. "Period 1405/07 was locked by Sara at 14:02") and actions (change date, request override, discard).
- Number-block exhaustion warning ("40 invoice numbers left offline").
- Moadian actions are disabled offline with an explanation (desktop Solo-local can submit when online).
- Never block typing: all forms work offline in team mode as drafts.

### 11.2 Empty, loading and error states
- **Empty:** illustration (simple line art), one sentence, primary CTA ("Create your first invoice"), secondary ("Import from Excel", "Load sample data"). Filtered-empty differs: "No results for these filters" + "Clear filters".
- **Loading:** skeletons matching layout (`PageSkeleton`, table row skeletons). Never spinners for > 300 ms content. Long jobs use a progress bar with cancel.
- **Errors:** inline field errors (fa/en). Form-level `Alert` summarising errors with links to fields. Page-level `ErrorState` with error code, "Retry", "Copy details", "Report issue" (creates a local diagnostic bundle, does **not** send anything automatically). Network errors trigger an automatic retry with backoff and an offline transition.
- **Permission:** "You don't have permission to post invoices. Ask an Admin (role needed: Accountant)."
- **Locked:** a lock icon plus the reason and override path.

### 11.3 Accessibility (WCAG 2.2 AA)
- All interactive elements are reachable by keyboard, with a visible focus ring (`--ring`, 2px) and logical tab order in RTL.
- `lang` and `dir` set on `<html>`. Mixed-direction content wrapped in `<bdi>`. Numbers are read correctly by screen readers (`aria-label` with Latin digits where Persian digits confuse readers `⚠ verify` with NVDA/VoiceOver fa).
- Grid follows the ARIA grid pattern (`role="grid"`, `aria-rowindex`, `aria-colindex`), with live region announcements for balance changes ("Difference 0, entry balanced").
- Colour is never the only signal (icons + text on statuses). Minimum target size 24×24. Reduced motion respected. Text resizes to 200 % without loss.
- axe checks in Storybook and Playwright for every screen in fa and en.

---

## 12. Keyboard shortcut map

Platform: `Mod` = Ctrl (Windows/Linux) or ⌘ (macOS). Arrow keys that mean "next/previous" are **direction-aware** (in RTL, ← means "forward").

| Scope | Shortcut | Action |
|---|---|---|
| Global | `Mod+K` | Command palette |
| Global | `/` or `Mod+Shift+F` | Global search |
| Global | `N` then `I/B/R/P/J/C/Q/E` | New invoice / bill / receipt / payment / journal / customer / cheque / expense |
| Global | `G` then `D/S/P/B/A/R/T` | Go to dashboard / sales / purchases / banking / accounting / reports / tax |
| Global | `Mod+B` | Toggle sidebar |
| Global | `Mod+Shift+L` | Toggle language fa/en |
| Global | `Mod+Shift+D` | Toggle dark mode |
| Global | `?` | Shortcuts dialog |
| Global | `Mod+,` | Settings |
| Global | `Alt+1..9` | Switch to pinned company 1..9 |
| Lists | `↑/↓`, `Enter`, `Space`, `x` | Move, open, preview, select |
| Lists | `J/K` | Next/previous record (detail view) |
| Lists | `F` | Focus filter bar; `Mod+S` save view |
| Lists | `E` | Export menu |
| Forms | `Mod+S` | Save draft |
| Forms | `Mod+Enter` | Post |
| Forms | `Mod+Shift+Enter` | Post & new |
| Forms | `Mod+P` | Print/PDF |
| Forms | `Esc` | Close/cancel (confirm if dirty) |
| Grid | `Enter` / `Tab` / `Shift+Tab` | Next cell / next / previous |
| Grid | `Ins` or `Mod+Enter` in last row | New line |
| Grid | `Mod+D` | Duplicate line above |
| Grid | `Mod+Delete` | Delete line |
| Grid | `=` (in amount cell) | Fill balancing amount |
| Grid | `F2` | Edit cell |
| Grid | `Mod+Z` / `Mod+Y` | Undo / redo |
| Grid | `Mod+V` | Paste range from Excel |
| Dates | `t`, `+n`, `-n`, `Mod+↑/↓` | Today, relative, ±1 day |
| Reconcile | `Enter`, `M`, `C`, `T`, `S`, `X` | Accept, match, create, transfer, split, exclude |
| Approvals | `A`, `R` | Approve, reject |
| Comments | `Mod+Shift+M` | Toggle comments panel |
| Reports | `+`/`-`, `Backspace` | Expand/collapse level, drill back |

---

## 13. Folder structure (`apps/web`)

```
apps/web/
├─ index.html                     # <html lang="fa" dir="rtl"> (set at runtime from prefs)
├─ components.json                # shadcn config: "rtl": true, aliases
├─ vite.config.ts
├─ src/
│  ├─ main.tsx                    # providers: Direction, Theme, I18n, QueryClient, Router, Transport
│  ├─ bindings.ts                 # GENERATED by tauri-specta (do not edit)
│  ├─ api/
│  │  ├─ openapi.d.ts             # GENERATED from utoipa spec
│  │  ├─ client.ts                # op → transport mapping
│  │  └─ errors.ts                # ApiError, problem+json mapping
│  ├─ lib/
│  │  ├─ transport.ts             # Transport interface, Tauri + HTTP impls
│  │  ├─ money.ts                 # decimal wrapper, arithmetic, allocation
│  │  ├─ format.ts                # amounts, numbers, digits, bidi helpers
│  │  ├─ jalali.ts                # conversions, parsing, presets (golden-tested vs Rust)
│  │  ├─ words-fa.ts              # amount in Persian words
│  │  ├─ normalize-fa.ts          # ی/ک/ZWNJ/digit normalisation for search and input
│  │  ├─ validators.ts            # national ID, legal ID, Sheba, postal code, Sayad ID
│  │  ├─ events.ts                # domain event → query invalidation map
│  │  ├─ platform.ts              # desktop/web capability checks
│  │  └─ shortcuts.ts             # central shortcut registry (direction-aware)
│  ├─ i18n/
│  │  ├─ index.ts
│  │  └─ locales/{fa,en}/{common,sales,purchases,banking,cheques,inventory,payroll,accounting,tax,reports,settings,errors}.json
│  ├─ styles/
│  │  ├─ globals.css              # tailwind, tokens, Vazirmatn @font-face, density vars
│  │  └─ print.css
│  ├─ components/
│  │  ├─ ui/                      # shadcn generated components (RTL-transformed)
│  │  ├─ app-shell/               # Sidebar, TopBar, CommandPalette, GlobalSearch, QuickCreate, Notifications, CompanySwitcher, SyncStatus
│  │  ├─ data/                    # DataTable, TreeTable, FilterBar, EntryGrid, ReportViewer, BalanceBar
│  │  ├─ inputs/                  # MoneyInput, JalaliDatePicker, AccountPicker, TafsiliPicker, PartyPicker, ItemPicker, ShebaInput, …
│  │  ├─ display/                 # AmountText, StatusBadge, StateTimeline, AmountInWords
│  │  ├─ feedback/                # EmptyState, ErrorState, PageSkeleton
│  │  └─ collab/                  # CommentsPanel, ActivityList, ShareDialog
│  ├─ features/                   # one folder per module: queries.ts, schemas.ts, columns.tsx, components/, hooks/
│  │  ├─ onboarding/  dashboard/  sales/  purchases/  inventory/  banking/  cheques/
│  │  ├─ payroll/  assets/  accounting/  tax/  reports/  approvals/  team/  settings/  sync/  import/
│  ├─ routes/                     # TanStack Router file routes (/$companyId/…)
│  └─ test/                       # test utils, msw handlers, fixtures (shared with Rust goldens)
├─ e2e/                           # Playwright specs (web) + wdio/tauri-driver specs (desktop)
└─ .storybook/                    # stories for every component in fa/en × light/dark × densities
```

Rules: features import from `components/` and `lib/`, never from other features (shared hooks move to `lib/`). No default exports except routes. One component per file. Barrel files are avoided.

---

## 14. UI features checklist

Milestone tags match ACCOUNTING-SPEC.md §7: **M0** foundation · **M1** Solo MVP · **M2** Small business · **M3** Mid · **M4** Enterprise.

### 14.1 Foundation
- [ ] M0 Vite + React + TS strict; Tailwind v4; shadcn init with `rtl: true`; `DirectionProvider`; Vazirmatn self-hosted
- [ ] M0 Theme tokens (light/dark), density variables, print stylesheet
- [ ] M0 i18n setup (fa default, en), runtime language switch flips `dir` without reload
- [ ] M0 `Transport` interface + Tauri and HTTP implementations + contract test
- [ ] M0 Generated `bindings.ts` (tauri-specta) and OpenAPI types in CI; build fails if stale
- [ ] M0 `lib/money`, `lib/format`, `lib/jalali`, `words-fa`, `normalize-fa`, `validators` with unit tests and shared golden fixtures with Rust
- [ ] M0 Storybook with fa/en × light/dark decorators; axe checks
- [ ] M0 Playwright + tauri-driver harness; CI runs both

### 14.2 App shell
- [ ] M1 Sidebar (collapsible, tier/permission-aware, badges, favourites)
- [ ] M1 Top bar: company switcher, FY selector, search, quick-create, notifications, sync indicator, user menu
- [ ] M1 Command palette with navigation, create, record actions, Persian-normalised fuzzy search
- [ ] M1 Global search with grouped results and preview
- [ ] M1 Shortcut registry + `?` dialog; direction-aware arrows
- [ ] M1 Window state, single instance, deep links (desktop)
- [ ] M2 Native notifications (opt-in); multi-window per company (desktop)

### 14.3 Onboarding
- [ ] M1 Steps 0–10 and 12–13 of §2 with persistence and resume
- [ ] M1 COA template preview tree; opening balances grid with difference bar
- [ ] M1 Excel import wizard with mapping memory and validation grid
- [ ] M2 Invite team step; Hesabfa/Sepidar/Holoo import sources
- [ ] M1 Sample data toggle + one-click removal

### 14.4 Core components
- [ ] M1 `EntryGrid` with all keyboard behaviours of §9.2, Excel paste, undo/redo, 1,000-line virtualisation
- [ ] M1 `MoneyInput` (multi-digit-system parsing, suffixes, arithmetic, words) — never emits number
- [ ] M1 `JalaliDatePicker` / range picker with typed entry, presets, holidays, Saturday week start
- [ ] M1 `AccountPicker`, `TafsiliPicker` (level rules), `PartyPicker`, `ItemPicker` (barcode)
- [ ] M1 `DataTable` (server pagination, totals, saved views, bulk actions), `TreeTable`, `FilterBar` (URL-synced)
- [ ] M1 `StatusBadge`, `StateTimeline`, `ActionGuard` (disabled reasons), `BalanceBar`, `AmountText` (bidi-safe)
- [ ] M1 `EmptyState`, `ErrorState`, `PageSkeleton`; toasts
- [ ] M1 ID inputs: national ID, legal ID, postal code, Sheba (bank detection), Sayad ID

### 14.5 Dashboard
- [ ] M1 Cash, AR, AP, cheques, sales, expenses, Moadian health, reconciliation cards with drill links
- [ ] M2 Inventory alerts, payroll, approvals cards; card customisation persisted
- [ ] M3 Cash-flow forecast card

### 14.6 Sales & purchases
- [ ] M1 Invoice list/form/detail incl. Moadian tab, print preview, share link, receive payment
- [ ] M1 Quote → invoice; receipts with mixed methods and allocation grid
- [ ] M1 Bills, quick expense with attachment, payments with cheque issue
- [ ] M1 Customer/supplier detail with statement export/share
- [ ] M1 Corrective / cancel / return wizards with rule explanations and diff
- [ ] M2 Sales/purchase orders, deliveries, GRN, 3-way match indicator, recurring invoices, reminders
- [ ] M2 Incoming Moadian invoices inbox (accept/reject, create bill)
- [ ] M2 OCR prefill with confidence highlighting
- [ ] M3 Discount approvals UI, landed cost UI, requisitions

### 14.7 Inventory
- [ ] M1 Items list/form/detail with stock card
- [ ] M2 Warehouses, transfers, adjustments, stock count with scanner mode and variance review
- [ ] M3 Batches/serials, bins, multi-UoM entry

### 14.8 Banking & cheques
- [ ] M1 Accounts overview, register, statement import with mapper
- [ ] M1 Reconciliation workspace with suggestions, rules, keyboard actions, finish & lock, report
- [ ] M1 Cheque lists (received/issued), detail with lifecycle timeline, next-action buttons, calendar view
- [ ] M2 Cheque batch actions, cheque books leaf grid, bounced workflow, Sayad helper checklist
- [ ] M2 Cash count, POS settlement screens
- [ ] M3 Auto-reconcile review queue, cash-flow forecast view

### 14.9 Accounting, tax, reports
- [ ] M1 Journal list/form with balance bar and templates; COA tree editor; tafsili lists
- [ ] M1 Moadian queue with deadlines, bulk send/retry, error explainer, JSON drawer; settings with test connection
- [ ] M1 VAT worksheet
- [ ] M1 Report gallery + viewer with drill-down chain and exports; trial balance column modes
- [ ] M2 Seasonal report screen + TTMS export
- [ ] M2 Saved/scheduled reports
- [ ] M3 Report builder (pivot) and statement designer

### 14.10 Payroll & assets
- [ ] M2 Employees list/form; payroll run wizard; outputs page (payslips, bank file, DISK, tax list)
- [ ] M2 Law data review screen with ⚠ verify badges
- [ ] M2 Fixed assets register, depreciation run preview/post, disposal

### 14.11 Managing the books
- [ ] M1 Periods timeline, lock dialog, year-end close wizard
- [ ] M1 Amend flow (reverse + new draft) with diff highlight
- [ ] M1 Activity tab on every record; audit log screen; integrity check page
- [ ] M1 Backup settings, backup now, restore preview, 7-day reminder
- [ ] M2 Approvals inbox and document approval stepper
- [ ] M3 Close checklist with owners/due dates; approval rules editor
- [ ] M2 Multi-company management; M4 consolidation group screens

### 14.12 Sharing the books
- [ ] M2 Team screen, invite dialog, roles list; auditor read-only banner
- [ ] M3 Custom role permission matrix + "preview as role"
- [ ] M2 Read-only share links with expiry/password/watermark and revoke
- [ ] M1 Excel/PDF/CSV export on every list and report (RTL sheets, BOM CSV)
- [ ] M2 Public invoice page with payment instructions + generic gateway button; view tracking
- [ ] M2 Customer portal (phone OTP)
- [ ] M2 Accountant portal client list with health indicators
- [ ] M2 Comments with mentions, resolve, line-level comments; activity feed

### 14.13 Offline, states, accessibility
- [ ] M2 Sync indicator, offline banner, provisional badges, sync panel, sync inbox with field-level conflict resolution, rejected-posting actions, number-block warnings
- [ ] M1 Empty/loading/error/permission/locked states on every screen
- [ ] M1 WCAG 2.2 AA: keyboard, focus, ARIA grid, `<bdi>` numbers, axe clean in fa/en
- [ ] M1 Visual regression snapshots for key screens in fa/en × light/dark

### 14.14 Web build
- [ ] M2 Same bundle served by `apps/server` at `/app`; desktop-only features degrade gracefully
- [ ] M2 Responsive layouts down to tablet (1024px). Phone layouts for invoice view, approvals and dashboard
- [ ] M3 PWA install (optional), auth via server session/OIDC

---

## 15. Frontend-only Claude Code prompt

````text
You are building the frontend of "Daftar", an open-source Iranian accounting app. Read docs/FRONTEND-UI-GUIDE.md (this guide) and docs/ACCOUNTING-SPEC.md §4.3 (transport) and §6 before coding. Work only in apps/web (and apps/desktop/src-tauri capability/config files if needed). Do not modify Rust domain crates; if you need an API change, write it in docs/api-requests.md and use a mock meanwhile.

FIXED STACK (do not substitute): React + TypeScript (strict) + Vite; shadcn/ui with "rtl": true in components.json and DirectionProvider; Tailwind v4; TanStack Router (file routes, zod-validated search params), TanStack Query, TanStack Table + Virtual; react-hook-form + zod; cmdk via shadcn Command; i18next (fa default, en); Vazirmatn self-hosted; decimal.js or big.js wrapped in src/lib/money.ts; Tauri v2 APIs via generated src/bindings.ts (tauri-specta); HTTP via generated OpenAPI types + openapi-fetch; Vitest + Testing Library; Storybook; Playwright (web) and tauri-driver (desktop) for e2e; MSW for mocks.

HARD RULES:
1. RTL-first: build every screen in fa/RTL first and verify in en/LTR. Only logical utilities (ms/me/ps/pe/start/end/text-start). Directional icons flip with rtl:rotate-180. Pass dir to portalled components.
2. Money is never a JS number. Amounts are decimal strings end-to-end. All formatting goes through lib/format.ts (Persian digits default in fa, bidi-safe with <bdi>). MoneyInput emits strings only. Add a lint rule or test that forbids parseFloat/Number() on money fields.
3. Dates: ISO Gregorian strings internally. Jalali for display/input via lib/jalali.ts only. The golden fixtures in src/test/fixtures/jalali.json must match the Rust table.
4. All user-facing strings go through i18n with keys in both fa and en. CI fails on missing keys.
5. Transport abstraction: components never call invoke/fetch directly. Use feature hooks → api client → Transport.
6. Every action that the backend may refuse renders through ActionGuard and shows the localized reason.
7. Keyboard-first: implement the shortcut map of §12 via lib/shortcuts.ts. Every screen is fully operable without a mouse.
8. Accessibility WCAG 2.2 AA; axe has zero violations in Storybook and Playwright for fa and en.
9. Follow the folder structure of §13 exactly. Features don't import from other features.

MILESTONES (tick items in §14 of this guide when merged with passing tests; write docs/milestones/ui-Mx.md at the end of each):
- UI-M0 Foundation: §14.1 complete. DoD: the app boots in Tauri and in a browser, toggles fa/en and light/dark at runtime, Storybook shows all shadcn components in RTL, and lib/* unit tests pass, including the shared golden fixtures.
- UI-M1 Solo MVP: §14.2 (M1), 14.3 (M1), 14.4, 14.5 (M1), 14.6 (M1), 14.7 (M1), 14.8 (M1), 14.9 (M1), 14.11 (M1), 14.12 (M1 items), 14.13 (M1). DoD: Playwright + tauri-driver journeys pass for: onboarding with the sample company; create & post an invoice entirely by keyboard in under 60 s of scripted input; receive a payment by Sayad cheque and clear it; import a bank statement and reconcile 20 lines; post a manual journal with tafsili; run the trial balance and drill to the source document; year-end close; backup and restore. Visual snapshots exist for all M1 screens in fa/en × light/dark.
- UI-M2 Small business: all M2 items. DoD: two clients against the server show live invalidation via events. Offline mode creates provisional postings that resolve after reconnect (e2e with network toggling). The conflict inbox resolves a field conflict. Payroll run wizard e2e. Share link and public invoice page e2e on the web build.
- UI-M3 Mid-market: all M3 items. DoD: report builder pivot e2e, approval rules editor e2e, and auto-reconcile review queue e2e. Lists stay at 60 fps scrolling with 100k rows (performance test).
- UI-M4 Enterprise: consolidation screens and SSO login flow. DoD: e2e for group consolidation run and elimination preview.

TESTING: unit tests for every lib function, component tests for every custom component (keyboard behaviour especially for EntryGrid, MoneyInput, JalaliDatePicker, pickers), Storybook stories with interaction tests, Playwright for web journeys, tauri-driver for desktop journeys, axe in both.

CONVENTIONS: Conventional Commits; small PRs per checklist item; named exports; one component per file; colocated tests (*.test.tsx) and stories (*.stories.tsx); query keys from per-feature factories; no `any`; no default exports except routes.

Start with UI-M0. At the end of every milestone print the ticked/remaining counts of §14.
````
