//! Text content renderer — orchestrates text, cursor, flame and scrollbar drawing.

use std::time::Instant;

use femtovg::{Canvas, Color, FontId, Paint, Path, renderer::OpenGl};

use crate::config::rendering;
use crate::tab::Tab;
use crate::theme::Theme;
use crate::ui::{ContentArea, ScrollbarWidget};

use super::super::flame::FlameSystem;
use super::text::{self as txt, DrawCtx};

/// Scrollbar widget + per-frame interaction state.
pub struct ScrollbarState<'a> {
    pub widget:    &'a ScrollbarWidget,
    pub hovered:   bool,
    pub dragging:  bool,
}

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
        tab: &Tab,
        content_area: &ContentArea,
        scrollbar: &ScrollbarState<'_>,
        cursor_visible: bool,
        flame_system: &mut FlameSystem,
        typing_flame_positions: &[(usize, usize, std::time::Instant)],
    ) {
        let mut text_paint = Paint::color(Color::rgbf(self.theme.fg.0, self.theme.fg.1, self.theme.fg.2));
        text_paint.set_font(self.fonts);
        text_paint.set_font_size(rendering::CONTENT_FONT_SIZE * self.scale);
        let char_width = txt::measure_char_width(self.canvas, &text_paint, self.scale);

        let ctx = DrawCtx {
            tab,
            text_area:  &content_area.text,
            char_width,
            viewport_h: content_area.rect.height + content_area.rect.y,
            viewport_w: content_area.rect.width  + content_area.rect.x,
        };

        // ── Flame positions (selection + typing) ──────────────────────────
        let mut char_positions = txt::collect_selection_positions(&ctx);
        append_typing_flame_positions(&mut char_positions, &ctx, typing_flame_positions);

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
        if cursor_visible {
            if let Some((cx, cy)) = cursor_rect {
                txt::draw_cursor(self.canvas, self.theme, self.scale, cx, cy, ctx.text_area.line_height);
            }
        }

        if !char_positions.is_empty() { flame_system.draw_layer(self.canvas, false); }

        // ── Scrollbar ─────────────────────────────────────────────────────
        self.draw_scrollbar(tab, content_area, scrollbar);
    }

}

fn append_typing_flame_positions(
    positions: &mut Vec<(f32, f32, f32, f32)>,
    ctx: &DrawCtx<'_>,
    typing: &[(usize, usize, std::time::Instant)],
) {
    let now = Instant::now();
    let text_lines: Vec<&str> = ctx.tab.content().lines().collect();
    let scroll_offset = ctx.tab.scroll_offset();
    let scroll_x      = ctx.tab.scroll_offset_x();
    for &(line, col, timestamp) in typing {
        if line < scroll_offset { continue; }
        let y = ctx.text_area.line_y(line - scroll_offset);
        if y > ctx.viewport_h { continue; }
        let char_x = if line < text_lines.len() {
            crate::visual_position::VisualLine::new(text_lines[line])
                .char_col_to_visual_center_x(col, ctx.text_area.text_padding - scroll_x, ctx.char_width)
        } else {
            ctx.text_area.text_padding - scroll_x + ctx.char_width * 0.5
        };
        let lh = ctx.text_area.line_height;
        positions.push((char_x, y + lh * 0.5, y + lh,
            now.duration_since(timestamp).as_secs_f32().min(1.0)));
    }
}

impl<'a> TextContentRenderer<'a> {
    fn draw_scrollbar(
        &mut self,
        tab: &Tab,
        content_area: &ContentArea,
        scrollbar: &ScrollbarState<'_>,
    ) {
        let total_lines = tab.total_lines().max(1);
        let max_visible = content_area.visible_line_count();
        if let Some(m) = scrollbar.widget.thumb(total_lines, max_visible, tab.scroll_offset()) {
            let r = m.rect;
            let alpha = if scrollbar.dragging { 140u8 } else if scrollbar.hovered { 90 } else { 50 };
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
}
