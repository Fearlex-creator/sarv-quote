# Plan 015: Keyboard shortcuts the README promises, and a command palette that works from the keyboard

> **Executor instructions**: Follow this plan step by step. Run every verification command. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-desktop/src/main.rs crates/sarv-desktop/src/ui/shell.rs crates/sarv-desktop/src/ui/views/settings.rs README.md`

## Status

- **Priority**: P2
- **Effort**: M
- **Risk**: LOW
- **Depends on**: 004 (calls its `open_*` functions)
- **Category**: direction (feature) / docs
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

`README.md` tells users: "Use **Ctrl + Alt** … with **N** for a new sales invoice, **O** for an order, **C** for a customer, **P** for a product, **V** for a vendor, **B** for a purchase order, **W** for a warehouse, **U** for a unit, **J** for a journal, **T** for a stock transfer, and **S** for Settings … **F9** toggles the sidebar, **F1** opens shortcut help". None of that exists in the Rune app: only ⌘/Ctrl + K, B, D, L are handled. The command palette shows a "↵" hint but cannot be driven with arrow keys or Enter, and choosing a contact or product in it only switches page instead of opening the record. Search does not fold Arabic/Persian letter variants, so `علي` does not find `علی`.

## Current state

- `crates/sarv-desktop/src/main.rs:18-54` — `handle_key(s: &mut State, ev: &rune::KeyEvent, _tx: &mut rune::Tx) -> bool`. Handles `Key::Escape` (closes palette / sidebar peek), `ev.is_command('k')`, `'b'` or `is_sidebar_toggle(ev)`, `'d'`, `'l'`. Returns `true` when consumed.
- Rune key model (pinned revision, `crates/rune/src/edit/keys.rs`): `KeyEvent { key: Key, mods: Mods }`; `Mods { shift, cmd, alt, ctrl, logo }` where `cmd` is "⌘ on macOS, Ctrl elsewhere"; `Key::Command(char)` is "any character typed with Cmd (Ctrl off macOS), lowercased", reported as the US-layout key even on a Persian layout; `ev.is_command(c)` matches it. Other variants include `Text(String)`, `Enter`, `Up`, `Down`, `Escape`, `Tab`. The enum is `#[non_exhaustive]`; check whether function keys exist: `grep -n "F1\|Function" ~/.cargo/git/checkouts/rune-*/b556a38/crates/rune/src/edit/keys.rs`. The docs there recommend matching with a `Chord` ("mod+w") — check `grep -rn "pub struct Chord" -A20` in the same crate and prefer it if it supports `mod+alt+n`.
- `crates/sarv-desktop/src/ui/views/settings.rs:64-76` — the shortcuts tab lists four rows built with `shortcut_row(ui, "sc-k", &ui.t("Search"), &[mod_key(), "K"])`.
- Palette: `crates/sarv-desktop/src/ui/shell.rs:257-395` (`mount_palette`). Filtering: `let query = s.forms.get("palette-query").to_lowercase(); let query_latin = latin_digits(&query)…`. Party/item hits (lines ~326-373) run `st.nav(Page::Contacts)` / `st.nav(Page::Inventory)` only. Item rows are built by `palette_item(ui, key, glyph, title, hint, on_press)`.
- Record openers that already exist: contacts — `grep -n "fn open_contact\|fn open_party" crates/sarv-desktop/src/ui/views/contacts.rs`; products — `fn open_product(st: &mut State, item_id: &str)` at `ui/views/inventory.rs:510`. Make them `pub(crate)` if they are private.
- Dialog openers (after plan 004): `sales::open_sale`, `purchases::open_purchase`, `contacts::open_new_contact(st, supplier)`, `inventory::open_new_product`, `inventory::open_transfer`, `accounting::open_journal`, `payments::open_payment(st, receive)`. If plan 004 has not landed, the equivalents are the closures in `ui/views/overview.rs:175-221`.
- Named-resource creation (warehouse, unit): `overview.rs` `open_named(st, NamedKind)` (line ~223).
- Whether a text field has focus: fields are `TextEdit`s in `State.forms` with a `focused` flag (`app.rs:277` sets `self.forms.edit("pos-search").focused = true`). Add `Forms::any_focused(&self) -> bool`.
- Whether a dialog is open: each view state has `modal_open` (and others such as `transfer_open`, `ledger_open`, `business_open`, `restore_open`, `palette_open`).

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |

## Scope

**In scope**: `crates/sarv-desktop/src/main.rs`, `src/app.rs`, new `src/shortcuts.rs`, `src/i18n.rs` (search normalisation), `src/ui/shell.rs`, `src/ui/views/settings.rs`, `src/ui/views/{contacts,inventory}.rs` (visibility of openers, filter normalisation), `src/fa_messages.rs`, `README.md` (shortcut paragraph).

**Out of scope**: user-configurable key bindings; global (OS-level) hotkeys.

## Git workflow

- Branch `advisor/015-shortcuts-and-command-palette`; commits `feat(desktop): creation shortcuts`, `feat(desktop): keyboard navigation in the command menu`, `fix(desktop): Persian-aware search`.

