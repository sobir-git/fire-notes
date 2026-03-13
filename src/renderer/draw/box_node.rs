//! Box node rendering — filled + optionally bordered rounded rect.

use femtovg::{Paint, Path};
use crate::layout::node::BoxStyle;
use crate::ui::Rect;
use super::super::draw_ctx::DrawCtx;
use super::super::node_renderer::nc;

pub fn draw(rect: &Rect, style: &BoxStyle, ctx: &mut DrawCtx<'_>) {
    let mut path = Path::new();
    if style.radius > 0.0 {
        path.rounded_rect(rect.x, rect.y, rect.width, rect.height, style.radius);
    } else {
        path.rect(rect.x, rect.y, rect.width, rect.height);
    }
    ctx.canvas.fill_path(&path, &Paint::color(nc(style.fill)));

    if style.border_width > 0.0 {
        let mut bp = Path::new();
        if style.radius > 0.0 {
            bp.rounded_rect(rect.x, rect.y, rect.width, rect.height, style.radius);
        } else {
            bp.rect(rect.x, rect.y, rect.width, rect.height);
        }
        ctx.canvas.stroke_path(
            &bp,
            &Paint::color(nc(style.border)).with_line_width(style.border_width),
        );
    }
}
