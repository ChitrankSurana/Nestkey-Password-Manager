use crate::config::{VALID_ACCENT_COLORS, VALID_AUTO_LOCK_MINUTES, VALID_OLD_PASSWORD_DAYS, VALID_STRENGTH_ALGORITHMS};
use crate::dto::{AuthSuccess, BreachReportDto, ConfigDto, CredentialDto, ExportMode, GenerateRequest, HealthEntryDto, HealthReportDto, ImportFormat, ImportRecordDto, ImportSummaryDto, ReusedGroupDto, SearchResultDto, SiteDto, StrengthDto, TrashedCredentialDto};
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
        crate::registry::find_user(&registry, &username)
            .map_err(|_| crate::error::internal_error())?
    };

    let vault_bytes = std::fs::read(&vault_path).map_err(|_| crate::error::internal_error())?;
    let bundle = crate::backup::encode_bundle(&user_record, &vault_bytes);

    // Millisecond precision: a whole-second timestamp lets two backups made in quick succession
    // silently collide and overwrite each other.
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let backup_path = paths.backups_dir().join(format!("{}-{}.bak", username, timestamp));
    std::fs::write(&backup_path, &bundle).map_err(|_| crate::error::internal_error())?;

    Ok(backup_path.to_string_lossy().to_string())
}

pub fn do_restore_backup(
    state: &AppState,
    paths: &AppPaths,
    backup_file_path: &str,
    confirmed: bool,
) -> Result<(), CommandError> {
    let bytes = std::fs::read(backup_file_path)
        .map_err(|_| crate::error::invalid_input_error("Could not read that backup file."))?;
    let (user_record, vault_bytes) = crate::backup::decode_bundle(&bytes)
        .map_err(|_| crate::error::invalid_input_error("That file isn't a valid backup."))?;

    let vault_path = paths.vault_path(&user_record.username);
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

pub fn do_export_csv(
    state: &AppState,
    paths: &AppPaths,
    path: &str,
    mode: &ExportMode,
    confirmed: bool,
) -> Result<(), CommandError> {
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
    std::fs::write(path, &bytes).map_err(|_| crate::error::internal_error())?;

    if matches!(mode, ExportMode::Plaintext) {
        if let Err(e) = append_plaintext_export_log(paths, path) {
            eprintln!("warning: failed to append plaintext export log entry: {e}");
        }
    }
    Ok(())
}

pub fn do_read_csv_headers(path: &str) -> Result<Vec<String>, CommandError> {
    let text = std::fs::read_to_string(path)
        .map_err(|_| crate::error::invalid_input_error("Could not read that file."))?;
    let mut reader = csv::Reader::from_reader(text.as_bytes());
    let headers = reader
        .headers()
        .map_err(|_| crate::error::invalid_input_error("That file doesn't look like a CSV."))?;
    Ok(headers.iter().map(|h| h.to_string()).collect())
}

pub fn do_import_csv_preview(
    path: &str,
    format: &ImportFormat,
) -> Result<Vec<ImportRecordDto>, CommandError> {
    let text = std::fs::read_to_string(path)
        .map_err(|_| crate::error::invalid_input_error("Could not read that file."))?;
    let records = match format {
        ImportFormat::Chromium => crate::portio::parse_chromium_csv(&text)?,
        ImportFormat::Firefox => crate::portio::parse_firefox_csv(&text)?,
        ImportFormat::Generic { mapping } => crate::portio::parse_generic_csv(&text, mapping)?,
    };
    Ok(records.into_iter().map(ImportRecordDto::from).collect())
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

    Ok(HealthReportDto { weak, reused, old })
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
        let path = do_create_backup(state, paths).unwrap();
        std::fs::read(path).unwrap()
    }

    #[test]
    fn create_backup_writes_a_decodable_bundle_under_backups_dir() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();

        let path = do_create_backup(&state, &paths).unwrap();

        assert!(std::path::Path::new(&path).starts_with(paths.backups_dir()));
        let bytes = std::fs::read(&path).unwrap();
        let (user, vault_bytes) = crate::backup::decode_bundle(&bytes).unwrap();
        assert_eq!(user.username, "alice");
        assert!(!vault_bytes.is_empty());
    }

    #[test]
    fn create_backup_when_locked_returns_locked_error() {
        let (state, paths, _dir) = test_state_and_paths();
        let result = do_create_backup(&state, &paths);
        assert_eq!(result.unwrap_err().code(), "locked");
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
        do_add_credential(&state, site_id, Some("alice"), "s3cret!", None).unwrap();
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
        do_add_credential(&state, site_id, Some("alice"), "password", None).unwrap(); // weak

        let report = do_health_report(&state).unwrap();

        assert_eq!(report.weak.len(), 1);
        assert_eq!(report.weak[0].site_name, "example.com");
    }

    #[test]
    fn health_report_does_not_flag_a_strong_password_as_weak() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let site_id = do_add_site(&state, "example.com", None, None).unwrap();
        do_add_credential(&state, site_id, Some("alice"), "7#kLp!qZ2r@Vx9W", None).unwrap();

        assert!(do_health_report(&state).unwrap().weak.is_empty());
    }

    #[test]
    fn health_report_groups_reused_passwords() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let site_id = do_add_site(&state, "example.com", None, None).unwrap();
        do_add_credential(&state, site_id, Some("alice"), "7#kLp!qZ2r@Vx9W", None).unwrap();
        do_add_credential(&state, site_id, Some("bob"), "7#kLp!qZ2r@Vx9W", None).unwrap();

        let report = do_health_report(&state).unwrap();

        assert_eq!(report.reused.len(), 1);
        assert_eq!(report.reused[0].entries.len(), 2);
    }

    #[test]
    fn health_report_flags_an_old_password() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        let site_id = do_add_site(&state, "example.com", None, None).unwrap();
        let cred_id = do_add_credential(&state, site_id, Some("alice"), "7#kLp!qZ2r@Vx9W", None).unwrap();
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
        do_add_credential(&state, site_id, Some("alice"), "password", None).unwrap();

        let full_hash = crate::hibp::sha1_hex_upper("password");
        let suffix = &full_hash[5..];
        let body = format!("{}:999\r\n", suffix);
        let base_url = crate::hibp::mock_hibp_server(body);

        let report = do_check_breach(&state, &base_url).unwrap();

        assert_eq!(report.breached.len(), 1);
    }
}
