# Sarv for agents

`crates/sarv-quote` is the accounting engine. `crates/sarv-desktop` is the native Rune desktop. There is no React or Tauri shell.

The UI calls `sarv_quote::desktop::dispatch`. It does not invent journal lines, tax, or document ids. Preview an action before posting it, and pass `expected_generation` when the preview returns one. A blank `request_key` is filled in by the desktop; a dialog that can be clicked twice must reuse the key it minted when it opened.

`SARV_DATA_DIR` selects the books. `SARV_DEMO=1` opens a throwaway fixture. Headless checks:

```bash
cargo test -p sarv-desktop --bin sarv-desktop
cargo test -p sarv-desktop --test png_scenes
```

`png_scenes` needs a GPU. Money on screen is isolated for bidi; the books store integer rials. A toman the user typed is sent as that toman string with `display_unit: Toman` on a sales draft. Purchases, payments, journals, and banking parse rials, so those screens convert toman to rials before sending. Operation quantities are display units. `remaining_quantity` from the engine is thousandths and must be divided by 1000 before it is sent back.

Law data is a read-only notice. It is not an input to a journal. Agents are configured from the CLI, not from a desktop post.

First run is a welcome, then the company and fiscal year, then opening balances. The sidebar stays hidden until the books are open.
