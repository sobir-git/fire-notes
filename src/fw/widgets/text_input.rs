#![allow(dead_code)]
//! Merged text-input widget — state + geometry in one retained struct.
//!
//! Unlike the split `ui::TextInput` (state) + `ui::TextInputWidget` (geometry),
//! this type owns both halves. Call `relayout()` when the parent rect changes;
//! all editing state (text, cursor, selection, scroll) is preserved.

use crate::ui::{CursorShape, Rect, TextInput as RawTextInput};

/// Single-line text-input widget.
///
/// Owns editing state AND screen geometry. No external state threading required.
///
/// ```text
/// let mut search = TextInput::new_empty();
/// search.relayout(rect, scale);
/// if search.on_pointer_down(x, y, char_width) { /* focus claimed */ }
/// ```
#[derive(Debug, Clone)]
pub struct TextInput {
    /// All text-editing state (text, cursor, selection, horizontal scroll).
    /// Exposes the full `ui::TextInput` editing API.
    pub state: RawTextInput,

    // ── Geometry (updated by relayout) ──────────────────────────────────────
    /// Bounding rect of the input box.
    pub rect: Rect,
    /// Font size in physical pixels.
    pub font_size: f32,
    /// X-position where text rendering starts (rect.x + padding).
    pub text_x: f32,
    /// Text baseline Y (vertically centred in rect).
    pub text_baseline_y: f32,
    pub scale: f32,
}

impl TextInput {
    /// Create with empty text. Geometry is zeroed until `relayout()` is called.
    pub fn new_empty() -> Self {
        Self {
            state: RawTextInput::new(String::new()),
            rect: Rect::ZERO,
            font_size: 0.0,
            text_x: 0.0,
            text_baseline_y: 0.0,
            scale: 1.0,
        }
    }

    /// Create with pre-populated text. Geometry is zeroed until `relayout()`.
    pub fn new(text: String) -> Self {
        let mut s = Self::new_empty();
        s.state = RawTextInput::new(text);
        s
    }

    /// Update geometry while preserving all editing state.
    /// Call on window resize or when the parent rect changes.
    pub fn relayout(&mut self, rect: Rect, scale: f32) {
        let padding = 8.0 * scale;
        let font_size = 14.0 * scale;
        self.rect = rect;
        self.scale = scale;
        self.font_size = font_size;
        self.text_x = rect.x + padding;
        self.text_baseline_y = rect.y + rect.height / 2.0 + font_size * 0.35;
    }

    // ── Convenience state accessors ──────────────────────────────────────────

    pub fn text(&self) -> &str {
        self.state.text()
    }

    pub fn is_empty(&self) -> bool {
        self.state.text().is_empty()
    }

    pub fn scroll_offset(&self) -> f32 {
        self.state.scroll_offset
    }

    // ── Event handling ───────────────────────────────────────────────────────

    /// Handle a pointer-down event. Returns `true` if this widget claimed focus.
    /// Sets the cursor position from (x, y).
    pub fn on_pointer_down(&mut self, x: f32, y: f32, char_width: f32) -> bool {
        if !self.rect.contains(x, y) {
            return false;
        }
        let relative_x = (x - self.text_x + self.state.scroll_offset).max(0.0);
        self.state.set_cursor_from_x(relative_x, char_width, false);
        true
    }

    /// Handle a drag event (extends the selection from the pointer-down position).
    pub fn on_drag(&mut self, x: f32, char_width: f32) {
        let relative_x = (x - self.text_x + self.state.scroll_offset).max(0.0);
        self.state.set_cursor_from_x(relative_x, char_width, true);
    }

    /// Scroll the text so the cursor is visible inside the input box.
    pub fn ensure_cursor_visible(&mut self, char_width: f32) {
        let padding = self.text_x - self.rect.x;
        let visible_width = (self.rect.width - padding * 2.0).max(0.0);
        self.state.ensure_cursor_visible(visible_width, char_width);
    }

    /// Cursor shape appropriate for the given screen position.
    pub fn cursor_shape_at(&self, x: f32, y: f32) -> CursorShape {
        if self.rect.contains(x, y) {
            CursorShape::Text
        } else {
            CursorShape::Default
        }
    }

    // ── Geometry + state helpers (require both halves) ────────────────────────

    /// Screen rect of the text cursor bar (uses embedded state).
    pub fn cursor_rect(&self, char_width: f32) -> Rect {
        let byte_pos = self.state.cursor();
        let chars = self.state.text()[..byte_pos].chars().count();
        let x = self.text_x + chars as f32 * char_width - self.state.scroll_offset;
        let v_pad = 4.0;
        Rect {
            x,
            y: self.rect.y + v_pad,
            width: 2.0,
            height: self.rect.height - v_pad * 2.0,
        }
    }

    /// Screen rect covering the selected text, or `None` when no selection.
    pub fn selection_rect(&self, char_width: f32) -> Option<Rect> {
        let (start, end) = self.state.selection_range()?;
        let start_chars = self.state.text()[..start].chars().count();
        let end_chars   = self.state.text()[..end].chars().count();
        let x = self.text_x + start_chars as f32 * char_width - self.state.scroll_offset;
        let w = (end_chars - start_chars) as f32 * char_width;
        let v_pad = 3.0;
        Some(Rect {
            x,
            y: self.rect.y + v_pad,
            width: w,
            height: self.rect.height - v_pad * 2.0,
        })
    }
}
