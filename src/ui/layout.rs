//! `Layout` trait — the contract every widget must fulfil.
//!
//! Implementing `Layout` means: "given a rect and a scale factor, I can
//! produce myself."  This makes the widget hierarchy formal: `UiTree` just
//! calls `Widget::layout(rect, scale)` top-down; no layout code lives
//! anywhere else.

use super::types::Rect;

/// A widget that can lay itself out from a parent-supplied rect.
pub trait Layout: Sized {
    /// Produce this widget from the space the parent allocates.
    ///
    /// `rect`  — screen-space bounding rect handed down by the parent.
    /// `scale` — DPI scale factor (physical pixels per logical pixel).
    fn layout(rect: Rect, scale: f32) -> Self;
}
