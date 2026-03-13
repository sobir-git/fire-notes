#![allow(unused_imports)]
//! Layout combinators — Column, Row for composing UI slots.
//!
//! Also defines the `Widget` trait and `EventResponse` so layout combinators
//! can automatically dispatch pointer events to their children.

pub mod column;
pub mod row;

pub use column::Column;
pub use row::Row;

use crate::ui::{CursorShape, Rect};

// ── Widget trait ──────────────────────────────────────────────────────────────

/// Minimal contract every interactive widget must implement.
/// `Ev` is the widget's domain event type (e.g. `ListEvent`, `InputEvent`).
pub trait Widget {
    type Event;
    /// Handle a pointer-down at (x, y). Returns the domain event produced.
    fn on_pointer_down(&mut self, x: f32, y: f32) -> Self::Event;
    /// Update hover state. Returns `true` if a redraw is needed.
    fn on_hover(&mut self, x: f32, y: f32) -> bool;
    /// Cursor shape the widget wants when the pointer is at (x, y).
    fn cursor_shape_at(&self, x: f32, y: f32) -> CursorShape;
}

// ── EventResponse ─────────────────────────────────────────────────────────────

/// Universal event return type for widget interactions.
#[derive(Debug, Clone, PartialEq)]
pub enum EventResponse<Msg = ()> {
    /// Event not handled — propagate to parent.
    Pass,
    /// Consumed internally; no redraw needed.
    Handled,
    /// Consumed internally; redraw needed.
    Redraw,
    /// Produced a business-level message for the parent (implies redraw).
    Emit(Msg),
}
