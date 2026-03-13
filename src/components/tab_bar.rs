//! TabBar component — snapshot baking, one file.
//!
//! The geometry and hit-testing live in `ui::TabBar` (already well-structured).
//! This file owns the snapshot method that bakes `ui::TabBar` into pure
//! `TabBarFrameData` for the renderer.

use crate::render_frame::{FrameRect, TabBarFrameData, TabRectData};
use crate::ui::TabBar;

// ── Snapshot ──────────────────────────────────────────────────────────────────

/// Bake a `TabBar` widget into pure frame data for the renderer.
pub fn snapshot(tb: &TabBar) -> TabBarFrameData {
    TabBarFrameData {
        rect:              to_frame(tb.rect),
        tabs_clip_x:       tb.tabs_clip_x,
        tabs:              tb.scroll_area.tabs.iter()
            .map(|t| TabRectData { index: t.index, rect: to_frame(t.rect) })
            .collect(),
        new_tab_rect:      to_frame(tb.new_tab_rect),
        minimize_rect:     to_frame(tb.minimize_rect),
        maximize_rect:     to_frame(tb.maximize_rect),
        close_rect:        to_frame(tb.close_rect),
        hovered_tab_index: tb.hovered_tab_index,
        hovered_plus:      tb.hovered_plus,
        hovered_minimize:  tb.hovered_minimize,
        hovered_maximize:  tb.hovered_maximize,
        hovered_close:     tb.hovered_close,
    }
}

fn to_frame(r: crate::ui::Rect) -> FrameRect {
    FrameRect { x: r.x, y: r.y, width: r.width, height: r.height }
}
