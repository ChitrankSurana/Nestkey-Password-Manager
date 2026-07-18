use crate::dto::{AuthSuccess, ConfigDto, CredentialDto, SearchResultDto, SiteDto, TrashedCredentialDto};
use crate::error::CommandError;
use crate::paths::AppPaths;
use crate::session::SystemClock;
use crate::state::AppState;

pub fn do_unlock_status(state: &AppState) -> bool {
    !state.session.lock().unwrap().is_locked()
}

pub fn do_login(
    state: &AppState,
    paths: &AppPaths,
    username: &str,
    master_password: &str,
) -> Result<AuthSuccess, CommandError> {
    let vault_path = paths.vault_path(username);
    let key = {
        let registry = state.registry.lock().unwrap();
        crate::auth::login(&registry, &vault_path, username, master_password)?
    };
    let conn = crate::vault::open_vault(&vault_path, &key)?;
    state.session.lock().unwrap().unlock(key, &SystemClock);
    state.set_vault(conn, vault_path);
    Ok(AuthSuccess { username: username.to_string() })
}

pub fn do_lock(state: &AppState) {
    state.session.lock().unwrap().lock();
    state.clear_vault();
}

pub fn do_touch(state: &AppState) {
    state.session.lock().unwrap().touch(&SystemClock);
}

pub fn do_list_sites(state: &AppState) -> Result<Vec<SiteDto>, CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    let sites = crate::vault::list_sites(conn)?;
    Ok(sites.into_iter().map(SiteDto::from).collect())
}

pub fn do_list_credentials_for_site(
    state: &AppState,
    site_id: i64,
) -> Result<Vec<CredentialDto>, CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    let session = state.session.lock().unwrap();
    let key = session.key().ok_or_else(crate::error::locked_error)?;
    let creds = crate::vault::list_credentials_for_site(conn, key, site_id)?;
    Ok(creds.into_iter().map(CredentialDto::from).collect())
}

pub fn do_search(state: &AppState, query: &str) -> Result<Vec<SearchResultDto>, CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    let results = crate::vault::search(conn, query)?;
    Ok(results.into_iter().map(SearchResultDto::from).collect())
}

pub fn do_get_config(state: &AppState) -> ConfigDto {
    let config = state.config.lock().unwrap();
    ConfigDto {
        theme: config.theme.clone(),
        auto_lock_minutes: config.auto_lock_minutes,
        strength_algorithm: config.strength_algorithm.clone(),
        accent_color: config.accent_color.clone(),
        old_password_days: config.old_password_days,
        hibp_opt_in: config.hibp_opt_in,
        nav_collapsed: config.nav_collapsed,
    }
}

pub fn do_register(
    state: &AppState,
    paths: &AppPaths,
    username: &str,
    master_password: &str,
) -> Result<AuthSuccess, CommandError> {
    let vault_path = paths.vault_path(username);
    let key = {
        let registry = state.registry.lock().unwrap();
        crate::auth::register_user(&registry, &vault_path, username, master_password)?
    };
    let conn = crate::vault::open_vault(&vault_path, &key)?;
    state.session.lock().unwrap().unlock(key, &SystemClock);
    state.set_vault(conn, vault_path);
    Ok(AuthSuccess { username: username.to_string() })
}

pub fn do_add_site(
    state: &AppState,
    name: &str,
    url: Option<&str>,
    notes: Option<&str>,
) -> Result<i64, CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    Ok(crate::vault::add_site(conn, name, url, notes)?)
}

pub fn do_add_credential(
    state: &AppState,
    site_id: i64,
    username: Option<&str>,
    secret: &str,
    remarks: Option<&str>,
) -> Result<i64, CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    let session = state.session.lock().unwrap();
    let key = session.key().ok_or_else(crate::error::locked_error)?;
    Ok(crate::vault::add_credential(conn, key, site_id, username, secret, remarks)?)
}

pub fn do_update_credential(
    state: &AppState,
    credential_id: i64,
    username: Option<&str>,
    secret: &str,
    remarks: Option<&str>,
) -> Result<(), CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    let session = state.session.lock().unwrap();
    let key = session.key().ok_or_else(crate::error::locked_error)?;
    Ok(crate::vault::update_credential(conn, key, credential_id, username, secret, remarks)?)
}

