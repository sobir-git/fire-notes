//! TextEditor component — content area snapshot baking, one file.
//!
//! The geometry lives in `ui::ContentArea` (already well-structured).
//! This file owns the snapshot functions that bake `ContentArea` into pure
//! `ContentFrameData` + `ScrollbarFrameData` for the renderer.

use crate::render_frame::{ContentFrameData, FrameRect, ScrollbarFrameData};
use crate::ui::ContentArea;

// ── Snapshot ──────────────────────────────────────────────────────────────────

/// Bake a `ContentArea` into pure frame data for the renderer.
pub fn snapshot(ca: &ContentArea) -> ContentFrameData {
    ContentFrameData {
        rect:           to_frame(ca.rect),
        text_rect:      to_frame(ca.text.rect),
        line_height:    ca.text.line_height,
        text_padding:   ca.text.text_padding,
        doc_top_margin: ca.text.doc_top_margin,
        scrollbar_rect: to_frame(ca.scrollbar.rect),
    }
}

/// Bake the scrollbar pixel geometry from a `ContentArea`.
/// Returns `None` when the content fits without scrolling.
pub fn scrollbar_snapshot(
    ca: &ContentArea,
    total_lines: usize,
    visible_count: usize,
    scroll_offset: usize,
) -> Option<ScrollbarFrameData> {
    let m = ca.scrollbar.thumb(total_lines, visible_count, scroll_offset)?;
    Some(ScrollbarFrameData {
        thumb_rect: to_frame(m.rect),
        hovered:    ca.scrollbar_hovered,
        dragging:   ca.scrollbar.is_dragging(),
    })
}

fn to_frame(r: crate::ui::Rect) -> FrameRect {
    FrameRect { x: r.x, y: r.y, width: r.width, height: r.height }
}
