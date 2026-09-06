//! Layout combinators — Column, Row for composing UI slots.
//!
//! Also defines the `Widget` trait and `EventResponse` so layout combinators
//! can automatically dispatch pointer events to their children.

pub mod column;
pub mod node;
pub mod row;

#[allow(unused_imports)]
pub use column::Column;
pub use node::Node;

use crate::layout::node::{BoxStyle, Color};
use crate::theme::Theme;
use crate::ui::Rect;

// ── Overlay helpers ──────────────────────────────────────────────────────────


#[allow(dead_code)]
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
    };
    Rect { x, y, width: panel_w, height: panel_h }
}