pub fn do_soft_delete_credential(state: &AppState, credential_id: i64) -> Result<(), CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    Ok(crate::vault::soft_delete_credential(conn, credential_id)?)
}

pub fn do_list_trash(state: &AppState) -> Result<Vec<TrashedCredentialDto>, CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    let trash = crate::vault::list_trash(conn)?;
    Ok(trash.into_iter().map(TrashedCredentialDto::from).collect())
}

pub fn do_restore_credential(state: &AppState, credential_id: i64) -> Result<(), CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    Ok(crate::vault::restore_credential(conn, credential_id)?)
}

pub fn do_purge_trash(state: &AppState) -> Result<usize, CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    Ok(crate::vault::purge_trash(conn)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::session::Clock;
    use std::cell::RefCell;
    use std::sync::Mutex;
    use std::time::Duration;
    use std::time::Instant;

    struct FakeClock(RefCell<Instant>);
    impl FakeClock {
        fn new() -> Self {
            FakeClock(RefCell::new(Instant::now()))
        }
        fn advance(&self, d: Duration) {
            *self.0.borrow_mut() += d;
        }
    }
    impl Clock for FakeClock {
        fn now(&self) -> Instant {
            *self.0.borrow()
        }
    }

    fn test_state_and_paths() -> (AppState, AppPaths, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(dir.path().to_path_buf());
        paths.ensure_app_dirs().unwrap();
        let registry = crate::registry::open_registry(&paths.registry_db_path()).unwrap();
        let state = AppState {
            registry: Mutex::new(registry),
            vault: Mutex::new(None),
            session: Mutex::new(crate::session::Session::new(Duration::from_secs(300))),
            config: Mutex::new(Config::default()),
        };
        (state, paths, dir)
    }

    // `crate::auth::register_user` only writes the vault file + registry row and returns the
    // derived key - it does NOT touch `state.session`/`state.vault` (only the higher-level
    // `do_register`, which stays in src-tauri and isn't promoted here, does that). So calling it
    // directly leaves the test's AppState untouched (still locked, no vault open) - exactly the
    // "arrange" step needed before each test's `do_login` call is the thing actually under test.
    fn register_test_user(state: &AppState, paths: &AppPaths, username: &str, master_password: &str) {
        let vault_path = paths.vault_path(username);
        let registry = state.registry.lock().unwrap();
        crate::auth::register_user(&registry, &vault_path, username, master_password).unwrap();
    }

    #[test]
    fn do_unlock_status_reflects_locked_session() {
        let (state, _paths, _dir) = test_state_and_paths();
        assert!(!do_unlock_status(&state));
    }

    #[test]
    fn do_login_unlocks_the_session_and_opens_the_vault() {
        let (state, paths, _dir) = test_state_and_paths();
        register_test_user(&state, &paths, "alice", "correct horse battery staple");
        let result = do_login(&state, &paths, "alice", "correct horse battery staple").unwrap();
        assert_eq!(result.username, "alice");
        assert!(do_unlock_status(&state));
    }

    #[test]
    fn do_login_with_wrong_password_returns_single_generic_error_and_stays_locked() {
        let (state, paths, _dir) = test_state_and_paths();
        register_test_user(&state, &paths, "alice", "correct horse battery staple");
        let result = do_login(&state, &paths, "alice", "wrong password");
        assert_eq!(result.unwrap_err().code(), "invalid_credentials");
        assert!(!do_unlock_status(&state));
    }

    #[test]
    fn do_lock_locks_the_session_and_clears_the_vault() {
        let (state, paths, _dir) = test_state_and_paths();
        register_test_user(&state, &paths, "alice", "correct horse battery staple");
        do_login(&state, &paths, "alice", "correct horse battery staple").unwrap();
        do_lock(&state);
        assert!(!do_unlock_status(&state));
        assert!(state.vault.lock().unwrap().is_none());
    }

    #[test]
    fn do_touch_does_not_lock_the_session() {
        let (state, paths, _dir) = test_state_and_paths();
        register_test_user(&state, &paths, "alice", "correct horse battery staple");
        do_login(&state, &paths, "alice", "correct horse battery staple").unwrap();
        do_touch(&state);
        assert!(do_unlock_status(&state));
    }

    #[test]
    fn do_list_sites_requires_an_unlocked_session() {
        let (state, _paths, _dir) = test_state_and_paths();
        assert_eq!(do_list_sites(&state).unwrap_err().code(), "locked");
    }

    #[test]
    fn do_list_sites_and_do_list_credentials_for_site_roundtrip_through_dtos() {
        let (state, paths, _dir) = test_state_and_paths();
        register_test_user(&state, &paths, "alice", "correct horse battery staple");
        do_login(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let site_id = {
            let vault_guard = state.vault.lock().unwrap();
            let (conn, _) = vault_guard.as_ref().unwrap();
            let session = state.session.lock().unwrap();
            let key = session.key().unwrap();
            let site_id = crate::vault::add_site(conn, "example.com", None, None).unwrap();
            crate::vault::add_credential(conn, key, site_id, Some("alice"), "s3cret", None).unwrap();
            site_id
        };
        let sites = do_list_sites(&state).unwrap();
        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].name, "example.com");
        let creds = do_list_credentials_for_site(&state, site_id).unwrap();
        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0].username.as_deref(), Some("alice"));
        assert_eq!(creds[0].secret.as_str(), "s3cret");
    }

    #[test]
    fn do_search_requires_an_unlocked_session() {
        let (state, _paths, _dir) = test_state_and_paths();
        assert_eq!(do_search(&state, "example").unwrap_err().code(), "locked");
    }

    #[test]
    fn do_search_finds_a_credential_by_site_name() {
        let (state, paths, _dir) = test_state_and_paths();
        register_test_user(&state, &paths, "alice", "correct horse battery staple");
        do_login(&state, &paths, "alice", "correct horse battery staple").unwrap();
        {
            let vault_guard = state.vault.lock().unwrap();
            let (conn, _) = vault_guard.as_ref().unwrap();
            let session = state.session.lock().unwrap();
            let key = session.key().unwrap();
            let site_id = crate::vault::add_site(conn, "example.com", None, None).unwrap();
            crate::vault::add_credential(conn, key, site_id, Some("alice"), "secret", None).unwrap();
        }
        let results = do_search(&state, "example").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].username.as_deref(), Some("alice"));
    }

    #[test]
    fn do_get_config_returns_current_theme_and_all_fields() {
        let (state, _paths, _dir) = test_state_and_paths();
        let config = do_get_config(&state);
        assert_eq!(config.theme, "dark");
        assert_eq!(config.accent_color, "indigo");
        assert_eq!(config.old_password_days, 365);
        assert_eq!(config.hibp_opt_in, false);
        assert_eq!(config.nav_collapsed, false);
    }

    #[test]
    fn unlock_status_inner_reflects_locked_session() {
        let (state, _paths, _dir) = test_state_and_paths();
        assert!(!do_unlock_status(&state));
    }

    #[test]
    fn register_then_login_both_unlock_the_session_and_open_the_vault() {
        let (state, paths, _dir) = test_state_and_paths();

        let reg = do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        assert_eq!(reg.username, "alice");
        assert!(!state.session.lock().unwrap().is_locked());
        assert!(state.vault.lock().unwrap().is_some());

        do_lock(&state);
        assert!(state.session.lock().unwrap().is_locked());
        assert!(state.vault.lock().unwrap().is_none());

        let login = do_login(&state, &paths, "alice", "correct horse battery staple").unwrap();
        assert_eq!(login.username, "alice");
        assert!(!state.session.lock().unwrap().is_locked());
        assert!(state.vault.lock().unwrap().is_some());
    }

    #[test]
    fn login_with_wrong_password_returns_single_generic_error_and_stays_locked() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        do_lock(&state);

        let result = do_login(&state, &paths, "alice", "wrong password");

        assert!(result.is_err());
        let debug = format!("{:?}", result.unwrap_err());
        assert!(debug.contains("invalid_credentials"));
        assert!(state.session.lock().unwrap().is_locked());
        assert!(state.vault.lock().unwrap().is_none());
    }

    #[test]
    fn register_duplicate_username_returns_username_taken_error() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "password one").unwrap();
        do_lock(&state);

        let dir2 = tempfile::tempdir().unwrap();
        let paths2 = AppPaths::new(dir2.path().to_path_buf());
        paths2.ensure_app_dirs().unwrap();

        let result = do_register(&state, &paths2, "alice", "password two");

        let debug = format!("{:?}", result.unwrap_err());
        assert!(debug.contains("username_taken"));
    }

    #[test]
    fn touch_does_not_lock_the_session() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();

        do_touch(&state);

        assert!(!state.session.lock().unwrap().is_locked());
    }

    #[test]
    fn full_register_autolock_relogin_lifecycle_survives_on_shared_state() {
        let (state, paths, _dir) = test_state_and_paths();
        let clock = FakeClock::new();
        let timeout = Duration::from_secs(300);

        let reg = do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        assert_eq!(reg.username, "alice");
        assert!(state.vault.lock().unwrap().is_some());
        assert!(!state.session.lock().unwrap().is_locked());

        for _ in 0..2 {
            clock.advance(timeout + Duration::from_secs(30));
            assert!(crate::autolock::tick(&state, &clock));
            assert!(state.vault.lock().unwrap().is_none());
            assert!(state.session.lock().unwrap().is_locked());

            let login =
                do_login(&state, &paths, "alice", "correct horse battery staple").unwrap();
            assert_eq!(login.username, "alice");
            assert!(state.vault.lock().unwrap().is_some());
            assert!(!state.session.lock().unwrap().is_locked());
        }
    }

    #[test]
    fn vault_commands_require_an_unlocked_session() {
        let (state, _paths, _dir) = test_state_and_paths();
        assert!(format!("{:?}", do_list_sites(&state).unwrap_err()).contains("locked"));
        assert!(
            format!("{:?}", do_add_site(&state, "x", None, None).unwrap_err()).contains("locked")
        );
        assert!(format!("{:?}", do_search(&state, "x").unwrap_err()).contains("locked"));
    }

    #[test]
    fn add_site_then_list_sites_roundtrips_through_dtos() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();

        let site_id = do_add_site(&state, "example.com", Some("https://example.com"), None).unwrap();
        let sites = do_list_sites(&state).unwrap();

        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].id, site_id);
        assert_eq!(sites[0].name, "example.com");
    }

    #[test]
    fn add_credential_update_then_list_reflects_the_update() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let site_id = do_add_site(&state, "example.com", None, None).unwrap();

        let cred_id = do_add_credential(&state, site_id, Some("bob"), "s3cret", None).unwrap();
        do_update_credential(&state, cred_id, Some("bob2"), "new-secret", Some("updated")).unwrap();
        let creds = do_list_credentials_for_site(&state, site_id).unwrap();

        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0].username.as_deref(), Some("bob2"));
        assert_eq!(creds[0].secret.as_str(), "new-secret");
    }

    #[test]
    fn search_and_soft_delete_credential_work_through_dtos() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let site_id = do_add_site(&state, "GitHub", None, None).unwrap();
        let cred_id = do_add_credential(&state, site_id, Some("alice"), "s3cret", None).unwrap();

        assert_eq!(do_search(&state, "github").unwrap().len(), 1);

        do_soft_delete_credential(&state, cred_id).unwrap();

        assert!(do_search(&state, "github").unwrap().is_empty());
    }

    #[test]
    fn trash_lifecycle_list_restore_purge() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let site_id = do_add_site(&state, "example.com", None, None).unwrap();
        let cred_id = do_add_credential(&state, site_id, Some("bob"), "s3cret", None).unwrap();

        do_soft_delete_credential(&state, cred_id).unwrap();
        let trash = do_list_trash(&state).unwrap();
        assert_eq!(trash.len(), 1);
        assert_eq!(trash[0].credential_id, cred_id);

        do_restore_credential(&state, cred_id).unwrap();
        assert!(do_list_trash(&state).unwrap().is_empty());
        assert_eq!(do_list_credentials_for_site(&state, site_id).unwrap().len(), 1);

        do_soft_delete_credential(&state, cred_id).unwrap();
        let purged = do_purge_trash(&state).unwrap();
        assert_eq!(purged, 1);
        assert!(do_list_trash(&state).unwrap().is_empty());
    }
}
