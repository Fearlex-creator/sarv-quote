# Plan 003: The desktop keeps one open Store instead of reloading the books on every action

> **Executor instructions**: Follow this plan step by step. Run every verification command. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-desktop/src/data.rs crates/sarv-desktop/src/ui/documents.rs crates/sarv-quote/src/desktop_bridge.rs crates/sarv-quote/src/store.rs`

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: MED
- **Depends on**: none (001 recommended first)
- **Category**: perf
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

Every button press in the desktop opens SQLite, reads the entire book as JSON, parses it and runs the full validator — then does it three more times to refresh the screen. One counter sale triggers about 20 full loads, on the UI thread. The engine already has the fix (`dispatch_in` + `refresh_if_stale`) and the local agent service uses it; the desktop does not. The same flaw means the background integrity check (ADR-0007) can never report to the Overview, because the `Store` that holds its result is dropped after each call.

## Current state

- `crates/sarv-desktop/src/data.rs:8-10, 44-47`:
  ```rust
  pub struct Engine {
      pub dir: PathBuf,
  }
  ...
  pub fn call(&self, action: &str, payload: Value) -> Result<Value, String> {
      sarv_quote::desktop::dispatch(&self.dir, action, payload).map_err(|e| e.to_string())
  }
  ```
- `crates/sarv-quote/src/desktop_bridge.rs:1106-1114`:
  ```rust
  pub fn dispatch(directory: &Path, action: &str, p: Value) -> Result<Value> {
      let mut store = Store::open(directory)?;
      dispatch_in(&mut store, action, p)
  }

  /// Dispatch against a store the caller keeps open across commands. The owner
  /// must call `refresh_if_stale` first so a second app or the local agent is
  /// never served a cached snapshot.
  pub fn dispatch_in(store: &mut Store, action: &str, p: Value) -> Result<Value> {
  ```
- The exemplar to copy — `crates/sarv-quote/src/agent.rs:119-128`:
  ```rust
  fn dispatch(&self, action: &str, payload: Value) -> Result<Value> {
      let mut store = self.accounting.lock().map_err(|_| anyhow!("Accounting state interrupted"))?;
      store.refresh_if_stale()?;
      crate::desktop::dispatch_in(&mut store, action, payload)
  }
  ```
- `crates/sarv-quote/src/store.rs:200-256` — `Store::open` (full parse + `state.validate()`); `store.rs:471-492` — `refresh_if_stale` (one `SELECT generation`, reloads only when it advanced). It reads generation and payload in two separate queries; fix that here (Step 4).
- `crates/sarv-desktop/src/ui/documents.rs:24` — PDF export opens its own second `Store::open(&st.engine.dir)` to read state.
- `Engine::call` takes `&self`, and `State::query(&self, …)` calls it from view code that only has `&State`. So the store needs interior mutability.
- Confirm `dispatch_in` and `Store` are exported: `grep -n "pub use\|pub fn dispatch_in" crates/sarv-quote/src/desktop.rs crates/sarv-quote/src/desktop_bridge.rs`.

## Commands you will need

| Purpose | Command | Expected |
|---|---|---|
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |
| Engine tests | `cargo test -p sarv-quote` | all pass |
| Store tests | `cargo test -p sarv-quote --test storage_hardening --test process_recovery` | all pass |

## Scope

**In scope**:
- `crates/sarv-desktop/src/data.rs`
- `crates/sarv-desktop/src/ui/documents.rs`
- `crates/sarv-quote/src/store.rs` (only `refresh_if_stale`)
- `crates/sarv-quote/src/desktop.rs` (only to re-export `dispatch_in` if it is not already public)
- `crates/sarv-desktop/src/main.rs` (tests)

**Out of scope**:
- `dispatch` itself — keep it; 44 engine test files call it.
- Moving engine calls to a worker thread. That is a larger change; this plan removes the bulk of the cost first.
- `crates/sarv-quote/src/agent.rs`.

## Git workflow

- Branch `advisor/003-long-lived-store`; commits `perf(desktop): keep one open store across actions`, `fix(engine): read generation and payload in one query`.

## Steps

### Step 1: Hold the store in `Engine`

Change `Engine` to:

```rust
pub struct Engine {
    pub dir: PathBuf,
    store: std::cell::RefCell<Option<sarv_quote::store::Store>>,
}
```

Update the three constructors (`open`, `demo`, `with_dir`) to initialise `store: RefCell::new(None)`. If the compiler reports that `State` must be `Send`/`Sync` (Rune's `AppSpec` bound), use `std::sync::Mutex` instead of `RefCell` and `lock().map_err(|_| "Books are busy".to_string())`.

### Step 2: Route `call` through the held store

```rust
pub fn call(&self, action: &str, payload: Value) -> Result<Value, String> {
    let mut slot = self.store.borrow_mut();
    if slot.is_none() {
        *slot = Some(sarv_quote::store::Store::open(&self.dir).map_err(|e| e.to_string())?);
    }
    let store = slot.as_mut().expect("store was just opened");
    if let Err(e) = store.refresh_if_stale() {
        *slot = None; // reopen on the next call rather than stay stuck
        return Err(e.to_string());
    }
    sarv_quote::desktop::dispatch_in(store, action, payload).map_err(|e| e.to_string())
}
```

A failed open leaves the slot empty so the next call retries (this matters for plan 007, recovery).

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass.

### Step 3: Stop the PDF path opening a second store

In `ui/documents.rs`, add to `Engine` a method `pub fn with_state<T>(&self, f: impl FnOnce(&sarv_quote::domain::State) -> Result<T, String>) -> Result<T, String>` that opens/refreshes the held store the same way and passes `&store.state`. Rewrite `documents::write` to use it instead of `Store::open`.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop test_owner_walks_profiles_documents_and_books` → passes (it exercises PDF writing).

### Step 4: One query in `refresh_if_stale`

In `store.rs` `refresh_if_stale`, keep the cheap `SELECT generation` fast path. When the generation differs, replace the second `SELECT payload` with one `SELECT generation,payload FROM state WHERE id=1` and use the generation from that same row for `self.generation`, exactly as `Store::open` does at `store.rs:207-210`.

**Verify**: `cargo test -p sarv-quote --test storage_hardening --test process_recovery --test agent_access` → all pass.

### Step 5: Tests

Add to the `tests` module in `main.rs`:

- `test_engine_sees_writes_from_another_store`: build a runtime; with `sarv_quote::desktop::dispatch(&rt.state.engine.dir, "settings", …)` (a second, independent store) change `show_pos`; call `rt.state.refresh()`; assert the workspace reflects the change. This proves `refresh_if_stale` is honoured. Model the settings payload on `test_export_writes_csv_and_language_persists`.
- `test_integrity_status_survives_between_calls`: call `rt.state.query("integrity", json!({}))` twice and assert both succeed and return the same `status` field (read the `"integrity"` arm in `desktop_bridge.rs` to see the response shape first).

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass, 2 more than before.

## Done criteria

- [ ] `grep -n "desktop::dispatch(" crates/sarv-desktop/src/data.rs` returns nothing (only `dispatch_in` is used)
- [ ] `grep -n "Store::open" crates/sarv-desktop/src/ui/documents.rs` returns nothing
- [ ] Desktop and engine test suites pass
- [ ] `plans/README.md` updated

## STOP conditions

- `dispatch_in` or `Store::refresh_if_stale` is not reachable from the desktop crate and exposing it needs more than a `pub use`.
- A desktop test fails with "changed in another window" / "Books changed since review" after Step 2: a view is holding a stale `_generation`; report the test and the action.
- `Store` is not `Send` and Rune requires `State: Send` — report rather than wrapping in unsafe.

## Maintenance notes

- The engine's `Store` has an `ensure_idle` guard used during background validation; if a call returns that error, the UI should show it and retry — do not bypass it.
- Follow-up (not in this plan): move PDF, backup and CSV export to a worker thread with a busy indicator; cache typed row models per refresh (plan 012).
- ADR-0007's promise "failed integrity status shown on the Overview" only becomes true after this plan; mention it in the PR.
