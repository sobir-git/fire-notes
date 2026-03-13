#![allow(dead_code)]
//! List primitive — selection state + scrollable geometry + events, one file.
//!
//! Replaces `ui::ListWidget<T>` (state) + `ui::ListViewWidget` (geometry) + `fw::List<T>` (merged).

use crate::config::layout as cfg;
use crate::layout::Widget;
use crate::ui::{CursorShape, Rect};
use super::scrollbar::{Scrollbar, ScrollbarAction};

// ── Row geometry ────────────────────────────────────────────────────────────

/// Geometry for one visible row, passed to snapshot closures.
#[derive(Debug, Clone, Copy)]
pub struct RowGeometry {
    /// Row bounding rect.
    pub rect:       crate::ui::Rect,
    /// Text baseline Y (rect.y + height * 0.65).
    pub baseline_y: f32,
    /// Center Y (rect.y + height * 0.5).
    pub center_y:   f32,
    /// True if this row is the selected one.
    pub is_selected: bool,
    /// 0-based display index within the visible window.
    pub display_index: usize,
}

// ── Event type ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ListPointerResult {
    None,
    Selected,
    Confirmed(usize),
    Scrolled,
    ScrollbarDragStart { drag_offset: f32 },
}

// ── Primitive ─────────────────────────────────────────────────────────────────

/// Scrollable list — owns items, selection, filter, geometry, and scrollbar drag.
#[derive(Debug, Clone)]
pub struct List<T> {
    items:            Vec<T>,
    filtered_indices: Vec<usize>,
    selected_index:   usize,
    scroll_offset:    usize,
    max_visible:      usize,

    // ── Geometry ──────────────────────────────────────────────────────────────
    rect:        Rect,
    list_rect:   Rect,
    scrollbar:   Scrollbar,
    item_height: f32,
    scale:       f32,
}

impl<T> List<T> {
    /// Create with items. Geometry is zeroed until `relayout()` is called.
    pub fn new(items: Vec<T>) -> Self {
        let n = items.len();
        Self {
            items,
            filtered_indices: (0..n).collect(),
            selected_index:   0,
            scroll_offset:    0,
            max_visible:      10,
            rect:        Rect::ZERO,
            list_rect:   Rect::ZERO,
            scrollbar:   Scrollbar::new(Rect::ZERO, 1.0),
            item_height: 32.0,
            scale:       1.0,
        }
    }

    /// Update geometry, preserving all state.
    pub fn relayout(&mut self, rect: Rect, scale: f32) {
        let sb_w        = cfg::SCROLLBAR_WIDTH * scale;
        let item_height = 32.0 * scale;
        let rects       = rect.split_h(&[1.0, -sb_w]);
        let list_rect   = rects[0];
        let sb_rect     = rects[1];
        self.rect        = rect;
        self.list_rect   = list_rect;
        self.scrollbar.relayout(sb_rect, scale);
        self.item_height = item_height;
        self.scale       = scale;
    }

    pub fn set_max_visible(&mut self, max: usize) { self.max_visible = max.max(1); }

    // ── State accessors ───────────────────────────────────────────────────────

    pub fn items(&self)            -> &[T]    { &self.items }
    pub fn filtered_indices(&self) -> &[usize] { &self.filtered_indices }
    pub fn selected_index(&self)   -> usize   { self.selected_index }
    pub fn scroll_offset(&self)    -> usize   { self.scroll_offset }
    pub fn len(&self)              -> usize   { self.filtered_indices.len() }
    pub fn is_empty(&self)         -> bool    { self.filtered_indices.is_empty() }

    pub fn selected_item(&self) -> Option<&T> {
        self.filtered_indices.get(self.selected_index).and_then(|&i| self.items.get(i))
    }

    pub fn selected_original_index(&self) -> Option<usize> {
        self.filtered_indices.get(self.selected_index).copied()
    }

    // ── Navigation ────────────────────────────────────────────────────────────

