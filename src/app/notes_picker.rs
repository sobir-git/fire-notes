//! Notes picker — the ONE place that knows about NotesPicker.
//! Constructs it, boxes it, hands it to the generic overlay system.
//! Everything else routes through Overlay trait — zero NotesPicker coupling.

use std::path::PathBuf;

use crate::app::active_overlay::ActiveOverlay;
use crate::app::overlay_event::OverlayEvent;
use crate::components::notes_picker::NotesPicker;
use crate::persistence;
use super::focus::NoteEntry;
use super::state::AppResult;
use super::App;

impl App {
    pub fn open_notes_picker(&mut self) -> AppResult {
        let all_note_paths = persistence::list_notes().unwrap_or_default();
        let open_paths: Vec<&PathBuf> = self.logic.tabs
            .iter().filter_map(|tab| tab.path()).collect();
        let notes: Vec<NoteEntry> = all_note_paths.into_iter().map(|path| {
            let title = persistence::load_note_title(&path).unwrap_or_else(|| {
                path.file_name().and_then(|n| n.to_str()).unwrap_or("Unknown").to_string()
            });
            let is_open = open_paths.iter().any(|p| **p == path);
            NoteEntry { path, title, is_open }
        }).collect();
        if notes.is_empty() { return AppResult::Ok; }
        let cw = self.renderer.get_picker_char_width();
        let picker = NotesPicker::new(notes, cw);
        self.logic.open_overlay_with(ActiveOverlay::NotesPicker(picker))
    }

    pub fn is_notes_picker_open(&self) -> bool { self.logic.focus.is_overlay() }

    pub fn scroll_notes_picker(&mut self, lines: isize) -> AppResult {
        self.logic.dispatch_overlay(OverlayEvent::Scroll { lines })
    }

    pub fn hover_notes_picker(&mut self, x: f32, y: f32) -> AppResult {
        if let Some(o) = &self.logic.overlay {
            self.logic.cursor_shape = o.cursor_shape_at(x, y);
        }
        self.logic.dispatch_overlay(OverlayEvent::PointerMove { x, y })
    }
}
