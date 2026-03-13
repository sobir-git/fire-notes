#![allow(dead_code)]
//! `fw::Scrollbar` — merged geometry + drag state widget.
//!
//! Wraps `ui::ScrollbarWidget` (pure geometry) and adds the per-session drag state
//! so callers never need to store `drag_offset` separately in `UiState`.

use crate::ui::{Layout, Rect, ScrollbarAction, ScrollbarWidget, ThumbMetrics};

/// Scrollbar widget that owns both its rect and its drag state.
///
/// Drag lifecycle:
///   `on_click()` → returns `ScrollbarAction::StartDrag` → call `start_drag(offset)`
///   `continue_drag(y)` → returns new scroll ratio
///   `end_drag()` → clears drag state
pub struct Scrollbar {
    pub widget: ScrollbarWidget,
    /// `Some(drag_offset)` while the user is dragging the thumb.
    drag_offset: Option<f32>,
}

impl Scrollbar {
    pub fn new(rect: Rect, scale: f32) -> Self {
        Self {
            widget: ScrollbarWidget::layout(rect, scale),
            drag_offset: None,
        }
    }

    pub fn relayout(&mut self, rect: Rect, scale: f32) {
        self.widget = ScrollbarWidget::layout(rect, scale);
    }

    // ── Geometry accessors ─────────────────────────────────────────────────

    pub fn rect(&self) -> Rect { self.widget.rect }

    pub fn hit_test(&self, x: f32, y: f32) -> bool {
        self.widget.hit_test(x, y)
    }

    pub fn is_scrollable(&self, total_lines: usize, visible_lines: usize) -> bool {
        self.widget.is_scrollable(total_lines, visible_lines)
    }

    pub fn thumb(
        &self,
        total_lines: usize,
        visible_lines: usize,
        scroll_offset: usize,
    ) -> Option<ThumbMetrics> {
        self.widget.thumb(total_lines, visible_lines, scroll_offset)
    }

    // ── Click ──────────────────────────────────────────────────────────────

    pub fn on_click(
        &mut self,
        x: f32,
        y: f32,
        total_lines: usize,
        visible_lines: usize,
        scroll_offset: usize,
    ) -> ScrollbarAction {
        let action = self.widget.on_click(x, y, total_lines, visible_lines, scroll_offset);
        if let ScrollbarAction::StartDrag { drag_offset } = action {
            self.drag_offset = Some(drag_offset);
        }
        action
    }

    // ── Drag state ─────────────────────────────────────────────────────────

    pub fn is_dragging(&self) -> bool { self.drag_offset.is_some() }

    /// Start a drag at `drag_offset` screen pixels within the thumb.
    pub fn start_drag(&mut self, drag_offset: f32) {
        self.drag_offset = Some(drag_offset);
    }

    /// Continue drag at screen y, returns the new scroll ratio (0..1) or `None`.
    pub fn continue_drag(
        &self,
        y: f32,
        total_lines: usize,
        visible_lines: usize,
        scroll_offset: usize,
    ) -> Option<f32> {
        let offset = self.drag_offset?;
        self.widget.drag_ratio(y, total_lines, visible_lines, offset, scroll_offset)
    }

    pub fn end_drag(&mut self) {
        self.drag_offset = None;
    }
}
