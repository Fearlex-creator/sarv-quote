# Plan 035: Fix accounting & money-moving view bugs (Toman/Rial drafts, revisions, allocations, retries, guards)

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md` — unless a reviewer dispatched you and told you they
> maintain the index.
>
> **Drift check (run first)**: `git diff --stat 1e6866ac..HEAD -- crates/sarv-desktop/src/ui/views/sales.rs crates/sarv-desktop/src/ui/views/purchases.rs crates/sarv-desktop/src/ui/views/payments.rs crates/sarv-desktop/src/ui/views/banking.rs crates/sarv-desktop/src/ui/views/pos.rs crates/sarv-desktop/src/ui/views/accounting.rs crates/sarv-desktop/src/ui/views/contacts.rs crates/sarv-desktop/src/ui/views/setup.rs crates/sarv-desktop/src/ui/ops.rs crates/sarv-desktop/src/ui/operation_dialog.rs crates/sarv-desktop/src/ui/pickers.rs`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts against the live code before proceeding; on a
> mismatch, treat it as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: LOW
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `1e6866ac`, 2026-11-19

## Why this matters

Several confirmed bugs in the money-moving views directly corrupt or block accounting workflows:

1. **10× Toman/Rial draft mismatch**: `sales.rs` passes `draft["supplied_total"]` (which the engine formats in `draft.display_unit`) directly to `s.fmt.money(...)` (which expects rials and divides by 10 in Toman mode), showing every quotation at 1/10th value; editing a draft after switching units silently scales prices by 10×.
2. **Edited quotations cannot complete as sales**: `post_ticket` hardcodes `revision: "0"` / `expected_revision: null` instead of propagating `editing_revision`.
3. **Silent unallocated advances**: `payments.rs::build_allocations` swallows `payment_options` query errors and still posts the payment/cheque endorsement with zero allocations.
4. **Stuck retries & partial-post drift**: `purchases.rs` Receive & Bill doesn't track `order_id`/`receipt_id` across retries; POS/Sales allow mutating lines after `posted_record_id` is already committed.
5. **Validation, live-total & idempotency gaps**: `issue_quotation` skips line validation; `typed_display_amount` silently returns `"0"` on bad input; `estimate_total` parses fractional qty as `i64` (falling back to 1) and ignores discounts; `banking.rs`, `setup.rs` (also unchecked `cash + bank`), and `period_lock` miss `posting`/`mint_key`/`expected_generation` invariants; quick-created products don't sync `st.forms`; `purchases.rs` keeps stale `selected` across tabs; `post_ticket` scans only the first 100 drafts; dual-role contacts hide supplier balances.

## Current state

- `crates/sarv-desktop/src/ui/views/sales.rs`:
  - `build_draft_rows` (`lines 632-641`): passes `draft["supplied_total"]` straight to `s.fmt.money(&amount_raw)`. Engine produces `draft.supplied_total = total.display(draft.display_unit)` (`crates/sarv-quote/src/desktop_bridge.rs:1682`).
  - `start_edit` (`lines 95-101`): copies `unit_price` and `line_discount` verbatim without converting from `draft["display_unit"]` to `st.fmt.unit`.
  - `post_ticket` (`lines 301-329`): hardcodes `"expected_revision": null` and scans `st.books.list("drafts")` (first 100) instead of reading `draft_save`'s returned revision.
  - `issue_quotation` (`lines 502-551`): does not validate `qty`/`price`/`discount`.
  - `estimate_total` (`lines 887-889`): uses `.parse::<i64>().unwrap_or(1).max(1)` and ignores `l.discount`.
  - Quick-create (`lines 1129-1132`) sets `l.price` without `st.forms.set(&format!("sale-price-{idx}"), ...)`.
- `crates/sarv-desktop/src/ui/views/payments.rs`:
  - `build_allocations` (`lines 718-746`): `if let Ok(o) = opts` ignores `Err(_)`.
- `crates/sarv-desktop/src/ui/views/purchases.rs`:
  - `post_purchase` (`lines 207-253`): no `posted_order_id`/`received_record_id` progress tracking.
  - Tab switch (`lines 353-358`): does not clear `st.purchases.selected`.
  - Quick-create (`lines 632-635`): sets `l.price` without `st.forms.set(&format!("buy-price-{idx}"), ...)`.
- `crates/sarv-desktop/src/ui/views/pos.rs`:
  - `ensure_keys` (`lines 77-79`) uses raw `timestamp_nanos_opt()`.
  - `add_item`/`step` (`lines 84-133`) mutate `st.pos.lines` even when `posted_record_id.is_some()`.
- `crates/sarv-desktop/src/ui/views/banking.rs`:
  - `BankingState` (`lines 14-24, 242-292, 315`): no `posting: bool`, uses raw `timestamp_nanos_opt()`, does not clear `post_key` on success.
- `crates/sarv-desktop/src/ui/views/accounting.rs`:
  - `toggle_period` (`lines 939-949`): dispatches `"period_lock"` without `expected_generation`.
- `crates/sarv-desktop/src/ui/views/setup.rs`:
  - `lines 380-411`: mints fresh `request_key` on every click with no `posting` guard; `line 532`: unchecked `cash + bank`.
- `crates/sarv-desktop/src/ui/views/contacts.rs`:
  - `balance_for` (`lines 79-85`): always prefers `customer:{id}` over `vendor:{id}` even on `ContactFilter::Suppliers`.
- `crates/sarv-desktop/src/ui/ops.rs`:
  - `typed_display_amount` (`lines 26-36`): accepts either unit and silently returns `"0"` on parse error.

Invariants from `AGENTS.md`:
- Books store integer rials. Toman typed by the user is sent as that toman string with `display_unit: Toman` on a sales draft; purchases, payments, journals, and banking parse rials.
- Preview before posting and pass `expected_generation` when returned.
- A dialog that can be clicked twice must reuse the `request_key` minted when it opened.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Desktop unit/UI tests | `cargo test -p sarv-desktop --bin sarv-desktop` | exit 0, all pass |
| Workspace clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, no warnings |
| Format check | `cargo fmt --check` | exit 0 |

## Scope

**In scope** (the only files you should modify):
- `crates/sarv-desktop/src/ui/views/sales.rs`
- `crates/sarv-desktop/src/ui/views/purchases.rs`
- `crates/sarv-desktop/src/ui/views/payments.rs`
- `crates/sarv-desktop/src/ui/views/banking.rs`
- `crates/sarv-desktop/src/ui/views/pos.rs`
- `crates/sarv-desktop/src/ui/views/accounting.rs`
- `crates/sarv-desktop/src/ui/views/contacts.rs`
- `crates/sarv-desktop/src/ui/views/setup.rs`
- `crates/sarv-desktop/src/ui/ops.rs`
- `crates/sarv-desktop/src/ui/operation_dialog.rs`
- `crates/sarv-desktop/src/tests/sales.rs`, `purchases.rs`, `payments.rs`, `banking.rs`, `pos.rs`, `setup.rs`

**Out of scope**:
- Any changes to `crates/sarv-quote` accounting math or bridge schemas.
- Sidebar, overlay, or Settings UI redesign (handled in Plans 036 and 039).

## Git workflow

- Branch: `advisor/035-accounting-and-money-flow-fixes`
- Conventional commits (e.g., `fix(desktop): fix sales draft unit conversion and revision propagation`).
- Do NOT push or open a PR unless instructed.

## Steps

### Step 1: Fix sales draft unit conversion, `editing_revision`, direct revision parsing, and validation

1. In `crates/sarv-desktop/src/ui/ops.rs`:
   - Change `typed_display_amount(raw: &str, unit: Unit) -> Result<String, MoneyError>` so it validates strictly against `unit` (`parse_money(raw, unit)`) and returns `Err` on invalid input instead of `"0"`.
2. In `crates/sarv-desktop/src/ui/views/sales.rs`:
   - Add a helper `draft_unit(draft: &Value) -> Unit` (`"Rial"` → `Unit::Rial`, otherwise `Unit::Toman`).
   - In `build_draft_rows`: parse `draft["supplied_total"]` with `parse_money(&amount_raw, draft_unit(draft))` into rials (falling back to raw if unparseable) before passing to `s.fmt.money(&rials.to_string())` and `SortKey::num_str`.
   - In `start_edit` / `edit_selected_draft`: convert each line's `unit_price` and `line_discount` from `draft_unit(draft)` into rials and format via `st.fmt.money_input(rials)` into both `SaleLine` and `st.forms`.
   - Add `editing_revision: Option<String>` to `Ticket`; in `post_ticket`, set `draft["revision"]` and `"expected_revision"` when `Some` (matching `issue_quotation`), and pass `st.sales.editing_revision.clone()` from `complete_sale` (`None` from `pos.rs`).
   - Read the saved revision directly from `let saved_rev = st.run("draft_save", ...)?` instead of scanning `st.books.list("drafts")`.
   - Extract shared line validation from `post_ticket` and call it in `issue_quotation`.
   - Fix `estimate_total` to parse fractional quantities via `crate::ui::operation_dialog::parse_thousandths`, compute `(i128::from(price) * i128::from(qty_th) + 500) / 1000`, and subtract parsed `l.discount`.
   - Use an `AtomicU64` counter for `SaleLine::blank()` IDs instead of `timestamp_millis()`.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop sales` → exit 0.

