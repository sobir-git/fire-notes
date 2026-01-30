//! Focus management - determines which component receives keyboard input
//!
//! This module provides a clean abstraction for input routing. Instead of
//! checking `renaming_tab.is_some()` in every input handler, we have a single
//! Focus enum that determines where input goes.
//!
//! The Focus enum implements InputHandler, dispatching input to the currently
//! focused widget. Adding a new focusable widget only requires updating this
//! module, not the central input handler.

use super::input_handler::{InputHandler, InputResult};
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
#[derive(Debug, Clone)]
pub enum Focus {
    /// Main text editor (the active tab's content)
    Editor,
    /// Tab title rename input
    TabRename {
        tab_index: usize,
        input: TextInput,
    },
    /// Notes picker (quick open)
    NotesPicker {
        input: TextInput,
        list: ListWidget<NoteEntry>,
    },
}

impl Default for Focus {
    fn default() -> Self {
        Focus::Editor
    }
}

impl Focus {
    /// Check if we're currently renaming a tab
    pub fn is_renaming(&self) -> bool {
        matches!(self, Focus::TabRename { .. })
    }

    /// Get the tab index being renamed, if any
    pub fn renaming_tab_index(&self) -> Option<usize> {
        match self {
            Focus::TabRename { tab_index, .. } => Some(*tab_index),
            _ => None,
        }
    }

    /// Get read access to the rename input, if active
    pub fn rename_input(&self) -> Option<&TextInput> {
        match self {
            Focus::TabRename { input, .. } => Some(input),
            _ => None,
        }
    }

    /// Start renaming a tab
    pub fn start_rename(tab_index: usize, current_title: &str) -> Self {
        let mut input = TextInput::new(current_title.to_string());
        input.select_all();
        Focus::TabRename { tab_index, input }
    }

    /// Confirm rename and return the new title, transitioning back to Editor focus
    pub fn confirm_rename(&mut self) -> Option<(usize, String)> {
        match std::mem::take(self) {
            Focus::TabRename { tab_index, input } => {
                let title = input.text().trim().to_string();
                *self = Focus::Editor;
                if title.is_empty() {
                    None
                } else {
                    Some((tab_index, title))
                }
            }
            other => {
                *self = other;
                None
            }
        }
    }

    /// Cancel rename and return to Editor focus
    pub fn cancel_rename(&mut self) -> bool {
        if self.is_renaming() {
            *self = Focus::Editor;
            true
        } else {
            false
        }
    }

    /// Check if we're in the notes picker
    pub fn is_notes_picker(&self) -> bool {
        matches!(self, Focus::NotesPicker { .. })
    }

    /// Start the notes picker with a list of notes
    pub fn start_notes_picker(notes: Vec<NoteEntry>) -> Self {
        Focus::NotesPicker {
            input: TextInput::new(String::new()),
            list: ListWidget::new(notes),
        }
    }

    /// Get notes picker state for rendering
    pub fn notes_picker_state(&self) -> Option<(&TextInput, &ListWidget<NoteEntry>)> {
        match self {
            Focus::NotesPicker { input, list } => Some((input, list)),
            _ => None,
        }
    }

    /// Get mutable notes picker list for mouse interaction
    pub fn notes_picker_list_mut(&mut self) -> Option<&mut ListWidget<NoteEntry>> {
        match self {
            Focus::NotesPicker { list, .. } => Some(list),
            _ => None,
        }
    }

    /// Update filtered notes based on search input
    pub fn update_notes_filter(&mut self) {
        if let Focus::NotesPicker { input, list } = self {
            let query = input.text().to_lowercase();
            if query.is_empty() {
                list.clear_filter();
            } else {
                list.filter(|note| note.title.to_lowercase().contains(&query));
            }
        }
    }

    /// Move selection up in notes picker
    pub fn notes_picker_up(&mut self) {
        if let Focus::NotesPicker { list, .. } = self {
            list.select_up();
        }
    }

    /// Move selection down in notes picker
    pub fn notes_picker_down(&mut self) {
        if let Focus::NotesPicker { list, .. } = self {
            list.select_down();
        }
    }

