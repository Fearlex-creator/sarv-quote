// Release builds on Windows open no console window beside the app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use serde_json::Value;
use std::path::PathBuf;
use tauri::{Emitter, Manager};

struct Profile(PathBuf);
struct Agents(std::sync::Arc<sarv_quote::agent::AgentService>);
#[tauri::command]
async fn accounting(
    state: tauri::State<'_, Agents>,
    action: String,
    payload: Value,
) -> Result<Value, Value> {
    // Agent controls are accessible only through this trusted native command.
    // The loopback bridge rejects them before invoking the accounting engine.
    let service = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || service.ui_dispatch(&action, payload))
    .await
    .map_err(|_| serde_json::json!({"code":"worker_failed","message":"Accounting worker interrupted"}))?
    .map_err(|e| serde_json::json!({"code":"accounting_error","message":e.to_string()}))
}
// Save paths come only from a native dialog in Rust, never from arbitrary JS.
#[tauri::command]
async fn save_report(
    app: tauri::AppHandle,
    state: tauri::State<'_, Profile>,
    action: String,
    payload: Value,
) -> Result<bool, String> {
    use tauri_plugin_dialog::DialogExt;
    {
        let dir = state.0.clone();
        let mut payload = payload.clone();
        let action = action.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let store = sarv_quote::store::Store::open(&dir).map_err(|e| e.to_string())?;
            sarv_quote::access::enforce(&store, &action, &mut payload).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())??;
    }
    if action == "backup_export" {
        let dir = state.0.clone();
        return tauri::async_runtime::spawn_blocking(move || {
            let store = sarv_quote::store::Store::open(&dir).map_err(|e| e.to_string())?;
            let stamp = sarv_quote::domain::now().replace(':', "").replace('+', "-");
            let Some(file) = app
                .dialog()
                .file()
                .set_file_name(format!("sarv-backup-{stamp}.json"))
                .add_filter("Sarv backup", &["json"])
                .blocking_save_file()
            else {
                return Ok(false);
            };
            let path = file.into_path().map_err(|e| e.to_string())?;
            store.backup(&path).map_err(|e| e.to_string())?;
            Ok(true)
        })
        .await
        .map_err(|e| e.to_string())?;
    }
    if action == "sales_pdf" {
        let dir = state.0.clone();
        return tauri::async_runtime::spawn_blocking(move || {
            let company_id = payload["company_id"].as_str().ok_or("Missing company")?;
            let document_id = payload["document_id"].as_str().ok_or("Missing document")?;
            let Some(file) = app.dialog().file().set_file_name("sales-document.pdf").add_filter("PDF", &["pdf"]).blocking_save_file() else { return Ok(false); };
            let path = file.into_path().map_err(|e| e.to_string())?;
            let store = sarv_quote::store::Store::open(&dir).map_err(|e| e.to_string())?;
            sarv_quote::sales_export::printable_document_pdf(&store.state, company_id, document_id, payload["locale"].as_str().unwrap_or("en"), &path).map_err(|e| e.to_string())?;
            Ok(true)
        }).await.map_err(|e| e.to_string())?;
    }
    if action == "quotation_pdf" {
        let dir = state.0.clone();
        return tauri::async_runtime::spawn_blocking(move || {
            let company_id = payload["company_id"].as_str().ok_or("Missing company")?;
            let draft_id = payload["draft_id"].as_str().ok_or("Missing quotation")?;
            let store = sarv_quote::store::Store::open(&dir).map_err(|e| e.to_string())?;
            let number = store
                .state
                .sales_drafts
                .iter()
                .find(|d| d.id == draft_id)
                .and_then(|d| d.quote_issue.as_ref())
                .map(|i| i.number.clone())
                .unwrap_or_else(|| "quotation".into());
            let file_name = format!("{}.pdf", number.replace('/', "-"));
            let Some(file) = app.dialog().file().set_file_name(file_name).add_filter("PDF", &["pdf"]).blocking_save_file() else { return Ok(false); };
            let path = file.into_path().map_err(|e| e.to_string())?;
            sarv_quote::sales_export::printable_quotation_pdf(&store.state, company_id, draft_id, payload["locale"].as_str().unwrap_or("en"), &path).map_err(|e| e.to_string())?;
            Ok(true)
        }).await.map_err(|e| e.to_string())?;
    }
    if action != "report_csv" && action != "profit_csv" {
        return Err("Unsupported export".into());
    }
    let dir = state.0.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        sarv_quote::desktop::dispatch(&dir, &action, payload)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    let name = result["filename"]
        .as_str()
        .unwrap_or("report.csv")
        .to_owned();
    let content = result["content"]
        .as_str()
        .ok_or("Missing export content")?
        .to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        let Some(file) = app
            .dialog()
            .file()
            .set_file_name(name)
            .add_filter("CSV", &["csv"])
            .blocking_save_file()
        else {
            return Ok(false);
        };
        let path = file.into_path().map_err(|e| e.to_string())?;
        sarv_quote::store::write_new(&path, content.as_bytes()).map_err(|e| e.to_string())?;
        Ok(true)
    })
    .await
    .map_err(|e| e.to_string())?
}
fn main() {
    // WebKit's DMA-BUF transport can fail with GBM allocation errors and
    // Wayland protocol errors on Linux graphics drivers. Keep an explicit
    // user choice, otherwise use its compatible rendering path.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        // This runs before GTK, Tauri or application worker threads start.
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let path = std::env::var_os("SARV_DATA_DIR")
                .map(PathBuf::from)
                .unwrap_or(app.path().app_data_dir()?.join("books"));
            sarv_quote::store::Store::open(&path)?;
            let executable = format!("sarv-agent{}", std::env::consts::EXE_SUFFIX);
            let sibling = std::env::current_exe()?.with_file_name(&executable);
            let source_cli = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join(if cfg!(debug_assertions) {
                    "../../target/debug"
                } else {
                    "../../target/release"
                })
                .join(&executable);
            let cli = std::env::var_os("SARV_AGENT_BIN")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    if sibling.is_file() {
                        sibling
                    } else {
                        source_cli
                    }
                });
            let cli = cli.canonicalize().unwrap_or(cli);
            let agents = sarv_quote::agent::AgentService::start(&path, cli)?;
            let events = app.handle().clone();
            agents.on_change(move || {
                let _ = events.emit("sarv-books-changed", ());
            });
            app.manage(Agents(std::sync::Arc::new(agents)));
            app.manage(Profile(path));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![accounting, save_report])
        .run(tauri::generate_context!())
        .expect("Sarv desktop failed");
}
