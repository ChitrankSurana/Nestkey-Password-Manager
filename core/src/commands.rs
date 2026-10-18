use crate::config::{VALID_ACCENT_COLORS, VALID_AUTO_LOCK_MINUTES, VALID_MOBILE_NAV_STYLES, VALID_OLD_PASSWORD_DAYS, VALID_REPROMPT_GRACE_SECONDS, VALID_REPROMPT_MODES, VALID_STRENGTH_ALGORITHMS, VALID_UI_VARIANTS};
use crate::dto::{AuthSuccess, BreachReportDto, ChangeLogEntryDto, ConfigDto, CredentialDto, ExportMode, FolderDto, GenerateRequest, HealthEntryDto, HealthReportDto, HistoryEntryDto, ImportFormat, ImportRecordDto, ImportSummaryDto, ReusedGroupDto, SearchResultDto, SiteDto, StrengthDto, TotpCodeDto, TrashedCredentialDto};
use crate::error::CommandError;
use crate::paths::AppPaths;
use crate::session::SystemClock;
use crate::state::AppState;
use zeroize::Zeroizing;

pub fn do_unlock_status(state: &AppState) -> bool {
    !state.session.lock().unwrap().is_locked()
}

pub fn do_login(
    state: &AppState,
    paths: &AppPaths,
    username: &str,
    master_password: &str,
) -> Result<AuthSuccess, CommandError> {
    let vault_path = paths.checked_vault_path(username)?;
    let key = {
        let registry = state.registry.lock().unwrap();
        crate::auth::login(&registry, &vault_path, username, master_password)?
    };
    let conn = crate::vault::open_vault(&vault_path, &key)?;
    state.session.lock().unwrap().unlock(key, username, &SystemClock);
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

/// Re-auth gate for a per-item "require master password" (reprompt) field - the vault stays
/// unlocked either way, this just answers "did they type the right master password right now."
pub fn do_verify_master_password(state: &AppState, master_password: &str) -> Result<bool, CommandError> {
    let username = {
        let session = state.session.lock().unwrap();
        session.username().ok_or_else(crate::error::locked_error)?.to_string()
    };
    let registry = state.registry.lock().unwrap();
    Ok(crate::auth::verify_master_password(&registry, &username, master_password)?)
}

/// Whether `username` has PIN unlock configured - callable while locked (the login screen needs
/// this before offering a "Unlock with PIN" option), so it only touches the registry, never
/// `state.session`/`state.vault`.
pub fn do_has_pin(state: &AppState, username: &str) -> Result<bool, CommandError> {
    let registry = state.registry.lock().unwrap();
    Ok(crate::auth::has_pin(&registry, username)?)
}

/// N1: whether ANY account has ever been registered on this machine - callable while locked (the
/// Login/Register choice happens before any vault is open), so this only touches the registry.
/// `registry::list_usernames` (used only here until now) has no per-user filter because there is
/// none to apply: this is a single yes/no over the whole registry, not a lookup for one username.
pub fn do_vault_exists(state: &AppState) -> Result<bool, CommandError> {
    let registry = state.registry.lock().unwrap();
    Ok(!crate::registry::list_usernames(&registry)?.is_empty())
}

// Holds `session` while acquiring `registry` (needs the live vault key to seal, and the registry
// connection to persist the result, at the same time) - the one place in this file that nests
// those two locks. Safe: every other function touching both locks them sequentially (one dropped
// before the other is acquired, see do_login/do_verify_master_password/do_clear_pin), so no path
// exists that holds `registry` first and then waits on `session`, which is what would be needed
// for this order to deadlock against it.
pub fn do_set_pin(state: &AppState, pin: &str) -> Result<(), CommandError> {
    let session = state.session.lock().unwrap();
    let username = session.username().ok_or_else(crate::error::locked_error)?;
    let key = session.key().ok_or_else(crate::error::locked_error)?;
    let registry = state.registry.lock().unwrap();
    Ok(crate::auth::set_pin(&registry, username, pin, key)?)
}

pub fn do_clear_pin(state: &AppState) -> Result<(), CommandError> {
    let username = {
        let session = state.session.lock().unwrap();
        session.username().ok_or_else(crate::error::locked_error)?.to_string()
    };
    let registry = state.registry.lock().unwrap();
    Ok(crate::auth::clear_pin(&registry, &username)?)
}

pub fn do_unlock_with_pin(
    state: &AppState,
    paths: &AppPaths,
    username: &str,
    pin: &str,
) -> Result<AuthSuccess, CommandError> {
    let vault_path = paths.checked_vault_path(username)?;
    let key = {
        let registry = state.registry.lock().unwrap();
        crate::auth::unlock_with_pin(&registry, username, pin)?
    };
    let conn = crate::vault::open_vault(&vault_path, &key)?;
    state.session.lock().unwrap().unlock(key, username, &SystemClock);
    state.set_vault(conn, vault_path);
    Ok(AuthSuccess { username: username.to_string() })
}

/// Whether `username` has biometric unlock configured - callable while locked (the login screen
/// needs this before offering the option), and gated on the config toggle: a disabled feature
/// never reports as available even if a record happens to still exist from before it was turned
/// off.
pub fn do_has_biometric(state: &AppState, username: &str, provider: &dyn crate::biometric::BiometricProvider) -> bool {
    if !state.config.lock().unwrap().biometric_enabled || !provider.is_supported() {
        return false;
    }
    let registry = state.registry.lock().unwrap();
    crate::registry::find_biometric(&registry, username).ok().flatten().is_some()
}

pub fn do_set_biometric(
    state: &AppState,
    provider: &dyn crate::biometric::BiometricProvider,
) -> Result<(), CommandError> {
    if !state.config.lock().unwrap().biometric_enabled {
        return Err(crate::error::invalid_input_error("Enable biometric unlock in Settings first."));
    }
    let session = state.session.lock().unwrap();
    let username = session.username().ok_or_else(crate::error::locked_error)?;
    let key = session.key().ok_or_else(crate::error::locked_error)?;
    if !provider.verify()? {
        return Err(crate::biometric::BiometricError::VerificationFailed.into());
    }
    let wrapped_key = provider.protect(key.as_bytes())?;
    let registry = state.registry.lock().unwrap();
    crate::registry::set_biometric(&registry, &crate::registry::BiometricRecord {
        username: username.to_string(),
        wrapped_key,
    })?;
    Ok(())
}

pub fn do_clear_biometric(state: &AppState) -> Result<(), CommandError> {
    let username = {
        let session = state.session.lock().unwrap();
        session.username().ok_or_else(crate::error::locked_error)?.to_string()
    };
    let registry = state.registry.lock().unwrap();
    Ok(crate::registry::clear_biometric(&registry, &username)?)
}

pub fn do_unlock_with_biometric(
    state: &AppState,
    paths: &AppPaths,
    username: &str,
    provider: &dyn crate::biometric::BiometricProvider,
) -> Result<AuthSuccess, CommandError> {
    if !state.config.lock().unwrap().biometric_enabled {
        return Err(crate::error::invalid_input_error("Biometric unlock is disabled in Settings."));
    }
    if !provider.is_supported() {
        return Err(crate::biometric::BiometricError::Unsupported.into());
    }
    let wrapped_key = {
        let registry = state.registry.lock().unwrap();
        crate::registry::find_biometric(&registry, username)?
            .ok_or(crate::biometric::BiometricError::VerificationFailed)?
            .wrapped_key
    };
    if !provider.verify()? {
        return Err(crate::biometric::BiometricError::VerificationFailed.into());
    }
    let raw = zeroize::Zeroizing::new(provider.unprotect(&wrapped_key)?);
    if raw.len() != 32 {
        return Err(crate::biometric::BiometricError::StorageFailed.into());
    }
    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&raw);
    let key = crate::crypto::VaultKey::from_bytes(key_bytes);

    let vault_path = paths.checked_vault_path(username)?;
    let conn = crate::vault::open_vault(&vault_path, &key)?;
    state.session.lock().unwrap().unlock(key, username, &SystemClock);
    state.set_vault(conn, vault_path);
    Ok(AuthSuccess { username: username.to_string() })
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
    let session = state.session.lock().unwrap();
    let key = session.key().ok_or_else(crate::error::locked_error)?;
    let results = crate::vault::search(conn, key, query)?;
    Ok(results.into_iter().map(SearchResultDto::from).collect())
}

pub fn do_list_credential_history(
    state: &AppState,
    credential_id: i64,
) -> Result<Vec<HistoryEntryDto>, CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    let session = state.session.lock().unwrap();
    let key = session.key().ok_or_else(crate::error::locked_error)?;
    let history = crate::vault::list_credential_history(conn, key, credential_id)?;
    Ok(history.into_iter().map(HistoryEntryDto::from).collect())
}

/// Wave 4 follow-up (M7.2): the per-credential change log, separate from password history above -
/// no key needed since these rows are plain TEXT, not sealed (see `log_field_change`'s doc comment
/// on why: never secrets or custom fields, just username/remarks/reprompt-exempt).
pub fn do_list_credential_change_log(
    state: &AppState,
    credential_id: i64,
) -> Result<Vec<ChangeLogEntryDto>, CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    let log = crate::vault::list_credential_change_log(conn, credential_id)?;
    Ok(log.into_iter().map(ChangeLogEntryDto::from).collect())
}

/// B1-6, the orchestrator: three sequential, non-overlapping lock windows (registry, then vault,
/// then registry again), matching the "never hold `vault` and `registry` at once" discipline every
/// other function in this file already follows (see `do_set_pin`'s own comment on its one
/// deliberate exception, which this is not - this function never holds two of these locks
/// simultaneously). See `auth::PasswordChangeMaterial`, `vault::reseal_all_blobs` and
/// `vault::rekey_vault`'s doc comments for why this exact ordering and the pre-flight backup are
/// both load-bearing, not defensive extras.
pub fn do_change_master_password(
    state: &AppState,
    paths: &AppPaths,
    current_password: &str,
    new_password: &str,
) -> Result<(), CommandError> {
    if new_password.is_empty() {
        return Err(crate::error::invalid_input_error("Choose a new master password."));
    }

    let username = {
        let session = state.session.lock().unwrap();
        session.username().ok_or_else(crate::error::locked_error)?.to_string()
    };

    // Step 1 (registry only): verify current_password, derive old/new keys and the new registry
    // record. No vault access yet.
    let material = {
        let registry = state.registry.lock().unwrap();
        crate::auth::prepare_password_change(&registry, &username, current_password, new_password)?
    };

    // Pre-flight backup - same bundle format the Data Hub's own backup command writes - taken
    // BEFORE any mutation. The one recovery path if step 2 fails partway: restore this file and
    // the vault is fully consistent again under the OLD password (registry hasn't changed yet).
    let (backup_username, bundle) = build_backup_bundle(state)?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let backup_path = paths.backups_dir().join(format!("{}-premasterchange-{}.bak", backup_username, timestamp));
    std::fs::write(&backup_path, &bundle).map_err(|_| crate::error::internal_error())?;

    // Step 2 (vault only): re-seal every sealed blob under the new key, then re-key the file
    // itself. Order matters - see vault::reseal_all_blobs's doc comment.
    {
        let vault_guard = state.vault.lock().unwrap();
        let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
        crate::vault::reseal_all_blobs(conn, &material.old_vault_key, &material.new_vault_key)?;
        crate::vault::rekey_vault(conn, &material.new_vault_key)?;
    }

    // Step 3 (registry only): commit the new salts/verifier/sentinel, clear PIN + biometric (both
    // wrap the old key). Only reached once the vault is fully consistent under the new key.
    {
        let registry = state.registry.lock().unwrap();
        crate::auth::commit_password_change(&registry, &username, &material)?;
    }

    // Re-arm the session with the new key so the vault stays usable without a fresh login.
    state.session.lock().unwrap().unlock(material.new_vault_key, &username, &SystemClock);

    Ok(())
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
        mobile_nav_style: config.mobile_nav_style.clone(),
        biometric_enabled: config.biometric_enabled,
        ssh_agent_enabled: config.ssh_agent_enabled,
        ui_variant: config.ui_variant.clone(),
        reprompt_mode: config.reprompt_mode.clone(),
        reprompt_grace_seconds: config.reprompt_grace_seconds,
        delete_reprompt_trash_item: config.delete_reprompt_trash_item,
        delete_reprompt_credential: config.delete_reprompt_credential,
        delete_reprompt_empty_trash: config.delete_reprompt_empty_trash,
        delete_reprompt_folder: config.delete_reprompt_folder,
    }
}

