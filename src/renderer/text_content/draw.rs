//! Text, cursor, flame and scrollbar drawing.

use std::time::Instant;

use femtovg::{Canvas, Color, FontId, Paint, Path, renderer::OpenGl};

use crate::config::{layout, rendering};
use crate::tab::Tab;
use crate::theme::Theme;
use crate::ui::ScrollbarWidget;

use super::layout::{FlameHit, build_flame_lookup, get_cursor_line_col};
use super::super::flame::FlameSystem;

/// Snap a coordinate to the pixel grid to prevent blurry text rendering.
#[inline]
fn snap_to_pixel(coord: f32) -> f32 {
    coord.round()
}

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
        scrollbar: &ScrollbarWidget,
        cursor_visible: bool,
        hovered_scrollbar: bool,
        dragging_scrollbar: bool,
        flame_system: &mut FlameSystem,
        typing_flame_positions: &[(usize, usize, std::time::Instant)],
    ) {
        let tab_height = layout::TAB_HEIGHT * self.scale;
        let padding = layout::PADDING * self.scale;
        let line_height = layout::LINE_HEIGHT * self.scale;
        let start_y = tab_height + padding;
        let scroll_offset = tab.scroll_offset();
        let scroll_x = tab.scroll_offset_x();
        let do_wrap = tab.word_wrap();
        let text = tab.content();
        let cursor_pos = tab.cursor_position();

        let mut text_paint = Paint::color(Color::rgbf(self.theme.fg.0, self.theme.fg.1, self.theme.fg.2));
        text_paint.set_font(self.fonts);
        text_paint.set_font_size(rendering::CONTENT_FONT_SIZE * self.scale);
        let char_width = self.measure_char_width(&text_paint);

        let mut char_positions = self.collect_selection_positions(
            tab, text, scroll_offset, scroll_x, do_wrap,
            start_y, line_height, padding, char_width,
        );

        let now = std::time::Instant::now();
        let text_lines: Vec<&str> = text.lines().collect();
        for &(line, col, timestamp) in typing_flame_positions {
            if line < scroll_offset { continue; }
            let visible_idx = line - scroll_offset;
            let y = start_y + (visible_idx as f32 * line_height);
            if y > self.height { continue; }
            let age = now.duration_since(timestamp).as_secs_f32().min(1.0);
            let line_bottom_y = y + line_height;
            let char_x = if line < text_lines.len() {
                let visual_line = crate::visual_position::VisualLine::new(text_lines[line]);
                visual_line.char_col_to_visual_center_x(col, padding - scroll_x, char_width)
            } else {
                padding - scroll_x + char_width * 0.5
            };
            let char_y = y + line_height * 0.5;
            char_positions.push((char_x, char_y, line_bottom_y, age));
        }

        if !char_positions.is_empty() {
            flame_system.update_legacy(&char_positions, self.scale);
        } else {
            flame_system.clear();
        }

        if !char_positions.is_empty() {
            flame_system.draw_layer(self.canvas, true);
        }

        let cursor_rect = self.calculate_cursor_position(
            text, cursor_pos, scroll_offset, scroll_x,
            start_y, line_height, padding, char_width,
        );

        self.draw_text_lines(
            text, scroll_offset, scroll_x, do_wrap,
            start_y, line_height, padding, char_width,
            &text_paint, &char_positions,
        );

        if cursor_visible {
            if let Some((cx, cy)) = cursor_rect {
                let mut cursor_path = Path::new();
                cursor_path.rect(cx, cy, 2.0 * self.scale, line_height);
                self.canvas.fill_path(
                    &cursor_path,
                    &Paint::color(Color::rgbf(self.theme.cursor.0, self.theme.cursor.1, self.theme.cursor.2)),
                );
            }
        }

        if !char_positions.is_empty() {
            flame_system.draw_layer(self.canvas, false);
        }

        self.draw_scrollbar(tab, scrollbar, scroll_offset, hovered_scrollbar, dragging_scrollbar);
    }

    fn collect_selection_positions(
        &self,
        tab: &Tab,
        text: &str,
        scroll_offset: usize,
        scroll_x: f32,
        do_wrap: bool,
        start_y: f32,
        line_height: f32,
        padding: f32,
        char_width: f32,
    ) -> Vec<(f32, f32, f32, f32)> {
        let mut char_positions = Vec::new();
        if !do_wrap {
            if let Some(((start_line, start_col), (end_line, end_col))) = tab.selection_range_line_col() {
                let visible_start = scroll_offset.max(start_line);
                let visible_end = end_line;
                for (line_idx, line_content) in text.lines()
                    .enumerate()
                    .skip(visible_start)
                    .take_while(|(idx, _)| *idx <= visible_end)
                {
                    let visible_idx = line_idx.saturating_sub(scroll_offset);
                    let y = start_y + (visible_idx as f32 * line_height);
                    if y > self.height { break; }
                    let line_bottom_y = y + line_height;
                    let start_col_in_line = if line_idx == start_line { start_col } else { 0 };
                    let end_col_in_line = if line_idx == end_line {
                        end_col.min(line_content.chars().count())
                    } else {
                        line_content.chars().count()
                    };
                    let visual_line = crate::visual_position::VisualLine::new(line_content);
                    for col in start_col_in_line..end_col_in_line {
                        let char_x = visual_line.char_col_to_visual_center_x(col, padding - scroll_x, char_width);
                        if char_x < -char_width || char_x > self.width + char_width { continue; }
                        let char_y = y + line_height * 0.5;
                        char_positions.push((char_x, char_y, line_bottom_y, 0.0));
                    }
                }
            }
        }
        char_positions
    }

    fn calculate_cursor_position(
        &self,
        text: &str,
        cursor_pos: usize,
        scroll_offset: usize,
        scroll_x: f32,
        start_y: f32,
        line_height: f32,
        padding: f32,
        char_width: f32,
    ) -> Option<(f32, f32)> {
        let (cursor_line, cursor_col) = get_cursor_line_col(text, cursor_pos);
        if cursor_line < scroll_offset { return None; }
        let visual_line = cursor_line - scroll_offset;
        let y = start_y + (visual_line as f32 * line_height);
        if y > self.height { return None; }
        let line_content = text.lines().nth(cursor_line).unwrap_or("");
        let vl = crate::visual_position::VisualLine::new(line_content);
        let x = vl.char_col_to_visual_x(cursor_col, padding - scroll_x, char_width);
        Some((x, y))
    }

    fn draw_text_lines(
        &mut self,
        text: &str,
        scroll_offset: usize,
        scroll_x: f32,
        do_wrap: bool,
        start_y: f32,
        line_height: f32,
        padding: f32,
        char_width: f32,
        text_paint: &Paint,
        char_positions: &[(f32, f32, f32, f32)],
    ) {
        let flame_lookup = build_flame_lookup(char_positions, char_width, line_height);
        let cell_w = char_width.max(1.0);
        let cell_h = line_height.max(1.0);
        let mut current_y = start_y;

        for line in text.lines().skip(scroll_offset) {
            if current_y > self.height { break; }
            let mut x_offset = if do_wrap { padding } else { padding - scroll_x };

            for ch in line.chars() {
                let advance = crate::visual_position::get_char_visual_width(ch);
                let char_w = char_width * advance as f32;

                if do_wrap && x_offset + char_w > self.width - padding {
                    current_y += line_height;
                    x_offset = padding;
                    if current_y > self.height { break; }
                }

                if current_y + line_height > 0.0 && current_y < self.height {
                    if !ch.is_control() && ch != ' ' {
                        let text_x = snap_to_pixel(x_offset);
                        let text_y_snapped = snap_to_pixel(current_y + line_height * 0.75);
                        let char_center_x = x_offset + char_width * 0.5;
                        let char_center_y = current_y + line_height * 0.5;
                        let grid_x = (char_center_x / cell_w) as i32;
                        let grid_y = (char_center_y / cell_h) as i32;
                        let flame_hit = flame_lookup.get(&(grid_x, grid_y)).copied().unwrap_or(FlameHit::None);

                        let mut buf = [0u8; 4];
                        let s = ch.encode_utf8(&mut buf);

                        match flame_hit {
                            FlameHit::Typing(age) => {
                                let fade = age;
                                let r = 1.0;
                                let g = 0.3 * (1.0 - fade) + 1.0 * fade;
                                let b = 0.0 * (1.0 - fade) + 1.0 * fade;
                                let mut paint = Paint::color(Color::rgbf(r, g, b));
                                paint.set_font(self.fonts);
                                paint.set_font_size(16.0 * self.scale);
                                let _ = self.canvas.fill_text(text_x, text_y_snapped, s, &paint);
                            }
                            FlameHit::Selection => {
                                let paint = self.create_burning_paint(x_offset, current_y);
                                let _ = self.canvas.fill_text(text_x, text_y_snapped, s, &paint);
                            }
                            FlameHit::None => {
                                let _ = self.canvas.fill_text(text_x, text_y_snapped, s, text_paint);
                            }
                        }
                    }
                }
                x_offset += char_w;
            }
            current_y += line_height;
        }
    }

    fn create_burning_paint(&self, x_offset: f32, current_y: f32) -> Paint {
        let phase_offset = (x_offset * 0.1 + current_y * 0.07) % std::f32::consts::TAU;
        let time = self.animation_start.elapsed().as_secs_f32() * 2.5;
        let cycle = (time + phase_offset).sin() * 0.5 + 0.5;
        let r = 0.9 + cycle * 0.1;
        let g = 0.15 + cycle * 0.25;
        let b = cycle * 0.05;
        let mut burning_paint = Paint::color(Color::rgbf(r, g, b));
        burning_paint.set_font(self.fonts);
        burning_paint.set_font_size(rendering::CONTENT_FONT_SIZE * self.scale);
        burning_paint
    }

    fn draw_scrollbar(
        &mut self,
        tab: &Tab,
        scrollbar: &ScrollbarWidget,
        scroll_offset: usize,
        hovered_scrollbar: bool,
        dragging_scrollbar: bool,
    ) {
        let line_height = layout::LINE_HEIGHT * self.scale;
        let padding = layout::PADDING * self.scale;
        let start_y = layout::TAB_HEIGHT * self.scale + padding;
        let max_visible_lines = ((self.height - start_y - padding) / line_height).ceil() as usize;
        let total_lines = tab.total_lines().max(1);
        if total_lines > max_visible_lines {
            if let Some(metrics) = scrollbar.metrics(total_lines, max_visible_lines, scroll_offset) {
                let mut path = Path::new();
                path.rounded_rect(metrics.thumb.x, metrics.thumb.y, metrics.thumb.width, metrics.thumb.height, 4.0);
                let thumb_alpha = if dragging_scrollbar { 140 } else if hovered_scrollbar { 90 } else { 50 };
                let thumb_color = Paint::color(Color::rgba(
                    (self.theme.fg.0 * 255.0) as u8,
                    (self.theme.fg.1 * 255.0) as u8,
                    (self.theme.fg.2 * 255.0) as u8,
                    thumb_alpha,
                ));
                self.canvas.fill_path(&path, &thumb_color);
            }
        }
    }


    pub(super) fn measure_char_width(&self, paint: &Paint) -> f32 {
        if let Ok(metrics) = self.canvas.measure_text(0.0, 0.0, "M", paint) {
            metrics.width()
        } else {
            rendering::FALLBACK_CHAR_WIDTH * self.scale
        }
    }
}
