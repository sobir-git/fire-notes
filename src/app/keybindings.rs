//! Keyboard shortcut registry
//!
//! Maps keyboard input to Actions. All keybindings are defined in one place,
//! making it easy to:
//! - See all shortcuts at a glance
//! - Add new shortcuts
//! - Eventually support user-customizable keybindings
//!
//! The matching uses a priority system: more specific bindings (with more
//! modifiers) are checked first.

use super::action::Action;

/// Modifier key state
#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Modifiers {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn ctrl() -> Self {
        Self { ctrl: true, ..Self::default() }
    }

    pub fn shift() -> Self {
        Self { shift: true, ..Self::default() }
    }

    pub fn alt() -> Self {
        Self { alt: true, ..Self::default() }
    }

    pub fn ctrl_shift() -> Self {
        Self { ctrl: true, shift: true, ..Self::default() }
    }
}

/// Represents a key that can be pressed
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Escape,
    Enter,
    Tab,
    Backspace,
    Delete,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Home,
    End,
    PageUp,
    PageDown,
    Space,
}

/// A keyboard input event (key + modifiers)
#[derive(Debug, Clone)]
pub struct KeyEvent {
    pub key: Key,
    pub modifiers: Modifiers,
}

impl KeyEvent {
    pub fn new(key: Key, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }
}

/// Resolve a key event to an action
/// 
/// This is the single source of truth for all keyboard shortcuts.
/// Returns None if the key event doesn't map to any action.
pub fn resolve(event: &KeyEvent) -> Option<Action> {
    let KeyEvent { key, modifiers } = event;
    let Modifiers { ctrl, shift, alt } = *modifiers;

    // Match in order of specificity (most modifiers first)
    match key {
        // =================================================================
        // Escape - Cancel current operation
        // =================================================================
        Key::Escape => Some(Action::Cancel),

        // =================================================================
        // Enter - Confirm or insert newline
        // =================================================================
        Key::Enter => Some(Action::Confirm),

        // =================================================================
        // Tab navigation (Ctrl+Tab, Ctrl+Shift+Tab)
        // =================================================================
        Key::Tab if ctrl && shift => Some(Action::PreviousTab),
        Key::Tab if ctrl => Some(Action::NextTab),
        Key::Tab if !ctrl && !alt => Some(Action::InsertChar('\t')),

        // =================================================================
        // Backspace/Delete
        // =================================================================
        Key::Backspace if ctrl => Some(Action::DeleteWordLeft),
        Key::Backspace => Some(Action::Backspace),
        Key::Delete if ctrl => Some(Action::DeleteWordRight),
        Key::Delete => Some(Action::Delete),

        // =================================================================
        // Arrow keys
        // =================================================================
        Key::ArrowLeft if ctrl => Some(Action::CursorWordLeft { selecting: shift }),
        Key::ArrowLeft => Some(Action::CursorLeft { selecting: shift }),
        Key::ArrowRight if ctrl => Some(Action::CursorWordRight { selecting: shift }),
        Key::ArrowRight => Some(Action::CursorRight { selecting: shift }),
        Key::ArrowUp if alt => Some(Action::MoveLinesUp),
        Key::ArrowUp => Some(Action::CursorUp { selecting: shift }),
        Key::ArrowDown if alt => Some(Action::MoveLinesDown),
        Key::ArrowDown => Some(Action::CursorDown { selecting: shift }),

        // =================================================================
        // Home/End
        // =================================================================
        Key::Home if ctrl => Some(Action::CursorDocStart { selecting: shift }),
        Key::Home => Some(Action::CursorLineStart { selecting: shift }),
        Key::End if ctrl => Some(Action::CursorDocEnd { selecting: shift }),
        Key::End => Some(Action::CursorLineEnd { selecting: shift }),

        // =================================================================
        // Page Up/Down
        // =================================================================
        Key::PageUp => Some(Action::PageUp { selecting: shift }),
        Key::PageDown => Some(Action::PageDown { selecting: shift }),

        // =================================================================
        // Space
        // =================================================================
        Key::Space if !ctrl && !alt => Some(Action::InsertChar(' ')),

        // =================================================================
        // Character shortcuts
        // =================================================================
        Key::Char(c) => resolve_char(*c, ctrl, shift, alt),

        // Handled above with modifiers, but need to catch the reference patterns
        _ => None,
    }
}

/// Resolve character key shortcuts
fn resolve_char(c: char, ctrl: bool, shift: bool, alt: bool) -> Option<Action> {
    // Normalize to lowercase for matching
    let lower = c.to_ascii_lowercase();

    match lower {
        // Ctrl+<key> shortcuts
        'n' if ctrl => Some(Action::NewTab),
        'w' if ctrl => Some(Action::CloseTab),
        's' if ctrl => Some(Action::Save),
        'o' if ctrl => Some(Action::OpenFile),
        'p' if ctrl => Some(Action::OpenNotesPicker),
        'r' if ctrl => Some(Action::RenameTab),
        'a' if ctrl => Some(Action::SelectAll),
        'c' if ctrl => Some(Action::Copy),
        'x' if ctrl => Some(Action::Cut),
        'v' if ctrl => Some(Action::Paste),
        'z' if ctrl && shift => Some(Action::Redo),
        'z' if ctrl => Some(Action::Undo),
        'y' if ctrl => Some(Action::Redo),

        // Alt+<key> shortcuts
        'z' if alt => Some(Action::ToggleWordWrap),

        // Ctrl+<digit> for tab switching
        '1'..='9' if ctrl => {
            let index = lower.to_digit(10).unwrap() as usize - 1;
            Some(Action::GoToTab(index))
        }

        // Regular character input (no ctrl/alt modifiers)
        _ if !ctrl && !alt => Some(Action::InsertChar(c)),

        // Unknown shortcut
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ctrl_n_new_tab() {
        let event = KeyEvent::new(Key::Char('n'), Modifiers::ctrl());
        assert_eq!(resolve(&event), Some(Action::NewTab));
    }

    #[test]
    fn test_regular_char() {
        let event = KeyEvent::new(Key::Char('a'), Modifiers::none());
        assert_eq!(resolve(&event), Some(Action::InsertChar('a')));
    }

    #[test]
    fn test_shift_arrow() {
        let event = KeyEvent::new(Key::ArrowLeft, Modifiers::shift());
        assert_eq!(resolve(&event), Some(Action::CursorLeft { selecting: true }));
    }
}
