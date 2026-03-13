#![allow(unused_imports, dead_code)]
//! Layout combinators — Column, Row for composing UI slots.
//!
//! Also defines the `Widget` trait and `EventResponse` so layout combinators
//! can automatically dispatch pointer events to their children.

pub mod column;
pub mod node;
pub mod row;

pub use column::Column;
pub use node::Node;
pub use row::Row;

use crate::layout::node::{BoxStyle, Color};
use crate::theme::Theme;
use crate::ui::{CursorShape, Rect};

// ── Component trait ───────────────────────────────────────────────────────────

/// The core abstraction. Implement this and you get:
/// - Automatic rendering (framework walks `render()` output)
/// - Automatic event dispatch (framework hits the right child)
/// - Free headless tests (assert on `render()` output — no GPU)
///
/// Analogous to a React function component: declare structure in `render()`,
/// handle domain events in `on_event()`, emit typed messages upward.
pub trait Component {
    /// The domain event this component can emit to its parent.
    type Event;
    /// Declare the current UI structure as a pure data tree.
    /// The framework walks this to draw and to dispatch pointer events.
    fn render(&self) -> Node;
    /// Handle a framework-dispatched input event.
    /// Returns `Some(event)` to bubble a domain message to the parent.
    fn on_event(&mut self, event: FrameworkEvent) -> Option<Self::Event>;
}

/// Low-level input events the framework delivers to components.
#[derive(Debug, Clone)]
pub enum FrameworkEvent {
    /// A printable character was typed.
    Char(char),
    /// Backspace key.
    Backspace,
    /// Delete key.
    Delete,
    /// Arrow up.
    ArrowUp,
    /// Arrow down.
    ArrowDown,
    /// Arrow left.
    ArrowLeft,
    /// Arrow right.
    ArrowRight,
    /// Any other key press.
    Key(crate::app::Key),
    /// Pointer pressed at (x, y).
    PointerDown { x: f32, y: f32, char_width: f32 },
    /// Pointer moved to (x, y).
    PointerMove { x: f32, y: f32 },
    /// Pointer dragged to x (for text selection).
    PointerDrag { x: f32, char_width: f32 },
    /// Scroll by `lines` lines.
    Scroll { lines: isize },
}

// ── Overlay trait ─────────────────────────────────────────────────────────────

/// Result returned by every `Overlay` event method.
/// The caller (logic layer) matches this — it never knows which overlay type it is.
#[derive(Debug)]
pub enum OverlayResult {
    /// Nothing happened.
    Nothing,
    /// UI changed — redraw needed.
    Redraw,
    /// Overlay is done — close it.
    Close,
    /// Overlay completed with an action — open this file path.
    OpenPath(std::path::PathBuf),
}

/// Type-erased overlay component.
/// Implement this for any full-screen modal (picker, dialog, etc.).
/// The logic layer holds `Option<Box<dyn Overlay>>` and routes generically.
pub trait Overlay {
    /// Declare the current UI as a Node tree (called once per frame).
    fn render(&self, theme: &Theme) -> Node;
    /// Handle a typed input event. Returns what the caller should do.
    fn on_event(&mut self, event: FrameworkEvent) -> OverlayResult;
    /// Continue a pointer drag (e.g. scrollbar). Returns true if redraw needed.
    fn on_drag(&mut self, x: f32, y: f32) -> bool;
    /// End any in-progress drag.
    fn end_drag(&mut self);
    /// True if the overlay's interactive area contains (x, y).
    fn contains(&self, x: f32, y: f32) -> bool;
    /// Cursor shape the overlay wants at (x, y).
    fn cursor_shape_at(&self, x: f32, y: f32) -> CursorShape;
}

// ── Overlay helpers ──────────────────────────────────────────────────────────