pub fn do_register(
    state: &AppState,
    paths: &AppPaths,
    username: &str,
    master_password: &str,
) -> Result<AuthSuccess, CommandError> {
    let vault_path = paths.checked_vault_path(username)?;
    let key = {
        let registry = state.registry.lock().unwrap();
        crate::auth::register_user(&registry, &vault_path, username, master_password)?
    };
    let conn = crate::vault::open_vault(&vault_path, &key)?;
    state.session.lock().unwrap().unlock(key, username, &SystemClock);
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

pub fn do_update_site(
    state: &AppState,
    site_id: i64,
    name: &str,
    url: Option<&str>,
    notes: Option<&str>,
) -> Result<(), CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    Ok(crate::vault::update_site(conn, site_id, name, url, notes)?)
}

pub fn do_set_site_favorite(state: &AppState, site_id: i64, favorite: bool) -> Result<(), CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    Ok(crate::vault::set_site_favorite(conn, site_id, favorite)?)
}

pub fn do_set_site_item_type(state: &AppState, site_id: i64, item_type: &str) -> Result<(), CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    Ok(crate::vault::set_site_item_type(conn, site_id, item_type)?)
}

pub fn do_set_site_card_hint(
    state: &AppState,
    site_id: i64,
    brand: Option<&str>,
    last4: Option<&str>,
) -> Result<(), CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    Ok(crate::vault::set_site_card_hint(conn, site_id, brand, last4)?)
}

pub fn do_set_site_folder(state: &AppState, site_id: i64, folder_id: Option<i64>) -> Result<(), CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    Ok(crate::vault::set_site_folder(conn, site_id, folder_id)?)
}

pub fn do_list_folders(state: &AppState) -> Result<Vec<FolderDto>, CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    Ok(crate::vault::list_folders(conn)?.into_iter().map(FolderDto::from).collect())
}

pub fn do_add_folder(state: &AppState, name: &str) -> Result<i64, CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    Ok(crate::vault::add_folder(conn, name)?)
}

pub fn do_rename_folder(state: &AppState, folder_id: i64, name: &str) -> Result<(), CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    Ok(crate::vault::rename_folder(conn, folder_id, name)?)
}

/// Unfiles the folder's items; never deletes them (see vault::delete_folder).
pub fn do_delete_folder(state: &AppState, folder_id: i64) -> Result<(), CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    Ok(crate::vault::delete_folder(conn, folder_id)?)
}

pub fn do_soft_delete_site(state: &AppState, site_id: i64) -> Result<(), CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    // No refuse-outright guard here anymore (08-09-PLAN.md, gap G-08-6): the command layer used
    // to refuse a non-empty site because the lower-level soft_delete_site had no cascade logic.
    // It cascades now, so a site with live credentials deletes cleanly - the user is warned by
    // the frontend's confirm copy (src/lib/confirm.ts's siteDeleteMessage) before this is ever
    // called, and Trash plus restore_credential's site-revival are the recovery path.
    Ok(crate::vault::soft_delete_site(conn, site_id)?)
}

pub fn do_add_credential(
    state: &AppState,
    site_id: i64,
    username: Option<&str>,
    secret: &str,
    remarks: Option<&str>,
    custom_fields: &[crate::vault::CustomField],
    reprompt_exempt: bool,
) -> Result<i64, CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    let session = state.session.lock().unwrap();
    let key = session.key().ok_or_else(crate::error::locked_error)?;
    Ok(crate::vault::add_credential(conn, key, site_id, username, secret, remarks, custom_fields, reprompt_exempt)?)
}

pub fn do_update_credential(
    state: &AppState,
    credential_id: i64,
    username: Option<&str>,
    secret: &str,
    remarks: Option<&str>,
    custom_fields: &[crate::vault::CustomField],
    reprompt_exempt: bool,
) -> Result<(), CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    let session = state.session.lock().unwrap();
    let key = session.key().ok_or_else(crate::error::locked_error)?;
    Ok(crate::vault::update_credential(conn, key, credential_id, username, secret, remarks, custom_fields, reprompt_exempt)?)
}

pub fn do_soft_delete_credential(state: &AppState, credential_id: i64) -> Result<(), CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    Ok(crate::vault::soft_delete_credential(conn, credential_id)?)
}

pub fn do_set_totp_secret(state: &AppState, credential_id: i64, secret: &str) -> Result<(), CommandError> {
    // Fail fast on a bad secret before persisting it - generate_code's own base32 decode is the
    // validation; the timestamp passed here is irrelevant to whether decoding succeeds.
    crate::totp::generate_code(secret, 0)?;
    // J2: persist the normalized form (whitespace/dashes/padding stripped, uppercased), not the
    // raw pasted string - do_totp_code already re-normalizes on every read regardless, so this
    // doesn't change behavior, it just stores the value it actually reads as instead of leaving
    // the raw one sitting in the vault only because normalize_secret happens to be idempotent.
    let normalized = crate::totp::normalize_secret(secret);
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    let session = state.session.lock().unwrap();
    let key = session.key().ok_or_else(crate::error::locked_error)?;
    Ok(crate::vault::set_totp_secret(conn, key, credential_id, &normalized)?)
}

pub fn do_clear_totp_secret(state: &AppState, credential_id: i64) -> Result<(), CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    Ok(crate::vault::clear_totp_secret(conn, credential_id)?)
}

pub fn do_totp_code(state: &AppState, credential_id: i64) -> Result<Option<TotpCodeDto>, CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    let session = state.session.lock().unwrap();
    let key = session.key().ok_or_else(crate::error::locked_error)?;
    let secret = crate::vault::get_totp_secret(conn, key, credential_id)?;
    drop(session);
    match secret {
        None => Ok(None),
        Some(s) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let code = crate::totp::generate_code(&s, now)?;
            Ok(Some(TotpCodeDto { code, seconds_remaining: crate::totp::seconds_remaining(now) }))
        }
    }
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

pub fn do_update_biometric_enabled(state: &AppState, paths: &AppPaths, enabled: bool) -> Result<(), CommandError> {
    let mut config = state.config.lock().unwrap();
    let mut updated = config.clone();
    updated.biometric_enabled = enabled;
    updated.save(&paths.config_path()).map_err(|_| crate::error::internal_error())?;
    *config = updated;
    Ok(())
}

pub fn do_update_ssh_agent_enabled(state: &AppState, paths: &AppPaths, enabled: bool) -> Result<(), CommandError> {
    let mut config = state.config.lock().unwrap();
    let mut updated = config.clone();
    updated.ssh_agent_enabled = enabled;
    updated.save(&paths.config_path()).map_err(|_| crate::error::internal_error())?;
    *config = updated;
    Ok(())
}

pub fn do_add_ssh_key(state: &AppState, label: &str, private_key_openssh: &str) -> Result<i64, CommandError> {
    // Fail fast on an unparsable/encrypted key before ever persisting it.
    crate::ssh_agent::public_key_line(private_key_openssh)?;
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    let session = state.session.lock().unwrap();
    let key = session.key().ok_or_else(crate::error::locked_error)?;
    Ok(crate::ssh_keys::add_ssh_key(conn, key, label, private_key_openssh)?)
}

pub fn do_list_ssh_keys(state: &AppState) -> Result<Vec<crate::dto::SshKeyDto>, CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    let session = state.session.lock().unwrap();
    let key = session.key().ok_or_else(crate::error::locked_error)?;
    let entries = crate::ssh_keys::list_ssh_keys(conn, key)?;
    entries
        .into_iter()
        .map(|e| {
            let public_key = crate::ssh_agent::public_key_line(&e.private_key_openssh)?;
            Ok(crate::dto::SshKeyDto { id: e.id, label: e.label, public_key })
        })
        .collect()
}

pub fn do_delete_ssh_key(state: &AppState, id: i64) -> Result<(), CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    Ok(crate::ssh_keys::delete_ssh_key(conn, id)?)
}

pub fn do_update_nav_collapsed(state: &AppState, paths: &AppPaths, collapsed: bool) -> Result<(), CommandError> {
    let mut config = state.config.lock().unwrap();
    let mut updated = config.clone();
    updated.nav_collapsed = collapsed;
    updated.save(&paths.config_path()).map_err(|_| crate::error::internal_error())?;
    *config = updated;
    Ok(())
}

pub fn do_update_mobile_nav_style(state: &AppState, paths: &AppPaths, style: &str) -> Result<(), CommandError> {
    if !VALID_MOBILE_NAV_STYLES.contains(&style) {
        return Err(crate::error::invalid_input_error("Unknown mobile nav style."));
    }
    let mut config = state.config.lock().unwrap();
    let mut updated = config.clone();
    updated.mobile_nav_style = style.to_string();
    updated.save(&paths.config_path()).map_err(|_| crate::error::internal_error())?;
    *config = updated;
    Ok(())
}

pub fn do_update_ui_variant(state: &AppState, paths: &AppPaths, variant: &str) -> Result<(), CommandError> {
    if !VALID_UI_VARIANTS.contains(&variant) {
        return Err(crate::error::invalid_input_error("Unknown UI variant."));
    }
    let mut config = state.config.lock().unwrap();
    let mut updated = config.clone();
    updated.ui_variant = variant.to_string();
    updated.save(&paths.config_path()).map_err(|_| crate::error::internal_error())?;
    *config = updated;
    Ok(())
}

