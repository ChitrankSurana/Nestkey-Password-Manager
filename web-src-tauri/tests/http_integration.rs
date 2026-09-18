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

#[tokio::test]
async fn update_reprompt_mode_route_persists_and_is_visible_in_get_config() {
    let (base, _dir) = spawn_server().await;
    let client = reqwest::Client::builder().cookie_store(true).build().unwrap();

    client
        .post(format!("{base}/api/register_user"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"username":"alice","masterPassword":"correct horse battery staple"}))
        .send()
        .await
        .unwrap();

    let res = client
        .post(format!("{base}/api/update_reprompt_mode"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"mode":"global"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let config: serde_json::Value = client
        .post(format!("{base}/api/get_config"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(config["repromptMode"], "global");
}

#[tokio::test]
async fn update_reprompt_mode_route_rejects_an_unknown_mode() {
    let (base, _dir) = spawn_server().await;
    let client = reqwest::Client::builder().cookie_store(true).build().unwrap();

    client
        .post(format!("{base}/api/register_user"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"username":"alice","masterPassword":"correct horse battery staple"}))
        .send()
        .await
        .unwrap();

    let res = client
        .post(format!("{base}/api/update_reprompt_mode"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"mode":"always"}))
        .send()
        .await
        .unwrap();
    assert_ne!(res.status(), 200, "an unknown reprompt mode must not succeed");
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["code"], "invalid_input");
}

#[tokio::test]
async fn update_reprompt_grace_seconds_route_persists() {
    let (base, _dir) = spawn_server().await;
    let client = reqwest::Client::builder().cookie_store(true).build().unwrap();

    client
        .post(format!("{base}/api/register_user"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"username":"alice","masterPassword":"correct horse battery staple"}))
        .send()
        .await
        .unwrap();

    let res = client
        .post(format!("{base}/api/update_reprompt_grace_seconds"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"seconds":60}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let config: serde_json::Value = client
        .post(format!("{base}/api/get_config"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(config["repromptGraceSeconds"], 60);
}

#[tokio::test]
async fn export_csv_returns_the_file_bytes_in_the_response_body() {
    let (base, _dir) = spawn_server().await;
    let client = reqwest::Client::builder().cookie_store(true).build().unwrap();

    client
        .post(format!("{base}/api/register_user"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"username":"alice","masterPassword":"correct horse battery staple"}))
        .send()
        .await
        .unwrap();

    // No destination in the request body at all (SEC-01/D-01) - just mode and confirmed.
    let res = client
        .post(format!("{base}/api/export_csv"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"mode":{"mode":"thisDevice"},"confirmed":false}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let disposition = res
        .headers()
        .get("content-disposition")
        .expect("export_csv must return a content-disposition header")
        .to_str()
        .unwrap()
        .to_string();
    assert!(
        disposition.contains("attachment; filename="),
        "unexpected content-disposition: {disposition}"
    );
    let body = res.bytes().await.unwrap();
    assert!(!body.is_empty(), "exported body must not be empty");
}

#[tokio::test]
async fn export_csv_returns_bytes_for_every_mode() {
    let (base, _dir) = spawn_server().await;
    let client = reqwest::Client::builder().cookie_store(true).build().unwrap();

    client
        .post(format!("{base}/api/register_user"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"username":"alice","masterPassword":"correct horse battery staple"}))
        .send()
        .await
        .unwrap();

    let site_id: i64 = client
        .post(format!("{base}/api/add_site"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"name":"example.com","url":null,"notes":null}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    client
        .post(format!("{base}/api/add_credential"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"siteId": site_id, "username":"alice", "secret":"s3cret!", "remarks": null}))
        .send()
        .await
        .unwrap();

    let this_device = client
        .post(format!("{base}/api/export_csv"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"mode":{"mode":"thisDevice"},"confirmed":false}))
        .send()
        .await
        .unwrap();
    assert_eq!(this_device.status(), 200);
    assert!(this_device.headers().get("content-disposition").is_some());
    let this_device_body = this_device.bytes().await.unwrap();
    assert!(!this_device_body.is_empty());

    let with_password = client
        .post(format!("{base}/api/export_csv"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"mode":{"mode":"withPassword","password":"pw"},"confirmed":false}))
        .send()
        .await
        .unwrap();
    assert_eq!(with_password.status(), 200);
    assert!(with_password.headers().get("content-disposition").is_some());
    let with_password_body = with_password.bytes().await.unwrap();
    assert!(!with_password_body.is_empty());
    assert_ne!(
        this_device_body, with_password_body,
        "withPassword mode embeds a fresh random salt, so its bytes must differ from thisDevice"
    );

    let plaintext = client
        .post(format!("{base}/api/export_csv"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"mode":{"mode":"plaintext"},"confirmed":true}))
        .send()
        .await
        .unwrap();
    assert_eq!(plaintext.status(), 200);
    assert!(plaintext.headers().get("content-disposition").is_some());
    let plaintext_body = plaintext.text().await.unwrap();
    assert!(plaintext_body.contains("example.com"), "plaintext body must contain the site name");
}

