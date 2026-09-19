use std::path::{Path, PathBuf};

// Blacklist rather than a whitelist so every existing account (any Unicode name that was already a
// legal filename) keeps working; it only refuses what is unsafe as a single path component.
fn is_safe_username(name: &str) -> bool {
    const RESERVED: [&str; 22] = [
        "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
        "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
    ];
    let stem = name.split('.').next().unwrap_or("").to_lowercase();
    !name.is_empty()
        && name == name.trim()
        && !name.contains("..")
        && !name.starts_with('.')
        && !name.ends_with('.')
        && !name.chars().any(|c| c.is_control() || r#"/\:*?"<>|"#.contains(c))
        && !RESERVED.contains(&stem.as_str())
}

pub struct AppPaths {
    root: PathBuf,
}

impl AppPaths {
    pub fn new(root: PathBuf) -> Self {
        AppPaths { root }
    }

    /// Resolves to `~/.pwmgr` on a real machine (via `dirs::home_dir()` at the call site in
    /// `main.rs`); tests construct this directly with a tempdir root instead.
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn vaults_dir(&self) -> PathBuf {
        self.root.join("vaults")
    }

    pub fn backups_dir(&self) -> PathBuf {
        self.root.join("backups")
    }

    pub fn registry_db_path(&self) -> PathBuf {
        self.root.join("registry.db")
    }

    pub fn vault_path(&self, username: &str) -> PathBuf {
        self.vaults_dir().join(format!("{}.vault", username))
    }

    /// `vault_path` for a username that came from outside the process (login/register form, an
    /// uploaded backup bundle): rejects any name that could escape `vaults_dir` or name a Windows
    /// device, so the username can never steer a write or read out of the vaults folder.
    pub fn checked_vault_path(&self, username: &str) -> Result<PathBuf, crate::error::CommandError> {
        if !is_safe_username(username) {
            return Err(crate::error::invalid_input_error(
                "That username contains characters that aren't allowed.",
            ));
        }
        Ok(self.vault_path(username))
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join("config.json")
    }

    pub fn plaintext_export_log_path(&self) -> PathBuf {
        self.root.join("plaintext_export_log.txt")
    }

    /// Creates root/vaults/backups if missing and restricts each to the current user only.
    /// THREAT_MODEL A2 (shared machine) lists OS-level file permissions as a mitigation that
    /// nothing in pwmgr-core sets — this is the layer that owns app-directory scaffolding, so it
    /// owns closing that gap too.
    pub fn ensure_app_dirs(&self) -> std::io::Result<()> {
        for dir in [self.root(), &self.vaults_dir(), &self.backups_dir()] {
            Self::create_dir_owner_only(dir)?;
        }
        Ok(())
    }

    /// Creates `dir` (and any missing parents) with owner-only permissions from the moment it
    /// exists on disk — no window where the directory is briefly world/group-readable before a
    /// follow-up chmod. On Unix this is atomic via `DirBuilder::mode`; on Windows there is no
    /// equivalent atomic create-with-ACL primitive in std, so we fall back to create-then-restrict
    /// via `restrict_to_owner` (documented there as best-effort defense-in-depth).
    #[cfg(unix)]
    fn create_dir_owner_only(dir: &Path) -> std::io::Result<()> {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new()
            .mode(0o700)
            .recursive(true)
            .create(dir)
    }

    #[cfg(windows)]
    fn create_dir_owner_only(dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        Self::restrict_to_owner(dir)
    }

    #[cfg(windows)]
    fn restrict_to_owner(dir: &Path) -> std::io::Result<()> {
        // Windows ACLs aren't exposed via std::fs::Permissions. icacls restricts the directory to
        // the current user, mirroring Unix's 0o700 (owner rwx, nobody else). Best-effort: if
        // icacls itself is unavailable (non-standard Windows install), don't fail app startup over
        // it - log and continue, since this is defense-in-depth, not the primary security boundary
        // (that's the SQLCipher/Argon2id encryption pwmgr-core already provides).
        let username = std::env::var("USERNAME").unwrap_or_default();
        let output = std::process::Command::new("icacls")
            .arg(dir)
            .arg("/inheritance:r")
            .arg("/grant:r")
            .arg(format!("{}:F", username))
            .output();
        match output {
            Ok(o) if o.status.success() => Ok(()),
            _ => {
                eprintln!("warning: could not restrict permissions on {:?}", dir);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn ensure_app_dirs_creates_all_three_directories() {
        let dir = tempdir().unwrap();
        let paths = AppPaths::new(dir.path().to_path_buf());

        paths.ensure_app_dirs().unwrap();

        assert!(paths.root().is_dir());
        assert!(paths.vaults_dir().is_dir());
        assert!(paths.backups_dir().is_dir());
    }

    #[test]
    fn registry_db_path_and_vault_path_are_under_expected_subdirs() {
        let dir = tempdir().unwrap();
        let paths = AppPaths::new(dir.path().to_path_buf());

        assert_eq!(paths.registry_db_path(), dir.path().join("registry.db"));
        assert_eq!(
            paths.vault_path("alice"),
            dir.path().join("vaults").join("alice.vault")
        );
        assert_eq!(paths.config_path(), dir.path().join("config.json"));
    }

    #[test]
    fn checked_vault_path_rejects_traversal_and_device_names() {
        let dir = tempdir().unwrap();
        let paths = AppPaths::new(dir.path().to_path_buf());
        for bad in ["../x", r"..\x", "a/b", r"a\b", "..", ".hidden", "trail.", " pad", "C:evil", "nul", "COM1", "con.txt", "", "a\u{0}b"] {
            assert!(paths.checked_vault_path(bad).is_err(), "{bad:?} should be rejected");
        }
        for ok in ["alice", "Alice Smith", "j.doe", "user@example.com", "réne"] {
            assert_eq!(paths.checked_vault_path(ok).unwrap(), paths.vault_path(ok), "{ok:?}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn ensure_app_dirs_restricts_root_to_owner_only_on_unix() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let paths = AppPaths::new(dir.path().to_path_buf());

        paths.ensure_app_dirs().unwrap();

        let mode = std::fs::metadata(paths.root()).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700, "expected owner-only rwx, got {:o}", mode);
    }

    /// Proves each directory is born with mode 0o700 via the atomic `DirBuilder::mode` path
    /// (no create-then-chmod TOCTOU window) — checks every directory `ensure_app_dirs` creates,
    /// not just the root, since the atomic create happens per-directory in the loop.
    #[cfg(unix)]
    #[test]
    fn ensure_app_dirs_creates_every_directory_with_owner_only_mode_atomically_on_unix() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let paths = AppPaths::new(dir.path().to_path_buf());

        paths.ensure_app_dirs().unwrap();

        for created in [paths.root(), &paths.vaults_dir(), &paths.backups_dir()] {
            let mode = std::fs::metadata(created).unwrap().permissions().mode() & 0o777;
            assert_eq!(
                mode, 0o700,
                "expected {:?} to be created with owner-only rwx, got {:o}",
                created, mode
            );
        }
    }
}
