//! Scrollbar widget and hit-testing

use crate::config::layout;
use super::types::Rect;

#[derive(Debug, Clone, Copy)]
pub enum ScrollbarAction {
    None,
    StartDrag { drag_offset: f32 },
    JumpTo { ratio: f32 },
}

#[derive(Debug, Clone, Copy)]
pub struct ScrollbarMetrics {
    pub track: Rect,
    pub thumb: Rect,
}

#[derive(Debug, Clone)]
pub struct ScrollbarWidget {
    pub rect: Rect,
    scale: f32,
}

impl ScrollbarWidget {
    pub fn new(width: f32, height: f32, scale: f32) -> Self {
        let tab_height = layout::TAB_HEIGHT * scale;
        let padding = layout::PADDING * scale;
        let scrollbar_width = layout::SCROLLBAR_WIDTH * scale;
        Self {
            rect: Rect {
                x: width - scrollbar_width,
                y: tab_height,
                width: scrollbar_width,
                height: (height - tab_height - padding).max(0.0),
            },
            scale,
        }
    }

    pub fn hit_test(&self, x: f32, y: f32) -> bool {
        self.rect.contains(x, y)
    }

    pub fn is_scrollable(&self, total_lines: usize, visible_lines: usize) -> bool {
        total_lines > visible_lines && visible_lines > 0
    }

    pub fn thumb_rect(
        &self,
        total_lines: usize,
        visible_lines: usize,
        scroll_offset: usize,
    ) -> Option<Rect> {
        self.metrics(total_lines, visible_lines, scroll_offset)
            .map(|metrics| metrics.thumb)
    }

    pub fn metrics(
        &self,
        total_lines: usize,
        visible_lines: usize,
        scroll_offset: usize,
    ) -> Option<ScrollbarMetrics> {
        if !self.is_scrollable(total_lines, visible_lines) {
            return None;
        }

        let track_height = self.rect.height;
        let track = Rect {
            x: self.rect.x,
            y: self.rect.y,
            width: self.rect.width,
            height: track_height,
        };

        let view_ratio = visible_lines as f32 / total_lines as f32;
        let min_thumb = layout::MIN_SCROLLBAR_THUMB * self.scale;
        let thumb_height = (track_height * view_ratio).max(min_thumb);

        let max_scroll = total_lines.saturating_sub(visible_lines);
        let scroll_ratio = if max_scroll > 0 {
            scroll_offset as f32 / max_scroll as f32
        } else {
            0.0
        };

        let track_space = (track_height - thumb_height).max(0.0);
        let thumb_y = self.rect.y + track_space * scroll_ratio.clamp(0.0, 1.0);
        let thumb = Rect {
            x: track.x,
            y: thumb_y,
            width: track.width,
            height: thumb_height,
        };

        Some(ScrollbarMetrics { track, thumb })
    }

    pub fn on_click(
        &self,
        x: f32,
        y: f32,
        total_lines: usize,
        visible_lines: usize,
        scroll_offset: usize,
    ) -> ScrollbarAction {
        if !self.is_scrollable(total_lines, visible_lines) {
            return ScrollbarAction::None;
        }

        if let Some(thumb) = self.thumb_rect(total_lines, visible_lines, scroll_offset) {
            if thumb.contains(x, y) {
                return ScrollbarAction::StartDrag {
                    drag_offset: y - thumb.y,
                };
            }
        }

        ScrollbarAction::JumpTo {
            ratio: self.jump_ratio(y),
        }
    }

    pub fn drag_ratio(
        &self,
        y: f32,
        total_lines: usize,
        visible_lines: usize,
        drag_offset: f32,
        scroll_offset: usize,
    ) -> Option<f32> {
        if !self.is_scrollable(total_lines, visible_lines) {
            return None;
        }

        let thumb = self.thumb_rect(total_lines, visible_lines, scroll_offset)?;
        let track_space = (self.rect.height - thumb.height).max(0.0);
        let relative_y = (y - self.rect.y - drag_offset).clamp(0.0, track_space);
        Some(if track_space > 0.0 {
            relative_y / track_space
        } else {
            0.0
        })
    }

    fn jump_ratio(&self, y: f32) -> f32 {
        let track_height = self.rect.height.max(1.0);
        let relative_y = (y - self.rect.y).clamp(0.0, track_height);
        (relative_y / track_height).clamp(0.0, 1.0)
    }
}
