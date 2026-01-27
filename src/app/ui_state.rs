//! Transient UI state - hover, cursor blink, mouse interactions
//!
//! This module contains only ephemeral UI state that doesn't need to be
//! persisted. Document state lives in Tab, focus state in Focus.

use std::time::Instant;

use crate::ui::ResizeEdge;

/// Mouse interaction state machine - only one interaction at a time
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum MouseInteraction {
    #[default]
    None,
    WindowDrag,
    WindowResize(ResizeEdge),
    ScrollbarDrag { drag_offset: f32 },
    TabDrag { tab_index: usize },
    TextSelection,
}

/// Transient UI state for rendering and interactions
pub struct UiState {
    // Cursor blink
    pub cursor_visible: bool,
    pub last_cursor_blink: Instant,

    // Hover states
    pub hovered_tab_index: Option<usize>,
    pub hovered_plus: bool,
    pub hovered_scrollbar: bool,
    pub hovered_window_minimize: bool,
    pub hovered_window_maximize: bool,
    pub hovered_window_close: bool,
    pub hovered_resize_edge: Option<ResizeEdge>,

    // Mouse state
    pub mouse_interaction: MouseInteraction,
    pub last_drag_scroll: Instant,
    pub last_mouse_x: f32,
    pub last_mouse_y: f32,

    // Tab bar scroll
    pub tab_scroll_x: f32,

    // Flame effect positions (line, col, timestamp)
    pub typing_flame_positions: Vec<(usize, usize, Instant)>,
}

impl UiState {
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
            typing_flame_positions: Vec::new(),
        }
    }

    /// Reset cursor blink (call after user action)
    pub fn reset_cursor_blink(&mut self) {
        self.cursor_visible = true;
        self.last_cursor_blink = Instant::now();
    }

    /// Update cursor blink state, returns true if changed
    pub fn tick_cursor_blink(&mut self, blink_interval_ms: u64) -> bool {
        if self.last_cursor_blink.elapsed().as_millis() >= blink_interval_ms as u128 {
            self.cursor_visible = !self.cursor_visible;
            self.last_cursor_blink = Instant::now();
            true
        } else {
            false
        }
    }

    /// Clean up expired typing flame positions, returns true if any were present
    pub fn cleanup_typing_flames(&mut self, expiry_secs: f32) -> bool {
        let had_flames = !self.typing_flame_positions.is_empty();
        let now = Instant::now();
        self.typing_flame_positions
            .retain(|(_, _, ts)| now.duration_since(*ts).as_secs_f32() < expiry_secs);
        had_flames
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}
