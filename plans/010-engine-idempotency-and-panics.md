# Plan 010: Retried requests are matched strictly, quotation numbers stay unique, bad payloads cannot panic

> **Executor instructions**: Follow this plan step by step; each step is an independent commit. Run every verification command. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-quote/src/cheques.rs crates/sarv-quote/src/desktop_bridge.rs crates/sarv-quote/src/agent.rs`

## Status

- **Priority**: P2
- **Effort**: M
- **Risk**: LOW–MED
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

The engine promises idempotent retries: sending the same `request_key` twice must not post twice, and reusing a key for *different* content must be an error. The journal kernel honours that. The cheque code and the bridge's generic replay check do not, which lets a cheque action report success while doing nothing. Separately, quotation numbers come from a counter that a stale settings save can move backwards, and a malformed payload can panic the engine thread.

## Current state

- Reference behaviour — journal kernel, `crates/sarv-quote/src/accounting.rs:1288-1294`: on a key match it compares the whole command and fails with `"کلید درخواست با محتوای متفاوت استفاده شده است"` if it differs.
- `crates/sarv-quote/src/cheques.rs:167-169`:
  ```rust
  fn by_request(&self, key: &str) -> Option<&ChequeEvent> {
      self.events.iter().find(|e| e.request_key == key)
  }
  ```
  Callers return success on any match, e.g. `cheques.rs:299-302`:
  ```rust
  pub fn deposit_cheque(&mut self, ctx: &PaymentContext, cheque_id: &str) -> Result<()> {
      if self.cheques.by_request(&ctx.request_key).is_some() {
          return Ok(());
      }
  ```
  Same shape in the reverse/bounce/void path (~line 349), `spend_cheque` (~line 371) and registration (~line 236, returns the earlier cheque's id).
- `crates/sarv-quote/src/desktop_bridge.rs:1115-1152` — `dispatch_in` computes `replay` = "this key exists in `accounting.requests`, `sales.requests`, `procurement.requests` or `core.zero_requests`" and skips the `expected_generation` check when true. It does not look at `payments` request records or cheque events, and it does not check that the key belongs to the same action.
- Quotation numbering, `desktop_bridge.rs:1481-1497`:
  ```rust
  // A quotation keeps its issued number across revisions.
  if draft.quote_issue.is_none() && let Some(stored) = … { draft.quote_issue = Some(stored); }
  if p["issue_quotation"] == true && draft.quote_issue.is_none() {
      json!(store.change(|s| {
          …
          let number = format!("{series}{}", s.settings.next_number.max(1));
          s.settings.next_number = s.settings.next_number.saturating_add(1);
  ```
  A client-supplied `quote_issue` wins over the stored one. The `"settings"` arm (~lines 1746-1765) replaces the whole settings struct including `next_number`, with no generation check.
- Panics: `desktop_bridge.rs:1054-1056` `let mut q = p.clone(); q["side"] = json!(side);` panics if `p` is not a JSON object; `.unwrap()` at ~`:355` and ~`:631`. `agent.rs:121-124` — a panic poisons the `accounting` mutex and every later agent call returns "Accounting state interrupted".
- Tests to model on: `crates/sarv-quote/tests/cheques.rs`, `tests/payments_trust.rs`, `tests/desktop_bridge.rs` (helpers `fixture()`, `ctx()`, `call()` at lines 5-19), `tests/settings.rs`, `tests/sales_drafts.rs`.

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Engine tests | `cargo test -p sarv-quote` | all pass |
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |

## Scope

**In scope**: `crates/sarv-quote/src/cheques.rs`, `src/desktop_bridge.rs`, `src/agent.rs` (mutex recovery only), tests under `crates/sarv-quote/tests/`.

**Out of scope**: persisted data shapes (no new fields in `ChequeEvent`, settings or drafts — replay of existing books must be unchanged); the desktop crate.

## Git workflow

- Branch `advisor/010-engine-idempotency-and-panics`; commits `fix(cheques): …`, `fix(engine): …`.

## Steps

### Step 1: Cheque retries must match the cheque and the action

Read `ChequeEvent` (`grep -n "pub struct ChequeEvent" -A14 crates/sarv-quote/src/cheques.rs`) to see which fields identify an event (cheque id, action). Replace each `if self.cheques.by_request(key).is_some() { return Ok(()) }` with a helper:

```rust
/// Ok(true) = exact retry, nothing to do. Ok(false) = new request. Err = key reused for something else.
fn replayed(&self, key: &str, cheque_id: &str, action: <ActionType>) -> Result<bool>
```

that returns the same "key used with different content" error text as `accounting.rs:1291-1294` when the cheque id or action differs. For registration, compare the stored details with the incoming ones.

Tests in `tests/cheques.rs`:
- `retry_of_the_same_deposit_is_a_no_op` (same key, same cheque, same action → Ok, one event).
- `key_reused_for_another_cheque_is_rejected`.
- `key_reused_for_another_action_is_rejected` (deposit then bounce with the deposit's key → Err, and the cheque status is unchanged).

**Verify**: `cargo test -p sarv-quote --test cheques` → all pass.

### Step 2: The bridge's replay check covers payments and cheques

In `dispatch_in`, extend `replay` to also be true when the key exists in the payments request log and in cheque events (find the field names with `grep -n "requests" crates/sarv-quote/src/payments.rs | head` and Step 1's reading). This fixes genuine retries of payments/cheques being refused with "Books changed since review" after an unrelated save.

Test in `tests/payments_trust.rs`: post a payment with generation G; make an unrelated save (e.g. a `settings` call); resend the identical payment request (same key, old `expected_generation`) → returns the original result, and the payments count is unchanged.

### Step 3: Quotation numbers

- Once a draft has a stored `quote_issue`, always use the stored one: in the `draft_save` arm, look up the stored issue first and overwrite `draft.quote_issue` with it whenever it exists (ignore what the client sent).
- When issuing a new number, loop: build the candidate; if any existing draft or approved quote already has that number (see how `domain.rs` `approve` checks uniqueness, ~lines 1331-1336), increment and retry (bounded to 10,000 iterations, then error).
- In the `"settings"` arm: keep `next_number = max(stored, incoming)`.

Tests in `tests/sales_drafts.rs` / `tests/settings.rs`:
- `issued_quotation_number_cannot_be_changed_by_the_client`.
- `stale_settings_save_cannot_move_the_counter_back` (issue quotation → `next_number` is N+1; save settings with `next_number` N → stays N+1; next issued number is unique).
- Existing `settings_round_trip_preserves_upstream_fiscal_month_and_books` must still pass.

### Step 4: No panics from payload shape

- At the top of `dispatch_in`: `ensure!(p.is_object(), "Payload must be an object");`
- Replace the `.unwrap()` calls at ~`:355` and ~`:631` (find with `grep -n "\.unwrap()" crates/sarv-quote/src/desktop_bridge.rs | head -20`; skip ones inside `#[cfg(test)]`) with `ok_or_else(|| anyhow!("…"))?`.
- In `agent.rs` `Inner::dispatch`, recover a poisoned mutex: `let mut store = self.accounting.lock().unwrap_or_else(|e| e.into_inner());` then, if it was poisoned, replace the store with a freshly opened one (`Store::open(&self.directory)?`) before continuing, so no half-applied in-memory state is used.

Test in `tests/desktop_bridge.rs`: `non_object_payload_is_an_error_not_a_panic` — `dispatch(dir, "payment_preview", json!([1,2,3]))` and `json!("x")` return `Err`.

**Verify**: `cargo test -p sarv-quote` → all pass; `cargo test -p sarv-desktop --bin sarv-desktop` → all pass.

## Done criteria

- [ ] `grep -n "by_request(&ctx.request_key).is_some()" crates/sarv-quote/src/cheques.rs` returns nothing
- [ ] 8 new engine tests exist and pass (3 + 1 + 3 + 1)
- [ ] `cargo test -p sarv-quote` and the desktop suite pass
- [ ] `plans/README.md` updated

## STOP conditions

- Any replay/migration test fails (`tests/*_migration.rs`, `tests/process_recovery.rs`, `tests/accounting_kernel.rs`): the change altered how existing books replay. Revert that step and report.
- The desktop's cheque buttons break because they send one fixed key per action per cheque (`crates/sarv-desktop/src/ui/views/payments.rs:453-455`, `format!("{action}-{cheque_id}")`) and the engine legitimately allows the same action twice on a cheque (e.g. deposit → return → deposit). Report it; plan 013 changes the desktop to mint a key per click.

## Maintenance notes

- The desktop's fixed cheque keys rely on the lenient behaviour removed in Step 1 only in the "same action twice" case; plan 013 is the matching desktop change. Land 013 soon after this plan.
- Requiring `expected_generation` for *all* mutating actions (settings, profiles, sheet saves) is a larger behaviour change left for a later plan; today's desktop does not send it for those.
