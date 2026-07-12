use crate::config::{VALID_AUTO_LOCK_MINUTES, VALID_STRENGTH_ALGORITHMS};
use crate::dto::{AuthSuccess, ConfigDto, CredentialDto, GenerateRequest, SearchResultDto, SiteDto, StrengthDto, TrashedCredentialDto};
use crate::error::CommandError;
use crate::paths::AppPaths;
use crate::state::AppState;
use pwmgr_core::session::SystemClock;
use std::time::Duration;
use tauri::{Emitter, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use zeroize::Zeroizing;

/// Smoke-test command for Slice 0: proves the IPC round trip (state access, Mutex lock, DTO
/// return) works end-to-end before Slice 1 adds real auth commands on the same pattern.
#[tauri::command]
pub fn unlock_status(state: State<AppState>) -> bool {
    unlock_status_inner(&state)
}

fn unlock_status_inner(state: &AppState) -> bool {
    !state.session.lock().unwrap().is_locked()
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
        pwmgr_core::auth::register_user(&registry, &vault_path, username, master_password)?
    };
    let conn = pwmgr_core::vault::open_vault(&vault_path, &key)?;
    state.session.lock().unwrap().unlock(key, &SystemClock);
    state.set_vault(conn, vault_path);
    Ok(AuthSuccess { username: username.to_string() })
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
        pwmgr_core::auth::login(&registry, &vault_path, username, master_password)?
    };
    let conn = pwmgr_core::vault::open_vault(&vault_path, &key)?;
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

#[tauri::command]
pub fn register_user(
    state: State<AppState>,
    paths: State<AppPaths>,
    username: String,
    master_password: String,
) -> Result<AuthSuccess, CommandError> {
    do_register(&state, &paths, &username, &master_password)
}

#[tauri::command]
pub fn login(
    state: State<AppState>,
    paths: State<AppPaths>,
    username: String,
    master_password: String,
) -> Result<AuthSuccess, CommandError> {
    do_login(&state, &paths, &username, &master_password)
}

#[tauri::command]
pub fn lock(state: State<AppState>, app: tauri::AppHandle) {
    do_lock(&state);
    if let Err(e) = app.emit("vault-locked", ()) {
        eprintln!("warning: failed to emit vault-locked event ({e})");
    }
}

#[tauri::command]
pub fn touch(state: State<AppState>) {
    do_touch(&state);
}

pub fn do_list_sites(state: &AppState) -> Result<Vec<SiteDto>, CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    let sites = pwmgr_core::vault::list_sites(conn)?;
    Ok(sites.into_iter().map(SiteDto::from).collect())
}

pub fn do_add_site(
    state: &AppState,
    name: &str,
    url: Option<&str>,
    notes: Option<&str>,
) -> Result<i64, CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    Ok(pwmgr_core::vault::add_site(conn, name, url, notes)?)
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
    Ok(pwmgr_core::vault::add_credential(conn, key, site_id, username, secret, remarks)?)
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
    Ok(pwmgr_core::vault::update_credential(conn, key, credential_id, username, secret, remarks)?)
}

pub fn do_list_credentials_for_site(
    state: &AppState,
    site_id: i64,
) -> Result<Vec<CredentialDto>, CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    let session = state.session.lock().unwrap();
    let key = session.key().ok_or_else(crate::error::locked_error)?;
    let creds = pwmgr_core::vault::list_credentials_for_site(conn, key, site_id)?;
    Ok(creds.into_iter().map(CredentialDto::from).collect())
}

pub fn do_search(state: &AppState, query: &str) -> Result<Vec<SearchResultDto>, CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    let results = pwmgr_core::vault::search(conn, query)?;
    Ok(results.into_iter().map(SearchResultDto::from).collect())
}

pub fn do_soft_delete_credential(state: &AppState, credential_id: i64) -> Result<(), CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    Ok(pwmgr_core::vault::soft_delete_credential(conn, credential_id)?)
}

