#![allow(dead_code)]
//! TextInput primitive — editing state + geometry, one file.
//!
//! Replaces `ui::TextInput` (state) + `ui::TextInputWidget` (geometry) + `fw::TextInput` (merged).

use crate::layout::Widget;
use crate::ui::{CursorShape, Rect, TextInput as RawState};

// ── Primitive ─────────────────────────────────────────────────────────────────

/// Single-line text input — owns editing state and screen geometry.
#[derive(Debug, Clone)]
pub struct TextInput {
    /// Editing state: text, cursor, selection_anchor, scroll_offset.
    pub state: RawState,
    /// Bounding rect of the input box.
    pub rect: Rect,
    /// Font size in physical pixels.
    pub font_size: f32,
    /// X where text rendering starts (rect.x + padding).
    pub text_x: f32,
    /// Text baseline Y (vertically centred in rect).
    pub text_baseline_y: f32,
    pub scale: f32,
    /// Cached char width — set by caller after font measurement.
    pub char_width: f32,
}

// ── Widget impl ───────────────────────────────────────────────────────────────

/// `Widget` impl uses the stored `char_width`. Call `set_char_width` after
/// font measurement so on_pointer_down has correct hit-testing.
#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    /// Click was inside the input — cursor repositioned.
    Focused,
    /// Click was outside the input.
    Miss,
}

impl Widget for TextInput {
    type Event = InputEvent;
    fn on_pointer_down(&mut self, x: f32, y: f32) -> InputEvent {
        if self.on_pointer_down_with(x, y, self.char_width) {
            InputEvent::Focused
        } else {
            InputEvent::Miss
        }
    }
    fn on_hover(&mut self, _x: f32, _y: f32) -> bool { false }
    fn cursor_shape_at(&self, x: f32, y: f32) -> CursorShape {
        if self.rect.contains(x, y) { CursorShape::Text } else { CursorShape::Default }
    }
}

impl TextInput {
    pub fn new_empty() -> Self {
        Self {
            state:            RawState::new(String::new()),
            rect:             Rect::ZERO,
            font_size:        0.0,
            text_x:           0.0,
            text_baseline_y:  0.0,
            scale:            1.0,
            char_width:       8.0,
        }
    }

    pub fn set_char_width(&mut self, w: f32) { self.char_width = w; }

    pub fn new(text: String) -> Self {
        let mut s = Self::new_empty();
        s.state = RawState::new(text);
        s
    }

    /// Update geometry, preserving all editing state.
    pub fn relayout(&mut self, rect: Rect, scale: f32) {
        let padding   = 8.0 * scale;
        let font_size = 14.0 * scale;
        self.rect             = rect;
        self.scale            = scale;
        self.font_size        = font_size;
        self.text_x           = rect.x + padding;
        self.text_baseline_y  = rect.y + rect.height / 2.0 + font_size * 0.35;
    }

    // ── State accessors ───────────────────────────────────────────────────────

    pub fn text(&self)         -> &str { self.state.text() }
    pub fn is_empty(&self)     -> bool { self.state.text().is_empty() }
    pub fn scroll_offset(&self)-> f32  { self.state.scroll_offset }
    pub fn cursor(&self)       -> usize { self.state.cursor() }

    // ── Events ────────────────────────────────────────────────────────────────

    /// Returns `true` if this widget claimed the event.
    pub fn on_pointer_down_with(&mut self, x: f32, y: f32, char_width: f32) -> bool {
        if !self.rect.contains(x, y) { return false; }
        let relative_x = (x - self.text_x + self.state.scroll_offset).max(0.0);
        self.state.set_cursor_from_x(relative_x, char_width, false);
        true
    }

    pub fn on_drag(&mut self, x: f32, char_width: f32) {
        let relative_x = (x - self.text_x + self.state.scroll_offset).max(0.0);
        self.state.set_cursor_from_x(relative_x, char_width, true);
    }

    pub fn ensure_cursor_visible(&mut self, char_width: f32) {
        let padding       = self.text_x - self.rect.x;
        let visible_width = (self.rect.width - padding * 2.0).max(0.0);
        self.state.ensure_cursor_visible(visible_width, char_width);
    }

    pub fn cursor_shape_at(&self, x: f32, y: f32) -> CursorShape {
        if self.rect.contains(x, y) { CursorShape::Text } else { CursorShape::Default }
    }

    // ── Geometry helpers ──────────────────────────────────────────────────────

    pub fn cursor_rect(&self, char_width: f32) -> Rect {
        let byte_pos = self.state.cursor();
        let chars    = self.state.text()[..byte_pos].chars().count();
        let x        = self.text_x + chars as f32 * char_width - self.state.scroll_offset;
        let v_pad    = 4.0;
        Rect { x, y: self.rect.y + v_pad, width: 2.0, height: self.rect.height - v_pad * 2.0 }
    }

    pub fn selection_rect(&self, char_width: f32) -> Option<Rect> {
        let (start, end)  = self.state.selection_range()?;
        let start_chars   = self.state.text()[..start].chars().count();
        let end_chars     = self.state.text()[..end].chars().count();
        let x = self.text_x + start_chars as f32 * char_width - self.state.scroll_offset;
        let w = (end_chars - start_chars) as f32 * char_width;
        let v_pad = 3.0;
        Some(Rect { x, y: self.rect.y + v_pad, width: w, height: self.rect.height - v_pad * 2.0 })
    }
}
