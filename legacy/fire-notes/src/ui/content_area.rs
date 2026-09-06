//! Content area — the region below the tab bar.
//!
//! Owns its two children: `text` (`TextArea`) and `scrollbar` (`ScrollbarWidget`).
//! Layout is expressed entirely via `Rect` split primitives — no raw arithmetic.

use crate::config::layout as cfg_layout;
use crate::primitives::Scrollbar;
use super::layout::Layout;
use super::text_area::TextArea;
use super::types::Rect;

/// The scrollable content region.
///
/// Split hierarchy:
///   window → cut_top(tab_h) → content rect
///   content rect → split_h([fill, -sb_w]) → text | scrollbar
pub struct ContentArea {
    /// Full content rect (below tab bar + top padding).
    pub rect: Rect,
    /// Text column widget.
    pub text: TextArea,
    /// Scrollbar widget — owns geometry + drag state.
    pub scrollbar: Scrollbar,
    /// Scaled line height (kept here for visible_line_count).
    pub line_height: f32,
    /// Hover state — updated by UiTree::on_hover.
    pub scrollbar_hovered: bool,
    /// True while a text selection drag is in progress.
    pub is_text_selecting: bool,
}

impl Layout for ContentArea {
    fn layout(rect: Rect, scale: f32) -> Self {
        let scrollbar_width = cfg_layout::SCROLLBAR_WIDTH * scale;
        let line_height     = cfg_layout::LINE_HEIGHT     * scale;

        let cols = rect.split_h(&[1.0, -scrollbar_width]);
        Self {
            rect,
            text:      TextArea::layout(cols[0], scale),
            scrollbar: Scrollbar::new(cols[1], scale),
            line_height,
            scrollbar_hovered: false,
            is_text_selecting: false,
        }
    }
}

impl ContentArea {
    /// Update scrollbar hover state. Returns `true` if changed.
    pub fn on_hover(&mut self, x: f32, y: f32, total_lines: usize, visible_lines: usize) -> bool {
        let hovered = self.scrollbar.hit_test(x, y)
            && self.scrollbar.is_scrollable(total_lines, visible_lines);
        let changed = hovered != self.scrollbar_hovered;
        self.scrollbar_hovered = hovered;
        changed
    }

    /// Number of fully visible text lines.
    pub fn visible_line_count(&self) -> usize {
        (self.rect.height / self.line_height).floor().max(1.0) as usize
    }

}
