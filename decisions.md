# Decision register — 2026-10-07

Only explicit user choices and delegation are confirmed. The latest delegation permits visible, versioned guide-based core accounting defaults; the documents alone do not authorize external actions, live data, credentials, paid services or license changes. Accounting fixtures are synthetic technical expectations; professional production acceptance remains pending.

| ID | Status | Facts / proposal | Dependency and next decision owner |
| --- | --- | --- | --- |
| DEC-01 Initial market | Broad vision confirmed; launch boundary remains staged | User answered **“anyone”**: retain broad audience/product vision. Do not silently narrow the vision to service-only or advertise this first checkpoint as universal accounting coverage. | Dependency-ordered milestones remain necessary. G3 inventory/packs/cheques is required before trading/wholesaler completeness claims; sector-specific launch requirements still need explicit scope. |
| DEC-02 Free and funding | Core goal confirmed; funding pending | User explicitly wants a free offline-first application. No subscription/record quota/mandatory account/paywall for local export is a blueprint goal. Optional external-service funding is undecided. | User owns any funding/pricing/external-cost decision. No paid infrastructure introduced. |
| DEC-03 Source/distribution license | Pending user review | Existing `Cargo.toml` says `MIT`; existing `LICENSE` is MIT. This is repository metadata already present, not a newly selected product license. Bundled Vazirmatn has its own SIL OFL 1.1. | Dependency/font/SBOM review before user confirms future source/distribution terms; do not change license metadata silently. |
| DEC-04 Accounting and stock policy | Guide-based implementation delegated; professional production review pending | Latest user reply authorizes using the guide to build proper core accounting without further individual policy questions. Implement its proposed accrual/perpetual, exact IRR, moving-weighted-average and original-cost-return design as explicit versioned engineering policies with independently expected fixtures; expose assumptions and configuration. | This delegation permits dependent core development; it does not supply accountant certification. Professional review remains necessary before production accounting claims. Never reinterpret existing posted history or quotation snapshots. |
| DEC-05 Platforms/collaboration | Native stack confirmed; launch support pending | User explicitly confirms Rust + GTK4 + stable libadwaita; no React/Svelte/Tauri migration. Existing environment is Linux/X11. Windows/multiuser/mobile/cloud are separate scopes. | Confirm supported releases/hardware before public support claims; no shared/cloud-synced SQLite workaround. |
| DEC-06 Release authority | Pending user | Independent technical review is required by user. Accountant/tax specialist acceptance is not supplied by code tests. | User names product/accounting/security/statutory acceptors; production/static tax claims stay blocked. |
| MNY-IRR Canonical currency | Confirmed by user during this cycle | Canonical **integer IRR**, exact **1 toman = 10 rials**, and **fractional rial input rejected**. Current quote value/precision remains preserved. | Implement bounded exact signed journal amounts with checked range and explicit units; presentation conversion never changes stored economic value. This approval does not select tax/stock/recognition/rounding policies. |
| MNY-ROUND Calculation policy | Order/receipt/bill allocations evidenced; returns pending | Current quotation rounding remains unchanged. The next core version explicitly defines quantity thousandths, positive half-up, stable largest-remainder document discount allocation and cumulative original-value returns; see the core policy proposal. New core tax is disabled. | Version and independently fixture-test new policies; never reinterpret old totals. Professional production review remains pending. |
| INV-VAL Stock valuation/backdating | Receipt/transfer costing evidenced; sales/returns pending | The core policy proposal specifies per-company/item/warehouse moving weighted average, whole-rial residuals, original-cost returns, blocked negative stock and nondecreasing stock dates. | Implement checked quantity/value controls, same-date commit ordering, late-cost boundaries and lock enforcement before claiming G3. Professional production review remains pending. |
| IR-STAT Statutory applicability | Deferred by current user instruction | Vendor taxpayer-system claims are not tax authority schemas/rates/deadlines/certification. No current legal rule is selected. | Obtain effective primary technical/legal sources and entity applicability before G5; credentials/submission require separate authorization. |
| PERF-BUDGET Hardware/latency | Proposed, not accepted target proof | Blueprint targets four-core x86-64/8 GB/SSD with 100k documents/1m journal lines/50k items/10k parties/5 years. Current quote fixture is smaller and cloud-specific. | Declare actual measured hardware/datasets/build, distinguish cold definitions and latency boundaries; user accepts any revised budget. |
| DATA-CAP Existing 64 MiB bound | Baseline limitation | Quote aggregate cannot grow beyond a restorable 64 MiB JSON envelope. This is current safety behavior, not a free-core quota decision. | Future indexed transactional accounting schema must scale without quietly imposing a product record quota. Migration/restore rehearsal required. |

