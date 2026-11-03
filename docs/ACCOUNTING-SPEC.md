# Daftar — Open-Source Accounting for Iran and Beyond
### Product & Engineering Specification (v0.1, 1405-08-12 / 2026-11-03)

> Working name: **Daftar** (دفتر, "the ledger"). Rename freely.
> Fixed stack: **Rust backend** (Cargo workspace, modular monolith) + **Tauri v2 desktop app with React + shadcn/ui (Tailwind)**. The same React UI must later run as a plain web app against the same backend.
> Companion document: **[FRONTEND-UI-GUIDE.md](./FRONTEND-UI-GUIDE.md)**: the screen-by-screen UI build guide, design system, component inventory and a frontend-only build prompt.

**How to read this file**

| § | Section | Use it for |
|---|---|---|
| 1 | Executive summary | Why this exists, what to copy from winners, what gaps to exploit, design principles |
| 2 | Competitor research (global and Iran) | Comparison tables, loved features, pain points, all cited |
| 3 | Iran-specific requirements (verified) | VAT, Moadian, Sayad, payroll, COA, calendar, money |
| 4 | Architecture | Workspace, crates, data model, ledger invariants, sync, API, Tauri bridge |
| 5 | Module specification with scale tiers | Every module, with the tier (S / SB / M / E) that unlocks each feature |
| 6 | Frontend / UX summary | Patterns, libraries (details in FRONTEND-UI-GUIDE.md) |
| 7 | Master checklist | `- [ ]` items, concrete and testable, grouped by module |
| 8 | Claude Code build prompt | Copy-paste prompt to build it milestone by milestone |
| 9 | Sources | All links |

**Legend.** Tiers: **S** = Solo (one person, offline-capable, SQLite), **SB** = Small business (2–20 users, server or LAN), **M** = Mid-market (20–200 users, approvals, multi-warehouse, multi-currency), **E** = Enterprise (multi-company, consolidation, SSO, HA, very large data). A feature marked with a tier is available from that tier up. Tiers are **configuration/feature flags of one codebase, not paid editions.** Everything is open source.
`⚠ verify` marks a fact that is time-sensitive or that I could only confirm from secondary sources. Check it against the official text before shipping logic that depends on it.

---

## 1. Executive summary

### 1.1 The opportunity in one paragraph

Iranian businesses choose between two kinds of product. The first is **installed Windows software sold per module, per user, with hardware dongles** (Sepidar, Mahak, Holoo; Rahkaran at the enterprise end). It is deep in Iranian rules, but it is expensive, fragmented into paid sub-systems, and dated in UX. The second is **cloud-only SaaS** (Hesabfa and similar). It is cheap to start, but it is online-only, metered by document count, and shallow for payroll, fixed assets and contracting. Sepidar's own reseller states that a lost dongle cannot be replaced and you must buy the software again. The Accounting module alone lists at 34M toman, and bundles run 220M–506M toman (1405 list) [S-IR1]. Hesabfa states it has no offline version and meters plans by accounting documents per month [S-IR2][S-IR3]. Globally, users love the automation in QuickBooks/Xero (bank feeds, rules, auto-reconcile). They hate the price escalation: QBO Plus went $115→$140/mo and Advanced $275→$340/mo on 2026-08-01 [S-G1][S-G2]. They also complain about support quality and bank-feed glitches [S-G3][S-G4]. Open-source options are either dated (GnuCash), technical (Beancount/hledger), heavy ERPs (Odoo, ERPNext), or no longer truly open source (Akaunting moved to BSL 1.1 with paid marketplace apps) [S-G12]. **There is no fast, beautiful, keyboard-first, offline-capable, truly open-source accounting system that is fully compliant with Iranian law (Moadian, Sayad, insurance DISK, Jalali) and that scales from one person to a holding company.** That is the gap.

### 1.2 What the winners do right (copy these)

1. **Bank feed → rule → match → reconcile loop** (QBO, Xero): suggested matches, "always categorize like this" rules, one-click accept, auto-reconcile on higher tiers [S-G3][S-G2].
2. **Unlimited users per organisation** (Xero, Hesabfa): pricing is per organisation, not per seat [S-G2][S-IR2]. Per-user dongle pricing is the most hated Iranian pattern.
3. **Invoices that get paid**: branded templates, online pay link, automatic reminders before and after due date (Xero, FreshBooks, Invoice Ninja) [S-G5].
4. **Accountant-in-the-loop collaboration**: free accountant seats (QBO gives 2–3) and accountant portals (Pennylane, Digits) [S-G1][S-G8].
5. **Governed AI**: AI drafts categorizations, reconciliations and close tasks, and **nothing posts without human approval**, with a full audit trail (Puzzle) [S-G7]. Digits goes further with confidence-routed autonomous categorization and reports 97.8 % categorisation accuracy in its own benchmark [S-G8].
6. **Dimensions instead of exploding the chart of accounts** (Sage Intacct, NetSuite, Business Central): tag lines with department/project/location rather than creating accounts per combination.
7. **Plain-text, diffable, verifiable books** (Beancount/hledger): balance assertions, reproducible reports, git history. We borrow the *ideas* (assertions, reproducibility, export to ledger format), not the UX.
8. **Iranian incumbents' depth**: Sepidar's modular coverage (payroll with insurance list, fixed assets, contracting, distribution "hot/cold" sales, two-unit items, batch/expiry tracking, foreign orders) shows what serious Iranian companies expect [S-IR1].

### 1.3 Gaps to exploit

| Gap | Evidence | Our answer |
|---|---|---|
| Dongles, per-module and per-user licensing | Sepidar: lost dongle means rebuying; +20M toman per extra user; Moadian is a separate 34M-toman module [S-IR1]. Mahak: network capability is billed per user, and renewal is 25 % of list price per year [S-IR4] | Free and open source. No dongle. Unlimited users. Every module included. Paid offerings are hosting, support and implementation only |
| Online-only cloud | Hesabfa: no offline version; document-count metering; lower plans exclude Moadian/API/multi-currency [S-IR2][S-IR3] | Local-first desktop (SQLite) that syncs to a self-hosted or cloud server. Works through internet outages |
| Old Windows UX, mouse-heavy forms | Rahkaran and others criticised for dated UI and heavy implementation [S-IR6] | Keyboard-first grid entry, command palette, sub-100 ms interactions, RTL-first design system |
| Moadian pain (errors, corrective/cancel flows, statuses) | Rules on corrective/cancel/return invoices and status transitions are intricate [S-IR9][S-IR10] | A first-class e-invoice state machine with clear explanations, retry queues and simulation in a sandbox |
| Price escalation in global SaaS | QBO +13–24 % (Aug 2026), Xero +7–8 % (Oct 2026) [S-G1][S-G2] | No licence cost; self-hostable forever |
| "Open source" that is not | Akaunting is BSL 1.1 with paid apps [S-G12] | OSI licence (recommend **AGPL-3.0** for server/app, **Apache-2.0** for SDKs and plugin API) |
| Shallow OSS ERPs or heavy OSS ERPs | Odoo: powerful but complex/costly to customise; ERPNext: weaker accounting UX/reporting, needs DevOps [S-G11] | Accounting-first, opinionated, with plugins for the long tail |
| AI that is a black box | Users distrust auto-categorisation errors (QBO "uncategorized asset", wrong rules) [S-G4] | Explainable suggestions, confidence scores, rule provenance, approve-before-post, Persian OCR |

### 1.4 Design principles (non-negotiable)

1. **Correctness first.** Double entry is enforced in the database and in the type system. Posted entries are immutable; corrections are reversals. Every report is reproducible from the journal.
2. **Fast.** Cold start < 2 s on a mid laptop. Any list < 100 ms for 100k rows (virtualised and server-paginated). Keyboard round trip < 16 ms.
3. **Keyboard-first, mouse-friendly.** Every action is reachable from the command palette (Ctrl/⌘ K). Grid entry works like Excel. Shortcuts are documented in-app.
4. **Beautiful and calm.** shadcn/ui, Vazirmatn, generous whitespace, tabular numerals, dark mode, density toggle.
5. **RTL-first, bilingual.** Persian is the default, English is first-class, and switching is instant. Jalali is the primary calendar and Gregorian is always available.
6. **Offline-first for Solo; server-authoritative for teams.** One codebase.
7. **API-first.** The UI uses the same API that third parties use. OpenAPI is published. Webhooks are available for every domain event.
8. **Open and extensible.** WASM plugins in a sandbox. Importers for Excel, Sepidar and Holoo. Exports to Excel, PDF, CSV and ledger-text.
9. **Self-host and cloud.** A single binary plus Postgres, Docker images and a Helm chart. The same build is used for the hosted cloud.
10. **Law as data.** Tax rates, salary-tax brackets, insurance rates, Moadian schemas and COA templates are **effective-dated data packs**, not code constants. They update yearly without a release.
11. **Privacy & security.** Encryption at rest for local files is optional, argon2id passwords, least-privilege roles, signed audit chain.
12. **Progressive disclosure.** A Solo user never sees consolidation; an enterprise user can turn everything on.

---

## 2. Competitor research

> Method: vendor pricing pages and docs, G2, Capterra, NerdWallet and Reddit summaries, and Iranian reseller price lists. Prices are list prices on the date they were read and change often. Rows without a cited price say "see pricing page". No product fact here is invented. Anything I could not open first-hand is marked `⚠ verify`.

### 2.1 Global products

