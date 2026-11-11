//! A working spreadsheet. Figures from the books are read, never posted.

use std::collections::BTreeMap;

use rune::{Cx, El, col, row, text};
use rune_kit::TextInput;
use sarv_quote::sheets::{CellValue, Sheet, Workbook, addr, eval_cell};

use crate::app::{State, Tone};
use crate::ui::glass::{Btn, Ui};
use crate::ui::icons;

const COLS: u32 = 12;
const ROWS: u32 = 32;

pub struct SheetsState {
    pub book: Workbook,
    pub active: usize,
    pub col: u32,
    pub row: u32,
    pub loaded: bool,
    pub loaded_for: String,
    pub dirty: bool,
}

impl Default for SheetsState {
    fn default() -> Self {
        Self {
            book: Workbook { sheets: vec![blank_sheet("Sheet 1")] },
            active: 0,
            col: 0,
            row: 0,
            loaded: false,
            loaded_for: String::new(),
            dirty: false,
        }
    }
}

fn blank_sheet(name: &str) -> Sheet {
    Sheet { name: name.to_string(), cells: BTreeMap::new() }
}

fn starter_cells(st: &State) -> BTreeMap<String, String> {
    let mut cells = BTreeMap::new();
    let rows = [
        ("A1", st.fmt.t("Cash and bank"), "B1", "=BOOKS(\"cash\")".to_string()),
        ("A2", st.fmt.t("Receivables"), "B2", "=BOOKS(\"receivables\")".to_string()),
        ("A3", st.fmt.t("Payables"), "B3", "=BOOKS(\"payables\")".to_string()),
        ("A4", st.fmt.t("Profit this period"), "B4", "=BOOKS(\"profit\")".to_string()),
    ];
    for (a, label, b, formula) in rows {
        cells.insert(a.to_string(), label);
        cells.insert(b.to_string(), formula);
    }
    cells.insert("A6".to_string(), st.fmt.t("Notes"));
    cells
}

fn starter(st: &State) -> Workbook {
    Workbook { sheets: vec![Sheet { name: st.fmt.t("Sheet 1"), cells: starter_cells(st) }] }
}

fn fill_from_books(st: &mut State) {
    if st.sheets.book.sheets.is_empty() {
        return;
    }
    let cells = starter_cells(st);
    let sheet = current_mut(st);
    if !sheet.cells.is_empty() {
        return;
    }
    sheet.cells = cells;
    st.sheets.dirty = true;
    sync_fields(st);
}

fn current<'a>(st: &'a State) -> &'a Sheet {
    st.sheets.book.sheets.get(st.sheets.active).unwrap_or(&st.sheets.book.sheets[0])
}

fn current_mut(st: &mut State) -> &mut Sheet {
    let i = st.sheets.active.min(st.sheets.book.sheets.len().saturating_sub(1));
    st.sheets.active = i;
    &mut st.sheets.book.sheets[i]
}

pub fn ensure(st: &mut State) {
    if st.sheets.loaded && st.sheets.loaded_for == st.books.company_id {
        return;
    }
    match st.query("sheet_load", serde_json::json!({})) {
        Ok(v) if v["workbook"].is_null() => {
            st.sheets.book = starter(st);
            finish_load(st, false);
        }
        Ok(v) => match serde_json::from_value::<Workbook>(v["workbook"].clone()) {
            Ok(book) if !book.sheets.is_empty() => {
                st.sheets.book = book;
                finish_load(st, false);
            }
            _ => {
                st.sheets.book = starter(st);
                finish_load(st, false);
            }
        },
        Err(e) => {
            st.toast(Tone::Danger, e);
            st.sheets.loaded = false;
        }
    }
}

fn finish_load(st: &mut State, dirty: bool) {
    st.sheets.active = 0;
    st.sheets.col = 0;
    st.sheets.row = 0;
    st.sheets.loaded = true;
    st.sheets.loaded_for = st.books.company_id.clone();
    st.sheets.dirty = dirty;
    sync_fields(st);
}

fn sync_fields(st: &mut State) {
    let address = addr(st.sheets.col, st.sheets.row);
    let raw = current(st).cells.get(&address).cloned().unwrap_or_default();
    let name = current(st).name.clone();
    st.forms.set("sheet-formula", &raw);
    st.forms.set("sheet-name", &name);
}

