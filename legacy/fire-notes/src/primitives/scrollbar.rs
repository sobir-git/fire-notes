#![allow(dead_code)]
//! Scrollbar primitive — state + geometry + drag, one file.
//!
//! Replaces `ui::ScrollbarWidget` + `fw::Scrollbar`.

use crate::config::layout as cfg;
use crate::ui::Rect;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct ThumbMetrics {
    pub rect: Rect,
}

#[derive(Debug, Clone, Copy)]
pub enum ScrollbarAction {
    None,
    StartDrag { drag_offset: f32 },
    JumpTo    { ratio: f32 },
}

// ── Primitive ─────────────────────────────────────────────────────────────────

/// Scrollbar that owns its rect and drag state.
#[derive(Debug, Clone)]
pub struct Scrollbar {
    pub rect:   Rect,
    pub scale:  f32,
    drag_offset: Option<f32>,
}

impl Scrollbar {
    pub fn new(rect: Rect, scale: f32) -> Self {
        Self { rect, scale, drag_offset: None }
    }

    pub fn relayout(&mut self, rect: Rect, scale: f32) {
        self.rect  = rect;
        self.scale = scale;
    }

    // ── Geometry ──────────────────────────────────────────────────────────────

    pub fn hit_test(&self, x: f32, y: f32) -> bool {
        self.rect.contains(x, y)
    }

    pub fn is_scrollable(&self, total: usize, visible: usize) -> bool {
        total > visible && visible > 0
    }

    /// Compute thumb rect, or `None` when not scrollable.
    pub fn thumb(&self, total: usize, visible: usize, offset: usize) -> Option<ThumbMetrics> {
        if !self.is_scrollable(total, visible) { return None; }
        let track_h   = self.rect.height;
        let view_ratio = visible as f32 / total as f32;
        let min_thumb  = cfg::MIN_SCROLLBAR_THUMB * self.scale;
        let thumb_h    = (track_h * view_ratio).max(min_thumb);
        let max_scroll = total.saturating_sub(1);
        let scroll_ratio = if max_scroll > 0 { offset as f32 / max_scroll as f32 } else { 0.0 };
        let track_space  = (track_h - thumb_h).max(0.0);
        let thumb_y      = self.rect.y + track_space * scroll_ratio.clamp(0.0, 1.0);
        Some(ThumbMetrics {
            rect: Rect { x: self.rect.x, y: thumb_y, width: self.rect.width, height: thumb_h },
        })
    }

    // ── Click ─────────────────────────────────────────────────────────────────

    pub fn on_click(&mut self, x: f32, y: f32, total: usize, visible: usize, offset: usize) -> ScrollbarAction {
        if !self.is_scrollable(total, visible) { return ScrollbarAction::None; }
        if let Some(m) = self.thumb(total, visible, offset) {
            if m.rect.contains(x, y) {
                let drag_offset = y - m.rect.y;
                self.drag_offset = Some(drag_offset);
                return ScrollbarAction::StartDrag { drag_offset };
            }
        }
        ScrollbarAction::JumpTo { ratio: self.jump_ratio(y) }
    }

    // ── Drag ──────────────────────────────────────────────────────────────────

    pub fn is_dragging(&self) -> bool { self.drag_offset.is_some() }

    pub fn start_drag(&mut self, drag_offset: f32) {
        self.drag_offset = Some(drag_offset);
    }

    pub fn continue_drag(&self, y: f32, total: usize, visible: usize, offset: usize) -> Option<f32> {
        let drag_offset = self.drag_offset?;
        let m = self.thumb(total, visible, offset)?;
        let track_space = (self.rect.height - m.rect.height).max(0.0);
        let relative_y = (y - self.rect.y - drag_offset).clamp(0.0, track_space);
        Some(if track_space > 0.0 { relative_y / track_space } else { 0.0 })
    }

    pub fn end_drag(&mut self) { self.drag_offset = None; }

    pub fn drag_offset_value(&self) -> Option<f32> { self.drag_offset }

    // ── List scrollbar variant (max = total - visible, not total - 1) ─────────

    /// Thumb for a list scrollbar (correct max_scroll formula for lists).
    pub fn list_thumb(&self, total: usize, visible: usize, offset: usize) -> Option<ThumbMetrics> {
        if !self.is_scrollable(total, visible) { return None; }
        let track_h    = self.rect.height;
        let view_ratio = visible as f32 / total as f32;
        let min_thumb  = cfg::MIN_SCROLLBAR_THUMB * self.scale;
        let thumb_h    = (track_h * view_ratio).max(min_thumb);
        let max_scroll = total.saturating_sub(visible);
        let scroll_ratio = if max_scroll > 0 {
            (offset as f32 / max_scroll as f32).clamp(0.0, 1.0)
        } else { 0.0 };
        let track_space = (track_h - thumb_h).max(0.0);
        let thumb_y = self.rect.y + track_space * scroll_ratio;
        Some(ThumbMetrics {
            rect: Rect { x: self.rect.x, y: thumb_y, width: self.rect.width, height: thumb_h },
        })
    }

    pub fn list_drag_ratio(&self, y: f32, total: usize, visible: usize, offset: usize, drag_offset: f32) -> Option<f32> {
        if !self.is_scrollable(total, visible) { return None; }
        let m = self.list_thumb(total, visible, offset)?;
        let track_space = (self.rect.height - m.rect.height).max(0.0);
        let relative_y = (y - self.rect.y - drag_offset).clamp(0.0, track_space);
        Some(if track_space > 0.0 { relative_y / track_space } else { 0.0 })
    }

    // ── Private ───────────────────────────────────────────────────────────────

    fn jump_ratio(&self, y: f32) -> f32 {
        let track_h = self.rect.height.max(1.0);
        ((y - self.rect.y).clamp(0.0, track_h) / track_h).clamp(0.0, 1.0)
    }
}
