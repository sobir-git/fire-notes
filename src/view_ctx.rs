#![allow(dead_code)]
//! `ViewCtx` — context passed to `App::view()` and `TextInput::view_with_ctx()`.
//!
//! Lives at the crate root so both `primitives` and `runtime` can import it
//! without a circular dependency.

use std::collections::{HashMap, HashSet};
use crate::ui::Rect;

/// Context passed to [`crate::runtime::App::view`] on every frame.
///
/// Passed as `&mut` so widgets can register their layout rects during rendering
/// via [`ViewCtx::register_rect`]. The runtime then uses those rects to call
/// `TextInput::handle_key` with the correct geometry.
#[derive(Debug, Clone)]
pub struct ViewCtx {
    /// Physical pixels per logical pixel (HiDPI scale factor).
    /// Framework-internal — app code should never need to read this.
    pub(crate) scale: f32,
    /// Width of one monospace character in **logical pixels**.
    pub char_width: f32,
    /// Whether the cursor blink is currently "on".
    pub cursor_on: bool,
    /// Set of widget IDs that currently have focus.
    pub focused: HashSet<u64>,
    /// Full window rect in **logical pixels** (origin always 0,0).
    pub window: Rect,
    /// Layout cache: widget ID → last rendered rect in **logical pixels**.
    /// Populated by [`ViewCtx::register_rect`] during `App::view`.
    pub layout: HashMap<u64, Rect>,
}

impl ViewCtx {
    /// Returns `true` when the widget with the given ID is focused.
    pub fn is_focused(&self, id: u64) -> bool {
        self.focused.contains(&id)
    }

    /// Register the logical rect used to render widget `id` this frame.
    /// Called automatically by [`crate::primitives::TextInput::view_with_ctx`].
    pub fn register_rect(&mut self, id: u64, rect: Rect) {
        self.layout.insert(id, rect);
    }

    /// Scale a logical rect to physical pixels. Used internally by primitives.
    pub(crate) fn to_physical(&self, r: Rect) -> Rect {
        let s = self.scale;
        Rect { x: r.x * s, y: r.y * s, width: r.width * s, height: r.height * s }
    }

    /// Look up the last registered rect for widget `id`.
    pub fn rect_for(&self, id: u64) -> Option<Rect> {
        self.layout.get(&id).copied()
    }

    // ── Node-building helpers (logical → physical) ────────────────────────────

    /// Background box filling the full window.
    pub fn background(&self, color: crate::layout::node::Color) -> crate::layout::Node {
        use crate::layout::node::BoxStyle;
        use crate::layout::Node;
        Node::Box { rect: self.to_physical(self.window), style: BoxStyle::filled(color) }
    }

    /// A text node with logical `font_size`, `baseline_y`, and `text_x`.
    pub fn text(
        &self,
        content: impl Into<String>,
        font_size: f32,
        color: crate::layout::node::Color,
        baseline_y: f32,
        text_x: f32,
    ) -> crate::layout::Node {
        use crate::layout::node::TextStyle;
        use crate::layout::Node;
        let s = self.scale;
        Node::Text {
            text:  content.into(),
            style: TextStyle {
                font_size:  font_size * s,
                color,
                baseline_y: baseline_y * s,
                text_x:     text_x * s,
                scroll_x:   0.0,
            },
            clip: None,
        }
    }
}
