#![allow(dead_code)]
//! Label primitive — static text display, one file.

use crate::ui::Rect;

// ── Primitive ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Label {
    pub text:      String,
    pub rect:      Rect,
    pub font_size: f32,
    pub scale:     f32,
    /// Baseline Y (vertically centred in rect).
    pub baseline_y: f32,
}

impl Label {
    pub fn new(rect: Rect, scale: f32, text: impl Into<String>) -> Self {
        let font_size  = 14.0 * scale;
        let baseline_y = rect.y + rect.height / 2.0 + font_size * 0.35;
        Self { text: text.into(), rect, font_size, scale, baseline_y }
    }

    pub fn relayout(&mut self, rect: Rect, scale: f32) {
        let font_size  = 14.0 * scale;
        let baseline_y = rect.y + rect.height / 2.0 + font_size * 0.35;
        self.rect       = rect;
        self.scale      = scale;
        self.font_size  = font_size;
        self.baseline_y = baseline_y;
    }
}
