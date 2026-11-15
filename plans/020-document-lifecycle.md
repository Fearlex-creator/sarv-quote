# Plan 020: Quotations, sales and purchase orders can be continued — edit, convert, receive, bill, deliver and return in part

> **Executor instructions**: Follow this plan step by step. Run every verification command. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-desktop/src/ui/ops.rs crates/sarv-desktop/src/ui/views/sales.rs crates/sarv-desktop/src/ui/views/purchases.rs`

## Status

- **Priority**: P1
- **Effort**: L
- **Risk**: MED (quantity units; multi-step posting)
- **Depends on**: 004 (keys), 013 (confirm + `review_operation`), 014 (dates) — land them first
- **Category**: direction (feature)
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

The desktop can only run each document flow in one shot, at full quantity, at creation time:

- A **purchase order** saved without the "Receive & bill" switch can never be received or billed from the UI.
- A **quotation** can be issued and printed, but not reopened, edited, converted into a sale, or deleted. The README promises "converting to an order keeps the exact line prices".
- **Delivery, receipt, billing and returns** always use the full remaining quantity of every line. Wholesalers deliver in parts; customers return one item, not the whole invoice.

The engine supports all of it: `operation_options` reports per-line remaining quantities, the `receive`/`bill`/`fulfill`/`sale_return`/`purchase_return` preview+post actions take explicit line quantities, drafts have revisions, and `draft_delete` exists.

## Current state

- `crates/sarv-desktop/src/ui/ops.rs:31-75` — `post_operation(st, record_id, operation, reference, reason, request_key)`:
  ```rust
  let opts = st.query("operation_options", json!({ "record_id": record_id, "operation": operation }))?;
  let lines: Vec<Value> = opts["lines"]…filter_map(|l| {
      let rem = crate::data::s(&l["remaining_quantity"]);
      if rem == "0" || rem.is_empty() { None } else { Some(json!({ "id": l["id"], "quantity": qty_from_thousandths(&rem) })) }
  }).collect();
  …
  let mut payload = json!({ "record_id": record_id, "order_id": record_id, "receipt_id": record_id, "source_id": request_key, "request_key": request_key,
      "date": date, "due_date": date, "reference": reference, "reason": reason, "lines": lines });
  let review = st.query(&format!("{operation}_preview"), payload.clone())?;
  payload["expected_generation"] = review["_generation"].clone();
  st.run(&format!("{operation}_post"), payload, None)
  ```
- **Quantity rule (AGENTS.md)**: "Operation quantities are display units. `remaining_quantity` from the engine is thousandths and must be divided by 1000 before it is sent back." `ops::qty_from_thousandths("2500") == "2.5"`. Display with `st.fmt.qty(thousandths)`.
- Purchases create flow: `crates/sarv-desktop/src/ui/views/purchases.rs:175-215` — only when `st.purchases.receive_and_bill`: `post_operation(order_id, "receive", "R-{reference}", "Receive goods", "{key}-recv")` then `post_operation(receipt_id or order_id, "bill", "B-{reference}", "Bill received goods", "{key}-bill")`. Header actions: "New purchase order", "Return" (~329-334). `purchase_return` at ~514-527.
- Sales create flow: `crates/sarv-desktop/src/ui/views/sales.rs:163-258` — `draft_preview` → `draft_save {draft, expected_revision: null}` → `draft_post {id, revision, request_key}` → `post_operation(record_id, "fulfill", invoice_ref, …)` → optional settle. Quotation: `issue_quotation` (~335-376) calls `draft_save` with `issue_quotation: true`. Drafts tab table: `sales.rs:429-452`; header actions Print / Return (~476-477). `SalesState::start_new` (`:56-69`) builds a blank ticket; `build_draft_json(st, draft_id, customer, reference, lines, note)` builds the draft.
- Engine draft actions (`crates/sarv-quote/src/desktop_bridge.rs`): `draft_save` (~1475-1524; `expected_revision` as a string or null; keeps the issued quotation number), `draft_delete` (`:1525-1528`: `remove_sales_draft(string(&p,"id")?, integer(&p,"revision")?)`), `draft_post`. Drafts in `workspace.drafts` carry `id`, `revision`, `reference`, lines, `quote_issue`.
- How a record's state (ordered / partly received / fully billed) can be known: `operation_options` for `(record_id, operation)` returns lines with `remaining_quantity`; an empty or all-zero result means nothing left. Read the `"operation_options"` arm (`grep -n '"operation_options"' -A40 crates/sarv-quote/src/desktop_bridge.rs`) to learn which `operation` strings it accepts and which record kinds each applies to (order vs receipt vs invoice).
- Reference numbers are currently invented by the UI (`R-…`, `B-…`, `RET-…`, `PRET-…`, `next_invoice_ref`). Keep that scheme here; plan 026 moves numbering into the engine.
- Dialog/confirm conventions: plans 004 and 013 (`open_*` functions, `mint_key`, `review_operation`/`post_reviewed`, `Confirm`). `State::run` toasts errors; do not toast again.

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |
| Engine tests (should be untouched) | `cargo test -p sarv-quote` | all pass |

## Scope

**In scope**: `crates/sarv-desktop/src/ui/ops.rs`, new `src/ui/operation_dialog.rs`, `src/ui/views/{sales,purchases}.rs`, `src/fa_messages.rs`, `src/main.rs` (tests).

**Out of scope**: engine changes; document numbering (plan 026); discounts and per-line warehouses.

## Git workflow

- Branch `advisor/020-document-lifecycle`; commits `feat(desktop): operation dialog with editable quantities`, `feat(desktop): receive and bill open purchase orders`, `feat(desktop): edit, convert and delete quotations`, `feat(desktop): partial returns`.

## Steps

### Step 1: Operation dialog with editable quantities

Create `ui/operation_dialog.rs` with state on `State`:

```rust
pub struct OperationState { pub open: bool, pub record_id: String, pub operation: String, pub title: String,
    pub reference: String, pub reason: String, pub key: String, pub lines: Vec<OpLine>, pub posting: bool }
