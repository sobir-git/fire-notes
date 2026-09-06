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
    /// Top margin before line 0 — part of the document, scrolls away with content.
    pub doc_top_margin: f32,
}

#[allow(dead_code)]
impl TextArea {}

impl Layout for TextArea {
    fn layout(rect: Rect, scale: f32) -> Self {
        Self {
            rect,
            line_height:    layout::LINE_HEIGHT    * scale,
            char_width:     0.0,   // set after font measurement via set_char_width()
            text_padding:   layout::PADDING        * scale,
            doc_top_margin: layout::DOC_TOP_MARGIN * scale,
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

    /// Y coordinate for a visual line index (0 = first visible line at current scroll).
    /// This is the single source of truth — both `char_rect` and the renderer use this.
    pub fn line_y(&self, visual_line: usize) -> f32 {
        self.rect.y + self.doc_top_margin + visual_line as f32 * self.line_height
    }

    pub fn hit_test(&self, x: f32, y: f32) -> bool {
        self.rect.contains(x, y)
    }

    /// Number of fully visible text lines.
    #[allow(dead_code)]
    pub fn visible_line_count(&self) -> usize {
        (self.rect.height / self.line_height).floor().max(1.0) as usize
    }

    /// Clamps out-of-bounds Y to a signed visual line index,
    /// useful for selection dragging that extends above/below the viewport.
    pub fn hit_to_visual_line_clamped(
        &self,
        y: f32,
    ) -> isize {
        let rel_y = y - self.rect.y;
        (rel_y / self.line_height).floor() as isize
    }

    /// Convert a mouse position to a document `(line, col)`, handling all edge cases:
    /// - Clamps `visual_line` to `[0, total_lines)` so clicks below the last line land there.
    /// - When below the last line, returns `(last_line, line_char_len)` via the `below_last`
    ///   flag so the caller can set col = end-of-line without knowing line lengths here.
    /// - Uses `visual_col_to_char_col` indirection: returns the raw visual col; callers
    ///   use `Tab::visual_col_to_char_col` to get the char col.
    ///
    /// Returns `(raw_line, visual_col, below_last_line)`.
    pub fn hit_to_doc_position(
        &self,
        x: f32,
        y: f32,
        scroll_offset: usize,
        scroll_x: f32,
        char_width: f32,
        total_lines: usize,
    ) -> (usize, usize, bool) {
        let rel_y = (y - self.rect.y).max(0.0);
        let visual_line = (rel_y / self.line_height).floor() as usize;
        let raw_line = scroll_offset + visual_line;
        let below_last = raw_line >= total_lines;
        let line = raw_line.min(total_lines.saturating_sub(1));
        let rel_x = (x - self.rect.x - self.text_padding + scroll_x).max(0.0);
        let visual_col = (rel_x / char_width.max(1.0)).round() as usize;
        (line, visual_col, below_last)
    }

    /// Screen rect of the character at (`line`, `col`) given the current `scroll_offset`.
    ///
    /// This is the source of truth for character positions — the flame system
    /// calls this instead of re-measuring text in the renderer.
    #[allow(dead_code)]
    pub fn char_rect(&self, line: usize, col: usize, scroll_offset: usize) -> Rect {
        let visual_line = line.saturating_sub(scroll_offset);
        let x = self.rect.x + self.text_padding + col as f32 * self.char_width;
        let y = self.line_y(visual_line);
        Rect { x, y, width: self.char_width.max(1.0), height: self.line_height }
    }
}
