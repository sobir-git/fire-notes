//! Text content renderer — orchestrates text, cursor, flame and scrollbar drawing.

use std::time::Instant;

use femtovg::{Canvas, Color, FontId, Paint, Path, renderer::OpenGl};

use crate::config::rendering;
use crate::tab::Tab;
use crate::theme::Theme;
use crate::ui::{ContentArea, ScrollbarWidget};

use super::super::flame::FlameSystem;
use super::text as txt;

pub struct TextContentRenderer<'a> {
    pub(super) canvas: &'a mut Canvas<OpenGl>,
    pub(super) fonts: &'a [FontId],
    pub(super) theme: &'a Theme,
    pub(super) width: f32,
    pub(super) height: f32,
    pub(super) scale: f32,
    pub(super) animation_start: Instant,
}

impl<'a> TextContentRenderer<'a> {
    pub fn new(
        canvas: &'a mut Canvas<OpenGl>,
        fonts: &'a [FontId],
        theme: &'a Theme,
        width: f32,
        height: f32,
        scale: f32,
        animation_start: Instant,
    ) -> Self {
        Self { canvas, fonts, theme, width, height, scale, animation_start }
    }

    pub fn draw(
        &mut self,
        tab: &Tab,
        content_area: &ContentArea,
        scrollbar: &ScrollbarWidget,
        cursor_visible: bool,
        hovered_scrollbar: bool,
        dragging_scrollbar: bool,
        flame_system: &mut FlameSystem,
        typing_flame_positions: &[(usize, usize, std::time::Instant)],
    ) {
        let start_y     = content_area.start_y();
        let line_height = content_area.line_height;
        let padding     = content_area.text_padding(self.scale);
        let scroll_offset = tab.scroll_offset();
        let scroll_x      = tab.scroll_offset_x();
        let text          = tab.content();

        let mut text_paint = Paint::color(Color::rgbf(self.theme.fg.0, self.theme.fg.1, self.theme.fg.2));
        text_paint.set_font(self.fonts);
        text_paint.set_font_size(rendering::CONTENT_FONT_SIZE * self.scale);
        let char_width = txt::measure_char_width(self.canvas, &text_paint, self.scale);

        // ── Flame positions (selection + typing) ──────────────────────────
        let mut char_positions = txt::collect_selection_positions(
            self.width, self.height, tab, text,
            scroll_offset, scroll_x, start_y, line_height, padding, char_width,
        );
        let now = Instant::now();
        let text_lines: Vec<&str> = text.lines().collect();
        for &(line, col, timestamp) in typing_flame_positions {
            if line < scroll_offset { continue; }
            let y = start_y + ((line - scroll_offset) as f32 * line_height);
            if y > self.height { continue; }
            let char_x = if line < text_lines.len() {
                crate::visual_position::VisualLine::new(text_lines[line])
                    .char_col_to_visual_center_x(col, padding - scroll_x, char_width)
            } else {
                padding - scroll_x + char_width * 0.5
            };
            char_positions.push((char_x, y + line_height * 0.5, y + line_height,
                now.duration_since(timestamp).as_secs_f32().min(1.0)));
        }

        if !char_positions.is_empty() { flame_system.update_legacy(&char_positions, self.scale); }
        else                          { flame_system.clear(); }

        if !char_positions.is_empty() { flame_system.draw_layer(self.canvas, true); }

        // ── Cursor position ───────────────────────────────────────────────
        let cursor_rect = txt::calculate_cursor_position(
            text, tab.cursor_position(), scroll_offset, scroll_x,
            start_y, line_height, self.height, padding, char_width,
        );

        // ── Text lines ────────────────────────────────────────────────────
        txt::draw_text_lines(
            self.canvas, self.fonts, self.theme,
            self.width, self.height, self.scale, self.animation_start,
            text, scroll_offset, scroll_x, tab.word_wrap(),
            start_y, line_height, padding, char_width,
            &text_paint, &char_positions,
        );

        // ── Cursor ────────────────────────────────────────────────────────
        if cursor_visible {
            if let Some((cx, cy)) = cursor_rect {
                txt::draw_cursor(self.canvas, self.theme, self.scale, cx, cy, line_height);
            }
        }

        if !char_positions.is_empty() { flame_system.draw_layer(self.canvas, false); }

        // ── Scrollbar ─────────────────────────────────────────────────────
        self.draw_scrollbar(tab, content_area, scrollbar, scroll_offset, hovered_scrollbar, dragging_scrollbar);
    }

    fn draw_scrollbar(
        &mut self,
        tab: &Tab,
        content_area: &ContentArea,
        scrollbar: &ScrollbarWidget,
        scroll_offset: usize,
        hovered: bool,
        dragging: bool,
    ) {
        let total_lines = tab.total_lines().max(1);
        let max_visible = content_area.visible_line_count();
        if let Some(m) = scrollbar.thumb(total_lines, max_visible, scroll_offset) {
            let r = m.rect;
            let alpha = if dragging { 140u8 } else if hovered { 90 } else { 50 };
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