pub fn do_update_reprompt_mode(state: &AppState, paths: &AppPaths, mode: &str) -> Result<(), CommandError> {
    if !VALID_REPROMPT_MODES.contains(&mode) {
        return Err(crate::error::invalid_input_error("Unknown reprompt mode."));
    }
    let mut config = state.config.lock().unwrap();
    let mut updated = config.clone();
    updated.reprompt_mode = mode.to_string();
    updated.save(&paths.config_path()).map_err(|_| crate::error::internal_error())?;
    *config = updated;
    Ok(())
}

pub fn do_update_reprompt_grace_seconds(state: &AppState, paths: &AppPaths, seconds: u32) -> Result<(), CommandError> {
    if !VALID_REPROMPT_GRACE_SECONDS.contains(&seconds) {
        return Err(crate::error::invalid_input_error("Unknown reprompt grace window."));
    }
    let mut config = state.config.lock().unwrap();
    let mut updated = config.clone();
    updated.reprompt_grace_seconds = seconds;
    updated.save(&paths.config_path()).map_err(|_| crate::error::internal_error())?;
    *config = updated;
    Ok(())
}

pub fn do_update_delete_reprompt(state: &AppState, paths: &AppPaths, action: &str, enabled: bool) -> Result<(), CommandError> {
    let mut config = state.config.lock().unwrap();
    let mut updated = config.clone();
    match action {
        "trashItem" => updated.delete_reprompt_trash_item = enabled,
        "deleteCredential" => updated.delete_reprompt_credential = enabled,
        "emptyTrash" => updated.delete_reprompt_empty_trash = enabled,
        "deleteFolder" => updated.delete_reprompt_folder = enabled,
        _ => return Err(crate::error::invalid_input_error("Unknown delete action.")),
    }
    updated.save(&paths.config_path()).map_err(|_| crate::error::internal_error())?;
    *config = updated;
    Ok(())
}

pub fn do_generate_password(req: &GenerateRequest) -> Result<Zeroizing<String>, CommandError> {
    use crate::generator as gen;
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

/// `destination_path`: `None` means the app-owned backups directory (the historical, unchanged
/// default). `Some` is only ever reached from the desktop transport, where the path came from a
/// native save dialog the user drove — never from the web transport, which always passes `None`
/// (see `web-src-tauri/src/handlers.rs::create_backup` for why that asymmetry is deliberate).
pub fn do_create_backup(
    state: &AppState,
    paths: &AppPaths,
    destination_path: Option<&str>,
) -> Result<String, CommandError> {
    let (username, bundle) = build_backup_bundle(state)?;
    let backup_path = match destination_path {
        None => {
            // Millisecond precision: a whole-second timestamp lets two backups made in quick
            // succession silently collide and overwrite each other.
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            paths.backups_dir().join(format!("{}-{}.bak", username, timestamp))
        }
        Some(p) if p.trim().is_empty() => {
            return Err(crate::error::invalid_input_error("Choose a backup destination."));
        }
        Some(p) => std::path::PathBuf::from(p),
    };
    std::fs::write(&backup_path, &bundle).map_err(|_| crate::error::internal_error())?;

    Ok(backup_path.to_string_lossy().to_string())
}

/// Path-free backup (web download): returns `(filename, bytes)` and touches no destination path.
pub fn do_create_backup_download(state: &AppState) -> Result<(String, Vec<u8>), CommandError> {
    let (username, bundle) = build_backup_bundle(state)?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    Ok((format!("{}-{}.bak", username, timestamp), bundle))
}

fn build_backup_bundle(state: &AppState) -> Result<(String, Vec<u8>), CommandError> {
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
        crate::registry::find_user(&registry, &username)
            .map_err(|_| crate::error::internal_error())?
    };

    let vault_bytes = std::fs::read(&vault_path).map_err(|_| crate::error::internal_error())?;
    let bundle = crate::backup::encode_bundle(&user_record, &vault_bytes);
    Ok((username, bundle))
}

pub fn do_restore_backup(
    state: &AppState,
    paths: &AppPaths,
    backup_file_path: &str,
    confirmed: bool,
) -> Result<(), CommandError> {
    let bytes = std::fs::read(backup_file_path)
        .map_err(|_| crate::error::invalid_input_error("Could not read that backup file."))?;
    do_restore_backup_from_bytes(state, paths, &bytes, confirmed)
}

/// Path-free restore (SEC-01/D-05): the web handler feeds it the bytes the browser uploaded.
pub fn do_restore_backup_from_bytes(
    state: &AppState,
    paths: &AppPaths,
    bytes: &[u8],
    confirmed: bool,
) -> Result<(), CommandError> {
    let (user_record, vault_bytes) = crate::backup::decode_bundle(bytes)
        .map_err(|_| crate::error::invalid_input_error("That file isn't a valid backup."))?;

    let vault_path = paths.checked_vault_path(&user_record.username)?;
    let already_exists = vault_path.exists() || {
        let registry = state.registry.lock().unwrap();
        crate::registry::find_user(&registry, &user_record.username).is_ok()
    };
    if already_exists && !confirmed {
        return Err(crate::error::confirm_required_error(
            "A vault for that username already exists. Restoring will overwrite it.",
        ));
    }

    // If the vault being restored over is the one currently open in AppState, close it first (its
    // connection/key is stale the instant we replace the file, and on Windows a live handle blocks
    // the rename). Locking the session in lockstep keeps vault.is_some() <=> session.is_unlocked().
    {
        let mut vault_guard = state.vault.lock().unwrap();
        if matches!(vault_guard.as_ref(), Some((_, open_path)) if *open_path == vault_path) {
            *vault_guard = None;
            state.session.lock().unwrap().lock();
        }
    }

    // Write to a temp path and commit the registry row BEFORE renaming into place: rename is the
    // last step and is effectively atomic on the same volume, so the failure window is far narrower
    // than a plain write-then-upsert ordering. Note this is the mirror-opposite order of
    // auth::register_user (vault-before-registry, to avoid an orphaned registry row) - here a crash
    // between the upsert and the rename can leave the registry pointing at a username with no vault
    // file yet. That's recoverable (retrying the same restore is deterministic and self-heals via the
    // already_exists check above), unlike reversing the order, which would risk destroying the old
    // vault file with no rollback if the upsert then failed.
    let tmp_path = vault_path.with_extension("vault.tmp");
    std::fs::write(&tmp_path, &vault_bytes).map_err(|_| crate::error::internal_error())?;

    let registry = state.registry.lock().unwrap();
    if crate::registry::upsert_user(&registry, &user_record).is_err() {
        let _ = std::fs::remove_file(&tmp_path); // best-effort cleanup
        return Err(crate::error::internal_error());
    }
    drop(registry);

    if std::fs::rename(&tmp_path, &vault_path).is_err() {
        let _ = std::fs::remove_file(&tmp_path); // best-effort cleanup
        return Err(crate::error::internal_error());
    }
    Ok(())
}

fn build_csv_rows(
    conn: &rusqlite::Connection,
    key: &crate::crypto::VaultKey,
) -> Result<Vec<crate::portio::CsvRow>, CommandError> {
    let sites = crate::vault::list_sites(conn)?;
    let mut rows = Vec::new();
    for site in sites {
        let creds = crate::vault::list_credentials_for_site(conn, key, site.id)?;
        for c in creds {
            rows.push(crate::portio::CsvRow {
                site_name: site.name.clone(),
                url: site.url.clone(),
                notes: site.notes.clone(),
                username: c.username,
                secret: c.secret,
                remarks: c.remarks,
            });
        }
    }
    Ok(rows)
}

fn append_plaintext_export_log(paths: &AppPaths, path: &str) -> std::io::Result<()> {
    use std::io::Write;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(paths.plaintext_export_log_path())?;
    writeln!(file, "{}\t{}", ts, path)
}

/// Produces the export bytes for `mode`, performing no filesystem access at all - the byte-producing
/// half of export shared by both the desktop (path-writing) and web (bytes-over-HTTP) transports.
pub fn do_export_csv_bytes(
    state: &AppState,
    mode: &ExportMode,
    confirmed: bool,
) -> Result<Vec<u8>, CommandError> {
    if matches!(mode, ExportMode::Plaintext) && !confirmed {
        return Err(crate::error::invalid_input_error(
            "Plaintext export requires explicit confirmation.",
        ));
    }

    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    let session = state.session.lock().unwrap();
    let key = session.key().ok_or_else(crate::error::locked_error)?;

    let rows = build_csv_rows(conn, key)?;
    let csv_text = crate::portio::rows_to_csv(&rows)?;

    let bytes: Vec<u8> = match mode {
        ExportMode::Plaintext => csv_text.into_bytes(),
        ExportMode::ThisDevice => {
            let (ciphertext, nonce) = crate::crypto::seal(key, csv_text.as_bytes())
                .map_err(|_| crate::error::internal_error())?;
            crate::portio::encode_sealed_csv(None, &nonce, &ciphertext)
        }
        ExportMode::WithPassword { password } => {
            let salt = crate::crypto::random_salt();
            let pw_key = crate::crypto::derive_key(password, &salt)
                .map_err(|_| crate::error::internal_error())?;
            let (ciphertext, nonce) = crate::crypto::seal(&pw_key, csv_text.as_bytes())
                .map_err(|_| crate::error::internal_error())?;
            crate::portio::encode_sealed_csv(Some(&salt), &nonce, &ciphertext)
        }
    };
    Ok(bytes)
}

/// Web-mode export: never touches the filesystem for the export target itself (SEC-01/D-01) - the
/// bytes are handed back for the HTTP response body. The plaintext-export log still gets an entry
/// (DATA-02), but with a fixed `<web download>` label standing in for the filesystem destination
/// desktop exports would otherwise record.
pub fn do_export_csv_download(
    state: &AppState,
    paths: &AppPaths,
    mode: &ExportMode,
    confirmed: bool,
) -> Result<(String, Vec<u8>), CommandError> {
    let bytes = do_export_csv_bytes(state, mode, confirmed)?;

    let filename = if matches!(mode, ExportMode::Plaintext) {
        if let Err(e) = append_plaintext_export_log(paths, "<web download>") {
            eprintln!("warning: failed to append plaintext export log entry: {e}");
        }
        "vault-export.csv"
    } else {
        "vault-export.pwenc"
    };
    Ok((filename.to_string(), bytes))
}

pub fn do_export_csv(
    state: &AppState,
    paths: &AppPaths,
    path: &str,
    mode: &ExportMode,
    confirmed: bool,
) -> Result<(), CommandError> {
    let bytes = do_export_csv_bytes(state, mode, confirmed)?;
    std::fs::write(path, &bytes).map_err(|_| crate::error::internal_error())?;

    if matches!(mode, ExportMode::Plaintext) {
        if let Err(e) = append_plaintext_export_log(paths, path) {
            eprintln!("warning: failed to append plaintext export log entry: {e}");
        }
    }
    Ok(())
}

/// Path-free header parse (SEC-01/D-01): the web branch reads no filesystem path at all — its
/// caller decodes the session's cached upload bytes into a `&str` and hands it here directly.
pub fn do_read_csv_headers_from_str(text: &str) -> Result<Vec<String>, CommandError> {
    let mut reader = csv::Reader::from_reader(text.as_bytes());
    let headers = reader
        .headers()
        .map_err(|_| crate::error::invalid_input_error("That file doesn't look like a CSV."))?;
    Ok(headers.iter().map(|h| h.to_string()).collect())
}