    pub fn select_up(&mut self) -> bool {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.ensure_visible();
            true
        } else { false }
    }

    pub fn select_down(&mut self) -> bool {
        if !self.filtered_indices.is_empty() && self.selected_index < self.filtered_indices.len() - 1 {
            self.selected_index += 1;
            self.ensure_visible();
            true
        } else { false }
    }

    pub fn select_index(&mut self, index: usize) -> bool {
        if index < self.filtered_indices.len() {
            self.selected_index = index;
            self.ensure_visible();
            true
        } else { false }
    }

    pub fn scroll_to(&mut self, offset: usize) {
        let max = self.filtered_indices.len().saturating_sub(1);
        self.scroll_offset = offset.min(max);
    }

    pub fn scroll_by(&mut self, lines: isize) -> bool {
        if lines > 0 {
            (0..lines as usize).fold(false, |c, _| self.select_down() || c)
        } else {
            (0..(-lines) as usize).fold(false, |c, _| self.select_up() || c)
        }
    }

    // ── Filter ────────────────────────────────────────────────────────────────

    pub fn filter<F: Fn(&T) -> bool>(&mut self, pred: F) {
        self.filtered_indices = self.items.iter().enumerate()
            .filter(|(_, item)| pred(item))
            .map(|(i, _)| i)
            .collect();
        self.selected_index = 0;
        self.scroll_offset  = 0;
    }

    pub fn clear_filter(&mut self) {
        self.filtered_indices = (0..self.items.len()).collect();
        self.selected_index   = 0;
        self.scroll_offset    = 0;
    }

    // ── Geometry accessors ────────────────────────────────────────────────────

    pub fn visible_count(&self) -> usize {
        let max = (self.list_rect.height / self.item_height).floor() as usize;
        max.min(self.filtered_indices.len()).min(self.max_visible)
    }

    pub fn item_rect(&self, display_idx: usize) -> Rect {
        Rect {
            x:      self.list_rect.x,
            y:      self.list_rect.y + display_idx as f32 * self.item_height,
            width:  self.list_rect.width,
            height: self.item_height,
        }
    }

    pub fn item_baseline_y(&self, display_idx: usize, font_size: f32) -> f32 {
        self.list_rect.y + display_idx as f32 * self.item_height + self.item_height / 2.0 + font_size * 0.35
    }

    pub fn item_center_y(&self, display_idx: usize) -> f32 {
        self.list_rect.y + (display_idx as f32 + 0.5) * self.item_height
    }

    pub fn scrollbar_rect(&self) -> Rect { self.scrollbar.rect }

    /// Returns `(track_rect, thumb_rect)` when scrollbar is visible.
    pub fn scrollbar_rects(&self) -> Option<(Rect, Rect)> {
        let total   = self.filtered_indices.len();
        let visible = self.visible_count();
        let thumb   = self.scrollbar.list_thumb(total, visible, self.scroll_offset)?;
        Some((self.scrollbar.rect, thumb.rect))
    }

    pub fn list_rect(&self) -> Rect { self.list_rect }

    /// Iterate visible rows, calling `f(item, geometry)` for each.
    /// Returns the mapped results as a `Vec<R>`.
    /// This is the canonical way to build row snapshot data without
    /// duplicating the scroll/select/index arithmetic in callers.
    pub fn visible_rows_snapshot<R, F>(&self, f: F) -> Vec<R>
    where
        F: Fn(&T, RowGeometry) -> R,
    {
        let scroll   = self.scroll_offset;
        let selected = self.selected_index;
        let visible  = self.visible_count();
        let item_h   = self.item_height;

        self.filtered_indices.iter()
            .skip(scroll)
            .take(visible)
            .enumerate()
            .filter_map(|(di, &fi)| {
                let item = self.items.get(fi)?;
                let y    = self.list_rect.y + di as f32 * item_h;
                let geo  = RowGeometry {
                    rect:          Rect { x: self.list_rect.x, y, width: self.list_rect.width, height: item_h },
                    baseline_y:    y + item_h * 0.65,
                    center_y:      y + item_h * 0.5,
                    is_selected:   scroll + di == selected,
                    display_index: di,
                };
                Some(f(item, geo))
            })
            .collect()
    }

    // ── Events ────────────────────────────────────────────────────────────────

    pub fn on_pointer_down(&mut self, x: f32, y: f32) -> ListPointerResult {
        let total   = self.filtered_indices.len();
        let visible = self.visible_count();
        let offset  = self.scroll_offset;

        if self.scrollbar.hit_test(x, y) {
            match self.scrollbar.on_click(x, y, total, visible, offset) {
                ScrollbarAction::StartDrag { drag_offset } => {
                    return ListPointerResult::ScrollbarDragStart { drag_offset };
                }
                ScrollbarAction::JumpTo { ratio } => {
                    let max_scroll = total.saturating_sub(visible);
                    let target = (ratio.clamp(0.0, 1.0) * max_scroll as f32).round() as usize;
                    self.scroll_to(target);
                    return ListPointerResult::Scrolled;
                }
                ScrollbarAction::None => {}
            }
        }

        if self.list_rect.contains(x, y) {
            let display_idx = ((y - self.list_rect.y) / self.item_height).floor() as usize;
            if display_idx < visible {
                let abs_idx    = offset + display_idx;
                let was_selected = self.selected_index == abs_idx;
                self.select_index(abs_idx);
                return if was_selected {
                    ListPointerResult::Confirmed(abs_idx)
                } else {
                    ListPointerResult::Selected
                };
            }
        }

        ListPointerResult::None
    }

    pub fn continue_scrollbar_drag(&mut self, y: f32) -> bool {
        let Some(drag_offset) = self.scrollbar.drag_offset_value() else { return false; };
        let total   = self.filtered_indices.len();
        let visible = self.visible_count();
        let offset  = self.scroll_offset;
        if let Some(ratio) = self.scrollbar.list_drag_ratio(y, total, visible, offset, drag_offset) {
            let max_scroll = total.saturating_sub(visible);
            let target = (ratio.clamp(0.0, 1.0) * max_scroll as f32).round() as usize;
            self.scroll_to(target);
            return true;
        }
        false
    }

    pub fn end_drag(&mut self) { self.scrollbar.end_drag(); }

    pub fn is_scrollbar_dragging(&self) -> bool { self.scrollbar.is_dragging() }

    pub fn on_hover(&mut self, x: f32, y: f32) -> bool {
        let total   = self.filtered_indices.len();
        let visible = self.visible_count();
        let offset  = self.scroll_offset;
        if self.list_rect.contains(x, y) {
            let display_idx = ((y - self.list_rect.y) / self.item_height).floor() as usize;
            if display_idx < visible {
                let target = offset + display_idx;
                if total > 0 && target < total && self.selected_index != target {
                    self.select_index(target);
                    return true;
                }
            }
        }
        false
    }

    // ── Private ───────────────────────────────────────────────────────────────

    fn ensure_visible(&mut self) {
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + self.max_visible {
            self.scroll_offset = self.selected_index - self.max_visible + 1;
        }
    }
}

