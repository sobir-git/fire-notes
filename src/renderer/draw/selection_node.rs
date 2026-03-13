//! Selection node rendering — highlight rect behind selected text.

use femtovg::{Paint, Path};
use crate::layout::node::SelectionNode;
use super::super::draw_ctx::DrawCtx;
use super::super::node_renderer::nc;

pub fn draw(s: &SelectionNode, ctx: &mut DrawCtx<'_>) {
    let mut path = Path::new();
    path.rect(s.rect.x, s.rect.y, s.rect.width, s.rect.height);
    ctx.canvas.fill_path(&path, &Paint::color(nc(s.color)));
}