pub fn do_read_csv_headers(path: &str) -> Result<Vec<String>, CommandError> {
    let text = std::fs::read_to_string(path)
        .map_err(|_| crate::error::invalid_input_error("Could not read that file."))?;
    do_read_csv_headers_from_str(&text)
}

// --- Full-fidelity JSON vault export/import (Wave 4 follow-up) ---
// Mirrors the CSV export functions immediately above (same three modes via the same
// `ExportMode`, same sealed envelope from `portio::encode_sealed_csv`) plus a real import path,
// which the CSV encrypted export has never had (see portio.rs's decode_sealed_bytes doc comment).

pub fn do_export_json_bytes(state: &AppState, mode: &ExportMode, confirmed: bool) -> Result<Vec<u8>, CommandError> {
    if matches!(mode, ExportMode::Plaintext) && !confirmed {
        return Err(crate::error::invalid_input_error(
            "Plaintext export requires explicit confirmation.",
        ));
    }

    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    let session = state.session.lock().unwrap();
    let key = session.key().ok_or_else(crate::error::locked_error)?;

    let export = crate::jsonio::build_json_export(conn, key)?;
    let json_bytes = crate::jsonio::export_to_json_bytes(&export)?;

    let bytes: Vec<u8> = match mode {
        ExportMode::Plaintext => json_bytes,
        ExportMode::ThisDevice => {
            let (ciphertext, nonce) = crate::crypto::seal(key, &json_bytes).map_err(|_| crate::error::internal_error())?;
            crate::portio::encode_sealed_csv(None, &nonce, &ciphertext)
        }
        ExportMode::WithPassword { password } => {
            let salt = crate::crypto::random_salt();
            let pw_key = crate::crypto::derive_key(password, &salt).map_err(|_| crate::error::internal_error())?;
            let (ciphertext, nonce) = crate::crypto::seal(&pw_key, &json_bytes).map_err(|_| crate::error::internal_error())?;
            crate::portio::encode_sealed_csv(Some(&salt), &nonce, &ciphertext)
        }
    };
    Ok(bytes)
}

pub fn do_export_json_download(
    state: &AppState,
    paths: &AppPaths,
    mode: &ExportMode,
    confirmed: bool,
) -> Result<(String, Vec<u8>), CommandError> {
    let bytes = do_export_json_bytes(state, mode, confirmed)?;
    let filename = if matches!(mode, ExportMode::Plaintext) {
        if let Err(e) = append_plaintext_export_log(paths, "<web download>") {
            eprintln!("warning: failed to append plaintext export log entry: {e}");
        }
        "vault-export.json"
    } else {
        "vault-export.pwenc.json"
    };
    Ok((filename.to_string(), bytes))
}

pub fn do_export_json(
    state: &AppState,
    paths: &AppPaths,
    path: &str,
    mode: &ExportMode,
    confirmed: bool,
) -> Result<(), CommandError> {
    let bytes = do_export_json_bytes(state, mode, confirmed)?;
    std::fs::write(path, &bytes).map_err(|_| crate::error::internal_error())?;
    if matches!(mode, ExportMode::Plaintext) {
        if let Err(e) = append_plaintext_export_log(paths, path) {
            eprintln!("warning: failed to append plaintext export log entry: {e}");
        }
    }
    Ok(())
}

/// Unseals (if needed) and parses an uploaded/read export file into the structure both preview and
/// commit need. `password` is required only when the file turns out to be in with-password mode;
/// `ThisDevice`-mode files decrypt with the CURRENT session's vault key, so they can only be
/// re-imported into the same vault they were exported from - portability across vaults needs
/// with-password or plaintext mode, same distinction the export side already documents.
fn decrypt_json_export(state: &AppState, bytes: &[u8], password: Option<&str>) -> Result<crate::jsonio::JsonExportVault, CommandError> {
    // Plaintext files parse directly as JSON; anything else is our sealed envelope.
    if let Ok(export) = crate::jsonio::parse_json_export(bytes) {
        return Ok(export);
    }
    let payload = crate::portio::decode_sealed_bytes(bytes)?;
    let plaintext = match payload {
        crate::portio::SealedPayload::ThisDevice { nonce, ciphertext } => {
            let session = state.session.lock().unwrap();
            let key = session.key().ok_or_else(crate::error::locked_error)?;
            crate::crypto::open(key, &ciphertext, &nonce).map_err(|_| crate::error::invalid_input_error("Could not decrypt that file with this vault's key."))?
        }
        crate::portio::SealedPayload::WithPassword { salt, nonce, ciphertext } => {
            let password = password.ok_or_else(|| crate::error::invalid_input_error("This file needs the password it was exported with."))?;
            let pw_key = crate::crypto::derive_key(password, &salt).map_err(|_| crate::error::internal_error())?;
            crate::crypto::open(&pw_key, &ciphertext, &nonce).map_err(|_| crate::error::invalid_input_error("Incorrect export password."))?
        }
    };
    Ok(crate::jsonio::parse_json_export(&plaintext)?)
}

pub fn do_import_json_preview(state: &AppState, bytes: &[u8], password: Option<&str>) -> Result<crate::jsonio::JsonImportPreview, CommandError> {
    let export = decrypt_json_export(state, bytes, password)?;
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    Ok(crate::jsonio::preview_json_import(conn, &export)?)
}

pub fn do_import_json_commit(state: &AppState, bytes: &[u8], password: Option<&str>) -> Result<ImportSummaryDto, CommandError> {
    let export = decrypt_json_export(state, bytes, password)?;
    let mut vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_mut().ok_or_else(crate::error::locked_error)?;
    let session = state.session.lock().unwrap();
    let key = session.key().ok_or_else(crate::error::locked_error)?;
    let summary = crate::jsonio::import_json_vault(conn, key, &export)?;
    Ok(ImportSummaryDto::from(summary))
}

/// Path-free preview parse (SEC-01/D-01/D-03): the web branch reads no filesystem path at all —
/// its caller decodes the session's cached upload bytes into a `&str` and hands it here directly.
/// Reading does not consume anything (this takes `&str`, not ownership of the cache), so the same
/// upload can be previewed again with a different format.
pub fn do_import_csv_preview_from_str(
    text: &str,
    format: &ImportFormat,
) -> Result<Vec<ImportRecordDto>, CommandError> {
    let records = match format {
        ImportFormat::Chromium => crate::portio::parse_chromium_csv(text)?,
        ImportFormat::Firefox => crate::portio::parse_firefox_csv(text)?,
        ImportFormat::LastPass => crate::portio::parse_lastpass_csv(text)?,
        ImportFormat::Generic { mapping } => crate::portio::parse_generic_csv(text, mapping)?,
    };
    Ok(records.into_iter().map(ImportRecordDto::from).collect())
}

pub fn do_import_csv_preview(
    path: &str,
    format: &ImportFormat,
) -> Result<Vec<ImportRecordDto>, CommandError> {
    let text = std::fs::read_to_string(path)
        .map_err(|_| crate::error::invalid_input_error("Could not read that file."))?;
    do_import_csv_preview_from_str(&text, format)
}

pub fn do_import_csv_commit(
    state: &AppState,
    records: Vec<ImportRecordDto>,
) -> Result<ImportSummaryDto, CommandError> {
    let core_records: Vec<crate::portio::ImportRecord> =
        records.into_iter().map(Into::into).collect();

    let mut vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_mut().ok_or_else(crate::error::locked_error)?;
    let session = state.session.lock().unwrap();
    let key = session.key().ok_or_else(crate::error::locked_error)?;

    let summary = crate::vault::import_credentials(conn, key, &core_records)?;
    Ok(ImportSummaryDto::from(summary))
}

// Module-level (not nested) so do_check_breach can reuse it - same shape build_csv_rows uses.
struct HealthRow {
    credential_id: i64,
    site_id: i64,
    site_name: String,
    site_url: Option<String>,
    username: Option<String>,
    secret: String,
}

fn build_health_rows(
    conn: &rusqlite::Connection,
    key: &crate::crypto::VaultKey,
) -> Result<Vec<HealthRow>, CommandError> {
    let sites = crate::vault::list_sites(conn)?;
    let mut rows = Vec::new();
    for site in &sites {
        for c in crate::vault::list_credentials_for_site(conn, key, site.id)? {
            rows.push(HealthRow {
                credential_id: c.id,
                site_id: site.id,
                site_name: site.name.clone(),
                site_url: site.url.clone(),
                username: c.username,
                secret: c.secret,
            });
        }
    }
    Ok(rows)
}

pub fn do_health_report(state: &AppState) -> Result<HealthReportDto, CommandError> {
    let vault_guard = state.vault.lock().unwrap();
    let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
    let session = state.session.lock().unwrap();
    let key = session.key().ok_or_else(crate::error::locked_error)?;
    let rows = build_health_rows(conn, key)?;
    // Drop session before locking state.config below - do_update_auto_lock_minutes locks config then
    // session, so holding both here in the opposite order would be a lock-order inversion.
    drop(session);

    let weak: Vec<HealthEntryDto> = rows
        .iter()
        .filter(|r| do_analyze_strength(&r.secret).score <= 1)
        .map(|r| HealthEntryDto {
            credential_id: r.credential_id,
            site_id: r.site_id,
            site_name: r.site_name.clone(),
            username: r.username.clone(),
        })
        .collect();

    let reuse_inputs: Vec<crate::health::ReuseInput> = rows
        .iter()
        .map(|r| crate::health::ReuseInput { credential_id: r.credential_id, secret: r.secret.clone() })
        .collect();
    let reused: Vec<ReusedGroupDto> = crate::health::classify_reused(&reuse_inputs)
        .into_iter()
        .map(|group| ReusedGroupDto {
            entries: group
                .credential_ids
                .into_iter()
                .filter_map(|id| rows.iter().find(|r| r.credential_id == id))
                .map(|r| HealthEntryDto {
                    credential_id: r.credential_id,
                    site_id: r.site_id,
                    site_name: r.site_name.clone(),
                    username: r.username.clone(),
                })
                .collect(),
        })
        .collect();

    let old_days = state.config.lock().unwrap().old_password_days;
    let old: Vec<HealthEntryDto> = crate::vault::list_old_credentials(conn, old_days)?
        .into_iter()
        .map(|o| HealthEntryDto {
            credential_id: o.credential_id,
            site_id: o.site_id,
            site_name: o.site_name,
            username: o.username,
        })
        .collect();

    let unsecured: Vec<HealthEntryDto> = rows
        .iter()
        .filter(|r| r.site_url.as_deref().is_some_and(crate::health::is_unsecured_url))
        .map(|r| HealthEntryDto {
            credential_id: r.credential_id,
            site_id: r.site_id,
            site_name: r.site_name.clone(),
            username: r.username.clone(),
        })
        .collect();

    Ok(HealthReportDto { weak, reused, old, unsecured })
}

