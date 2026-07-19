use pwmgr_core::paths::AppPaths;
use pwmgr_web::{build_router, AppContext, SessionStore};
use std::sync::Arc;

/// Starts the real router on an ephemeral 127.0.0.1 port; returns its base URL and the tempdir whose
/// ~/.pwmgr the server reads/writes (kept alive by the caller binding `_dir`).
async fn spawn_server() -> (String, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(dir.path().to_path_buf());
    paths.ensure_app_dirs().unwrap();
    let ctx = AppContext { store: SessionStore::new(), paths: Arc::new(paths) };
    let app = build_router(ctx);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}"), dir)
}

#[tokio::test]
async fn register_then_list_sites_roundtrips_over_http() {
    let (base, _dir) = spawn_server().await;
    let client = reqwest::Client::builder().cookie_store(true).build().unwrap();

    let res = client
        .post(format!("{base}/api/register_user"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"username":"alice","masterPassword":"correct horse battery staple"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200, "register should succeed and set the session cookie");

    let res = client
        .post(format!("{base}/api/list_sites"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let sites: Vec<serde_json::Value> = res.json().await.unwrap();
    assert!(sites.is_empty(), "a brand-new vault has no sites");
}

#[tokio::test]
async fn a_mutating_request_without_the_csrf_header_is_rejected() {
    let (base, _dir) = spawn_server().await;
    let client = reqwest::Client::new();
    let res = client
        .post(format!("{base}/api/list_sites"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 403);
}

#[tokio::test]
async fn an_authed_route_without_a_session_cookie_is_unauthorized() {
    let (base, _dir) = spawn_server().await;
    let client = reqwest::Client::new();
    let res = client
        .post(format!("{base}/api/list_sites"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 401);
}
