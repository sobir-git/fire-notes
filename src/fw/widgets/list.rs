#![allow(dead_code)]
//! Merged list widget — state + geometry + scrollbar in one retained struct.
//!
//! Replaces the split `ui::ListWidget<T>` (state-only) + `ui::ListViewWidget` (geometry-only)
//! pattern. Call `relayout()` when the parent rect changes; all selection/filter/scroll
//! state is preserved.

use crate::ui::{Layout, ListViewWidget, ListWidget, Rect, ScrollbarAction, ThumbMetrics};

/// Result of a pointer-down event on the list.
#[derive(Debug, Clone, PartialEq)]
pub enum ListPointerResult {
    /// Click landed outside the widget.
    None,
    /// An item was selected (not previously selected).
    Selected,
    /// The already-selected item was clicked again (double-click equivalent for single-click confirm).
    Confirmed(usize),
    /// Scrollbar track was clicked — scroll position jumped.
    Scrolled,
    /// Scrollbar thumb drag started. `drag_offset` is screen-Y within the thumb.
    ScrollbarDragStart { drag_offset: f32 },
}

/// Merged scrollable list widget.
///
/// Owns both the data/selection state (`ListWidget<T>`) and the screen geometry
/// (`ListViewWidget` + scrollbar drag state). No external state threading.
///
/// ```text
/// let mut list = List::new(entries);
/// list.relayout(rect, scale);
/// match list.on_pointer_down(x, y) {
///     ListPointerResult::Confirmed(i) => { /* open item i */ }
///     ListPointerResult::ScrollbarDragStart { drag_offset } => { /* record for drag */ }
///     _ => {}
/// }
/// ```
#[derive(Debug, Clone)]
pub struct List<T> {
    state: ListWidget<T>,
    view: ListViewWidget,
    /// Active scrollbar drag offset, set by `on_pointer_down`, cleared by `end_drag`.
    scrollbar_drag_offset: Option<f32>,
}

impl<T> List<T> {
    /// Create with items. Geometry is zeroed until `relayout()` is called.
    pub fn new(items: Vec<T>) -> Self {
        Self {
            state: ListWidget::new(items),
            view: ListViewWidget::layout(Rect::ZERO, 1.0),
            scrollbar_drag_offset: None,
        }
    }

    /// Update geometry, preserve state.
    pub fn relayout(&mut self, rect: Rect, scale: f32) {
        self.view = ListViewWidget::layout(rect, scale);
    }

    // ── State delegation ─────────────────────────────────────────────────────

    pub fn set_max_visible(&mut self, max: usize) {
        self.state.set_max_visible(max);
    }

    pub fn items(&self) -> &[T] {
        self.state.items()
    }

    pub fn filtered_indices(&self) -> &[usize] {
        self.state.filtered_indices()
    }

    pub fn selected_index(&self) -> usize {
        self.state.selected_index()
    }

    pub fn selected_item(&self) -> Option<&T> {
        self.state.selected_item()
    }

    pub fn scroll_offset(&self) -> usize {
        self.state.scroll_offset()
    }

    pub fn len(&self) -> usize {
        self.state.len()
    }

    pub fn is_empty(&self) -> bool {
        self.state.is_empty()
    }

    pub fn select_up(&mut self) -> bool {
        self.state.select_up()
    }

    pub fn select_down(&mut self) -> bool {
        self.state.select_down()
    }

    pub fn select_index(&mut self, i: usize) -> bool {
        self.state.select_index(i)
    }

    pub fn clear_filter(&mut self) {
        self.state.clear_filter();
    }

    pub fn filter<F: Fn(&T) -> bool>(&mut self, f: F) {
        self.state.filter(f);
    }

    // ── Geometry delegation ──────────────────────────────────────────────────

    pub fn visible_count(&self) -> usize {
        self.view.visible_count(self.state.len())
    }

    pub fn item_rect(&self, display_idx: usize) -> Rect {
        self.view.item_rect(display_idx)
    }

    pub fn item_baseline_y(&self, display_idx: usize, font_size: f32) -> f32 {
        self.view.item_baseline_y(display_idx, font_size)
    }

