//! Notes picker overlay — layout and hit-testing.
//!
//! Implements `Layout`: given the window rect, computes all geometry via
//! `Rect` primitives. No inline arithmetic in renderers.

use super::layout::Layout;
use super::types::{CursorShape, Rect};

pub const MAX_VISIBLE_ITEMS: usize = 8;

/// Layout for a single list item row.
#[derive(Debug, Clone, Copy)]
pub struct PickerItemMetrics {
    pub row_rect: Rect,
    pub text_baseline_y: f32,
    pub indicator_x: f32,
}

/// Complete layout for the notes picker overlay.
#[derive(Debug, Clone)]
pub struct NotesPicker {
    /// Full window — semi-transparent backdrop.
    pub backdrop_rect: Rect,
    /// Floating panel rect.
    pub overlay_rect: Rect,
    /// Search input box.
    pub input_rect: Rect,
    /// X where input text starts (left padding inside input).
    pub input_text_x: f32,
    /// Y baseline for input text.
    pub input_text_baseline_y: f32,
    /// Y where the list area starts.
    pub list_y: f32,
    /// Height of one list row.
    pub item_height: f32,
    /// Font size for all picker text.
    pub font_size: f32,
    /// Scale factor.
    pub scale: f32,
    /// X of the right-side "open" indicator dot.
    indicator_x: f32,
    /// Number of visible items this layout was computed for.
    visible_items: usize,
}

impl NotesPicker {
    /// Build from window dimensions, scale, and current list length.
    /// Uses `Layout::layout` internally after computing the overlay rect.
    pub fn new(window_width: f32, window_height: f32, scale: f32, list_len: usize) -> Self {
        let window = Rect { x: 0.0, y: 0.0, width: window_width, height: window_height };
        let mut picker = Self::layout(window, scale);
        // Re-layout with actual list length to get correct overlay height.
        picker.resize_for_list(list_len);
        picker
    }

    fn resize_for_list(&mut self, list_len: usize) {
        let visible = list_len.min(MAX_VISIBLE_ITEMS);
        if visible == self.visible_items { return; }
        let list_height = visible as f32 * self.item_height;
        let padding = self.scale * 8.0;
        let input_height = self.scale * 36.0;
        self.overlay_rect.height = input_height + list_height + 2.0 * padding;
        self.visible_items = visible;
    }

    /// Metrics for the `display_idx`-th visible row.
    pub fn item_metrics(&self, display_idx: usize) -> PickerItemMetrics {
        let row_rect = Rect {
            x:      self.input_rect.x,
            y:      self.list_y + display_idx as f32 * self.item_height,
            width:  self.input_rect.width,
            height: self.item_height - 2.0 * self.scale,
        };
        PickerItemMetrics {
            row_rect,
            text_baseline_y: row_rect.y + self.item_height / 2.0 + self.font_size * 0.35,
            indicator_x: self.indicator_x,
        }
    }

    /// Hit-test: returns display index of item under (x, y), if any.
    pub fn item_hit_test(&self, x: f32, y: f32, visible_count: usize) -> Option<usize> {
        if !self.overlay_rect.contains(x, y) { return None; }
        (0..visible_count).find(|&i| self.item_metrics(i).row_rect.contains(x, y))
    }

    /// Cursor shape appropriate for the area under (x, y).
    /// Text cursor over the search input, Default everywhere else.
    pub fn cursor_shape_at(&self, x: f32, y: f32) -> CursorShape {
        if self.input_rect.contains(x, y) {
            CursorShape::Text
        } else {
            CursorShape::Default
        }
    }
}

impl Layout for NotesPicker {
    /// Layout from the full window rect.
    /// List height defaults to `MAX_VISIBLE_ITEMS`; call `new()` for
    /// the correct height based on actual list length.
    fn layout(window: Rect, scale: f32) -> Self {
        let padding      = 8.0  * scale;
        let input_height = 36.0 * scale;
        let item_height  = 32.0 * scale;
        let font_size    = 14.0 * scale;

        // Overlay: 60% of window width, capped at 500 logical px, dropped 60px from top.
        let overlay_w = (window.width * 0.6).min(500.0 * scale);
        let overlay_h = input_height + MAX_VISIBLE_ITEMS as f32 * item_height + 2.0 * padding;
        let (_, below_top) = window.cut_top(60.0 * scale);
        let overlay_rect   = below_top.centered_in(overlay_w, overlay_h);

        // Input box: overlay inset by padding, input_height tall.
        let (input_strip, _) = overlay_rect.inset(padding).cut_top(input_height - 4.0 * scale);
        let input_rect = input_strip;

        // Indicator dot: right edge of overlay, offset inward.
        let (ind_strip, _) = overlay_rect.cut_right(2.0 * padding + 4.0 * scale);
        let indicator_x = ind_strip.x;

        Self {
            backdrop_rect: window,
            overlay_rect,
            input_rect,
            input_text_x: input_rect.x + padding,
            input_text_baseline_y: input_rect.y + input_rect.height / 2.0 + font_size * 0.35,
            list_y: input_rect.y + input_height + 4.0 * scale,
            item_height,
            font_size,
            scale,
            indicator_x,
            visible_items: MAX_VISIBLE_ITEMS,
        }
    }
}
