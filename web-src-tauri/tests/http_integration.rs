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

#[tokio::test]
async fn logout_removes_the_session_so_the_old_cookie_no_longer_works() {
    let (base, _dir) = spawn_server().await;
    let client = reqwest::Client::builder().cookie_store(true).build().unwrap();

    let res = client
        .post(format!("{base}/api/register_user"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"username":"alice","masterPassword":"correct horse battery staple"}))
        .send()
        .await
        .unwrap();
    // Grab the raw session cookie so it can be replayed explicitly after logout - proves the server
    // actually dropped the session, not just that this client's own cookie jar forgot it.
    let set_cookie = res.headers().get("set-cookie").unwrap().to_str().unwrap().to_string();
    let raw_cookie = set_cookie.split(';').next().unwrap().to_string();

    let res = client
        .post(format!("{base}/api/logout"))
        .header("x-pwmgr-csrf", "1")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let res = reqwest::Client::new()
        .post(format!("{base}/api/list_sites"))
        .header("x-pwmgr-csrf", "1")
        .header("cookie", raw_cookie)
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        401,
        "logout must remove the session server-side, not just clear the client's cookie"
    );
}

#[tokio::test]
async fn two_browser_sessions_over_real_connections_stay_isolated() {
    let (base, _dir) = spawn_server().await;
    // Two independent cookie jars - the real-connection equivalent of two separate browser tabs/
    // profiles, per the deferred item this test resolves (docs/KNOWN_LIMITATIONS.md).
    let alice = reqwest::Client::builder().cookie_store(true).build().unwrap();
    let bob = reqwest::Client::builder().cookie_store(true).build().unwrap();

    alice
        .post(format!("{base}/api/register_user"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"username":"alice","masterPassword":"alice password one"}))
        .send()
        .await
        .unwrap();
    bob.post(format!("{base}/api/register_user"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"username":"bob","masterPassword":"bob password one"}))
        .send()
        .await
        .unwrap();

    alice
        .post(format!("{base}/api/add_site"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"name":"AliceOnlySite","url":null,"notes":null}))
        .send()
        .await
        .unwrap();

    let bob_sites: Vec<serde_json::Value> = bob
        .post(format!("{base}/api/list_sites"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(bob_sites.is_empty(), "bob's session must never see alice's site");

    let alice_sites: Vec<serde_json::Value> = alice
        .post(format!("{base}/api/list_sites"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(alice_sites.len(), 1, "alice's own session must see her site");

    // Locking one session must not affect the other - each holds fully independent AppState.
    let lock_res = alice
        .post(format!("{base}/api/lock"))
        .header("x-pwmgr-csrf", "1")
        .send()
        .await
        .unwrap();
    assert_eq!(lock_res.status(), 200);

    let bob_unlocked: bool = bob
        .post(format!("{base}/api/unlock_status"))
        .header("x-pwmgr-csrf", "1")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(bob_unlocked, "locking alice's session must not lock bob's");
}

#[tokio::test]
async fn create_backup_route_ignores_a_client_supplied_destination() {
    let (base, dir) = spawn_server().await;
    let client = reqwest::Client::builder().cookie_store(true).build().unwrap();

    client
        .post(format!("{base}/api/register_user"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"username":"alice","masterPassword":"correct horse battery staple"}))
        .send()
        .await
        .unwrap();

    // A destination the server's paths (backups_dir under `dir`) never own.
    let attacker_dir = tempfile::tempdir().unwrap();
    let attacker_target = attacker_dir.path().join("attacker-chosen.bak");

    let res = client
        .post(format!("{base}/api/create_backup"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"destinationPath": attacker_target.to_string_lossy()}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200, "create_backup should succeed regardless of the ignored body");

    let returned_path: String = res.json().await.unwrap();
    let paths = AppPaths::new(dir.path().to_path_buf());
    assert!(
        std::path::Path::new(&returned_path).starts_with(paths.backups_dir()),
        "returned path must be inside the server's own backups directory, got {returned_path}"
    );
    assert!(
        !attacker_target.exists(),
        "no file should ever be created at the client-supplied destination"
    );
}