    pub fn item_center_y(&self, display_idx: usize) -> f32 {
        self.view.item_center_y(display_idx)
    }

    pub fn scrollbar_thumb(&self) -> Option<ThumbMetrics> {
        self.view.scrollbar_thumb(self.state.len(), self.state.scroll_offset())
    }

    pub fn scrollbar_rect(&self) -> Rect {
        self.view.scrollbar.rect
    }

    /// Returns `(track_rect, thumb_rect)` when the scrollbar is visible, else `None`.
    pub fn scrollbar_rects(&self) -> Option<(Rect, Rect)> {
        let thumb = self.view.scrollbar_thumb(self.state.len(), self.state.scroll_offset())?;
        Some((self.view.scrollbar.rect, thumb.rect))
    }

    pub fn list_rect(&self) -> Rect {
        self.view.list_rect
    }

    // ── Event handling ───────────────────────────────────────────────────────

    /// Handle a pointer-down at (x, y). Identifies scrollbar vs item hit.
    pub fn on_pointer_down(&mut self, x: f32, y: f32) -> ListPointerResult {
        let total = self.state.len();
        let offset = self.state.scroll_offset();

        // Scrollbar hit?
        if self.view.scrollbar.hit_test(x, y) {
            match self.view.scrollbar_click(x, y, total, offset) {
                ScrollbarAction::StartDrag { drag_offset } => {
                    self.scrollbar_drag_offset = Some(drag_offset);
                    return ListPointerResult::ScrollbarDragStart { drag_offset };
                }
                ScrollbarAction::JumpTo { ratio } => {
                    let target = self.view.scroll_offset_from_ratio(ratio, total);
                    self.state.scroll_to(target);
                    return ListPointerResult::Scrolled;
                }
                ScrollbarAction::None => {}
            }
        }

        // List item hit?
        if let Some(display_idx) = self.view.hit_test_item(x, y, total) {
            let abs_idx = offset + display_idx;
            let was_selected = self.state.selected_index() == abs_idx;
            self.state.select_index(abs_idx);
            return if was_selected {
                ListPointerResult::Confirmed(abs_idx)
            } else {
                ListPointerResult::Selected
            };
        }

        ListPointerResult::None
    }

    /// Continue a scrollbar drag started by `on_pointer_down`.
    /// Returns `true` if the scroll position changed.
    pub fn continue_scrollbar_drag(&mut self, y: f32) -> bool {
        let Some(drag_offset) = self.scrollbar_drag_offset else {
            return false;
        };
        let total = self.state.len();
        let offset = self.state.scroll_offset();
        if let Some(ratio) = self.view.scrollbar_drag_ratio(y, total, offset, drag_offset) {
            let target = self.view.scroll_offset_from_ratio(ratio, total);
            self.state.scroll_to(target);
            return true;
        }
        false
    }

    /// Release the scrollbar drag.
    pub fn end_drag(&mut self) {
        self.scrollbar_drag_offset = None;
    }

    /// Returns true if a scrollbar drag is in progress.
    pub fn is_scrollbar_dragging(&self) -> bool {
        self.scrollbar_drag_offset.is_some()
    }

    /// Update hover selection when the pointer moves over the list.
    /// Returns `true` if the selected item changed (redraw needed).
    pub fn on_hover(&mut self, x: f32, y: f32) -> bool {
        let total = self.state.len();
        let offset = self.state.scroll_offset();
        if let Some(display_idx) = self.view.hit_test_item(x, y, total) {
            let target = offset + display_idx;
            if self.state.selected_index() != target {
                self.state.select_index(target);
                return true;
            }
        }
        false
    }

    /// Scroll the list. Positive = down, negative = up.
    /// Returns `true` if the position changed.
    pub fn scroll_by(&mut self, lines: isize) -> bool {
        if lines > 0 {
            (0..lines as usize).fold(false, |changed, _| self.state.select_down() || changed)
        } else {
            (0..(-lines) as usize).fold(false, |changed, _| self.state.select_up() || changed)
        }
    }
}
