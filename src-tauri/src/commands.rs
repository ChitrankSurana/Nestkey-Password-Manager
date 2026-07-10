use crate::state::AppState;
use tauri::State;

/// Smoke-test command for Slice 0: proves the IPC round trip (state access, Mutex lock, DTO
/// return) works end-to-end before Slice 1 adds real auth commands on the same pattern.
#[tauri::command]
pub fn unlock_status(state: State<AppState>) -> bool {
    !state.session.lock().unwrap().is_locked()
}
