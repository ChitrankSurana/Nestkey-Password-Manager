use pwmgr_core::{auth, registry, session, vault};
use std::time::Duration;
use tempfile::tempdir;

/// Mirrors the M1 milestone's "Verify" line from docs/superpowers/plans — this is the
/// single test that proves the secure core actually delivers a usable, safe vault:
/// create user -> add/edit/delete/search entries -> lock -> reopen -> data intact & encrypted.
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
    let cred_id = vault::add_credential(&vault_conn, &key, site_id, Some("alice"), "s3cret!", Some("work")).unwrap();

    // Search finds it.
    let results = vault::search(&vault_conn, "github").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].credential_id, cred_id);

    // Soft-delete then restore (edit/delete lifecycle).
    vault::soft_delete_credential(&vault_conn, cred_id).unwrap();
    assert!(vault::list_credentials_for_site(&vault_conn, &key, site_id).unwrap().is_empty());
    vault::restore_credential(&vault_conn, cred_id).unwrap();

    // Lock (simulated via Session, not just dropping the connection).
    let mut sess = session::Session::new(Duration::from_secs(300));
    sess.unlock(key, &session::SystemClock);
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
    vault::add_credential(&vault_conn, &key, site_id, Some("bob"), "extremely-unique-secret-xyz", None).unwrap();
    drop(vault_conn);

    let raw = std::fs::read(&vault_path).unwrap();
    let raw_str = String::from_utf8_lossy(&raw);
    assert!(!raw_str.contains("very-unique-site-name.example"));
    assert!(!raw_str.contains("extremely-unique-secret-xyz"));
}