## Steps

### Step 1: One shortcut table

Create `src/shortcuts.rs` with a single source of truth used by the key handler, the Settings list and the palette:

```rust
pub struct Shortcut { pub id: &'static str, pub label: &'static str /* i18n key */, pub keys: &'static [&'static str] /* for display */, pub run: fn(&mut State) }
pub const CREATE: &[(char, Shortcut)]  // mod+alt+<char>
```

Entries (letter → action): `n` new sales invoice, `c` new customer, `v` new supplier, `p` new product, `b` new purchase order, `j` new journal, `t` stock transfer, `w` new warehouse, `u` new unit, `r` receive money, `s` open Settings. Drop `O` ("order") from the README: the Rune app has one sale flow, not separate order and invoice entry.

### Step 2: Handle the keys

In `handle_key`:
- Creation chords: match `ev.mods.cmd && ev.mods.alt` with `Key::Command(c)` (or the `Chord` API if available). Skip when `s.forms.any_focused()` **or** any dialog is open (add `State::any_dialog_open(&self) -> bool` that ORs the flags listed above) — the README promises they "pause in text fields and dialogs".
- `F1` (or, if the pinned Rune has no function keys, `mod+/`) → navigate to Settings with `settings.tab_idx = 1`.
- `F9` → `toggle_sidebar()` if function keys exist; otherwise leave `mod+B` as the only binding.
- Esc: after the existing palette/peek branches, close the topmost open dialog (if plan 013 landed, call its `close_or_confirm`; otherwise set the flag false) and return `true`; return `false` when nothing was open.

### Step 3: Settings lists the real shortcuts

Replace the four hard-coded `shortcut_row`s with rows generated from the table plus the existing four (search, sidebar, language, theme). Display `mod_key()` + `Alt` + letter.

### Step 4: Palette keyboard navigation and deep links

- Add `palette_cursor: usize` to `State`. Build the result list as data first (`Vec<PaletteEntry { key, icon, title, hint, run: Rc<dyn Fn(&mut State)> }>`), then render; highlight the entry at the cursor (`fill(k.c.hover)`).
- In `handle_key`, when `s.palette_open`: `Key::Down`/`Key::Up` move the cursor (wrapping), `Key::Enter` runs the selected entry and closes the palette. The query field keeps focus; make sure the field does not swallow Up/Down/Enter first — test it. Reset the cursor to 0 when the query changes (compare with a stored `palette_last_query`).
- Party hit → `st.nav(Page::Contacts)` then open that contact's profile; item hit → `st.nav(Page::Inventory)` then `inventory::open_product(st, &id)`.
- Add every `CREATE` entry to the palette's action list (so "new supplier" is searchable), replacing the hand-written four-item `actions` array where they overlap.

### Step 5: Persian-aware matching

In `i18n.rs` add `pub fn fold(s: &str) -> String`: lowercase; `ي`/`ى`→`ی`; `ك`→`ک`; `ة`→`ه`; `أ`/`إ`/`آ`→`ا`; strip ZWNJ (`\u{200c}`), tatweel (`\u{640}`) and Arabic diacritics (`\u{64b}`–`\u{652}`); digits via `latin_digits`. Unit-test it. Use `fold` on both haystack and query in the palette, the Contacts filter (`party_matches`), the Inventory filter and the Purchases filter.

### Step 6: README

Rewrite the shortcut paragraph in `README.md` to list exactly what the table implements.

### Step 7: Tests (`main.rs`; pattern `test_keyboard_sidebar_and_theme`, which shows how to send key events)

1. `test_create_shortcut_opens_the_sale_dialog` and `…_is_ignored_while_typing` (focus a field first).
2. `test_palette_arrow_and_enter_run_the_selection`.
3. `test_palette_opens_the_contact_profile`.
4. `test_search_folds_arabic_yeh_and_kaf` — create a contact named `علی کریمی`, search `علي كريمي`, expect one row (pattern: `test_contacts_search_filters_the_table`).
5. `test_escape_closes_the_open_dialog`.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass, 6 new.

## Done criteria

- [ ] `grep -c "shortcut_row" crates/sarv-desktop/src/ui/views/settings.rs` shows the list is generated (one call inside a loop plus at most four fixed rows)
- [ ] `grep -n "Ctrl + Alt" README.md` lists only implemented letters
- [ ] Desktop suite passes with the new tests
- [ ] `plans/README.md` updated

## STOP conditions

- The pinned Rune delivers `mod+alt+letter` to the focused text field before `on_key` and it cannot be intercepted — report; fall back to palette-only actions and correct the README.
- The text field consumes Up/Down/Enter so the palette cursor cannot move — report the Rune behaviour; do not patch the kit.

## Maintenance notes

- New dialogs get a shortcut by adding one row to `shortcuts.rs`; Settings and the palette update automatically.
- macOS: `mods.cmd` is ⌘ there, so the chord is ⌘⌥N — matches the README's "Command + Option".
