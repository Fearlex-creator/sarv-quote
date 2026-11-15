//! Shared table row model and stable sorting helpers.

use std::cell::{Ref, RefCell};
use std::cmp::Ordering;

use rune_kit::{SortDir, TableState};

#[derive(Default)]
struct FilterCacheInner {
    query: String,
    rows_gen: u64,
    indices: Vec<usize>,
}

/// Caches the indices of rows matching a search/filter query so per-cell callbacks
/// never re-run the filter or clone `books.list(...)`.
#[derive(Default)]
pub struct FilterCache(RefCell<FilterCacheInner>);

impl FilterCache {
    pub fn visible<'a>(&'a self, rows: &[Row], rows_gen: u64, query: &str, matches: impl Fn(&Row, &str) -> bool) -> Ref<'a, Vec<usize>> {
        {
            let mut inner = self.0.borrow_mut();
            if inner.rows_gen != rows_gen || inner.query != query {
                inner.indices.clear();
                let q = fold_text(query);
                for (idx, row) in rows.iter().enumerate() {
                    if q.is_empty() || matches(row, &q) {
                        inner.indices.push(idx);
                    }
                }
                inner.query.clear();
                inner.query.push_str(query);
                inner.rows_gen = rows_gen;
            }
        }
        Ref::map(self.0.borrow(), |inner| &inner.indices)
    }
}

/// One table row prepared for display and sorting. Built once per refresh or filter change.
#[derive(Clone, Debug)]
pub struct Row {
    pub id: String,
    /// Display text per column, already formatted (digits, money, date).
    pub cells: Vec<String>,
    /// Sort key per column: numbers sort numerically, text by normalised string.
    pub keys: Vec<SortKey>,
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum SortKey {
    Num(i128),
    Text(String),
}

impl SortKey {
    pub fn text(s: impl AsRef<str>) -> Self {
        SortKey::Text(fold_text(s.as_ref()))
    }

    pub fn num_str(s: &str) -> Self {
        SortKey::Num(s.trim().parse::<i128>().unwrap_or(0))
    }
}

/// Fold Arabic/Persian variants, diacritics, ZWNJ, digits and case for consistent sorting and filtering.
pub fn fold_text(s: &str) -> String {
    crate::i18n::fold(s.trim())
}

fn cmp_key(a: &SortKey, b: &SortKey) -> Ordering {
    match (a, b) {
        (SortKey::Num(x), SortKey::Num(y)) => x.cmp(y),
        (SortKey::Text(x), SortKey::Text(y)) => fold_text(x).cmp(&fold_text(y)),
        (SortKey::Num(_), SortKey::Text(_)) => Ordering::Less,
        (SortKey::Text(_), SortKey::Num(_)) => Ordering::Greater,
    }
}

/// Stably sort `rows` by `column` in `dir`. Equal keys preserve their existing relative order.
pub fn sort_rows(rows: &mut [Row], column: usize, dir: SortDir) {
    rows.sort_by(|a, b| {
        let ord = match (a.keys.get(column), b.keys.get(column)) {
            (Some(ka), Some(kb)) => cmp_key(ka, kb),
            (Some(_), None) => Ordering::Greater,
            (None, Some(_)) => Ordering::Less,
            (None, None) => Ordering::Equal,
        };
        dir.apply(ord)
    });
}

/// Re-apply the active sort stored in `table`, if any, using `col_index` to map column id -> key index.
pub fn reapply_sort(table: &TableState, rows: &mut [Row], col_index: impl Fn(&str) -> Option<usize>) {
    if let Some((col, dir)) = table.sort()
        && let Some(idx) = col_index(col)
    {
        sort_rows(rows, idx, dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, keys: Vec<SortKey>) -> Row {
        Row {
            id: id.into(),
            cells: Vec::new(),
            keys,
        }
    }

    #[test]
    fn sorts_numbers_ascending_and_descending() {
        let mut rows = vec![
            row("a", vec![SortKey::Num(100)]),
            row("b", vec![SortKey::Num(20)]),
            row("c", vec![SortKey::Num(-5)]),
            row("d", vec![SortKey::Num(1000)]),
        ];
        sort_rows(&mut rows, 0, SortDir::Ascending);
        let ids: Vec<_> = rows.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["c", "b", "a", "d"]);

        sort_rows(&mut rows, 0, SortDir::Descending);
        let ids: Vec<_> = rows.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["d", "a", "b", "c"]);
    }

    #[test]
    fn sorts_text_with_arabic_persian_folding_and_case() {
        let mut rows = vec![
            row("1", vec![SortKey::Text("کتاب".into())]),
            row("2", vec![SortKey::Text("آسمان".into())]),
            row("3", vec![SortKey::Text("كفش".into())]),
            row("4", vec![SortKey::Text("ياس".into())]),
            row("5", vec![SortKey::Text("یاس".into())]),
        ];
        sort_rows(&mut rows, 0, SortDir::Ascending);
        let ids: Vec<_> = rows.iter().map(|r| r.id.as_str()).collect();
        // آسمان < کتاب < كفش (folded to کفش, 'ف' > 'ت') < ياس == یاس (stable: 4 before 5)
        assert_eq!(ids, vec!["2", "1", "3", "4", "5"]);

        let mut ascii = vec![row("b", vec![SortKey::text("Beta")]), row("a", vec![SortKey::text("alpha")])];
        sort_rows(&mut ascii, 0, SortDir::Ascending);
        assert_eq!(ascii.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), vec!["a", "b"]);
    }

    #[test]
    fn sort_is_stable_for_equal_keys_in_both_directions() {
        let mut rows = vec![
            row("first", vec![SortKey::Num(50)]),
            row("other", vec![SortKey::Num(10)]),
            row("second", vec![SortKey::Num(50)]),
            row("third", vec![SortKey::Num(50)]),
        ];
        sort_rows(&mut rows, 0, SortDir::Ascending);
        assert_eq!(
            rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            vec!["other", "first", "second", "third"]
        );

        sort_rows(&mut rows, 0, SortDir::Descending);
        assert_eq!(
            rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            vec!["first", "second", "third", "other"]
        );
    }
}