Historical first checkpoint authorization: preserve and reproduce the native quote baseline; fix reproduced save/focus defects; implement the bounded generic company/custom-chart/fiscal/opening/manual-journal/duplicate/reversal/locks/trial-balance workflow. Integer IRR was approved. The latest delegation below additionally authorizes visible guide-based core defaults; the earlier pending-policy boundary does not block that implementation.

No push, merge, deployment, billing, credential creation, security setting change, customer message or live financial data is authorized in this checkpoint.

Latest continuation steering: target trading businesses/wholesalers and small individual sellers; fully redesign the native sidebar and provide Jalali fiscal-year/date input and display. User asks 6.1-sol/high agents; runtime allows seven active slots including the integration lead. The user subsequently delegated core policy choices to the guide and asked to leave actual legal/tax integrations aside. Preserve advanced/statutory roadmap entries as deferred, keep the native/offline/free goal, and continue dependency-ordered usable core workflows. This grants no push/merge, deployment, external financial action, credentials, paid service, security-setting or license authority. Existing generic quotation tax/addition snapshots are preserved.

M3 implementation boundary: a fresh-company guided chart exposes the versioned policy before saving; names/codes are editable, timezone is explicit, role classifications/control gates are validated, and the full setup is atomic. All twelve protected account checks are explicit at opening: ten detail registers and zero-only COGS/inventory-loss controls. New core tax remains disabled. Reconciliation is certified only for the beginning-of-day opening boundary; no later-day or live-bank agreement is inferred. Ordinary posting before that required opening, manual control bypass, a second opening, policy remapping and unpaired generic opening reversal are rejected. Existing custom books remain manual and unchanged. The richer 349 cash contribution, moving average/returns, settlements and cheque operations remain later workflow requirements rather than generic control-account journals.

M4 bounded implementation uses the delegated `core-policy-v1` engineering rules,
without claiming professional approval. A purchase order freezes selected prices,
unit ratios and deterministic discounts without a journal or stock movement.
Valued receipts debit Inventory and credit GRNI; supplier bills debit the exact
remaining received cost and credit AP. Partial allocations use the cumulative
original value and give the final part its exact residual. Per-line price/cost
differences are rejected until a separately evidenced correction policy exists.
New core tax remains disabled. Opening GRNI can be billed without recreating stock.

Warehouse transfers use per-pool moving weighted average and paired quantity/
value movements. Their company-wide financial effect is zero, so they record a
nonfinancial source event and audit without a journal. Zero-valued receipts/bills
also retain their business effect without inventing a financial entry. Negative
stock and company-wide stock backdating are blocked; equal-date events follow
durable audit sequence. Operational reports disclose end-of-day cutoff, while
the immutable M3 report continues to show beginning-of-day opening balances.
These rules are explicit engineering defaults under the user's delegation;
accountant review remains a production gate. Posted cancellation, cost variance,
direct expense/asset bills, purchase returns, sales and settlement remain tracked
dependent workflows and are not claimed by M4.

M4's unposted purchase rows are explicitly staged in the native editor; they are
not persisted editable drafts. Failed previews/commits retain safe input, and
posted sources survive restart/restore. Closing/discarding an unposted editor
cannot be represented as a saved document. Durable editable purchase drafts and
the common linked document lifecycle remain a tracked UX-08/SAL-01 dependency;
this bounded checkpoint does not claim draft restart support.

Duplicate identity is explicit: the retained generic manual-ledger boundary uses
company/source revision/economic event, as specified by ACC-03. It does not
establish a safe generic document-amendment workflow. M4's typed procurement
sources enforce one effect per company/source/event across all revisions and
also check external supplier references against both opening and operational
documents. Corrections still require an independently evidenced linked workflow;
no universal cross-revision correction or duplicate guarantee is inferred from
generic manual journals.

M4 migration preserves historically unrestricted manual event text. A retained
M3 manual journal named `procurement.receive` or `procurement.receive.v1` remains
a manual journal, with its original requests, aliases, audit and linked reversal.
The actual immutable typed source association controls protected reversal and
native source navigation; an event-name prefix alone does not grant capability.
Replay accepts that existing text with ordinary account safeguards. New manual
commands and new persistence appends cannot impersonate a reserved typed event.
Independent checks use the archived M3 implementation to create and recover the
old histories, rather than assuming a synthetic current profile proves legacy
compatibility. This change introduces no new accounting policy or schema field.

