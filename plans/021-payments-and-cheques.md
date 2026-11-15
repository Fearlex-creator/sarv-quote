# Plan 021: Payments can be reversed; cheques can be endorsed, registered at opening, and tracked by due date

> **Executor instructions**: Follow this plan step by step. Run every verification command. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-desktop/src/ui/views/payments.rs crates/sarv-desktop/src/ui/views/overview.rs crates/sarv-quote/src/desktop_bridge.rs crates/sarv-quote/src/cheques.rs`

## Status

- **Priority**: P2
- **Effort**: M
- **Risk**: LOW–MED
- **Depends on**: 004, 010, 013 (confirm dialog, strict cheque keys), 014 (date field)
- **Category**: direction (feature)
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

Cheques are the main credit instrument for Iranian small businesses, and settling invoices is daily work. The engine has four capabilities the desktop does not expose:

- **Payment reversal** — the README promises "a typed mirrored reversal". Today a wrong allocation can only be undone by reversing the raw journal, which bypasses the receivable/payable subledger.
- **Endorsing a received cheque to a supplier** (خرج کردن چک) — the UI shows the action as a dead label.
- **Registering cheques that existed before the books were opened.**
- **Due dates** — the cheque list shows number and status only, capped at 8 rows; nothing tells the owner which cheques fall due this week.

## Current state

- Engine (`crates/sarv-quote/src/desktop_bridge.rs`):
  - `payment_reverse_preview` (`:1622-1629`): payload `{company_id, payment_id}` → preview JSON.
  - Reversal post: `fn payment(s, p)` (`:1012-1018`) — when `p["payment_id"]` is a string, `payment_post`/`payment_preview` build `PaymentCommand::Reversal { context, payment_id }`. So the post is `payment_post` with `payment_id` plus the usual context (`request_key`, `source_id`, `date`, `reason`, `reference`, `expected_generation`).
  - `cheque_spend` (`:1268-1273`): `cheque_payment(&store.state, &p, "make", "cheques_receivable")` + `cheque_id` → `spend_cheque`. It takes a payment-shaped payload on the **make** (pay supplier) side: `party_id`, `allocations`, `advance_amount`, `reference`, `reason`, context. Read `fn cheque_payment` (~`:1040-1062`) for the exact fields.
  - `cheque_register_opening` (`:1274-1280`): context + `cheque_details(&p)` + `opening_cheque_id`. Read `register_opening_cheque` (`crates/sarv-quote/src/cheques.rs:258…`) to learn what `opening_cheque_id` must refer to (an opening-balance cheque line created by the opening import) and when it is allowed.
  - Engine refuses reversing a cheque-backed payment and a second reversal (see `crates/sarv-quote/src/payments.rs:487…` `preview_payment_reversal`).
- Desktop (`crates/sarv-desktop/src/ui/views/payments.rs`):
  - Payment dialog Save (`:255-360`): builds `{side, party_id, funds_role, reference, reason, source_id, request_key, advance_amount, allocations}`; previews with `payment_preview`; posts `payment_post`, `cheque_receive` or `cheque_issue`.
  - Cheque payload (`:327-336`): `{"sayad_id", "number", "bank", "branch": "", "due_date", "drawer_national_id": "", "purpose": "", "notes": ""}`.
  - `cheque_strip` (`:426-…`): `rows.iter().take(8)`; per row: number, status badge, action buttons from `entry["allowed"]`. `cheque_dispatch_action` (`:396-406`) maps `deposit/clear/bounce/return/void`; `spend` is not mapped (a comment marks it "separate flow").
  - Payments table rows come from `s.books.list("payments")`; selection stored in `st.payments.selected` (check the field name).
  - Workspace cheque entries: `entry["cheque"]["id"]`, `entry["cheque"]["details"]["number"|"due_date"|"sayad_id"|"bank"]`, `entry["status"]`, `entry["allowed"]`; amount field — print one entry to find it.
- Validators available to the desktop (it already links the engine crate): `sarv_quote::iran::ids::sayad_id(&str) -> Result<(), IdError>`, `sarv_quote::iran::ids::normalize_digits`.
- ADR-0003 (`docs/adr/0003-advisory-identifier-validation.md`): Iranian identifier checks are advisory; stored contact data stays free text. For the Sayad id specifically, check whether the **engine** already enforces its shape on `cheque_receive` (read `cheque_details`), and match that.
- Overview cards: `crates/sarv-desktop/src/ui/views/overview.rs` builds metric cards with `ui.metric(key, glyph, title, value, sub, tone)` in an `overview-metrics` row.
- Conventions: confirm dialog `Confirm` (plan 013), `st.mint_key(prefix)` (plan 004), `ui.date_field` / `st.form_date` (plan 014), `State::run` toasts errors.

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |
| Engine tests | `cargo test -p sarv-quote` | all pass |

## Scope

**In scope**: `crates/sarv-desktop/src/ui/views/payments.rs`, new `src/ui/views/cheques.rs` if the file grows past ~600 lines (then add to `views/mod.rs`), `src/ui/views/overview.rs` (one card), `src/ui/views/setup.rs` only if opening cheques are entered there, `src/fa_messages.rs`, `src/main.rs` (tests). Engine: read-only, except adding a `"due_cheques"` summary to the `workspace` or `report` response if Step 4 needs it.

**Out of scope**: cheque books / leaf ranges, batch actions, bank API plugins, guarantee cheques (listed as "Not built" in ADR-0005); payroll.

## Git workflow

- Branch `advisor/021-payments-and-cheques`; commits `feat(desktop): reverse a payment`, `feat(desktop): endorse a received cheque`, `feat(desktop): cheque register with due dates`, `feat(desktop): cheques-due card`.

## Steps

### Step 1: Reverse a payment

Header action **Reverse payment** on the Payments page, enabled when a payment row is selected. Handler: `st.query("payment_reverse_preview", json!({"payment_id": id}))`; on error toast it (the engine explains cheque-backed/already-reversed cases). On success show `Confirm` (danger) listing party, amount and the journal lines from the preview. Confirm posts `st.run("payment_post", json!({"payment_id": id, "request_key": key, "source_id": key, "reason": st.fmt.t("Payment reversal"), "reference": key, "expected_generation": preview["_generation"]}), Some("Payment reversed"))` with `key = st.mint_key("payrev")`.

Mark reversed payments in the table (a muted "Reversed" badge) if the record JSON says so — print a reversed record to find the field.

### Step 2: Endorse a received cheque

Map the `spend` allowed-action to a button "Endorse to supplier". It opens a form dialog: supplier picker (reuse the party combobox from the payment dialog, suppliers only), then the supplier's open bills with allocation exactly like "Pay bill" (reuse the allocation-building code from the payment Save handler by extracting it into `fn build_allocations(st, side, party_id, amount) -> (allocations, advance)`); amount is fixed to the cheque amount and shown read-only. Save: preview with `payment_preview` using the same payload plus `cheque_id` if the engine supports previewing a spend (read `cheque_payment`); otherwise use the workspace `_generation`. Post `cheque_spend` with `cheque_id` + payment fields + minted key.

### Step 3: A real cheque register

Replace the 8-row strip with a "Cheques" tab on the Payments page (segmented control: Payments | Cheques): a kit `Table` with columns number, Sayad id (wrapped in `i18n::isolate`), party, bank, amount (`fmt.money`), due date (`fmt.date`), direction (received/issued), status. Default order: due date ascending, open cheques first. Row actions stay as buttons in a details area under the table for the selected cheque (the existing allowed-action buttons, now behind plan 013's confirm).

In the payment dialog's cheque fields: use `ui.date_field` for the due date; add fields for branch and drawer national id (currently sent as empty strings); validate the Sayad id with `sarv_quote::iran::ids::sayad_id(&normalize_digits(&typed))` and show the result as a hint under the field ("A Sayad ID has exactly 16 digits." — key already in `fa_messages.rs`). Block or warn according to what the engine enforces (see "Current state").

### Step 4: "Cheques due" on Overview

Add a metric card: count and total of **open** received cheques due within 7 days and overdue (due date < today, status not cleared/bounced/returned/void), and the same for issued cheques, e.g. title "Cheques due this week", value = total amount, sub = "{n} to collect · {m} to pay". Compute from `workspace["cheques"]` by filtering status and comparing ISO date strings with `crate::data::today()` — this is presentation filtering, not accounting. Clicking the card navigates to Payments → Cheques tab. If the workspace list of cheques is capped (check for `.take(` near `"cheques"` in `desktop_bridge.rs`), add an engine-side summary instead and use that.

### Step 5: Opening cheques

Read `register_opening_cheque` and the opening CSV format (`docs/opening-csv.md`, kinds `cheques_receivable` / `cheques_payable`). If opening cheque balances can only come from the CSV import path (not from the simple cash+bank opening the Rune setup offers), implement: in the Cheques tab, when the engine reports unregistered opening cheque lines (find how — e.g. a list in the workspace or in the setup record), show a "Register opening cheque" action that opens the cheque-details form and posts `cheque_register_opening`. If no such lines can exist in books created by the Rune setup, do **not** build UI for it; record that finding in your report and in `plans/README.md` instead.

### Step 6: Strings and tests

Persian entries for all new keys (`("Reverse payment", "برگشت پرداخت")`, `("Payment reversed", "پرداخت برگشت خورد")`, `("Payment reversal", "برگشت پرداخت")`, `("Endorse to supplier", "خرج چک نزد تأمین‌کننده")`, `("Cheques due this week", "چک‌های سررسید این هفته")`, `("Reversed", "برگشت‌خورده")`, …).

Tests (`main.rs`; patterns `test_payment_allocates_across_open_items`, `test_cheque_strip_labels_are_translated_keys`):

1. `test_payment_reversal_restores_the_open_invoice` — pay an invoice, reverse, the invoice is open again with the same remaining amount; reversing again shows one error toast.
2. `test_received_cheque_can_be_endorsed_to_a_supplier` — receive a cheque from a customer, create a supplier bill, endorse; the bill is settled and the cheque status changed.
3. `test_cheque_register_sorts_by_due_date`.
4. `test_overview_counts_cheques_due_within_a_week` (uses the fixed test clock from plan 002).

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass.

## Done criteria

- [ ] `grep -rn '"payment_reverse_preview"\|"cheque_spend"' crates/sarv-desktop/src` → both present
- [ ] `grep -n "take(8)" crates/sarv-desktop/src/ui/views/payments.rs` returns nothing
- [ ] Desktop suite passes with 4 new tests
- [ ] `plans/README.md` updated (including the Step 5 finding)

## STOP conditions

- `cheque_spend` cannot be previewed and the engine applies it without any generation check: still implement with confirm, but report it so the engine can gain a preview.
- Reversal of a payment that carried an advance leaves the party balance different from before the payment — stop and report (engine issue).
- The engine's cheque status names differ from those in `cheque_status_label` (payments.rs) — use the engine's; do not invent statuses.

## Maintenance notes

- The cheques-due computation must stay a filter over engine data; if it ever needs amounts netted or statuses inferred, move it into the engine.
- Cheque books, batch deposit and a due-date calendar view are the next cheque features (ADR-0005 "Not built").