// ── Widget impl ───────────────────────────────────────────────────────────────

impl<T: Clone> Widget for List<T> {
    type Event = ListPointerResult;
    fn on_pointer_down(&mut self, x: f32, y: f32) -> ListPointerResult {
        self.on_pointer_down(x, y)
    }
    fn on_hover(&mut self, x: f32, y: f32) -> bool {
        self.on_hover(x, y)
    }
    fn cursor_shape_at(&self, _x: f32, _y: f32) -> CursorShape {
        CursorShape::Default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection() {
        let mut list: List<&str> = List::new(vec!["a", "b", "c", "d"]);
        assert_eq!(list.selected_index(), 0);
        assert!(list.select_down());
        assert_eq!(list.selected_index(), 1);
        assert!(list.select_down());
        assert_eq!(list.selected_index(), 2);
        assert!(list.select_up());
        assert_eq!(list.selected_index(), 1);
        list.select_index(3);
        assert_eq!(list.selected_index(), 3);
        assert!(!list.select_down()); // already at end
        assert_eq!(list.selected_index(), 3);
    }

    #[test]
    fn test_filter() {
        let mut list: List<&str> = List::new(vec!["apple", "banana", "apricot", "cherry"]);
        assert_eq!(list.len(), 4);
        list.filter(|s| s.starts_with('a'));
        assert_eq!(list.len(), 2);
        assert_eq!(list.selected_index(), 0);
        assert_eq!(list.selected_item(), Some(&"apple"));
        assert!(list.select_down());
        assert_eq!(list.selected_item(), Some(&"apricot"));
        list.clear_filter();
        assert_eq!(list.len(), 4);
        assert_eq!(list.selected_index(), 0);
    }
}
