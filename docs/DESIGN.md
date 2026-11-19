# Sarv Design System

Native desktop UI specification for `crates/sarv-desktop` (Rune + `rune-kit`).
For the historical v1 design record, see [design-gtk-v1.md](milestones/design-gtk-v1.md).

## 1. Principles

From ADR-0001 (`docs/adr/0001-keep-sarv-identity.md`):

- Minimal, easy to use, calm neutrals, hairlines, **one accent (teal)**, clear hierarchy, short ease-out motion.
- Restraint over decoration: do not imitate Apple visuals (no system blue, no SF fonts).

## 2. Colour

`sarv_kit(dark)` in `crates/sarv-desktop/src/ui/kit.rs` starts from `Kit::neutral(dark)` and applies `sarv_accent(dark)`:

| Mode | Value | Token fields set |
|---|---|---|
| Light | `rune::oklch(0.511, 0.096, 186.4)` | `bg_accent`, `text_accent`, `icon_accent`, `border_focus`, `border_active` |
| Dark | `rune::oklch(0.72, 0.10, 186.0)` | `bg_accent`, `text_accent`, `icon_accent`, `border_focus`, `border_active` |

- **Where teal is allowed**: focus rings, active selection, links, accent badges, and the one primary action per view. (`ButtonVariant::Submit` in `rune-kit` currently paints from `bg_contrast` shared with contrast icon buttons, so `Btn::Primary` remains neutral until `rune-kit` separates those tokens.)
- **Status colours**: sourced from `Palette::{success, warning, danger, info}` (`StateColors { bg, fg, border, icon }`) via `Ui::tone_fg`, `Ui::tone_accent`, and `Ui::badge`.
- **Accounting rule**: debit and credit columns/amounts are never coloured red or green.

## 3. Type

- **Font family**: `Vazirmatn` (`Regular`, `Medium`, `SemiBold`, `Bold` embedded in `crates/sarv-desktop/src/main.rs`) is used for all UI text today, covering both Persian and Latin glyphs. (ADR-0001 specifies IBM Plex Sans and Space Grotesk for Latin text; they will be added once the font files and licences are placed in `assets/fonts/`.)
- **Numerals**: tabular numerals for money and numeric columns.
- **Text helpers** (`crates/sarv-desktop/src/ui/glass.rs`):
  - `Ui::body` (`ty.ui`, `c.text_base`) — default UI copy and field values
  - `Ui::muted` (`ty.caption()`, `c.text_muted`) — secondary descriptions, subtitles, and hints
  - `Ui::heading` (`ty.ui_medium`, `c.text_base`) — subsection headings inside cards and dialogs
  - `Ui::title` (`ty.body_medium`, `c.text_base`) — prominent titles and metric values
  - `Ui::cell` (`ty.ui`, `c.text_base`, truncated) — primary table cell text
  - `Ui::cell_muted` (`ty.ui`, `c.text_muted`, truncated) — secondary table cell text

## 4. Layout

- **Page**: `Ui::page` applies `px(24.0)`, `pt(8.0)`, `pb(16.0)`, `gap(16.0)`, and `grow(1.0).min_h(0.0)` on the body so a child `Table` acts as the scroll viewport.
- **Sections & lists**: `Ui::section` (`gap(8.0)` with `text_xsmall_medium` muted title), `Ui::panel_rows` (`layer_01`, `radius.md`, `0.5` `border_base`, `elevation.raised`, `Divider::horizontal()` between rows), `Ui::fill_or_empty` (switches between `Ui::empty` and the table).
- **Form grid**: `Ui::form_grid` lays out two equal-width columns (`gap(12.0)`, `grow(1.0).min_w(0.0)`).
- **Dialog & sheet tokens** (`crates/sarv-desktop/src/ui/glass.rs`):
  - `DIALOG_SM = 480.0` — confirmations and single-field forms
  - `DIALOG_MD = 640.0` — standard two-column forms and detail sheets
  - `DIALOG_LG = 900.0` — line editors and two `ACCOUNT_PICKER_WIDTH` pickers side by side (journal and banking dialogs)
  - `SHEET_RADIUS = 14.0` — corner radius for dialogs and `Ui::frost` sheets
