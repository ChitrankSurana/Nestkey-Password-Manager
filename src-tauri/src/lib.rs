mod biometric_provider;
mod commands;
mod ssh_agent_service;

use nestkey_core::paths::AppPaths;
use nestkey_core::state::AppState;
use tauri::{Emitter, Manager};

pub fn run() {
    let home = dirs::home_dir().expect("could not resolve home directory");
    let app_paths = AppPaths::new(home.join(".pwmgr"));
    app_paths.ensure_app_dirs().expect("failed to create ~/.pwmgr directories");

    let app_state = AppState::open(&app_paths).expect("failed to open registry database");
    let ssh_agent_enabled_at_startup = app_state.config.lock().unwrap().ssh_agent_enabled;
    let ui_variant = app_state.config.lock().unwrap().ui_variant.clone();

    tauri::Builder::default()
        // Wave 0 (2026-09-29): must be the first plugin registered - it intercepts a second
        // launch attempt before anything else initializes. Without this, launching the app while
        // an instance is already running silently spawned a second, fully independent process
        // (confirmed live: Get-Process nestkey-app going 1 -> 2 -> 3 across relaunches), which is
        // also the most likely explanation for the original "vault data loss" report - a stale
        // orphaned window surviving a terminal close, with the user then looking at a different
        // process than they think. Focuses the existing "main" window instead of building a new one.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
        .manage(app_paths)
        .manage(ssh_agent_service::SshAgentHandle::default())
        .setup(move |app| {
            // D-03: the window is no longer declared in tauri.conf.json's static `windows` array
            // (emptied below) - it is built here from the persisted ui_variant preference instead.
            // The config value is matched against the fixed allowlist below and mapped to one of
            // the compiled-in filenames below - never interpolated into a path (T-08-01).
            // Checkpoint decision (Task 2, 08-01-PLAN.md): "asset-guard, extended". Before pointing
            // the window at a variant's entry, confirm the bundled asset actually exists - a
            // missing/stale build must never produce a blank window (T-08-03). AssetResolver::get()
            // reads the embedded production bundle, or `dist/` directly in dev mode (requires at
            // least one `npm run build`, same caveat Tauri's own docs state for this API in dev mode).
            let guarded = |asset: &'static str| -> &'static str {
                if app.asset_resolver().get(asset.to_string()).is_some() {
                    asset
                } else {
                    eprintln!(
                        "warning: ui_variant needs {asset} but that asset is missing; falling back to Classic"
                    );
                    "index.html"
                }
            };
            let entry = match ui_variant.as_str() {
                "new" => guarded("newui.html"),
                "make" => guarded("makeui.html"),
                // Catch-all: "classic", empty/absent (already snapped by Config::load), or any
                // value that slipped past validation resolves to Classic - never a dead window.
                _ => "index.html",
            };
            // Wave 0 (2026-09-29): WebView2's GPU/ANGLE compositor occasionally never paints at
            // all (confirmed live: solid black content area, correct title/chrome, process
            // responding, no panic - stuck through resize/maximize/minimize-restore, reproduced
            // with exactly one process running so this is distinct from the single-instance gap
            // above). Must be set before WebView2 initializes. Acceptable tradeoff here - this is
            // a CRUD UI with no heavy rendering, not a case that needs GPU acceleration.
            #[cfg(windows)]
            std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", "--disable-gpu");

            // Window label MUST stay "main" - src-tauri/capabilities/default.json scopes the
            // clipboard-manager and dialog permissions to that label (D-04/T-08-02).
            let window = tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App(entry.into()))
                // N17/D3: was the literal generic string "tauri-app" (Tauri's own scaffold
                // default), not a branded name - cosmetic but confusing in the taskbar/title bar.
                .title("Nestkey Password Manager")
                .inner_size(800.0, 600.0)
                .build()?;

            // Best-effort repaint nudge for any stuck-paint case --disable-gpu above doesn't
            // catch: forces WebView2's compositor to redraw.
            let size = window.inner_size()?;
            window.set_size(tauri::Size::Physical(tauri::PhysicalSize::new(size.width + 1, size.height)))?;
            window.set_size(tauri::Size::Physical(size))?;

            eprintln!("info: main window created and shown (ui_variant={ui_variant})");

            if ssh_agent_enabled_at_startup {
                // B4/D2: a failed start must not take the boot sequence down with it, and must not
                // silently keep re-attempting on every future launch - revert the persisted flag so
                // a bad state self-heals to "off" after one failed boot instead of crash-looping.
                if let Err(msg) = ssh_agent_service::start(&app.handle().clone()) {
                    eprintln!("warning: ssh-agent did not start at boot, disabling it: {msg}");
                    let state = app.state::<AppState>();
                    let mut config = state.config.lock().unwrap();
                    let mut updated = config.clone();
                    updated.ssh_agent_enabled = false;
                    if updated.save(&app.state::<AppPaths>().config_path()).is_ok() {
                        *config = updated;
                    }
                }
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
                        && nestkey_core::autolock::suspend_gap_exceeds(elapsed, tick_interval);
                    if suspended {
                        nestkey_core::commands::do_lock(&state);
                    }
                    if suspended || nestkey_core::autolock::tick(&state, &nestkey_core::session::SystemClock) {
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
            commands::vault_exists,
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
            commands::set_site_favorite,
            commands::set_site_item_type,
            commands::set_site_card_hint,
            commands::set_site_folder,
            commands::list_folders,
            commands::add_folder,
            commands::rename_folder,
            commands::delete_folder,
            commands::add_credential,
            commands::update_credential,
            commands::list_credentials_for_site,
            commands::search,
            commands::soft_delete_credential,
            commands::set_totp_secret,
            commands::clear_totp_secret,
            commands::totp_code,
            commands::list_credential_history,
            commands::list_credential_change_log,
            commands::change_master_password,
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
            commands::update_reprompt_mode,
            commands::update_reprompt_grace_seconds,
            commands::update_delete_reprompt,
            commands::generate_password,
            commands::analyze_strength,
            commands::copy_to_clipboard,
            commands::create_backup,
            commands::restore_backup,
            commands::export_csv,
            commands::export_json,
            commands::import_json_preview,
            commands::import_json_commit,
            commands::read_csv_headers,
            commands::import_csv_preview,
            commands::import_csv_commit,
            commands::health_report,
            commands::check_breach,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
