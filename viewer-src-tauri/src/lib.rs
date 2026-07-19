mod commands;

use pwmgr_core::paths::AppPaths;
use pwmgr_core::state::AppState;
use tauri::{Emitter, Manager};

pub fn run() {
    let home = dirs::home_dir().expect("could not resolve home directory");
    let app_paths = AppPaths::new(home.join(".pwmgr"));
    app_paths.ensure_app_dirs().expect("failed to create ~/.pwmgr directories");

    let app_state = AppState::open(&app_paths).expect("failed to open registry database");

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(app_state)
        .manage(app_paths)
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    let state = handle.state::<AppState>();
                    if pwmgr_core::autolock::tick(&state, &pwmgr_core::session::SystemClock) {
                        if let Err(e) = handle.emit("vault-locked", ()) {
                            eprintln!("warning: failed to emit vault-locked event ({e})");
                        }
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::unlock_status,
            commands::login,
            commands::lock,
            commands::touch,
            commands::list_sites,
            commands::list_credentials_for_site,
            commands::search,
            commands::get_config,
            commands::copy_to_clipboard,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
