# Plan 009: Agent access decisions come from the app, not from a file the agent can edit

> **Executor instructions**: Follow this plan step by step. Run every verification command. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-quote/src/agent.rs crates/sarv-quote/src/desktop_bridge.rs crates/sarv-quote/tests/agent_access.rs`

## Status

- **Priority**: P2
- **Effort**: S–M
- **Risk**: LOW
- **Depends on**: none
- **Category**: security
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

`sarv-quote` ships a local agent interface (CLI/MCP over a loopback socket) with two levels: read-only and full. The stated rule is that access is controlled only by the application. Three gaps undermine that:

1. The read-only/full decision is read from `.agent-access.json` on disk on every request. An agent running as the same OS user can edit that file and promote itself, leaving no trace.
2. A full-access agent can **reopen** a locked fiscal period, post into it and lock it again.
3. The "preview first" rule (a post must carry the `expected_generation` from its preview) does not cover `payment_post`, although a payment posts a cash/bank journal.

Note for context: at this commit nothing in the Rune desktop starts the agent service (`grep -rn AgentService crates/sarv-desktop/src` is empty), so the agent is currently unreachable in the shipped app. Fix these before it is re-enabled. Whether to host it again is an owner decision recorded in `plans/README.md`; do not wire it into the desktop in this plan.

## Current state

- `crates/sarv-quote/src/agent.rs:80-84`:
  ```rust
  struct Policy { enabled: bool, access: Access, epoch: String }
  ```
- `agent.rs:355-381` (request handling):
  ```rust
  let disk = load_policy(&inner.directory)?;
  ensure!(!inner.shutdown.load(Ordering::Acquire) && state.policy.enabled && disk.enabled, "Agent access is off");
  ensure!(disk.epoch == state.policy.epoch && token_matches(&request.token, &state.token), "Agent connection expired or unauthorized");
  if request.action == "status" {
      return Ok(json!({"enabled":true,"access":disk.access,"transport":"local"}));
  }
  …
  ensure!(readonly || disk.access == Access::Full, "Read-only agent access");
  ```
  `state.policy` is the in-memory copy set by the application at `agent.rs:273` (`state.policy = policy.clone();`).
- `agent.rs:384-399` — the list of actions that must carry `expected_generation`:
  `opening_csv_post, receive_post, bill_post, fulfill_post, sale_return_post, purchase_return_post, transfer_post, journal_post, journal_reverse`. Missing: `payment_post`, `draft_post`, `purchase_post`, `period_lock`.
- `agent.rs:43-69` — `WRITE` list includes `"period_lock"`, `"payment_post"`, `"settings"`.
- `crates/sarv-quote/src/desktop_bridge.rs:21-27` — `actor(p)` returns `"local-agent"` when `p["_actor"] == "local-agent"`.
- `desktop_bridge.rs:1686-1700` — `"period_lock"`: `let closing = p["locked"] == true;` then `set_period_lock(period_id, closing, actor, reason)`; no generation check, reopening allowed for any actor.
- `desktop_bridge.rs:1143-1151` — the generic generation check runs only when the payload contains `expected_generation`.
- Tests: `crates/sarv-quote/tests/agent_access.rs` (416 lines) — the structural pattern for agent tests (starts `AgentService`, sends requests).

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Agent tests | `cargo test -p sarv-quote --test agent_access` | all pass |
| Engine tests | `cargo test -p sarv-quote` | all pass |
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |

## Scope

**In scope**: `crates/sarv-quote/src/agent.rs`, `crates/sarv-quote/src/desktop_bridge.rs` (only the `period_lock` arm), `crates/sarv-quote/tests/agent_access.rs`.

**Out of scope**: starting the agent from the desktop; the token scheme (sound: two UUIDv4s compared in constant time); `crates/sarv-quote/examples/desktop_qa.rs`.

## Git workflow

- Branch `advisor/009-agent-access-hardening`; commit style `fix(agent): …`.

## Steps

### Step 1: Decide access from memory

In the request handler, keep reading the disk policy only as a **revocation** signal (`disk.enabled` and matching `epoch` must still hold), but take the level from memory:

```rust
ensure!(readonly || state.policy.access == Access::Full, "Read-only agent access");
```

and report `state.policy.access` in the `status` response. Additionally `ensure!(disk.access == state.policy.access, "Agent access file was changed outside Sarv. Turn agent access off and on in Settings.")` — a mismatch is tampering, and failing closed is the safe response.

Test `read_only_agent_cannot_self_promote` (in `agent_access.rs`): enable read-only access; rewrite `.agent-access.json` on disk with `"access"` changed to the full value and the same epoch (read the file, modify the JSON, write it back); send a write action; assert it is refused; assert a read action is also refused with the tampering message.

**Verify**: `cargo test -p sarv-quote --test agent_access` → all pass including the new test.

### Step 2: Agents may lock, never reopen

In the `period_lock` arm of `desktop_bridge.rs`, before the change:

```rust
ensure!(closing || actor(&p) != "local-agent", "Only the desktop can reopen a locked fiscal period");
```

Test `agent_cannot_reopen_a_locked_period`: with full access, lock a period (allowed), then send `period_lock` with `locked:false` → refused; the period is still locked.

Check the desktop is unaffected: `cargo test -p sarv-desktop --bin sarv-desktop test_period_label_is_locked_or_open` passes.

### Step 3: Preview-first covers every financial post

Extend the `financial_post` match in `agent.rs` with `"payment_post" | "draft_post" | "purchase_post" | "period_lock"`. Update the tool descriptions in `tools()` (search for the string `Financial posts require expected_generation`) so the text lists the same actions. Check how the agent obtains a generation for `period_lock` and `draft_post` (a `workspace` read returns `_generation`; confirm with `grep -n '"_generation"' crates/sarv-quote/src/desktop_bridge.rs`) and say so in the tool text.

Test `agent_payment_requires_preview_generation`: full access; `payment_post` without `expected_generation` → refused with "Preview first and supply expected_generation"; with the generation from `payment_preview` → succeeds.

Existing tests in `agent_access.rs` that post payments/drafts without a generation will now fail: update them to preview first (that is the intended behaviour change).

### Step 4: Bound request reads

`agent.rs:350` sets a 2-second *per-read* timeout and reads up to 2 MB before checking the token; with 8 worker slots, a slow local client can hold all of them. Add a total deadline: record `Instant::now()` when the connection is accepted and, in the read loop of `read_message`, fail once 5 seconds have elapsed. Read the function first (`grep -n "fn read_message" -A30 crates/sarv-quote/src/agent.rs`).

Test `slow_client_is_dropped`: connect, send one byte, sleep 6 s, assert the socket is closed by the server (read returns 0 or an error). Mark it `#[ignore]` if it makes the suite noticeably slower, and run it once manually.

## Done criteria

- [ ] `grep -n "disk.access == Access::Full" crates/sarv-quote/src/agent.rs` returns nothing
- [ ] `cargo test -p sarv-quote --test agent_access` passes with 3 new tests (4 with the ignored one run by hand)
- [ ] `cargo test -p sarv-quote` and the desktop suite pass
- [ ] `plans/README.md` updated

## STOP conditions

- The in-memory `state.policy` is not updated on every path that changes access (e.g. a `set_access` that only writes the file). Then Step 1 would lock users out — report the code path.
- More than a handful (≈10) of existing agent tests need rewriting for Step 3.

## Maintenance notes

- This is defence in depth: an agent with filesystem access as the same user can still edit the SQLite file directly. Say so in any security note; do not describe the read-only mode as a sandbox.
- If the agent is hosted in the Rune desktop later, the Settings toggle must call the same `AgentService` methods so memory and disk stay in step.
