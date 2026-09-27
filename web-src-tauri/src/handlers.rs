use crate::error_map::ApiError;
use crate::session_store::{SessionStore, UploadCache, UserSession};
use axum::body::Bytes;
use axum::extract::{ConnectInfo, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Extension, Json, Router};
use pwmgr_core::commands::*;
use pwmgr_core::dto::{
    AuthSuccess, BreachReportDto, ConfigDto, CredentialDto, ExportMode, GenerateRequest,
    FolderDto, HealthReportDto, HistoryEntryDto, ImportFormat, ImportRecordDto, ImportSummaryDto, SearchResultDto, SiteDto,
    StrengthDto, TotpCodeDto, TrashedCredentialDto,
};
use crate::kdf_guard::KdfGuard;
use crate::throttle_store::ThrottleStore;
use pwmgr_core::paths::AppPaths;
use pwmgr_core::session::SystemClock;
use pwmgr_core::state::AppState;
use pwmgr_core::vault::CustomField;
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tower_http::services::{ServeDir, ServeFile};
use zeroize::Zeroizing;

/// Shared axum state: the session store + the fixed app-data paths.
#[derive(Clone)]
pub struct AppContext {
    pub store: SessionStore,
    pub paths: Arc<AppPaths>,
    /// In-process failed-attempt throttle for the credential routes (SEC-02).
    pub throttle: ThrottleStore,
    /// Bounds concurrent Argon2 derivations on login/register/password-mode export (SEC-02).
    pub kdf: KdfGuard,
}

