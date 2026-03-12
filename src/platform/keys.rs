//! Key event conversion from winit to application key types.

use crate::app::{Key as AppKey, KeyEvent, Modifiers};
use winit::keyboard::{Key, ModifiersState, NamedKey};

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
