//! Content area — the region below the tab bar.
//!
//! Owns its two children: `text` (`TextArea`) and `scrollbar` (`ScrollbarWidget`).
//! Layout is expressed entirely via `Rect` split primitives — no raw arithmetic.

use crate::config::layout as cfg_layout;
use super::layout::Layout;
use super::scrollbar::ScrollbarWidget;
use super::text_area::TextArea;
use super::types::Rect;

/// The scrollable content region.
///
/// Split hierarchy:
///   window → cut_top(tab_h) → content rect
///   content rect → split_h([fill, -sb_w]) → text | scrollbar
#[derive(Debug, Clone, Copy)]
pub struct ContentArea {
    /// Full content rect (below tab bar + top padding).
    pub rect: Rect,
    /// Text column widget.
    pub text: TextArea,
    /// Scrollbar widget.
    pub scrollbar: ScrollbarWidget,
    /// Scaled line height (kept here for visible_line_count).
    pub line_height: f32,
}

impl Layout for ContentArea {
    fn layout(rect: Rect, scale: f32) -> Self {
        let scrollbar_width = cfg_layout::SCROLLBAR_WIDTH * scale;
        let line_height     = cfg_layout::LINE_HEIGHT     * scale;

        let cols = rect.split_h(&[1.0, -scrollbar_width]);
        Self {
            rect,
            text:      TextArea::layout(cols[0], scale),
            scrollbar: ScrollbarWidget::layout(cols[1], scale),
            line_height,
        }
    }
}

impl ContentArea {
    /// Number of fully visible text lines.
    pub fn visible_line_count(&self) -> usize {
        (self.rect.height / self.line_height).floor().max(1.0) as usize
    }

    /// Y coordinate where line 0 renders (rect.y + doc top margin, scrolls away with content).
    pub fn start_y(&self) -> f32 {
        self.text.rect.y + self.text.doc_top_margin
    }

    /// Horizontal text padding (left margin).
    pub fn text_padding(&self, _scale: f32) -> f32 {
        self.text.text_padding
    }

}