fn commit_name(st: &mut State) {
    let name = st.forms.get("sheet-name").trim().to_string();
    if name.is_empty() || st.sheets.book.sheets.is_empty() {
        return;
    }
    let sheet = current_mut(st);
    if sheet.name != name {
        sheet.name = name;
        st.sheets.dirty = true;
    }
}

fn apply_formula(st: &mut State) {
    if st.sheets.book.sheets.is_empty() {
        return;
    }
    let raw = st.forms.get("sheet-formula").trim().to_string();
    let address = addr(st.sheets.col, st.sheets.row);
    let sheet = current_mut(st);
    let previous = sheet.cells.get(&address).cloned().unwrap_or_default();
    if raw.is_empty() {
        if sheet.cells.remove(&address).is_some() {
            st.sheets.dirty = true;
        }
    } else if previous != raw {
        sheet.cells.insert(address, raw);
        st.sheets.dirty = true;
    }
}

pub fn pick(st: &mut State, col: u32, row: u32) {
    apply_formula(st);
    commit_name(st);
    st.sheets.col = col;
    st.sheets.row = row;
    sync_fields(st);
}

fn book_rial(st: &State, key: &str) -> Option<i64> {
    let raw = match key {
        "cash" => st.books.position("cash"),
        "receivables" => st.books.position("receivables"),
        "payables" => st.books.position("payables"),
        "profit" => crate::data::s(&st.books.report["profit_loss"]["profit"]),
        _ => return None,
    };
    if raw.is_empty() { None } else { raw.parse().ok() }
}

fn grouped(n: i64) -> String {
    let neg = n < 0;
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    let mut text: String = out.chars().rev().collect();
    if neg {
        text.insert(0, '-');
    }
    text
}

fn shown(st: &State, raw: &str) -> String {
    let cash = book_rial(st, "cash");
    let receivables = book_rial(st, "receivables");
    let payables = book_rial(st, "payables");
    let profit = book_rial(st, "profit");
    let grid = &current(st).cells;
    let mut cell = |address: &str| grid.get(address).cloned();
    let mut books = |key: &str| match key {
        "cash" => cash,
        "receivables" => receivables,
        "payables" => payables,
        "profit" => profit,
        _ => None,
    };
    match eval_cell(raw, &mut cell, &mut books) {
        Ok(CellValue::Blank) => String::new(),
        Ok(CellValue::Text(t)) => t,
        Ok(CellValue::Number(n)) if n.money => st.fmt.money(&n.value.to_string()),
        Ok(CellValue::Number(n)) => st.fmt.digits(&grouped(n.value)),
        Err(()) => "—".into(),
    }
}

fn save(st: &mut State) {
    if !st.sheets.loaded {
        return;
    }
    apply_formula(st);
    commit_name(st);
    let Ok(workbook) = serde_json::to_value(&st.sheets.book) else {
        return;
    };
    if st.run("sheet_save", serde_json::json!({ "workbook": workbook }), Some("Sheet saved")).is_ok() {
        st.sheets.dirty = false;
    }
}

fn add_sheet(st: &mut State) {
    if st.sheets.book.sheets.len() >= 12 {
        return;
    }
    apply_formula(st);
    commit_name(st);
    let n = st.sheets.book.sheets.len() + 1;
    st.sheets.book.sheets.push(blank_sheet(&format!("{} {n}", st.fmt.t("Sheet"))));
    st.sheets.active = st.sheets.book.sheets.len() - 1;
    st.sheets.col = 0;
    st.sheets.row = 0;
    st.sheets.dirty = true;
    sync_fields(st);
}

