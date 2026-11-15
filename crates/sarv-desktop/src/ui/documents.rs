//! Customer documents written beside the books. The engine paints the PDF; the UI only chooses the file.

use std::path::{Path, PathBuf};

use crate::app::State;
use crate::i18n::{CalendarKind, Lang, Unit};

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

fn engine_prefs(
    st: &State,
) -> (
    sarv_quote::accounting::MoneyUnit,
    sarv_quote::domain::DateDisplay,
    sarv_quote::domain::DisplayDigits,
) {
    let unit = match st.fmt.unit {
        Unit::Toman => sarv_quote::accounting::MoneyUnit::Toman,
        Unit::Rial => sarv_quote::accounting::MoneyUnit::Rial,
    };
    let dates = match st.fmt.calendar {
        CalendarKind::Jalali => sarv_quote::domain::DateDisplay::Jalali,
        CalendarKind::Gregorian => sarv_quote::domain::DateDisplay::Iso,
    };
    let digits = if st.fmt.persian_digits {
        sarv_quote::domain::DisplayDigits::Persian
    } else {
        sarv_quote::domain::DisplayDigits::Latin
    };
    (unit, dates, digits)
}

fn exports_dir(st: &State) -> Result<PathBuf, String> {
    let dir = st.engine.dir.join("exports");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn unique_path(dir: &Path, stem: &str, id: &str, ext: &str) -> PathBuf {
    let base = format!("{stem}-{}", safe_name(id));
    let first = dir.join(format!("{base}.{ext}"));
    if !first.exists() {
        return first;
    }
    let stamp = chrono::Local::now().format("%H%M%S").to_string();
    let dated = dir.join(format!("{base}-{stamp}.{ext}"));
    if !dated.exists() {
        return dated;
    }
    for n in 1..1000 {
        let candidate = dir.join(format!("{base}-{stamp}-{n}.{ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    dir.join(format!("{base}-{stamp}-{}.{ext}", std::process::id()))
}

fn write_ext(
    st: &State,
    stem: &str,
    id: &str,
    ext: &str,
    paint: impl FnOnce(&sarv_quote::domain::State, &Path) -> Result<(), String>,
) -> Result<PathBuf, String> {
    let dir = exports_dir(st)?;
    let path = unique_path(&dir, stem, id, ext);
    st.engine.with_state(|state| paint(state, &path))?;
    Ok(path)
}

pub fn save_text_export(st: &State, filename: &str, content: &str) -> Result<PathBuf, String> {
    let dir = exports_dir(st)?;
    let path = dir.join(filename);
    std::fs::write(&path, content.as_bytes()).map_err(|e| e.to_string())?;
    Ok(path)
}

pub fn reveal(path: &Path, folder: bool) -> Result<(), String> {
    let target = if folder { path.parent().unwrap_or(path) } else { path };
    #[cfg(target_os = "linux")]
    let mut cmd = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(target);
        c
    };
    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = std::process::Command::new("open");
        c.arg(target);
        c
    };
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "start", ""]).arg(target);
        c
    };
    cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
}

pub fn write_sales_pdf(st: &State, document_id: &str) -> Result<PathBuf, String> {
    let company = st.books.company_id.clone();
    let language = locale(st);
    let id = document_id.to_string();
    write_ext(st, "invoice", document_id, "pdf", move |state, path| {
        sarv_quote::sales_export::printable_document_pdf(state, &company, &id, language, path)
            .map_err(|e| e.to_string())
    })
}

pub fn write_quotation_pdf(st: &State, draft_id: &str) -> Result<PathBuf, String> {
    let company = st.books.company_id.clone();
    let language = locale(st);
    let id = draft_id.to_string();
    write_ext(st, "quotation", draft_id, "pdf", move |state, path| {
        sarv_quote::sales_export::printable_quotation_pdf(state, &company, &id, language, path)
            .map_err(|e| e.to_string())
    })
}

pub fn write_purchase_pdf(st: &State, record_id: &str) -> Result<PathBuf, String> {
    let company = st.books.company_id.clone();
    let (unit, dates, digits) = engine_prefs(st);
    let id = record_id.to_string();
    write_ext(st, "purchase", record_id, "pdf", move |state, path| {
        sarv_quote::procurement_export::document_pdf(&state.accounting, &company, &id, unit, dates, digits, path)
            .map_err(|e| e.to_string())
    })
}

pub fn write_trial_balance_pdf(st: &State, _from: &str, to: &str) -> Result<PathBuf, String> {
    let company_id = st.books.company_id.clone();
    let company_name = {
        let name = st.books.company_name();
        if name.is_empty() { company_id.clone() } else { name }
    };
    let (unit, dates, _) = engine_prefs(st);
    let cutoff = to.to_string();
    write_ext(st, "trial-balance", to, "pdf", move |state, path| {
        let tb = state
            .accounting
            .trial_balance(&company_id, &cutoff)
            .map_err(|e| e.to_string())?;
        sarv_quote::accounting_export::trial_pdf_with_dates(&company_name, &tb, unit, dates, path)
            .map_err(|e| e.to_string())
    })
}

pub fn write_sales_document_csv(st: &State, document_id: &str) -> Result<PathBuf, String> {
    let company = st.books.company_id.clone();
    let (unit, dates, digits) = engine_prefs(st);
    let id = document_id.to_string();
    write_ext(st, "sale", document_id, "csv", move |state, path| {
        sarv_quote::sales_export::document_csv(&state.accounting, &company, &id, unit, dates, digits, path)
            .map_err(|e| e.to_string())
    })
}

pub fn write_purchase_document_csv(st: &State, record_id: &str) -> Result<PathBuf, String> {
    let company = st.books.company_id.clone();
    let (unit, dates, digits) = engine_prefs(st);
    let id = record_id.to_string();
    write_ext(st, "purchase", record_id, "csv", move |state, path| {
        sarv_quote::procurement_export::document_csv(&state.accounting, &company, &id, unit, dates, digits, path)
            .map_err(|e| e.to_string())
    })
}
