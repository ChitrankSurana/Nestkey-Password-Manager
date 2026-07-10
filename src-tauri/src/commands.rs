use crate::state::AppState;
use tauri::State;

/// Smoke-test command for Slice 0: proves the IPC round trip (state access, Mutex lock, DTO
/// return) works end-to-end before Slice 1 adds real auth commands on the same pattern.
#[tauri::command]
pub fn unlock_status(state: State<AppState>) -> bool {
    unlock_status_inner(&state)
}

fn unlock_status_inner(state: &AppState) -> bool {
    !state.session.lock().unwrap().is_locked()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::time::Duration;

    #[test]
    fn unlock_status_inner_reflects_locked_session() {
        let state = AppState::new(Config::default(), Duration::from_secs(300));
        assert!(!unlock_status_inner(&state));
    }
}
