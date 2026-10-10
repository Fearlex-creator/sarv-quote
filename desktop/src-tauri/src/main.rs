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
