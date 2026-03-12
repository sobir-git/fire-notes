//! Text area widget — owns the text column rect and character geometry.
//!
//! `char_rect` is the key method: it returns the screen rect of any
//! character, enabling the flame system to emit particles at exact glyph
//! positions without re-measuring text in the renderer.

use crate::config::layout;
use super::content_area::ContentArea;
use super::layout::Layout;
use super::types::Rect;

#[derive(Debug, Clone, Copy)]
pub struct TextArea {
    pub rect: Rect,
    pub line_height: f32,
    pub char_width: f32,
    pub text_padding: f32,
}

#[allow(dead_code)]
impl TextArea {}

impl Layout for TextArea {
    fn layout(rect: Rect, scale: f32) -> Self {
        Self {
            rect,
            line_height:  layout::LINE_HEIGHT * scale,
            char_width:   0.0,   // set after font measurement via set_char_width()
            text_padding: layout::PADDING * scale,
        }
    }
}

impl TextArea {
    /// Construct from a `ContentArea` — the canonical path via the layout hierarchy.
    #[allow(dead_code)]
    pub fn from_content_area(content_area: &ContentArea, scale: f32) -> Self {
        Self::layout(content_area.rect, scale)
    }

    /// Update the measured character width (called once after font metrics are known).
    #[allow(dead_code)]
    pub fn set_char_width(&mut self, width: f32) {
        self.char_width = width;
    }

    pub fn hit_test(&self, x: f32, y: f32) -> bool {
        self.rect.contains(x, y)
    }

    /// Number of fully visible text lines.
    #[allow(dead_code)]
    pub fn visible_line_count(&self) -> usize {
        (self.rect.height / self.line_height).floor().max(1.0) as usize
    }

    /// Screen rect of the character at (`line`, `col`) given the current `scroll_offset`.
    ///
    /// This is the source of truth for character positions — the flame system
    /// calls this instead of re-measuring text in the renderer.
    #[allow(dead_code)]
    pub fn char_rect(&self, line: usize, col: usize, scroll_offset: usize) -> Rect {
        let visual_line = line.saturating_sub(scroll_offset);
        let x = self.rect.x + self.text_padding + col as f32 * self.char_width;
        let y = self.rect.y + visual_line as f32 * self.line_height;
        Rect { x, y, width: self.char_width.max(1.0), height: self.line_height }
    }
}
