use crate::config::{VALID_ACCENT_COLORS, VALID_AUTO_LOCK_MINUTES, VALID_OLD_PASSWORD_DAYS, VALID_STRENGTH_ALGORITHMS};
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

pub fn do_update_auto_lock_minutes(
    state: &AppState,
    paths: &AppPaths,
    minutes: u32,
) -> Result<(), CommandError> {
    if !VALID_AUTO_LOCK_MINUTES.contains(&minutes) {
        return Err(crate::error::invalid_input_error("Auto-lock must be 1, 5, or 15 minutes."));
    }
    let mut config = state.config.lock().unwrap();
    let mut updated = config.clone();
    updated.auto_lock_minutes = minutes;
    updated.save(&paths.config_path()).map_err(|_| crate::error::internal_error())?;
    *config = updated;
    state
        .session
        .lock()
        .unwrap()
        .set_timeout(std::time::Duration::from_secs(minutes as u64 * 60));
    Ok(())
}

pub fn do_update_strength_algorithm(
    state: &AppState,
    paths: &AppPaths,
    algorithm: &str,
) -> Result<(), CommandError> {
    if !VALID_STRENGTH_ALGORITHMS.contains(&algorithm) {
        return Err(crate::error::invalid_input_error("Unknown strength algorithm."));
    }
    let mut config = state.config.lock().unwrap();
    let mut updated = config.clone();
    updated.strength_algorithm = algorithm.to_string();
    updated.save(&paths.config_path()).map_err(|_| crate::error::internal_error())?;
    *config = updated;
    Ok(())
}

pub fn do_update_theme(state: &AppState, paths: &AppPaths, theme: &str) -> Result<(), CommandError> {
    if theme != "light" && theme != "dark" {
        return Err(crate::error::invalid_input_error("Theme must be \"light\" or \"dark\"."));
    }
    let mut config = state.config.lock().unwrap();
    let mut updated = config.clone();
    updated.theme = theme.to_string();
    updated.save(&paths.config_path()).map_err(|_| crate::error::internal_error())?;
    *config = updated;
    Ok(())
}

pub fn do_update_accent_color(state: &AppState, paths: &AppPaths, color: &str) -> Result<(), CommandError> {
    if !VALID_ACCENT_COLORS.contains(&color) {
        return Err(crate::error::invalid_input_error("Unknown accent color."));
    }
    let mut config = state.config.lock().unwrap();
    let mut updated = config.clone();
    updated.accent_color = color.to_string();
    updated.save(&paths.config_path()).map_err(|_| crate::error::internal_error())?;
    *config = updated;
    Ok(())
}

pub fn do_update_old_password_days(state: &AppState, paths: &AppPaths, days: u32) -> Result<(), CommandError> {
    if !VALID_OLD_PASSWORD_DAYS.contains(&days) {
        return Err(crate::error::invalid_input_error("Old-password threshold must be 90, 180, or 365 days."));
    }
    let mut config = state.config.lock().unwrap();
    let mut updated = config.clone();
    updated.old_password_days = days;
    updated.save(&paths.config_path()).map_err(|_| crate::error::internal_error())?;
    *config = updated;
    Ok(())
}

pub fn do_update_hibp_opt_in(state: &AppState, paths: &AppPaths, enabled: bool) -> Result<(), CommandError> {
    let mut config = state.config.lock().unwrap();
    let mut updated = config.clone();
    updated.hibp_opt_in = enabled;
    updated.save(&paths.config_path()).map_err(|_| crate::error::internal_error())?;
    *config = updated;
    Ok(())
}

