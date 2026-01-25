//! Text area layout

use crate::config::layout;
use super::types::Rect;

#[derive(Debug, Clone)]
pub struct TextArea {
    pub rect: Rect,
}

impl TextArea {
    pub fn new(width: f32, height: f32, scale: f32) -> Self {
        let tab_height = layout::TAB_HEIGHT * scale;
        let padding = layout::PADDING * scale;
        let y = tab_height + padding;
        let height = (height - y - padding).max(0.0);
        Self {
            rect: Rect {
                x: 0.0,
                y,
                width,
                height,
            },
        }
    }

    pub fn hit_test(&self, x: f32, y: f32) -> bool {
        self.rect.contains(x, y)
    }
}
