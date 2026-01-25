//! Application state types

use std::time::Instant;

use crate::ui::ResizeEdge;

/// Represents the current mouse interaction state.
/// Only one interaction can be active at a time, preventing event leaking.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum MouseInteraction {
    /// No active mouse interaction
    #[default]
    None,
    /// Dragging the window (borderless title bar drag)
    WindowDrag,
    /// Resizing the window from an edge
    WindowResize(ResizeEdge),
    /// Dragging the scrollbar
    ScrollbarDrag { drag_offset: f32 },
    /// Dragging a tab to reorder
    TabDrag { tab_index: usize },
    /// Text selection in progress
    TextSelection,
}


/// Result type for application actions that may trigger UI updates
#[must_use = "Handle the AppResult to ensure the UI updates correctly"]
pub enum AppResult {
    /// No action needed
    Ok,
    /// UI needs to be redrawn
    Redraw,
    /// Minimize window
    WindowMinimize,
    /// Maximize/restore window
    WindowMaximize,
    /// Close window
    WindowClose,
    /// Start window drag
    WindowDrag,
    /// Start window resize from edge
    WindowResize(ResizeEdge),
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
    pub hovered_window_minimize: bool,
    pub hovered_window_maximize: bool,
    pub hovered_window_close: bool,
    pub hovered_resize_edge: Option<ResizeEdge>,
    /// Current mouse interaction - only one can be active at a time
    pub mouse_interaction: MouseInteraction,
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
            hovered_window_minimize: false,
            hovered_window_maximize: false,
            hovered_window_close: false,
            hovered_resize_edge: None,
            mouse_interaction: MouseInteraction::None,
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
