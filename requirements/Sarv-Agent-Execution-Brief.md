# Sarv agent execution brief

Attach `Sarv-Accounting-Requirements.md` to the agent conversation, then paste the instructions below. This is a staged implementation brief; attaching it does not make the whole roadmap one release.

---

Build Sarv toward a free, Persian-first, offline accounting product with excellent native UX and a reliable Rust core. Read the attached `Sarv-Accounting-Requirements.md` fully and use its requirement IDs, evidence rules and release gates as the implementation contract. Preserve the working quotation app and do not claim universal competitor parity, tax compliance or superior speed without evidence.

Repository: https://github.com/Fearlex-creator/sarv-quote
Reported published baseline: c21c4ed
Reported later local commit: 4a1dbf7
A newer UI pass may be active or unmerged. Verify actual HEAD, branch, dirty state, schema and concurrent work before touching files. Coordinate with the existing task; never reset or overwrite it. Existing implementation is Rust, GTK4 and libadwaita. Keep it native and preserve verified quotation, search, catalog-to-quote price selection, settings, import and PDF behavior.

Start with Gate G0: inspect, run and measure the current app, verify what actually works, record gaps and propose the first bounded accounting milestone. Work through its approved scope to evidence-backed acceptance. The remaining roadmap stays tracked for later checkpoints; do not silently attempt the entire accounting suite. Ask only for decisions or authority that genuinely block dependent work, especially initial market, accounting/valuation policy, licensing and external costs.

Use this loop for each vertical slice:
1. Research the exact workflow using official sources and named editions. Keep facts, assumptions and proposals separate.
2. Capture the baseline, requirement IDs, dependencies and acceptance fixtures before changing code.
3. Implement the smallest complete workflow with stable domain/UI boundaries and migrations that preserve existing data.
4. Run arithmetic, state-machine, database, migration, regression and native workflow tests. Render and inspect real GTK screens and generated PDFs.
5. Have an independent reviewer reproduce the result, inspect evidence and challenge edge cases. Fix material defects, rerun affected checks and create a clean checkpoint.

Use up to ten agents only if runtime capacity and independent work justify it. Give each bounded ownership and clear interfaces; one owner controls shared schemas and migrations, and one integrator manages commits. Useful workstreams include accounting kernel, persistence, native UX, reporting/import, localization/accessibility, performance and independent QA. Avoid overlapping file edits and empty feature menus.

Non-negotiables: exact checked money arithmetic; explicit rial/toman and rounding rules; balanced atomic and idempotent posting; immutable posted effects with linked corrections; original-cost return policy; quantity and value ledgers; partial settlement/advances; enforced period locks; and AR/AP, inventory, cheque and bank reconciliation. Approve golden fixtures with an Iranian accountant before production use. No live accounting data experiments without a protected copy and a tested recovery path.

Core scope is phased across ledger, sales/receivables, purchases/payables, inventory, cash/bank/cheques, reports, permissions, backup/restore, settings and Persian localization. Manufacturing, POS, payroll, assets, FX, multiuser/sync, industry packs, mobile/cloud and statutory adapters have separate prerequisites and acceptance gates in the blueprint. Do not omit them from the backlog or represent them as shipped.

For UX, prove the fast searchable product-and-price picker, party/item search, keyboard muscle memory, focus preservation, readable Persian/RTL forms, clear states, narrow-window behavior and correct printed amounts. Verify font licenses before redistribution. Keep heavy work off the GTK main thread. Measure cold/warm latency, p95, memory and large-dataset behavior on declared hardware; Rust is not performance evidence.

Maintain `requirements/evidence.md`, `decisions.md`, `migrations.md` and a fixture index. Each completed requirement needs a commit, repeatable command, expected/actual outcome, screenshot/report where relevant, reviewer and remaining limits. Say untested when untested. Test counts and checked boxes alone are insufficient.

Do not create paid services, credentials, live financial or government actions, production-data sharing, destructive changes or public deployments without explicit approval. Keep secrets out of code, logs, screenshots and prompts. Do not choose the source license or free-product funding model silently.

Stop at the accepted milestone or an exact blocker, not at the first build and not after endless polishing. Deliver the working commit/build, requirement evidence, native screenshots, sample PDFs, measured performance, migration/restore proof and precise remaining gaps. Then propose the next bounded gate.