- **Picker widths** (`crates/sarv-desktop/src/ui/pickers.rs`): `PICKER_WIDTH = 280.0`, `ACCOUNT_PICKER_WIDTH = 420.0`.

## 5. Components

| Component | Usage rule |
|---|---|
| `Btn::Primary` | One primary action per view or dialog |
| `Btn::Glass` | Secondary actions in toolbars and dialogs |
| `Btn::Ghost` | Cancel, dismiss, and quiet inline actions |
| `Btn::Danger` | Destructive confirmation only |

- `Ui::badge(tone, label)`: `Tone::Info` uses `BadgeVariant::Accent`; `Success`, `Warning`, and `Danger` render with `k.c.<tone>.{bg, border, fg}`, `radius.sm`, `0.5` border, `h(18.0).px(6.0)`, and `ty.text_xsmall_medium`.
- `Ui::metric`: `Card` with `tone_accent(tone)` left rule, icon, title, subtitle, and `body_medium` value.
- `Ui::empty`: `Card` with icon, translated title, and description.
- `Ui::dialog` / `Ui::dialog_described`: `SHEET_RADIUS`, `layer_01`, `0.5` `border_base`, `elevation.overlay`, `max_h(420.0)` scrolled body, dismiss button (`{key}-close`), and trailing `portals` so `Combobox`/`Select` menus paint above the sheet.
- `Ui::form_dialog`: same chrome as `Ui::dialog_described` with backdrop click-outside disabled so dirty forms are closed only via Cancel, `{key}-close`, or Escape (routing through `on_close`).
- `Ui::secret_field` / `Ui::secret_field_submit`: masked `TextInput::secure()` with toggleable `revealed` state and key `field-{key}`.
- `Ui::money_field`: `Ui::field` paired with the active unit suffix (`toman` / `Rial`) and digit-formatted zero placeholder.
- `Ui::date_field`: `Ui::field` with today's formatted date as placeholder, live parsed-date preview, fiscal-year range warning (`Tone::Warning`), and parse error caption (`Tone::Danger`), wrapped in `Ui::labeled`.
- `Ui::hinted`: wraps a control in `Ui::labeled` with an optional tone-coloured advisory caption (`hint-{label_key}`).
- `Ui::tabs`: `SegmentedControl` with `.fit()`.
- Toasts (`shell::mount_toasts`): `ToastRegion` with tone-matched lead icon (`CHECK` for `Success`, `WARNING` for `Warning`/`Danger`, `INFO` for `Info`) coloured with `ui.tone_accent(t.tone)`.

## 6. RTL, digits, dates, money

- Root direction (`Direction::Rtl` / `Direction::Ltr`) follows `s.lang().is_rtl()`.
- Money is bidi-isolated by `Fmt::money` and stored in books as integer rials (`Toman` display unit is sent as-is on sales drafts; purchases, payments, journals, and banking convert toman to rials before sending).
- Digits (Persian/Latin) and calendar (Jalali/Gregorian) follow Settings.
- Latin identifiers (codes, references, filenames) are wrapped with `i18n::isolate`.

## 7. Naming for tests

Deterministic element keys used by desktop tests:

- Buttons: `btn-{English label key}`
- Form fields: `field-{form key}`
- Sidebar navigation: `nav-{page}` (3 sections: `nav-group-daily`, `nav-group-people`, `nav-group-books`)
- Sidebar footer: `org-selector` (company & fiscal year switcher) and `nav-settings`
- Dialogs: `{dialog}-dialog` and `{dialog}-close`

## 8. Dialog contract

- One `open_*` function per dialog: clears its form prefix, mints a fresh `request_key`, and sets `posting = false`.
- Save previews via `sarv_quote::desktop::dispatch`, copies `_generation` to `expected_generation`, posts with `State::run`, and closes the dialog only on success.
- Never call `toast` after `State::run` (it already surfaces success/error feedback).

## 9. Not allowed

- Hard-coded `Color::` literals or numeric dialog widths/sheet radii in views
- Hard-coded Persian placeholder strings
- New accent colours
- Apple system blue or SF fonts
