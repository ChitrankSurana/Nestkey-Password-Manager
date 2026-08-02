use crate::error_map::ApiError;
use crate::session_store::{SessionStore, UserSession};
use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Extension, Json, Router};
use pwmgr_core::commands::*;
use pwmgr_core::dto::{
    AuthSuccess, BreachReportDto, ConfigDto, CredentialDto, ExportMode, GenerateRequest,
    HealthReportDto, ImportFormat, ImportRecordDto, ImportSummaryDto, SearchResultDto, SiteDto,
    StrengthDto, TotpCodeDto, TrashedCredentialDto,
};
use pwmgr_core::paths::AppPaths;
use pwmgr_core::state::AppState;
use pwmgr_core::vault::CustomField;
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tower_http::services::{ServeDir, ServeFile};
use zeroize::Zeroizing;

/// Shared axum state: the session store + the fixed app-data paths.
#[derive(Clone)]
pub struct AppContext {
    pub store: SessionStore,
    pub paths: Arc<AppPaths>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthRequest {
    username: String,
    master_password: String,
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
    reprompt: bool,
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
    reprompt: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SiteIdRequest {
    site_id: i64,
}

#[derive(Deserialize)]
struct SearchRequest {
    query: String,
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
    path: String,
    confirmed: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportCsvRequest {
    path: String,
    mode: ExportMode,
    confirmed: bool,
}

#[derive(Deserialize)]
struct ReadCsvHeadersRequest {
    path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportCsvPreviewRequest {
    path: String,
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
    };
    let token = ctx.store.insert(session);
    let cookie = format!("session={token}; HttpOnly; SameSite=Strict; Path=/");
    let mut res = Json(AuthSuccess { username: body.username.clone() }).into_response();
    res.headers_mut().insert(header::SET_COOKIE, cookie.parse().unwrap());
    Ok(res)
}

async fn login(State(ctx): State<AppContext>, Json(body): Json<AuthRequest>) -> Result<Response, ApiError> {
    issue_session(&ctx, &body, false)
}

async fn register(State(ctx): State<AppContext>, Json(body): Json<AuthRequest>) -> Result<Response, ApiError> {
    issue_session(&ctx, &body, true)
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

async fn add_credential(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<AddCredentialRequest>,
) -> Result<Json<i64>, ApiError> {
    Ok(Json(do_add_credential(
        &s.app_state, b.site_id, b.username.as_deref(), &b.secret, b.remarks.as_deref(),
        &b.custom_fields, b.reprompt,
    )?))
}

async fn update_credential(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<UpdateCredentialRequest>,
) -> Result<StatusCode, ApiError> {
    do_update_credential(
        &s.app_state, b.credential_id, b.username.as_deref(), &b.secret, b.remarks.as_deref(),
        &b.custom_fields, b.reprompt,
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

async fn create_backup(
    Extension(s): Extension<Arc<UserSession>>,
    State(ctx): State<AppContext>,
) -> Result<Json<String>, ApiError> {
    Ok(Json(do_create_backup(&s.app_state, &ctx.paths)?))
}

async fn restore_backup(
    Extension(s): Extension<Arc<UserSession>>,
    State(ctx): State<AppContext>,
    Json(b): Json<RestoreBackupRequest>,
) -> Result<StatusCode, ApiError> {
    do_restore_backup(&s.app_state, &ctx.paths, &b.path, b.confirmed)?;
    Ok(StatusCode::OK)
}

async fn export_csv(
    Extension(s): Extension<Arc<UserSession>>,
    State(ctx): State<AppContext>,
    Json(b): Json<ExportCsvRequest>,
) -> Result<StatusCode, ApiError> {
    do_export_csv(&s.app_state, &ctx.paths, &b.path, &b.mode, b.confirmed)?;
    Ok(StatusCode::OK)
}

async fn read_csv_headers(
    Extension(_s): Extension<Arc<UserSession>>,
    Json(b): Json<ReadCsvHeadersRequest>,
) -> Result<Json<Vec<String>>, ApiError> {
    Ok(Json(do_read_csv_headers(&b.path)?))
}

async fn import_csv_preview(
    Extension(_s): Extension<Arc<UserSession>>,
    Json(b): Json<ImportCsvPreviewRequest>,
) -> Result<Json<Vec<ImportRecordDto>>, ApiError> {
    Ok(Json(do_import_csv_preview(&b.path, &b.format)?))
}

async fn import_csv_commit(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<ImportCsvCommitRequest>,
) -> Result<Json<ImportSummaryDto>, ApiError> {
    Ok(Json(do_import_csv_commit(&s.app_state, b.records)?))
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
        .route("/api/list_sites", post(list_sites))
        .route("/api/add_site", post(add_site))
        .route("/api/update_site", post(update_site))
        .route("/api/soft_delete_site", post(soft_delete_site))
        .route("/api/add_credential", post(add_credential))
        .route("/api/update_credential", post(update_credential))
        .route("/api/list_credentials_for_site", post(list_credentials_for_site))
        .route("/api/search", post(search))
        .route("/api/soft_delete_credential", post(soft_delete_credential))
        .route("/api/set_totp_secret", post(set_totp_secret))
        .route("/api/clear_totp_secret", post(clear_totp_secret))
        .route("/api/totp_code", post(totp_code))
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
        .route("/api/generate_password", post(generate_password))
        .route("/api/analyze_strength", post(analyze_strength))
        .route("/api/create_backup", post(create_backup))
        .route("/api/restore_backup", post(restore_backup))
        .route("/api/export_csv", post(export_csv))
        .route("/api/read_csv_headers", post(read_csv_headers))
        .route("/api/import_csv_preview", post(import_csv_preview))
        .route("/api/import_csv_commit", post(import_csv_commit))
        .route("/api/health_report", post(health_report))
        .route("/api/check_breach", post(check_breach))
        .route_layer(axum::middleware::from_fn_with_state(
            ctx.store.clone(),
            crate::middleware::auth_middleware,
        ));

    let public = Router::new()
        .route("/api/login", post(login))
        .route("/api/register_user", post(register))
        .route("/api/logout", post(logout));

    Router::new()
        .merge(public)
        .merge(protected)
        .fallback_service(ServeDir::new("dist").fallback(ServeFile::new("dist/index.html")))
        .layer(axum::middleware::from_fn(crate::middleware::csrf_middleware))
        .with_state(ctx)
}
