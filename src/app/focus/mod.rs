//! Focus management — determines which component receives keyboard input.

mod input;

use crate::ui::{ListWidget, TextInput};
use std::path::PathBuf;

/// A note entry for the notes picker
#[derive(Debug, Clone)]
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
    /// Tab title rename input
    TabRename { tab_index: usize, input: TextInput },
    /// Notes picker (quick open)
    NotesPicker { input: TextInput, list: ListWidget<NoteEntry> },
}

impl Focus {
    pub fn is_renaming(&self) -> bool { matches!(self, Focus::TabRename { .. }) }

    pub fn renaming_tab_index(&self) -> Option<usize> {
        match self { Focus::TabRename { tab_index, .. } => Some(*tab_index), _ => None }
    }

    pub fn rename_input(&self) -> Option<&TextInput> {
        match self { Focus::TabRename { input, .. } => Some(input), _ => None }
    }

    pub fn start_rename(tab_index: usize, current_title: &str) -> Self {
        let mut input = TextInput::new(current_title.to_string());
        input.select_all();
        Focus::TabRename { tab_index, input }
    }

    pub fn confirm_rename(&mut self) -> Option<(usize, String)> {
        match std::mem::take(self) {
            Focus::TabRename { tab_index, input } => {
                let title = input.text().trim().to_string();
                *self = Focus::Editor;
                if title.is_empty() { None } else { Some((tab_index, title)) }
            }
            other => { *self = other; None }
        }
    }

    pub fn cancel_rename(&mut self) -> bool {
        if self.is_renaming() { *self = Focus::Editor; true } else { false }
    }

    pub fn is_notes_picker(&self) -> bool { matches!(self, Focus::NotesPicker { .. }) }

    pub fn start_notes_picker(notes: Vec<NoteEntry>) -> Self {
        Focus::NotesPicker {
            input: TextInput::new(String::new()),
            list: ListWidget::new(notes),
        }
    }

    pub fn notes_picker_state(&self) -> Option<(&TextInput, &ListWidget<NoteEntry>)> {
        match self { Focus::NotesPicker { input, list } => Some((input, list)), _ => None }
    }

    pub fn notes_picker_list_mut(&mut self) -> Option<&mut ListWidget<NoteEntry>> {
        match self { Focus::NotesPicker { list, .. } => Some(list), _ => None }
    }

    pub fn update_notes_filter(&mut self) {
        if let Focus::NotesPicker { input, list } = self {
            let query = input.text().to_lowercase();
            if query.is_empty() { list.clear_filter(); }
            else { list.filter(|note| note.title.to_lowercase().contains(&query)); }
        }
    }

    pub fn notes_picker_up(&mut self) {
        if let Focus::NotesPicker { list, .. } = self { list.select_up(); }
    }

    pub fn notes_picker_down(&mut self) {
        if let Focus::NotesPicker { list, .. } = self { list.select_down(); }
    }

    pub fn confirm_notes_picker(&mut self) -> Option<PathBuf> {
        match std::mem::take(self) {
            Focus::NotesPicker { list, .. } => {
                let path = list.selected_item().map(|n| n.path.clone());
                *self = Focus::Editor;
                path
            }
            other => { *self = other; None }
        }
    }

    pub fn cancel_notes_picker(&mut self) -> bool {
        if self.is_notes_picker() { *self = Focus::Editor; true } else { false }
    }
}
