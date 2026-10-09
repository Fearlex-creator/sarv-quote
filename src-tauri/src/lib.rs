mod commands;



#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            use tauri::Manager;
            let data_dir = std::env::var_os("FREE_ACCOUNTING_DATA_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or(app.path().app_local_data_dir()?);
            let legacy_dir = std::env::var_os("SARV_DATA_DIR").map(std::path::PathBuf::from)
                .unwrap_or_else(|| {
                    let old = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data");
                    if old.join("sarv.sqlite3").exists() {old} else {data_dir.join("sarv")}
                });
            app.manage(std::sync::Arc::new(sarv_quote::desktop::DesktopHost::open(legacy_dir)?));
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::desktop_view,
            commands::desktop_mutate,
        ])
        .on_page_load(|webview, payload| {
            // Opt-in integration probe for an isolated test database. Never
            // compiled into the production application.
            #[cfg(debug_assertions)]
            if std::env::var_os("FREE_ACCOUNTING_NATIVE_SMOKE").is_some()
                && matches!(payload.event(), tauri::webview::PageLoadEvent::Finished)
            {
                let _ = webview.eval(include_str!("native-smoke.js"));
            }
            #[cfg(not(debug_assertions))]
            let _ = (webview, payload);
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
