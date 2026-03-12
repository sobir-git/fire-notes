//! Single-line text input widget.
//!
//! Two types:
//! - `TextInput`       — pure text editing state (cursor, selection, text). No geometry.
//! - `TextInputWidget` — `TextInput` + screen `Rect` via `Layout`. Owns hit-testing and
//!                       cursor-shape logic. Use this wherever a text field appears in the UI.

mod cursor;
mod edit;

use super::layout::Layout;
use super::types::{CursorShape, Rect};

/// A text input field widget: state + geometry.
///
/// Construct via `TextInputWidget::layout(rect, scale)` or by calling
/// `TextInputWidget::new(rect, scale, initial_text)`.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TextInputWidget {
    /// Screen rect this field occupies.
    pub rect: Rect,
    /// Font size derived from scale.
    pub font_size: f32,
    /// X where text starts (left padding).
    pub text_x: f32,
    /// Y baseline for text rendering.
    pub text_baseline_y: f32,
    /// The text editing state.
    pub input: TextInput,
}

impl Layout for TextInputWidget {
    fn layout(rect: Rect, scale: f32) -> Self {
        let padding   = 8.0 * scale;
        let font_size = 14.0 * scale;
        Self {
            rect,
            font_size,
            text_x:          rect.x + padding,
            text_baseline_y: rect.y + rect.height / 2.0 + font_size * 0.35,
            input: TextInput::new(String::new()),
        }
    }
}

#[allow(dead_code)]
impl TextInputWidget {
    /// Construct with an initial text value.
    pub fn with_text(rect: Rect, scale: f32, text: String) -> Self {
        let mut w = Self::layout(rect, scale);
        w.input = TextInput::new(text);
        w
    }

    /// Cursor shape when the pointer is at (x, y).
    /// Always `Text` when inside the field, `Default` outside.
    pub fn cursor_shape_at(&self, x: f32, y: f32) -> CursorShape {
        if self.rect.contains(x, y) { CursorShape::Text } else { CursorShape::Default }
    }

    /// Convert a screen x-coordinate to a byte offset into the text.
    /// Returns the nearest cursor position for use with `set_cursor_from_x`.
    pub fn x_to_cursor(&self, x: f32, char_width: f32) -> usize {
        let relative_x = (x - self.text_x + self.input.scroll_offset).max(0.0);
        let char_index = (relative_x / char_width).round() as usize;
        let mut byte_idx = 0;
        for (i, ch) in self.input.text.chars().enumerate() {
            if i >= char_index { break; }
            byte_idx += ch.len_utf8();
        }
        byte_idx.min(self.input.text.len())
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TextInput {
    pub text: String,
    pub cursor: usize,
    pub selection_anchor: Option<usize>,
    pub scroll_offset: f32,
}

#[allow(dead_code)]
impl TextInput {
    pub fn new(text: String) -> Self {
        let cursor = text.len();
        Self { text, cursor, selection_anchor: None, scroll_offset: 0.0 }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn selection_range(&self) -> Option<(usize, usize)> {
        self.selection_anchor.map(|anchor| {
            if anchor < self.cursor {
                (anchor, self.cursor)
            } else {
                (self.cursor, anchor)
            }
        })
    }
}
