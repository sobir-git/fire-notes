//! Builds the flame spawn-point list for the current frame.

use crate::layout::node::FlamePos;
use crate::logic::AppLogic;
use crate::visual_position::VisualLine;

pub(super) fn build(
    logic: &AppLogic,
    selection_rects: &[crate::ui::Rect],
    scroll_offset: usize,
    line_height: f32,
    text_padding: f32,
    doc_top_margin: f32,
    scroll_x: f32,
) -> Vec<FlamePos> {
    let ca         = &logic.ui_tree.content_area;
    let char_width = logic.char_width_hint;
    let cw         = char_width.max(1.0);
    let now        = std::time::Instant::now();

    let mut flames: Vec<FlamePos> = Vec::new();

    // Selection flames — one spawn point per character column across each selection rect
    for r in selection_rects {
        let bottom  = r.y + r.height;
        let cy      = r.y + r.height * 0.5;
        let n_chars = (r.width / cw).ceil() as usize;
        for i in 0..n_chars.max(1) {
            let cx = r.x + (i as f32 + 0.5) * cw;
            flames.push(FlamePos { cx, cy, bottom, age: 0.0 });
        }
    }

    // Typing flames
    let current_tab = &logic.tabs[logic.active_tab];
    for &(line, col, timestamp) in &logic.typing_flame_positions {
        if line < scroll_offset { continue; }
        let visual = line - scroll_offset;
        let y = ca.text.rect.y + doc_top_margin + visual as f32 * line_height;
        if y > ca.rect.y + ca.rect.height { continue; }
        let line_text = current_tab.content().lines().nth(line).unwrap_or("");
        let vl  = VisualLine::new(line_text);
        let cx  = vl.char_col_to_visual_center_x(col, text_padding - scroll_x, char_width);
        let age = now.duration_since(timestamp).as_secs_f32().min(1.0);
        flames.push(FlamePos { cx, cy: y + line_height * 0.5, bottom: y + line_height, age });
    }

    flames
}
