//! Notes picker — app-level operations.

use std::path::PathBuf;

use crate::fw::widgets::list::ListPointerResult;
use crate::persistence;

use super::focus::NoteEntry;
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
        self.logic.cursor_shape = crate::ui::CursorShape::Default;
        self.logic.open_notes_picker_with(notes)
    }

    pub fn is_notes_picker_open(&self) -> bool {
        self.logic.focus.is_notes_picker()
    }

    /// Scroll the picker list by `lines` (positive = down, negative = up).
    pub fn scroll_notes_picker(&mut self, lines: isize) -> AppResult {
        if let Some(picker) = &mut self.logic.notes_picker {
            if picker.scroll_by(lines) { return AppResult::Redraw; }
        }
        AppResult::Ok
    }

    /// Handle hover over the picker — update cursor shape and highlight item.
    pub fn hover_notes_picker(&mut self, x: f32, y: f32) -> AppResult {
        if let Some(picker) = &mut self.logic.notes_picker {
            self.logic.cursor_shape = picker.cursor_shape_at(x, y);
            if picker.on_hover(x, y) { return AppResult::Redraw; }
        }
        AppResult::Ok
    }

    /// Handle a mouse click in the picker.
    pub fn handle_notes_picker_click(&mut self, x: f32, y: f32) -> AppResult {
        let char_width = self.renderer.get_picker_char_width();

        // Detect outside click using picker's own overlay_rect.
        let outside = if let Some(picker) = &self.logic.notes_picker {
            let window = crate::ui::Rect {
                x: 0.0, y: 0.0,
                width: self.logic.width, height: self.logic.height,
            };
            !picker.overlay_rect(window, self.logic.scale).contains(x, y)
        } else {
            return AppResult::Ok;
        };

        if outside {
            return self.logic.cancel_notes_picker();
        }

        // Check search input first.
        if let Some(picker) = &mut self.logic.notes_picker {
            if picker.search.on_pointer_down(x, y, char_width) {
                self.logic.ui_tree.content_area.is_text_selecting = true;
                return AppResult::Redraw;
            }
        }

        // Check list.
        if let Some(picker) = &mut self.logic.notes_picker {
            match picker.list.on_pointer_down(x, y) {
                ListPointerResult::ScrollbarDragStart { .. } => return AppResult::Redraw,
                ListPointerResult::Scrolled                 => return AppResult::Redraw,
                ListPointerResult::Selected                 => return AppResult::Redraw,
                ListPointerResult::Confirmed(_)             => return self.logic.confirm_notes_picker(),
                ListPointerResult::None                     => {}
            }
        }
        AppResult::Ok
    }

    /// Handle picker input drag (text selection within the search field).
    pub(crate) fn drag_picker_input(&mut self, x: f32) -> AppResult {
        let char_width = self.renderer.get_picker_char_width();
        if let Some(picker) = &mut self.logic.notes_picker {
            picker.search.on_drag(x, char_width);
            return AppResult::Redraw;
        }
        AppResult::Ok
    }
}