pub struct OpLine { pub id: serde_json::Value, pub name: String, pub remaining_thousandths: String /* from engine */ }
pub fn open(st: &mut State, record_id: &str, operation: &str, title_key: &str, reference: String, reason_key: &str)
```

`open` queries `operation_options`; if no line has remaining quantity, toast "Nothing left to process on this document." and do not open; otherwise mint the key (`st.mint_key(operation)`), fill one form field per line `op-qty-{i}` with the **display** quantity (`qty_from_thousandths(remaining)`), set `open`.

The dialog (form dialog, width 760) shows per line: item name, remaining (`fmt.qty`), an editable quantity field, and a date field (plan 014's `date_field`, key `op-date`). Footer: Cancel, "Review".

"Review": parse each quantity (accept Persian digits and decimal separator via `i18n::latin_digits`; reject negatives, non-numbers, and values above remaining — compare in thousandths: parse the typed decimal to thousandths with integer math, never floats); drop zero lines; require at least one line. Build the same payload shape as `post_operation` with these quantities (display units) and the chosen date, call `{operation}_preview`, copy `_generation`, then show plan 013's `Confirm` with the lines and the preview total; confirming calls `{operation}_post` via `st.run`, closes the dialog on success.

Add a unit test for the decimal-to-thousandths parser (`"2.5"`→2500, `"۲٫۵"`→2500, `"0.0005"`→error, `"-1"`→error).

### Step 2: Purchases — continue an order

In the Purchases view add header actions, enabled by selection: **Receive goods** (`operation "receive"` on an order), **Enter bill** (`"bill"` on a receipt, falling back to the order — mirror the `bill_against` logic at `purchases.rs:199-200`), and change **Return** to open the operation dialog with `"purchase_return"`. Each calls `operation_dialog::open` with the reference scheme used today (`R-`, `B-`, `PRET-` + the selected record's reference).

Show document progress in the purchases table: add a "Status" column derived from data already in the workspace if available (look for received/billed quantities on procurement records in `workspace.purchases`; print one record to check). If it is not derivable without calling `operation_options` per row, leave the column out and rely on the "Nothing left to process" message.

Keep the one-click "Receive & bill" switch in the create dialog for the fast path.

### Step 3: Sales — deliver and return in part

- **Deliver & invoice** action on a posted sales order with remaining quantity → operation dialog `"fulfill"`.
- **Return** → operation dialog `"sale_return"` (replaces the all-or-nothing return).

### Step 4: Quotations and drafts

On the Sales → Drafts tab add actions, enabled by selection:

- **Edit**: load the draft into the sale dialog — add `SalesState::start_edit(&mut self, draft: &Value)` that fills customer, lines (item, quantity, unit price — note the draft stores the price as typed in its `display_unit`), note, and stores `draft_id` and `editing_revision`. Saving an edited draft calls `draft_save` with `expected_revision: revision.to_string()` (not null) and keeps `issue_quotation` unset so the stored number is preserved.
- **Convert to sale**: runs the existing post chain from the saved draft: `draft_post {id, revision, request_key}` then the operation dialog for `"fulfill"` on the returned `record_id`. Prices come from the draft unchanged.
- **Delete**: plan 013 `Confirm` ("Delete this quotation?"), then `st.run("draft_delete", json!({"id": id, "revision": revision_string}), Some("Deleted"))`. Check how `integer(&p, "revision")` parses (string vs number) by reading the helper at the top of `desktop_bridge.rs`.

Persian strings for every new label and message (e.g. `("Receive goods", "دریافت کالا")`, `("Enter bill", "ثبت صورت‌حساب خرید")`, `("Deliver & invoice", "تحویل و صدور فاکتور")`, `("Convert to sale", "تبدیل به فروش")`, `("Nothing left to process on this document.", "چیزی برای ثبت روی این سند باقی نمانده است.")`). Check `fa_messages.rs` first — several exist already.

### Step 5: Tests (`main.rs`; patterns: `test_purchase_and_journal_dialogs_headlessly`, `test_owner_walks_profiles_documents_and_books`, `test_display_quantity_is_not_thousandths`)

1. `test_open_purchase_order_can_be_received_later` — create a PO with the switch off; select it; Receive goods; confirm; stock for the item increases.
2. `test_partial_receive_leaves_a_remainder` — receive 1 of 3; reopen the dialog: remaining shows 2.
3. `test_partial_sale_return_restocks_only_that_quantity`.
4. `test_quantity_above_remaining_is_rejected_before_preview`.
5. `test_quotation_edit_keeps_its_number` — issue, edit a line, save; `quote_issue` number unchanged, revision incremented.
6. `test_quotation_converts_with_the_same_prices`.
7. `test_draft_delete_removes_it`.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass, 7 new + the parser unit test.

## Done criteria

- [ ] `grep -rn '"draft_delete"' crates/sarv-desktop/src` matches
- [ ] `grep -rn "operation_dialog::open" crates/sarv-desktop/src/ui/views | wc -l` ≥ 5
- [ ] No float arithmetic on quantities: `grep -n "f64\|f32" crates/sarv-desktop/src/ui/operation_dialog.rs` shows only layout numbers
- [ ] Desktop suite passes; engine suite untouched and passing
- [ ] `plans/README.md` updated

## STOP conditions

- `operation_options` rejects an operation for a record kind you expected to work (e.g. `bill` directly on an order): read the engine's error, adjust which record the action targets; if the engine genuinely lacks a path (e.g. bill-before-receive), remove that action and report.
- A partial quantity posts 1000× or 1/1000× the intended amount in a test — stop immediately; the unit conversion is wrong.
- Editing a draft that has an issued quotation number changes or clears the number.

## Maintenance notes

- Reviewer focus: every quantity sent to the engine is a display-unit decimal string; every quantity read from it is thousandths.
- Plan 026 will replace the UI-built references (`R-`, `B-`, `RET-`) with engine-issued numbers; keep reference construction in one helper so that swap is a one-line change.
