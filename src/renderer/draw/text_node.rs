//! Text node rendering — single string with optional scissor clip.

use femtovg::Paint;
use crate::layout::node::TextStyle;
use crate::ui::Rect;
use super::super::draw_ctx::DrawCtx;
use super::super::node_renderer::nc;

pub fn draw(text: &str, style: &TextStyle, clip: Option<&Rect>, ctx: &mut DrawCtx<'_>) {
    if text.is_empty() { return; }
    let mut paint = Paint::color(nc(style.color));
    paint.set_font(ctx.fonts);
    paint.set_font_size(style.font_size);
    let x = style.text_x - style.scroll_x;
    if let Some(r) = clip {
        ctx.canvas.save();
        ctx.canvas.scissor(r.x, r.y, r.width, r.height);
        let _ = ctx.canvas.fill_text(x, style.baseline_y, text, &paint);
        ctx.canvas.restore();
    } else {
        let _ = ctx.canvas.fill_text(x, style.baseline_y, text, &paint);
    }
}