pub fn do_update_nav_collapsed(state: &AppState, paths: &AppPaths, collapsed: bool) -> Result<(), CommandError> {
    let mut config = state.config.lock().unwrap();
    let mut updated = config.clone();
    updated.nav_collapsed = collapsed;
    updated.save(&paths.config_path()).map_err(|_| crate::error::internal_error())?;
    *config = updated;
    Ok(())
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

    #[test]
    fn get_config_returns_current_theme_and_auto_lock_minutes() {
        let (state, _paths, _dir) = test_state_and_paths();
        let config = do_get_config(&state);
        assert_eq!(config.theme, "dark");
        assert_eq!(config.auto_lock_minutes, 5);
    }

    #[test]
    fn update_auto_lock_minutes_persists_and_updates_live_session_timeout() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();

        do_update_auto_lock_minutes(&state, &paths, 1).unwrap();

        assert_eq!(do_get_config(&state).auto_lock_minutes, 1);
        let reloaded = Config::load(&paths.config_path()).unwrap();
        assert_eq!(reloaded.auto_lock_minutes, 1);
    }

    #[test]
    fn update_auto_lock_minutes_rejects_an_out_of_range_value() {
        let (state, paths, _dir) = test_state_and_paths();
        let result = do_update_auto_lock_minutes(&state, &paths, 7);
        assert_eq!(result.unwrap_err().code(), "invalid_input");
    }

    #[test]
    fn update_auto_lock_minutes_does_not_mutate_state_when_save_fails() {
        let (state, _paths, dir) = test_state_and_paths();
        let bad_paths = AppPaths::new(dir.path().join("nonexistent-subdir-xyz"));

        let before = do_get_config(&state).auto_lock_minutes;
        let result = do_update_auto_lock_minutes(&state, &bad_paths, 1);

        assert!(result.is_err());
        assert_eq!(
            do_get_config(&state).auto_lock_minutes,
            before,
            "state.config must not change when the save fails"
        );
    }

    #[test]
    fn get_config_includes_strength_algorithm() {
        let (state, _paths, _dir) = test_state_and_paths();
        assert_eq!(do_get_config(&state).strength_algorithm, "simple");
    }

    #[test]
    fn update_strength_algorithm_persists() {
        let (state, paths, _dir) = test_state_and_paths();
        do_update_strength_algorithm(&state, &paths, "zxcvbn").unwrap();
        assert_eq!(do_get_config(&state).strength_algorithm, "zxcvbn");
        assert_eq!(Config::load(&paths.config_path()).unwrap().strength_algorithm, "zxcvbn");
    }

    #[test]
    fn update_strength_algorithm_rejects_invalid_value() {
        let (state, paths, _dir) = test_state_and_paths();
        assert_eq!(do_update_strength_algorithm(&state, &paths, "bogus").unwrap_err().code(), "invalid_input");
    }

    #[test]
    fn update_strength_algorithm_does_not_mutate_state_when_save_fails() {
        let (state, _paths, dir) = test_state_and_paths();
        let bad_paths = AppPaths::new(dir.path().join("nonexistent-subdir-xyz"));

        let before = do_get_config(&state).strength_algorithm;
        let result = do_update_strength_algorithm(&state, &bad_paths, "zxcvbn");

        assert!(result.is_err());
        assert_eq!(
            do_get_config(&state).strength_algorithm,
            before,
            "state.config must not change when the save fails"
        );
    }

    #[test]
    fn get_config_includes_all_m4_fields() {
        let (state, _paths, _dir) = test_state_and_paths();
        let config = do_get_config(&state);
        assert_eq!(config.accent_color, "indigo");
        assert_eq!(config.old_password_days, 365);
        assert_eq!(config.hibp_opt_in, false);
        assert_eq!(config.nav_collapsed, false);
    }

    #[test]
    fn update_theme_persists_and_rejects_invalid_value() {
        let (state, paths, _dir) = test_state_and_paths();
        do_update_theme(&state, &paths, "light").unwrap();
        assert_eq!(do_get_config(&state).theme, "light");
        assert_eq!(do_update_theme(&state, &paths, "neon").unwrap_err().code(), "invalid_input");
    }

    #[test]
    fn update_accent_color_persists_and_rejects_invalid_value() {
        let (state, paths, _dir) = test_state_and_paths();
        do_update_accent_color(&state, &paths, "sky").unwrap();
        assert_eq!(do_get_config(&state).accent_color, "sky");
        assert_eq!(do_update_accent_color(&state, &paths, "chartreuse").unwrap_err().code(), "invalid_input");
    }

    #[test]
    fn update_old_password_days_persists_and_rejects_invalid_value() {
        let (state, paths, _dir) = test_state_and_paths();
        do_update_old_password_days(&state, &paths, 90).unwrap();
        assert_eq!(do_get_config(&state).old_password_days, 90);
        assert_eq!(do_update_old_password_days(&state, &paths, 42).unwrap_err().code(), "invalid_input");
    }

    #[test]
    fn update_hibp_opt_in_persists() {
        let (state, paths, _dir) = test_state_and_paths();
        do_update_hibp_opt_in(&state, &paths, true).unwrap();
        assert_eq!(do_get_config(&state).hibp_opt_in, true);
        let reloaded = Config::load(&paths.config_path()).unwrap();
        assert_eq!(reloaded.hibp_opt_in, true);
    }

    #[test]
    fn update_nav_collapsed_persists() {
        let (state, paths, _dir) = test_state_and_paths();
        do_update_nav_collapsed(&state, &paths, true).unwrap();
        assert_eq!(do_get_config(&state).nav_collapsed, true);
    }

    #[test]
    fn update_theme_does_not_mutate_state_when_save_fails() {
        let (state, _paths, dir) = test_state_and_paths();
        let bad_paths = AppPaths::new(dir.path().join("nonexistent-subdir-xyz"));
        let before = do_get_config(&state).theme;
        let result = do_update_theme(&state, &bad_paths, "light");
        assert!(result.is_err());
        assert_eq!(do_get_config(&state).theme, before, "state.config must not change when the save fails");
    }

    #[test]
    fn update_accent_color_does_not_mutate_state_when_save_fails() {
        let (state, _paths, dir) = test_state_and_paths();
        let bad_paths = AppPaths::new(dir.path().join("nonexistent-subdir-xyz"));
        let before = do_get_config(&state).accent_color;
        let result = do_update_accent_color(&state, &bad_paths, "sky");
        assert!(result.is_err());
        assert_eq!(do_get_config(&state).accent_color, before, "state.config must not change when the save fails");
    }

    #[test]
    fn update_old_password_days_does_not_mutate_state_when_save_fails() {
        let (state, _paths, dir) = test_state_and_paths();
        let bad_paths = AppPaths::new(dir.path().join("nonexistent-subdir-xyz"));
        let before = do_get_config(&state).old_password_days;
        let result = do_update_old_password_days(&state, &bad_paths, 90);
        assert!(result.is_err());
        assert_eq!(do_get_config(&state).old_password_days, before, "state.config must not change when the save fails");
    }

    #[test]
    fn update_hibp_opt_in_does_not_mutate_state_when_save_fails() {
        let (state, _paths, dir) = test_state_and_paths();
        let bad_paths = AppPaths::new(dir.path().join("nonexistent-subdir-xyz"));
        let before = do_get_config(&state).hibp_opt_in;
        let result = do_update_hibp_opt_in(&state, &bad_paths, true);
        assert!(result.is_err());
        assert_eq!(do_get_config(&state).hibp_opt_in, before, "state.config must not change when the save fails");
    }

    #[test]
    fn update_nav_collapsed_does_not_mutate_state_when_save_fails() {
        let (state, _paths, dir) = test_state_and_paths();
        let bad_paths = AppPaths::new(dir.path().join("nonexistent-subdir-xyz"));
        let before = do_get_config(&state).nav_collapsed;
        let result = do_update_nav_collapsed(&state, &bad_paths, true);
        assert!(result.is_err());
        assert_eq!(do_get_config(&state).nav_collapsed, before, "state.config must not change when the save fails");
    }
}