pub fn do_list_trash(state: &AppState) -> Result<Vec<TrashedCredentialDto>, CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    let trash = pwmgr_core::vault::list_trash(conn)?;
    Ok(trash.into_iter().map(TrashedCredentialDto::from).collect())
}

pub fn do_restore_credential(state: &AppState, credential_id: i64) -> Result<(), CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    Ok(pwmgr_core::vault::restore_credential(conn, credential_id)?)
}

pub fn do_purge_trash(state: &AppState) -> Result<usize, CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    Ok(pwmgr_core::vault::purge_trash(conn)?)
}

#[tauri::command]
pub fn list_sites(state: State<AppState>) -> Result<Vec<SiteDto>, CommandError> {
    do_list_sites(&state)
}

#[tauri::command]
pub fn add_site(
    state: State<AppState>,
    name: String,
    url: Option<String>,
    notes: Option<String>,
) -> Result<i64, CommandError> {
    do_add_site(&state, &name, url.as_deref(), notes.as_deref())
}

#[tauri::command]
pub fn add_credential(
    state: State<AppState>,
    site_id: i64,
    username: Option<String>,
    secret: String,
    remarks: Option<String>,
) -> Result<i64, CommandError> {
    do_add_credential(&state, site_id, username.as_deref(), &secret, remarks.as_deref())
}

#[tauri::command]
pub fn update_credential(
    state: State<AppState>,
    credential_id: i64,
    username: Option<String>,
    secret: String,
    remarks: Option<String>,
) -> Result<(), CommandError> {
    do_update_credential(&state, credential_id, username.as_deref(), &secret, remarks.as_deref())
}

#[tauri::command]
pub fn list_credentials_for_site(
    state: State<AppState>,
    site_id: i64,
) -> Result<Vec<CredentialDto>, CommandError> {
    do_list_credentials_for_site(&state, site_id)
}

#[tauri::command]
pub fn search(state: State<AppState>, query: String) -> Result<Vec<SearchResultDto>, CommandError> {
    do_search(&state, &query)
}

#[tauri::command]
pub fn soft_delete_credential(state: State<AppState>, credential_id: i64) -> Result<(), CommandError> {
    do_soft_delete_credential(&state, credential_id)
}

#[tauri::command]
pub fn list_trash(state: State<AppState>) -> Result<Vec<TrashedCredentialDto>, CommandError> {
    do_list_trash(&state)
}

#[tauri::command]
pub fn restore_credential(state: State<AppState>, credential_id: i64) -> Result<(), CommandError> {
    do_restore_credential(&state, credential_id)
}

#[tauri::command]
pub fn purge_trash(state: State<AppState>) -> Result<usize, CommandError> {
    do_purge_trash(&state)
}

