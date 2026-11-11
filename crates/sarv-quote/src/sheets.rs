//! A working spreadsheet stored beside the ledger. Cells never post a journal.

use std::collections::{BTreeMap, HashSet};

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workbook {
    pub sheets: Vec<Sheet>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sheet {
    pub name: String,
    /// Address (`A1`) to the raw text the user stored. A leading `=` is a formula.
    pub cells: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Number {
    pub value: i64,
    /// True when the number was read from the books, so the screen can show it as money.
    pub money: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CellValue {
    Blank,
    Text(String),
    Number(Number),
}

/// Reject a workbook that is not a small grid of text cells.
pub fn validate_workbook(value: &Value) -> Result<()> {
    let sheets = value["sheets"].as_array().ok_or_else(|| anyhow::anyhow!("Workbook has no sheets"))?;
    ensure!((1..=12).contains(&sheets.len()), "A workbook has between 1 and 12 sheets");
    let mut count = 0usize;
    for sheet in sheets {
        let name = sheet["name"].as_str().unwrap_or("");
        ensure!(!name.trim().is_empty() && name.chars().count() <= 40, "Sheet name is empty or too long");
        let cells = sheet["cells"].as_object().ok_or_else(|| anyhow::anyhow!("Sheet cells are missing"))?;
        count = count.saturating_add(cells.len());
        ensure!(count <= 4_000, "A workbook holds at most 4000 cells");
        for (addr, raw) in cells {
            ensure!(parse_addr(addr).is_some(), "Cell address is not on the sheet");
            let text = raw.as_str().ok_or_else(|| anyhow::anyhow!("A cell must be text"))?;
            ensure!(text.chars().count() <= 500, "A cell is too long");
        }
    }
    Ok(())
}

pub fn parse_addr(raw: &str) -> Option<(u32, u32)> {
    let bytes = raw.trim().as_bytes();
    let mut i = 0;
    let mut col = 0u32;
    while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
        if i == 2 {
            return None;
        }
        col = col.checked_mul(26)?.checked_add(u32::from(bytes[i].to_ascii_uppercase() - b'A' + 1))?;
        i += 1;
    }
    if col == 0 || col > 26 * 26 || i == bytes.len() {
        return None;
    }
    let row: u32 = std::str::from_utf8(&bytes[i..]).ok()?.parse().ok()?;
    if row == 0 || row > 200 {
        return None;
    }
    Some((col - 1, row - 1))
}

pub fn addr(col: u32, row: u32) -> String {
    let mut n = col;
    let mut letters = String::new();
    loop {
        letters.insert(0, char::from(b'A' + (n % 26) as u8));
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    format!("{letters}{}", row + 1)
}

/// Evaluate one stored cell. `cell` returns the raw text of an address. `books` returns a
/// rial figure for `cash`, `receivables`, `payables`, or `profit`.
pub fn eval_cell(
    raw: &str,
    cell: &mut dyn FnMut(&str) -> Option<String>,
    books: &mut dyn FnMut(&str) -> Option<i64>,
) -> Result<CellValue, ()> {
    let mut stack = HashSet::new();
    value_of(raw, cell, books, &mut stack)
}

fn value_of(
    raw: &str,
    cell: &mut dyn FnMut(&str) -> Option<String>,
    books: &mut dyn FnMut(&str) -> Option<i64>,
    stack: &mut HashSet<String>,
) -> Result<CellValue, ()> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(CellValue::Blank);
    }
    if let Some(formula) = raw.strip_prefix('=') {
        return eval_expr(formula.trim(), cell, books, stack).map(CellValue::Number);
    }
    if let Some(n) = parse_number(raw) {
        return Ok(CellValue::Number(Number { value: n, money: false }));
    }
    Ok(CellValue::Text(raw.to_string()))
}

fn eval_expr(
    formula: &str,
    cell: &mut dyn FnMut(&str) -> Option<String>,
    books: &mut dyn FnMut(&str) -> Option<i64>,
    stack: &mut HashSet<String>,
) -> Result<Number, ()> {
    let mut p = Parser { s: formula.as_bytes(), i: 0 };
    let n = p.expr(cell, books, stack)?;
    p.skip();
    if p.i != p.s.len() {
        return Err(());
    }
    Ok(n)
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> Parser<'a> {
    fn skip(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }
    fn peek(&mut self) -> Option<u8> {
        self.skip();
        self.s.get(self.i).copied()
    }
    fn bump(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.i += 1;
        Some(c)
    }
    fn expr(
        &mut self,
        cell: &mut dyn FnMut(&str) -> Option<String>,
        books: &mut dyn FnMut(&str) -> Option<i64>,
        stack: &mut HashSet<String>,
    ) -> Result<Number, ()> {
        let mut left = self.term(cell, books, stack)?;
        while let Some(op) = self.peek() {
            if op != b'+' && op != b'-' {
                break;
            }
            self.bump();
            let right = self.term(cell, books, stack)?;
            let value = if op == b'+' { left.value.checked_add(right.value) } else { left.value.checked_sub(right.value) };
            left = Number { value: value.ok_or(())?, money: left.money || right.money };
        }
        Ok(left)
    }
    fn term(
        &mut self,
        cell: &mut dyn FnMut(&str) -> Option<String>,
        books: &mut dyn FnMut(&str) -> Option<i64>,
        stack: &mut HashSet<String>,
    ) -> Result<Number, ()> {
        let mut left = self.factor(cell, books, stack)?;
        while let Some(op) = self.peek() {
            if op != b'*' && op != b'/' {
                break;
            }
            self.bump();
            let right = self.factor(cell, books, stack)?;
            let value = if op == b'*' {
                left.value.checked_mul(right.value)
            } else {
                left.value.checked_div(right.value)
            };
            left = Number { value: value.ok_or(())?, money: left.money || right.money };
        }
        Ok(left)
    }
    fn factor(
        &mut self,
        cell: &mut dyn FnMut(&str) -> Option<String>,
        books: &mut dyn FnMut(&str) -> Option<i64>,
        stack: &mut HashSet<String>,
    ) -> Result<Number, ()> {
        let c = self.peek().ok_or(())?;
        if c == b'+' {
            self.bump();
            return self.factor(cell, books, stack);
        }
        if c == b'-' {
            self.bump();
            let n = self.factor(cell, books, stack)?;
            return Ok(Number { value: n.value.checked_neg().ok_or(())?, money: n.money });
        }
        if c == b'(' {
            self.bump();
            let n = self.expr(cell, books, stack)?;
            if self.bump() != Some(b')') {
                return Err(());
            }
            return Ok(n);
        }
        if c.is_ascii_digit() {
            return Ok(Number { value: self.number()?, money: false });
        }
        if c.is_ascii_alphabetic() {
            let name = self.word()?;
            if self.peek() == Some(b'(') {
                return self.function(&name, cell, books, stack);
            }
            let row = self.number()?;
            let addr = format!("{name}{row}");
            return lookup(&addr, cell, books, stack);
        }
        Err(())
    }
    fn function(
        &mut self,
        name: &str,
        cell: &mut dyn FnMut(&str) -> Option<String>,
        books: &mut dyn FnMut(&str) -> Option<i64>,
        stack: &mut HashSet<String>,
    ) -> Result<Number, ()> {
        if self.bump() != Some(b'(') {
            return Err(());
        }
        let upper = name.to_ascii_uppercase();
        let number = if upper == "SUM" {
            let (c1, r1) = self.addr()?;
            if self.bump() != Some(b':') {
                return Err(());
            }
            let (c2, r2) = self.addr()?;
            let (c1, c2) = (c1.min(c2), c1.max(c2));
            let (r1, r2) = (r1.min(r2), r1.max(r2));
            if (c2 - c1 + 1) * (r2 - r1 + 1) > 500 {
                return Err(());
            }
            let mut total = Number { value: 0, money: false };
            for c in c1..=c2 {
                for r in r1..=r2 {
                    let n = lookup(&addr(c, r), cell, books, stack)?;
                    total.value = total.value.checked_add(n.value).ok_or(())?;
                    total.money = total.money || n.money;
                }
            }
            total
        } else if upper == "BOOKS" {
            let key = self.quoted()?.to_ascii_lowercase();
            Number { value: books(&key).ok_or(())?, money: true }
        } else {
            return Err(());
        };
        if self.bump() != Some(b')') {
            return Err(());
        }
        Ok(number)
    }
    fn addr(&mut self) -> Result<(u32, u32), ()> {
        let name = self.word()?;
        let row = self.number()?;
        parse_addr(&format!("{name}{row}")).ok_or(())
    }
    fn word(&mut self) -> Result<String, ()> {
        self.skip();
        let start = self.i;
        while self.i < self.s.len() && self.s[self.i].is_ascii_alphabetic() {
            self.i += 1;
        }
        if start == self.i {
            return Err(());
        }
        Ok(std::str::from_utf8(&self.s[start..self.i]).unwrap_or("").to_ascii_uppercase())
    }
    fn number(&mut self) -> Result<i64, ()> {
        self.skip();
        let start = self.i;
        while self.i < self.s.len() && self.s[self.i].is_ascii_digit() {
            self.i += 1;
        }
        if start == self.i {
            return Err(());
        }
        std::str::from_utf8(&self.s[start..self.i]).ok().and_then(|s| s.parse().ok()).ok_or(())
    }
    fn quoted(&mut self) -> Result<String, ()> {
        let quote = self.bump().ok_or(())?;
        if quote != b'"' && quote != b'\'' {
            return Err(());
        }
        let start = self.i;
        while self.i < self.s.len() && self.s[self.i] != quote {
            self.i += 1;
        }
        if self.i >= self.s.len() {
            return Err(());
        }
        let text = std::str::from_utf8(&self.s[start..self.i]).unwrap_or("").to_string();
        self.i += 1;
        Ok(text)
    }
}

fn lookup(
    address: &str,
    cell: &mut dyn FnMut(&str) -> Option<String>,
    books: &mut dyn FnMut(&str) -> Option<i64>,
    stack: &mut HashSet<String>,
) -> Result<Number, ()> {
    if !stack.insert(address.to_string()) {
        return Err(());
    }
    let raw = cell(address).unwrap_or_default();
    let value = value_of(&raw, cell, books, stack);
    stack.remove(address);
    match value? {
        CellValue::Blank => Ok(Number { value: 0, money: false }),
        CellValue::Number(n) => Ok(n),
        CellValue::Text(_) => Err(()),
    }
}

fn parse_number(raw: &str) -> Option<i64> {
    let trimmed = raw.trim().replace([',', '٬'], "");
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn cells(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect()
    }

    fn run(raw: &str, grid: &HashMap<String, String>, books: &[(&str, i64)]) -> CellValue {
        let mut books_of = |key: &str| books.iter().find(|(k, _)| *k == key).map(|(_, n)| *n);
        let mut cell_of = |addr: &str| grid.get(addr).cloned();
        eval_cell(raw, &mut cell_of, &mut books_of).unwrap_or(CellValue::Text("err".into()))
    }

    #[test]
    fn formulas_sum_cells_and_read_the_books() {
        let grid = cells(&[("A1", "10"), ("A2", "25"), ("B1", "=A1+A2"), ("C1", "=SUM(A1:A2)"), ("D1", "=BOOKS(\"cash\")")]);
        assert_eq!(run("=B1*2", &grid, &[("cash", 1500)]), CellValue::Number(Number { value: 70, money: false }));
        assert_eq!(run("=C1", &grid, &[]), CellValue::Number(Number { value: 35, money: false }));
        assert_eq!(run("=D1", &grid, &[("cash", 1500)]), CellValue::Number(Number { value: 1500, money: true }));
        assert_eq!(run("=A1/0", &grid, &[]), CellValue::Text("err".into()));
    }

    #[test]
    fn a_cycle_does_not_loop() {
        let grid = cells(&[("A1", "=B1"), ("B1", "=A1")]);
        assert_eq!(run("=A1", &grid, &[]), CellValue::Text("err".into()));
    }

    #[test]
    fn saving_a_workbook_leaves_the_ledger_generation() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path()).unwrap();
        let generation = store.local_generation();
        let book = r#"{"sheets":[{"name":"Sheet 1","cells":{"A1":"=1+2"}}]}"#;
        store.save_workbook("co", book).unwrap();
        assert_eq!(store.local_generation(), generation);
        assert_eq!(store.load_workbook("co").unwrap().as_deref(), Some(book));
        let reopened = crate::store::Store::open(dir.path()).unwrap();
        assert_eq!(reopened.load_workbook("co").unwrap().as_deref(), Some(book));
        assert_eq!(reopened.local_generation(), generation);
    }
}