    /// Confirm notes picker selection, returns the selected note path
    pub fn confirm_notes_picker(&mut self) -> Option<PathBuf> {
        match std::mem::take(self) {
            Focus::NotesPicker { list, .. } => {
                let path = list.selected_item().map(|n| n.path.clone());
                *self = Focus::Editor;
                path
            }
            other => {
                *self = other;
                None
            }
        }
    }

    /// Cancel notes picker and return to Editor focus
    pub fn cancel_notes_picker(&mut self) -> bool {
        if self.is_notes_picker() {
            *self = Focus::Editor;
            true
        } else {
            false
        }
    }
}

/// InputHandler implementation for Focus - dispatches to the focused widget
impl InputHandler for Focus {
    fn handle_char(&mut self, ch: char) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { input, .. } => {
                input.insert_char(ch);
                InputResult::Handled
            }
            Focus::NotesPicker { input, .. } => {
                input.insert_char(ch);
                self.update_notes_filter();
                InputResult::Handled
            }
        }
    }

    fn handle_backspace(&mut self) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { input, .. } => {
                input.backspace();
                InputResult::Handled
            }
            Focus::NotesPicker { input, .. } => {
                input.backspace();
                self.update_notes_filter();
                InputResult::Handled
            }
        }
    }

    fn handle_delete(&mut self) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { input, .. } => {
                input.delete();
                InputResult::Handled
            }
            Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

    fn handle_delete_word_left(&mut self) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { input, .. } => {
                input.delete_word_left();
                InputResult::Handled
            }
            Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

    fn handle_delete_word_right(&mut self) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { input, .. } => {
                input.delete_word_right();
                InputResult::Handled
            }
            Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

    fn handle_select_all(&mut self) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { input, .. } => {
                input.select_all();
                InputResult::Handled
            }
            Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

    fn move_left(&mut self, selecting: bool) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { input, .. } => {
                input.move_left(selecting);
                InputResult::Handled
            }
            Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

    fn move_right(&mut self, selecting: bool) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { input, .. } => {
                input.move_right(selecting);
                InputResult::Handled
            }
            Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

    fn move_up(&mut self, _selecting: bool) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { .. } => InputResult::Ignored,
            Focus::NotesPicker { .. } => {
                self.notes_picker_up();
                InputResult::Handled
            }
        }
    }

    fn move_down(&mut self, _selecting: bool) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { .. } => InputResult::Ignored,
            Focus::NotesPicker { .. } => {
                self.notes_picker_down();
                InputResult::Handled
            }
        }
    }

    fn move_word_left(&mut self, selecting: bool) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { input, .. } => {
                input.move_word_left(selecting);
                InputResult::Handled
            }
            Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

    fn move_word_right(&mut self, selecting: bool) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { input, .. } => {
                input.move_word_right(selecting);
                InputResult::Handled
            }
            Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

    fn move_to_line_start(&mut self, selecting: bool) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { input, .. } => {
                input.move_to_start(selecting);
                InputResult::Handled
            }
            Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

    fn move_to_line_end(&mut self, selecting: bool) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { input, .. } => {
                input.move_to_end(selecting);
                InputResult::Handled
            }
            Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

    fn move_to_start(&mut self, _selecting: bool) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { .. } | Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

    fn move_to_end(&mut self, _selecting: bool) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { .. } | Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

    fn copy(&self) -> Option<String> {
        match self {
            Focus::Editor => None,
            Focus::TabRename { input, .. } => input.copy(),
            Focus::NotesPicker { .. } => None,
        }
    }

    fn cut(&mut self) -> Option<String> {
        match self {
            Focus::Editor => None,
            Focus::TabRename { input, .. } => input.cut(),
            Focus::NotesPicker { .. } => None,
        }
    }

    fn paste(&mut self, text: &str) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { input, .. } => {
                input.paste(text);
                InputResult::Handled
            }
            Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

    fn undo(&mut self) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { .. } | Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }

    fn redo(&mut self) -> InputResult {
        match self {
            Focus::Editor => InputResult::NotHandled,
            Focus::TabRename { .. } | Focus::NotesPicker { .. } => InputResult::Ignored,
        }
    }
}
