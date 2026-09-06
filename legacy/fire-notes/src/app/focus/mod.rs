//! Focus management — determines which component receives keyboard input.

mod input;

use std::path::PathBuf;

/// A note entry for the notes picker
#[derive(Debug, Clone, PartialEq)]
pub struct NoteEntry {
    pub path: PathBuf,
    pub title: String,
    pub is_open: bool,
}

/// Represents what currently has keyboard focus
#[derive(Debug, Clone, Default)]
pub enum Focus {
    /// Main text editor (the active tab's content)
    #[default]
    Editor,
    /// Tab title rename input — state lives in `AppLogic.rename_input`
    TabRename,
    /// A full-screen overlay is active (picker, dialog, etc.)
    Overlay,
}

impl Focus {
    pub fn is_renaming(&self) -> bool { matches!(self, Focus::TabRename) }

    pub fn cancel_rename(&mut self) -> bool {
        if self.is_renaming() { *self = Focus::Editor; true } else { false }
    }

    pub fn is_overlay(&self) -> bool { matches!(self, Focus::Overlay) }

    pub fn open_overlay() -> Self { Focus::Overlay }

    pub fn close_overlay(&mut self) -> bool {
        if self.is_overlay() { *self = Focus::Editor; true } else { false }
    }
}
