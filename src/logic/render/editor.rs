//! Builds the `Node::Editor` subtree.

use crate::layout::Node;
use crate::layout::node::{
    EditorCursor, EditorNode, EditorSelection, ScrollbarNode, TextLine,
};
use crate::layout::node::Color as NColor;
use crate::logic::AppLogic;
use crate::visual_position::VisualLine;

/// Returns `(EditorNode, selection_rects)`.
/// The selection rects are returned separately so `flames.rs` can reuse them.
pub(super) fn build(logic: &AppLogic) -> (Node, Vec<crate::ui::Rect>) {
    let ca           = &logic.ui_tree.content_area;
    let current_tab  = &logic.tabs[logic.active_tab];
    let scroll_offset = current_tab.scroll_offset();
    let total_lines  = current_tab.total_lines();
    let visible_count = ca.visible_line_count();
    let scroll_x      = current_tab.scroll_offset_x();
    let line_height   = ca.text.line_height;
    let text_padding  = ca.text.text_padding;
    let doc_top_margin = ca.text.doc_top_margin;
    let char_width    = logic.char_width_hint;

    let lines: Vec<TextLine> = current_tab.content().lines()
        .enumerate()
        .skip(scroll_offset)
        .take(visible_count)
        .map(|(idx, text)| {
            let visual = idx - scroll_offset;
            let y = ca.text.rect.y + doc_top_margin + visual as f32 * line_height;
            TextLine { line_index: idx, text: text.to_string(), y }
        })
        .collect();

    // Cursor
    let cursor = {
        let cl = current_tab.cursor_line();
        let cc = current_tab.cursor_col();
        let (cx, cy) = if cl >= scroll_offset {
            let visual = cl - scroll_offset;
            let y = ca.text.rect.y + doc_top_margin + visual as f32 * line_height;
            let line_text = current_tab.content().lines().nth(cl).unwrap_or("");
            let vl = VisualLine::new(line_text);
            let x  = vl.char_col_to_visual_x(cc, text_padding - scroll_x, char_width);
            (x, y)
        } else { (0.0, -999.0) };
        EditorCursor { x: cx, y: cy, line_height, visible: logic.cursor_visible }
    };

    // Selection
    let selection = current_tab.selection_range_line_col().map(|(start, end)| {
        let (start_line, start_col) = start;
        let (end_line, end_col)     = end;
        let mut rects = Vec::new();
        for ld in lines.iter() {
            let li = ld.line_index;
            if li < start_line || li > end_line { continue; }
            let sc = if li == start_line { start_col } else { 0 };
            let line_text = &ld.text;
            let ec = if li == end_line { end_col.min(line_text.chars().count()) }
                     else              { line_text.chars().count() };
            if sc >= ec { continue; }
            let vl = VisualLine::new(line_text);
            let x1 = vl.char_col_to_visual_x(sc, text_padding - scroll_x, char_width);
            let x2 = vl.char_col_to_visual_x(ec, text_padding - scroll_x, char_width);
            let sel_rect = crate::ui::Rect {
                x: x1, y: ld.y,
                width: (x2 - x1).max(char_width), height: line_height,
            };
            rects.push(sel_rect);
        }
        EditorSelection { rects }
    });

    // Scrollbar
    let scrollbar = if total_lines > visible_count && visible_count > 0 {
        let s = &ca.scrollbar;
        s.thumb(total_lines, visible_count, scroll_offset).map(|thumb| ScrollbarNode {
            track:       s.rect,
            thumb:       thumb.rect,
            track_color: NColor::rgba(0.0, 0.0, 0.0, 0.0),
            thumb_color: NColor::rgba(1.0, 1.0, 1.0,
                if ca.scrollbar_hovered { 0.35 } else { 0.20 }),
        })
    } else { None };

    let sel_rects = selection.as_ref().map(|s| s.rects.clone()).unwrap_or_default();

    let node = Node::Editor(EditorNode {
        rect:        ca.rect,
        line_height,
        text_padding,
        char_width,
        scroll_x,
        word_wrap:   current_tab.word_wrap(),
        lines,
        cursor,
        selection,
        scrollbar,
        flames: Vec::new(), // filled in by the caller after flames::build()
    });

    (node, sel_rects)
}