### Step 2: Fix `payments.rs` allocation error handling and `purchases.rs` retries/selection/sync

1. In `crates/sarv-desktop/src/ui/views/payments.rs`:
   - Change `build_allocations` to return `Result<(Vec<serde_json::Value>, String), String>` and propagate `st.query("payment_options", ...)?`.
   - At both call sites (`lines 600, 917`), on `Err(e)`, call `st.toast_err(e)`, reset `posting`/`spend_posting = false`, and return early.
2. In `crates/sarv-desktop/src/ui/views/purchases.rs`:
   - Add `posted_order_id: Option<String>` and `received_record_id: Option<String>` to `PurchasesState`; skip already-completed steps in `post_purchase`, and reset them in `start_new` and on full success.
   - Clear `st.purchases.selected` on tab switch (`lines 353-358`).
   - Use an `AtomicU64` counter for `PurchaseLine::blank()` IDs.
3. In both `sales.rs` (`lines 1129-1132`) and `purchases.rs` (`lines 632-635`):
   - Call `st.forms.set(&format!("sale-price-{idx}"), &price)` / `buy-price-{idx}` when quick-creating a product, and ensure clearing a pre-filled price/qty field updates `line.price`/`line.qty` rather than keeping stale values.
4. In `pos.rs` (`lines 84-133`) and `sales.rs`:
   - Block cart/line edits when `posted_record_id.is_some()` (toast an info message to retry completion or clear the in-flight order).

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → exit 0.

