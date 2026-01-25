//! Text content and editor area rendering

use crate::tab::Tab;
use crate::theme::Theme;
use crate::ui::ScrollbarWidget;
use femtovg::{Canvas, Color, FontId, Paint, Path, renderer::OpenGl};
use std::time::Instant;

use super::flame::FlameSystem;

/// Snap a coordinate to the pixel grid to prevent blurry text rendering.
#[inline]
fn snap_to_pixel(coord: f32) -> f32 {
    coord.round()
}

pub struct TextContentRenderer<'a> {
    canvas: &'a mut Canvas<OpenGl>,
    fonts: &'a [FontId],
    theme: &'a Theme,
    width: f32,
    height: f32,
    scale: f32,
    animation_start: Instant,
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
        Self {
            canvas,
            fonts,
            theme,
            width,
            height,
            scale,
            animation_start,
        }
    }

    pub fn draw(
        &mut self,
        tab: &Tab,
        cursor_visible: bool,
        hovered_scrollbar: bool,
        dragging_scrollbar: bool,
        flame_system: &mut FlameSystem,
    ) {
        let tab_height = 40.0 * self.scale;
        let padding = 16.0 * self.scale;
        let line_height = 24.0 * self.scale;
        let start_y = tab_height + padding;
        let scroll_offset = tab.scroll_offset();
        let scroll_x = tab.scroll_offset_x();
        let do_wrap = tab.word_wrap();

        let text = tab.content();
        let cursor_pos = tab.cursor_position();

        // Setup text paint
        let mut text_paint = Paint::color(Color::rgbf(
            self.theme.fg.0,
            self.theme.fg.1,
            self.theme.fg.2,
        ));
        text_paint.set_font(self.fonts);
        text_paint.set_font_size(16.0 * self.scale);
        let char_width = self.measure_char_width(&text_paint);

        // Collect character positions for flame spawning (no selection rectangle)
        let char_positions = self.collect_selection_positions(
            tab,
            text,
            scroll_offset,
            scroll_x,
            do_wrap,
            start_y,
            line_height,
            padding,
            char_width,
        );

        // Update flame particles
        if !char_positions.is_empty() {
            flame_system.update(&char_positions, self.scale);
        } else {
            flame_system.clear();
        }

        // Draw background flame layer (behind text)
        if !char_positions.is_empty() {
            flame_system.draw_layer(self.canvas, &char_positions, self.scale, true);
        }

        // Draw text and cursor
        let cursor_rect = self.draw_text_lines(
            text,
            cursor_pos,
            scroll_offset,
            scroll_x,
            do_wrap,
            start_y,
            line_height,
            padding,
            char_width,
            &text_paint,
            &char_positions,
        );

        // Draw Cursor
        if cursor_visible {
            if let Some((cx, cy)) = cursor_rect {
                let mut cursor_path = Path::new();
                cursor_path.rect(cx, cy, 2.0 * self.scale, line_height);
                self.canvas.fill_path(
                    &cursor_path,
                    &Paint::color(Color::rgbf(
                        self.theme.cursor.0,
                        self.theme.cursor.1,
                        self.theme.cursor.2,
                    )),
                );
            }
        }

        // Draw foreground flame layer (in front of text)
        if !char_positions.is_empty() {
            flame_system.draw_layer(self.canvas, &char_positions, self.scale, false);
        }

        // Draw scrollbar
        self.draw_scrollbar(
            tab,
            start_y,
            padding,
            line_height,
            scroll_offset,
            hovered_scrollbar,
            dragging_scrollbar,
        );
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
    ) -> Vec<(f32, f32, f32)> {
        let mut char_positions = Vec::new();

        if !do_wrap {
            if let Some(((start_line, start_col), (end_line, end_col))) =
                tab.selection_range_line_col()
            {
                let text_lines: Vec<&str> = text.lines().collect();

                for line_idx in start_line..=end_line {
                    if line_idx < scroll_offset {
                        continue;
                    }
                    let visible_idx = line_idx - scroll_offset;
                    let y = start_y + (visible_idx as f32 * line_height);
                    if y > self.height {
                        break;
                    }

                    let line_bottom_y = y + line_height;

                    // Get the actual line content
                    if line_idx >= text_lines.len() {
                        continue;
                    }
                    let line_content = text_lines[line_idx];

                    let start_col_in_line = if line_idx == start_line { start_col } else { 0 };
                    let end_col_in_line = if line_idx == end_line {
                        end_col.min(line_content.chars().count())
                    } else {
                        line_content.chars().count()
                    };

                    // Collect position for each selected character
                    for col in start_col_in_line..end_col_in_line {
                        let char_x =
                            padding - scroll_x + (col as f32 * char_width) + (char_width * 0.5);
                        let char_y = y + line_height * 0.5;
                        char_positions.push((char_x, char_y, line_bottom_y));
                    }
                }
            }
        }

        char_positions
    }

    fn draw_text_lines(
        &mut self,
        text: &str,
        cursor_pos: usize,
        scroll_offset: usize,
        scroll_x: f32,
        do_wrap: bool,
        start_y: f32,
        line_height: f32,
        padding: f32,
        char_width: f32,
        text_paint: &Paint,
        char_positions: &[(f32, f32, f32)],
    ) -> Option<(f32, f32)> {
        let lines: Vec<&str> = text.lines().skip(scroll_offset).collect();
        let mut current_y = start_y;
        let (cursor_line_idx, cursor_col_idx) = get_cursor_line_col(text, cursor_pos);
        let mut cursor_rect = None;

        for (idx, line) in lines.iter().enumerate() {
            let logical_line_idx = scroll_offset + idx;
            if current_y > self.height {
                break;
            }

            let mut x_offset = if do_wrap {
                padding
            } else {
                padding - scroll_x
            };
            let line_has_cursor = logical_line_idx == cursor_line_idx;

            // Check cursor at start of line (col 0)
            if line_has_cursor && cursor_col_idx == 0 {
                cursor_rect = Some((x_offset, current_y));
            }

            let mut current_col = 0;
            let mut line_chars = line.chars();

            while let Some(ch) = line_chars.next() {
                let advance = if ch == '\t' { 4 } else { 1 };
                let char_w = char_width * advance as f32;

                // Wrap check
                if do_wrap && x_offset + char_w > self.width - padding {
                    current_y += line_height;
                    x_offset = padding;
                    if current_y > self.height {
                        break;
                    }
                }

                if current_y + line_height > 0.0 && current_y < self.height {
                    if !ch.is_control() && ch != ' ' {
                        let text_x = snap_to_pixel(x_offset);
                        let text_y_snapped = snap_to_pixel(current_y + line_height * 0.75);

                        // Check if this character is in the burning selection
                        let is_burning = !char_positions.is_empty()
                            && char_positions.iter().any(|&(cx, cy, _)| {
                                let dx = (cx - (x_offset + char_width * 0.5)).abs();
                                let dy = (cy - (current_y + line_height * 0.5)).abs();
                                dx < char_width && dy < line_height * 0.5
                            });

                        // Apply animated burning color to selected characters
                        let char_paint = if is_burning {
                            self.create_burning_paint(x_offset, current_y)
                        } else {
                            text_paint.clone()
                        };

                        let mut buf = [0u8; 4];
                        let s = ch.encode_utf8(&mut buf);
                        let _ = self.canvas.fill_text(text_x, text_y_snapped, s, &char_paint);
                    }
                }

                x_offset += char_w;
                current_col += 1;

                if line_has_cursor && current_col == cursor_col_idx {
                    cursor_rect = Some((x_offset, current_y));
                }
            }

            // Check if cursor is at end of line (after last character)
            if line_has_cursor && cursor_col_idx == current_col && cursor_rect.is_none() {
                cursor_rect = Some((x_offset, current_y));
            }

            // Move to next line
            current_y += line_height;
        }

        // Handle cursor if it's past the last line (e.g., on empty line after trailing newline)
        if cursor_rect.is_none() && scroll_offset <= cursor_line_idx {
            // cursor_line_idx is absolute; lines.len() is count after skip(scroll_offset)
            // So cursor is past visible lines if cursor_line_idx >= scroll_offset + lines.len()
            if cursor_line_idx >= scroll_offset + lines.len() {
                let visual_line = cursor_line_idx - scroll_offset;
                let cursor_y = start_y + (visual_line as f32 * line_height);
                cursor_rect = Some((padding - scroll_x, cursor_y));
            } else if text.is_empty() {
                cursor_rect = Some((padding - scroll_x, start_y));
            }
        }

        cursor_rect
    }

    fn create_burning_paint(&self, x_offset: f32, current_y: f32) -> Paint {
        // Use character position as random seed for phase offset
        let phase_offset = (x_offset * 0.1 + current_y * 0.07) % std::f32::consts::TAU;
        let time = self.animation_start.elapsed().as_secs_f32() * 2.5;

        // Subtle oscillation - stays reddish-orange
        let cycle = (time + phase_offset).sin() * 0.5 + 0.5; // 0.0 to 1.0

        // Deep Red (0.9, 0.15, 0.0) -> Burning Orange (1.0, 0.4, 0.05)
        let r = 0.9 + cycle * 0.1; // 0.9 to 1.0
        let g = 0.15 + cycle * 0.25; // 0.15 to 0.4
        let b = cycle * 0.05; // 0.0 to 0.05

        let mut burning_paint = Paint::color(Color::rgbf(r, g, b));
        burning_paint.set_font(self.fonts);
        burning_paint.set_font_size(16.0 * self.scale);
        burning_paint
    }

    fn draw_scrollbar(
        &mut self,
        tab: &Tab,
        start_y: f32,
        padding: f32,
        line_height: f32,
        scroll_offset: usize,
        hovered_scrollbar: bool,
        dragging_scrollbar: bool,
    ) {
        let max_visible_lines = ((self.height - start_y - padding) / line_height).ceil() as usize;
        let total_lines = tab.total_lines().max(1);

        if total_lines > max_visible_lines {
            let scrollbar = ScrollbarWidget::new(self.width, self.height, self.scale);
            if let Some(metrics) = scrollbar.metrics(total_lines, max_visible_lines, scroll_offset)
            {
                let mut path = Path::new();
                path.rounded_rect(
                    metrics.thumb.x,
                    metrics.thumb.y,
                    metrics.thumb.width,
                    metrics.thumb.height,
                    4.0,
                );

                let thumb_alpha = if dragging_scrollbar {
                    140
                } else if hovered_scrollbar {
                    90
                } else {
                    50
                };

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

    fn measure_char_width(&self, paint: &Paint) -> f32 {
        if let Ok(metrics) = self.canvas.measure_text(0.0, 0.0, "M", paint) {
            metrics.width()
        } else {
            9.6 * self.scale // Fallback approximate width
        }
    }
}

/// Calculate cursor position in line/column from byte position
pub fn get_cursor_line_col(text: &str, cursor_pos: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;
    let mut pos = 0;

    for ch in text.chars() {
        if pos >= cursor_pos {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
        pos += 1;
    }

    (line, col)
}
