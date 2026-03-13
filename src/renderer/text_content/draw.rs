//! Text content renderer — orchestrates text, cursor, flame and scrollbar drawing.

use std::time::Instant;

use femtovg::{Canvas, Color, FontId, Paint, Path, renderer::OpenGl};

use crate::config::rendering;
use crate::render_frame::RenderFrame;
use crate::theme::Theme;

use super::super::flame::FlameSystem;
use super::text::{self as txt, DrawCtx};


pub struct TextContentRenderer<'a> {
    pub(super) canvas: &'a mut Canvas<OpenGl>,
    pub(super) fonts: &'a [FontId],
    pub(super) theme: &'a Theme,
    pub(super) scale: f32,
    pub(super) animation_start: Instant,
}

impl<'a> TextContentRenderer<'a> {
    pub fn new(
        canvas: &'a mut Canvas<OpenGl>,
        fonts: &'a [FontId],
        theme: &'a Theme,
        scale: f32,
        animation_start: Instant,
    ) -> Self {
        Self { canvas, fonts, theme, scale, animation_start }
    }

    pub fn draw(
        &mut self,
        frame: &RenderFrame,
        flame_system: &mut FlameSystem,
    ) {
        let mut text_paint = Paint::color(Color::rgbf(self.theme.fg.0, self.theme.fg.1, self.theme.fg.2));
        text_paint.set_font(self.fonts);
        text_paint.set_font_size(rendering::CONTENT_FONT_SIZE * self.scale);
        let char_width = txt::measure_char_width(self.canvas, &text_paint, self.scale);

        let ctx = DrawCtx {
            frame,
            char_width,
        };

        // ── Flame positions (selection + typing) ──────────────────────────
        let mut char_positions = txt::collect_selection_positions(&ctx);
        append_typing_flame_positions(&mut char_positions, &ctx, &frame.flame_positions);

        if !char_positions.is_empty() { flame_system.update_legacy(&char_positions, self.scale); }
        else                          { flame_system.clear(); }

        if !char_positions.is_empty() { flame_system.draw_layer(self.canvas, true); }

        // ── Cursor position ───────────────────────────────────────────────
        let cursor_rect = txt::calculate_cursor_position(&ctx);

        // ── Text lines ────────────────────────────────────────────────────
        txt::draw_text_lines(
            self.canvas, self.fonts,
            self.scale, self.animation_start,
            &ctx, &text_paint, &char_positions,
        );

        // ── Cursor ────────────────────────────────────────────────────────
        if frame.cursor.visible {
            if let Some((cx, cy)) = cursor_rect {
                txt::draw_cursor(self.canvas, self.theme, self.scale, cx, cy, frame.content.line_height);
            }
        }

        if !char_positions.is_empty() { flame_system.draw_layer(self.canvas, false); }

        // ── Scrollbar ─────────────────────────────────────────────────────
        self.draw_scrollbar_from_frame(frame);
    }

}

fn append_typing_flame_positions(
    positions: &mut Vec<(f32, f32, f32, f32)>,
    ctx: &DrawCtx<'_>,
    typing: &[(usize, usize, std::time::Instant)],
) {
    let now = Instant::now();
    let frame         = ctx.frame;
    let scroll_offset = frame.scroll_offset;
    let scroll_x      = frame.scroll_offset_x;
    let bottom        = frame.content.rect.y + frame.content.rect.height;
    let line_height   = frame.content.line_height;
    let text_padding  = frame.content.text_padding;
    for &(line, col, timestamp) in typing {
        if line < scroll_offset { continue; }
        let visual = line - scroll_offset;
        let y = frame.content.text_rect.y + frame.content.doc_top_margin
            + visual as f32 * line_height;
        if y > bottom { continue; }
        let line_text = frame.visible_lines.iter()
            .find(|l| l.line_index == line)
            .map(|l| l.text.as_str())
            .unwrap_or("");
        let char_x = crate::visual_position::VisualLine::new(line_text)
            .char_col_to_visual_center_x(col, text_padding - scroll_x, ctx.char_width);
        positions.push((char_x, y + line_height * 0.5, y + line_height,
            now.duration_since(timestamp).as_secs_f32().min(1.0)));
    }
}

impl<'a> TextContentRenderer<'a> {
    fn draw_scrollbar_from_frame(&mut self, frame: &RenderFrame) {
        let Some(sb) = &frame.scrollbar_geom else { return; };
        let r = &sb.thumb_rect;
        let alpha = if sb.dragging { 140u8 } else if sb.hovered { 90 } else { 50 };
        let color = Paint::color(Color::rgba(
            (self.theme.fg.0 * 255.0) as u8,
            (self.theme.fg.1 * 255.0) as u8,
            (self.theme.fg.2 * 255.0) as u8,
            alpha,
        ));
        let mut path = Path::new();
        path.rounded_rect(r.x, r.y, r.width, r.height, 4.0);
        self.canvas.fill_path(&path, &color);
    }
}