/// Per-route body-size cap for the raw-bytes upload endpoints only (SEC-01/D-01, D-03). Applied as
/// a `DefaultBodyLimit` layer on `/api/upload_csv` (and, in plan 09-03, `/api/upload_backup`) —
/// never globally, so axum's 2 MiB default (RUSTSEC-2022-0055) still guards every JSON command
/// route without silently rejecting a real vault-sized CSV/backup upload.
pub const UPLOAD_BODY_LIMIT: usize = 25 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthRequest {
    username: String,
    master_password: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HasPinRequest {
    username: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UnlockWithPinRequest {
    username: String,
    pin: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetPinRequest {
    pin: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddSiteRequest {
    name: String,
    url: Option<String>,
    notes: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateSiteRequest {
    site_id: i64,
    name: String,
    url: Option<String>,
    notes: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddCredentialRequest {
    site_id: i64,
    username: Option<String>,
    secret: String,
    remarks: Option<String>,
    #[serde(default)]
    custom_fields: Vec<CustomField>,
    #[serde(default)]
    reprompt_exempt: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateCredentialRequest {
    credential_id: i64,
    username: Option<String>,
    secret: String,
    remarks: Option<String>,
    #[serde(default)]
    custom_fields: Vec<CustomField>,
    #[serde(default)]
    reprompt_exempt: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SiteIdRequest {
    site_id: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetFavoriteRequest {
    site_id: i64,
    favorite: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetItemTypeRequest {
    site_id: i64,
    item_type: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetCardHintRequest {
    site_id: i64,
    brand: Option<String>,
    last4: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetSiteFolderRequest {
    site_id: i64,
    folder_id: Option<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FolderNameRequest {
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenameFolderRequest {
    folder_id: i64,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FolderIdRequest {
    folder_id: i64,
}

#[derive(Deserialize)]
struct SearchRequest {
    query: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChangeMasterPasswordRequest {
    current_password: String,
    new_password: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VerifyMasterPasswordRequest {
    master_password: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CredentialIdRequest {
    credential_id: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetTotpSecretRequest {
    credential_id: i64,
    secret: String,
}

#[derive(Deserialize)]
struct MinutesRequest {
    minutes: u32,
}

#[derive(Deserialize)]
struct AlgorithmRequest {
    algorithm: String,
}

#[derive(Deserialize)]
struct ThemeRequest {
    theme: String,
}

#[derive(Deserialize)]
struct ColorRequest {
    color: String,
}

#[derive(Deserialize)]
struct DaysRequest {
    days: u32,
}

#[derive(Deserialize)]
struct EnabledRequest {
    enabled: bool,
}

#[derive(Deserialize)]
struct CollapsedRequest {
    collapsed: bool,
}

#[derive(Deserialize)]
struct MobileNavStyleRequest {
    style: String,
}

#[derive(Deserialize)]
struct UiVariantRequest {
    variant: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RepromptModeRequest {
    mode: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RepromptGraceSecondsRequest {
    seconds: u32,
}

#[derive(Deserialize)]
struct GeneratePasswordRequest {
    request: GenerateRequest,
}

#[derive(Deserialize)]
struct AnalyzeStrengthRequest {
    password: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestoreBackupRequest {
    confirmed: bool,
}

// No `path` field (SEC-01/D-01): the web transport never accepts a client-supplied export
// destination. `export_csv` returns the bytes in the HTTP response body instead.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportCsvRequest {
    mode: ExportMode,
    confirmed: bool,
}

// Same shape as ExportCsvRequest - the JSON vault export reuses the identical three-mode contract.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportJsonRequest {
    mode: ExportMode,
    confirmed: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportJsonRequest {
    password: Option<String>,
}

// No `path` field (SEC-01/D-01/D-03): reads the CSV bytes this session already uploaded via
// `/api/upload_csv` instead of a client-supplied filesystem path.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportCsvPreviewRequest {
    format: ImportFormat,
}

#[derive(Deserialize)]
struct ImportCsvCommitRequest {
    records: Vec<ImportRecordDto>,
}

/// Builds a fresh, locked `AppState` (own registry connection to the shared registry.db, config
/// loaded from disk) — the per-session state each login/register unlocks. Thin wrapper over the
/// shared `AppState::open` constructor, just mapping its error onto this crate's `ApiError`.
fn build_locked_state(paths: &AppPaths) -> Result<AppState, ApiError> {
    AppState::open(paths).map_err(|_| pwmgr_core::error::internal_error().into())
}

fn issue_session(ctx: &AppContext, body: &AuthRequest, register: bool) -> Result<Response, ApiError> {
    let app_state = build_locked_state(&ctx.paths)?;
    if register {
        do_register(&app_state, &ctx.paths, &body.username, &body.master_password)?;
    } else {
        do_login(&app_state, &ctx.paths, &body.username, &body.master_password)?;
    }
    let session = UserSession {
        username: body.username.clone(),
        app_state,
        last_seen: Mutex::new(Instant::now()),
        upload_cache: Mutex::new(None),
    };
    let token = ctx.store.insert(session);
    let cookie = format!("session={token}; HttpOnly; SameSite=Strict; Path=/");
    let mut res = Json(AuthSuccess { username: body.username.clone() }).into_response();
    res.headers_mut().insert(header::SET_COOKIE, cookie.parse().unwrap());
    Ok(res)
}

/// Throttle keys for one credential attempt (D-09): one per username, one per peer IP. The username
/// key is lowercased and built the same way whether or not the account exists, so a lockout never
/// reveals account existence.
fn throttle_keys(username: &str, peer: SocketAddr) -> (String, String) {
    (format!("user:{}", username.to_lowercase()), format!("ip:{}", peer.ip()))
}

/// Counts an attempt against both keys (both counters always advance) and refuses with a
/// `throttled` error if either is locked out. Runs before any credential work or KDF permit.
fn begin_credential_attempt(ctx: &AppContext, username: &str, peer: SocketAddr) -> Result<(), ApiError> {
    let (user_key, ip_key) = throttle_keys(username, peer);
    let clock = SystemClock;
    let user = ctx.throttle.begin_attempt(&user_key, &clock);
    let ip = ctx.throttle.begin_attempt(&ip_key, &clock);
    if let Err(remaining) = user {
        let secs = remaining.as_secs().max(1);
        return Err(pwmgr_core::error::throttled_error(&format!(
            "Too many failed attempts. Try again in {secs} seconds."
        ))
        .into());
    }
    if let Err(remaining) = ip {
        let secs = remaining.as_secs().max(1);
        return Err(pwmgr_core::error::throttled_error(&format!(
            "Too many failed attempts from this machine. Try again in {secs} seconds."
        ))
        .into());
    }
    Ok(())
}

/// Successful authentication resets both keys (D-10).
fn reset_credential_attempts(ctx: &AppContext, username: &str, peer: SocketAddr) {
    let (user_key, ip_key) = throttle_keys(username, peer);
    ctx.throttle.record_success(&user_key);
    ctx.throttle.record_success(&ip_key);
}

async fn login(
    State(ctx): State<AppContext>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(body): Json<AuthRequest>,
) -> Result<Response, ApiError> {
    begin_credential_attempt(&ctx, &body.username, peer)?;
    let _kdf_permit = ctx.kdf.try_acquire()?;
    let res = issue_session(&ctx, &body, false)?;
    reset_credential_attempts(&ctx, &body.username, peer);
    Ok(res)
}

async fn register(
    State(ctx): State<AppContext>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(body): Json<AuthRequest>,
) -> Result<Response, ApiError> {
    begin_credential_attempt(&ctx, &body.username, peer)?;
    let _kdf_permit = ctx.kdf.try_acquire()?;
    let res = issue_session(&ctx, &body, true)?;
    reset_credential_attempts(&ctx, &body.username, peer);
    Ok(res)
}

// Public (unauthenticated), like login/register - the login screen needs this before a session
// exists, to decide whether to offer a "Unlock with PIN" option at all.
async fn has_pin(State(ctx): State<AppContext>, Json(body): Json<HasPinRequest>) -> Result<Json<bool>, ApiError> {
    let app_state = build_locked_state(&ctx.paths)?;
    Ok(Json(do_has_pin(&app_state, &body.username)?))
}

// N1: same "public, pre-session" rationale as has_pin above - the Login/Register choice happens
// before any session exists.
async fn vault_exists(State(ctx): State<AppContext>) -> Result<Json<bool>, ApiError> {
    let app_state = build_locked_state(&ctx.paths)?;
    Ok(Json(do_vault_exists(&app_state)?))
}

async fn unlock_with_pin(
    State(ctx): State<AppContext>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(body): Json<UnlockWithPinRequest>,
) -> Result<Response, ApiError> {
    // Same throttle keys and KDF cap as login: a PIN is a much shorter secret than a master
    // password, so leaving this route open would make it the cheapest way to brute-force a vault.
    begin_credential_attempt(&ctx, &body.username, peer)?;
    let _kdf_permit = ctx.kdf.try_acquire()?;
    let app_state = build_locked_state(&ctx.paths)?;
    do_unlock_with_pin(&app_state, &ctx.paths, &body.username, &body.pin)?;
    reset_credential_attempts(&ctx, &body.username, peer);
    let session = UserSession {
        username: body.username.clone(),
        app_state,
        last_seen: Mutex::new(Instant::now()),
        upload_cache: Mutex::new(None),
    };
    let token = ctx.store.insert(session);
    let cookie = format!("session={token}; HttpOnly; SameSite=Strict; Path=/");
    let mut res = Json(AuthSuccess { username: body.username.clone() }).into_response();
    res.headers_mut().insert(header::SET_COOKIE, cookie.parse().unwrap());
    Ok(res)
}

/// Fully terminates this browser's session (unlike `lock`, which just re-locks the vault and keeps
/// the entry in the store): removes it from `SessionStore` outright, freeing its vault/registry
/// connections, and clears the cookie client-side. Deliberately NOT behind `auth_middleware` — that
/// gate 401s an already-locked session, but logging out must work from any state (locked, stale, or
/// even an unrecognized/expired cookie), so the token is resolved independently here, same as
/// `login`/`register`.
async fn logout(State(ctx): State<AppContext>, req: axum::http::Request<axum::body::Body>) -> Response {
    let token = req
        .headers()
        .get(header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .and_then(crate::middleware::parse_session_cookie);
    if let Some(token) = token {
        // Wipe any cached upload promptly (SEC-01/D-04) rather than waiting for the last `Arc`
        // clone of this session to drop.
        if let Some(session) = ctx.store.get(&token) {
            session.clear_upload();
        }
        ctx.store.remove(&token);
    }
    let mut res = StatusCode::OK.into_response();
    res.headers_mut().insert(
        header::SET_COOKIE,
        "session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0".parse().unwrap(),
    );
    res
}

async fn unlock_status(Extension(s): Extension<Arc<UserSession>>) -> Json<bool> {
    Json(do_unlock_status(&s.app_state))
}

async fn lock(Extension(s): Extension<Arc<UserSession>>) -> StatusCode {
    do_lock(&s.app_state);
    StatusCode::OK
}

async fn touch(Extension(s): Extension<Arc<UserSession>>) -> StatusCode {
    do_touch(&s.app_state);
    StatusCode::OK
}

async fn verify_master_password(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<VerifyMasterPasswordRequest>,
) -> Result<Json<bool>, ApiError> {
    Ok(Json(do_verify_master_password(&s.app_state, &b.master_password)?))
}

async fn set_pin(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<SetPinRequest>,
) -> Result<StatusCode, ApiError> {
    do_set_pin(&s.app_state, &b.pin)?;
    Ok(StatusCode::OK)
}

async fn clear_pin(Extension(s): Extension<Arc<UserSession>>) -> Result<StatusCode, ApiError> {
    do_clear_pin(&s.app_state)?;
    Ok(StatusCode::OK)
}

async fn list_sites(Extension(s): Extension<Arc<UserSession>>) -> Result<Json<Vec<SiteDto>>, ApiError> {
    Ok(Json(do_list_sites(&s.app_state)?))
}

async fn add_site(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<AddSiteRequest>,
) -> Result<Json<i64>, ApiError> {
    Ok(Json(do_add_site(&s.app_state, &b.name, b.url.as_deref(), b.notes.as_deref())?))
}

async fn update_site(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<UpdateSiteRequest>,
) -> Result<StatusCode, ApiError> {
    do_update_site(&s.app_state, b.site_id, &b.name, b.url.as_deref(), b.notes.as_deref())?;
    Ok(StatusCode::OK)
}

async fn soft_delete_site(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<SiteIdRequest>,
) -> Result<StatusCode, ApiError> {
    do_soft_delete_site(&s.app_state, b.site_id)?;
    Ok(StatusCode::OK)
}

async fn set_site_favorite(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<SetFavoriteRequest>,
) -> Result<StatusCode, ApiError> {
    do_set_site_favorite(&s.app_state, b.site_id, b.favorite)?;
    Ok(StatusCode::OK)
}

async fn set_site_item_type(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<SetItemTypeRequest>,
) -> Result<StatusCode, ApiError> {
    do_set_site_item_type(&s.app_state, b.site_id, &b.item_type)?;
    Ok(StatusCode::OK)
}

async fn set_site_card_hint(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<SetCardHintRequest>,
) -> Result<StatusCode, ApiError> {
    do_set_site_card_hint(&s.app_state, b.site_id, b.brand.as_deref(), b.last4.as_deref())?;
    Ok(StatusCode::OK)
}

async fn set_site_folder(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<SetSiteFolderRequest>,
) -> Result<StatusCode, ApiError> {
    do_set_site_folder(&s.app_state, b.site_id, b.folder_id)?;
    Ok(StatusCode::OK)
}

async fn list_folders(Extension(s): Extension<Arc<UserSession>>) -> Result<Json<Vec<FolderDto>>, ApiError> {
    Ok(Json(do_list_folders(&s.app_state)?))
}

async fn add_folder(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<FolderNameRequest>,
) -> Result<Json<i64>, ApiError> {
    Ok(Json(do_add_folder(&s.app_state, &b.name)?))
}

async fn rename_folder(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<RenameFolderRequest>,
) -> Result<StatusCode, ApiError> {
    do_rename_folder(&s.app_state, b.folder_id, &b.name)?;
    Ok(StatusCode::OK)
}

async fn delete_folder(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<FolderIdRequest>,
) -> Result<StatusCode, ApiError> {
    do_delete_folder(&s.app_state, b.folder_id)?;
    Ok(StatusCode::OK)
}

async fn add_credential(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<AddCredentialRequest>,
) -> Result<Json<i64>, ApiError> {
    Ok(Json(do_add_credential(
        &s.app_state, b.site_id, b.username.as_deref(), &b.secret, b.remarks.as_deref(),
        &b.custom_fields, b.reprompt_exempt,
    )?))
}

async fn update_credential(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<UpdateCredentialRequest>,
) -> Result<StatusCode, ApiError> {
    do_update_credential(
        &s.app_state, b.credential_id, b.username.as_deref(), &b.secret, b.remarks.as_deref(),
        &b.custom_fields, b.reprompt_exempt,
    )?;
    Ok(StatusCode::OK)
}

async fn list_credentials_for_site(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<SiteIdRequest>,
) -> Result<Json<Vec<CredentialDto>>, ApiError> {
    Ok(Json(do_list_credentials_for_site(&s.app_state, b.site_id)?))
}

async fn search(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<SearchRequest>,
) -> Result<Json<Vec<SearchResultDto>>, ApiError> {
    Ok(Json(do_search(&s.app_state, &b.query)?))
}

async fn soft_delete_credential(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<CredentialIdRequest>,
) -> Result<StatusCode, ApiError> {
    do_soft_delete_credential(&s.app_state, b.credential_id)?;
    Ok(StatusCode::OK)
}

async fn set_totp_secret(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<SetTotpSecretRequest>,
) -> Result<StatusCode, ApiError> {
    do_set_totp_secret(&s.app_state, b.credential_id, &b.secret)?;
    Ok(StatusCode::OK)
}

async fn clear_totp_secret(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<CredentialIdRequest>,
) -> Result<StatusCode, ApiError> {
    do_clear_totp_secret(&s.app_state, b.credential_id)?;
    Ok(StatusCode::OK)
}

async fn totp_code(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<CredentialIdRequest>,
) -> Result<Json<Option<TotpCodeDto>>, ApiError> {
    Ok(Json(do_totp_code(&s.app_state, b.credential_id)?))
}

async fn list_credential_history(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<CredentialIdRequest>,
) -> Result<Json<Vec<HistoryEntryDto>>, ApiError> {
    Ok(Json(do_list_credential_history(&s.app_state, b.credential_id)?))
}

async fn list_trash(
    Extension(s): Extension<Arc<UserSession>>,
) -> Result<Json<Vec<TrashedCredentialDto>>, ApiError> {
    Ok(Json(do_list_trash(&s.app_state)?))
}

async fn restore_credential(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<CredentialIdRequest>,
) -> Result<StatusCode, ApiError> {
    do_restore_credential(&s.app_state, b.credential_id)?;
    Ok(StatusCode::OK)
}

async fn purge_trash(Extension(s): Extension<Arc<UserSession>>) -> Result<Json<usize>, ApiError> {
    Ok(Json(do_purge_trash(&s.app_state)?))
}

async fn get_config(Extension(s): Extension<Arc<UserSession>>) -> Json<ConfigDto> {
    Json(do_get_config(&s.app_state))
}

async fn update_auto_lock_minutes(
    Extension(s): Extension<Arc<UserSession>>,
    State(ctx): State<AppContext>,
    Json(b): Json<MinutesRequest>,
) -> Result<StatusCode, ApiError> {
    do_update_auto_lock_minutes(&s.app_state, &ctx.paths, b.minutes)?;
    Ok(StatusCode::OK)
}

async fn update_strength_algorithm(
    Extension(s): Extension<Arc<UserSession>>,
    State(ctx): State<AppContext>,
    Json(b): Json<AlgorithmRequest>,
) -> Result<StatusCode, ApiError> {
    do_update_strength_algorithm(&s.app_state, &ctx.paths, &b.algorithm)?;
    Ok(StatusCode::OK)
}

async fn update_theme(
    Extension(s): Extension<Arc<UserSession>>,
    State(ctx): State<AppContext>,
    Json(b): Json<ThemeRequest>,
) -> Result<StatusCode, ApiError> {
    do_update_theme(&s.app_state, &ctx.paths, &b.theme)?;
    Ok(StatusCode::OK)
}

async fn update_accent_color(
    Extension(s): Extension<Arc<UserSession>>,
    State(ctx): State<AppContext>,
    Json(b): Json<ColorRequest>,
) -> Result<StatusCode, ApiError> {
    do_update_accent_color(&s.app_state, &ctx.paths, &b.color)?;
    Ok(StatusCode::OK)
}

async fn update_old_password_days(
    Extension(s): Extension<Arc<UserSession>>,
    State(ctx): State<AppContext>,
    Json(b): Json<DaysRequest>,
) -> Result<StatusCode, ApiError> {
    do_update_old_password_days(&s.app_state, &ctx.paths, b.days)?;
    Ok(StatusCode::OK)
}

async fn update_hibp_opt_in(
    Extension(s): Extension<Arc<UserSession>>,
    State(ctx): State<AppContext>,
    Json(b): Json<EnabledRequest>,
) -> Result<StatusCode, ApiError> {
    do_update_hibp_opt_in(&s.app_state, &ctx.paths, b.enabled)?;
    Ok(StatusCode::OK)
}

async fn update_nav_collapsed(
    Extension(s): Extension<Arc<UserSession>>,
    State(ctx): State<AppContext>,
    Json(b): Json<CollapsedRequest>,
) -> Result<StatusCode, ApiError> {
    do_update_nav_collapsed(&s.app_state, &ctx.paths, b.collapsed)?;
    Ok(StatusCode::OK)
}

async fn update_mobile_nav_style(
    Extension(s): Extension<Arc<UserSession>>,
    State(ctx): State<AppContext>,
    Json(b): Json<MobileNavStyleRequest>,
) -> Result<StatusCode, ApiError> {
    do_update_mobile_nav_style(&s.app_state, &ctx.paths, &b.style)?;
    Ok(StatusCode::OK)
}

async fn update_ui_variant(
    Extension(s): Extension<Arc<UserSession>>,
    State(ctx): State<AppContext>,
    Json(b): Json<UiVariantRequest>,
) -> Result<StatusCode, ApiError> {
    do_update_ui_variant(&s.app_state, &ctx.paths, &b.variant)?;
    Ok(StatusCode::OK)
}

async fn update_reprompt_mode(
    Extension(s): Extension<Arc<UserSession>>,
    State(ctx): State<AppContext>,
    Json(b): Json<RepromptModeRequest>,
) -> Result<StatusCode, ApiError> {
    do_update_reprompt_mode(&s.app_state, &ctx.paths, &b.mode)?;
    Ok(StatusCode::OK)
}

async fn update_reprompt_grace_seconds(
    Extension(s): Extension<Arc<UserSession>>,
    State(ctx): State<AppContext>,
    Json(b): Json<RepromptGraceSecondsRequest>,
) -> Result<StatusCode, ApiError> {
    do_update_reprompt_grace_seconds(&s.app_state, &ctx.paths, b.seconds)?;
    Ok(StatusCode::OK)
}

async fn generate_password(
    Extension(_s): Extension<Arc<UserSession>>,
    Json(b): Json<GeneratePasswordRequest>,
) -> Result<Json<Zeroizing<String>>, ApiError> {
    Ok(Json(do_generate_password(&b.request)?))
}

async fn analyze_strength(
    Extension(_s): Extension<Arc<UserSession>>,
    Json(b): Json<AnalyzeStrengthRequest>,
) -> Json<StrengthDto> {
    Json(do_analyze_strength(&b.password))
}

async fn change_master_password(
    Extension(s): Extension<Arc<UserSession>>,
    State(ctx): State<AppContext>,
    Json(b): Json<ChangeMasterPasswordRequest>,
) -> Result<StatusCode, ApiError> {
    do_change_master_password(&s.app_state, &ctx.paths, &b.current_password, &b.new_password)?;
    Ok(StatusCode::OK)
}

// Deliberately does not accept a client-supplied destination, unlike the desktop `create_backup`
// Tauri command. On desktop the path comes from a native save dialog the user personally drove;
// in web mode it would be an arbitrary attacker-controlled string. SECURITY_CONCERNS.md H1 is the
// still-open finding about exactly that class of parameter on four other handlers — this route
// must not become a fifth. Phase 9 SEC-01 is chartered to confine the existing four.
async fn create_backup(
    Extension(s): Extension<Arc<UserSession>>,
    State(ctx): State<AppContext>,
) -> Result<Json<String>, ApiError> {
    Ok(Json(do_create_backup(&s.app_state, &ctx.paths, None)?))
}

// Path-free counterpart of `create_backup` (same reasoning): returns the backup bytes in the response
// body so the browser can offer a save-location picker. Nothing is written server-side.
async fn create_backup_download(
    Extension(s): Extension<Arc<UserSession>>,
) -> Result<Response, ApiError> {
    let (filename, bytes) = do_create_backup_download(&s.app_state)?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{filename}\"")),
        ],
        bytes,
    )
        .into_response())
}

async fn restore_backup(
    Extension(s): Extension<Arc<UserSession>>,
    State(ctx): State<AppContext>,
    Json(b): Json<RestoreBackupRequest>,
) -> Result<StatusCode, ApiError> {
    // SEC-01/D-05: bytes come from the session's upload cache, never a client-named path. The cache
    // survives `confirm_required` so the confirmed retry needs no re-upload.
    let bytes = s
        .backup_upload()
        .ok_or_else(|| pwmgr_core::error::invalid_input_error("No backup file uploaded yet."))?;
    do_restore_backup_from_bytes(&s.app_state, &ctx.paths, &bytes, b.confirmed)?;
    s.clear_upload();
    Ok(StatusCode::OK)
}

async fn upload_backup(Extension(s): Extension<Arc<UserSession>>, body: Bytes) -> StatusCode {
    s.set_upload(UploadCache::Backup(Zeroizing::new(body.to_vec())));
    StatusCode::OK
}

// Returns the export bytes directly in the response body (SEC-01/D-01) - this handler performs no
// filesystem access itself; `do_export_csv_download` is the only path-writing-free export producer
// it is allowed to call.
async fn export_csv(
    Extension(s): Extension<Arc<UserSession>>,
    State(ctx): State<AppContext>,
    Json(b): Json<ExportCsvRequest>,
) -> Result<Response, ApiError> {
    // Only the password-protected mode runs Argon2 (D-13); the other modes take no permit.
    let _kdf_permit = match b.mode {
        ExportMode::WithPassword { .. } => Some(ctx.kdf.try_acquire()?),
        _ => None,
    };
    let (filename, bytes) = do_export_csv_download(&s.app_state, &ctx.paths, &b.mode, b.confirmed)?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{filename}\"")),
        ],
        bytes,
    )
        .into_response())
}

// Caches the raw uploaded bytes on the requesting browser's own session (SEC-01/D-01/D-03/D-04) -
// performs no filesystem access and returns no data, since `read_csv_headers`/`import_csv_preview`
// read the cache back on their own subsequent calls.
async fn upload_csv(Extension(s): Extension<Arc<UserSession>>, body: Bytes) -> StatusCode {
    s.set_upload(UploadCache::Csv(Zeroizing::new(body.to_vec())));
    StatusCode::OK
}

// No `path` field (SEC-01/D-01): reads the CSV bytes this session already uploaded via
// `/api/upload_csv` instead of a client-supplied filesystem path. Never surfaces a path or raw
// byte content in its error - "No CSV file uploaded yet." / "That file doesn't look like a CSV."
// are the only two messages this can return.
async fn read_csv_headers(Extension(s): Extension<Arc<UserSession>>) -> Result<Json<Vec<String>>, ApiError> {
    let bytes = s
        .csv_upload()
        .ok_or_else(|| pwmgr_core::error::invalid_input_error("No CSV file uploaded yet."))?;
    let text = String::from_utf8(bytes)
        .map_err(|_| pwmgr_core::error::invalid_input_error("That file doesn't look like a CSV."))?;
    Ok(Json(do_read_csv_headers_from_str(&text)?))
}

async fn import_csv_preview(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<ImportCsvPreviewRequest>,
) -> Result<Json<Vec<ImportRecordDto>>, ApiError> {
    let bytes = s
        .csv_upload()
        .ok_or_else(|| pwmgr_core::error::invalid_input_error("No CSV file uploaded yet."))?;
    let text = String::from_utf8(bytes)
        .map_err(|_| pwmgr_core::error::invalid_input_error("That file doesn't look like a CSV."))?;
    Ok(Json(do_import_csv_preview_from_str(&text, &b.format)?))
}

async fn import_csv_commit(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<ImportCsvCommitRequest>,
) -> Result<Json<ImportSummaryDto>, ApiError> {
    let summary = do_import_csv_commit(&s.app_state, b.records)?;
    // Success path only (SEC-01/D-04) - a failed commit keeps the upload cached so the user can
    // retry without re-uploading.
    s.clear_upload();
    Ok(Json(summary))
}

// Explicit cancel point (SEC-01/D-04): the frontend calls this when the user backs out of the
// import flow or switches source after uploading, so a cancelled import does not leave credential
// bytes resident in the server process.
async fn clear_upload(Extension(s): Extension<Arc<UserSession>>) -> StatusCode {
    s.clear_upload();
    StatusCode::OK
}

// Full-fidelity JSON vault export/import (Wave 4 follow-up) - mirrors export_csv/upload_csv/
// import_csv_preview/import_csv_commit immediately above exactly, including the SEC-01/D-01 "no
// client-supplied path, bytes only" rule and the SEC-01/D-04 clear-cache-on-success discipline.
async fn export_json(
    Extension(s): Extension<Arc<UserSession>>,
    State(ctx): State<AppContext>,
    Json(b): Json<ExportJsonRequest>,
) -> Result<Response, ApiError> {
    let _kdf_permit = match b.mode {
        ExportMode::WithPassword { .. } => Some(ctx.kdf.try_acquire()?),
        _ => None,
    };
    let (filename, bytes) = do_export_json_download(&s.app_state, &ctx.paths, &b.mode, b.confirmed)?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{filename}\"")),
        ],
        bytes,
    )
        .into_response())
}

async fn upload_json(Extension(s): Extension<Arc<UserSession>>, body: Bytes) -> StatusCode {
    s.set_upload(UploadCache::Json(Zeroizing::new(body.to_vec())));
    StatusCode::OK
}

async fn import_json_preview(
    Extension(s): Extension<Arc<UserSession>>,
    State(ctx): State<AppContext>,
    Json(b): Json<ImportJsonRequest>,
) -> Result<Json<pwmgr_core::jsonio::JsonImportPreview>, ApiError> {
    let _kdf_permit = if b.password.is_some() { Some(ctx.kdf.try_acquire()?) } else { None };
    let bytes = s
        .json_upload()
        .ok_or_else(|| pwmgr_core::error::invalid_input_error("No export file uploaded yet."))?;
    Ok(Json(do_import_json_preview(&s.app_state, &bytes, b.password.as_deref())?))
}

async fn import_json_commit(
    Extension(s): Extension<Arc<UserSession>>,
    State(ctx): State<AppContext>,
    Json(b): Json<ImportJsonRequest>,
) -> Result<Json<ImportSummaryDto>, ApiError> {
    let _kdf_permit = if b.password.is_some() { Some(ctx.kdf.try_acquire()?) } else { None };
    let bytes = s
        .json_upload()
        .ok_or_else(|| pwmgr_core::error::invalid_input_error("No export file uploaded yet."))?;
    let summary = do_import_json_commit(&s.app_state, &bytes, b.password.as_deref())?;
    s.clear_upload();
    Ok(Json(summary))
}

async fn health_report(
    Extension(s): Extension<Arc<UserSession>>,
) -> Result<Json<HealthReportDto>, ApiError> {
    Ok(Json(do_health_report(&s.app_state)?))
}

// spawn_blocking is mandatory here: do_check_breach uses reqwest::blocking, which panics if run
// directly on a tokio async worker (Finding 1). Arc<UserSession> is Send+Sync, so the clone moves in.
async fn check_breach(
    Extension(s): Extension<Arc<UserSession>>,
) -> Result<Json<BreachReportDto>, ApiError> {
    let report = tokio::task::spawn_blocking(move || {
        do_check_breach(&s.app_state, pwmgr_core::hibp::HIBP_BASE_URL)
    })
    .await
    .map_err(|_| ApiError(pwmgr_core::error::internal_error()))??;
    Ok(Json(report))
}

/// Assembles the full app: public `login`/`register`, the auth-gated command routes, and a
/// static-file fallback that serves the built React bundle (SPA-style: unknown paths → index.html).
/// `csrf_middleware` wraps everything; GET static requests are exempt, all POST commands require the
/// header. Exposed (not private) so the integration test can build the app without a real bind.
pub fn build_router(ctx: AppContext) -> Router {
    let protected = Router::new()
        .route("/api/unlock_status", post(unlock_status))
        .route("/api/lock", post(lock))
        .route("/api/touch", post(touch))
        .route("/api/verify_master_password", post(verify_master_password))
        .route("/api/change_master_password", post(change_master_password))
        .route("/api/set_pin", post(set_pin))
        .route("/api/clear_pin", post(clear_pin))
        .route("/api/list_sites", post(list_sites))
        .route("/api/add_site", post(add_site))
        .route("/api/update_site", post(update_site))
        .route("/api/soft_delete_site", post(soft_delete_site))
        .route("/api/set_site_favorite", post(set_site_favorite))
        .route("/api/set_site_item_type", post(set_site_item_type))
        .route("/api/set_site_card_hint", post(set_site_card_hint))
        .route("/api/set_site_folder", post(set_site_folder))
        .route("/api/list_folders", post(list_folders))
        .route("/api/add_folder", post(add_folder))
        .route("/api/rename_folder", post(rename_folder))
        .route("/api/delete_folder", post(delete_folder))
        .route("/api/add_credential", post(add_credential))
        .route("/api/update_credential", post(update_credential))
        .route("/api/list_credentials_for_site", post(list_credentials_for_site))
        .route("/api/search", post(search))
        .route("/api/soft_delete_credential", post(soft_delete_credential))
        .route("/api/set_totp_secret", post(set_totp_secret))
        .route("/api/clear_totp_secret", post(clear_totp_secret))
        .route("/api/totp_code", post(totp_code))
        .route("/api/list_credential_history", post(list_credential_history))
        .route("/api/list_trash", post(list_trash))
        .route("/api/restore_credential", post(restore_credential))
        .route("/api/purge_trash", post(purge_trash))
        .route("/api/get_config", post(get_config))
        .route("/api/update_auto_lock_minutes", post(update_auto_lock_minutes))
        .route("/api/update_strength_algorithm", post(update_strength_algorithm))
        .route("/api/update_theme", post(update_theme))
        .route("/api/update_accent_color", post(update_accent_color))
        .route("/api/update_old_password_days", post(update_old_password_days))
        .route("/api/update_hibp_opt_in", post(update_hibp_opt_in))
        .route("/api/update_nav_collapsed", post(update_nav_collapsed))
        .route("/api/update_mobile_nav_style", post(update_mobile_nav_style))
        .route("/api/update_ui_variant", post(update_ui_variant))
        .route("/api/update_reprompt_mode", post(update_reprompt_mode))
        .route("/api/update_reprompt_grace_seconds", post(update_reprompt_grace_seconds))
        .route("/api/generate_password", post(generate_password))
        .route("/api/analyze_strength", post(analyze_strength))
        .route("/api/create_backup", post(create_backup))
        .route("/api/create_backup_download", post(create_backup_download))
        .route("/api/restore_backup", post(restore_backup))
        .route("/api/export_csv", post(export_csv))
        .route("/api/export_json", post(export_json))
        .route(
            "/api/upload_json",
            post(upload_json).layer(axum::extract::DefaultBodyLimit::max(UPLOAD_BODY_LIMIT)),
        )
        .route("/api/import_json_preview", post(import_json_preview))
        .route("/api/import_json_commit", post(import_json_commit))
        .route(
            "/api/upload_csv",
            post(upload_csv).layer(axum::extract::DefaultBodyLimit::max(UPLOAD_BODY_LIMIT)),
        )
        .route(
            "/api/upload_backup",
            post(upload_backup).layer(axum::extract::DefaultBodyLimit::max(UPLOAD_BODY_LIMIT)),
        )
        .route("/api/read_csv_headers", post(read_csv_headers))
        .route("/api/import_csv_preview", post(import_csv_preview))
        .route("/api/import_csv_commit", post(import_csv_commit))
        .route("/api/clear_upload", post(clear_upload))
        .route("/api/health_report", post(health_report))
        .route("/api/check_breach", post(check_breach))
        .route_layer(axum::middleware::from_fn_with_state(
            ctx.store.clone(),
            crate::middleware::auth_middleware,
        ));

    let public = Router::new()
        .route("/api/login", post(login))
        .route("/api/register_user", post(register))
        .route("/api/has_pin", post(has_pin))
        .route("/api/vault_exists", post(vault_exists))
        .route("/api/unlock_with_pin", post(unlock_with_pin))
        .route("/api/logout", post(logout));

    Router::new()
        .merge(public)
        .merge(protected)
        .fallback_service(ServeDir::new("dist").fallback(ServeFile::new("dist/index.html")))
        .layer(axum::middleware::from_fn(crate::middleware::csrf_middleware))
        // ORDER MATTERS: last-added layer is outermost. Security headers must sit outside CSRF/auth/
        // throttle so rejected requests still get them. Do not reorder.
        .layer(axum::middleware::from_fn(crate::middleware::security_headers_middleware))
        .with_state(ctx)
}
