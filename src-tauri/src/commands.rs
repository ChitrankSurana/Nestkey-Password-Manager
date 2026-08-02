use pwmgr_core::commands::{
    do_add_credential, do_add_site, do_analyze_strength, do_check_breach, do_create_backup,
    do_export_csv, do_generate_password, do_get_config, do_health_report, do_import_csv_commit,
    do_import_csv_preview, do_list_credentials_for_site, do_list_sites, do_list_trash, do_lock,
    do_login, do_purge_trash, do_read_csv_headers, do_register, do_restore_backup,
    do_restore_credential, do_search, do_soft_delete_credential, do_soft_delete_site, do_touch,
    do_unlock_status, do_update_accent_color, do_update_auto_lock_minutes, do_update_credential,
    do_update_hibp_opt_in, do_update_mobile_nav_style, do_update_nav_collapsed,
    do_update_old_password_days, do_update_site, do_update_strength_algorithm, do_update_theme,
};
use pwmgr_core::dto::{
    AuthSuccess, BreachReportDto, ConfigDto, CredentialDto, ExportMode, GenerateRequest,
    HealthReportDto, ImportFormat, ImportRecordDto, ImportSummaryDto, SearchResultDto, SiteDto,
    StrengthDto, TrashedCredentialDto,
};
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
pub fn register_user(
    state: State<AppState>,
    paths: State<AppPaths>,
    username: String,
    master_password: String,
) -> Result<AuthSuccess, CommandError> {
    do_register(&state, &paths, &username, &master_password)
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
pub fn add_site(
    state: State<AppState>,
    name: String,
    url: Option<String>,
    notes: Option<String>,
) -> Result<i64, CommandError> {
    do_add_site(&state, &name, url.as_deref(), notes.as_deref())
}

#[tauri::command]
pub fn update_site(
    state: State<AppState>,
    site_id: i64,
    name: String,
    url: Option<String>,
    notes: Option<String>,
) -> Result<(), CommandError> {
    do_update_site(&state, site_id, &name, url.as_deref(), notes.as_deref())
}

#[tauri::command]
pub fn soft_delete_site(state: State<AppState>, site_id: i64) -> Result<(), CommandError> {
    do_soft_delete_site(&state, site_id)
}

#[tauri::command]
pub fn add_credential(
    state: State<AppState>,
    site_id: i64,
    username: Option<String>,
    secret: String,
    remarks: Option<String>,
) -> Result<i64, CommandError> {
    do_add_credential(&state, site_id, username.as_deref(), &secret, remarks.as_deref())
}

#[tauri::command]
pub fn update_credential(
    state: State<AppState>,
    credential_id: i64,
    username: Option<String>,
    secret: String,
    remarks: Option<String>,
) -> Result<(), CommandError> {
    do_update_credential(&state, credential_id, username.as_deref(), &secret, remarks.as_deref())
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
pub fn soft_delete_credential(state: State<AppState>, credential_id: i64) -> Result<(), CommandError> {
    do_soft_delete_credential(&state, credential_id)
}

#[tauri::command]
pub fn list_trash(state: State<AppState>) -> Result<Vec<TrashedCredentialDto>, CommandError> {
    do_list_trash(&state)
}

#[tauri::command]
pub fn restore_credential(state: State<AppState>, credential_id: i64) -> Result<(), CommandError> {
    do_restore_credential(&state, credential_id)
}

#[tauri::command]
pub fn purge_trash(state: State<AppState>) -> Result<usize, CommandError> {
    do_purge_trash(&state)
}

#[tauri::command]
pub fn get_config(state: State<AppState>) -> ConfigDto {
    do_get_config(&state)
}

#[tauri::command]
pub fn update_auto_lock_minutes(
    state: State<AppState>,
    paths: State<AppPaths>,
    minutes: u32,
) -> Result<(), CommandError> {
    do_update_auto_lock_minutes(&state, &paths, minutes)
}

#[tauri::command]
pub fn update_strength_algorithm(
    state: State<AppState>,
    paths: State<AppPaths>,
    algorithm: String,
) -> Result<(), CommandError> {
    do_update_strength_algorithm(&state, &paths, &algorithm)
}

#[tauri::command]
pub fn update_theme(state: State<AppState>, paths: State<AppPaths>, theme: String) -> Result<(), CommandError> {
    do_update_theme(&state, &paths, &theme)
}

#[tauri::command]
pub fn update_accent_color(state: State<AppState>, paths: State<AppPaths>, color: String) -> Result<(), CommandError> {
    do_update_accent_color(&state, &paths, &color)
}

#[tauri::command]
pub fn update_old_password_days(state: State<AppState>, paths: State<AppPaths>, days: u32) -> Result<(), CommandError> {
    do_update_old_password_days(&state, &paths, days)
}

#[tauri::command]
pub fn update_hibp_opt_in(state: State<AppState>, paths: State<AppPaths>, enabled: bool) -> Result<(), CommandError> {
    do_update_hibp_opt_in(&state, &paths, enabled)
}

#[tauri::command]
pub fn update_nav_collapsed(state: State<AppState>, paths: State<AppPaths>, collapsed: bool) -> Result<(), CommandError> {
    do_update_nav_collapsed(&state, &paths, collapsed)
}

#[tauri::command]
pub fn update_mobile_nav_style(state: State<AppState>, paths: State<AppPaths>, style: String) -> Result<(), CommandError> {
    do_update_mobile_nav_style(&state, &paths, &style)
}

#[tauri::command]
pub fn generate_password(request: GenerateRequest) -> Result<Zeroizing<String>, CommandError> {
    do_generate_password(&request)
}

#[tauri::command]
pub fn analyze_strength(password: String) -> StrengthDto {
    do_analyze_strength(&password)
}

const CLIPBOARD_CLEAR_SECS: u64 = 30; // hardcoded constant, NOT a user setting

// Thin wrapper only - the effect is an OS clipboard side effect requiring a real `AppHandle`, so it
// stays a per-Tauri-binary command (promoting it would force pwmgr-core to depend on tauri).
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

#[tauri::command]
pub fn create_backup(state: State<AppState>, paths: State<AppPaths>) -> Result<String, CommandError> {
    do_create_backup(&state, &paths)
}

#[tauri::command]
pub fn restore_backup(
    state: State<AppState>,
    paths: State<AppPaths>,
    app: tauri::AppHandle,
    path: String,
    confirmed: bool,
) -> Result<(), CommandError> {
    do_restore_backup(&state, &paths, &path, confirmed)?;
    // do_restore_backup locks the session when the restored backup overwrites the open vault; the
    // frontend's only route back to login is the "vault-locked" event - same pattern as `lock`.
    if state.session.lock().unwrap().is_locked() {
        if let Err(e) = app.emit("vault-locked", ()) {
            eprintln!("warning: failed to emit vault-locked event ({e})");
        }
    }
    Ok(())
}

#[tauri::command]
pub fn export_csv(
    state: State<AppState>,
    paths: State<AppPaths>,
    path: String,
    mode: ExportMode,
    confirmed: bool,
) -> Result<(), CommandError> {
    do_export_csv(&state, &paths, &path, &mode, confirmed)
}

#[tauri::command]
pub fn read_csv_headers(path: String) -> Result<Vec<String>, CommandError> {
    do_read_csv_headers(&path)
}

#[tauri::command]
pub fn import_csv_preview(
    path: String,
    format: ImportFormat,
) -> Result<Vec<ImportRecordDto>, CommandError> {
    do_import_csv_preview(&path, &format)
}

#[tauri::command]
pub fn import_csv_commit(
    state: State<AppState>,
    records: Vec<ImportRecordDto>,
) -> Result<ImportSummaryDto, CommandError> {
    do_import_csv_commit(&state, records)
}

#[tauri::command]
pub fn health_report(state: State<AppState>) -> Result<HealthReportDto, CommandError> {
    do_health_report(&state)
}

#[tauri::command]
pub fn check_breach(state: State<AppState>) -> Result<BreachReportDto, CommandError> {
    do_check_breach(&state, pwmgr_core::hibp::HIBP_BASE_URL)
}
