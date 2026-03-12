//! Notes picker operations

use std::path::PathBuf;

use crate::persistence;
use crate::tab::Tab;

use super::focus::{Focus, NoteEntry};
use super::state::AppResult;
use super::App;

impl App {
    /// Open the notes picker with all available notes
    pub fn open_notes_picker(&mut self) -> AppResult {
        // Get all notes from the data directory
        let all_note_paths = persistence::list_notes().unwrap_or_default();

        // Get paths of currently open tabs
        let open_paths: Vec<&PathBuf> = self.logic.tabs
            .iter()
            .filter_map(|tab| tab.path())
            .collect();

        // Build note entries with title and open status
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
                NoteEntry {
                    path,
                    title,
                    is_open,
                }
            })
            .collect();

        if notes.is_empty() {
            return AppResult::Ok;
        }

        self.logic.focus = Focus::start_notes_picker(notes);
        self.logic.ui_state.cursor_shape = crate::ui::CursorShape::Default;
        AppResult::Redraw
    }

    /// Open a note by path (either switch to existing tab or open new)
    pub fn open_note_by_path(&mut self, path: PathBuf) -> AppResult {
        // Check if already open
        for (i, tab) in self.logic.tabs.iter().enumerate() {
            if tab.path() == Some(&path) {
                self.logic.activate_tab(i);
                return AppResult::Redraw;
            }
        }

        // Open as new tab
        if let Some(tab) = Tab::from_file(path) {
            self.logic.tabs.push(tab);
            self.logic.activate_tab(self.logic.tabs.len() - 1);
            return AppResult::Redraw;
        }

        AppResult::Ok
    }

    /// Confirm notes picker selection
    pub fn confirm_notes_picker(&mut self) -> AppResult {
        if let Some(path) = self.logic.focus.confirm_notes_picker() {
            return self.open_note_by_path(path);
        }
        AppResult::Ok
    }

    pub fn is_notes_picker_open(&self) -> bool {
        self.logic.focus.is_notes_picker()
    }

    /// Scroll the notes picker list by `lines` (positive = down, negative = up).
    pub fn scroll_notes_picker(&mut self, lines: isize) -> AppResult {
        if let Some(list) = self.logic.focus.notes_picker_list_mut() {
            let changed = if lines > 0 {
                (0..lines as usize).fold(false, |acc, _| list.select_down() || acc)
            } else {
                (0..(-lines) as usize).fold(false, |acc, _| list.select_up() || acc)
            };
            if changed { return AppResult::Redraw; }
        }
        AppResult::Ok
    }

    /// Hover over the picker — highlight item under cursor.
    pub fn hover_notes_picker(&mut self, x: f32, y: f32) -> AppResult {
        let list_len = self.logic.focus.notes_picker_state()
            .map(|(_, list)| list.len())
            .unwrap_or(0);
        let layout = crate::ui::NotesPicker::new(
            self.logic.width, self.logic.height, self.logic.scale, list_len,
        );
        self.logic.ui_state.cursor_shape = if layout.input_rect.contains(x, y) {
            crate::ui::CursorShape::Text
        } else {
            crate::ui::CursorShape::Default
        };
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

    /// Cancel notes picker
    pub fn cancel_notes_picker(&mut self) -> AppResult {
        if self.logic.focus.cancel_notes_picker() {
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    /// Handle mouse click in notes picker
    pub fn handle_notes_picker_click(&mut self, x: f32, y: f32) -> AppResult {
        let list_len = self.logic.focus.notes_picker_state()
            .map(|(_, list)| list.len())
            .unwrap_or(0);
        let layout = crate::ui::NotesPicker::new(
            self.logic.width, self.logic.height, self.logic.scale, list_len,
        );

        // Click outside overlay → cancel
        if !layout.overlay_rect.contains(x, y) {
            return self.cancel_notes_picker();
        }

        // Click in list area → select or confirm
        if let Some(display_idx) = layout.item_hit_test(x, y, list_len.min(crate::ui::MAX_VISIBLE_ITEMS)) {
            if let Some(list) = self.logic.focus.notes_picker_list_mut() {
                let scroll_offset = list.scroll_offset();
                let clicked_idx = scroll_offset + display_idx;
                let was_already_selected = list.selected_index() == clicked_idx;
                if list.select_index(clicked_idx) {
                    if was_already_selected {
                        return self.confirm_notes_picker();
                    }
                    return AppResult::Redraw;
                }
            }
        }

        AppResult::Ok
    }
}