/// The backdrop + panel box for a full-screen overlay.
/// Returns a `Node::layer` containing backdrop, panel chrome, and `children`.
/// `item_count` drives the panel height; pass `0` for a fixed-height dialog.
pub fn overlay_panel(window: Rect, scale: f32, item_count: usize, theme: &Theme, children: Vec<Node>) -> Node {
    let rect = centered_overlay_rect(window, scale, item_count);
    let (br, bg, bb, ba) = theme.overlay_backdrop;
    let (pr, pg, pb, pa) = theme.overlay_bg;
    let (er, eg, eb, ea) = theme.overlay_border;
    Node::layer(vec![
        Node::Box {
            rect: window,
            style: BoxStyle::filled(Color::rgba(br, bg, bb, ba)),
        },
        Node::Box {
            rect,
            style: BoxStyle::filled(Color::rgba(pr, pg, pb, pa))
                .with_border(Color::rgba(er, eg, eb, ea), theme.overlay_border_width)
                .with_radius(theme.overlay_radius * scale),
        },
        Node::layer(children),
    ])
}

/// Compute the panel rect: 60 % of window width (max 500 logical px),
/// tall enough for `item_count` rows plus one input row.
pub fn centered_overlay_rect(window: Rect, scale: f32, item_count: usize) -> Rect {
    const INPUT_H:   f32 = 36.0;
    const ITEM_H:    f32 = 32.0;
    const PADDING:   f32 = 8.0;
    const MAX_ITEMS: usize = 8;
    let visible   = item_count.min(MAX_ITEMS);
    let overlay_w = (window.width * 0.6).min(500.0 * scale);
    let overlay_h = (INPUT_H + visible as f32 * ITEM_H + 2.0 * PADDING) * scale;
    let (_, below_top) = window.cut_top(60.0 * scale);
    below_top.centered_in(overlay_w, overlay_h)
}

/// Direction preference for a floating overlay anchored to a rect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FloatDir {
    /// Prefer opening below the anchor, fall back to above if no room.
    Down,
    /// Prefer opening above the anchor, fall back to below if no room.
    Up,
}

/// Compute a panel rect anchored to `anchor`, preferring `dir`.
/// Falls back to the opposite direction if the panel would clip the window.
/// Same width/height heuristic as `centered_overlay_rect`.
pub fn floating_rect(anchor: Rect, window: Rect, scale: f32, item_count: usize, dir: FloatDir) -> Rect {
    const INPUT_H:   f32 = 36.0;
    const ITEM_H:    f32 = 32.0;
    const PADDING:   f32 = 8.0;
    const MAX_ITEMS: usize = 8;
    let visible   = item_count.min(MAX_ITEMS);
    let panel_w   = (window.width * 0.5).min(400.0 * scale);
    let panel_h   = (INPUT_H + visible as f32 * ITEM_H + 2.0 * PADDING) * scale;
    let gap       = 4.0 * scale;

    let x = (anchor.x).min(window.x + window.width - panel_w).max(window.x);

    let y_below = anchor.y + anchor.height + gap;
    let y_above = anchor.y - panel_h - gap;

    let y = match dir {
        FloatDir::Down => {
            if y_below + panel_h <= window.y + window.height { y_below }
            else if y_above >= window.y { y_above }
            else { y_below }
        }
        FloatDir::Up => {
            if y_above >= window.y { y_above }
            else if y_below + panel_h <= window.y + window.height { y_below }
            else { y_above }
        }
    };
    Rect { x, y, width: panel_w, height: panel_h }
}

// ── InlineWidget trait ────────────────────────────────────────────────────────

/// Result returned by `InlineWidget::on_event`.
#[derive(Debug)]
pub enum InlineResult {
    /// Event not consumed — do nothing.
    Nothing,
    /// State changed — redraw needed.
    Redraw,
    /// User confirmed — carries the committed string value.
    Commit(String),
    /// User cancelled — discard changes.
    Cancel,
}

/// Self-contained inline editor (e.g. tab rename, tag rename).
/// `AppLogic` holds `Option<Box<dyn InlineWidget>>` and routes generically.
pub trait InlineWidget {
    fn on_event(&mut self, event: FrameworkEvent) -> InlineResult;
    /// For downcasting — implement as `fn as_any(&self) -> &dyn std::any::Any { self }`.
    fn as_any(&self) -> &dyn std::any::Any;
}

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