pub fn do_check_breach(state: &AppState, hibp_base_url: &str) -> Result<BreachReportDto, CommandError> {
    // Server-side opt-in gate: rejects regardless of what the frontend sends. The temporary guard
    // drops at the end of the statement, so config is never held while vault/session are acquired.
    if !state.config.lock().unwrap().hibp_opt_in {
        return Err(crate::error::invalid_input_error(
            "Breach checking is not enabled. Turn it on in Settings first.",
        ));
    }

    // vault_guard and session are scoped to this block and dropped as soon as rows is built - this
    // function then blocks on network I/O per distinct secret, so neither lock is held across it.
    let rows = {
        let vault_guard = state.vault.lock().unwrap();
        let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
        let session = state.session.lock().unwrap();
        let key = session.key().ok_or_else(crate::error::locked_error)?;
        build_health_rows(conn, key)?
    };

    // One HTTP call per distinct secret, not per credential. Grouped by hand (not via
    // classify_reused, which only returns groups of 2+) so a password used exactly once is still
    // checked.
    let mut by_secret: std::collections::HashMap<&str, Vec<&HealthRow>> = std::collections::HashMap::new();
    for r in &rows {
        by_secret.entry(r.secret.as_str()).or_default().push(r);
    }

    let client = reqwest::blocking::Client::new();
    let mut breached = Vec::new();
    for (secret, members) in &by_secret {
        let hit = crate::hibp::check_password(&client, hibp_base_url, secret);
        if matches!(hit, Ok(Some(_))) {
            for r in members {
                breached.push(HealthEntryDto {
                    credential_id: r.credential_id,
                    site_id: r.site_id,
                    site_name: r.site_name.clone(),
                    username: r.username.clone(),
                });
            }
        }
        // A single failed HIBP request doesn't fail the whole batch - best-effort.
    }

    Ok(BreachReportDto { breached })
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
            crate::vault::add_credential(conn, key, site_id, Some("alice"), "s3cret", None, &[], false).unwrap();
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
            crate::vault::add_credential(conn, key, site_id, Some("alice"), "secret", None, &[], false).unwrap();
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
    fn vault_exists_reflects_whether_any_account_has_ever_registered() {
        let (state, paths, _dir) = test_state_and_paths();
        assert_eq!(do_vault_exists(&state).unwrap(), false);

        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        assert_eq!(do_vault_exists(&state).unwrap(), true);

        // Still true even locked - this only ever looks at the registry, never state.session or
        // state.vault (both are None/locked here), matching do_has_pin's own "works while locked"
        // contract that the Login/Register screen relies on.
        do_lock(&state);
        assert_eq!(do_vault_exists(&state).unwrap(), true);
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
    fn update_site_then_list_sites_reflects_the_update() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let site_id = do_add_site(&state, "example.com", None, None).unwrap();

        do_update_site(&state, site_id, "new-name.com", Some("https://new-name.com"), None).unwrap();
        let sites = do_list_sites(&state).unwrap();

        assert_eq!(sites[0].name, "new-name.com");
    }

    #[test]
    fn do_soft_delete_site_succeeds_when_the_site_still_has_credentials() {
        // Previously this asserted a refusal (invalid_input). The user rejected that behavior
        // (G-08-6, 08-UAT.md) - a site with credentials now deletes cleanly, cascading to its
        // credentials, gated behind a frontend confirmation instead of a hard backend refusal.
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let site_id = do_add_site(&state, "example.com", None, None).unwrap();
        do_add_credential(&state, site_id, Some("bob"), "s3cret", None, &[], false).unwrap();

        do_soft_delete_site(&state, site_id).unwrap();

        assert!(do_list_sites(&state).unwrap().is_empty());
    }

    #[test]
    fn soft_delete_site_succeeds_once_its_credential_is_gone() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let site_id = do_add_site(&state, "example.com", None, None).unwrap();
        let cred_id = do_add_credential(&state, site_id, Some("bob"), "s3cret", None, &[], false).unwrap();
        do_soft_delete_credential(&state, cred_id).unwrap();

        do_soft_delete_site(&state, site_id).unwrap();

        assert!(do_list_sites(&state).unwrap().is_empty());
    }

    #[test]
    fn add_credential_update_then_list_reflects_the_update() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let site_id = do_add_site(&state, "example.com", None, None).unwrap();

        let cred_id = do_add_credential(&state, site_id, Some("bob"), "s3cret", None, &[], false).unwrap();
        do_update_credential(&state, cred_id, Some("bob2"), "new-secret", Some("updated"), &[], false).unwrap();
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
        let cred_id = do_add_credential(&state, site_id, Some("alice"), "s3cret", None, &[], false).unwrap();

        assert_eq!(do_search(&state, "github").unwrap().len(), 1);

        do_soft_delete_credential(&state, cred_id).unwrap();

        assert!(do_search(&state, "github").unwrap().is_empty());
    }

    #[test]
    fn trash_lifecycle_list_restore_purge() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let site_id = do_add_site(&state, "example.com", None, None).unwrap();
        let cred_id = do_add_credential(&state, site_id, Some("bob"), "s3cret", None, &[], false).unwrap();

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
    fn update_mobile_nav_style_persists_and_rejects_invalid_value() {
        let (state, paths, _dir) = test_state_and_paths();
        do_update_mobile_nav_style(&state, &paths, "drawer").unwrap();
        assert_eq!(do_get_config(&state).mobile_nav_style, "drawer");
        assert_eq!(do_update_mobile_nav_style(&state, &paths, "sidebar-carousel").unwrap_err().code(), "invalid_input");
    }

    #[test]
    fn get_config_includes_mobile_nav_style_default() {
        let (state, _paths, _dir) = test_state_and_paths();
        assert_eq!(do_get_config(&state).mobile_nav_style, "icon-rail");
    }

    #[test]
    fn update_mobile_nav_style_does_not_mutate_state_when_save_fails() {
        let (state, _paths, dir) = test_state_and_paths();
        let bad_paths = AppPaths::new(dir.path().join("nonexistent-subdir-xyz"));
        let before = do_get_config(&state).mobile_nav_style;
        let result = do_update_mobile_nav_style(&state, &bad_paths, "drawer");
        assert!(result.is_err());
        assert_eq!(do_get_config(&state).mobile_nav_style, before, "state.config must not change when the save fails");
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
    fn update_biometric_enabled_persists() {
        let (state, paths, _dir) = test_state_and_paths();
        do_update_biometric_enabled(&state, &paths, true).unwrap();
        assert_eq!(do_get_config(&state).biometric_enabled, true);
        let reloaded = Config::load(&paths.config_path()).unwrap();
        assert_eq!(reloaded.biometric_enabled, true);
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

    #[test]
    fn generate_password_random_returns_requested_length() {
        let req = GenerateRequest::Random { length: 20, uppercase: true, lowercase: true, digits: true, symbols: false };
        let pw = do_generate_password(&req).unwrap();
        assert_eq!(pw.chars().count(), 20);
    }

    #[test]
    fn generate_password_memorable_uses_separator() {
        // "|" not "-": the EFF short wordlist's "yo-yo" entry contains a literal hyphen.
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

    fn backup_for(state: &AppState, paths: &AppPaths, username: &str) -> Vec<u8> {
        do_register(state, paths, username, "correct horse battery staple").unwrap();
        let path = do_create_backup(state, paths, None).unwrap();
        std::fs::read(path).unwrap()
    }

    #[test]
    fn create_backup_writes_a_decodable_bundle_under_backups_dir() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();

        let path = do_create_backup(&state, &paths, None).unwrap();

        assert!(std::path::Path::new(&path).starts_with(paths.backups_dir()));
        let bytes = std::fs::read(&path).unwrap();
        let (user, vault_bytes) = crate::backup::decode_bundle(&bytes).unwrap();
        assert_eq!(user.username, "alice");
        assert!(!vault_bytes.is_empty());
    }

    #[test]
    fn create_backup_when_locked_returns_locked_error() {
        let (state, paths, _dir) = test_state_and_paths();
        let result = do_create_backup(&state, &paths, None);
        assert_eq!(result.unwrap_err().code(), "locked");
    }

    #[test]
    fn create_backup_writes_to_an_explicit_destination_when_given_one() {
        let (state, paths, dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let chosen = dir.path().join("chosen-name.bak");

        let path = do_create_backup(&state, &paths, Some(chosen.to_str().unwrap())).unwrap();

        assert_eq!(path, chosen.to_string_lossy().to_string());
        assert!(chosen.exists());
        let entries: Vec<_> = std::fs::read_dir(paths.backups_dir())
            .map(|rd| rd.filter_map(|e| e.ok()).collect())
            .unwrap_or_default();
        assert!(entries.is_empty(), "nothing should be written under backups_dir()");
        let bytes = std::fs::read(&chosen).unwrap();
        let (user, vault_bytes) = crate::backup::decode_bundle(&bytes).unwrap();
        assert_eq!(user.username, "alice");
        assert!(!vault_bytes.is_empty());
    }

    #[test]
    fn create_backup_rejects_an_empty_destination() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();

        let result = do_create_backup(&state, &paths, Some(""));

        assert_eq!(result.unwrap_err().code(), "invalid_input");
        let entries: Vec<_> = std::fs::read_dir(paths.backups_dir())
            .map(|rd| rd.filter_map(|e| e.ok()).collect())
            .unwrap_or_default();
        assert!(entries.is_empty(), "nothing should be written for an empty destination");
    }

    #[test]
    fn create_backup_when_locked_ignores_the_destination() {
        let (state, paths, dir) = test_state_and_paths();
        let chosen = dir.path().join("x.bak");

        let result = do_create_backup(&state, &paths, Some(chosen.to_str().unwrap()));

        assert_eq!(result.unwrap_err().code(), "locked");
        assert!(!chosen.exists());
    }

    #[test]
    fn restore_backup_onto_a_brand_new_username_needs_no_confirmation() {
        let (state, paths, dir) = test_state_and_paths();
        let bundle = backup_for(&state, &paths, "alice");
        let bundle_path = dir.path().join("alice.bak");
        std::fs::write(&bundle_path, &bundle).unwrap();

        let (fresh_state, fresh_paths, _fresh_dir) = test_state_and_paths();

        let result = do_restore_backup(&fresh_state, &fresh_paths, bundle_path.to_str().unwrap(), false);

        assert!(result.is_ok());
        let registry = fresh_state.registry.lock().unwrap();
        assert!(crate::registry::find_user(&registry, "alice").is_ok());
        assert!(fresh_paths.vault_path("alice").exists());
    }

    #[test]
    fn restore_backup_onto_an_existing_username_without_confirmation_is_rejected() {
        let (state, paths, dir) = test_state_and_paths();
        let bundle = backup_for(&state, &paths, "alice");
        let bundle_path = dir.path().join("alice.bak");
        std::fs::write(&bundle_path, &bundle).unwrap();

        let result = do_restore_backup(&state, &paths, bundle_path.to_str().unwrap(), false);

        assert_eq!(result.unwrap_err().code(), "confirm_required");
    }

    #[test]
    fn restore_backup_onto_an_existing_username_with_confirmation_overwrites() {
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

    fn seeded_vault_state() -> (AppState, AppPaths, tempfile::TempDir) {
        let (state, paths, dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let site_id = do_add_site(&state, "example.com", None, None).unwrap();
        do_add_credential(&state, site_id, Some("alice"), "s3cret!", None, &[], false).unwrap();
        (state, paths, dir)
    }

    #[test]
    fn export_csv_this_device_mode_produces_a_sealable_file_decryptable_with_the_session_key() {
        let (state, paths, dir) = seeded_vault_state();
        let out_path = dir.path().join("export.pwenc");

        do_export_csv(&state, &paths, out_path.to_str().unwrap(), &ExportMode::ThisDevice, false).unwrap();

        let bytes = std::fs::read(&out_path).unwrap();
        assert_eq!(bytes[1], 0); // mode byte: this-device
        let nonce: [u8; 12] = bytes[2..14].try_into().unwrap();
        let len = u32::from_le_bytes(bytes[14..18].try_into().unwrap()) as usize;
        let ciphertext = &bytes[18..18 + len];
        let session = state.session.lock().unwrap();
        let key = session.key().unwrap();
        let plaintext = crate::crypto::open(key, ciphertext, &nonce).unwrap();
        assert!(String::from_utf8(plaintext).unwrap().contains("example.com"));
    }

    #[test]
    fn export_csv_with_password_mode_embeds_a_salt_and_is_decryptable_with_that_password() {
        let (state, paths, dir) = seeded_vault_state();
        let out_path = dir.path().join("export.pwenc");

        do_export_csv(
            &state, &paths, out_path.to_str().unwrap(),
            &ExportMode::WithPassword { password: "export password".to_string() }, false,
        ).unwrap();

        let bytes = std::fs::read(&out_path).unwrap();
        assert_eq!(bytes[1], 1); // mode byte: with-password
        let salt: [u8; 16] = bytes[2..18].try_into().unwrap();
        let nonce: [u8; 12] = bytes[18..30].try_into().unwrap();
        let len = u32::from_le_bytes(bytes[30..34].try_into().unwrap()) as usize;
        let ciphertext = &bytes[34..34 + len];
        let key = crate::crypto::derive_key("export password", &salt).unwrap();
        let plaintext = crate::crypto::open(&key, ciphertext, &nonce).unwrap();
        assert!(String::from_utf8(plaintext).unwrap().contains("example.com"));
    }

    #[test]
    fn export_csv_plaintext_mode_without_confirmation_is_rejected() {
        let (state, paths, dir) = seeded_vault_state();
        let out_path = dir.path().join("export.csv");

        let result = do_export_csv(&state, &paths, out_path.to_str().unwrap(), &ExportMode::Plaintext, false);

        assert_eq!(result.unwrap_err().code(), "invalid_input");
        assert!(!out_path.exists());
        assert!(!paths.plaintext_export_log_path().exists(), "no log entry for a rejected export");
    }

    #[test]
    fn export_csv_plaintext_mode_with_confirmation_writes_readable_csv() {
        let (state, paths, dir) = seeded_vault_state();
        let out_path = dir.path().join("export.csv");

        do_export_csv(&state, &paths, out_path.to_str().unwrap(), &ExportMode::Plaintext, true).unwrap();

        let text = std::fs::read_to_string(&out_path).unwrap();
        assert!(text.contains("example.com"));
        assert!(text.contains("s3cret!"));
    }

    #[test]
    fn export_json_with_password_then_import_json_preview_and_commit_round_trips_through_commands() {
        // Command-layer round trip (unlike jsonio.rs's own unit tests, which call jsonio functions
        // directly): export from one state, import into a second, fresh, differently-keyed vault -
        // the actual portability use case this feature exists for.
        let (state, paths, dir) = seeded_vault_state();
        let out_path = dir.path().join("export.json");
        do_export_json(
            &state, &paths, out_path.to_str().unwrap(),
            &ExportMode::WithPassword { password: "export password".to_string() }, false,
        ).unwrap();
        let bytes = std::fs::read(&out_path).unwrap();

        let (target_state, target_paths, _target_dir) = test_state_and_paths();
        do_register(&target_state, &target_paths, "bob", "another password entirely").unwrap();

        let preview = do_import_json_preview(&target_state, &bytes, Some("export password")).unwrap();
        assert_eq!(preview.site_count, 1);
        assert_eq!(preview.credential_count, 1);
        assert_eq!(preview.duplicate_credential_count, 0);

        let summary = do_import_json_commit(&target_state, &bytes, Some("export password")).unwrap();
        assert_eq!(summary.imported, 1);
        assert_eq!(summary.duplicates, 0);

        let sites = do_list_sites(&target_state).unwrap();
        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].name, "example.com");
    }

    #[test]
    fn import_json_commit_rejects_the_wrong_export_password() {
        let (state, paths, dir) = seeded_vault_state();
        let out_path = dir.path().join("export.json");
        do_export_json(
            &state, &paths, out_path.to_str().unwrap(),
            &ExportMode::WithPassword { password: "correct password".to_string() }, false,
        ).unwrap();
        let bytes = std::fs::read(&out_path).unwrap();

        let result = do_import_json_commit(&state, &bytes, Some("wrong password"));
        assert_eq!(result.unwrap_err().code(), "invalid_input");
    }

    #[test]
    fn import_json_commit_rejects_a_file_that_is_not_a_vault_export() {
        let (state, _paths, _dir) = seeded_vault_state();
        let result = do_import_json_commit(&state, b"just some random bytes, not an export", None);
        assert_eq!(result.unwrap_err().code(), "invalid_input");
    }

    #[test]
    fn export_json_plaintext_mode_without_confirmation_is_rejected() {
        let (state, paths, dir) = seeded_vault_state();
        let out_path = dir.path().join("export.json");
        let result = do_export_json(&state, &paths, out_path.to_str().unwrap(), &ExportMode::Plaintext, false);
        assert_eq!(result.unwrap_err().code(), "invalid_input");
        assert!(!out_path.exists());
    }

    #[test]
    fn export_json_plaintext_mode_writes_readable_json_with_every_field() {
        let (state, paths, dir) = seeded_vault_state();
        let out_path = dir.path().join("export.json");
        do_export_json(&state, &paths, out_path.to_str().unwrap(), &ExportMode::Plaintext, true).unwrap();
        let text = std::fs::read_to_string(&out_path).unwrap();
        assert!(text.contains("example.com"));
        assert!(text.contains("s3cret!"));
        // Confirms this is not just the CSV shape re-serialized - fields CSV never carried.
        assert!(text.contains("repromptExempt"));
        assert!(text.contains("customFields"));
    }

    #[test]
    fn export_csv_plaintext_mode_appends_an_entry_to_the_local_export_log() {
        let (state, paths, dir) = seeded_vault_state();
        let out_path = dir.path().join("export.csv");

        do_export_csv(&state, &paths, out_path.to_str().unwrap(), &ExportMode::Plaintext, true).unwrap();
        let out_path_2 = dir.path().join("export2.csv");
        do_export_csv(&state, &paths, out_path_2.to_str().unwrap(), &ExportMode::Plaintext, true).unwrap();

        let log = std::fs::read_to_string(paths.plaintext_export_log_path()).unwrap();
        let lines: Vec<&str> = log.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains(out_path.to_str().unwrap()));
        assert!(lines[1].contains(out_path_2.to_str().unwrap()));
        let ts: u64 = lines[0].split('\t').next().unwrap().parse().unwrap();
        assert!(ts > 1_700_000_000);
    }

    #[test]
    fn export_csv_this_device_and_with_password_modes_do_not_touch_the_export_log() {
        let (state, paths, dir) = seeded_vault_state();
        do_export_csv(&state, &paths, dir.path().join("a.pwenc").to_str().unwrap(), &ExportMode::ThisDevice, false).unwrap();
        do_export_csv(
            &state, &paths, dir.path().join("b.pwenc").to_str().unwrap(),
            &ExportMode::WithPassword { password: "x".to_string() }, false,
        ).unwrap();

        assert!(!paths.plaintext_export_log_path().exists());
    }

    #[test]
    fn export_csv_when_locked_returns_locked_error() {
        let (state, paths, dir) = test_state_and_paths();
        let out_path = dir.path().join("export.csv");
        let result = do_export_csv(&state, &paths, out_path.to_str().unwrap(), &ExportMode::Plaintext, true);
        assert_eq!(result.unwrap_err().code(), "locked");
    }

    #[test]
    fn read_csv_headers_returns_the_header_row() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.csv");
        std::fs::write(&path, "Site,Login,Pwd\nexample.com,alice,secret\n").unwrap();

        let headers = do_read_csv_headers(path.to_str().unwrap()).unwrap();

        assert_eq!(headers, vec!["Site".to_string(), "Login".to_string(), "Pwd".to_string()]);
    }

    fn import_record_dto(site: &str, username: &str, secret: &str) -> ImportRecordDto {
        ImportRecordDto {
            site_name: site.to_string(),
            url: None,
            notes: None,
            username: Some(username.to_string()),
            secret: Zeroizing::new(secret.to_string()),
            remarks: None,
        }
    }

    #[test]
    fn read_csv_headers_on_a_missing_file_returns_invalid_input() {
        let result = do_read_csv_headers("this-file-does-not-exist.csv");
        assert_eq!(result.unwrap_err().code(), "invalid_input");
    }

    #[test]
    fn import_csv_preview_parses_a_chromium_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("chrome.csv");
        std::fs::write(&path, "name,url,username,password\nexample.com,https://example.com,alice,s3cret\n").unwrap();

        let records = do_import_csv_preview(path.to_str().unwrap(), &ImportFormat::Chromium).unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].site_name, "example.com");
        assert_eq!(&*records[0].secret, "s3cret");
    }

    #[test]
    fn import_csv_preview_parses_a_lastpass_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("lastpass.csv");
        std::fs::write(
            &path,
            "url,username,password,totp,extra,name,grouping,fav\nhttps://example.com,alice,s3cret,,,My Login,,0\n",
        )
        .unwrap();

        let records = do_import_csv_preview(path.to_str().unwrap(), &ImportFormat::LastPass).unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].site_name, "My Login");
        assert_eq!(&*records[0].secret, "s3cret");
    }

    #[test]
    fn import_csv_preview_surfaces_a_header_mismatch_as_invalid_input() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.csv");
        std::fs::write(&path, "a,b,c\n1,2,3\n").unwrap();

        let result = do_import_csv_preview(path.to_str().unwrap(), &ImportFormat::Chromium);

        assert_eq!(result.unwrap_err().code(), "invalid_input");
    }

    #[test]
    fn import_csv_preview_parses_generic_with_a_mapping() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("generic.csv");
        std::fs::write(&path, "Site,Pwd\nexample.com,s3cret\n").unwrap();
        let mut mapping = std::collections::HashMap::new();
        mapping.insert("Site".to_string(), "site_name".to_string());
        mapping.insert("Pwd".to_string(), "secret".to_string());

        let records = do_import_csv_preview(
            path.to_str().unwrap(),
            &ImportFormat::Generic { mapping },
        ).unwrap();

        assert_eq!(records[0].site_name, "example.com");
    }

    #[test]
    fn import_csv_commit_inserts_records_and_returns_counts() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();

        let summary = do_import_csv_commit(
            &state,
            vec![import_record_dto("example.com", "bob", "s3cret")],
        ).unwrap();

        assert_eq!(summary.imported, 1);
        assert_eq!(summary.duplicates, 0);
        assert_eq!(do_list_sites(&state).unwrap().len(), 1);
    }

    #[test]
    fn import_csv_commit_when_locked_returns_locked_error() {
        let (state, _paths, _dir) = test_state_and_paths();
        let result = do_import_csv_commit(&state, vec![import_record_dto("example.com", "bob", "s3cret")]);
        assert_eq!(result.unwrap_err().code(), "locked");
    }

    #[test]
    fn import_csv_commit_surfaces_a_missing_site_name_as_invalid_input() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let result = do_import_csv_commit(&state, vec![import_record_dto("", "bob", "s3cret")]);
        assert_eq!(result.unwrap_err().code(), "invalid_input");
    }

    #[test]
    fn health_report_flags_a_weak_password() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let site_id = do_add_site(&state, "example.com", None, None).unwrap();
        do_add_credential(&state, site_id, Some("alice"), "password", None, &[], false).unwrap(); // weak

        let report = do_health_report(&state).unwrap();

        assert_eq!(report.weak.len(), 1);
        assert_eq!(report.weak[0].site_name, "example.com");
    }

    #[test]
    fn health_report_does_not_flag_a_strong_password_as_weak() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let site_id = do_add_site(&state, "example.com", None, None).unwrap();
        do_add_credential(&state, site_id, Some("alice"), "7#kLp!qZ2r@Vx9W", None, &[], false).unwrap();

        assert!(do_health_report(&state).unwrap().weak.is_empty());
    }

    #[test]
    fn health_report_groups_reused_passwords() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let site_id = do_add_site(&state, "example.com", None, None).unwrap();
        do_add_credential(&state, site_id, Some("alice"), "7#kLp!qZ2r@Vx9W", None, &[], false).unwrap();
        do_add_credential(&state, site_id, Some("bob"), "7#kLp!qZ2r@Vx9W", None, &[], false).unwrap();

        let report = do_health_report(&state).unwrap();

        assert_eq!(report.reused.len(), 1);
        assert_eq!(report.reused[0].entries.len(), 2);
    }

    #[test]
    fn health_report_flags_an_unsecured_http_site() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let http_site = do_add_site(&state, "insecure.example", Some("http://insecure.example"), None).unwrap();
        let https_site = do_add_site(&state, "secure.example", Some("https://secure.example"), None).unwrap();
        do_add_credential(&state, http_site, Some("alice"), "7#kLp!qZ2r@Vx9W", None, &[], false).unwrap();
        do_add_credential(&state, https_site, Some("alice"), "7#kLp!qZ2r@Vx9W", None, &[], false).unwrap();

        let report = do_health_report(&state).unwrap();

        assert_eq!(report.unsecured.len(), 1);
        assert_eq!(report.unsecured[0].site_name, "insecure.example");
    }

    #[test]
    fn health_report_flags_an_old_password() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let site_id = do_add_site(&state, "example.com", None, None).unwrap();
        let cred_id = do_add_credential(&state, site_id, Some("alice"), "7#kLp!qZ2r@Vx9W", None, &[], false).unwrap();
        {
            let vault_guard = state.vault.lock().unwrap();
            let (conn, _) = vault_guard.as_ref().unwrap();
            conn.execute(
                "UPDATE credentials SET password_updated_at = datetime('now', '-400 days') WHERE id = ?1",
                rusqlite::params![cred_id],
            ).unwrap();
        }

        let report = do_health_report(&state).unwrap();

        assert_eq!(report.old.len(), 1);
        assert_eq!(report.old[0].credential_id, cred_id);
    }

    #[test]
    fn health_report_when_locked_returns_locked_error() {
        let (state, _paths, _dir) = test_state_and_paths();
        assert_eq!(do_health_report(&state).unwrap_err().code(), "locked");
    }

    #[test]
    fn totp_code_is_none_when_no_secret_is_configured() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let site_id = do_add_site(&state, "example.com", None, None).unwrap();
        let cred_id = do_add_credential(&state, site_id, Some("alice"), "s3cret", None, &[], false).unwrap();

        assert_eq!(do_totp_code(&state, cred_id).unwrap(), None);
    }

    #[test]
    fn set_totp_secret_then_totp_code_returns_a_six_digit_code() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let site_id = do_add_site(&state, "example.com", None, None).unwrap();
        let cred_id = do_add_credential(&state, site_id, Some("alice"), "s3cret", None, &[], false).unwrap();

        do_set_totp_secret(&state, cred_id, "JBSWY3DPEHPK3PXP").unwrap();
        let totp = do_totp_code(&state, cred_id).unwrap().unwrap();

        assert_eq!(totp.code.len(), 6);
        assert!(totp.code.chars().all(|c| c.is_ascii_digit()));
        assert!(totp.seconds_remaining >= 1 && totp.seconds_remaining <= 30);
    }

    #[test]
    fn set_totp_secret_accepts_a_padded_messy_secret_and_persists_the_normalized_form() {
        // J2: a padded/spaced/lowercase secret (as pasted from some issuer's setup screen) must
        // both be accepted and stored normalized, so a later read doesn't depend on
        // do_totp_code's own re-normalization to work - do_get_ssh... err, list_ssh_keys aside,
        // reads should see the same clean value that generated the very first code.
        let (state, paths, dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let site_id = do_add_site(&state, "example.com", None, None).unwrap();
        let cred_id = do_add_credential(&state, site_id, Some("alice"), "s3cret", None, &[], false).unwrap();

        do_set_totp_secret(&state, cred_id, " jbsw-y3dp ehpk3pxp=\t\n").unwrap();

        let vault_guard = state.vault.lock().unwrap();
        let (conn, _) = vault_guard.as_ref().unwrap();
        let session = state.session.lock().unwrap();
        let key = session.key().unwrap();
        let stored = crate::vault::get_totp_secret(conn, key, cred_id).unwrap().unwrap();
        assert_eq!(stored, "JBSWY3DPEHPK3PXP");
        drop(session);
        drop(vault_guard);
        drop(dir);
    }

    #[test]
    fn set_totp_secret_rejects_an_invalid_base32_secret() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let site_id = do_add_site(&state, "example.com", None, None).unwrap();
        let cred_id = do_add_credential(&state, site_id, Some("alice"), "s3cret", None, &[], false).unwrap();

        let result = do_set_totp_secret(&state, cred_id, "not valid base32 !!!");

        assert_eq!(result.unwrap_err().code(), "invalid_input");
    }

    #[test]
    fn clear_totp_secret_makes_totp_code_none_again() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let site_id = do_add_site(&state, "example.com", None, None).unwrap();
        let cred_id = do_add_credential(&state, site_id, Some("alice"), "s3cret", None, &[], false).unwrap();
        do_set_totp_secret(&state, cred_id, "JBSWY3DPEHPK3PXP").unwrap();

        do_clear_totp_secret(&state, cred_id).unwrap();

        assert_eq!(do_totp_code(&state, cred_id).unwrap(), None);
    }

    #[test]
    fn totp_commands_require_an_unlocked_session() {
        let (state, _paths, _dir) = test_state_and_paths();
        assert_eq!(do_set_totp_secret(&state, 1, "JBSWY3DPEHPK3PXP").unwrap_err().code(), "locked");
        assert_eq!(do_totp_code(&state, 1).unwrap_err().code(), "locked");
    }

    #[test]
    fn has_pin_is_false_then_true_after_set_pin() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();

        assert_eq!(do_has_pin(&state, "alice").unwrap(), false);

        do_set_pin(&state, "1234").unwrap();

        assert_eq!(do_has_pin(&state, "alice").unwrap(), true);
    }

    #[test]
    fn unlock_with_pin_logs_in_without_the_master_password() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        do_set_pin(&state, "1234").unwrap();
        do_lock(&state);

        let result = do_unlock_with_pin(&state, &paths, "alice", "1234");

        assert!(result.is_ok());
        assert!(do_unlock_status(&state));
    }

    #[test]
    fn unlock_with_pin_rejects_a_wrong_pin_and_leaves_session_locked() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        do_set_pin(&state, "1234").unwrap();
        do_lock(&state);

        let result = do_unlock_with_pin(&state, &paths, "alice", "0000");

        assert_eq!(result.unwrap_err().code(), "invalid_credentials");
        assert!(state.session.lock().unwrap().is_locked());
    }

    #[test]
    fn clear_pin_then_unlock_with_pin_fails() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        do_set_pin(&state, "1234").unwrap();
        do_clear_pin(&state).unwrap();
        do_lock(&state);

        let result = do_unlock_with_pin(&state, &paths, "alice", "1234");

        assert_eq!(result.unwrap_err().code(), "invalid_credentials");
    }

    #[test]
    fn set_pin_and_clear_pin_require_an_unlocked_session() {
        let (state, _paths, _dir) = test_state_and_paths();
        assert_eq!(do_set_pin(&state, "1234").unwrap_err().code(), "locked");
        assert_eq!(do_clear_pin(&state).unwrap_err().code(), "locked");
    }

    #[test]
    fn has_biometric_is_false_when_the_feature_is_disabled_even_if_a_record_exists() {
        use crate::biometric::fake::FakeProvider;
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        do_update_biometric_enabled(&state, &paths, true).unwrap();
        let provider = FakeProvider::new(true, true);
        do_set_biometric(&state, &provider).unwrap();

        do_update_biometric_enabled(&state, &paths, false).unwrap();

        assert_eq!(do_has_biometric(&state, "alice", &provider), false);
    }

    #[test]
    fn set_biometric_requires_the_feature_enabled_and_a_successful_verify() {
        use crate::biometric::fake::FakeProvider;
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();

        // Feature not yet enabled in Settings.
        let provider = FakeProvider::new(true, true);
        assert_eq!(do_set_biometric(&state, &provider).unwrap_err().code(), "invalid_input");

        do_update_biometric_enabled(&state, &paths, true).unwrap();

        // User cancels/fails the OS prompt.
        let cancelling_provider = FakeProvider::new(true, false);
        assert_eq!(
            do_set_biometric(&state, &cancelling_provider).unwrap_err().code(),
            "invalid_credentials"
        );

        do_set_biometric(&state, &provider).unwrap();
        assert!(do_has_biometric(&state, "alice", &provider));
    }

    #[test]
    fn unlock_with_biometric_recovers_the_session_without_the_master_password() {
        use crate::biometric::fake::FakeProvider;
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        do_update_biometric_enabled(&state, &paths, true).unwrap();
        let provider = FakeProvider::new(true, true);
        do_set_biometric(&state, &provider).unwrap();
        do_lock(&state);

        let result = do_unlock_with_biometric(&state, &paths, "alice", &provider);

        assert!(result.is_ok());
        assert!(do_unlock_status(&state));
    }

    #[test]
    fn unlock_with_biometric_fails_when_verification_is_cancelled() {
        use crate::biometric::fake::FakeProvider;
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        do_update_biometric_enabled(&state, &paths, true).unwrap();
        let setup_provider = FakeProvider::new(true, true);
        do_set_biometric(&state, &setup_provider).unwrap();
        do_lock(&state);

        let cancelling_provider = FakeProvider::new(true, false);
        let result = do_unlock_with_biometric(&state, &paths, "alice", &cancelling_provider);

        assert_eq!(result.unwrap_err().code(), "invalid_credentials");
        assert!(state.session.lock().unwrap().is_locked());
    }

    #[test]
    fn unlock_with_biometric_rejects_when_no_record_is_configured() {
        use crate::biometric::fake::FakeProvider;
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        do_update_biometric_enabled(&state, &paths, true).unwrap();
        do_lock(&state);

        let provider = FakeProvider::new(true, true);
        let result = do_unlock_with_biometric(&state, &paths, "alice", &provider);

        assert_eq!(result.unwrap_err().code(), "invalid_credentials");
    }

    #[test]
    fn clear_biometric_then_unlock_with_biometric_fails() {
        use crate::biometric::fake::FakeProvider;
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        do_update_biometric_enabled(&state, &paths, true).unwrap();
        let provider = FakeProvider::new(true, true);
        do_set_biometric(&state, &provider).unwrap();

        do_clear_biometric(&state).unwrap();
        do_lock(&state);

        let result = do_unlock_with_biometric(&state, &paths, "alice", &provider);
        assert_eq!(result.unwrap_err().code(), "invalid_credentials");
    }

    #[test]
    fn check_breach_rejects_when_hibp_opt_in_is_false() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();

        let result = do_check_breach(&state, "http://127.0.0.1:1"); // unreachable - must reject first

        assert_eq!(result.unwrap_err().code(), "invalid_input");
    }

    #[test]
    fn check_breach_when_locked_returns_locked_error() {
        let (state, paths, _dir) = test_state_and_paths();
        do_update_hibp_opt_in(&state, &paths, true).unwrap();

        let result = do_check_breach(&state, "http://127.0.0.1:1");

        assert_eq!(result.unwrap_err().code(), "locked");
    }

    #[test]
    fn check_breach_finds_a_breached_credential_via_a_mock_server() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        do_update_hibp_opt_in(&state, &paths, true).unwrap();
        let site_id = do_add_site(&state, "example.com", None, None).unwrap();
        do_add_credential(&state, site_id, Some("alice"), "password", None, &[], false).unwrap();

        let full_hash = crate::hibp::sha1_hex_upper("password");
        let suffix = &full_hash[5..];
        let body = format!("{}:999\r\n", suffix);
        let base_url = crate::hibp::mock_hibp_server(body);

        let report = do_check_breach(&state, &base_url).unwrap();

        assert_eq!(report.breached.len(), 1);
    }

    fn sample_ed25519_openssh_key() -> String {
        use ssh_key::{Algorithm, PrivateKey};
        let key = PrivateKey::random(&mut rand::rngs::OsRng, Algorithm::Ed25519).unwrap();
        key.to_openssh(ssh_key::LineEnding::LF).unwrap().to_string()
    }

    #[test]
    fn add_ssh_key_then_list_ssh_keys_exposes_label_and_public_key_but_never_the_private_key() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let openssh = sample_ed25519_openssh_key();

        do_add_ssh_key(&state, "laptop key", &openssh).unwrap();

        let keys = do_list_ssh_keys(&state).unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].label, "laptop key");
        assert!(keys[0].public_key.starts_with("ssh-ed25519 "));
    }

    #[test]
    fn add_ssh_key_rejects_a_key_that_does_not_parse() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();

        let result = do_add_ssh_key(&state, "bad", "not an ssh key");

        assert_eq!(result.unwrap_err().code(), "invalid_input");
    }

    #[test]
    fn delete_ssh_key_removes_it_from_the_list() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let id = do_add_ssh_key(&state, "temp", &sample_ed25519_openssh_key()).unwrap();

        do_delete_ssh_key(&state, id).unwrap();

        assert!(do_list_ssh_keys(&state).unwrap().is_empty());
    }

    #[test]
    fn ssh_key_commands_require_an_unlocked_session() {
        let (state, _paths, _dir) = test_state_and_paths();
        assert_eq!(do_add_ssh_key(&state, "x", &sample_ed25519_openssh_key()).unwrap_err().code(), "locked");
        assert_eq!(do_list_ssh_keys(&state).unwrap_err().code(), "locked");
    }

    #[test]
    fn update_ssh_agent_enabled_persists() {
        let (state, paths, _dir) = test_state_and_paths();
        do_update_ssh_agent_enabled(&state, &paths, true).unwrap();
        assert_eq!(do_get_config(&state).ssh_agent_enabled, true);
        let reloaded = Config::load(&paths.config_path()).unwrap();
        assert_eq!(reloaded.ssh_agent_enabled, true);
    }

    #[test]
    fn update_ui_variant_persists_and_rejects_invalid_value() {
        let (state, paths, _dir) = test_state_and_paths();
        do_update_ui_variant(&state, &paths, "new").unwrap();
        assert_eq!(do_get_config(&state).ui_variant, "new");
        do_update_ui_variant(&state, &paths, "make").unwrap();
        assert_eq!(do_get_config(&state).ui_variant, "make");
        assert_eq!(do_update_ui_variant(&state, &paths, "figma-preview").unwrap_err().code(), "invalid_input");
    }

    #[test]
    fn get_config_includes_ui_variant_default() {
        let (state, _paths, _dir) = test_state_and_paths();
        assert_eq!(do_get_config(&state).ui_variant, "classic");
    }

    #[test]
    fn update_ui_variant_does_not_mutate_state_when_save_fails() {
        let (state, _paths, dir) = test_state_and_paths();
        let bad_paths = AppPaths::new(dir.path().join("nonexistent-subdir-xyz"));
        let before = do_get_config(&state).ui_variant;
        let result = do_update_ui_variant(&state, &bad_paths, "new");
        assert!(result.is_err());
        assert_eq!(do_get_config(&state).ui_variant, before, "state.config must not change when the save fails");
    }

    #[test]
    fn update_delete_reprompt_persists_and_rejects_unknown_action() {
        let (state, paths, _dir) = test_state_and_paths();
        assert!(do_get_config(&state).delete_reprompt_folder);
        do_update_delete_reprompt(&state, &paths, "deleteFolder", false).unwrap();
        assert!(!do_get_config(&state).delete_reprompt_folder);
        assert!(do_get_config(&state).delete_reprompt_empty_trash);
        assert!(!crate::config::Config::load(&paths.config_path()).unwrap().delete_reprompt_folder);
        assert_eq!(do_update_delete_reprompt(&state, &paths, "bogus", true).unwrap_err().code(), "invalid_input");
    }

    #[test]
    fn update_reprompt_mode_persists_and_rejects_invalid_value() {
        let (state, paths, _dir) = test_state_and_paths();
        do_update_reprompt_mode(&state, &paths, "always").unwrap();
        assert_eq!(do_get_config(&state).reprompt_mode, "always");
        assert_eq!(do_update_reprompt_mode(&state, &paths, "perCredential").unwrap_err().code(), "invalid_input");
    }

    #[test]
    fn update_reprompt_grace_seconds_persists_and_rejects_invalid_value() {
        let (state, paths, _dir) = test_state_and_paths();
        do_update_reprompt_grace_seconds(&state, &paths, 60).unwrap();
        assert_eq!(do_get_config(&state).reprompt_grace_seconds, 60);
        assert_eq!(do_update_reprompt_grace_seconds(&state, &paths, 7).unwrap_err().code(), "invalid_input");
    }

    #[test]
    fn get_config_includes_reprompt_defaults() {
        let (state, _paths, _dir) = test_state_and_paths();
        let config = do_get_config(&state);
        assert_eq!(config.reprompt_mode, "grace");
        assert_eq!(config.reprompt_grace_seconds, 30);
    }

    #[test]
    fn update_reprompt_mode_does_not_mutate_state_when_save_fails() {
        let (state, _paths, dir) = test_state_and_paths();
        let bad_paths = AppPaths::new(dir.path().join("nonexistent-subdir-xyz"));
        let before = do_get_config(&state).reprompt_mode;
        let result = do_update_reprompt_mode(&state, &bad_paths, "always");
        assert!(result.is_err());
        assert_eq!(do_get_config(&state).reprompt_mode, before, "state.config must not change when the save fails");
    }

    #[test]
    fn make_ui_item_commands_require_unlock_and_roundtrip_through_dtos() {
        let (state, paths, _dir) = test_state_and_paths();
        assert!(format!("{:?}", do_list_folders(&state).unwrap_err()).contains("locked"));
        assert!(format!("{:?}", do_set_site_favorite(&state, 1, true).unwrap_err()).contains("locked"));

        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let site = do_add_site(&state, "Chase", None, None).unwrap();
        let folder = do_add_folder(&state, "Finance").unwrap();

        do_set_site_favorite(&state, site, true).unwrap();
        do_set_site_item_type(&state, site, "card").unwrap();
        do_set_site_card_hint(&state, site, Some("Visa"), Some("3921")).unwrap();
        do_set_site_folder(&state, site, Some(folder)).unwrap();

        let s = &do_list_sites(&state).unwrap()[0];
        assert!(s.favorite);
        assert_eq!(s.item_type, "card");
        assert_eq!(s.folder_id, Some(folder));
        assert_eq!(s.card_last4.as_deref(), Some("3921"));
        assert_eq!(do_list_folders(&state).unwrap()[0].name, "Finance");

        do_rename_folder(&state, folder, "Money").unwrap();
        assert_eq!(do_list_folders(&state).unwrap()[0].name, "Money");
        do_delete_folder(&state, folder).unwrap();
        assert!(do_list_folders(&state).unwrap().is_empty());
        assert_eq!(do_list_sites(&state).unwrap()[0].folder_id, None);
    }

    #[test]
    fn make_ui_item_commands_surface_invalid_input_and_not_found_codes() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let site = do_add_site(&state, "a.com", None, None).unwrap();

        assert_eq!(do_set_site_item_type(&state, site, "passport").unwrap_err().code(), "invalid_input");
        assert_eq!(do_set_site_card_hint(&state, site, None, Some("12")).unwrap_err().code(), "invalid_input");
        assert_eq!(do_add_folder(&state, " ").unwrap_err().code(), "invalid_input");
        assert_eq!(do_set_site_favorite(&state, 999, true).unwrap_err().code(), "not_found");
        assert_eq!(do_delete_folder(&state, 999).unwrap_err().code(), "not_found");
    }
}
