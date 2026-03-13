#![allow(dead_code)]
//! Row layout — horizontal sequence of slots with auto hit-dispatch.
//!
//! Same weight convention as `Column`: negative = fixed pixels × scale,
//! positive = fractional share of remaining space.

use crate::ui::Rect;

// ── Row ───────────────────────────────────────────────────────────────────────

/// A horizontal sequence of rects, computed once from a parent rect.
#[derive(Debug, Clone)]
pub struct Row {
    pub slots: Vec<Rect>,
}

impl Row {
    /// Compute slot rects by splitting `rect` horizontally according to `weights`.
    pub fn new(rect: Rect, scale: f32, weights: &[f32]) -> Self {
        let scaled: Vec<f32> = weights.iter().map(|&w| if w < 0.0 { w * scale } else { w }).collect();
        Self { slots: rect.split_h(&scaled) }
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
