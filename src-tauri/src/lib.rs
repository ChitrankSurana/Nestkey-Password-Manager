mod commands;
mod config;
mod error;
mod paths;
mod state;

use config::Config;
use paths::AppPaths;
use state::AppState;
use std::sync::Mutex;
use std::time::Duration;

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
        .manage(app_state)
        .manage(app_paths)
        .invoke_handler(tauri::generate_handler![commands::unlock_status])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
