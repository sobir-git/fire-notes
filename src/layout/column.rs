#![allow(dead_code)]
//! Column layout — vertical stack of slots with auto hit-dispatch.
//!
//! Heights are specified as weights (positive = fractional share) or
//! fixed pixel sizes (negative = `-pixels * scale`). Same convention as
//! `Rect::split_v`.

use crate::ui::Rect;

// ── Column ────────────────────────────────────────────────────────────────────

/// A vertical sequence of rects, computed once from a parent rect.
///
/// Use negative weights for fixed sizes, positive for proportional:
/// ```text
/// Column::new(rect, scale, &[-36.0, 1.0, -48.0])
/// //                          ^^^^ 36 px   ^^^^ 48 px
/// //                                  ^^^ fills remaining space
/// ```
#[derive(Debug, Clone)]
pub struct Column {
    pub slots: Vec<Rect>,
}

impl Column {
    /// Compute slot rects by splitting `rect` vertically according to `weights`.
    /// Negative values are fixed pixel sizes multiplied by `scale`.
    /// Positive values are fractional shares of remaining space.
    pub fn new(rect: Rect, scale: f32, weights: &[f32]) -> Self {
        let scaled: Vec<f32> = weights.iter().map(|&w| if w < 0.0 { w * scale } else { w }).collect();
        Self { slots: rect.split_v(&scaled) }
    }

    pub fn slot(&self, index: usize) -> Rect {
        self.slots[index]
    }

    pub fn len(&self) -> usize { self.slots.len() }

    /// Find which slot contains (x, y), returns its index.
    pub fn hit_slot(&self, x: f32, y: f32) -> Option<usize> {
        self.slots.iter().position(|r| r.contains(x, y))
    }
}