M4 native acceptance requires correct GTK ownership and keyboard behavior. Freeze1 exposed ActionRows outside their required ListBox parent, disabled no-op pagers trapping focus, and GTK 4.22 traversing detached children of collapsed quotation Expanders. The final source uses native containers, useful paging actions and explicit collapsed-child visibility. Quotation catalog validation clears focus before disabling its fields; nested dialogs preserve usable weak return-focus targets and all edited inputs. Nonhomogeneous workspace sizing and one purchase/inventory scroll surface expose complete focused controls in short windows.

Freeze2 passes native purchase30 and short-window8, preserved core27/Jalali44/sidebar24/manual-ledger16/full quotation23/save-close3, and independent quotation28/disclosures4/nested Back-Cancel3/validation-recovery3. Actual final application logs contain no GTK warnings/criticals. Financial history remains exact after keyboard cancellation/restart. [Final native evidence](docs/assets/accounting-m4/native/freeze3/README.md) and [quotation focus evidence](docs/assets/accounting-m4/quotation-focus/README.md) retain before failures and final results separately; independent review and measured performance have their own gates.

Final freeze3 fixes the two subsequent independent presentation findings: successful or failed procurement refresh replaces the global progress message, and human resource-report roles use Persian بله/خیر while canonical snapshots retain their original booleans. Only those two presentation files differ from freeze2; [the exact patch](docs/assets/accounting-m4/checks/freeze3-presentation.patch) preserves the source boundary. Fresh final3 Rust gates, purchase30, short-window8, eight actual refresh-completion checks, quotation23 and independent financial/recovery14 phases pass. Freeze2 preserved regression/quotation focus suites remain attributed to their original executable and byte-identical unaffected sources. Performance and full-roadmap scope have separate evidence; no full-product acceptance follows.

Final M4 freeze4 retains the same financial contract and schema. The additional bounded native fixes send unmodified forward Tab from current nonempty document search to the first usable filtered detail action, while preserving default/modified/backward/stale/master routes, and prepare search indexes for copied catalogs over 1,000 products off the GTK thread. Weak dialog lifetime discards read-only index results on close; no source catalog or saved state is written. Product cloning, bounded GTK row creation and existing edit/delete reindexing remain main-thread work; this does not establish general large-catalog performance. Actual final timing and acceptance limits are recorded in [performance-M4.md](requirements/performance-M4.md).

Final M4 freeze5 clears procurement worker loading messages on success, rejection, stale results and closed dialogs; it does not change validation, posting or durable state. Catalog-editor plain Escape applies only to its current top-level closable dialog, with modifier, nested-dialog and popup guards. Opt-in keyboard diagnostics now query the visible dialog's actual focus before a labeled window fallback; unavailable window-relative coordinates stay null. The exact [freeze4-to5 UI patch](docs/assets/accounting-m4/checks/freeze4-to5-ui.patch) verifies these three changed UI files against the original freeze4 hashes. Earlier window-focus diagnostics do not establish actual modal focus. Controlled worker-delay cancellation tests are separately identified and do not establish natural timing.


## M4 final picker persistence boundary

The freeze5 scale/source diagnosis found that quotation product/customer picker
opening and explicit stored choices still performed synchronous draft saves.
The bounded correction uses the existing persistence gate and worker: current
validated draft plus an exact product/customer snapshot are one prepared,
generation-checked atomic candidate. A stored product choice verifies catalog
ID/fingerprint/index/code and original line request; a customer choice verifies
the current complete active record. A pending accepted choice disables search,
list, Choose and Back after clearing the actual Dialog focus, and prevents close
until its definite receipt. Errors restore controls and preserve the query for
retry. An old receipt cannot replace newer input; customer identity widgets are
updated only when both raw pre-choice texts remain untouched. Newer valid edits
are saved separately; newer invalid text stays visible and cannot commit.

The opt-in scale observer awaits actual picker/result and durable-choice native
readiness, preserving its original search/warm-open endpoints and strict external
budgets. Dispatch and choice readiness are separate measurements. This changes
no accounting policy, schema, money arithmetic, journal or procurement command.
Other synchronous commands and rendering limits remain tracked; this is not an
assertion that every expensive GTK path has been eliminated.
