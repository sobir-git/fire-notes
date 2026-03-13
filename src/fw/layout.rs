#![allow(dead_code)]
//! Named layout containers — thin wrappers over `Rect::split_v` / `Rect::split_h`.
//!
//! Slot convention (same as `Rect::split_v` / `Rect::split_h`):
//!   - negative value → fixed `|v| * scale` pixels
//!   - positive value → flex share of remaining space

use crate::ui::Rect;

/// Vertical stack — divides a rect into horizontal bands top-to-bottom.
pub struct VStack(pub Vec<Rect>);

impl VStack {
    /// Build from a rect and a slice of slot weights.
    pub fn new(rect: Rect, scale: f32, slots: &[f32]) -> Self {
        let scaled: Vec<f32> = slots
            .iter()
            .map(|&w| if w < 0.0 { w * scale } else { w })
            .collect();
        VStack(rect.split_v(&scaled))
    }

    pub fn slot(&self, i: usize) -> Rect {
        self.0[i]
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

/// Horizontal stack — divides a rect into vertical columns left-to-right.
pub struct HStack(pub Vec<Rect>);

impl HStack {
    /// Build from a rect and a slice of slot weights.
    pub fn new(rect: Rect, scale: f32, slots: &[f32]) -> Self {
        let scaled: Vec<f32> = slots
            .iter()
            .map(|&w| if w < 0.0 { w * scale } else { w })
            .collect();
        HStack(rect.split_h(&scaled))
    }

    pub fn slot(&self, i: usize) -> Rect {
        self.0[i]
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}
