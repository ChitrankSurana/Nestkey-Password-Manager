use pwmgr_core::paths::AppPaths;
use pwmgr_web::{build_router, AppContext, KdfGuard, SessionStore, ThrottleStore};
use std::sync::Arc;

/// Starts the real router on an ephemeral 127.0.0.1 port; returns its base URL and the tempdir whose
/// ~/.pwmgr the server reads/writes (kept alive by the caller binding `_dir`).
async fn spawn_server() -> (String, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(dir.path().to_path_buf());
    paths.ensure_app_dirs().unwrap();
    let ctx = AppContext {
        store: SessionStore::new(),
        paths: Arc::new(paths),
        throttle: ThrottleStore::new(),
        kdf: KdfGuard::new(),
    };
    let app = build_router(ctx);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>())
            .await
            .unwrap();
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

#[tokio::test]
async fn csv_import_uploads_once_then_previews_and_commits() {
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
    assert_eq!(upload_res.status(), 200);

    let preview_res = client
        .post(format!("{base}/api/import_csv_preview"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"format":{"format":"chromium"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(preview_res.status(), 200);
    let records: Vec<serde_json::Value> = preview_res.json().await.unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["siteName"], "example.com");

    let commit_res = client
        .post(format!("{base}/api/import_csv_commit"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"records": records}))
        .send()
        .await
        .unwrap();
    assert_eq!(commit_res.status(), 200);
    let summary: serde_json::Value = commit_res.json().await.unwrap();
    assert_eq!(summary["imported"], 1);

    let sites: Vec<serde_json::Value> = client
        .post(format!("{base}/api/list_sites"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        sites.iter().any(|s| s["name"] == "example.com"),
        "the committed import must be visible via list_sites"
    );
}

#[tokio::test]
async fn import_csv_preview_without_an_upload_is_rejected() {
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
        .post(format!("{base}/api/import_csv_preview"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"format":{"format":"chromium"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 400);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["code"], "invalid_input");
}

#[tokio::test]
async fn one_upload_serves_two_previews() {
    let (base, _dir) = spawn_server().await;
    let client = reqwest::Client::builder().cookie_store(true).build().unwrap();

    client
        .post(format!("{base}/api/register_user"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"username":"alice","masterPassword":"correct horse battery staple"}))
        .send()
        .await
        .unwrap();

    client
        .post(format!("{base}/api/upload_csv"))
        .header("x-pwmgr-csrf", "1")
        .header("content-type", "application/octet-stream")
        .body("name,url,username,password\nexample.com,https://example.com,alice,s3cret\n")
        .send()
        .await
        .unwrap();

    let first = client
        .post(format!("{base}/api/import_csv_preview"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"format":{"format":"chromium"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(first.status(), 200);

    // Same single upload, previewed again with a different format - the cache must not have been
    // consumed by the first preview.
    let second = client
        .post(format!("{base}/api/import_csv_preview"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"format":{"format":"generic","mapping":{"name":"site_name"}}}))
        .send()
        .await
        .unwrap();
    assert_eq!(second.status(), 200, "the same upload must serve a second preview with a different format");
}

#[tokio::test]
async fn upload_cache_is_cleared_after_a_successful_commit() {
    let (base, _dir) = spawn_server().await;
    let client = reqwest::Client::builder().cookie_store(true).build().unwrap();

    client
        .post(format!("{base}/api/register_user"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"username":"alice","masterPassword":"correct horse battery staple"}))
        .send()
        .await
        .unwrap();

    client
        .post(format!("{base}/api/upload_csv"))
        .header("x-pwmgr-csrf", "1")
        .header("content-type", "application/octet-stream")
        .body("name,url,username,password\nexample.com,https://example.com,alice,s3cret\n")
        .send()
        .await
        .unwrap();

    let records: Vec<serde_json::Value> = client
        .post(format!("{base}/api/import_csv_preview"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"format":{"format":"chromium"}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let commit_res = client
        .post(format!("{base}/api/import_csv_commit"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"records": records}))
        .send()
        .await
        .unwrap();
    assert_eq!(commit_res.status(), 200);

    let headers_res = client
        .post(format!("{base}/api/read_csv_headers"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        headers_res.status(),
        400,
        "a successful commit must clear the cached upload server-side"
    );
}

#[tokio::test]
async fn clear_upload_route_drops_the_cached_bytes() {
    let (base, _dir) = spawn_server().await;
    let client = reqwest::Client::builder().cookie_store(true).build().unwrap();

    client
        .post(format!("{base}/api/register_user"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"username":"alice","masterPassword":"correct horse battery staple"}))
        .send()
        .await
        .unwrap();

    client
        .post(format!("{base}/api/upload_csv"))
        .header("x-pwmgr-csrf", "1")
        .header("content-type", "application/octet-stream")
        .body("name,url\nexample.com,https://example.com\n")
        .send()
        .await
        .unwrap();

    let clear_res = client
        .post(format!("{base}/api/clear_upload"))
        .header("x-pwmgr-csrf", "1")
        .send()
        .await
        .unwrap();
    assert_eq!(clear_res.status(), 200);

    let headers_res = client
        .post(format!("{base}/api/read_csv_headers"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(headers_res.status(), 400, "clear_upload must drop the cached bytes");
}

#[tokio::test]
async fn uploads_are_isolated_between_two_browser_sessions() {
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
}

#[tokio::test]
async fn two_concurrent_uploads_leave_one_complete_buffer() {
    let (base, _dir) = spawn_server().await;
    let client = reqwest::Client::builder().cookie_store(true).build().unwrap();

    client
        .post(format!("{base}/api/register_user"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"username":"alice","masterPassword":"correct horse battery staple"}))
        .send()
        .await
        .unwrap();

    let upload_a = client
        .post(format!("{base}/api/upload_csv"))
        .header("x-pwmgr-csrf", "1")
        .header("content-type", "application/octet-stream")
        .body("colA\nvalueA\n");
    let upload_b = client
        .post(format!("{base}/api/upload_csv"))
        .header("x-pwmgr-csrf", "1")
        .header("content-type", "application/octet-stream")
        .body("colB\nvalueB\n");

    let (res_a, res_b) = tokio::join!(upload_a.send(), upload_b.send());
    assert_eq!(res_a.unwrap().status(), 200);
    assert_eq!(res_b.unwrap().status(), 200);

    let headers_res = client
        .post(format!("{base}/api/read_csv_headers"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(headers_res.status(), 200);
    let headers: Vec<String> = headers_res.json().await.unwrap();
    assert!(
        headers == vec!["colA".to_string()] || headers == vec!["colB".to_string()],
        "two concurrent uploads must leave exactly one complete buffer, never a splice; got {headers:?}"
    );
}

#[tokio::test]
async fn an_oversized_upload_is_rejected() {
    let (base, _dir) = spawn_server().await;
    let client = reqwest::Client::builder().cookie_store(true).build().unwrap();

    client
        .post(format!("{base}/api/register_user"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"username":"alice","masterPassword":"correct horse battery staple"}))
        .send()
        .await
        .unwrap();

    let oversized_body = vec![b'a'; pwmgr_web::handlers::UPLOAD_BODY_LIMIT + 1];
    let res = client
        .post(format!("{base}/api/upload_csv"))
        .header("x-pwmgr-csrf", "1")
        .header("content-type", "application/octet-stream")
        .body(oversized_body)
        .send()
        .await
        .unwrap();
    assert!(
        !res.status().is_success(),
        "an upload larger than UPLOAD_BODY_LIMIT must be rejected, got {}",
        res.status()
    );
}

async fn register_as(client: &reqwest::Client, base: &str, name: &str) {
    client
        .post(format!("{base}/api/register_user"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"username":name,"masterPassword":"correct horse battery staple"}))
        .send()
        .await
        .unwrap();
}

/// Creates a backup of the logged-in vault over HTTP and returns the bundle bytes.
async fn backup_bytes(client: &reqwest::Client, base: &str) -> Vec<u8> {
    let res = client
        .post(format!("{base}/api/create_backup"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();
    let path: String = res.json().await.unwrap();
    std::fs::read(path).unwrap()
}

async fn upload_backup(client: &reqwest::Client, base: &str, bytes: Vec<u8>) -> reqwest::Response {
    client
        .post(format!("{base}/api/upload_backup"))
        .header("x-pwmgr-csrf", "1")
        .body(bytes)
        .send()
        .await
        .unwrap()
}

async fn restore(client: &reqwest::Client, base: &str, confirmed: bool) -> reqwest::Response {
    client
        .post(format!("{base}/api/restore_backup"))
        .header("x-pwmgr-csrf", "1")
        .json(&serde_json::json!({"confirmed": confirmed}))
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn backup_upload_then_restore_roundtrips_over_http() {
    let (base, dir) = spawn_server().await;
    let paths = AppPaths::new(dir.path().to_path_buf());
    let alice = reqwest::Client::builder().cookie_store(true).build().unwrap();
    register_as(&alice, &base, "alice").await;
    let bytes = backup_bytes(&alice, &base).await;
    alice.post(format!("{base}/api/logout")).header("x-pwmgr-csrf", "1").json(&serde_json::json!({})).send().await.unwrap();

    // Drop alice's vault, then restore it from the upload as another logged-in user.
    std::fs::remove_file(paths.vault_path("alice")).unwrap();
    let bob = reqwest::Client::builder().cookie_store(true).build().unwrap();
    register_as(&bob, &base, "bob").await;
    assert_eq!(upload_backup(&bob, &base, bytes).await.status(), 200);
    // confirmed=true: the registry row for alice still exists even though the file is gone.
    assert_eq!(restore(&bob, &base, true).await.status(), 200);
    assert!(paths.vault_path("alice").exists(), "restored vault file should exist");
}

#[tokio::test]
async fn restore_backup_without_an_upload_is_rejected() {
    let (base, _dir) = spawn_server().await;
    let client = reqwest::Client::builder().cookie_store(true).build().unwrap();
    register_as(&client, &base, "alice").await;
    let res = restore(&client, &base, false).await;
    assert_eq!(res.status(), 400);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["code"], "invalid_input");
}

#[tokio::test]
async fn restore_backup_rejects_a_non_backup_upload() {
    let (base, dir) = spawn_server().await;
    let client = reqwest::Client::builder().cookie_store(true).build().unwrap();
    register_as(&client, &base, "alice").await;
    let vault_dir = AppPaths::new(dir.path().to_path_buf()).vault_path("alice").parent().unwrap().to_path_buf();
    let before = std::fs::read_dir(&vault_dir).unwrap().count();
    assert_eq!(upload_backup(&client, &base, b"not a backup".to_vec()).await.status(), 200);
    let res = restore(&client, &base, true).await;
    assert_eq!(res.status(), 400);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["code"], "invalid_input");
    assert_eq!(std::fs::read_dir(&vault_dir).unwrap().count(), before, "no vault file created");
}

#[tokio::test]
async fn upload_backup_without_a_session_is_unauthorized() {
    let (base, _dir) = spawn_server().await;
    let client = reqwest::Client::new();
    assert_eq!(upload_backup(&client, &base, vec![1, 2, 3]).await.status(), 401);
}

#[tokio::test]
async fn restore_backup_confirm_roundtrip_reuses_the_single_upload() {
    let (base, _dir) = spawn_server().await;
    let client = reqwest::Client::builder().cookie_store(true).build().unwrap();
    register_as(&client, &base, "alice").await;
    let bytes = backup_bytes(&client, &base).await;
    client.post(format!("{base}/api/logout")).header("x-pwmgr-csrf", "1").json(&serde_json::json!({})).send().await.unwrap();
    // A different logged-in user restores alice's backup over her existing (closed) vault; the
    // acting session stays unlocked so the post-success cache state is observable.
    register_as(&client, &base, "bob").await;

    assert_eq!(upload_backup(&client, &base, bytes).await.status(), 200);
    let res = restore(&client, &base, false).await;
    assert_eq!(res.status(), 409);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["code"], "confirm_required");

    // No second upload: the cache survived the confirm_required.
    assert_eq!(restore(&client, &base, true).await.status(), 200);

    // Cache cleared on success.
    let res = restore(&client, &base, true).await;
    assert_eq!(res.status(), 400);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["code"], "invalid_input");
}

#[test]
fn no_web_handler_accepts_a_client_supplied_destination() {
    // Guards SECURITY_CONCERNS.md H1: none of the four named handlers may take a client-supplied path.
    let src: String = include_str!("../src/handlers.rs")
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    for ty in ["ExportCsvRequest", "RestoreBackupRequest", "ImportCsvPreviewRequest"] {
        let start = src.find(&format!("struct {ty} {{")).unwrap_or_else(|| panic!("{ty} missing"));
        let body = &src[start..start + src[start..].find('}').unwrap()];
        assert!(!body.contains("path"), "SECURITY_CONCERNS.md H1: {ty} must not carry a path field");
    }
    assert!(
        !src.contains("ReadCsvHeadersRequest"),
        "SECURITY_CONCERNS.md H1: ReadCsvHeadersRequest must not exist"
    );
}

const CSRF: (&str, &str) = ("x-pwmgr-csrf", "1");

async fn post_auth(
    client: &reqwest::Client,
    base: &str,
    route: &str,
    user: &str,
    pw: &str,
) -> reqwest::Response {
    client
        .post(format!("{base}/api/{route}"))
        .header(CSRF.0, CSRF.1)
        .json(&serde_json::json!({"username": user, "masterPassword": pw}))
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn five_failed_logins_lock_the_key_out_with_429() {
    let (base, _dir) = spawn_server().await;
    let client = reqwest::Client::builder().cookie_store(true).build().unwrap();
    assert_eq!(post_auth(&client, &base, "register_user", "alice", "correct horse battery").await.status(), 200);
    client.post(format!("{base}/api/logout")).header(CSRF.0, CSRF.1).send().await.unwrap();

    for i in 0..5 {
        let res = post_auth(&client, &base, "login", "alice", "wrong password").await;
        assert_eq!(res.status(), 401, "attempt {i} should be a plain credential failure");
    }
    let res = post_auth(&client, &base, "login", "alice", "wrong password").await;
    assert_eq!(res.status(), 429);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["code"], "throttled");
}

#[tokio::test]
async fn register_user_route_is_throttled_like_login() {
    let (base, _dir) = spawn_server().await;
    let client = reqwest::Client::builder().cookie_store(true).build().unwrap();
    assert_eq!(post_auth(&client, &base, "register_user", "alice", "correct horse battery").await.status(), 200);
    // Re-registering a taken username fails (today as a 500 from the vault re-key, a pre-existing
    // quirk logged in deferred-items.md) - what matters here is only that each is a counted failure.
    for i in 0..5 {
        let s = post_auth(&client, &base, "register_user", "alice", "correct horse battery").await.status();
        assert!(s != 200 && s != 429, "attempt {i} should fail but not yet be throttled, got {s}");
    }
    let res = post_auth(&client, &base, "register_user", "alice", "correct horse battery").await;
    assert_eq!(res.status(), 429);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["code"], "throttled");
}

#[tokio::test]
async fn a_successful_login_resets_the_failure_counter() {
    let (base, _dir) = spawn_server().await;
    let client = reqwest::Client::builder().cookie_store(true).build().unwrap();
    post_auth(&client, &base, "register_user", "alice", "correct horse battery").await;
    let mut statuses = Vec::new();
    for _ in 0..4 {
        statuses.push(post_auth(&client, &base, "login", "alice", "wrong password").await.status());
    }
    statuses.push(post_auth(&client, &base, "login", "alice", "correct horse battery").await.status());
    for _ in 0..4 {
        statuses.push(post_auth(&client, &base, "login", "alice", "wrong password").await.status());
    }
    assert!(statuses.iter().all(|s| *s != 429), "success must reset both keys: {statuses:?}");
    assert_eq!(statuses[4], 200);
}

#[tokio::test]
async fn two_usernames_have_independent_throttle_counters() {
    let (base, _dir) = spawn_server().await;
    let client = reqwest::Client::builder().cookie_store(true).build().unwrap();
    for _ in 0..4 {
        assert_eq!(post_auth(&client, &base, "login", "alice", "wrong password").await.status(), 401);
    }
    // bob's own username key is untouched. (On a loopback-only server the shared IP counter is what
    // eventually catches both usernames - that is D-09 as written.)
    assert_eq!(post_auth(&client, &base, "login", "bob", "wrong password").await.status(), 401);
}

#[tokio::test]
async fn a_concurrent_burst_of_logins_cannot_exceed_the_threshold() {
    let (base, _dir) = spawn_server().await;
    let client = reqwest::Client::new();
    let (a, b, c, d, e, f) = tokio::join!(
        post_auth(&client, &base, "login", "alice", "wrong password"),
        post_auth(&client, &base, "login", "alice", "wrong password"),
        post_auth(&client, &base, "login", "alice", "wrong password"),
        post_auth(&client, &base, "login", "alice", "wrong password"),
        post_auth(&client, &base, "login", "alice", "wrong password"),
        post_auth(&client, &base, "login", "alice", "wrong password"),
    );
    let n429 = [a, b, c, d, e, f].iter().filter(|r| r.status() == 429).count();
    assert!(n429 >= 1, "six parallel attempts must hit the lockout");
}

// Multi-thread runtime on purpose: the handlers run Argon2 synchronously, so on a single-thread
// runtime requests serialize and the semaphore could never saturate (production is multi-thread).
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn a_saturated_kdf_semaphore_returns_429_rate_limited() {
    let (base, _dir) = spawn_server().await;
    let client = reqwest::Client::builder().cookie_store(true).build().unwrap();
    post_auth(&client, &base, "register_user", "alice", "correct horse battery").await;
    client.post(format!("{base}/api/logout")).header(CSRF.0, CSRF.1).send().await.unwrap();

    let n = pwmgr_web::kdf_guard::KDF_PERMITS + 3;
    let mut tasks = Vec::new();
    for _ in 0..n {
        let (c, b) = (reqwest::Client::new(), base.clone());
        tasks.push(tokio::spawn(async move {
            let res = post_auth(&c, &b, "login", "alice", "wrong password").await;
            let status = res.status().as_u16();
            let body: serde_json::Value = res.json().await.unwrap();
            (status, body["code"].as_str().unwrap_or("").to_string())
        }));
    }
    let mut seen = Vec::new();
    for t in tasks {
        seen.push(t.await.unwrap());
    }
    assert!(seen.iter().all(|(s, _)| *s < 500), "no 5xx expected: {seen:?}");
    assert!(
        seen.iter().any(|(s, c)| *s == 429 && c == "rate_limited"),
        "expected at least one rate_limited 429: {seen:?}"
    );
}
