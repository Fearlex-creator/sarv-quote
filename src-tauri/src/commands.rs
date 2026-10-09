use serde_json::Value;
use std::sync::Arc;
use tauri::State;
#[tauri::command]
pub async fn desktop_view(service: State<'_, Arc<sarv_quote::desktop::DesktopHost>>, action: String, payload: Value) -> Result<Value, String> {
    let service=Arc::clone(service.inner());
    tauri::async_runtime::spawn_blocking(move || service.read(&action,payload).map_err(|e|format!("{e:#}"))).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn desktop_mutate(service: State<'_, Arc<sarv_quote::desktop::DesktopHost>>, action: String, payload: Value) -> Result<Value, String> {
    let service=Arc::clone(service.inner());
    tauri::async_runtime::spawn_blocking(move || service.mutate(&action,payload).map_err(|e|format!("{e:#}"))).await.map_err(|e|e.to_string())?
}
