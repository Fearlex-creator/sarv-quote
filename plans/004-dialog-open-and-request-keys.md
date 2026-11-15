# Plan 004: Every dialog opens through one function that resets its fields and mints its request key

> **Executor instructions**: Follow this plan step by step. Run every verification command. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-desktop/src/ui/views crates/sarv-desktop/src/app.rs crates/sarv-desktop/src/ui/resources.rs`

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: LOW
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

Project rule (AGENTS.md): "A blank `request_key` is filled in by the desktop; a dialog that can be clicked twice must reuse the key it minted when it opened." Today the code that opens each dialog is copy-pasted between the owning view and the Overview quick actions, and the copies drifted. Results, all confirmed in code:

1. After posting one journal from Accounting, a journal started from **Overview → New journal** reuses the old key and the engine rejects it ("request key used with different content"). Same for **Overview → Record a payment**.
2. A stock transfer has no request key at all, so a double click posts two transfers.
3. New invoices, purchase orders, payments and contacts inherit field values typed into the previous dialog.
4. Several callers toast an engine error that `State::run` already toasted, so the user sees it twice.

## Current state

- `crates/sarv-desktop/src/app.rs:131-153` — `Forms`, a map of text fields by string key; has `clear_prefix(&mut self, prefix: &str)`.
- `app.rs:347-363` — `State::run` toasts the engine error itself (`self.toast(Tone::Danger, e.clone())`) and returns `Err(e)`.
- `app.rs:330-341` — `fill_books_context` mints a **new** `ui-{pid}-{nanos}` key on every call when `request_key` is blank.
- Journal — owner's open code, `ui/views/accounting.rs:184-192`:
  ```rust
  ui.btn(Btn::Primary, "New journal", Some(&icons::PLUS), |st| {
      st.accounting.modal_open = true;
      st.accounting.debit_account.clear();
      st.accounting.credit_account.clear();
      st.accounting.post_key =
          format!("journal-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
      st.accounting.posting = false;
      st.forms.set("journal-amount", "");
      st.forms.set("journal-reason", "");
  }),
  ```
  Overview's copy, `ui/views/overview.rs:718-725`, omits `post_key` and `posting`.
  Save uses the stored key when non-empty (`accounting.rs:269-273`) and never clears it after success.
- Payment — `ui/views/payments.rs:142-153` and `:154-165` (two pasted blocks, both mint `post_key`); Overview's `open_payment` at `overview.rs:211-221` does not. None of them clear `pay-sayad`, `pay-cheque-no`, `pay-bank`, `pay-due`.
- Sale — `ui/views/sales.rs:56-69` `SalesState::start_new` mints `post_key`/`draft_id` but cannot clear `Forms` (it only has `&mut self`); line fields `sale-qty-{i}`, `sale-price-{i}` survive. Purchases: `ui/views/purchases.rs` `start_new` has the same shape with `buy-*` fields.
- Contact — `overview.rs:187-195` clears 4 fields; the contact dialog has about 14 (`grep -n '"contact-' crates/sarv-desktop/src/ui/views/contacts.rs`).
- Product — `overview.rs:197-209`.
- Transfer — `ui/views/inventory.rs:475-506`: payload has no `request_key`/`source_id`, no `posting` guard.
- `ui/resources.rs:39-41` — `uid()` is `ui-{millis}`: two calls in one millisecond collide.
- Double toasts: `grep -n "toast_err" crates/sarv-desktop/src/ui/views/*.rs` and check each one — if the `Err` came from `st.run(...)` (directly or via `create_party`, `create_item`, `ensure_walk_in_customer`, `ops::post_operation`), it is a duplicate. `sales.rs:243` and `:253` work around it by comparing with the last toast.

## Commands you will need

| Purpose | Command | Expected |
|---|---|---|
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |
| Build | `cargo build -p sarv-desktop` | exit 0 |

## Scope

**In scope**: `crates/sarv-desktop/src/app.rs`, `src/ui/resources.rs`, `src/ui/ops.rs`, `src/ui/views/{accounting,payments,sales,purchases,contacts,inventory,overview,banking,pos}.rs`, `src/ui/shell.rs` (palette "New sales invoice" action), `src/main.rs` (tests and the `invoice` scene).

**Out of scope**: engine crate; visual changes; the cheque action buttons (plan 013).

## Git workflow

- Branch `advisor/004-dialog-open-and-request-keys`; one commit per dialog family, e.g. `fix(desktop): mint the journal key wherever the dialog opens`.

## Steps

### Step 1: One key generator

In `app.rs` add to `impl State`:

```rust
/// A request key unique within and across runs: prefix, process id, nanoseconds, counter.
pub fn mint_key(&mut self, prefix: &str) -> String {
    self.key_counter += 1;
    format!("{prefix}-{}-{}-{}", std::process::id(), chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0), self.key_counter)
}
```

with a private `key_counter: u64` field. Make `uid()` in `resources.rs` delegate to the same format (it takes `&self`; change it to `&mut self` and fix callers, or use an `AtomicU64` static counter — pick whichever compiles with fewer call-site changes).

### Step 2: One `open_*` function per dialog

For each dialog, create one `pub fn` in the owning view module that (a) clears every form field of that dialog with `st.forms.clear_prefix("<prefix>-")`, (b) resets the view state, (c) mints the key, (d) sets `modal_open = true`, `posting = false`. Then make every other opener call it.

| Dialog | Function to create | Form prefixes to clear | Callers to replace |
|---|---|---|---|
| Journal | `accounting::open_journal(st)` | `journal-` | `accounting.rs:184-192`, `overview.rs:718-725` |
| Payment | `payments::open_payment(st, receive: bool)` | `pay-` | `payments.rs:142-165` (both), `overview.rs:211-221` |
| Sale | `sales::open_sale(st)` — calls `st.sales.start_new()` then clears | `sale-` | `overview.rs:175-180`, `shell.rs` sidebar "New sales invoice" and palette `action-new-order`, the Sales page button, `main.rs` `invoice` scene |
| Purchase | `purchases::open_purchase(st)` | `buy-` | `overview.rs:182-185`, Purchases page button |
| Contact | `contacts::open_new_contact(st, supplier: bool)` | `contact-` | `overview.rs:187-195`, Contacts page button(s) |
| Product | `inventory::open_new_product(st)` | `item-` | `overview.rs:197-209`, Inventory page button |
| Transfer | `inventory::open_transfer(st)` | `transfer-` | Inventory page button |

Before clearing a prefix, run `grep -rn '"<prefix>-' crates/sarv-desktop/src` and confirm no *other* dialog shares it (e.g. the product **profile** dialog uses `prod-`, not `item-`). If a prefix is shared, rename one side's keys rather than clearing both.

After `open_*` Overview functions navigate with `st.nav(Page::…)` as they do today.

**Verify** after each row: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass.

### Step 3: Transfer gets a key and a guard

Add `transfer_key: String` and `transfer_posting: bool` to `InventoryState`; mint in `open_transfer`. In the Save handler (`inventory.rs:475-506`) return early if `transfer_posting`; set it true; add `"request_key": key, "source_id": key` to the payload; reset it on every exit path; close the dialog only on `Ok`.

### Step 4: Clear the key after success

In each Save handler, after the post succeeds and the dialog closes, clear the stored key (`st.accounting.post_key.clear()` etc.). Remove the "if key is empty, mint one now" fallbacks (`accounting.rs:269-273`, `payments.rs:298-302`, `sales.rs:186-187`) and replace them with: if the key is empty, toast `st.fmt.t("Reopen this form and try again.")` and return. Add that key to `fa_messages.rs` as `("Reopen this form and try again.", "فرم را ببندید و دوباره باز کنید."),`.

### Step 5: Remove double toasts

For each `toast_err(e)` identified in "Current state" whose error came through `run`, delete the toast. Delete the `st.toasts.last()...` comparisons in `sales.rs` and `purchases.rs` (search `toasts.last()`). Keep `toast_err` after `st.query(...)` — `query` does not toast.

`ops::post_operation` mixes both: its `query` calls do not toast, its final `run` does. Make it consistent: inside `post_operation`, toast the error from the two `query` calls itself (`st.toast_err`), so that **every** `Err` it returns has already been shown, and document that in its doc comment.

### Step 6: Tests

Add to `main.rs` tests (use `make_test_runtime()` and `click(&mut rt, "btn-…")` like `test_journal_post_uses_preview_generation`, which is the structural pattern for posting a journal):

1. `test_second_journal_from_overview_posts`: post a journal via the Accounting button; then `click` Overview's `btn-New journal`, fill different accounts/amount, save; assert no `Tone::Danger` toast and that `journal_count` grew by 2.
2. `test_second_payment_from_overview_posts`: same shape with `btn-Record a payment` (pattern: `test_payment_allocates_across_open_items`).
3. `test_new_sale_does_not_inherit_line_fields`: set `forms.set("sale-qty-0", "7")`, call `sales::open_sale`, assert `forms.get("sale-qty-0")` is empty.
4. `test_transfer_double_click_posts_once`: open transfer, fill, invoke Save twice in a row, assert exactly one new transfer (compare the workspace's stock or procurement/transfer count before and after — read `workspace` JSON keys with a debug print first to pick the right counter).
5. `test_engine_error_toasts_once`: trigger a failing post (e.g. journal with the same account on both sides) and assert `rt.state.toasts.iter().filter(|t| t.tone == Tone::Danger).count() == 1`.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass, 5 new.

## Done criteria

- [ ] `grep -n "modal_open = true" crates/sarv-desktop/src/ui/views/overview.rs` returns nothing (Overview only calls `open_*`)
- [ ] `grep -rn "toasts.last()" crates/sarv-desktop/src` returns nothing
- [ ] `grep -n "request_key" crates/sarv-desktop/src/ui/views/inventory.rs` shows the transfer key
- [ ] Desktop tests pass including the 5 new ones
- [ ] `plans/README.md` updated

## STOP conditions

- A form prefix is shared by two dialogs in a way that renaming would touch more than ~20 call sites.
- An existing test relies on values leaking between dialogs (it sets a field, then opens the dialog). Report the test; do not weaken the reset.
- Test 1 passes *before* you make changes — then the bug described here is not reproduced and the plan's premise needs re-checking.

## Maintenance notes

- New dialogs must follow the same rule: one `open_*`, prefix-scoped fields, key minted on open, cleared on success. Plan 011 records this in the design-system doc.
- Plan 013 builds the confirmation/review dialog and the unsaved-changes guard on top of these `open_*` functions.
