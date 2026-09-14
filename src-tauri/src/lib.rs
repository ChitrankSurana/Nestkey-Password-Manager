mod biometric_provider;
mod commands;
mod ssh_agent_service;

use pwmgr_core::paths::AppPaths;
use pwmgr_core::state::AppState;
use tauri::{Emitter, Manager};

pub fn run() {
    let home = dirs::home_dir().expect("could not resolve home directory");
    let app_paths = AppPaths::new(home.join(".pwmgr"));
    app_paths.ensure_app_dirs().expect("failed to create ~/.pwmgr directories");

    let app_state = AppState::open(&app_paths).expect("failed to open registry database");
    let ssh_agent_enabled_at_startup = app_state.config.lock().unwrap().ssh_agent_enabled;
    let ui_variant = app_state.config.lock().unwrap().ui_variant.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
        .manage(app_paths)
        .manage(ssh_agent_service::SshAgentHandle::default())
        .setup(move |app| {
            // D-03: the window is no longer declared in tauri.conf.json's static `windows` array
            // (emptied below) - it is built here from the persisted ui_variant preference instead.
            // The config value is matched against the fixed allowlist below and mapped to one of
            // two compiled-in filenames - never interpolated into a path (T-08-01).
            let entry = match ui_variant.as_str() {
                "new" => {
                    // Checkpoint decision (Task 2, 08-01-PLAN.md): "asset-guard, extended". Before
                    // pointing the window at the New UI's entry, confirm the bundled asset actually
                    // exists - a missing/stale build must never produce a blank window (T-08-03).
                    // AssetResolver::get() reads the embedded production bundle, or `dist/` directly
                    // in dev mode (requires at least one `npm run build`, same caveat Tauri's own
                    // docs state for this API in dev mode).
                    if app.asset_resolver().get("newui.html".to_string()).is_some() {
                        "newui.html"
                    } else {
                        eprintln!(
                            "warning: ui_variant is \"new\" but the newui.html asset is missing; falling back to Classic"
                        );
                        "index.html"
                    }
                }
                // Catch-all: "classic", empty/absent (already snapped by Config::load), or any
                // value that slipped past validation resolves to Classic - never a dead window.
                _ => "index.html",
            };
            // Window label MUST stay "main" - src-tauri/capabilities/default.json scopes the
            // clipboard-manager and dialog permissions to that label (D-04/T-08-02).
            tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App(entry.into()))
                .title("tauri-app")
                .inner_size(800.0, 600.0)
                .build()?;

            if ssh_agent_enabled_at_startup {
                ssh_agent_service::start(&app.handle().clone());
            }
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let tick_interval = std::time::Duration::from_secs(2);
                let mut last_tick = std::time::Instant::now();
                loop {
                    tokio::time::sleep(tick_interval).await;
                    let now = std::time::Instant::now();
                    let elapsed = now.duration_since(last_tick);
                    last_tick = now;

                    let state = handle.state::<AppState>();
                    // A suspend/resume gap forces an immediate lock regardless of the configured
                    // idle timeout - tick() alone only catches ordinary idle expiry, which could
                    // still be minutes away right after waking from sleep.
                    let was_unlocked_before_suspend_check =
                        !state.session.lock().unwrap().is_locked();
                    let suspended = was_unlocked_before_suspend_check
                        && pwmgr_core::autolock::suspend_gap_exceeds(elapsed, tick_interval);
                    if suspended {
                        pwmgr_core::commands::do_lock(&state);
                    }
                    if suspended || pwmgr_core::autolock::tick(&state, &pwmgr_core::session::SystemClock) {
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
            commands::has_pin,
            commands::unlock_with_pin,
            commands::set_pin,
            commands::clear_pin,
            commands::has_biometric,
            commands::unlock_with_biometric,
            commands::set_biometric,
            commands::clear_biometric,
            commands::update_biometric_enabled,
            commands::add_ssh_key,
            commands::list_ssh_keys,
            commands::delete_ssh_key,
            commands::update_ssh_agent_enabled,
            commands::lock,
            commands::touch,
            commands::verify_master_password,
            commands::list_sites,
            commands::add_site,
            commands::update_site,
            commands::soft_delete_site,
            commands::add_credential,
            commands::update_credential,
            commands::list_credentials_for_site,
            commands::search,
            commands::soft_delete_credential,
            commands::set_totp_secret,
            commands::clear_totp_secret,
            commands::totp_code,
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
            commands::update_mobile_nav_style,
            commands::update_ui_variant,
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
