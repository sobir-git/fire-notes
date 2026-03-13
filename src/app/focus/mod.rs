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
    /// Notes picker (quick open) — state lives in `AppLogic.notes_picker`
    NotesPicker,
}

impl Focus {
    pub fn is_renaming(&self) -> bool { matches!(self, Focus::TabRename) }

    pub fn cancel_rename(&mut self) -> bool {
        if self.is_renaming() { *self = Focus::Editor; true } else { false }
    }

    pub fn is_notes_picker(&self) -> bool { matches!(self, Focus::NotesPicker) }

    pub fn open_notes_picker() -> Self {
        Focus::NotesPicker
    }

    pub fn cancel_notes_picker(&mut self) -> bool {
        if self.is_notes_picker() { *self = Focus::Editor; true } else { false }
    }

    pub fn confirm_notes_picker(&mut self) -> bool {
        if self.is_notes_picker() { *self = Focus::Editor; true } else { false }
    }
}
