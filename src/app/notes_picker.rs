//! Notes picker operations

use std::path::PathBuf;

use crate::persistence;
use crate::tab::Tab;
use crate::ui::ScrollbarAction;

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

    /// After editing or moving the cursor in the picker input, scroll it so
    /// the cursor stays visible. Delegates to TextInputWidget.ensure_cursor_visible
    /// which owns the visible_width calculation from its own rect.
    pub(crate) fn ensure_picker_cursor_visible(&mut self) {
        let char_width = self.renderer.get_picker_char_width();
        let list_len = self.logic.focus.notes_picker_state()
            .map(|(_, list)| list.len()).unwrap_or(0);
        // Build a temporary layout to get the input widget geometry, then apply
        // scroll correction through the widget's own method.
        let layout = crate::ui::NotesPicker::new(
            self.logic.width, self.logic.height, self.logic.scale, list_len,
        );
        if let Some(input) = self.logic.focus.notes_picker_input_mut() {
            layout.input.ensure_cursor_visible(input, char_width);
        }
    }

    /// Click on the picker input field to place the cursor.
    /// Returns true if the click landed on the input.
    pub(crate) fn click_picker_input(&mut self, x: f32, y: f32, selecting: bool) -> bool {
        let list_len = self.logic.focus.notes_picker_state()
            .map(|(_, list)| list.len()).unwrap_or(0);
        let layout = crate::ui::NotesPicker::new(
            self.logic.width, self.logic.height, self.logic.scale, list_len,
        );
        if !layout.input.rect.contains(x, y) { return false; }
        let char_width = self.renderer.get_picker_char_width();
        let relative_x = x - layout.input.text_x;
        if let Some(input) = self.logic.focus.notes_picker_input_mut() {
            input.set_cursor_from_x(relative_x, char_width, selecting);
        }
        true
    }

    /// Hover over the picker — highlight item under cursor.
    pub fn hover_notes_picker(&mut self, x: f32, y: f32) -> AppResult {
        let list_len = self.logic.focus.notes_picker_state()
            .map(|(_, list)| list.len())
            .unwrap_or(0);
        let layout = crate::ui::NotesPicker::new(
            self.logic.width, self.logic.height, self.logic.scale, list_len,
        );
        self.logic.ui_state.cursor_shape = layout.cursor_shape_at(x, y);
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

    /// Handle mouse click in notes picker.
    /// Returns `Some(drag_offset)` when the user starts dragging the scrollbar thumb.
    pub fn handle_notes_picker_click(&mut self, x: f32, y: f32) -> PickerClickResult {
        let list_len = self.logic.focus.notes_picker_state()
            .map(|(_, list)| list.len())
            .unwrap_or(0);
        let layout = crate::ui::NotesPicker::new(
            self.logic.width, self.logic.height, self.logic.scale, list_len,
        );

        // Click outside overlay → cancel
        if !layout.overlay_rect.contains(x, y) {
            return PickerClickResult::App(self.cancel_notes_picker());
        }

        // Click on input field → place cursor
        if layout.input.rect.contains(x, y) {
            self.click_picker_input(x, y, false);
            self.logic.ui_state.mouse_interaction = crate::app::ui_state::MouseInteraction::TextSelection;
            return PickerClickResult::App(AppResult::Redraw);
        }

        // Click on scrollbar → start drag or jump
        let scroll_offset = self.logic.focus.notes_picker_state()
            .map(|(_, list)| list.scroll_offset()).unwrap_or(0);
        if let Some(action) = layout.scrollbar_action(x, y, list_len, scroll_offset) {
            match action {
                ScrollbarAction::StartDrag { drag_offset } => {
                    return PickerClickResult::StartScrollbarDrag(drag_offset);
                }
                ScrollbarAction::JumpTo { ratio } => {
                    let target = layout.list.scroll_offset_from_ratio(ratio, list_len);
                    if let Some(list) = self.logic.focus.notes_picker_list_mut() {
                        list.scroll_to(target);
                    }
                    return PickerClickResult::App(AppResult::Redraw);
                }
                ScrollbarAction::None => {}
            }
        }

        // Click in list area → select or confirm
        if let Some(display_idx) = layout.item_hit_test(x, y, list_len.min(crate::ui::MAX_VISIBLE_ITEMS)) {
            if let Some(list) = self.logic.focus.notes_picker_list_mut() {
                let scroll_offset = list.scroll_offset();
                let clicked_idx = scroll_offset + display_idx;
                let was_already_selected = list.selected_index() == clicked_idx;
                if list.select_index(clicked_idx) {
                    if was_already_selected {
                        return PickerClickResult::App(self.confirm_notes_picker());
                    }
                    return PickerClickResult::App(AppResult::Redraw);
                }
            }
        }

        PickerClickResult::App(AppResult::Ok)
    }

    /// Continue dragging the picker list scrollbar thumb.
    pub fn drag_picker_scrollbar(&mut self, y: f32, drag_offset: f32) -> AppResult {
        let list_len = self.logic.focus.notes_picker_state()
            .map(|(_, list)| list.len())
            .unwrap_or(0);
        let scroll_offset = self.logic.focus.notes_picker_state()
            .map(|(_, list)| list.scroll_offset()).unwrap_or(0);
        let layout = crate::ui::NotesPicker::new(
            self.logic.width, self.logic.height, self.logic.scale, list_len,
        );
        if let Some(ratio) = layout.list.scrollbar_drag_ratio(y, list_len, scroll_offset, drag_offset) {
            let target = layout.list.scroll_offset_from_ratio(ratio, list_len);
            if let Some(list) = self.logic.focus.notes_picker_list_mut() {
                list.scroll_to(target);
                return AppResult::Redraw;
            }
        }
        AppResult::Ok
    }
}

/// Result of a picker click — either a normal app result or the start of a scrollbar drag.
pub enum PickerClickResult {
    App(AppResult),
    StartScrollbarDrag(f32),
}
