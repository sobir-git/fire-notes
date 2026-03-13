//! Layout engine — walks an `Element` tree top-down and assigns `Rect`s.
//!
//! Rules:
//! - `Backdrop` fills the entire window rect passed in.
//! - `Column` splits its rect vertically among children using their `Size`.
//! - `Row` splits its rect horizontally among children using their `Size`.
//! - `TextInput` and `List` are leaves; their rect is whatever the parent assigned.
//!
//! After this pass every `Element` node has a non-zero `rect`.

use crate::framework::element::{Element, ElementKind, Size};
use crate::ui::Rect;

/// Assign rects to every node in the element tree.
/// `root_rect` is the full available area (typically the overlay panel rect).
/// `scale` is the physical pixels per logical pixel (for fixed sizes).
pub fn layout_tree<Msg>(el: &mut Element<Msg>, root_rect: Rect, scale: f32) {
    el.rect = root_rect;
    layout_children(el, root_rect, scale);
}

fn layout_children<Msg>(el: &mut Element<Msg>, parent_rect: Rect, scale: f32) {
    if el.children.is_empty() { return; }

    match &el.kind {
        ElementKind::Column | ElementKind::Backdrop { .. } => {
            layout_axis(el, parent_rect, scale, Axis::Vertical);
        }
        ElementKind::Row => {
            layout_axis(el, parent_rect, scale, Axis::Horizontal);
        }
        _ => {
            // Leaf nodes (TextInput, List) get their full rect and no children.
        }
    }
}

enum Axis { Vertical, Horizontal }

fn layout_axis<Msg>(el: &mut Element<Msg>, available: Rect, scale: f32, axis: Axis) {
    let n = el.children.len();
    if n == 0 { return; }

    // ── Compute fixed totals and fill weight sum ────────────────────────────
    let total_space = match axis {
        Axis::Vertical   => available.height,
        Axis::Horizontal => available.width,
    };

    let mut fixed_total = 0.0f32;
    let mut fill_total  = 0.0f32;

    for child in &el.children {
        match child.size {
            Size::Fixed(px)  => fixed_total += px * scale,
            Size::Fill(w)    => fill_total  += w,
        }
    }

    let fill_space = (total_space - fixed_total).max(0.0);
    let per_fill   = if fill_total > 0.0 { fill_space / fill_total } else { 0.0 };

    // ── Assign rects ────────────────────────────────────────────────────────
    let mut cursor = match axis {
        Axis::Vertical   => available.y,
        Axis::Horizontal => available.x,
    };

    for child in &mut el.children {
        let size = match child.size {
            Size::Fixed(px) => px * scale,
            Size::Fill(w)   => w * per_fill,
        };

        child.rect = match axis {
            Axis::Vertical => Rect {
                x:      available.x,
                y:      cursor,
                width:  available.width,
                height: size,
            },
            Axis::Horizontal => Rect {
                x:      cursor,
                y:      available.y,
                width:  size,
                height: available.height,
            },
        };
        cursor += size;

        // Recurse into non-leaf children.
        let rect = child.rect;
        layout_children(child, rect, scale);
    }
}