| Product | Segment & deployment | Pricing model (cited where verified) | What users love | Recurring pain points | Lesson for Daftar |
|---|---|---|---|---|---|
| **QuickBooks Online** (Intuit) | SMB, cloud | From 2026-08-01: Simple Start $38, Essentials $85 (3 users), Plus $140 (5), Advanced $340 (25 users, 3 accountant seats). Free/Lite/Ledger tiers also exist [S-G1][S-G9] | Bank-feed learning ("always categorise like this"), match suggestions, invoicing + payments, ecosystem [S-G4] | Price rises (Capterra: "high and rising subscription costs"), slow or scripted support, glitches, bank-feed sync and duplicate issues, the "uncategorized asset" trap, reconciled items can be deleted [S-G3][S-G4][S-G10] | Copy the learn-from-confirmation loop. **Never** allow deleting reconciled/posted lines. Make support unnecessary through clear errors |
| **Xero** | SMB, cloud | US from 2026-10-01: Early $27 (20 invoices, 5 bills), Growing $59, Established $97 (multi-currency, projects, expense claims); unlimited users on all plans [S-G2][S-G5] | Unlimited users, clean reconciliation, auto-reconcile, cash-flow forecast, app marketplace | Hard caps on the entry plan; multi-currency only on the top plan; payroll is US-partner based [S-G2] | Unlimited users. Multi-currency in every tier (it is a legal need in Iran). Built-in cash-flow forecast |
| **FreshBooks** | Freelancers/service SMB, cloud | Per-plan, client-count tiers; see pricing page | Invoicing UX, time tracking, support rated better than QBO (NerdWallet) [S-G4] | Weaker full accounting and inventory | Solo mode should feel like FreshBooks: invoice in 30 s, time to invoice |
| **Wave** | Micro/solo, cloud | Starter free (unlimited invoices, bills, records); Pro $19/mo or $190/yr adds bank import, auto-categorisation, receipts, collaborators [S-G6] | Free core | Feature gating of bank import; limited scale | A free forever solo tier with no feature gating is our default |
| **Zoho Books** | SMB, cloud | Free plan under $50k revenue (1 user + accountant, 1,000 invoices/yr); paid tiers above [S-G13] | Price/value, breadth, workflow rules, customer portal | Complexity at higher tiers; Zoho-suite lock-in | Workflow rules + customer portal are worth copying |
| **Sage Intacct** | Mid-market, cloud | Quote-based; see vendor | Multi-entity, **dimensions**, consolidation, audit trail | Cost, partner-led implementation | Dimensions on every line; multi-entity from the data model up |
| **NetSuite** (Oracle) | Mid/enterprise ERP | Quote-based; see vendor | OneWorld multi-subsidiary, consolidation, customisation | Cost, implementation length, UX | Ship consolidation and intercompany as first-class, not add-ons |
| **SAP Business One** | SMB/mid ERP, on-prem/hosted | Partner-sold; see vendor | Depth (inventory, production), partner network | Cost, partner dependence, UX | Respect deep inventory needs (batches, serials, multi-UoM) |
| **Dynamics 365 Business Central** | SMB/mid ERP, cloud/on-prem | Per-user; see vendor | Office/Excel integration, dimensions, extensions (AL) | Licensing complexity, partner costs | Excel round-trip editing; extension model |
| **Odoo** | SMB→mid ERP, open core (Community LGPL + paid Enterprise) | Per-user Enterprise; see vendor | Breadth, configurability, localisations | Setup and customisation become complex and costly, especially third-party modules and upgrades [S-G11] | Breadth via plugins, but a stable core API so upgrades never break |
| **ERPNext** (Frappe) | SMB ERP, GPL, self-host/cloud | Free self-host; paid hosting | All-in-one, truly open source | Self-hosting, updates and backups need DevOps/Python; accounting UX and reporting weaker [S-G11] | Single-binary self-host, one-click backups; accounting UX first |
| **Dolibarr** | Micro/SMB ERP/CRM, GPL, PHP | Free; paid modules on store | Easy install, modular | Dated UX, shallow accounting | — |
| **Akaunting** | SMB, Laravel, self-host/cloud | Core under **BSL 1.1** (converts to GPLv3 after four years); paid marketplace apps [S-G12] | Modern-ish UI, invoicing | "Open source" licence changed; buggy paid apps and refund complaints [S-G12] | Use an OSI licence. Never paywall core features |
| **Invoice Ninja** | Invoicing, open source (Elastic-style dual license; ⚠ verify current) | Self-host free; hosted plans | Invoices, client portal, recurring billing, payment gateways | Not full accounting | Client portal + recurring invoices pattern |
| **GnuCash** | Personal/small, desktop GPL | Free | Solid double entry, offline | UI "archaic", dialog-heavy [S-G14] | Offline + correctness is wanted; the UI is the barrier |
| **hledger / Beancount (+ Fava)** | Technical users, plain text | Free | Diffable books, balance assertions, reproducibility, git [S-G14] | Needs technical skill; manual categorisation | Balance assertions, reproducible reports, a ledger-text export |
| **Pennylane** | France/EU SMB + accountants, cloud | Per-plan; see vendor | Accountant–client collaboration in one ledger; document/receipt matching | EU-centric | Shared ledger between client and accountant, comment threads per transaction |
| **Puzzle** | Startups + accounting firms, cloud | See vendor | "Governed automation": AI Close drafts the work, nothing posts without sign-off, full audit trail; dual cash/accrual [S-G7] | — | Our AI policy: propose, explain, approve, audit |
| **Digits** | SMB + firms, cloud | See vendor | "Agentic General Ledger", 97.8 % categorisation accuracy in its own 2026 benchmark, live KPIs [S-G8] | Vendor-reported metrics | Confidence thresholds route uncertain items to humans |

**Themes from G2, Capterra and Reddit (cross-product)**
- *Price and nickel-and-diming*: add-ons, per-user fees and plan caps are the most common complaint (QBO: Capterra "55 % negative reviews out of 846" on cost) [S-G3].
- *Reliability and support*: glitches, errors loading, long support waits (Capterra "70 % negative reviews out of 615" on glitches) [S-G3][S-G10].
- *Bank-feed correctness*: duplicates, wrong auto-rules, "uncategorized" catch-alls, reconciled items that can be deleted [S-G4].
- *Customisation vs upgrade pain* in OSS ERPs (Odoo, ERPNext) [S-G11].
- *Open-source trust*: licence changes and paid marketplaces (Akaunting) [S-G12].
- *AI scepticism*: some users "do not like the AI features" [S-G10]. Governed, explainable AI earns trust [S-G7].

### 2.2 Iranian products

| Product (vendor) | Segment & deployment | Licensing & price (cited) | Strengths | Weaknesses / pain | Lesson |
|---|---|---|---|---|---|
| **Sepidar** (Sepidar System Asia, Hamkaran family) | SMB→mid, Windows client/server; cloud Sepidar is now offered [S-IR5] | Perpetual licence **per module** with a **hardware lock**. A lost lock cannot be replaced (buy again). Non-transferable, even to a subsidiary. 1 year free support, then paid. Default 1 user + 2 companies per lock; +20M toman per user; server install +5M. 1405 list: Accounting 34M, Receive/Pay 55M, Commercial inventory 65M, Sales 70M, Payroll 65M, **Moadian 34M**, Fixed assets 55M, Contracting 80M, Production 70M, Foreign orders 85M, Distribution hot/cold 130M each. Sub-systems: currency 32M, bilingual 40M, batch/expiry 12M, dual-unit 20M. Bundles 220M–506M toman [S-IR1] | Breadth and Iranian depth; >150k businesses and a large dealer network (vendor claims via [S-IR5]) | Total cost, dongle risk, per-user fees, paid add-ons for basics (currency, bilingual, batches) | Every one of those paid sub-systems is a free, built-in feature for us |
| **Rahkaran** (Hamkaran System / System Group) | Mid/large ERP: finance, sales, production, inventory, HR, BI, BPMS; web/cloud/on-prem [S-IR6] | No public price | Enterprise breadth, BPMS, Iranian compliance | Cost, complexity, long implementation, dated UI (reviews) [S-IR6][S-IR7] | Enterprise tier must have workflow/approvals, BI and HR depth, with a modern UX |
| **Hesabfa** | Micro/SMB, **cloud only** | Unlimited users. Yearly plans (Shahrivar 1405): 5.66M (1,800 docs/yr), 7.78M (3,600), 12.38M (7,200), 18.34M (36,000), 24.86M toman (unlimited). Moadian from plan 2; API, instalments, WooCommerce from plan 3; multi-currency and manufacturing from plan 4. Unused document credit expires. 15-day trial [S-IR2][S-IR3] | Easy, mobile app, cheque ops (deposit, clear, bounce, spend, due alerts), FIFO/LIFO/weighted average, perpetual/periodic switch, auto year-close, document lock, TTMS export, SMS, POS link, Jalali + Gregorian [S-IR2][S-IR3] | No offline, no direct DB access (Excel export only), document-count metering, feature gating [S-IR3] | Their solo/SB feature set is the baseline for our MVP, offline and ungated |
| **Mahak** | Micro→SMB, Windows | Perpetual. Families: store, production, company; 5.69M–45.89M toman (Mehr 1405). Add-ons (toman): multi-currency 5.69M, payroll 6.59M, fixed assets 3.89M, **network 3.59M per user**, remote work 7.49M, Moadian 3.59M, extra company 2.89M each. 1 yr support, then 25 % of current price per year [S-IR4] | Affordable entry, store/production editions | Add-on sprawl, per-user network fees, yearly renewal tax | — |
| **Holoo** (Kasra) | Retail/store, Windows | Search snippets show a range from ~5.7M toman (simple store) to ~87.7M (4-user network), and claim newer versions use a smart account card / SLM instead of a hardware lock. Could not open the price page `⚠ verify` [S-IR8] | Ubiquitous in retail; POS, barcode | Desktop-era UX | We need a Holoo importer (common migration source) |
| **Others seen** | Chortke, Karya Hesab, Prince, Darik, Hesabshop (cloud/desktop) | Not verified `⚠ verify` | — | — | Watch for importers/feature ideas only |

**Iran market takeaways**
1. *Moadian is the new must-have.* Every vendor sells it as a module or plan gate (Sepidar 34M, Mahak 3.59M, Hesabfa plan 2+) [S-IR1][S-IR4][S-IR3].
2. *Cheques are core, not an add-on.* Sayad registration, transfer and states belong in AR/AP and treasury [S-IR11].
3. *Payroll must produce the insurance DISK* (DSKKAR00/DSKWOR00 DBF) and the salary-tax list [S-IR13].
4. *Two-tier hierarchies of detail* (moein + tafsili, often multi-level tafsili/"شناور") are how Iranian accountants think. The COA model must be native, not emulated.
5. *Excel is the lingua franca.* Import/export of everything is required.
---

## 3. Iran-specific requirements (researched, with sources)

> All amounts in law and in Moadian are in **Rial**. "Toman" (= 10 Rial) is a display preference only. Store Rial, display either.

### 3.1 Calendar, fiscal year, locale
- The **Solar Hijri (Jalali)** calendar is primary. The fiscal year usually runs 1 Farvardin – 29/30 Esfand, but any 12-month fiscal year must be configurable. Store dates as ISO `DATE` (Gregorian, proleptic). Convert at the edges with a tested Jalali library (Rust: `icu_calendar` Persian calendar, or a dedicated crate; TS: `date-fns-jalali` or `@internationalized/date` Persian calendar). Golden-test conversions across 1300–1500 including leap years.
- Persian digits (۰–۹) for display, with input that accepts Persian, Arabic-Indic and Latin digits. The thousands separator is `٬` or `,` (user preference). Use the Persian decimal separator `٫` in fa locale.
- Weekend: Friday (and optionally Thursday). The holiday calendar is a data pack used for due-date and payroll working-day calculations.

