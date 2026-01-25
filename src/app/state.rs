//! Application state types

use std::time::Instant;

/// Result type for application actions that may trigger UI updates
#[must_use = "Handle the AppResult to ensure the UI updates correctly"]
pub enum AppResult {
    /// No action needed
    Ok,
    /// UI needs to be redrawn
    Redraw,
}

impl AppResult {
    pub fn needs_redraw(&self) -> bool {
        matches!(self, AppResult::Redraw)
    }
}

/// Transient editor state for UI interactions (cursor blink, hover, drag, etc.)
pub struct EditorState {
    pub cursor_visible: bool,
    pub last_cursor_blink: Instant,
    pub hovered_tab_index: Option<usize>,
    pub hovered_plus: bool,
    pub hovered_scrollbar: bool,
    pub is_dragging_scrollbar: bool,
    pub scrollbar_drag_offset: f32,
    pub dragging_tab_index: Option<usize>,
    pub last_drag_scroll: Instant,
    pub last_mouse_x: f32,
    pub last_mouse_y: f32,
    pub tab_scroll_x: f32,
    pub renaming_tab: Option<usize>,
    pub rename_buffer: String,
    pub typing_flame_positions: Vec<(usize, usize, Instant)>, // (line, col, timestamp)
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            cursor_visible: true,
            last_cursor_blink: Instant::now(),
            hovered_tab_index: None,
            hovered_plus: false,
            hovered_scrollbar: false,
            is_dragging_scrollbar: false,
            scrollbar_drag_offset: 0.0,
            dragging_tab_index: None,
            last_drag_scroll: Instant::now(),
            last_mouse_x: 0.0,
            last_mouse_y: 0.0,
            tab_scroll_x: 0.0,
            renaming_tab: None,
            rename_buffer: String::new(),
            typing_flame_positions: Vec::new(),
        }
    }

    /// Reset cursor blink (call after user action)
    pub fn reset_cursor_blink(&mut self) {
        self.cursor_visible = true;
        self.last_cursor_blink = Instant::now();
    }
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}
