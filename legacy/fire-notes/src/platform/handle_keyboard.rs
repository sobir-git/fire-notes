//! Handles `WindowEvent::KeyboardInput` — converts winit key events to Actions.

use winit::event::ElementState;
use winit::keyboard::ModifiersState;

use super::window::AppState;
use super::keys::convert_winit_key;

pub(super) fn handle_keyboard(
    state: &mut AppState,
    event: &winit::event::KeyEvent,
    is_synthetic: bool,
    modifiers: &ModifiersState,
) {
    if is_synthetic { return; }
    if event.state != ElementState::Pressed { return; }
    let key_event = convert_winit_key(&event.logical_key, modifiers);
    if let Some(key_event) = key_event {
        if let Some(action) = crate::app::resolve_keybinding(&key_event) {
            let result = state.app.execute(action);
            if result.needs_redraw() { state.window.request_redraw(); }
        }
    }
}
