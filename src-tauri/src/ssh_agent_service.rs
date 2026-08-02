//! Windows named-pipe SSH agent. Wraps `ssh-agent-lib`'s wire-protocol server: this module only
//! supplies the `Session` implementation (list stored keys, sign on request behind a native
//! confirmation dialog) and start/stop management for the Settings on/off toggle.
//!
//! Real interop with an actual OpenSSH/PuTTY client is untested from this crate's automated build
//! (no way to drive a real SSH client against a live named pipe here) - verify manually after this
//! lands. Compiles to a no-op stub on non-Windows targets.

use tauri::AppHandle;

/// Holds the currently-running listener task (if any), so the Settings toggle can start/stop it
/// live instead of requiring an app restart. Managed as Tauri state.
pub struct SshAgentHandle(std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>);

impl Default for SshAgentHandle {
    fn default() -> Self {
        SshAgentHandle(std::sync::Mutex::new(None))
    }
}

/// Starts the agent listener if it isn't already running. No-op (and never errors) if unsupported
/// on this platform.
pub fn start(app: &AppHandle) {
    #[cfg(target_os = "windows")]
    {
        use tauri::Manager;
        let handle_state = app.state::<SshAgentHandle>();
        let mut guard = handle_state.0.lock().unwrap();
        if guard.is_some() {
            return;
        }
        *guard = Some(imp::spawn(app.clone()));
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
    }
}

/// Stops the agent listener if running.
pub fn stop(app: &AppHandle) {
    use tauri::Manager;
    let handle_state = app.state::<SshAgentHandle>();
    let mut guard = handle_state.0.lock().unwrap();
    if let Some(task) = guard.take() {
        task.abort();
    }
}

#[cfg(target_os = "windows")]
mod imp {
    use pwmgr_core::state::AppState;
    use ssh_agent_lib::agent::{listen, NamedPipeListener, Session};
    use ssh_agent_lib::error::AgentError;
    use ssh_agent_lib::proto::{Identity, PublicCredential, SignRequest};
    use ssh_key::{PrivateKey, Signature};
    use tauri::{AppHandle, Manager};
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

    const PIPE_NAME: &str = r"\\.\pipe\pwmgr-ssh-agent";

    #[derive(Clone)]
    struct PwmgrSshSession {
        handle: AppHandle,
    }

    // Decrypts every stored key - only possible while the vault is unlocked; returns an empty list
    // otherwise (the agent simply has nothing to offer, rather than erroring the whole request).
    fn decrypted_keys(handle: &AppHandle) -> Vec<pwmgr_core::ssh_keys::SshKeyEntry> {
        let state = handle.state::<AppState>();
        let vault_guard = state.vault.lock().unwrap();
        let Some((conn, _)) = vault_guard.as_ref() else {
            return Vec::new();
        };
        let session = state.session.lock().unwrap();
        let Some(key) = session.key() else {
            return Vec::new();
        };
        pwmgr_core::ssh_keys::list_ssh_keys(conn, key).unwrap_or_default()
    }

    #[ssh_agent_lib::async_trait]
    impl Session for PwmgrSshSession {
        async fn request_identities(&mut self) -> Result<Vec<Identity>, AgentError> {
            let handle = self.handle.clone();
            let entries = tokio::task::spawn_blocking(move || decrypted_keys(&handle))
                .await
                .unwrap_or_default();
            let mut out = Vec::new();
            for e in entries {
                if let Ok(private) = PrivateKey::from_openssh(&e.private_key_openssh) {
                    out.push(Identity {
                        credential: PublicCredential::Key(private.public_key().key_data().clone()),
                        comment: e.label,
                    });
                }
            }
            Ok(out)
        }

        async fn sign(&mut self, request: SignRequest) -> Result<Signature, AgentError> {
            let handle = self.handle.clone();
            let entries = tokio::task::spawn_blocking(move || decrypted_keys(&handle))
                .await
                .map_err(AgentError::other)?;

            let matching = entries.into_iter().find(|e| {
                PrivateKey::from_openssh(&e.private_key_openssh)
                    .map(|k| k.public_key().key_data() == request.credential.key_data())
                    .unwrap_or(false)
            });
            let Some(entry) = matching else {
                return Err(AgentError::Failure);
            };

            let handle = self.handle.clone();
            let label = entry.label.clone();
            let approved = tokio::task::spawn_blocking(move || {
                handle
                    .dialog()
                    .message(format!("An application is requesting to sign with SSH key '{label}'. Allow?"))
                    .title("SSH Agent")
                    .buttons(MessageDialogButtons::YesNo)
                    .kind(MessageDialogKind::Info)
                    .blocking_show()
            })
            .await
            .unwrap_or(false);
            if !approved {
                return Err(AgentError::Failure);
            }

            pwmgr_core::ssh_agent::sign(&entry.private_key_openssh, &request.data).map_err(AgentError::other)
        }
    }

    pub fn spawn(handle: AppHandle) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            match NamedPipeListener::bind(PIPE_NAME) {
                Ok(listener) => {
                    if let Err(e) = listen(listener, PwmgrSshSession { handle }).await {
                        eprintln!("warning: ssh-agent listener stopped: {e}");
                    }
                }
                Err(e) => eprintln!("warning: failed to bind ssh-agent named pipe: {e}"),
            }
        })
    }
}
