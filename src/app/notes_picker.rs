//! Notes picker — app-level operations.
//!
//! This module is responsible only for:
//!   1. Building the note entry list from persistence (open_notes_picker).
//!   2. Routing PickerClickOutcome to the correct app action.
//!   3. Opening/switching tabs when a note is confirmed.
//!
//! All interaction geometry (click, hover, drag, scroll, filter, cursor-visible)
//! is owned by `NotesPicker` in `src/ui/notes_picker.rs`.

use std::path::PathBuf;

use crate::persistence;
use crate::tab::Tab;
use crate::ui::{NotesPicker, PickerClickOutcome};

use super::focus::{Focus, NoteEntry};
use super::state::AppResult;
use super::App;

impl App {
    /// Open the notes picker with all available notes.
    pub fn open_notes_picker(&mut self) -> AppResult {
        let all_note_paths = persistence::list_notes().unwrap_or_default();
        let open_paths: Vec<&PathBuf> = self.logic.tabs
            .iter()
            .filter_map(|tab| tab.path())
            .collect();
        let notes: Vec<NoteEntry> = all_note_paths
            .into_iter()
            .map(|path| {
                let title = persistence::load_note_title(&path).unwrap_or_else(|| {
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown")
                        .to_string()
                });
                let is_open = open_paths.iter().any(|p| **p == path);
                NoteEntry { path, title, is_open }
            })
            .collect();
        if notes.is_empty() { return AppResult::Ok; }
        self.logic.focus = Focus::start_notes_picker(notes);
        self.logic.ui_state.cursor_shape = crate::ui::CursorShape::Default;
        AppResult::Redraw
    }

    /// Open a note by path (switch to existing tab or open new).
    pub fn open_note_by_path(&mut self, path: PathBuf) -> AppResult {
        for (i, tab) in self.logic.tabs.iter().enumerate() {
            if tab.path() == Some(&path) {
                self.logic.activate_tab(i);
                return AppResult::Redraw;
            }
        }
        if let Some(tab) = Tab::from_file(path) {
            self.logic.tabs.push(tab);
            self.logic.activate_tab(self.logic.tabs.len() - 1);
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    /// Confirm the current picker selection.
    pub fn confirm_notes_picker(&mut self) -> AppResult {
        if let Some(path) = self.logic.focus.confirm_notes_picker() {
            return self.open_note_by_path(path);
        }
        AppResult::Ok
    }

    pub fn is_notes_picker_open(&self) -> bool {
        self.logic.focus.is_notes_picker()
    }

    /// Cancel notes picker.
    pub fn cancel_notes_picker(&mut self) -> AppResult {
        if self.logic.focus.cancel_notes_picker() { return AppResult::Redraw; }
        AppResult::Ok
    }

    /// Scroll the picker list by `lines` (positive = down, negative = up).
    pub fn scroll_notes_picker(&mut self, lines: isize) -> AppResult {
        let list_len = self.logic.focus.notes_picker_state()
            .map(|(_, list)| list.len()).unwrap_or(0);
        let layout = NotesPicker::new(
            self.logic.width, self.logic.height, self.logic.scale, list_len,
        );
        if let Some(list) = self.logic.focus.notes_picker_list_mut() {
            if layout.scroll_list(lines, list) { return AppResult::Redraw; }
        }
        AppResult::Ok
    }

    /// Handle hover over the picker — update cursor shape and highlight item.
    pub fn hover_notes_picker(&mut self, x: f32, y: f32) -> AppResult {
        let list_len = self.logic.focus.notes_picker_state()
            .map(|(_, list)| list.len()).unwrap_or(0);
        let layout = NotesPicker::new(
            self.logic.width, self.logic.height, self.logic.scale, list_len,
        );
        self.logic.ui_state.cursor_shape = layout.cursor_shape_at_hover(x, y);
        if let Some(display_idx) = layout.item_hit_test(x, y, list_len.min(crate::ui::MAX_VISIBLE_ITEMS)) {
            if let Some(list) = self.logic.focus.notes_picker_list_mut() {
                let target = list.scroll_offset() + display_idx;
                if list.selected_index() != target {
                    list.select_index(target);
                    return AppResult::Redraw;
                }
            }
        }
        AppResult::Ok
    }

    /// Handle a mouse click in the picker. Delegates all geometry decisions to
    /// `NotesPicker::handle_click`; this method only routes the outcome.
    pub fn handle_notes_picker_click(&mut self, x: f32, y: f32) -> PickerClickResult {
        let list_len = self.logic.focus.notes_picker_state()
            .map(|(_, list)| list.len()).unwrap_or(0);
        let layout = NotesPicker::new(
            self.logic.width, self.logic.height, self.logic.scale, list_len,
        );
        let char_width = self.renderer.get_picker_char_width();

        // Split borrows: get input and list separately.
        let (input_opt, list_opt) = match &mut self.logic.focus {
            Focus::NotesPicker { input, list } => (Some(input), Some(list)),
            _ => (None, None),
        };
        let (Some(input), Some(list)) = (input_opt, list_opt) else {
            return PickerClickResult::App(AppResult::Ok);
        };

        match layout.handle_click(x, y, input, list, char_width) {
            PickerClickOutcome::Cancel => {
                PickerClickResult::App(self.cancel_notes_picker())
            }
            PickerClickOutcome::InputFocus => {
                self.logic.ui_state.mouse_interaction =
                    crate::app::ui_state::MouseInteraction::TextSelection;
                PickerClickResult::App(AppResult::Redraw)
            }
            PickerClickOutcome::StartScrollbarDrag { drag_offset } => {
                PickerClickResult::StartScrollbarDrag(drag_offset)
            }
            PickerClickOutcome::Scrolled | PickerClickOutcome::Selected => {
                PickerClickResult::App(AppResult::Redraw)
            }
            PickerClickOutcome::Confirmed => {
                PickerClickResult::App(self.confirm_notes_picker())
            }
            PickerClickOutcome::None => {
                PickerClickResult::App(AppResult::Ok)
            }
        }
    }

    /// Continue dragging the picker list scrollbar thumb.
    pub fn drag_picker_scrollbar(&mut self, y: f32, drag_offset: f32) -> AppResult {
        let list_len = self.logic.focus.notes_picker_state()
            .map(|(_, list)| list.len()).unwrap_or(0);
        let layout = NotesPicker::new(
            self.logic.width, self.logic.height, self.logic.scale, list_len,
        );
        if let Some(list) = self.logic.focus.notes_picker_list_mut() {
            if layout.handle_scrollbar_drag(y, drag_offset, list) {
                return AppResult::Redraw;
            }
        }
        AppResult::Ok
    }

    /// Handle picker input drag (text selection within the search field).
    pub(crate) fn drag_picker_input(&mut self, x: f32) -> AppResult {
        let list_len = self.logic.focus.notes_picker_state()
            .map(|(_, list)| list.len()).unwrap_or(0);
        let layout = NotesPicker::new(
            self.logic.width, self.logic.height, self.logic.scale, list_len,
        );
        let char_width = self.renderer.get_picker_char_width();
        if let Some(input) = self.logic.focus.notes_picker_input_mut() {
            layout.handle_input_drag(x, input, char_width);
            return AppResult::Redraw;
        }
        AppResult::Ok
    }
}

/// Result of a picker click — either a normal app result or the start of a scrollbar drag.
pub enum PickerClickResult {
    App(AppResult),
    StartScrollbarDrag(f32),
}
