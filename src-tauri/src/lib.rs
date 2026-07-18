mod commands;
mod dto;
mod hibp;

use pwmgr_core::config::Config;
use pwmgr_core::paths::AppPaths;
use pwmgr_core::state::AppState;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{Emitter, Manager};

pub fn run() {
    let home = dirs::home_dir().expect("could not resolve home directory");
    let app_paths = AppPaths::new(home.join(".pwmgr"));
    app_paths.ensure_app_dirs().expect("failed to create ~/.pwmgr directories");

    let config = Config::load(&app_paths.config_path()).unwrap_or_else(|e| {
        eprintln!("warning: failed to load config.json ({e}), using defaults");
        Config::default()
    });
    let timeout = Duration::from_secs(config.auto_lock_minutes as u64 * 60);

    let registry_conn = pwmgr_core::registry::open_registry(&app_paths.registry_db_path())
        .expect("failed to open registry database");

    let app_state = AppState {
        registry: Mutex::new(registry_conn),
        vault: Mutex::new(None),
        session: Mutex::new(pwmgr_core::session::Session::new(timeout)),
        config: Mutex::new(config),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
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
            commands::register_user,
            commands::login,
            commands::lock,
            commands::touch,
            commands::list_sites,
            commands::add_site,
            commands::add_credential,
            commands::update_credential,
            commands::list_credentials_for_site,
            commands::search,
            commands::soft_delete_credential,
            commands::list_trash,
            commands::restore_credential,
            commands::purge_trash,
            commands::get_config,
            commands::update_auto_lock_minutes,
            commands::update_strength_algorithm,
            commands::update_theme,
            commands::update_accent_color,
            commands::update_old_password_days,
            commands::update_hibp_opt_in,
            commands::update_nav_collapsed,
            commands::generate_password,
            commands::analyze_strength,
            commands::copy_to_clipboard,
            commands::create_backup,
            commands::restore_backup,
            commands::export_csv,
            commands::read_csv_headers,
            commands::import_csv_preview,
            commands::import_csv_commit,
            commands::health_report,
            commands::check_breach,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
