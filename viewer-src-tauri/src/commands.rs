use pwmgr_core::commands::{do_get_config, do_lock, do_list_credentials_for_site, do_list_sites, do_login, do_search, do_touch, do_unlock_status};
use pwmgr_core::dto::{AuthSuccess, ConfigDto, CredentialDto, SearchResultDto, SiteDto};
use pwmgr_core::error::CommandError;
use pwmgr_core::paths::AppPaths;
use pwmgr_core::state::AppState;
use std::time::Duration;
use tauri::{Emitter, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use zeroize::Zeroizing;

#[tauri::command]
pub fn unlock_status(state: State<AppState>) -> bool {
    do_unlock_status(&state)
}

#[tauri::command]
pub fn login(
    state: State<AppState>,
    paths: State<AppPaths>,
    username: String,
    master_password: String,
) -> Result<AuthSuccess, CommandError> {
    do_login(&state, &paths, &username, &master_password)
}

#[tauri::command]
pub fn lock(state: State<AppState>, app: tauri::AppHandle) {
    do_lock(&state);
    if let Err(e) = app.emit("vault-locked", ()) {
        eprintln!("warning: failed to emit vault-locked event ({e})");
    }
}

#[tauri::command]
pub fn touch(state: State<AppState>) {
    do_touch(&state);
}

#[tauri::command]
pub fn list_sites(state: State<AppState>) -> Result<Vec<SiteDto>, CommandError> {
    do_list_sites(&state)
}

#[tauri::command]
pub fn list_credentials_for_site(
    state: State<AppState>,
    site_id: i64,
) -> Result<Vec<CredentialDto>, CommandError> {
    do_list_credentials_for_site(&state, site_id)
}

#[tauri::command]
pub fn search(state: State<AppState>, query: String) -> Result<Vec<SearchResultDto>, CommandError> {
    do_search(&state, &query)
}

#[tauri::command]
pub fn get_config(state: State<AppState>) -> ConfigDto {
    do_get_config(&state)
}

const CLIPBOARD_CLEAR_SECS: u64 = 30; // hardcoded constant, NOT a user setting (ponytail)

// Thin wrapper only - not split into a `do_*` + wrapper pair. The effect is an OS clipboard side
// effect requiring a real `AppHandle`, so it is not unit-testable in the headless harness. Same
// intentional, justified deviation as src-tauri/src/commands.rs's copy_to_clipboard - duplicated
// here (not shared via core) because this whole function is Tauri-AppHandle/clipboard-plugin-
// specific.
#[tauri::command]
pub fn copy_to_clipboard(app: tauri::AppHandle, text: String) -> Result<(), CommandError> {
    let secret = Zeroizing::new(text);
    app.clipboard().write_text(secret.as_str()).map_err(|_| pwmgr_core::error::internal_error())?;
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(CLIPBOARD_CLEAR_SECS)).await;
        // check-before-clear: only wipe if the clipboard still holds exactly what we wrote.
        if let Ok(current) = handle.clipboard().read_text() {
            if current == *secret {
                let _ = handle.clipboard().clear();
            }
        }
    });
    Ok(())
}
