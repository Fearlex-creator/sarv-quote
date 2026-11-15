# Plan 011: The Rune desktop wears the Sarv identity again, and the design system is written down

> **Executor instructions**: Follow this plan step by step. Run every verification command. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-desktop/src/ui/kit.rs crates/sarv-desktop/src/ui/glass.rs crates/sarv-desktop/src/ui/shell.rs docs/DESIGN.md docs/FRONTEND-UI-GUIDE.md`

## Status

- **Priority**: P1 (owner has asked for this identity twice)
- **Effort**: M
- **Risk**: LOW
- **Depends on**: none
- **Category**: tech-debt / docs
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

ADR-0001 (`docs/adr/0001-keep-sarv-identity.md`) decides: "Sarv already ships a teal accent, a sprout mark and an IBM Plex Sans / Space Grotesk / Vazirmatn type system. The owner has said that redesigns must keep that identity and that 'Apple-like' means restraint, not Apple visuals." The Rune migration dropped all of it: the desktop uses rune-kit's brand-less neutral theme whose accent is **blue** (`oklch(0.6, 0.17, 256)`), there is no teal anywhere in the crate, and the brand mark is absent. This is exactly the drift the owner corrected before ("keep the sarv identity and color"). In addition, nothing documents the current design system — `docs/DESIGN.md` describes the removed GTK app — so every new screen is built by copying the nearest view.

Design direction to hold (owner's words, recorded in ADR-0001): minimal, easy to use, calm neutrals, hairlines, **one accent (teal)**, clear hierarchy, short ease-out motion. Do not imitate Apple visuals (no system blue, no SF fonts).

## Current state

- `crates/sarv-desktop/src/ui/kit.rs` (whole file, 27 lines):
  ```rust
  pub fn sarv_kit(dark: bool) -> Kit {
      let mut k = Kit::neutral(dark);
      k.ty.ui = vazir(k.ty.ui);
      … (every text style switched to Vazirmatn)
      k
  }
  ```
- rune-kit's neutral palette (`~/.cargo/git/checkouts/rune-*/b556a38/crates/rune-kit/src/neutral.rs:17-96`) sets these public `Palette` fields from one `accent` colour: `bg_accent`, `text_accent`, `icon_accent`, `border_focus`, `border_active`. `Kit.c` is a public `Palette`, so `sarv_kit` can overwrite them. Colours are built with `rune::oklch(l, c, h)`.
- The teal Sarv shipped before the migration (from `desktop/src/styles/globals.css` at git commit `f5888a16`): light `--primary: oklch(0.511 0.096 186.391)`, dark `--primary: oklch(0.437 0.078 188.216)`, primary foreground `oklch(0.984 0.014 180.72)`.
- `crates/sarv-desktop/src/ui/glass.rs`:
  - `Btn::Primary` maps to `ButtonVariant::Submit` (line 76), which the neutral kit paints with `bg_contrast` (near-black/white), not the accent.
  - `badge()` (lines 166-172): every tone except Info becomes `BadgeVariant::Subtle`, so success/warning/danger badges look identical.
  - Hard-coded numbers: `radius(14.0)` at lines 136 and 346, dialog body `max_h(420.0)` at 333.
- `crates/sarv-desktop/src/ui/shell.rs:405-407`: every toast uses `icons::INFO` regardless of tone.
- Dialog widths are literals scattered across views: 480, 520, 560, 640, 760, 900 (`grep -rn "dialog\(_described\)\?(" -A8 crates/sarv-desktop/src/ui/views | grep -E "^\S+-\s+[0-9]+\.0,"`).
- Fonts: only Vazirmatn is embedded (`crates/sarv-desktop/src/main.rs:13-16, 64-65`: `fonts: &[…]`, `families: ("Vazirmatn", "monospace")`). `assets/fonts/` contains only Vazirmatn files.
- Brand asset: `assets/brand/sarv.png` (2.5 KB). A test asserts the sidebar has **no** brand header (`main.rs:152`, `assert!(rt.find("sidebar-brand").is_none())`) — that was a deliberate tidy-up (AGENT_PLAN: "no brand header; company footer"). Keep it.
- Snapshot scenes for visual checks: `SARV_DEMO=1 cargo run -p sarv-desktop -- --scene overview --snap out.png --scale 1 --size 1180x780` (needs a GPU; scene names are listed in `main.rs:66-110`).

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |
| Snapshots (GPU) | `cargo test -p sarv-desktop --test png_scenes` | pass, or skip if no GPU and say so |

## Scope

**In scope**: `crates/sarv-desktop/src/ui/kit.rs`, `src/ui/glass.rs`, `src/ui/shell.rs` (toast icons only), `src/ui/icons.rs` (add tone icons if missing), dialog width literals in `src/ui/views/*.rs`, `docs/DESIGN.md` (rewrite), `docs/FRONTEND-UI-GUIDE.md`, `src/main.rs` (tests).

**Out of scope**:
- The Rune / rune-kit revision pin.
- Adding IBM Plex Sans / Space Grotesk font files: they are not in the repository and must be supplied by the owner under their licences (see Maintenance notes). Do not download fonts.
- Re-adding a sidebar brand header.
- Layout redesigns of individual screens.

## Git workflow

- Branch `advisor/011-sarv-identity-and-design-system`; commits `feat(desktop): restore the Sarv teal accent`, `fix(desktop): tone-aware toasts and badges`, `refactor(desktop): dialog size and radius tokens`, `docs: describe the Rune design system`.

## Steps

### Step 1: Teal accent

In `kit.rs`, after `Kit::neutral(dark)`:

```rust
/// Sarv teal (ADR-0001). Same hue the product has always used.
pub fn sarv_accent(dark: bool) -> rune::Color {
    if dark { rune::oklch(0.72, 0.10, 186.0) } else { rune::oklch(0.511, 0.096, 186.4) }
}
```

(The dark value is lightened from the old `0.437` so it reads as text and focus rings on a dark surface; the hue is unchanged.) Assign it to `k.c.bg_accent`, `k.c.text_accent`, `k.c.icon_accent`, `k.c.border_focus`, `k.c.border_active`.

Then make the primary button teal: read how `ButtonVariant::Submit` is painted (`grep -n "Submit" ~/.cargo/git/checkouts/rune-*/b556a38/crates/rune-kit/src/button.rs`) and set the palette fields it uses (expected: `bg_contrast` is shared with other controls — do **not** change that; if `Submit` cannot be tinted through the palette without affecting unrelated controls, keep `Submit` neutral and note it in the report).

Add a unit test in `kit.rs`: `sarv_kit(false).c.bg_accent == sarv_accent(false)` and the same for dark, plus an assertion that the accent differs from `rune::tok::accent(0, false)`.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass. If a GPU is available, snap `overview` and `settings` in light and dark and confirm the selected sidebar row / focus ring / accent badges are teal, not blue.

### Step 2: Tone-aware toasts and badges

- `shell.rs` `mount_toasts`: pick the lead icon by `t.tone` — Success → a check icon, Warning/Danger → a warning icon, Info → `icons::INFO`. Look in `ui/icons.rs` for existing `CHECK` / warning glyphs; add a `WARNING` icon there following the file's existing pattern if none exists.
- `glass.rs` `badge()`: `BadgeVariant` only has `Neutral`, `Accent`, `Subtle`. Keep Info → Accent; for Success/Warning/Danger build the badge by hand: a `row()` with `radius(k.radius.sm)`, `fill(self.k.c.<tone>.bg)`, `border(0.5, self.k.c.<tone>.border)`, text coloured `self.k.c.<tone>.fg` in `k.ty.text_xsmall_medium`, padding matching the kit badge (read `rune-kit/src/badge.rs` for its padding and height). `Palette` has `success`, `warning`, `danger`, `info` of type `StateColors { bg, fg, border, icon }`.

**Verify**: desktop tests pass (`test_cheque_strip_labels_are_translated_keys` exercises badges).

### Step 3: Tokens instead of literals

In `glass.rs` add:

```rust
pub const DIALOG_SM: f32 = 480.0;  // confirmations, single-field forms
pub const DIALOG_MD: f32 = 640.0;  // most forms
pub const DIALOG_LG: f32 = 900.0;  // line editors, two account pickers side by side
pub const SHEET_RADIUS: f32 = 14.0;
```

Replace the two `radius(14.0)` with `SHEET_RADIUS`. Map each dialog width literal in the views to the nearest constant: 480/520/560 → `DIALOG_SM` or `DIALOG_MD` (choose `DIALOG_MD` when the form has two columns), 640/760 → `DIALOG_MD`, 900 → `DIALOG_LG`. **Keep** the journal and banking dialogs at `DIALOG_LG`: they hold two 420-wide account pickers (recorded decision: `ACCOUNT_PICKER_WIDTH=420`). Replace `max_h(420.0)` in the dialog body with a height that follows the window: if `Cx`/`State` exposes the window size use `min(420, window_h - 220)`; otherwise leave it and note it.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass (several tests click inside dialogs; a width change must not hide a button).

### Step 4: Write `docs/DESIGN.md`

Replace the historical file's content (move the old text to `docs/milestones/design-gtk-v1.md` with `git mv` + new file, so history is kept). The new document must be derived from the code, with these sections:

1. **Principles** — the ADR-0001 direction quoted above.
2. **Colour** — the teal accent values and where it is allowed (focus, selection, links, accent badges, the one primary action per view); status colours from `Palette::{success,warning,danger,info}`; rule from the spec adopted in ADR-0001: debit/credit are never coloured red/green.
3. **Type** — Vazirmatn for all UI text today; tabular numerals for money; the `Ui` text helpers `body`, `muted`, `heading`, `title`, `cell`, `cell_muted` and when to use each.
4. **Layout** — `Ui::page` (24 px side padding, 16 px gap), `section`, `panel_rows`, `form_grid`, `fill_or_empty`; dialog sizes from Step 3; picker widths 280 / 420.
5. **Components** — table of `Btn::{Primary, Glass, Ghost, Danger}` with usage rules (one Primary per view or dialog; Danger only for destructive confirmation), `badge`, `metric`, `empty`, `dialog`/`dialog_described`, `tabs`, toasts.
6. **RTL, digits, dates, money** — root direction follows language; money is bidi-isolated by `Fmt::money`; digits and calendar follow Settings; Latin identifiers wrapped with `i18n::isolate`.
7. **Naming for tests** — element keys: `btn-{English label key}`, `field-{form key}`, `nav-{page}`, `{dialog}-dialog`, `{dialog}-close`.
8. **Dialog contract** — one `open_*` function per dialog; it clears its form prefix, mints the request key, sets `posting = false`; Save previews, copies `_generation` to `expected_generation`, posts with `State::run`, closes only on success; never toast after `run`.
9. **Not allowed** — hard-coded colours, hard-coded Persian placeholder strings, new accent colours, Apple system blue, SF fonts.

Update `docs/FRONTEND-UI-GUIDE.md` to link to it and remove "rune-kit's neutral theme" wording.

**Verify**: `grep -c "libadwaita\|GTK" docs/DESIGN.md` → 0.

## Done criteria

- [ ] `grep -rn "oklch" crates/sarv-desktop/src/ui/kit.rs` shows the teal accent
- [ ] `grep -n "radius(14.0)" crates/sarv-desktop/src/ui/glass.rs` returns nothing
- [ ] `grep -rn "icons::INFO" crates/sarv-desktop/src/ui/shell.rs` is no longer the only toast icon
- [ ] `docs/DESIGN.md` has the nine sections; desktop tests pass
- [ ] `plans/README.md` updated

## STOP conditions

- Overriding the palette fields does not change the visible accent (the kit reads `tok::accent` directly somewhere). Report which control ignores the palette; do not patch rune-kit.
- A test fails because an element moved out of the clickable area after a dialog width change — restore that dialog's width and list it as an exception in DESIGN.md.

## Maintenance notes

- **Owner decision needed (fonts)**: ADR-0001 names IBM Plex Sans and Space Grotesk for Latin text. To restore them, the owner adds the `.ttf` files and licence texts under `assets/fonts/`, then a follow-up adds them to `fonts:` in `main.rs` and sets Latin families in `kit.rs`. Until then Vazirmatn (which includes Latin glyphs) stays the single face — DESIGN.md must say so honestly.
- Reviewers should reject new `Color::` literals or numeric radii/widths in views.
