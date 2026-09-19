use crate::error_map::ApiError;
use crate::session_store::{SessionStore, Token};
use axum::body::Body;
use axum::extract::State;
use axum::http::{header, HeaderValue, Method, Request};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use pwmgr_core::session::SystemClock;

/// Custom header a cross-site `<form>`/`<img>` cannot set. Its presence (any value) is required on
/// every mutating (non-GET) request — a second line of defense behind the SameSite=Strict cookie.
pub const CSRF_HEADER: &str = "x-pwmgr-csrf";

/// Resolves the session from the `session=<token>` cookie. 401 if the cookie is missing, the token
/// is unknown, or the entry's vault is locked (a locked entry stays in the map so the same browser
/// can re-authenticate, but its in-flight requests are rejected). On success, bumps `last_seen` and
/// injects the `Arc<UserSession>` into request extensions for the handler to extract.
pub async fn auth_middleware(
    State(store): State<SessionStore>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let token = req
        .headers()
        .get(header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .and_then(parse_session_cookie);
    let session = match token.and_then(|t| store.get(&t)) {
        Some(s) => s,
        None => return unauthorized(),
    };
    if session.app_state.session.lock().unwrap().is_locked() {
        return unauthorized();
    }
    session.bump_last_seen(&SystemClock);
    req.extensions_mut().insert(session);
    next.run(req).await
}

/// Requires the anti-CSRF header on every non-GET request. GET is exempt (it mutates nothing, and a
/// cross-origin GET's response is unreadable without CORS, which this server never sends).
pub async fn csrf_middleware(req: Request<Body>, next: Next) -> Response {
    if req.method() != Method::GET && req.headers().get(CSRF_HEADER).is_none() {
        return ApiError::from(pwmgr_core::error::forbidden_error("Missing CSRF header."))
            .into_response();
    }
    next.run(req).await
}

/// Production Content-Security-Policy (D-15/D-16). Must stay byte-identical to `app.security.csp`
/// in `src-tauri/tauri.conf.json`.
pub const CSP_POLICY: &str = "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self'; font-src 'self'; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'";

/// Stamps the security headers on EVERY response, unconditionally (no branch on route/status/body).
/// Must be the outermost layer so CSRF/auth/throttle rejections still pass back through it.
pub async fn security_headers_middleware(req: Request<Body>, next: Next) -> Response {
    let mut res = next.run(req).await;
    let h = res.headers_mut();
    h.insert(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static(CSP_POLICY));
    h.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    h.insert(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    h.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    res
}

/// Shared with `handlers::logout`, which needs to resolve the same cookie outside the
/// `auth_middleware` gate (logout must work even for an already-locked or unknown session).
pub(crate) fn parse_session_cookie(cookie_header: &str) -> Option<Token> {
    cookie_header
        .split(';')
        .map(str::trim)
        .find_map(|kv| kv.strip_prefix("session=").map(|v| v.to_string()))
}

fn unauthorized() -> Response {
    // Reuse pwmgr-core's sanitized "locked" error as the body - the same {code,message} the frontend
    // already branches on, and consistent with a genuine idle auto-lock 401. Routed through ApiError
    // (Task 13) so the status derives from status_for("locked") rather than a second hardcoded 401.
    ApiError::from(pwmgr_core::error::locked_error()).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_store::UserSession;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::{get, post};
    use axum::Router;
    use pwmgr_core::config::Config;
    use pwmgr_core::paths::AppPaths;
    use pwmgr_core::session::{Session, SystemClock};
    use pwmgr_core::state::AppState;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    use tower::ServiceExt;

    fn store_with_session(unlocked: bool) -> (SessionStore, String, tempfile::TempDir) {
        let store = SessionStore::new();
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(dir.path().to_path_buf());
        paths.ensure_app_dirs().unwrap();
        let registry = pwmgr_core::registry::open_registry(&paths.registry_db_path()).unwrap();
        let app_state = AppState {
            registry: Mutex::new(registry),
            vault: Mutex::new(None),
            session: Mutex::new(Session::new(Duration::from_secs(300))),
            config: Mutex::new(Config::default()),
        };
        if unlocked {
            let key = pwmgr_core::crypto::derive_key("pw", &[1u8; 16]).unwrap();
            app_state.session.lock().unwrap().unlock(key, "alice", &SystemClock);
        }
        let token = store.insert(UserSession {
            username: "alice".into(),
            app_state,
            last_seen: Mutex::new(Instant::now()),
            upload_cache: Mutex::new(None),
        });
        (store, token, dir)
    }

    fn auth_app(store: SessionStore) -> Router {
        Router::new()
            .route("/protected", get(|| async { "ok" }))
            .layer(axum::middleware::from_fn_with_state(store, auth_middleware))
    }

    fn csrf_app() -> Router {
        Router::new()
            .route("/thing", post(|| async { "ok" }).get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(csrf_middleware))
    }

    async fn status_of(app: Router, req: Request<Body>) -> StatusCode {
        app.oneshot(req).await.unwrap().status()
    }

    #[tokio::test]
    async fn missing_cookie_returns_401() {
        let (store, _t, _d) = store_with_session(true);
        let req = Request::builder().uri("/protected").body(Body::empty()).unwrap();
        assert_eq!(status_of(auth_app(store), req).await, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn unknown_token_returns_401() {
        let (store, _t, _d) = store_with_session(true);
        let req = Request::builder()
            .uri("/protected")
            .header("cookie", "session=deadbeef")
            .body(Body::empty())
            .unwrap();
        assert_eq!(status_of(auth_app(store), req).await, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn valid_unlocked_token_reaches_the_handler() {
        let (store, token, _d) = store_with_session(true);
        let req = Request::builder()
            .uri("/protected")
            .header("cookie", format!("session={token}"))
            .body(Body::empty())
            .unwrap();
        assert_eq!(status_of(auth_app(store), req).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn locked_session_returns_401() {
        let (store, token, _d) = store_with_session(false);
        let req = Request::builder()
            .uri("/protected")
            .header("cookie", format!("session={token}"))
            .body(Body::empty())
            .unwrap();
        assert_eq!(status_of(auth_app(store), req).await, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn post_without_csrf_header_returns_403() {
        let req = Request::builder().method("POST").uri("/thing").body(Body::empty()).unwrap();
        assert_eq!(status_of(csrf_app(), req).await, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn post_with_csrf_header_reaches_the_handler() {
        let req = Request::builder()
            .method("POST")
            .uri("/thing")
            .header(CSRF_HEADER, "1")
            .body(Body::empty())
            .unwrap();
        assert_eq!(status_of(csrf_app(), req).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn csrf_rejection_still_carries_the_csp() {
        let app = csrf_app().layer(axum::middleware::from_fn(security_headers_middleware));
        let req = Request::builder().method("POST").uri("/thing").body(Body::empty()).unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        assert_eq!(res.headers()["content-security-policy"], CSP_POLICY);
    }

    #[tokio::test]
    async fn get_without_csrf_header_is_allowed() {
        let req = Request::builder().method("GET").uri("/thing").body(Body::empty()).unwrap();
        assert_eq!(status_of(csrf_app(), req).await, StatusCode::OK);
    }
}
