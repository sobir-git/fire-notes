//! Key event conversion from winit to application key types.

use crate::app::{Key as AppKey, KeyEvent, Modifiers};
use winit::keyboard::{Key, ModifiersState, NamedKey};

/// Convert a [`KeyEvent`] (already decoded from winit) to a [`TextInputEvent`].
///
/// Returns `None` for keys that text inputs don't handle (arrows up/down,
/// Page Up/Down, Enter, Tab, …). The caller keeps those for its own logic.
///
/// Clipboard paste (`Ctrl+V`) returns `None` because the caller must read the
/// clipboard first; it should then call `TextInput::handle_key(Paste(text))`.
pub fn key_event_to_text_input(ev: &KeyEvent) -> Option<crate::primitives::text_input::TextInputEvent<'static>> {
    use crate::primitives::text_input::{MoveBy, MoveDir, TextInputEvent};
    let ctrl  = ev.modifiers.ctrl;
    let shift = ev.modifiers.shift;
    Some(match &ev.key {
        AppKey::Backspace  => TextInputEvent::Backspace { word: ctrl },
        AppKey::Delete     => TextInputEvent::Delete    { word: ctrl },
        AppKey::ArrowLeft  => TextInputEvent::Move { dir: MoveDir::Left,  by: if ctrl { MoveBy::Word } else { MoveBy::Char }, selecting: shift },
        AppKey::ArrowRight => TextInputEvent::Move { dir: MoveDir::Right, by: if ctrl { MoveBy::Word } else { MoveBy::Char }, selecting: shift },
        AppKey::Home       => TextInputEvent::Move { dir: MoveDir::Start, by: MoveBy::Char, selecting: shift },
        AppKey::End        => TextInputEvent::Move { dir: MoveDir::End,   by: MoveBy::Char, selecting: shift },
        AppKey::Escape     => TextInputEvent::Escape,
        AppKey::Space      => TextInputEvent::Char(' '),
        AppKey::Char(ch) if ctrl => match ch.to_ascii_lowercase() {
            'a' => TextInputEvent::SelectAll,
            'c' => TextInputEvent::Copy,
            'x' => TextInputEvent::Cut,
            _ => return None,
        },
        AppKey::Char(ch) if !ctrl => TextInputEvent::Char(*ch),
        _ => return None,
    })
}

/// Convert a winit Key to our KeyEvent.
pub fn convert_winit_key(key: &Key, modifiers: &ModifiersState) -> Option<KeyEvent> {
    let mods = Modifiers {
        ctrl: modifiers.control_key(),
        shift: modifiers.shift_key(),
        alt: modifiers.alt_key(),
    };

    let app_key = match key {
        Key::Named(NamedKey::Escape) => AppKey::Escape,
        Key::Named(NamedKey::Enter) => AppKey::Enter,
        Key::Named(NamedKey::Tab) => AppKey::Tab,
        Key::Named(NamedKey::Backspace) => AppKey::Backspace,
        Key::Named(NamedKey::Delete) => AppKey::Delete,
        Key::Named(NamedKey::ArrowLeft) => AppKey::ArrowLeft,
        Key::Named(NamedKey::ArrowRight) => AppKey::ArrowRight,
        Key::Named(NamedKey::ArrowUp) => AppKey::ArrowUp,
        Key::Named(NamedKey::ArrowDown) => AppKey::ArrowDown,
        Key::Named(NamedKey::Home) => AppKey::Home,
        Key::Named(NamedKey::End) => AppKey::End,
        Key::Named(NamedKey::PageUp) => AppKey::PageUp,
        Key::Named(NamedKey::PageDown) => AppKey::PageDown,
        Key::Named(NamedKey::Space) => AppKey::Space,
        Key::Character(c) => {
            let ch = c.chars().next()?;
            AppKey::Char(ch)
        }
        _ => return None,
    };

    Some(KeyEvent::new(app_key, mods))
}
