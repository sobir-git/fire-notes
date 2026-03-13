//! Notes picker — app-level operations.
//!
//! Thin shell: builds the entry list, opens the picker, routes typed
//! PickerEvent to business logic. No manual hit-testing or routing here.

use std::path::PathBuf;

use crate::components::notes_picker::PickerEvent;
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

    pub fn scroll_notes_picker(&mut self, lines: isize) -> AppResult {
        if let Some(p) = &mut self.logic.notes_picker {
            if p.on_scroll(lines) { return AppResult::Redraw; }
        }
        AppResult::Ok
    }

    pub fn hover_notes_picker(&mut self, x: f32, y: f32) -> AppResult {
        if let Some(p) = &mut self.logic.notes_picker {
            let (cursor, changed) = p.on_hover(x, y);
            self.logic.cursor_shape = cursor;
            if changed { return AppResult::Redraw; }
        }
        AppResult::Ok
    }

    pub fn handle_notes_picker_click(&mut self, x: f32, y: f32) -> AppResult {
        let char_width = self.renderer.get_picker_char_width();
        let event = if let Some(p) = &mut self.logic.notes_picker {
            p.on_pointer_down(x, y, char_width)
        } else {
            return AppResult::Ok;
        };
        match event {
            PickerEvent::None     => AppResult::Ok,
            PickerEvent::Redraw   => AppResult::Redraw,
            PickerEvent::Cancelled => self.logic.cancel_notes_picker(),
            PickerEvent::Confirmed(entry) => {
                self.logic.notes_picker = None;
                self.logic.focus.confirm_notes_picker();
                self.logic.open_or_switch_to(entry.path)
            }
        }
    }

    pub(crate) fn drag_picker_input(&mut self, x: f32) -> AppResult {
        let char_width = self.renderer.get_picker_char_width();
        if let Some(p) = &mut self.logic.notes_picker {
            p.on_drag(x, char_width);
            return AppResult::Redraw;
        }
        AppResult::Ok
    }
}