pub fn view(s: &State, _cx: &mut Cx<State>, ui: &Ui, _layer: &mut Vec<El>) -> El {
    let address = addr(s.sheets.col, s.sheets.row);
    let formula = row()
        .key("sheet-formula-bar")
        .w_full()
        .gap(8.0)
        .center_y()
        .child(
            row()
                .key("sheet-address")
                .w(72.0)
                .h(32.0)
                .radius(8.0)
                .center()
                .fill(ui.k.c.layer_02)
                .child(text(address).font(ui.k.ty.ui_medium).color(ui.k.c.text_base)),
        )
        .child(formula_field(ui, s))
        .child(ui.button(Btn::Glass, "sheet-apply", &ui.t("Apply"), Some(&icons::CHECK), apply_formula));

    let mut letters = row().key("sheet-columns").gap(0.0).child(row().w(44.0).h(24.0));
    for col in 0..COLS {
        letters = letters.child(
            row()
                .w(112.0)
                .h(24.0)
                .center()
                .child(ui.muted(addr(col, 0).trim_end_matches('1').to_string())),
        );
    }

    let mut grid = col().key("sheet-grid").w_full().gap(0.0);
    for row_i in 0..ROWS {
        let mut line = row().key(format!("sheet-row-{}", row_i + 1)).gap(0.0).child(
            row().w(44.0).h(28.0).center().child(ui.muted(ui.f.digits(&(row_i + 1).to_string()))),
        );
        for col in 0..COLS {
            let address = addr(col, row_i);
            let raw = current(s).cells.get(&address).map(String::as_str).unwrap_or("");
            let value = shown(s, raw);
            let selected = s.sheets.col == col && s.sheets.row == row_i;
            let mut cell = row()
                .key(format!("cell-{address}"))
                .w(112.0)
                .h(28.0)
                .px(6.0)
                .center_y()
                .border(0.5, if selected { ui.k.c.border_focus } else { ui.k.c.border_base })
                .on_press(move |st: &mut State| pick(st, col, row_i));
            if selected {
                cell = cell.fill(ui.k.c.layer_03);
            }
            let ink = if value == "—" { ui.k.c.text_muted } else { ui.k.c.text_base };
            line = line.child(cell.child(text(value).font(ui.k.ty.ui).color(ink).truncate()));
        }
        grid = grid.child(line);
    }

    let mut tabs = row().key("sheet-tabs").w_full().gap(6.0).center_y().flex_wrap();
    for (i, sheet) in s.sheets.book.sheets.iter().enumerate() {
        let kind = if i == s.sheets.active { Btn::Primary } else { Btn::Ghost };
        tabs = tabs.child(ui.button(kind, &format!("sheet-tab-{i}"), &sheet.name, None, move |st| {
            apply_formula(st);
            commit_name(st);
            st.sheets.active = i;
            st.sheets.col = 0;
            st.sheets.row = 0;
            sync_fields(st);
        }));
    }
    let sheet_empty = current(s).cells.is_empty();
    tabs = tabs.child(ui.compact_btn(Btn::Glass, "Add sheet", add_sheet)).child(ui.field(s, "sheet-name", "").w(160.0));
    if sheet_empty {
        tabs = tabs.child(ui.btn(Btn::Primary, "Fill from the books", Some(&icons::CHECK), fill_from_books));
    }
    let save_kind = if sheet_empty { Btn::Glass } else { Btn::Primary };
    tabs = tabs.child(el_grow()).child(ui.btn(save_kind, "Save sheet", Some(&icons::CHECK), save));

    let grid = col()
        .w_full()
        .min_w(0.0)
        .gap(0.0)
        .child(letters)
        .child(grid)
        .scroll_xy()
        .grow(1.0)
        .min_h(0.0);
    let mut sheet = col().w_full().gap(10.0).grow(1.0).min_h(0.0).child(formula);
    if sheet_empty {
        sheet = sheet.child(
            ui.kit_card()
                .title(ui.t("This sheet is empty"), Some(&icons::SHEETS))
                .description(ui.t("Bring cash, receivables, and profit in from the books."))
                .build(&ui.k)
                .key("sheet-empty"),
        );
    } else {
        sheet = sheet.child(ui.muted(ui.t("Type =A1+B1, =SUM(A1:A10), or =BOOKS(\"cash\") for a figure from the books.")));
    }
    let sheet = sheet.child(grid);
    let body = col()
        .w_full()
        .min_w(0.0)
        .gap(10.0)
        .grow(1.0)
        .min_h(0.0)
        .child(ui.frost("sheet-card", sheet).grow(1.0).min_h(0.0))
        .child(tabs);

    ui.page("sheets-screen", &ui.t("Sheets"), &s.books.company_name(), Vec::new(), body)
}

fn formula_field(ui: &Ui, s: &State) -> El {
    let edit = s.forms.view("sheet-formula");
    TextInput::new(&edit)
        .placeholder("=SUM(A1:A10)".to_string())
        .on_edit(|st: &mut State| st.forms.edit("sheet-formula"))
        .build(&ui.k)
        .key("field-sheet-formula")
        .grow(1.0)
        .min_w(0.0)
}

fn el_grow() -> El {
    row().grow(1.0)
}
