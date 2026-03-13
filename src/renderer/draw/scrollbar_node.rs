//! Scrollbar node rendering — track + rounded thumb.

use femtovg::{Paint, Path};
use crate::layout::node::ScrollbarNode;
use super::super::draw_ctx::DrawCtx;
use super::super::node_renderer::nc;

pub fn draw(s: &ScrollbarNode, ctx: &mut DrawCtx<'_>) {
    let mut track = Path::new();
    track.rect(s.track.x, s.track.y, s.track.width, s.track.height);
    ctx.canvas.fill_path(&track, &Paint::color(nc(s.track_color)));

    let r = s.thumb.width / 2.0;
    let mut thumb = Path::new();
    thumb.rounded_rect(s.thumb.x, s.thumb.y, s.thumb.width, s.thumb.height, r);
    ctx.canvas.fill_path(&thumb, &Paint::color(nc(s.thumb_color)));
}
