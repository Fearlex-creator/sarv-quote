//! Customer documents written beside the books. The engine paints the PDF; the UI only chooses the file.

use std::path::PathBuf;

use crate::app::State;
use crate::i18n::Lang;

fn safe_name(id: &str) -> String {
    let name: String = id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' })
        .collect();
    if name.is_empty() { "document".into() } else { name }
}

fn locale(st: &State) -> &'static str {
    if st.lang() == Lang::Fa { "fa" } else { "en" }
}

fn write(st: &State, stem: &str, id: &str, paint: impl FnOnce(&sarv_quote::domain::State, &std::path::Path) -> Result<(), String>) -> Result<PathBuf, String> {
    let dir = st.engine.dir.join("exports");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{stem}-{}.pdf", safe_name(id)));
    let store = sarv_quote::store::Store::open(&st.engine.dir).map_err(|e| e.to_string())?;
    paint(&store.state, &path)?;
    Ok(path)
}

pub fn write_sales_pdf(st: &State, document_id: &str) -> Result<PathBuf, String> {
    let company = st.books.company_id.clone();
    let language = locale(st);
    let id = document_id.to_string();
    write(st, "invoice", document_id, move |state, path| {
        sarv_quote::sales_export::printable_document_pdf(state, &company, &id, language, path).map_err(|e| e.to_string())
    })
}

pub fn write_quotation_pdf(st: &State, draft_id: &str) -> Result<PathBuf, String> {
    let company = st.books.company_id.clone();
    let language = locale(st);
    let id = draft_id.to_string();
    write(st, "quotation", draft_id, move |state, path| {
        sarv_quote::sales_export::printable_quotation_pdf(state, &company, &id, language, path).map_err(|e| e.to_string())
    })
}
