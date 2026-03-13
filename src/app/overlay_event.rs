//! `OverlayEvent` — typed events for overlay and inline components.
//!
//! Replaces the old `FrameworkEvent` enum that mixed keyboard, pointer, and
//! navigation events in one type. Overlay components now receive strongly-typed
//! events and return `OverlayResult` / `InlineResult` directly.

use crate::primitives::text_input::TextInputEvent;

/// Events that the `AppLogic` dispatches to the active overlay or inline widget.
#[derive(Debug)]
pub enum OverlayEvent<'a> {
    /// A text-editing key that should be routed to the component's `TextInput`.
    Text(TextInputEvent<'a>),
    /// Paste text into the component's `TextInput`.
    Paste(String),
    /// Arrow up — move list selection up.
    Up,
    /// Arrow down — move list selection down.
    Down,
    /// Confirm / Enter.
    Confirm,
    /// Cancel / Escape.
    Cancel,
    /// Pointer pressed at logical (x, y).
    PointerDown { x: f32, y: f32 },
    /// Double-click at logical (x, y) — select word in search field.
    PointerDoubleClick { x: f32, y: f32 },
    /// Triple-click at logical (x, y) — select all in search field.
    PointerTripleClick { x: f32, y: f32 },
    /// Pointer dragged to logical x.
    PointerDrag { x: f32 },
    /// Pointer moved to logical (x, y) — for hover.
    PointerMove { x: f32, y: f32 },
    /// Scroll by `lines`.
    Scroll { lines: isize },
}

/// What the overlay wants the caller to do.
#[derive(Debug)]
pub enum OverlayResult {
    /// Nothing changed.
    Nothing,
    /// Redraw needed.
    Redraw,
    /// Overlay is done — close it.
    Close,
    /// Overlay produced a file path to open.
    OpenPath(std::path::PathBuf),
}

/// What an inline widget wants the caller to do.
#[derive(Debug)]
pub enum InlineResult {
    /// Nothing changed.
    Nothing,
    /// Redraw needed.
    Redraw,
    /// User committed — carry the final string value.
    Commit(String),
    /// User cancelled.
    Cancel,
}