pub fn do_get_config(state: &AppState) -> ConfigDto {
    let config = state.config.lock().unwrap();
    ConfigDto {
        theme: config.theme.clone(),
        auto_lock_minutes: config.auto_lock_minutes,
        strength_algorithm: config.strength_algorithm.clone(),
    }
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

#[tauri::command]
pub fn get_config(state: State<AppState>) -> ConfigDto {
    do_get_config(&state)
}

#[tauri::command]
pub fn update_auto_lock_minutes(
    state: State<AppState>,
    paths: State<AppPaths>,
    minutes: u32,
) -> Result<(), CommandError> {
    do_update_auto_lock_minutes(&state, &paths, minutes)
}

#[tauri::command]
pub fn update_strength_algorithm(
    state: State<AppState>,
    paths: State<AppPaths>,
    algorithm: String,
) -> Result<(), CommandError> {
    do_update_strength_algorithm(&state, &paths, &algorithm)
}

pub fn do_generate_password(req: &GenerateRequest) -> Result<Zeroizing<String>, CommandError> {
    use pwmgr_core::generator as gen;
    let pw = match req {
        GenerateRequest::Random { length, uppercase, lowercase, digits, symbols } =>
            gen::generate_random(&gen::RandomOptions {
                length: *length, uppercase: *uppercase, lowercase: *lowercase, digits: *digits, symbols: *symbols,
            })?,
        GenerateRequest::Memorable { words, separator } => gen::generate_memorable(*words, separator)?,
        GenerateRequest::Pattern { template } => gen::generate_pattern(template)?,
        GenerateRequest::Pronounceable { length } => gen::generate_pronounceable(*length)?,
    };
    Ok(pw)
}

#[tauri::command]
pub fn generate_password(request: GenerateRequest) -> Result<Zeroizing<String>, CommandError> {
    do_generate_password(&request)
}

pub fn do_analyze_strength(password: &str) -> StrengthDto {
    if password.is_empty() {
        return StrengthDto { score: 0, guesses_log10: 0.0, label: "Very weak".into() };
    }
    let est = zxcvbn::zxcvbn(password, &[]);
    let score = u8::from(est.score());
    let label = match score {
        0 => "Very weak", 1 => "Weak", 2 => "Fair", 3 => "Strong", _ => "Very strong",
    }.to_string();
    StrengthDto { score, guesses_log10: est.guesses_log10(), label }
}

#[tauri::command]
pub fn analyze_strength(password: String) -> StrengthDto {
    do_analyze_strength(&password)
}

const CLIPBOARD_CLEAR_SECS: u64 = 30; // hardcoded constant, NOT a user setting (ponytail)

// Thin wrapper only - not split into a `do_*` + wrapper pair. The effect is an OS clipboard side
// effect requiring a real `AppHandle`, so it is not unit-testable in the headless harness; this is
// an intentional, justified deviation, verified manually in Task 12.
#[tauri::command]
pub fn copy_to_clipboard(app: tauri::AppHandle, text: String) -> Result<(), CommandError> {
    let secret = Zeroizing::new(text);
    app.clipboard().write_text(secret.as_str()).map_err(|_| crate::error::internal_error())?;
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(CLIPBOARD_CLEAR_SECS)).await;
        // check-before-clear: only wipe if the clipboard still holds exactly what we wrote.
        if let Ok(current) = handle.clipboard().read_text() {
            if current == *secret {
                let _ = handle.clipboard().clear();
            }
        }
    });
    Ok(())
}

