#![allow(dead_code)]
//! Button primitive — state + geometry + events, one file.

use crate::ui::Rect;

// ── Primitive ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Button {
    pub label:   String,
    pub rect:    Rect,
    pub hovered: bool,
    pub pressed: bool,
    pub scale:   f32,
}

impl Button {
    pub fn new(rect: Rect, scale: f32, label: impl Into<String>) -> Self {
        Self { label: label.into(), rect, hovered: false, pressed: false, scale }
    }

    pub fn relayout(&mut self, rect: Rect, scale: f32) {
        self.rect  = rect;
        self.scale = scale;
    }

    // ── Events ────────────────────────────────────────────────────────────────

    /// Returns `true` if the click landed on this button.
    pub fn on_pointer_down(&mut self, x: f32, y: f32) -> bool {
        if self.rect.contains(x, y) { self.pressed = true; true } else { false }
    }

    pub fn on_pointer_up(&mut self, x: f32, y: f32) -> bool {
        let was_pressed = self.pressed;
        self.pressed = false;
        was_pressed && self.rect.contains(x, y)
    }

    pub fn on_hover(&mut self, x: f32, y: f32) -> bool {
        let was = self.hovered;
        self.hovered = self.rect.contains(x, y);
        self.hovered != was
    }
}