#[tokio::test]
async fn export_csv_plaintext_without_confirmation_is_rejected() {
    let (base, _dir) = spawn_server().await;
    let client = reqwest::Client::builder().cookie_store(true).build().unwrap();

    client
        .post(format!("{base}/api/register_user"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"username":"alice","masterPassword":"correct horse battery staple"}))
        .send()
        .await
        .unwrap();

    let res = client
        .post(format!("{base}/api/export_csv"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"mode":{"mode":"plaintext"},"confirmed":false}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 400);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["code"], "invalid_input");
}

#[tokio::test]
async fn export_csv_writes_no_server_side_file_for_a_crafted_destination() {
    let (base, dir) = spawn_server().await;
    let client = reqwest::Client::builder().cookie_store(true).build().unwrap();

    client
        .post(format!("{base}/api/register_user"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"username":"alice","masterPassword":"correct horse battery staple"}))
        .send()
        .await
        .unwrap();

    let target = dir.path().join("pwned.csv");
    let res = client
        .post(format!("{base}/api/export_csv"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({
            "mode":{"mode":"thisDevice"},
            "confirmed": false,
            "path": target.to_string_lossy(),
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200, "export_csv should succeed regardless of the ignored extra field");
    let body = res.bytes().await.unwrap();
    assert!(!body.is_empty());

    assert!(!target.exists(), "no file should ever be created at the client-supplied destination");
}

#[tokio::test]
async fn plaintext_web_export_logs_the_download_label() {
    let (base, dir) = spawn_server().await;
    let client = reqwest::Client::builder().cookie_store(true).build().unwrap();

    client
        .post(format!("{base}/api/register_user"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"username":"alice","masterPassword":"correct horse battery staple"}))
        .send()
        .await
        .unwrap();

    let res = client
        .post(format!("{base}/api/export_csv"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"mode":{"mode":"plaintext"},"confirmed":true}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let paths = AppPaths::new(dir.path().to_path_buf());
    let log = std::fs::read_to_string(paths.plaintext_export_log_path()).unwrap();
    let last_line = log.lines().last().expect("plaintext export must append a log line");
    let fields: Vec<&str> = last_line.split('\t').collect();
    assert_eq!(
        fields.get(1),
        Some(&"<web download>"),
        "web-mode plaintext export log entry must record the download label, not a path; got: {last_line}"
    );
}

#[tokio::test]
async fn upload_csv_then_read_csv_headers_round_trips_over_http() {
    let (base, _dir) = spawn_server().await;
    let client = reqwest::Client::builder().cookie_store(true).build().unwrap();

    client
        .post(format!("{base}/api/register_user"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"username":"alice","masterPassword":"correct horse battery staple"}))
        .send()
        .await
        .unwrap();

    let upload_res = client
        .post(format!("{base}/api/upload_csv"))
        .header("x-pwmgr-csrf", "1")
        .header("content-type", "application/octet-stream")
        .body("name,url,username,password\nexample.com,https://example.com,alice,s3cret\n")
        .send()
        .await
        .unwrap();
    assert_eq!(upload_res.status(), 200, "upload_csv should cache the bytes and return 200");

    let headers_res = client
        .post(format!("{base}/api/read_csv_headers"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(headers_res.status(), 200);
    let headers: Vec<String> = headers_res.json().await.unwrap();
    assert_eq!(headers, vec!["name", "url", "username", "password"]);
}

#[tokio::test]
async fn read_csv_headers_without_an_upload_is_rejected() {
    let (base, _dir) = spawn_server().await;
    let client = reqwest::Client::builder().cookie_store(true).build().unwrap();

    client
        .post(format!("{base}/api/register_user"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"username":"alice","masterPassword":"correct horse battery staple"}))
        .send()
        .await
        .unwrap();

    let res = client
        .post(format!("{base}/api/read_csv_headers"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 400);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["code"], "invalid_input");
}

#[tokio::test]
async fn a_second_session_never_sees_the_first_sessions_uploaded_csv() {
    let (base, _dir) = spawn_server().await;
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
        .post(format!("{base}/api/upload_csv"))
        .header("x-pwmgr-csrf", "1")
        .header("content-type", "application/octet-stream")
        .body("name,url\nexample.com,https://example.com\n")
        .send()
        .await
        .unwrap();

    let bob_res = bob
        .post(format!("{base}/api/read_csv_headers"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(bob_res.status(), 400, "bob's session must never see alice's uploaded CSV");
    let body: serde_json::Value = bob_res.json().await.unwrap();
    assert_eq!(body["code"], "invalid_input");
}

#[tokio::test]
async fn upload_csv_without_a_session_cookie_is_unauthorized() {
    let (base, _dir) = spawn_server().await;
    let client = reqwest::Client::new();

    let res = client
        .post(format!("{base}/api/upload_csv"))
        .header("x-pwmgr-csrf", "1")
        .header("content-type", "application/octet-stream")
        .body("name,url\nexample.com,https://example.com\n")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 401);
}