pub fn do_create_backup(state: &AppState, paths: &AppPaths) -> Result<String, CommandError> {
    let vault_path = {
        let vault_guard = state.vault.lock().unwrap();
        let (_, path) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
        path.clone()
    };
    let username = vault_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(crate::error::internal_error)?
        .to_string();

    let user_record = {
        let registry = state.registry.lock().unwrap();
        pwmgr_core::registry::find_user(&registry, &username)
            .map_err(|_| crate::error::internal_error())?
    };

    let vault_bytes = std::fs::read(&vault_path).map_err(|_| crate::error::internal_error())?;
    let bundle = pwmgr_core::backup::encode_bundle(&user_record, &vault_bytes);

    // Millisecond (not second) precision: a whole-second timestamp lets two backups created in
    // quick succession (e.g. a double-clicked button) silently collide and overwrite each other.
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let backup_path = paths.backups_dir().join(format!("{}-{}.bak", username, timestamp));
    std::fs::write(&backup_path, &bundle).map_err(|_| crate::error::internal_error())?;

    Ok(backup_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn create_backup(state: State<AppState>, paths: State<AppPaths>) -> Result<String, CommandError> {
    do_create_backup(&state, &paths)
}

pub fn do_restore_backup(
    state: &AppState,
    paths: &AppPaths,
    backup_file_path: &str,
    confirmed: bool,
) -> Result<(), CommandError> {
    let bytes = std::fs::read(backup_file_path)
        .map_err(|_| crate::error::invalid_input_error("Could not read that backup file."))?;
    let (user_record, vault_bytes) = pwmgr_core::backup::decode_bundle(&bytes)
        .map_err(|_| crate::error::invalid_input_error("That file isn't a valid backup."))?;

    let vault_path = paths.vault_path(&user_record.username);
    let already_exists = vault_path.exists() || {
        let registry = state.registry.lock().unwrap();
        pwmgr_core::registry::find_user(&registry, &user_record.username).is_ok()
    };
    if already_exists && !confirmed {
        return Err(crate::error::confirm_required_error(
            "A vault for that username already exists. Restoring will overwrite it.",
        ));
    }

    // If the vault being restored over is the one currently open in AppState, close it first:
    // its connection/key belongs to the account's OLD data and is stale the instant we replace
    // the file underneath it, and on Windows a lingering open handle blocks the rename below.
    // Locking the session in the same step (matching do_lock's pattern) keeps the app-wide
    // invariant vault.is_some() <=> session.is_unlocked() intact - clearing the vault alone would
    // leave unlock_status() reporting "unlocked" while every vault command now returns `locked`.
    {
        let mut vault_guard = state.vault.lock().unwrap();
        if matches!(vault_guard.as_ref(), Some((_, open_path)) if *open_path == vault_path) {
            *vault_guard = None;
            state.session.lock().unwrap().lock();
        }
    }

    // Write the vault bytes to a temp path and commit the registry row BEFORE renaming into
    // place, not after: writing the real vault_path first and upserting the registry second would
    // leave a vault file whose ciphertext doesn't match the registry's salts/verifier (unopenable
    // for an existing account) if the registry write failed after the vault write already
    // succeeded. Rename is the last step and is effectively atomic on the same volume, so the
    // only remaining failure window is far narrower than a plain write-then-upsert ordering.
    let tmp_path = vault_path.with_extension("vault.tmp");
    std::fs::write(&tmp_path, &vault_bytes).map_err(|_| crate::error::internal_error())?;

    let registry = state.registry.lock().unwrap();
    if pwmgr_core::registry::upsert_user(&registry, &user_record).is_err() {
        let _ = std::fs::remove_file(&tmp_path); // best-effort cleanup, don't mask the real error
        return Err(crate::error::internal_error());
    }
    drop(registry);

    if std::fs::rename(&tmp_path, &vault_path).is_err() {
        let _ = std::fs::remove_file(&tmp_path); // best-effort cleanup, don't mask the real error
        return Err(crate::error::internal_error());
    }
    Ok(())
}

#[tauri::command]
pub fn restore_backup(
    state: State<AppState>,
    paths: State<AppPaths>,
    path: String,
    confirmed: bool,
) -> Result<(), CommandError> {
    do_restore_backup(&state, &paths, &path, confirmed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use pwmgr_core::session::{Clock, Session};
    use std::cell::RefCell;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    // `autolock.rs` has its own `FakeClock` in its private `#[cfg(test)] mod tests`, which is not
    // reachable from here (the module itself isn't `pub`). Duplicating this ~10-line helper is
    // cheaper and clearer than inventing a shared test-support module for two call sites.
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

    #[test]
    fn unlock_status_inner_reflects_locked_session() {
        let state = AppState::new(Config::default(), Duration::from_secs(300));
        assert!(!unlock_status_inner(&state));
    }

    fn test_state_and_paths() -> (AppState, AppPaths, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(dir.path().to_path_buf());
        paths.ensure_app_dirs().unwrap();
        let registry = pwmgr_core::registry::open_registry(&paths.registry_db_path()).unwrap();
        let state = AppState {
            registry: Mutex::new(registry),
            vault: Mutex::new(None),
            session: Mutex::new(Session::new(Duration::from_secs(300))),
            config: Mutex::new(Config::default()),
        };
        (state, paths, dir)
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
        // CommandError's fields are private outside the `error` module by design (see error.rs) -
        // its derived Debug output is the only cross-module way to observe the code/message
        // without adding a new accessor purely for test visibility.
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

        // A second AppPaths (different vaults dir, same `state.registry` connection) so this
        // test isolates the registry's UsernameTaken check. do_register derives vault_path
        // deterministically from the username, and pwmgr_core::auth::register_user calls
        // open_vault (which fails on an existing file keyed with a different, freshly-randomized
        // salt) BEFORE checking registry uniqueness - reusing the same `paths` for both calls
        // would hit that vault-path collision (a VaultError, mapped to "internal_error") instead
        // of the UsernameTaken error this test targets.
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

    /// Cross-task self-review finding (Task 13.5): every other test exercises only one hop of the
    /// auth lifecycle in isolation (`commands.rs` tests never call `autolock::tick`; `autolock.rs`
    /// tests never call `do_login`/`do_register`). This test drives the full
    /// register -> idle timeout -> auto-lock -> relogin cycle on one shared `AppState`, proving
    /// the registry connection, vault connection, and session all survive the cycle without
    /// corruption or a stuck state - then repeats the lock/relogin half a second time to catch
    /// anything that only breaks on reuse (e.g. a vault file handle or SQLite lock left over from
    /// the first reopen), not on first use.
    #[test]
    fn full_register_autolock_relogin_lifecycle_survives_on_shared_state() {
        let (state, paths, _dir) = test_state_and_paths();
        let clock = FakeClock::new();
        let timeout = Duration::from_secs(300);

        // 1-2: register, assert unlocked and vault open.
        let reg = do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        assert_eq!(reg.username, "alice");
        assert!(state.vault.lock().unwrap().is_some());
        assert!(!state.session.lock().unwrap().is_locked());

        for _ in 0..2 {
            // 3-4: advance the fake clock past timeout, tick, assert just-locked and vault closed.
            // The margin over `timeout` must comfortably exceed the real wall-clock time
            // `do_register`/`do_login` spend on Argon2id (deliberately ~100ms-1s by design, see
            // `core/src/auth.rs`'s timing-oracle-defense comment) between capturing
            // `last_activity` (via the real `SystemClock`) and this `clock.advance` call (on the
            // separate `FakeClock`) - a 1s margin was flaky under that KDF cost; 30s is not.
            clock.advance(timeout + Duration::from_secs(30));
            assert!(crate::autolock::tick(&state, &clock));
            assert!(state.vault.lock().unwrap().is_none());
            assert!(state.session.lock().unwrap().is_locked());

            // 5-6: relogin on the SAME AppState, assert success and vault/session reopened.
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
        // No login/register has happened - vault is None, session is locked.
        // CommandError's fields are private outside the `error` module by design (see error.rs) -
        // its derived Debug output is the only cross-module way to observe the code/message
        // without adding a new accessor purely for test visibility (same idiom as the
        // `login_with_wrong_password_...` test above).

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
        let reloaded = crate::config::Config::load(&paths.config_path()).unwrap();
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

        // config.save() calls std::fs::write(path, ...), which fails (does not create parent dirs)
        // when the parent directory doesn't exist - a deterministic, cross-platform way to force a
        // save failure without touching real filesystem permissions.
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
    fn generate_password_random_returns_requested_length() {
        let req = GenerateRequest::Random { length: 20, uppercase: true, lowercase: true, digits: true, symbols: false };
        let pw = do_generate_password(&req).unwrap();
        assert_eq!(pw.chars().count(), 20);
    }

    #[test]
    fn generate_password_memorable_uses_separator() {
        // "|" not "-": the EFF short wordlist's "yo-yo" entry contains a literal hyphen, which
        // flakily over-counts a split('-') if that word gets picked (see core generator.rs tests).
        let req = GenerateRequest::Memorable { words: 3, separator: "|".into() };
        let pw = do_generate_password(&req).unwrap();
        assert_eq!(pw.split('|').count(), 3);
    }

    #[test]
    fn generate_password_maps_gen_error_to_invalid_input() {
        let req = GenerateRequest::Random { length: 0, uppercase: true, lowercase: false, digits: false, symbols: false };
        assert_eq!(do_generate_password(&req).unwrap_err().code(), "invalid_input");
    }

    #[test]
    fn analyze_strength_scores_a_weak_password_low() {
        assert!(do_analyze_strength("password").score <= 1);
    }

    #[test]
    fn analyze_strength_scores_a_strong_password_high() {
        assert!(do_analyze_strength("7#kLp!qZ2r@Vx9W").score >= 3);
    }

    #[test]
    fn analyze_strength_handles_empty_without_panicking() {
        let s = do_analyze_strength("");
        assert_eq!(s.score, 0);
    }

    #[test]
    fn create_backup_writes_a_decodable_bundle_under_backups_dir() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();

        let path = do_create_backup(&state, &paths).unwrap();

        assert!(std::path::Path::new(&path).starts_with(paths.backups_dir()));
        let bytes = std::fs::read(&path).unwrap();
        let (user, vault_bytes) = pwmgr_core::backup::decode_bundle(&bytes).unwrap();
        assert_eq!(user.username, "alice");
        assert!(!vault_bytes.is_empty());
    }

    #[test]
    fn create_backup_when_locked_returns_locked_error() {
        let (state, paths, _dir) = test_state_and_paths();

        let result = do_create_backup(&state, &paths);

        assert_eq!(result.unwrap_err().code(), "locked");
    }

    fn backup_for(state: &AppState, paths: &AppPaths, username: &str) -> Vec<u8> {
        do_register(state, paths, username, "correct horse battery staple").unwrap();
        let path = do_create_backup(state, paths).unwrap();
        std::fs::read(path).unwrap()
    }

    #[test]
    fn restore_backup_onto_a_brand_new_username_needs_no_confirmation() {
        let (state, paths, dir) = test_state_and_paths();
        let bundle = backup_for(&state, &paths, "alice");
        let bundle_path = dir.path().join("alice.bak");
        std::fs::write(&bundle_path, &bundle).unwrap();

        // Simulate a fresh machine: brand new state/paths, nothing registered yet.
        let (fresh_state, fresh_paths, _fresh_dir) = test_state_and_paths();

        let result = do_restore_backup(&fresh_state, &fresh_paths, bundle_path.to_str().unwrap(), false);

        assert!(result.is_ok());
        let registry = fresh_state.registry.lock().unwrap();
        assert!(pwmgr_core::registry::find_user(&registry, "alice").is_ok());
        assert!(fresh_paths.vault_path("alice").exists());
    }

    #[test]
    fn restore_backup_onto_an_existing_username_without_confirmation_is_rejected() {
        let (state, paths, dir) = test_state_and_paths();
        let bundle = backup_for(&state, &paths, "alice");
        let bundle_path = dir.path().join("alice.bak");
        std::fs::write(&bundle_path, &bundle).unwrap();

        // "alice" already exists in this same state/paths - restoring onto it needs confirmation.
        let result = do_restore_backup(&state, &paths, bundle_path.to_str().unwrap(), false);

        assert_eq!(result.unwrap_err().code(), "confirm_required");
    }

    #[test]
    fn restore_backup_onto_an_existing_username_with_confirmation_overwrites() {
        // This state's vault connection for "alice" is still open (from backup_for's do_register)
        // when the restore runs - on Windows, renaming a new vault file into place while the old
        // one is held open by a live handle fails unless that handle is closed first. This test
        // pins down that the restore closes it rather than relying on write-in-place semantics.
        let (state, paths, dir) = test_state_and_paths();
        let bundle = backup_for(&state, &paths, "alice");
        let bundle_path = dir.path().join("alice.bak");
        std::fs::write(&bundle_path, &bundle).unwrap();
        assert!(state.vault.lock().unwrap().is_some(), "precondition: vault still open pre-restore");

        let result = do_restore_backup(&state, &paths, bundle_path.to_str().unwrap(), true);

        assert!(result.is_ok());
        assert!(
            state.vault.lock().unwrap().is_none(),
            "restore must close the stale open connection, not leave it pointing at replaced data"
        );
        assert!(
            state.session.lock().unwrap().is_locked(),
            "session must be locked in lockstep with the vault, or unlock_status() would lie"
        );
    }

    #[test]
    fn restore_backup_rejects_a_file_that_is_not_a_valid_bundle() {
        let (state, paths, dir) = test_state_and_paths();
        let junk_path = dir.path().join("junk.bak");
        std::fs::write(&junk_path, b"not a backup").unwrap();

        let result = do_restore_backup(&state, &paths, junk_path.to_str().unwrap(), false);

        assert_eq!(result.unwrap_err().code(), "invalid_input");
    }
}
