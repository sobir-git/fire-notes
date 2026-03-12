//! Scrollable list view widget — geometry-owning counterpart to `ListWidget<T>`.
//!
//! `ListWidget<T>` owns selection/filter/scroll *state* with no geometry.
//! `ListViewWidget` owns the *screen space*: item rects, visible count, and the
//! embedded scrollbar. Together they form a complete scrollable list.
//!
//! Usage pattern:
//! ```
//! // Layout phase (given a rect from the parent):
//! let list_view = ListViewWidget::layout(list_rect, scale);
//!
//! // At render/interaction time (state comes from ListWidget):
//! let visible = list_widget.len();
//! let offset  = list_widget.scroll_offset();
//! for display_idx in 0..list_view.visible_count(visible) {
//!     let row = list_view.item_rect(display_idx);
//!     // draw row...
//! }
//! // Scrollbar thumb:
//! if let Some(thumb) = list_view.scrollbar_thumb(visible, offset) { ... }
//! ```

use super::layout::Layout;
use super::scrollbar::{ScrollbarAction, ScrollbarWidget, ThumbMetrics};
use super::types::Rect;
use crate::config::layout as cfg;

/// A geometry-owning scrollable list view.
///
/// Implements `Layout` — receives its rect from the parent.
/// Pairs with `ListWidget<T>` which holds the data/selection state.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct ListViewWidget {
    /// Full rect including the scrollbar strip.
    pub rect: Rect,
    /// List content area (rect minus scrollbar).
    pub list_rect: Rect,
    /// Scrollbar geometry.
    pub scrollbar: ScrollbarWidget,
    /// Height of one row in screen pixels.
    pub item_height: f32,
    /// Scale factor.
    pub scale: f32,
}

impl Layout for ListViewWidget {
    fn layout(rect: Rect, scale: f32) -> Self {
        let sb_w = cfg::SCROLLBAR_WIDTH * scale;
        let item_height = 32.0 * scale;
        // Split: list content | scrollbar strip
        let rects = rect.split_h(&[1.0, -sb_w]);
        let list_rect = rects[0];
        let sb_rect   = rects[1];
        Self {
            rect,
            list_rect,
            scrollbar: ScrollbarWidget::layout(sb_rect, scale),
            item_height,
            scale,
        }
    }
}

#[allow(dead_code)]
impl ListViewWidget {
    /// Number of fully visible rows in the available height.
    pub fn visible_count(&self, total_items: usize) -> usize {
        let max = (self.list_rect.height / self.item_height).floor() as usize;
        max.min(total_items)
    }

    /// Screen rect for the `display_idx`-th visible row (0-based from scroll_offset).
    pub fn item_rect(&self, display_idx: usize) -> Rect {
        Rect {
            x:      self.list_rect.x,
            y:      self.list_rect.y + display_idx as f32 * self.item_height,
            width:  self.list_rect.width,
            height: self.item_height,
        }
    }

    /// Text baseline Y for `display_idx` (vertically centred, cap-height offset).
    pub fn item_baseline_y(&self, display_idx: usize, font_size: f32) -> f32 {
        let r = self.item_rect(display_idx);
        r.y + self.item_height / 2.0 + font_size * 0.35
    }

    /// Hit-test: returns the `display_idx` of the row under (x, y), if any.
    /// `scroll_offset` and `total_items` are from the paired `ListWidget`.
    pub fn hit_test_item(&self, x: f32, y: f32, total_items: usize) -> Option<usize> {
        if !self.list_rect.contains(x, y) { return None; }
        let display_idx = ((y - self.list_rect.y) / self.item_height).floor() as usize;
        if display_idx < self.visible_count(total_items) {
            Some(display_idx)
        } else {
            None
        }
    }

    /// Whether the scrollbar is needed and active.
    pub fn is_scrollable(&self, total_items: usize) -> bool {
        let visible = (self.list_rect.height / self.item_height).floor() as usize;
        self.scrollbar.is_scrollable(total_items, visible)
    }

    /// Scrollbar thumb geometry for a list (max scroll = total - visible, not total - 1).
    /// `total_items` and `scroll_offset` come from `ListWidget`.
    pub fn scrollbar_thumb(&self, total_items: usize, scroll_offset: usize) -> Option<ThumbMetrics> {
        let visible = (self.list_rect.height / self.item_height).floor() as usize;
        if !self.scrollbar.is_scrollable(total_items, visible) {
            return None;
        }
        let track_h    = self.scrollbar.rect.height;
        let view_ratio = visible as f32 / total_items as f32;
        let min_thumb  = crate::config::layout::MIN_SCROLLBAR_THUMB * self.scale;
        let thumb_h    = (track_h * view_ratio).max(min_thumb);

        let max_scroll = total_items.saturating_sub(visible);
        let scroll_ratio = if max_scroll > 0 {
            (scroll_offset as f32 / max_scroll as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let track_space = (track_h - thumb_h).max(0.0);
        let thumb_y     = self.scrollbar.rect.y + track_space * scroll_ratio;
        Some(ThumbMetrics {
            rect: Rect {
                x:      self.scrollbar.rect.x,
                y:      thumb_y,
                width:  self.scrollbar.rect.width,
                height: thumb_h,
            },
        })
    }

    /// Handle a click on the scrollbar.
    pub fn scrollbar_click(&self, x: f32, y: f32, total_items: usize, scroll_offset: usize) -> ScrollbarAction {
        let visible = (self.list_rect.height / self.item_height).floor() as usize;
        self.scrollbar.on_click(x, y, total_items, visible, scroll_offset)
    }

    /// Compute drag ratio while dragging the scrollbar thumb.
    pub fn scrollbar_drag_ratio(&self, y: f32, total_items: usize, scroll_offset: usize, drag_offset: f32) -> Option<f32> {
        let visible = (self.list_rect.height / self.item_height).floor() as usize;
        self.scrollbar.drag_ratio(y, total_items, visible, drag_offset, scroll_offset)
    }
}
