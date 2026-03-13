//! Cursor node rendering — blinking vertical bar.

use femtovg::{Paint, Path};
use crate::layout::node::CursorNode;
use super::super::draw_ctx::DrawCtx;
use super::super::node_renderer::nc;

pub fn draw(c: &CursorNode, ctx: &mut DrawCtx<'_>) {
    if !c.visible { return; }
    let mut path = Path::new();
    path.rect(c.rect.x, c.rect.y, c.rect.width, c.rect.height);
    ctx.canvas.fill_path(&path, &Paint::color(nc(c.color)));
}
