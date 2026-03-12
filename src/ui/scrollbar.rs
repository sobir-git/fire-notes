//! Scrollbar widget — owns its rect and operates in screen space.
//!
//! The rect is handed down by `ContentArea` via layout primitives, so
//! `ScrollbarWidget` never needs to know about window dimensions or the
//! tab bar.  It just owns its own space and does its own math within it.

use crate::config::layout;
use super::layout::Layout;
use super::types::Rect;

/// Thumb geometry in screen coordinates.
#[derive(Debug, Clone, Copy)]
pub struct ThumbMetrics {
    /// Screen rect of the thumb.
    pub rect: Rect,
}

#[derive(Debug, Clone, Copy)]
pub enum ScrollbarAction {
    None,
    /// User clicked on the thumb. `drag_offset` is screen Y within the thumb.
    StartDrag { drag_offset: f32 },
    /// User clicked on the track outside the thumb.
    JumpTo { ratio: f32 },
}

/// Scrollbar widget.  Receives its rect from the parent layout.
/// All methods operate in screen coordinates — no translation needed.
#[derive(Debug, Clone, Copy)]
pub struct ScrollbarWidget {
    pub rect: Rect,
    scale: f32,
}

impl Layout for ScrollbarWidget {
    fn layout(rect: Rect, scale: f32) -> Self {
        Self { rect, scale }
    }
}

impl ScrollbarWidget {
    /// Convenience alias for `Layout::layout` — construct from a pre-computed rect.
    #[allow(dead_code)]
    pub fn new(rect: Rect, scale: f32) -> Self {
        Self { rect, scale }
    }

    pub fn hit_test(&self, x: f32, y: f32) -> bool {
        self.rect.contains(x, y)
    }

    pub fn is_scrollable(&self, total_lines: usize, visible_lines: usize) -> bool {
        total_lines > visible_lines && visible_lines > 0
    }

    /// Compute the thumb rect in screen coordinates, or `None` if not scrollable.
    pub fn thumb(&self, total_lines: usize, visible_lines: usize, scroll_offset: usize) -> Option<ThumbMetrics> {
        if !self.is_scrollable(total_lines, visible_lines) {
            return None;
        }

        let track_h = self.rect.height;
        let view_ratio = visible_lines as f32 / total_lines as f32;
        let min_thumb = layout::MIN_SCROLLBAR_THUMB * self.scale;
        let thumb_h = (track_h * view_ratio).max(min_thumb);

        let max_scroll = total_lines.saturating_sub(1);
        let scroll_ratio = if max_scroll > 0 {
            scroll_offset as f32 / max_scroll as f32
        } else {
            0.0
        };

        let track_space = (track_h - thumb_h).max(0.0);
        let thumb_y = self.rect.y + track_space * scroll_ratio.clamp(0.0, 1.0);

        Some(ThumbMetrics {
            rect: Rect { x: self.rect.x, y: thumb_y, width: self.rect.width, height: thumb_h },
        })
    }

    /// Handle a click at screen (x, y).
    pub fn on_click(&self, x: f32, y: f32, total_lines: usize, visible_lines: usize, scroll_offset: usize) -> ScrollbarAction {
        if !self.is_scrollable(total_lines, visible_lines) {
            return ScrollbarAction::None;
        }
        if let Some(m) = self.thumb(total_lines, visible_lines, scroll_offset) {
            if m.rect.contains(x, y) {
                return ScrollbarAction::StartDrag { drag_offset: y - m.rect.y };
            }
        }
        ScrollbarAction::JumpTo { ratio: self.jump_ratio(y) }
    }

    /// Compute the scroll ratio (0..1) while dragging.
    pub fn drag_ratio(&self, y: f32, total_lines: usize, visible_lines: usize, drag_offset: f32, scroll_offset: usize) -> Option<f32> {
        if !self.is_scrollable(total_lines, visible_lines) {
            return None;
        }
        let m = self.thumb(total_lines, visible_lines, scroll_offset)?;
        let track_space = (self.rect.height - m.rect.height).max(0.0);
        let relative_y = (y - self.rect.y - drag_offset).clamp(0.0, track_space);
        Some(if track_space > 0.0 { relative_y / track_space } else { 0.0 })
    }

    fn jump_ratio(&self, y: f32) -> f32 {
        let track_h = self.rect.height.max(1.0);
        ((y - self.rect.y).clamp(0.0, track_h) / track_h).clamp(0.0, 1.0)
    }
}
