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
        let open_paths: Vec<&PathBuf> = self
            .tabs
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

        self.focus = Focus::start_notes_picker(notes);
        AppResult::Redraw
    }

    /// Open a note by path (either switch to existing tab or open new)
    pub fn open_note_by_path(&mut self, path: PathBuf) -> AppResult {
        // Check if already open
        for (i, tab) in self.tabs.iter().enumerate() {
            if tab.path() == Some(&path) {
                self.active_tab = i;
                self.auto_scroll();
                return AppResult::Redraw;
            }
        }

        // Open as new tab
        if let Some(tab) = Tab::from_file(path) {
            self.tabs.push(tab);
            self.active_tab = self.tabs.len() - 1;
            self.auto_scroll();
            return AppResult::Redraw;
        }

        AppResult::Ok
    }

    /// Confirm notes picker selection
    pub fn confirm_notes_picker(&mut self) -> AppResult {
        if let Some(path) = self.focus.confirm_notes_picker() {
            return self.open_note_by_path(path);
        }
        AppResult::Ok
    }

    /// Cancel notes picker
    pub fn cancel_notes_picker(&mut self) -> AppResult {
        if self.focus.cancel_notes_picker() {
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    /// Handle mouse click in notes picker
    pub fn handle_notes_picker_click(&mut self, x: f32, y: f32) -> AppResult {
        let scale = self.scale;
        
        // Calculate overlay dimensions (must match renderer)
        let overlay_width = (self.width * 0.6).min(500.0 * scale);
        let overlay_x = (self.width - overlay_width) / 2.0;
        let overlay_y = 60.0 * scale;
        
        let input_height = 36.0 * scale;
        let item_height = 32.0 * scale;
        let max_visible_items = 8;
        
        // Check if click is within overlay bounds
        let input_x = overlay_x + 8.0 * scale;
        let input_width = overlay_width - 16.0 * scale;
        let list_y = overlay_y + 8.0 * scale + input_height + 4.0 * scale;
        
        // Check if click is in the list area
        if x >= input_x && x <= input_x + input_width && y >= list_y {
            let relative_y = y - list_y;
            let clicked_visible_idx = (relative_y / item_height) as usize;
            
            if clicked_visible_idx < max_visible_items {
                if let Some(list) = self.focus.notes_picker_list_mut() {
                    let scroll_offset = list.scroll_offset();
                    let clicked_idx = scroll_offset + clicked_visible_idx;
                    let was_already_selected = list.selected_index() == clicked_idx;
                    
                    if list.select_index(clicked_idx) {
                        // If clicking already selected item, confirm (acts like double-click)
                        if was_already_selected {
                            return self.confirm_notes_picker();
                        }
                        return AppResult::Redraw;
                    }
                }
            }
        }
        
        // Check if click is outside the overlay (cancel)
        let list_count = self.focus.notes_picker_state()
            .map(|(_, list)| list.len().min(max_visible_items))
            .unwrap_or(0);
        let list_height = list_count as f32 * item_height;
        let overlay_height = input_height + list_height + 16.0 * scale;
        
        if x < overlay_x || x > overlay_x + overlay_width 
            || y < overlay_y || y > overlay_y + overlay_height 
        {
            return self.cancel_notes_picker();
        }
        
        AppResult::Ok
    }
}
