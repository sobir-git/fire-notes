//! Handles `WindowEvent::DroppedFile` — opens the dropped file in a new or existing tab.

use std::path::PathBuf;

use super::window::AppState;

pub(super) fn handle_dropped_file(state: &mut AppState, path: PathBuf) {
    let result = state.app.logic.open_or_switch_to(path);
    if result.needs_redraw() { state.window.request_redraw(); }
}
