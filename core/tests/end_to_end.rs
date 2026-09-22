use pwmgr_core::{auth, registry, session, vault};
use std::time::Duration;
use tempfile::tempdir;

/// Mirrors the M1 milestone's "Verify" line from docs/superpowers/plans — this is the
/// single test that proves the secure core actually delivers a usable, safe vault:
/// create user -> add/soft-delete/restore/search entries -> lock -> reopen -> data intact & encrypted.
#[test]
fn full_lifecycle_register_use_lock_reopen() {
    let dir = tempdir().unwrap();
    let registry_path = dir.path().join("registry.db");
    let vault_path = dir.path().join("alice.vault");

    let registry_conn = registry::open_registry(&registry_path).unwrap();

    // Register.
    let key = auth::register_user(&registry_conn, &vault_path, "alice", "correct horse battery staple").unwrap();
    let vault_conn = vault::open_vault(&vault_path, &key).unwrap();

    // Add.
    let site_id = vault::add_site(&vault_conn, "github.com", Some("https://github.com"), None).unwrap();
    let cred_id = vault::add_credential(&vault_conn, &key, site_id, Some("alice"), "s3cret!", Some("work"), &[], false).unwrap();

    // Search finds it.
    let results = vault::search(&vault_conn, &key, "github").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].credential_id, cred_id);

    // Soft-delete then restore (delete/restore lifecycle).
    vault::soft_delete_credential(&vault_conn, cred_id).unwrap();
    assert!(vault::list_credentials_for_site(&vault_conn, &key, site_id).unwrap().is_empty());
    vault::restore_credential(&vault_conn, cred_id).unwrap();

    // Lock (simulated via Session, not just dropping the connection).
    let mut sess = session::Session::new(Duration::from_secs(300));
    sess.unlock(key, "alice", &session::SystemClock);
    drop(vault_conn);
    sess.lock();
    assert!(sess.is_locked());

    // Reopen: wrong password must fail, correct password must succeed with data intact.
    let wrong = auth::login(&registry_conn, &vault_path, "alice", "not the password");
    assert!(wrong.is_err());

    let reopened_key = auth::login(&registry_conn, &vault_path, "alice", "correct horse battery staple").unwrap();
    let reopened_conn = vault::open_vault(&vault_path, &reopened_key).unwrap();
    let creds = vault::list_credentials_for_site(&reopened_conn, &reopened_key, site_id).unwrap();

    assert_eq!(creds.len(), 1);
    assert_eq!(creds[0].secret, "s3cret!");
    assert_eq!(creds[0].username.as_deref(), Some("alice"));
}

/// Proves the on-disk file is not readable without the key: raw bytes must not contain
/// the plaintext secret or site name.
#[test]
fn vault_file_on_disk_contains_no_plaintext_secrets() {
    let dir = tempdir().unwrap();
    let registry_path = dir.path().join("registry.db");
    let vault_path = dir.path().join("bob.vault");
    let registry_conn = registry::open_registry(&registry_path).unwrap();

    let key = auth::register_user(&registry_conn, &vault_path, "bob", "another master password").unwrap();
    let vault_conn = vault::open_vault(&vault_path, &key).unwrap();
    let site_id = vault::add_site(&vault_conn, "very-unique-site-name.example", None, None).unwrap();
    vault::add_credential(&vault_conn, &key, site_id, Some("bob"), "extremely-unique-secret-xyz", None, &[], false).unwrap();

    // Positive control: confirm the write actually round-trips BEFORE trusting the absence check
    // below. Without this, a silently-failed or wrong-table write would still correctly show no
    // plaintext on disk and this test would pass for the wrong reason.
    let creds = vault::list_credentials_for_site(&vault_conn, &key, site_id).unwrap();
    assert_eq!(creds.len(), 1);
    assert_eq!(creds[0].secret, "extremely-unique-secret-xyz");

    drop(vault_conn);

    // No journal_mode/synchronous pragma is set anywhere in this crate, so SQLite/SQLCipher use
    // the legacy rollback-journal default: each add_* call above commits straight to the main db
    // file, not to a WAL sidecar. That's why dropping the connection (rather than an explicit
    // checkpoint) is sufficient here. If this crate ever switches to WAL mode for performance,
    // this raw-byte read would need to account for data possibly still living in a `-wal` file.
    let raw = std::fs::read(&vault_path).unwrap();
    let contains_bytes = |haystack: &[u8], needle: &[u8]| haystack.windows(needle.len()).any(|w| w == needle);
    assert!(!contains_bytes(&raw, b"very-unique-site-name.example"));
    assert!(!contains_bytes(&raw, b"extremely-unique-secret-xyz"));
}