### 3.2 Chart of accounts (کدینگ حساب‌ها)
- **Hierarchy:** گروه (group) → کل (general ledger) → معین (subsidiary) → تفصیلی (detail). Tafsili is a **separate dimension**, not just a deeper account level. One tafsili (e.g. a customer, a project, a bank account, a person) can attach to many moein accounts. Moein accounts declare which **tafsili levels** they accept (level 1, level 2 … "سطوح تفصیلی", plus floating/شناور).
- Model it as: `account` (group/kol/moein tree with nature debit/credit/both, and a balance-sheet/P&L/memorandum type) + `tafsili` entities (typed: person, bank, project, cost-center, item…) + `account_tafsili_rule` (which tafsili types are required or allowed at level n).
- Ship **seeded COA templates**: trading, service, manufacturing, contracting, non-profit, and a solo/freelancer template. Use standard Persian names and codes (e.g. 1 assets, 2 liabilities, 3 equity, 4 revenue, 5/6 costs/expenses, 7/8 memorandum (حساب‌های انتظامی)), plus mappings to the Moadian/tax statement lines (`⚠ verify` code ranges against the official "تحریر دفاتر" guidance and your accountant's preference).
- **Legal books**: the journal (دفتر روزنامه) and ledger (دفتر کل) in the format accepted for tax audit, with sequential numbering, plus the closing entries (بستن حساب‌های موقت، سند اختتامیه) and opening (سند افتتاحیه) entries.

### 3.3 VAT (مالیات بر ارزش افزوده)
- **The general rate is 10 % for 1405 (and was 10 % for 1403–1404).** It is 9 % under Art. 7 of the VAT Law (1400) plus 1 percentage point added by annual budget laws. The 1405 budget *bill* proposed 12 %, but the final law kept 10 % [S-IR14][S-IR15][S-IR16][S-IR17]. One public-sector circular describes the 10 % split as 1 % health, 4 % general revenue, 4 % municipal, 1 % pensioner equalisation `⚠ verify` [S-IR18].
- **Special goods** (Art. 26: e.g. tobacco, petroleum products, certain vehicles) have different or additional rates. **Exempt goods/services** are listed in Art. 9 (with annual updates in budget laws). Implement them as **tax codes with effective dates**, linked to the product's Moadian **product/service ID** (شناسه کالا/خدمت).
- Tax calculation per line, rounding to whole Rial at line level (match Moadian's line-level fields). Keep a configurable rounding policy, and golden-test it against sample Moadian JSON.

### 3.4 Moadian e-invoicing (سامانه مؤدیان)
- **Identifiers.** The **fiscal memory ID (شناسه یکتای حافظه مالیاتی)** has 6 characters. A taxpayer may hold several, e.g. per branch or per software. The **Tax ID (شماره منحصربه‌فرد مالیاتی)** has 22 characters: memory ID (6) + date (5 hex) + internal serial (10 hex) + a Verhoeff check digit (1). On submission the system returns a **reference number (شماره ارجاع/پیگیری)** [S-IR9][S-IR19].
- **Integration paths.** (a) **Direct API**: the taxpayer's own software signs requests with a certificate from GICA (the Iranian government root CA). API v2 is based on digital signatures. Calls cover getting a challenge/nonce, server public keys, taxpayer and fiscal-memory info, send invoice, and inquire by reference number, UID or time range [S-IR9]. (b) **Trusted provider (شرکت معتمد / ارائه‌دهنده خدمات مالیاتی)**: submit through a licensed intermediary. (c) Manual entry in the portal. Implement (a) and (b) as pluggable `EinvoiceProvider` adapters. `⚠ verify` the current API version, endpoints and the signing/encryption scheme against the latest official technical guide before coding; one source notes instruction **v7.9 (Tir 1405)** renamed `indati2m` to "registration date-time" [S-IR20].
- **Invoice types.** **Type 1**: buyer fully identified, which creates the buyer's tax credit. Patterns: sale, currency/exchange, gold/jewellery/platinum, contracting, utility bills, air tickets, export. **Type 2**: buyer optional, no credit for buyer. **Type 3**: retail receipt via POS/gateway (`⚠ verify` current applicability) [S-IR9][S-IR21].
- **Invoice subjects.** Original (اصلی), **corrective (اصلاحی)**, **cancellation (ابطالی)**, **sales return (برگشت از فروش)**. Reference invoices must cite the original Tax ID and use the same type and pattern. Buyer identity, product ID and VAT rate **cannot** be corrected; cancel and reissue. A cancellation cannot itself be referenced. Reference invoices are not available for originals issued before 1402-10-01 [S-IR9][S-IR22].
- **Deadlines.** Submit within **12 days** of the issue date (rule since 1403-08-15; some sources say 15 days when using a trusted provider) `⚠ verify`. Late submission carries penalties `⚠ verify amount` [S-IR21][S-IR23].
- **Buyer reaction.** The buyer approves or rejects in their **کارپوشه** (taxpayer cartable). Sources disagree on the window (30 days vs. 12 days before auto-acceptance) `⚠ verify`. Model the statuses: `draft → queued → sent → (failed | pending_buyer) → accepted | rejected | auto_accepted | cancelled`.
- **Product IDs.** Every sold item needs a 13-digit product/service ID (شناسه کالا/خدمت) from the national catalogue (stuff ID). Store it on the item. Validate the length and checksum where defined `⚠ verify`.
- **Software design consequences.**
  1. A `tax_invoice` aggregate, separate from the commercial invoice, that maps 1:1 and holds the immutable submitted JSON, signatures, responses and status history.
  2. **Serial allocation per fiscal memory** is a monotonic sequence. In offline/multi-device setups, allocate **serial blocks per device** to avoid collisions.
  3. An outbox with idempotent retries. Never double-submit a Tax ID. Reconcile with an inquiry by time range nightly.
  4. Plain-Persian explanations of every API error code, with "how to fix" links.
  5. Corrective/cancel/return wizards that enforce the rules above.

### 3.5 Seasonal transactions report (گزارش معاملات فصلی, Art. 169 / TTMS)
- Transactions **registered and accepted in Moadian do not need to be repeated** in the seasonal report. Transactions **not** registered in Moadian (e.g. purchases from people outside the system, some services) still must be reported [S-IR24][S-IR25][S-IR26].
- Export the TTMS-compatible file (Hesabfa still offers TTMS Access file export [S-IR3]). Build a reconciliation view: "purchases not covered by Moadian this season".

### 3.6 Cheques: Sayad (چک صیادی)
- **16-digit Sayad ID.** The issuer **registers** the cheque in the Sayad system (amount, due date, payee national ID, purpose "بابت"). The payee **inquires** and **confirms or rejects**. Transfer (endorsement) is done **only in the system**: the holder registers the new payee, who confirms. Paper endorsement no longer transfers ownership [S-IR11][S-IR12].
- Credit status colours of the issuer: white, yellow (1 bounced), orange (2–4), brown (5–10), red (>10) [S-IR11]. Show the colour on inquiry results (user-entered or via a bank API when available).
- **Lifecycle (received):** in-hand (در صندوق) → deposited for collection (در جریان وصول) → cleared (وصول شده) | bounced (برگشتی) | spent/endorsed to a supplier (خرج شده) | returned to the customer (عودت). **Issued:** issued (پرداختنی) → cleared | bounced | voided.
- **Postings** [S-IR27]:
  - Receive: Dr Notes receivable (اسناد دریافتنی) / Cr AR (customer tafsili).
  - Deposit: Dr Notes in collection / Cr Notes receivable. Make this optional.
  - Clear: Dr Bank / Cr Notes in collection (or receivable).
  - Spend: Dr AP (supplier) / Cr Notes receivable.
  - Bounce: Dr AR (customer) / Cr Notes (and record a bounced-cheque memo).
  - Issue: Dr AP / Cr Notes payable (اسناد پرداختنی). Clear: Dr Notes payable / Cr Bank.
- Batch operations, due-date calendar and alerts, a printable cheque register, and a cheque-book (دسته‌چک) inventory with serial ranges.

### 3.7 Payroll, social insurance and salary tax
- **Social insurance (تأمین اجتماعی).** The standard total contribution is **30 %**: **7 % employee**, **20 % employer**, **3 % unemployment insurance** (`⚠ verify` special workshops and sectors such as hard/harmful jobs, construction and exemptions). Contribution base: insured wages (مزایای مشمول) with a monthly **ceiling** and **floor** set yearly (data pack) [S-IR13].
- **Insurance list (DISK) export:** `DSKKAR00.DBF` (workshop/header record with totals such as `DSK_TBIME`, `DSK_TKOSO`, `DSK_BIC`, `DSK_RATE`) and `DSKWOR00.DBF` (one record per insured person: days, daily wage, monthly wage, benefits…). dBase III format with Iran System/CP1256 encoding `⚠ verify encoding` [S-IR13][S-IR28]. Use the `dbase` crate. Golden-test it against a sample accepted by the e-services portal.
- **Salary tax (مالیات حقوق), 1405.** Monthly exemption **40,000,000 toman (400,000,000 Rial)** (480M toman/yr). Marginal rates on the excess: **10 %** to 80M toman, **15 %** to 100M, **20 %** to 120M, **25 %** to 140M, **30 %** above 140M per month [S-IR29][S-IR30]. `⚠ verify` against the 1405 budget law and the official circular. Brackets live in an effective-dated data pack.
- Labour-law items (data pack, yearly): minimum wage, housing and grocery allowances (حق مسکن، بن)، child allowance (حق اولاد), seniority (پایه سنوات), overtime 40 % premium, night shift, holidays, end-of-year bonus (عیدی) and severance (سنوات), leave accrual. `⚠ verify` yearly figures from the Supreme Labour Council decision.
- Outputs: payslips (fa/en), bank transfer file (generic CSV/fixed-width per bank `⚠ verify formats`), salary-tax list export for the tax portal (`⚠ verify format`), insurance DISK, payroll journal entry.

### 3.8 Other Iranian specifics
- **Identifiers:** national ID (کد ملی, 10 digits with checksum), legal entity national ID (شناسه ملی, 11 digits), economic code (کد اقتصادی), postal code (10 digits), Sheba/IBAN `IR` + 24 digits (mod-97 check). Validate all of them.
- **Withholding (کسر از مبالغ):** withholding on contracting payments and the 5 % insurance retention on contracts (کسور بیمه پیمانکاری) and good-performance retention (حسن انجام کار) `⚠ verify rates`. These are contracting-module concepts.
- **Multi-currency:** the official vs. free-market rate gap is large. Support **multiple rate types** (CBI, NIMA/exchange-centre, free market, custom) with effective dates, and revaluation per rate type `⚠ verify` the current names of official rate systems.
- **Payment gateways & POS:** generic adapter interface (IPG redirect, POS terminal pairing, card-to-card reference capture). No vendor-specific coupling in core.
- **Banking:** no universal open-banking feed exists. Plan for statement import (Excel/CSV from each bank's internet banking; per-bank column mappers as a data pack) and optional bank-API adapters as plugins.
- **SMS** (invoice links, cheque due reminders): a generic SMS provider adapter.
- **Inflation:** high inflation makes **multi-currency reporting, inventory valuation choice, and fixed-asset revaluation** more important than in low-inflation markets. Offer "report in USD/EUR at rate type X" views.
---

## 4. Architecture

### 4.1 Shape: a modular monolith in one Cargo workspace

One deployable server binary, one desktop app and one CLI, built from **domain crates with enforced boundaries**. Microservices are explicitly out of scope. Accounting needs transactions that span modules (sales → inventory → ledger) atomically.

```
daftar/
├─ Cargo.toml                 # [workspace], shared lints, [workspace.dependencies]
├─ rust-toolchain.toml
├─ crates/
│  ├─ kernel/                 # Money, Currency, Rate, Ids (UUIDv7), Jalali, Clock, Errors, Paging, Tenant ctx
│  ├─ storage/                # sqlx pools (Pg + SQLite), Unit of Work, outbox, migrations runner, sea-query helpers
│  ├─ iam/                    # users, orgs, companies, roles, permissions, sessions, API keys, SSO (E)
│  ├─ audit/                  # append-only audit log with hash chain, activity feed, comments/mentions
│  ├─ coa/                    # accounts (group/kol/moein), tafsili, dimensions, templates
│  ├─ ledger/                 # journal entries, posting engine, periods/locks, balances, closing/opening
│  ├─ parties/                # customers, suppliers, employees-as-parties, contacts, addresses, ids validation
│  ├─ catalog/                # items, services, UoM & conversions, price lists, Moadian product ids, barcodes
│  ├─ sales/                  # quotes, orders, invoices (factor), returns, credit notes, recurring, receipts
│  ├─ purchasing/             # requisitions, POs, GRN, bills, returns, landed cost
│  ├─ inventory/              # warehouses, stock moves, valuation (FIFO/WAvg/…), batches/serials, counts
│  ├─ treasury/               # cash boxes, bank accounts, receipts/payments, transfers, statements, reconciliation
│  ├─ cheques/                # Sayad cheques received/issued, cheque books, lifecycle state machine
│  ├─ tax/                    # tax codes, rates (effective-dated), withholding, seasonal report (TTMS)
│  ├─ einvoice-moadian/       # Moadian mapping, signing, outbox, providers (direct / trusted)
│  ├─ payroll/                # employees, contracts, pay items, runs, insurance DISK, salary tax
│  ├─ assets/                 # fixed assets, depreciation (Iranian tax tables), disposals, revaluation
│  ├─ fx/                     # currencies, rate types, rates, revaluation, realised/unrealised gains
│  ├─ projects/               # projects, cost centres, contracting (progress statements, retentions)
│  ├─ manufacturing/          # BOM, work orders, costing (M)
│  ├─ pos/                    # POS sessions, receipts, terminals (SB+)
│  ├─ budgeting/              # budgets, forecasts, cash-flow forecast
│  ├─ consolidation/          # groups, intercompany, eliminations, minority interest (E)
│  ├─ workflow/               # approvals, rules engine, notifications, scheduled jobs
│  ├─ reporting/              # report engine (trial balance, P&L, BS, CF, aging, ledgers), saved views, exports
│  ├─ documents/              # attachments, OCR pipeline, PDF rendering (Typst), templates
│  ├─ importers/              # Excel/CSV, Sepidar, Holoo, Hesabfa-Excel, ledger-text; exporters
│  ├─ ai/                     # categorisation suggestions, OCR extraction, anomaly checks, NL query (opt-in)
│  ├─ sync/                   # device registry, change feed, push/pull protocol, conflict handling
│  ├─ plugins/                # WASM host (wasmtime/extism), plugin manifest, capabilities
│  ├─ api/                    # axum routers, DTOs, OpenAPI (utoipa), auth middleware, webhooks
│  └─ app/                    # composition root: wires modules, config, feature flags (tiers)
├─ apps/
│  ├─ server/                 # bin: axum server (multi-tenant, Postgres)
│  ├─ desktop/                # Tauri v2 shell (src-tauri) embedding `app` with SQLite or remote mode
│  ├─ cli/                    # bin: admin, migrations, import/export, backup/restore, data packs
│  └─ web/                    # React app (shared by desktop and web build) — see FRONTEND-UI-GUIDE.md
├─ data-packs/                # versioned, effective-dated law data: VAT, salary tax, insurance, COA, holidays, bank mappers
├─ migrations/{postgres,sqlite}/
├─ tests/                     # cross-crate integration, golden reports, e2e harness
├─ docs/                      # ADRs (docs/adr/NNNN-*.md), domain glossary fa/en, API docs
└─ deploy/                    # Dockerfile, docker-compose, Helm chart, systemd unit
```

**Boundary rules (enforced by `cargo-deny`/a custom `xtask check-deps`):**
- Domain crates depend only on `kernel`, `storage` traits, and the **public API of other domain crates** (`pub mod api`). They never touch another crate's tables directly.
- Cross-module side effects go through **domain events** written to the outbox in the same DB transaction (e.g. `InvoicePosted` → inventory issue and tax_invoice creation), or through synchronous service calls inside one Unit of Work when atomicity is required (posting).
- `ledger` is the only crate allowed to write `journal_entry`/`journal_line`. Every other module produces a `PostingRequest`.

### 4.2 Recommended crates (and why)

| Concern | Crate(s) | Why / notes |
|---|---|---|
| Async runtime | `tokio` | Standard; required by axum, sqlx and Tauri |
| HTTP API | `axum`, `tower`, `tower-http` (cors, compression, trace, timeout, request-id) | Ergonomic, tower middleware ecosystem, first-class with tokio |
| DB access | `sqlx` (features: `postgres`, `sqlite`, `uuid`, `rust_decimal`, `time`/`chrono`, `migrate`) | Async, compile-time checked queries (`query!`), built-in migrations, supports both target DBs |
| Dynamic SQL | `sea-query` + `sea-query-binder` (sqlx) | Report filters, list screens with user-chosen columns/sorts, and **dialect differences between Postgres and SQLite**, without string concatenation. Prefer `sqlx::query!` for static hot paths |
| Money | `rust_decimal` (+ `rust_decimal_macros`) | Exact base-10, 28 significant digits, safe for Rial magnitudes. Never `f64`. Wrap it in a `Money { amount: Decimal, currency: CurrencyCode }` newtype with checked ops |
| Serialization | `serde`, `serde_json`, `serde_with` | Decimals serialised as **strings** in JSON to avoid JS float loss |
| IDs | `uuid` (v7) | Time-ordered, client-generatable (offline), index-friendly |
| Time | `time` or `chrono` (pick one), `icu_calendar` (Persian) | `⚠ verify` the Jalali crate choice. Wrap it behind `kernel::jalali` with exhaustive tests |
| Errors | `thiserror` (libs), `anyhow` (bins only) | Typed domain errors mapped to RFC 7807 problem+json |
| Validation | `garde` or `validator` | DTO validation; domain invariants live in constructors |
| Observability | `tracing`, `tracing-subscriber`, `tracing-opentelemetry` (M/E) | Structured logs, spans per request/command |
| OpenAPI | `utoipa`, `utoipa-axum`, `utoipa-scalar` (docs UI) | Code-first spec; generate the TS client from it |
| TS types for Tauri | `specta` + `tauri-specta` | Typed Tauri commands/events generated into TS |
| Auth | `argon2`, `jsonwebtoken` or PASETO (`pasetors`), `openidconnect` (E SSO), `totp-rs` | |
| Rate limiting | `tower_governor` | Public API |
| Jobs | Outbox table + a tokio worker; `apalis` optional | Moadian queue, emails, recurring invoices |
| Search | Postgres FTS/trigram; `tantivy` for desktop | Global search, Persian normalisation (ي→ی، ك→ک, ZWNJ) |
| Excel | `calamine` (read), `rust_xlsxwriter` (write) | Import/export everywhere |
| PDF | `typst` (embedded) with Vazirmatn | High-quality RTL invoices/reports, deterministic output for golden tests `⚠ verify` Persian shaping quality early (spike) |
| DBF | `dbase` | Insurance DISK files |
| Crypto (Moadian) | `rsa`/`ring`/`josekit`, `x509-parser` | `⚠ verify` the exact signing/encryption scheme from the official guide |
| Plugins | `wasmtime` or `extism` | Sandboxed extensions with capability grants |
| Testing | `proptest`, `insta` (snapshots), `cargo-nextest`, `testcontainers` (Postgres), `fake` | Invariants, golden reports, real DB tests |
| Desktop | `tauri` v2 + plugins: `updater`, `dialog`, `fs`, `store`, `single-instance`, `window-state`, `log`, `deep-link` | |

### 4.3 Tauri commands vs a local HTTP API

| Option | Pros | Cons |
|---|---|---|
| **A. Tauri commands (IPC)** call the Rust services in-process | No open port. No CORS/auth surface on localhost. Fast. Typed via tauri-specta. Works fully offline | Different transport from the web build |
| **B. Embedded axum on 127.0.0.1** with a random port + per-launch token | One transport for desktop and web; can be scripted locally | Opens a local port (other local processes, firewall/AV prompts, port conflicts); must handle auth on localhost anyway |
| **C. Remote server** (team mode) | Required for multi-user | Needs network |

**Decision:** design a **single application-service layer** (`app::Services`) with **command/query DTOs** shared by both transports.
- Desktop **solo/offline**: the React app calls **Tauri commands** (Option A). Each command is a thin adapter: `#[tauri::command] async fn post_invoice(state, dto) -> Result<Dto, ApiError>` → `services.sales.post_invoice(ctx, dto)`.
- Desktop **team mode** and **web build**: the same React code uses the **HTTP client** generated from OpenAPI against `apps/server` (Option C).
- The frontend depends on a `Transport` interface (`invoke` vs `fetch`), selected at startup. Both clients are generated: TS types from `specta` (commands) and from `utoipa` (HTTP), and CI fails if the two DTO sets diverge.
- Option B is available behind a flag for power users or scripting (`daftar serve --local`). It is not the default.

### 4.4 Data model essentials

- **Tenancy:** `organisation` (billing/self-host instance) → `company` (legal entity, own COA, fiscal years, base currency) → `branch` (optional; fiscal memory per branch). Every business row carries `company_id`. Postgres **Row-Level Security** keyed by a session variable as a second line of defence (M/E).
- **Ledger tables:**
  - `fiscal_year(id, company_id, start, end, status open|closing|closed)`
  - `period(id, fiscal_year_id, start, end, lock_level none|soft|hard)`
  - `journal_entry(id uuidv7, company_id, number bigint (per FY sequence), date, period_id, source_type, source_id, description, status draft|posted|reversed, reversal_of, reversed_by, posted_at, posted_by, hash, prev_hash)`
  - `journal_line(id, entry_id, line_no, account_id, tafsili_1..4 ids, dimension_set_id, debit numeric, credit numeric, currency, fx_rate, amount_fc, qty, description)`
  - `account_balance(company_id, account_id, tafsili…, period_id, debit_sum, credit_sum)` (derived, rebuildable).
- **Invariants (DB + code):**
  1. Per entry, `Σ debit = Σ credit` in base currency. Enforce with a **deferred constraint trigger** on Postgres and a check in the posting engine for SQLite, and proptest it.
  2. Each line has exactly one of debit/credit > 0, the other = 0, and neither negative.
  3. A posted entry is immutable: an `UPDATE`/`DELETE` trigger rejects changes when `status = posted` (except setting `reversed_by`).
  4. No posting into a hard-locked period or closed fiscal year. Soft lock requires the `ledger.post_in_locked` permission and leaves an audit reason.
  5. Account must be postable (leaf moein) and active, and its required tafsili levels must be present.
  6. Gap-free per-FY entry numbers are assigned at **post** time (drafts have none), under a row lock on a counter table.
  7. Hash chain: `hash = sha256(prev_hash || canonical_json(entry+lines))`. A verify command recomputes it (tamper evidence for auditors).
- **Documents vs ledger:** business documents (invoice, bill, receipt, payroll run…) are mutable while **draft**. **Posting** creates a journal entry with `source_type/source_id`. Editing a posted document = **auto-reversal + new posting** (with a visible "amended" chain), allowed only when the period is open and no downstream lock exists (e.g. an accepted Moadian invoice → must use corrective/cancel flows).
- **Dimensions:** `dimension(id, code, name, type)`, `dimension_value`, `dimension_set` (hash of sorted values) referenced by lines. Tafsili is Iranian-native. Dimensions (department, project, location, custom) are the international concept. Both are supported and reportable.
- **Money storage:** Postgres `NUMERIC(28,6)` for amounts, `NUMERIC(28,10)` for rates; SQLite stores amounts as canonical decimal TEXT with app-side arithmetic (or INTEGER minor units for base currency `⚠ decide in ADR-0003`). Base currency IRR has **0 decimals**. Each currency defines its minor unit. Rounding is per line with a documented strategy (banker's vs half-up: Moadian expects whole Rial; default half-up `⚠ verify`).
- **Soft delete** is not allowed for posted or referenced records. Use `active=false` for master data.

### 4.5 Sync model: from offline solo to multi-user server

**Modes**
1. **Solo-local:** the desktop app with an embedded SQLite file (`company.daftar`). Optionally SQLCipher-style encryption `⚠ verify` sqlx compatibility. Backups are file snapshots.
2. **Solo-synced:** the same, plus a sync to a server (self-hosted or cloud) for backup, a second device and the accountant's access.
3. **Team:** the server (Postgres) is authoritative. Desktop clients are online clients with a **read cache + offline draft queue**.

**Protocol (push/pull change feed)**
- Every mutable row has `id (uuidv7)`, `version (bigint)`, `updated_at`, `updated_by_device`. The server keeps a monotonically increasing `change_seq` per company. Clients pull `changes since seq N` and push **commands** (not raw row diffs), e.g. `CreateInvoiceDraft`, `PostInvoice`, `UpdateCustomer{fields…, base_version}`.
- **Master data and drafts:** optimistic concurrency with field-level merge. If the same field changed on both sides → **conflict record** shown in a "Sync inbox". Defaults: server wins, with the client's value kept for one-click re-apply.
- **Posting rules:**
  - In *Solo-local*, posting is local and final (the device is the authority).
  - In *Team/Solo-synced*, posting while offline creates a **provisional posting** (local number prefixed `P-`). The server validates on sync (period locks, stock availability policy, credit limits) and assigns the final number, or rejects the posting with a reason → it returns to draft with a notification. Provisional entries are excluded from "official" reports until confirmed (visible badge).
  - Posted entries never merge; they are immutable events. Corrections are reversals → no ledger conflicts by construction.
- **Sequences that must not collide** (invoice numbers, Moadian serials, cheque-book serials): **block allocation per device** (e.g. 1,000 numbers leased per device, renewable). Moadian submission happens **only from the server** in Team mode, or from the single owning device in Solo.
- **Promotion path** Solo → Team: `daftar migrate --from company.daftar --to postgres://…` (or upload in-app). This is a lossless copy of all IDs and the hash chain. After promotion the desktop switches to Team mode.
- **Security:** device registration with a per-device key; sync over TLS; per-company scopes.

### 4.6 API surface
- REST + JSON under `/api/v1`, resource-oriented, with **cursor pagination**, `?filter=` (simple DSL), `?fields=`, `Idempotency-Key` header on POSTs, ETag/`If-Match` for updates, and RFC 7807 errors with **localized messages (fa/en)** and stable `code`s.
- Commands for state transitions: `POST /invoices/{id}:post`, `:void`, `:send-to-moadian`, and so on.
- Webhooks: signed (HMAC), retried with backoff, plus a replay UI.
- Published OpenAPI 3.1 and a generated TS client. A Rust SDK and the plugin API are Apache-2.0.

### 4.7 Security, compliance and operations
- Roles + permissions (resource × action × scope company/branch/own), segregation-of-duties rules (M: the creator cannot approve).
- Audit log of every mutation (who, when, device, IP, before/after diff), plus a hash-chained journal.
- Backups: desktop does automatic rolling local backups + an export to a single encrypted archive; server runs `pg_dump`/WAL guidance. **Restore drills** in CI.
- Data packs signed and versioned; the app shows "law data version" in About and on reports.
- Telemetry: off by default; opt-in, anonymous.
- Licence: AGPL-3.0 (core) / Apache-2.0 (SDK, plugin API) — `⚠ decide` with counsel; ADR-0001.
---

## 5. Module specification with scale tiers

Tier tags: **[S]** Solo · **[SB]** Small business · **[M]** Mid-market · **[E]** Enterprise. A tag means "available from this tier up". Tiers are feature-flag presets (`company.settings.tier`). Any flag can be toggled individually.

### 5.0 Tier overview

| Capability | S | SB | M | E |
|---|:-:|:-:|:-:|:-:|
| Storage | SQLite (local) | Postgres server or LAN; SQLite+sync | Postgres | Postgres HA, read replicas, partitioning |
| Users | 1 (+ accountant via sync) | ≤ 20 | ≤ 200 | unlimited, SSO/SCIM |
| Companies | 1–3 | several | many | groups + consolidation |
| Approvals | — | simple (amount threshold) | multi-step, SoD | policy engine, delegation |
| Inventory | simple items | warehouses, perpetual | batches/serials, landed cost, multi-UoM | multi-site, MRP (plugin) |
| Payroll | — (owner draws) | ✓ | ✓ + attendance import | ✓ + HR plugins |
| Multi-currency | ✓ | ✓ | ✓ revaluation | ✓ + consolidation translation |
| Moadian | ✓ | ✓ | ✓ multi-memory/branch | ✓ multi-company, high-volume queue |
| Audit | audit log | + activity feed | + hash-chain verify, auditor role | + SIEM export, retention policies |

### 5.1 Organisation, companies & IAM
- [S] Local profile, optional app PIN/password, OS-keychain-backed secrets.
- [S] Company setup wizard (see UI guide §2): legal info, national ID / economic code, fiscal year, base currency, COA template.
- [SB] Users, invites, roles (Owner, Admin, Accountant, Bookkeeper, Sales, Purchasing, Storekeeper, Cashier, Payroll officer, Auditor read-only, External accountant).
- [SB] Fine-grained permissions (resource × action × scope), per-company assignment.
- [M] Segregation-of-duties policies; approval delegation; login policies (2FA TOTP mandatory option).
- [E] SSO (OIDC/SAML via proxy), SCIM provisioning, IP allow-lists, session policies, organisation-wide admin console.

### 5.2 Chart of accounts, tafsili & dimensions
- [S] Seeded Iranian COA templates; account tree editor (group/kol/moein); natures (Dr/Cr/both); postable flags.
- [S] Tafsili entities auto-created for customers, suppliers, banks, cash boxes, employees, projects; the user can add custom types.
- [S] Account ↔ tafsili-level rules (required/optional per level, up to 4 levels; floating tafsili).
- [SB] Dimensions (department, location, project, custom), with defaults per account and per party.
- [M] Account mapping to statutory/tax report lines; account groups for reporting; budget-control flags.
- [E] Group COA with company-level mapping for consolidation; COA change governance (approval required).

### 5.3 General ledger & periods
- [S] Manual journal entries with grid entry, templates, recurring entries, attachments, reverse/duplicate.
- [S] Immutable posting, reversal entries, sequential numbering, the legal journal/ledger prints.
- [S] Fiscal years and periods; soft/hard locks; year-end close wizard (temporary accounts → P&L summary → retained earnings; closing entry; opening entry for next year) per Iranian practice.
- [SB] Accruals/deferrals schedules; allocation entries (by fixed % or driver).
- [M] Entry approval workflow; per-period close checklist with owners; audit-adjustment period (period 13).
- [E] Intercompany entries with automatic due-to/due-from; parallel ledgers (statutory vs management/IFRS) `⚠ scope`.

### 5.4 Sales & receivables (فروش و دریافتنی‌ها)
- [S] Customers (person/legal; IDs validated), products/services, invoices (فاکتور) with line discounts, VAT per line, Moadian fields, PDF in fa/en, share link.
- [S] Receipts (cash, card/POS, transfer, cheque), allocation to invoices, customer statements, aging.
- [S] Pre-invoice/quote (پیش‌فاکتور) → invoice conversion.
- [SB] Sales orders, delivery notes, partial invoicing, price lists, customer groups, credit limits, recurring invoices, payment reminders (SMS/email), instalment sales.
- [SB] Sales returns (برگشت از فروش) linked to the Moadian return flow; credit notes.
- [M] Commissions/marketers, sales targets, approval of discounts above a threshold, distribution (route sales) `plugin`.
- [E] Revenue recognition schedules (subscriptions, contracts) `⚠ scope`; customer portal SSO.

### 5.5 Purchases & payables
- [S] Suppliers, bills, expenses (quick expense with receipt photo), payments, supplier aging.
- [SB] Purchase orders, goods receipt (GRN), 3-way match (PO–GRN–bill), purchase returns, purchase from non-Moadian sellers flagged for the seasonal report.
- [M] Requisitions with approvals, landed cost allocation (freight/customs/insurance) to items, foreign purchases (proforma, LC/bank transfer tracking).
- [E] Vendor portal, contract pricing, budget checks on commitment.

### 5.6 Inventory
- [S] Items and services, single warehouse, stock on hand, simple valuation (weighted average).
- [SB] Multiple warehouses, transfers, adjustments, stock counts (انبارگردانی), perpetual vs periodic, valuation FIFO/weighted average (moving or periodic) per company; reorder points.
- [M] Batches/lots with expiry, serial numbers, multi-UoM with conversions (two-unit items), barcodes, bins/locations, landed cost, reservations, cost revaluation.
- [E] Multi-site replenishment, consignment, MRP via plugin.
- Rules: inventory moves post to the ledger on posting (perpetual) or via period-end entries (periodic). Negative stock is policy-controlled (block/warn/allow).

### 5.7 Treasury: cash, banks, reconciliation
- [S] Cash boxes (صندوق), bank accounts (Sheba validated), petty cash (تنخواه), transfers, receipts/payments not linked to invoices.
- [S] Bank statement import (Excel/CSV with per-bank mapping packs), reconciliation with suggestions (amount/date/reference fuzzy match), rules ("always book like this"), reconciliation report and lock.
- [SB] POS terminal settlements (card receipts batched, fees), card-to-card capture, cash-count (close of day).
- [M] Bank API adapters (plugins), auto-reconcile above a confidence threshold with review queue, cash-flow forecast (30/60/180 days).
- [E] Cash pooling, intercompany settlements, treasury dashboards.

### 5.8 Cheques (Sayad)
- [S] Received and issued cheques with Sayad ID (16 digits), bank, branch, due date, amount, purpose, drawer national ID; full lifecycle (§3.6) with automatic postings; due calendar; reminders.
- [S] Cheque books (دسته‌چک) with serial ranges and status per leaf.
- [SB] Batch operations (deposit 20 cheques, clear many), bounced-cheque workflow (protest/legal follow-up notes), transfer (خرج) to supplier with Sayad transfer step recorded, colour status capture.
- [M] Bank-API Sayad inquiry/register via plugin when available `⚠ verify availability`; guarantee cheques/promissory notes (چک و سفته ضمانت) as memorandum entries.

### 5.9 Tax: VAT, Moadian, seasonal, withholding
- [S] Tax codes (standard 10 %, exempt, special Art. 26) effective-dated; product Moadian IDs; Moadian submission (direct API or trusted provider); status tracking; corrective/cancel/return wizards; VAT return worksheet.
- [S] Seasonal report (TTMS) export of non-Moadian transactions.
- [SB] Purchase-side Moadian: import the cartable (invoices issued to us), accept/reject workflow in-app, match to bills.
- [M] Multiple fiscal memories (per branch/software), queue dashboard, bulk submission, nightly reconciliation by time range.
- [E] High-volume submission (≥ 100k/month), partitioned queues, per-company certificates via HSM/KMS `⚠ verify` legal key-custody rules.
- [SB] Withholding on contracts and payments (e.g. contracting retentions), insurance retention on contracts `⚠ verify rates`.

### 5.10 Payroll & HR-lite
- [SB] Employees (national ID, insurance no., bank Sheba), contracts, pay elements (data-pack driven: base, housing, grocery, child, seniority, overtime, deductions, loans), monthly runs, payslips, salary tax, insurance (DISK export), bank file, payroll entry.
- [SB] Year-end: عیدی, سنوات, leave balances; settlement (تسویه حساب) on termination.
- [M] Attendance import (CSV from time clocks), shift/overtime rules, multiple workshops (کارگاه) per company, cost-centre allocation of payroll.
- [E] HR plugins (recruitment, performance); employee self-service portal.

### 5.11 Fixed assets
- [SB] Asset register, categories, acquisition (from bill), depreciation methods (straight-line, declining balance) with **Iranian tax depreciation table (جدول استهلاکات موضوع ماده ۱۴۹)** as a data pack `⚠ verify` current table, monthly depreciation runs, disposal/sale with gain/loss.
- [M] Revaluation (where permitted `⚠ verify`), transfers between locations/cost centres, asset tags/barcodes, maintenance log.
- [E] Group asset policies, IFRS vs tax books in parallel.

### 5.12 Multi-currency & FX
- [S] Currencies, rate types, manual rates, transactions in foreign currency with base-currency equivalents.
- [SB] Rate import (CSV/plugin), FX gain/loss on settlement (realised).
- [M] Period-end revaluation (unrealised) by rate type; reports in reporting currency.
- [E] Translation for consolidation (closing/average/historical rates), CTA.

### 5.13 Projects, cost centres & contracting
- [SB] Projects/cost centres as tafsili/dimensions; project P&L; time & expense to invoice.
- [M] Contracting (پیمانکاری): contracts, progress statements (صورت‌وضعیت), advances, retentions (good performance, insurance), withholding, adjustment (تعدیل) `⚠ verify` formulae, Moadian contracting pattern.
- [E] Portfolio reporting, earned-value.

### 5.14 Manufacturing (light)
- [M] BOMs, work orders, material issue, finished goods receipt, standard/actual costing, scrap. Full MRP is a plugin.

### 5.15 POS
- [SB] Touch-friendly POS screen (offline-capable), barcode scan, cash drawer sessions, card terminal pairing via generic adapter, receipt printing (80 mm thermal, Persian), daily Z-report to ledger, Moadian handling for retail `⚠ verify` type-2/3 rules.

### 5.16 Budgeting & planning
- [SB] Annual budget by account/tafsili/dimension, budget vs actual.
- [M] Versions/scenarios, monthly phasing, budget control on POs (warn/block), rolling cash-flow forecast.
- [E] Group budgets, driver-based planning (plugin).

### 5.17 Multi-company & consolidation
- [SB] Many companies in one install; company switcher; shared master data optional (parties/items catalogue).
- [E] Group structure with ownership %, intercompany matching, elimination entries, minority interest, currency translation, consolidated statements, consolidation audit trail.

### 5.18 Reporting & analytics
- [S] Trial balance (2/4/6/8-column, by kol/moein/tafsili), journal, general ledger, account/tafsili statements (صورتحساب), P&L, balance sheet, cash flow (indirect), AR/AP aging, sales/purchase by item/customer, inventory valuation & cardex (کاردکس کالا), VAT reports, cheque reports.
- [S] Drill-down from every number to entries to source documents. Compare periods. Export to Excel/PDF/CSV. Print-perfect fa/en layouts.
- [SB] Saved views, scheduled report emails, dashboards (KPIs: cash, AR, AP, sales, gross margin, runway).
- [M] Report builder (pivot on ledger + dimensions), financial statement designer (rows = account ranges), comparatives and %.
- [E] Read-replica analytics, data warehouse export (Parquet), BI connectors (ODBC via Postgres views).
- Golden reports: every standard report has fixture-based snapshot tests.

### 5.19 Workflow, approvals & notifications
- [SB] Simple approval: documents above amount X need approval by role Y.
- [M] Multi-step approval chains with conditions (amount, account, project, party), SoD, escalation, delegation; in-app + email + SMS notifications; scheduled jobs UI.
- [E] Policy engine (rules as data, versioned), BPMN-lite for custom flows (plugin).

### 5.20 Documents, OCR & AI (opt-in, governed)
- [S] Attachments on any record (drag-drop, camera on mobile web), storage in DB/file store with hashes.
- [SB] OCR of receipts/invoices (Persian + English) → draft bill/expense with field confidence `⚠ verify` OCR engine quality for Persian (spike: Tesseract fa vs hosted models).
- [SB] Smart categorisation for bank lines: rules first, then ML/LLM suggestions with **confidence and explanation**. Nothing posts without approval unless the user creates an explicit auto-post rule.
- [M] Anomaly checks (duplicate bills, unusual amounts, weekend postings, round-number entries, missing Moadian submission), close assistant (checklist pre-fill).
- [E] Natural-language query over reports ("فروش فصل گذشته به تفکیک استان") running read-only, permission-scoped queries; self-hosted model option. AI provider is pluggable; no data leaves the server without admin opt-in.

### 5.21 Import, export & migration
- [S] Excel templates for every master and opening balance; ledger-text (hledger/Beancount) export; full-company JSON export.
- [S] Importers: Excel (generic), **Hesabfa Excel exports**, **Sepidar** and **Holoo** (via their Excel/report exports or DB reading where legally/technically feasible `⚠ verify` formats) — accounts, tafsili, parties, items, opening balances, open invoices, cheques.
- [SB] Validation preview, error report, idempotent re-import, mapping memory.

### 5.22 Integrations & extensibility
- [SB] REST API + webhooks; generic payment-gateway (IPG) adapter; SMS adapter; e-commerce plugins (WooCommerce et al.) as plugins.
- [M] WASM plugin SDK (custom reports, document hooks, importers, UI extension points via a manifest).
- [E] Event streaming export (e.g. to Kafka-compatible) `plugin`.

### 5.23 Audit, backup & administration
- [S] Audit log, automatic local backups (rolling, keep N), restore, integrity check (hash chain, trial balance zero, subledger = control account).
- [SB] Activity feed, comments and @mentions on any record, read-only auditor access, export of audit trail.
- [M] Retention policies, data-change reports for auditors, period-close sign-off.
- [E] SIEM export, legal hold, PITR backups, HA guidance.
---

## 6. Frontend / UX (summary)

> The full screen-by-screen guide, design tokens, component inventory and a frontend-only build prompt are in **[FRONTEND-UI-GUIDE.md](./FRONTEND-UI-GUIDE.md)**. This section only fixes the decisions the backend depends on.

- **Stack:** React + TypeScript + Vite, **shadcn/ui** (it has first-class RTL support since January 2026: `rtl: true` in `components.json`, `DirectionProvider`, and logical classes generated by the CLI [S-G15]), Tailwind, **TanStack Router / Query / Table / Virtual**, **react-hook-form + zod**, `cmdk` (command palette), Vazirmatn font, a Jalali date library.
- **One UI, two hosts:** the same `apps/web` bundle is loaded by Tauri (desktop) and by any browser (web build). The `Transport` abstraction picks Tauri `invoke` or HTTP `fetch` (see §4.3).
- **Money in the UI:** amounts travel as **decimal strings**. They are parsed and formatted with a decimal library (e.g. `decimal.js`/`big.js`) and never as `number`. Inputs accept Persian/Arabic/Latin digits, `k`/`m` shortcuts ("12m" → 12,000,000) and toman/rial unit toggles.
- **Keyboard-first data entry:** an Excel-like grid for journal lines, invoice lines and bank reconciliation. Enter/Tab moves between cells; `=` balances the last line; F2 edits; Ctrl+Enter posts.
- **States the API must expose for UX:** draft/posted/reversed, provisional (offline), Moadian status timeline, lock reasons, permission reasons ("why can't I edit this?") as structured error codes.
- **Performance budgets:** first paint < 1 s in desktop. All lists are server-paginated + virtualised. Optimistic updates only for drafts; posting waits for the server's confirmation.
- **Accessibility:** WCAG 2.2 AA, full keyboard, screen-reader labels in fa/en, `prefers-reduced-motion`.
---

## 7. Master checklist

Each item carries a milestone tag. **M0** Foundation · **M1** Solo MVP · **M2** Small business · **M3** Mid-market · **M4** Enterprise. Tick an item only when its code is merged with tests passing and docs are updated. UI-only items live in FRONTEND-UI-GUIDE.md §14.

### 7.1 Foundation & repo
- [ ] M0 Cargo workspace with crates from §4.1; shared `[workspace.lints]` (clippy pedantic subset, `unsafe_code = "forbid"`)
- [ ] M0 `rust-toolchain.toml` pinned; `cargo-deny` (licences/advisories); `xtask check-deps` enforcing crate boundaries
- [ ] M0 CI: fmt, clippy -D warnings, nextest (SQLite + Postgres via testcontainers), frontend lint/typecheck/test, Tauri build for Win/macOS/Linux
- [ ] M0 ADRs: 0001 licence, 0002 modular monolith, 0003 money representation, 0004 transport (Tauri cmds + HTTP), 0005 sync protocol, 0006 Jalali library, 0007 PDF engine
- [ ] M0 `kernel::Money`, `Currency`, `Decimal` serde as string, checked arithmetic, allocation (split amount by ratios with remainder distribution) + proptests
- [ ] M0 `kernel::jalali` conversions, formatting/parsing fa/en, fiscal-year helpers; golden table 1300–1500
- [ ] M0 UUIDv7 ids, typed id newtypes per entity
- [ ] M0 Error model → RFC 7807 with stable codes and fa/en messages (Fluent `.ftl` files)
- [ ] M0 `storage`: pools for Pg/SQLite, Unit of Work, outbox table + worker, migrations for both dialects, sea-query helpers
- [ ] M0 Tracing setup, request ids, structured logs to file (desktop) / stdout JSON (server)
- [ ] M0 Data-pack loader: signed, versioned, effective-dated JSON/TOML; `daftar datapack verify`
- [ ] M0 Tauri v2 shell with tauri-specta bindings; `Transport` abstraction in web app; HTTP client generated from OpenAPI
- [ ] M0 Seed + fixtures framework (`fake`), demo company generator ("sample data" option)

### 7.2 IAM & tenancy
- [ ] M1 Local profile, app lock (password/PIN), secrets in OS keychain
- [ ] M1 Company create/edit with legal identifiers validation (national ID, legal ID, economic code, postal code, Sheba)
- [ ] M2 Users, invitations (email/link), roles & permissions matrix, per-company assignment
- [ ] M2 Session management, 2FA TOTP, password policy, argon2id
- [ ] M2 External accountant & auditor roles (read-only, time-boxed)
- [ ] M3 Segregation-of-duties rules; approval delegation
- [ ] M3 Postgres Row-Level Security by company as defence in depth
- [ ] M4 OIDC SSO, SCIM, IP allow-list, org admin console, audit of admin actions

### 7.3 Chart of accounts, tafsili, dimensions
- [ ] M1 Account tree (group/kol/moein), natures, postable/active flags, codes with configurable lengths
- [ ] M1 Seeded Iranian COA templates (trading, service, manufacturing, contracting, solo) in fa/en
- [ ] M1 Tafsili types + entities, auto-tafsili for parties/banks/cash/projects/employees
- [ ] M1 Account ↔ tafsili level rules (required/optional, up to 4 levels, floating)
- [ ] M2 Dimensions + dimension sets on lines, defaults by account/party
- [ ] M3 Statutory/tax line mapping; reporting groups
- [ ] M4 Group COA mapping for consolidation; governed COA changes

### 7.4 Ledger & periods
- [ ] M1 Journal entry draft/post/reverse; immutable posted entries (DB trigger + code)
- [ ] M1 Balanced-entry invariant (deferred constraint on Pg; engine check on SQLite) + proptest
- [ ] M1 Gap-free per-FY numbering at post time
- [ ] M1 Hash chain + `verify-ledger` command
- [ ] M1 Fiscal years, periods, soft/hard locks with audited overrides
- [ ] M1 Year-end close wizard: closing temp accounts, closing entry (اختتامیه), opening entry (افتتاحیه) next FY
- [ ] M1 Opening balances import/entry with automatic balancing check
- [ ] M1 Journal templates and recurring entries
- [ ] M1 Balance tables (rebuildable) + `rebuild-balances` command equals full recomputation (test)
- [ ] M2 Accruals/deferrals, allocations
- [ ] M3 Entry approvals, period-close checklist, period 13 (audit adjustments)
- [ ] M4 Intercompany entries; parallel ledgers `⚠ scope`

### 7.5 Parties & catalogue
- [ ] M1 Customers/suppliers (person/legal), contacts, addresses, ID validations, opening balances
- [ ] M1 Items/services, units, Moadian product ID, VAT code, prices, barcodes
- [ ] M2 Price lists, customer groups, credit limits, payment terms
- [ ] M3 Multi-UoM conversions, item variants, item categories with default accounts

### 7.6 Sales & AR
- [ ] M1 Quote (پیش‌فاکتور) → invoice; invoice draft/post/void (via reversal); fa/en PDF; share link
- [ ] M1 Receipts (cash/bank/card/cheque), allocation, unapplied credits
- [ ] M1 Customer statement, AR aging
- [ ] M1 Posting rules: Dr AR / Cr Revenue, Cr VAT payable; COGS + inventory (perpetual)
- [ ] M2 Sales orders, deliveries, partial invoicing, recurring invoices, reminders (SMS/email)
- [ ] M2 Sales returns + credit notes linked to Moadian return subject
- [ ] M2 Instalment sales schedules
- [ ] M3 Discount approvals, commissions/marketers, sales targets
- [ ] M4 Revenue recognition schedules `⚠ scope`; customer portal SSO

### 7.7 Purchases & AP
- [ ] M1 Bills, quick expenses with receipt attachment, supplier payments, AP aging
- [ ] M2 Purchase orders, GRN, 3-way match, purchase returns
- [ ] M2 Flag purchases from non-Moadian sellers for the seasonal report
- [ ] M3 Requisitions with approvals; landed cost; foreign purchase tracking
- [ ] M4 Vendor portal; commitment budget checks

### 7.8 Inventory
- [ ] M1 Single warehouse, stock moves, weighted-average valuation, stock card (کاردکس)
- [ ] M2 Multi-warehouse, transfers, adjustments, stock counts, FIFO, perpetual/periodic per company
- [ ] M2 Negative stock policy (block/warn/allow)
- [ ] M3 Batches/expiry, serials, bins, reservations, landed cost, revaluation
- [ ] M3 Inventory valuation golden tests (FIFO/WAvg with returns, backdated entries → recompute)
- [ ] M4 Multi-site replenishment; consignment; MRP plugin hook

### 7.9 Treasury & reconciliation
- [ ] M1 Cash boxes, bank accounts, petty cash, transfers, receipts/payments
- [ ] M1 Bank statement import with per-bank mapping packs (Excel/CSV)
- [ ] M1 Reconciliation: suggestions, manual match, create-from-line, rules, lock reconciled lines
- [ ] M2 POS settlements & fees; cash count / close of day
- [ ] M3 Bank API adapters (plugin), auto-reconcile with confidence threshold; cash-flow forecast 30/60/180 days
- [ ] M4 Cash pooling; intercompany settlement

### 7.10 Cheques (Sayad)
- [ ] M1 Received/issued cheques: Sayad ID (16 digits) validation, fields, attachments (front/back images)
- [ ] M1 Lifecycle state machine with postings (§3.6) and guarded transitions + tests per transition
- [ ] M1 Due calendar, reminders, cheque register report
- [ ] M2 Cheque books with leaf statuses; batch operations; transfer (خرج) recording Sayad transfer
- [ ] M2 Bounced-cheque workflow; colour status capture
- [ ] M3 Guarantee cheques/promissory notes as memorandum accounts
- [ ] M3 Sayad bank-API plugin `⚠ verify availability`

### 7.11 Tax, Moadian, seasonal
- [ ] M1 Tax codes with effective dates (10 % standard 1403–1405, exempt, special) from data pack
- [ ] M1 Moadian mapping (header/body/payments), Tax ID generation (memory ID + date + serial + Verhoeff)
- [ ] M1 Signing & submission via `EinvoiceProvider` (direct API) with sandbox; idempotent outbox; status inquiry
- [ ] M1 Status timeline; error-code catalogue in fa/en with fixes
- [ ] M1 Corrective / cancellation / return wizards enforcing reference rules
- [ ] M1 Deadline tracker (days remaining before the submission limit `⚠ verify`)
- [ ] M1 VAT return worksheet (output vs input VAT by period)
- [ ] M2 Trusted-provider adapter; buyer-side cartable import + accept/reject + match to bills
- [ ] M2 Seasonal report (TTMS) export of non-Moadian transactions + reconciliation view
- [ ] M2 Withholding & contract retentions `⚠ verify rates`
- [ ] M3 Multiple fiscal memories, serial block allocation per device, bulk submission, nightly reconciliation
- [ ] M4 High-volume queue, KMS/HSM key custody `⚠ verify`

### 7.12 Payroll
- [ ] M2 Employees, contracts, pay elements from yearly data pack (labour law figures `⚠ verify` each year)
- [ ] M2 Payroll run: gross → insurance (7 % / 20 % / 3 %, with ceiling) → salary tax (1405 brackets) → net; golden tests
- [ ] M2 Payslips fa/en; bank transfer file (generic + per-bank packs)
- [ ] M2 Insurance DISK export (DSKKAR00.DBF + DSKWOR00.DBF) validated against a sample file
- [ ] M2 Salary-tax list export `⚠ verify format`
- [ ] M2 Payroll journal entry; loans/advances; leave accruals; عیدی/سنوات; settlement
- [ ] M3 Attendance import, shifts/overtime rules, multiple workshops, cost allocation
- [ ] M4 Employee self-service portal; HR plugins

### 7.13 Fixed assets
- [ ] M2 Register, categories, acquisition from bill, depreciation runs (SL, DB), disposal with gain/loss
- [ ] M2 Iranian tax depreciation table data pack `⚠ verify`
- [ ] M3 Transfers, tags, revaluation `⚠ verify`, maintenance log
- [ ] M4 Parallel books

### 7.14 Multi-currency
- [ ] M1 Currencies, rate types, manual rates; foreign-currency documents
- [ ] M2 Realised FX gain/loss on settlement
- [ ] M3 Unrealised revaluation by rate type; reporting-currency views
- [ ] M4 Consolidation translation + CTA

### 7.15 Projects, contracting, manufacturing, POS, budgets
- [ ] M2 Projects/cost centres; project P&L; time & expense billing
- [ ] M2 POS: offline sessions, barcode, receipts (80 mm), Z-report posting
- [ ] M2 Annual budgets; budget vs actual
- [ ] M3 Contracting: contracts, progress statements, advances, retentions, Moadian contracting pattern
- [ ] M3 Manufacturing light: BOM, work orders, costing
- [ ] M3 Budget versions, phasing, PO budget control, rolling forecast

### 7.16 Multi-company & consolidation
- [ ] M2 Multiple companies per install; company switcher; optional shared catalogue/parties
- [ ] M4 Group structures, ownership %, intercompany matching, eliminations, NCI, consolidated statements

### 7.17 Reporting
- [ ] M1 Trial balance (2/4/6/8 columns; levels kol/moein/tafsili)
- [ ] M1 Journal & ledger (legal print formats), account/tafsili statements
- [ ] M1 P&L, balance sheet, cash-flow (indirect), AR/AP aging, sales/purchase summaries, inventory valuation, VAT, cheque reports
- [ ] M1 Drill-down everywhere; period comparison; Excel/PDF/CSV export
- [ ] M1 Golden report tests (insta snapshots) for the demo company
- [ ] M2 Saved views; scheduled email reports; KPI dashboard API
- [ ] M3 Report builder (pivot), statement designer
- [ ] M4 Warehouse export (Parquet), BI views, read replica routing

### 7.18 Workflow, notifications, collaboration
- [ ] M2 Comments & @mentions on any record; activity feed; in-app notifications
- [ ] M2 Simple amount-threshold approvals
- [ ] M3 Multi-step approvals, escalation, delegation; email/SMS notifications; scheduled jobs UI
- [ ] M4 Policy engine; BPMN-lite plugin

### 7.19 Documents, OCR, AI
- [ ] M1 Attachments on any record with content hash
- [ ] M2 Bank-line categorisation rules + suggestion explanations
- [ ] M2 OCR spike for Persian receipts; draft bill from OCR with field confidences
- [ ] M3 Anomaly checks; close assistant
- [ ] M4 NL query (read-only, permission-scoped), self-hosted model option

### 7.20 Import/export & migration
- [ ] M1 Excel templates & import for COA, tafsili, parties, items, opening balances; validation preview
- [ ] M1 Full company export (JSON + attachments) and re-import round-trip test
- [ ] M1 Ledger-text export (hledger/Beancount)
- [ ] M2 Hesabfa Excel importer; Sepidar importer; Holoo importer `⚠ verify formats`
- [ ] M2 Solo → Team promotion (`daftar migrate`) lossless, hash chain preserved

### 7.21 Sync
- [ ] M2 Device registry & keys; change feed with `change_seq`; pull/push of commands
- [ ] M2 Provisional offline postings → server confirmation/rejection flow
- [ ] M2 Conflict records + Sync inbox API
- [ ] M2 Number-block leasing per device (invoices, Moadian serials, cheque leaves)
- [ ] M2 Chaos tests: random offline/online interleavings preserve ledger invariants (proptest state machine)

### 7.22 API, webhooks, plugins
- [ ] M1 OpenAPI 3.1 spec published; generated TS client; API reference via Scalar
- [ ] M2 API keys, idempotency keys, ETags, cursor pagination, rate limiting
- [ ] M2 Signed webhooks with retries and replay
- [ ] M2 Generic IPG and SMS adapters
- [ ] M3 WASM plugin host with capabilities; plugin SDK (Apache-2.0); sample plugins

### 7.23 Audit, backup, ops
- [ ] M1 Audit log (before/after diff) for every mutation
- [ ] M1 Automatic local backups + restore + integrity check command
- [ ] M2 Server Docker image, compose file, health/readiness, backup guide
- [ ] M3 Retention policies; auditor exports; close sign-off
- [ ] M4 Helm chart, HA guide, PITR, SIEM export, load test (10M journal lines, p95 report < 2 s)

### 7.24 Quality gates (apply to every milestone)
- [ ] Double-entry invariants proptested (random documents → posted entries always balance; reversal nets to zero; balances = recomputation)
- [ ] Golden reports pass for the demo company in fa and en
- [ ] Playwright e2e (web build) + Tauri WebDriver e2e (desktop) for the milestone's journeys
- [ ] Migrations forward-only, tested from previous release snapshot
- [ ] i18n: no hard-coded strings; fa/en complete; RTL visual snapshots
- [ ] Docs: user guide pages (fa/en) for each shipped feature; CHANGELOG
---

## 8. Claude Code build prompt

Copy everything inside the block into Claude Code at the repository root, with this file and FRONTEND-UI-GUIDE.md committed under `docs/`.

````text
You are the lead engineer building "Daftar", an open-source double-entry accounting system for Iran (and international use) that scales from a solo user to an enterprise group. Read docs/ACCOUNTING-SPEC.md and docs/FRONTEND-UI-GUIDE.md fully before writing code. They are the source of truth. When you find a spec ambiguity, write an ADR in docs/adr/ with your decision and continue. Do not stop to ask unless a decision is irreversible and has legal consequences.

FIXED STACK — do not propose or introduce alternatives:
- Backend: Rust stable, Cargo workspace, modular monolith. tokio, axum + tower-http, sqlx (Postgres AND SQLite), sea-query (+ sea-query-binder) for dynamic SQL, rust_decimal for all money, serde (decimals serialized as strings), uuid v7, thiserror/anyhow (bins only), tracing, utoipa (+ utoipa-axum, utoipa-scalar), specta + tauri-specta, proptest, insta, cargo-nextest, testcontainers.
- Desktop: Tauri v2 hosting the React app. Solo mode embeds the Rust services and SQLite in-process and the UI calls Tauri commands. Team mode and the web build call the axum HTTP API. One application-service layer serves both transports.
- Frontend: React + TypeScript + Vite, shadcn/ui with rtl: true, Tailwind, TanStack Router/Query/Table/Virtual, react-hook-form + zod, cmdk, Vazirmatn, Persian (fa) default + English (en).

REPOSITORY LAYOUT: exactly as in spec §4.1 (crates/*, apps/{server,desktop,cli,web}, data-packs/, migrations/{postgres,sqlite}/, tests/, docs/, deploy/). Enforce crate boundaries with an `xtask check-deps` command run in CI. Only crates/ledger may write journal tables. Other modules submit PostingRequest values.

NON-NEGOTIABLE DOMAIN RULES:
1. Money: rust_decimal inside a Money{amount, currency} newtype with checked ops. Never f32/f64 for amounts. Base currency IRR has 0 decimals. Store Rial and display Toman only as a preference. Postgres NUMERIC(28,6). SQLite: follow ADR-0003 (decimal TEXT or integer minor units) consistently. JSON amounts are strings. The TS side uses a decimal library and never JS number for money.
2. Ledger: every journal entry balances (sum debit = sum credit in base currency). Enforce with a Postgres deferred constraint trigger, the posting engine and property tests. Lines are non-negative with exactly one side > 0. Posted entries are IMMUTABLE (DB trigger rejects UPDATE/DELETE). Corrections are reversal entries plus a new posting, linked both ways. Numbers are gap-free per fiscal year and assigned at post time. Entries are hash-chained, and `daftar verify-ledger` validates the chain.
3. Periods: soft/hard locks and fiscal-year close/open entries per spec §5.3. Nothing posts into a hard-locked period.
4. Iranian COA: group → kol → moein accounts, plus tafsili entities as a separate multi-level dimension with per-account level rules. Seed the Iranian COA templates from data-packs/coa/*.toml (trading, service, manufacturing, contracting, solo) in fa and en.
5. Law as data: VAT rates (10 % general for 1403–1405), salary-tax brackets (1405), insurance rates (7/20/3 %), holidays and bank statement mappers live in signed, effective-dated data packs under data-packs/. Mark any value the spec flags "⚠ verify" with a `verify = true` field and surface it in an admin "law data review" screen.
6. Moadian: a tax_invoice aggregate separate from the commercial invoice. 22-char Tax ID = 6-char memory ID + 5 hex date + 10 hex serial + Verhoeff digit. Implement an EinvoiceProvider trait with Direct and TrustedProvider adapters plus a Sandbox/Mock adapter. Use an idempotent outbox, status state machine and corrective/cancel/return rules from spec §3.4. Before implementing signing/endpoints, read the latest official technical guide. If unavailable, implement against the Mock adapter and leave a TODO with ADR.
7. Cheques: Sayad 16-digit IDs, lifecycle state machine and postings exactly as spec §3.6.
8. Dates: store Gregorian DATE, display Jalali by default, all conversions through kernel::jalali with golden tests.
9. i18n: no hard-coded user strings. Rust messages use Fluent (fa/en) with stable error codes (RFC 7807). The UI uses i18next (or equivalent) with fa/en catalogs. Persian digits are on by default in fa. RTL is the default layout.
10. Migrations: forward-only, numbered, one set per dialect, run on startup (desktop) and via `daftar migrate` (server). CI tests upgrade from the previous release's snapshot DB.

MILESTONES — build in order. At the end of each milestone, tick the items you completed in docs/ACCOUNTING-SPEC.md §7 and docs/FRONTEND-UI-GUIDE.md §14 (change "- [ ]" to "- [x]" only for items that are merged with passing tests). Update CHANGELOG.md and write a short docs/milestones/Mx.md report listing done, deferred and why.

M0 Foundation
- Workspace, CI (fmt, clippy -D warnings, nextest on SQLite + Postgres, web lint/typecheck/vitest, Tauri build matrix), kernel (Money, ids, jalali, errors), storage (UoW, outbox, migrations), data-pack loader, Tauri shell + tauri-specta, Transport abstraction, generated HTTP client, demo-company generator.
- DoD: `cargo xtask ci` green locally and in CI; the app opens an empty RTL shell in fa and en; the Money and jalali proptests and golden tables pass; ADRs 0001–0007 exist.

M1 Solo MVP (offline desktop, SQLite)
- Onboarding wizard, company + COA templates, tafsili, parties, items, journal entries, sales invoices + receipts, bills + payments, cash/bank, statement import + reconciliation, Sayad cheques, VAT codes, Moadian (Mock + Direct adapter skeleton, wizards, status timeline), core reports with drill-down, Excel import/export, PDF invoices fa/en, backups, audit log, year-end close.
- DoD: a new user can go from install to a posted invoice in under 3 minutes (scripted e2e). The demo company's trial balance, P&L and balance sheet match golden snapshots. Proptests prove balancing, immutability, reversal-nets-to-zero and balances = recomputation. Backup → restore → identical hash chain. Moadian Mock round-trip covers original, corrective, cancel and return. All M1 checklist items ticked.

M2 Small business (server + team + sync)
- apps/server with Postgres, users/roles/invites, accountant and auditor access, sync protocol (commands, change feed, provisional postings, conflicts, number-block leasing), comments/mentions/activity, simple approvals, payroll (insurance DISK, salary tax), fixed assets, multi-warehouse inventory + FIFO, sales/purchase orders, returns, recurring invoices, POS, projects, budgets, TTMS export, importers (Hesabfa Excel, Sepidar, Holoo), webhooks and API keys.
- DoD: two desktop clients plus a web client edit the same company. A chaos proptest of random offline/online interleavings keeps all ledger invariants. Payroll golden tests for 5 sample employees match hand-calculated sheets. The DISK files validate against the reference sample. Docker compose up → working server in under 5 minutes. All M2 checklist items ticked.

M3 Mid-market
- Multi-step approvals with SoD, RLS, batches/serials/landed cost, contracting, manufacturing light, FX revaluation, multiple fiscal memories + bulk Moadian, bank API plugin interface, auto-reconcile with confidence threshold, anomaly checks, report builder, statement designer, WASM plugin host + SDK.
- DoD: load test with 1M journal lines (p95 list < 300 ms, TB < 2 s on reference hardware). Every approval path is covered by e2e. A sample plugin adds a report and a document hook. All M3 items ticked.

M4 Enterprise
- Consolidation (intercompany, eliminations, NCI, translation), SSO/SCIM, policy engine, HA/Helm, PITR guidance, SIEM export, Parquet export, NL query (read-only, permission-scoped).
- DoD: a 3-company group with mixed currencies consolidates to golden statements. Load test with 10M lines meets spec §7.23. A security review checklist is completed. All M4 items ticked.

TESTING STRATEGY (every milestone):
- Unit tests per crate. Property tests (proptest) for Money, allocation, posting, reversal, inventory valuation, FX and sync state machines.
- Golden tests (insta) for reports, PDFs (render to text/PNG hash), Moadian JSON payloads, DISK DBF files, Excel exports.
- DB tests against real Postgres (testcontainers) and SQLite, so dialect parity is checked by the same test suite run twice.
- E2E: Playwright against the web build, and Tauri WebDriver (tauri-driver) for desktop journeys from FRONTEND-UI-GUIDE.md.
- Never mock the ledger in tests of other modules. Use the real posting engine with SQLite in-memory.

CONVENTIONS:
- Conventional Commits. Small PRs per checklist item. Each PR includes tests and updated docs.
- Rust: no unwrap/expect outside tests and main. Domain errors are typed. Public functions are documented. Every module has `api.rs` (public), `domain/`, `repo/`, `service.rs`, `events.rs`.
- SQL: snake_case. Every table has id uuid, company_id (if business data), created_at, created_by, updated_at, version. Index foreign keys.
- API: /api/v1 resources, cursor pagination, Idempotency-Key, ETag/If-Match, problem+json with code + localized message.
- Frontend: follow FRONTEND-UI-GUIDE.md (folder structure, components, tokens, shortcuts).
- Security: argon2id, least privilege, audit every mutation, no secrets in logs, telemetry off by default.

Start now with M0. Work milestone by milestone, keep the checklists truthful, and print a summary of ticked versus remaining items at the end of each milestone.
````
---

## 9. Sources

All sources were accessed on 2026-11-03 or shortly before. Prices change, so re-check them before quoting.

**Global**
- [S-G1] QuickBooks price changes (effective 2026-08-01): https://quickbooks.intuit.com/r/product-update/quickbooks-price-changes/ · accountant FAQ: https://www.firmofthefuture.com/product-update/quickbooks-price-changes/ · before/after table: https://getstackwatch.com/quickbooks-online-price-increase-2026
- [S-G2] Xero US pricing: https://www.xero.com/us/pricing-plans/ · Oct-2026 increase: https://www.booksla.com/xero-price-increase/ · https://getstackwatch.com/xero-pricing-2026
- [S-G3] Capterra, QuickBooks Online reviews: https://www.capterra.com/p/190778/QuickBooks-Online/
- [S-G4] NerdWallet QBO review 2026: https://www.nerdwallet.com/business/software/reviews/quickbooks-online · bank-feed pitfalls: https://www.mbgcpa.com/pros-cons-quickbooks-online/
- [S-G5] Xero invoicing plans/reminders: https://charminginvoices.com/blog/xero-invoicing
- [S-G6] Wave pricing: https://www.waveapps.com/pricing · https://support.waveapps.com/hc/en-us/articles/208621296-Is-Wave-free
- [S-G7] Puzzle AI Close: https://puzzle.io/ai-close · https://puzzle.io/blog/puzzle-launches-ai-suite
- [S-G8] Digits: https://digits.com/ · https://digits.com/blog/digits-vs-puzzle/ (vendor-authored comparison)
- [S-G9] QuickBooks pricing page: https://quickbooks.intuit.com/pricing/
- [S-G10] G2, QuickBooks Online reviews: https://www.g2.com/products/quickbooks-online/reviews
- [S-G11] Odoo vs ERPNext: https://www.reddit.com/r/Odoo/comments/1raf6nu/odoo_erpnext_2026_updated_comparison/ · https://www.capterra.com/compare/135618-164441/Odoo-vs-ERPNext · https://www.reddit.com/r/selfhosted/comments/1jr258e/frappe_frameworks_erpnext_is_it_really_this_good/
- [S-G12] Akaunting licence (BSL 1.1): https://akaunting.com/license · reviews: https://www.capterra.com.sg/software/1012064/akaunting
- [S-G13] Zoho Books pricing: https://www.zoho.com/us/books/pricing/
- [S-G14] GnuCash discussion: https://news.ycombinator.com/item?id=20109545 · Beancount vs GnuCash: https://beancount.io/compare/beancount-vs-gnucash
- [S-G15] shadcn/ui RTL: https://ui.shadcn.com/docs/rtl · https://ui.shadcn.com/docs/changelog/2026-01-rtl · https://ui.shadcn.com/docs/rtl/vite
- Product homepages for the remaining rows (features only, no prices claimed): https://www.freshbooks.com · https://www.sageintacct.com · https://www.netsuite.com · https://www.sap.com/products/erp/business-one.html · https://www.microsoft.com/dynamics-365/products/business-central · https://www.odoo.com · https://erpnext.com · https://www.dolibarr.org · https://invoiceninja.com · https://gnucash.org · https://hledger.org · https://beancount.github.io · https://www.pennylane.com

**Iran: products**
- [S-IR1] Sepidar 1405 price list and licensing terms: https://irhesab.com/price-list-sepidar/
- [S-IR2] Hesabfa pricing: https://hesabfa.com/Prices
- [S-IR3] Sepidar vs Hesabfa comparison (Hesabfa features, no offline, metering): https://mali365.ir/blog/sepidar-vs-hesabfa/
- [S-IR4] Mahak 1405 prices and add-ons: https://hesabme.com/قیمت-نرم-افزار-حسابداری-محک/
- [S-IR5] Sepidar scale and cloud offering (same article as S-IR3): https://mali365.ir/blog/sepidar-vs-hesabfa/
- [S-IR6] Rahkaran: https://www.systemgroup.net/products/rahkaran/
- [S-IR7] Rahkaran review: https://mali365.ir/software/rahkaran/
- [S-IR8] Holoo prices (`⚠ verify`; page timed out): https://www.hadiran.ir/hesabdari-holoo-pricelist/ · https://www.holoocenter.com/price-list/

**Iran: law and systems**
- [S-IR9] Moadian technical/accounting docs (IDs, API, types, subjects): https://gandom.dev/docs/fa/accounting/tax/moadian/
- [S-IR10] Corrective/reference invoices: https://mymotamed.ir/article/صورتحساب-ارجاعی-در-سامانه-مودیان/
- [S-IR11] Sayad cheques (registration, confirmation, transfer, colours): https://www.sb24.ir/saman-mag/sayad-cheque
- [S-IR12] Sayad in accounting software: https://www.sepidarsystem.com/blog/sayad/ · https://hesabshop.com/blog/sayadi-cheque-register-confirm-transfer
- [S-IR13] Insurance DISK files and rates: https://baje724.ir/blog/list-disk-va-disket-bime/ · https://tavafi.ir/post/dsk/
- [S-IR14] VAT circular (budget-law rate): https://dl.iranimpex.com/Bakhshnameh/9507_iranimpex-com.pdf
- [S-IR15] VAT 1405, 10 % vs 12 %: https://qavaed.ir/blog/vat-rate-1405-iran-10-or-12-percent
- [S-IR16] VAT 1405 guide: https://zharfban.com/guides/vat-1405/
- [S-IR17] Budget law 1404 tax rulings: https://finotax.co/summary-of-the-tax-rulings-the-budget-law-1404/ · https://irannewspaper.ir/8925/2/144691
- [S-IR18] VAT split and exemptions 1405 (public-sector PDF): https://www.abfa-alborz.ir/Dorsapax/Data/Sub_0/File/moafiat1405.pdf
- [S-IR19] Tax ID structure FAQ: https://taksatsp.ir/سوالات-متداول/
- [S-IR20] Invoice date fields / instruction v7.9: https://rima.tax/invoice-dates/
- [S-IR21] Moadian overview, deadlines, types: https://maliaty.com/سامانه-مودیان/ · https://orash.ir/blog/store-tax-system-2/ · https://maahertsp.com/moadian-system/
- [S-IR22] Reference invoice rules: https://mymotamed.ir/article/صورتحساب-ارجاعی-در-سامانه-مودیان/
- [S-IR23] Deadlines/penalties: https://limotx.ir/article/67
- [S-IR24] Seasonal report vs Moadian: https://ezhesab.com/store-terminals-and-payers-system-law-7/
- [S-IR25] Seasonal transactions report: https://pishdadacc.com/seasonal-transaction-reports/
- [S-IR26] TTMS: https://www.sepidarsystem.com/blog/ttms/
- [S-IR27] Cheque journal entries: https://darik.app/sayad-cheque-accounting-entry/ · https://pejvakamooz.ir/cheque-registration-prince-software/
- [S-IR28] DSKKAR file fields: https://baje724.ir/file-dskkar-bime/
- [S-IR29] Salary tax 1405: https://rayafintech.com/blog/salary-tax-1405/
- [S-IR30] Salary tax 1405: https://wiraf.ir/guides/salary-tax-1405 · https://chargoon.com/blog/salary-tax-calculation/

### Open "⚠ verify" items (resolve before the relevant milestone)
1. Moadian buyer-reaction window (30 vs 12 days) and the late-submission penalty amount.
2. Moadian submission deadline when using a trusted provider (12 vs 15 days) and its current effective date.
3. Moadian API version, signing/encryption scheme and endpoints (official technical guide); product-ID format and checksum.
4. The VAT 10 % split by beneficiary (public-sector circular only).
5. 1405 salary-tax brackets against the published budget law/circular. Labour-law yearly figures (minimum wage, allowances).
6. Insurance ceiling/floor for 1405, special-sector rates, DISK DBF encoding.
7. Holoo licensing (dongle vs SLM/account card) and prices. Invoice Ninja's current licence.
8. Iranian tax depreciation table, contracting retention/withholding rates, official FX rate system names.
9. Jalali crate choice, Typst Persian shaping quality, OCR engine for Persian (technical spikes).