### Step 3: Enforce `posting` guards, `mint_key`, `expected_generation`, checked math, and dual-role contact balances

1. In `crates/sarv-desktop/src/ui/views/banking.rs`:
   - Add `posting: bool` to `BankingState` (reset in `open_move`, set/reset around preview+post, clear `post_key` on `Ok`, use `st.mint_key("bank")` instead of raw `timestamp_nanos_opt()`).
2. In `crates/sarv-desktop/src/ui/views/pos.rs`:
   - Replace raw `timestamp_nanos_opt()` in `ensure_keys` with `st.mint_key(...)`.
3. In `crates/sarv-desktop/src/ui/operation_dialog.rs`:
   - Set `st.operation.posting = true` before preview/post and reset to `false` on all exit paths (matching `payments.rs`).
4. In `crates/sarv-desktop/src/ui/views/accounting.rs` (`toggle_period`, `lines 939-949`):
   - Include `"expected_generation": gen` when `st.books.workspace["_generation"]` is non-empty.
5. In `crates/sarv-desktop/src/ui/views/setup.rs`:
   - Store `opening_key: String` and `opening_posting: bool` on `SetupState` (minted once when entering opening step, reused across clicks, cleared on success).
   - Replace `cash + bank` (`line 532`) with `cash.checked_add(bank)` returning a validation error toast on overflow.
6. In `crates/sarv-desktop/src/ui/views/contacts.rs`:
   - Pass `s.contacts.filter` into `balance_for`: use `vendor:{id}` on `ContactFilter::Suppliers`, `customer:{id}` on `ContactFilter::Customers`, and combine/net on `ContactFilter::All`.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check` → exit 0.

## Test plan

- Add targeted headless tests in `crates/sarv-desktop/src/tests/`:
  - `sales.rs`: Toman draft displays exact amount (not 1/10th); editing a Toman draft after switching to Rial preserves economic value; editing an existing quotation and clicking Complete Sale succeeds; fractional quantity (`2.5`) + line discount computes accurate `estimate_total`; invalid price on quotation is rejected (not silently saved as `"0"`).
  - `purchases.rs`: tab switch clears `selected`; Receive & Bill resumes after a simulated mid-step failure.
  - `banking.rs`: `open_move` reuses `post_key` on retry and clears it on success.
  - `setup.rs`: overflow in `cash + bank` is rejected cleanly without panic.
- Model after existing tests in `crates/sarv-desktop/src/tests/sales.rs` and `purchases.rs`.
- Verification: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass, including ≥8 new tests.

## Done criteria

- [ ] `cargo test -p sarv-desktop --bin sarv-desktop` exits 0 with all new tests passing
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` exits 0
- [ ] `cargo fmt --check` exits 0
- [ ] No raw `timestamp_nanos_opt()` calls remain in `banking.rs`, `pos.rs`, or `setup.rs`
- [ ] Only in-scope files are modified (`git status`)
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back (do not improvise) if:
- `draft_save` in `crates/sarv-quote/src/desktop_bridge.rs` does not return the new revision in its JSON response (check `desktop_bridge.rs:1662` first).
- Changing `typed_display_amount` breaks any existing test that intentionally relied on empty string meaning `"0"` (empty string should still map to `"0"` for optional discounts; only *malformed* non-empty strings must return `Err`).
- A verification command fails twice after a reasonable fix attempt.

## Maintenance notes

- Quick-product dialog duplication between `sales.rs` and `purchases.rs` can be extracted into a shared helper in a later cleanup pass once these correctness fixes land.
