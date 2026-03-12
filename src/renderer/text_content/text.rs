//! Text line and cursor drawing.

use femtovg::{Canvas, Color, FontId, Paint, Path, renderer::OpenGl};

use crate::config::rendering;
use crate::tab::Tab;
use crate::theme::Theme;
use crate::ui::TextArea;

use super::layout::{FlameHit, build_flame_lookup, get_cursor_line_col};
use super::super::fonts::{self, snap_to_pixel};

pub fn draw_text_lines(
    canvas: &mut Canvas<OpenGl>,
    fonts: &[FontId],
    _theme: &Theme,
    width: f32,
    height: f32,
    scale: f32,
    animation_start: std::time::Instant,
    text: &str,
    scroll_offset: usize,
    scroll_x: f32,
    do_wrap: bool,
    text_area: &TextArea,
    char_width: f32,
    text_paint: &Paint,
    char_positions: &[(f32, f32, f32, f32)],
) {
    let line_height = text_area.line_height;
    let padding     = text_area.text_padding;
    let flame_lookup = build_flame_lookup(char_positions, char_width, line_height);
    let cell_w = char_width.max(1.0);
    let cell_h = line_height.max(1.0);
    let mut current_y = text_area.line_y(0);

    for line in text.lines().skip(scroll_offset) {
        if current_y > height { break; }
        let mut x_offset = if do_wrap { padding } else { padding - scroll_x };

        for ch in line.chars() {
            let advance = crate::visual_position::get_char_visual_width(ch);
            let char_w = char_width * advance as f32;

            if do_wrap && x_offset + char_w > width - padding {
                current_y += line_height;
                x_offset = padding;
                if current_y > height { break; }
            }

            if current_y + line_height > 0.0 && current_y < height
                && !ch.is_control() && ch != ' ' {
                    let text_x = snap_to_pixel(x_offset);
                    let text_y = snap_to_pixel(current_y + line_height * 0.75);
                    let cx = x_offset + char_width * 0.5;
                    let cy = current_y + line_height * 0.5;
                    let grid_x = (cx / cell_w) as i32;
                    let grid_y = (cy / cell_h) as i32;
                    let hit = flame_lookup.get(&(grid_x, grid_y)).copied().unwrap_or(FlameHit::None);

                    let mut buf = [0u8; 4];
                    let s = ch.encode_utf8(&mut buf);

                    match hit {
                        FlameHit::Typing(age) => {
                            let fade = age;
                            let r = 1.0;
                            let g = 0.3 * (1.0 - fade) + 1.0 * fade;
                            let b = 0.0 * (1.0 - fade) + 1.0 * fade;
                            let mut paint = Paint::color(Color::rgbf(r, g, b));
                            paint.set_font(fonts);
                            paint.set_font_size(16.0 * scale);
                            let _ = canvas.fill_text(text_x, text_y, s, &paint);
                        }
                        FlameHit::Selection => {
                            let paint = create_burning_paint(fonts, scale, animation_start, x_offset, current_y);
                            let _ = canvas.fill_text(text_x, text_y, s, &paint);
                        }
                        FlameHit::None => {
                            let _ = canvas.fill_text(text_x, text_y, s, text_paint);
                        }
                    }
            }
            x_offset += char_w;
        }
        current_y += line_height;
    }
}

pub fn calculate_cursor_position(
    text: &str,
    cursor_pos: usize,
    scroll_offset: usize,
    scroll_x: f32,
    text_area: &TextArea,
    height: f32,
    char_width: f32,
) -> Option<(f32, f32)> {
    let (cursor_line, cursor_col) = get_cursor_line_col(text, cursor_pos);
    if cursor_line < scroll_offset { return None; }
    let visual_line = cursor_line - scroll_offset;
    let y = text_area.line_y(visual_line);
    if y > height { return None; }
    let line_content = text.lines().nth(cursor_line).unwrap_or("");
    let vl = crate::visual_position::VisualLine::new(line_content);
    let x = vl.char_col_to_visual_x(cursor_col, text_area.text_padding - scroll_x, char_width);
    Some((x, y))
}

pub fn draw_cursor(
    canvas: &mut Canvas<OpenGl>,
    theme: &Theme,
    scale: f32,
    cx: f32,
    cy: f32,
    line_height: f32,
) {
    let mut path = Path::new();
    path.rect(cx, cy, 2.0 * scale, line_height);
    canvas.fill_path(
        &path,
        &Paint::color(Color::rgbf(theme.cursor.0, theme.cursor.1, theme.cursor.2)),
    );
}

pub fn collect_selection_positions(
    canvas_width: f32,
    canvas_height: f32,
    tab: &Tab,
    text: &str,
    scroll_offset: usize,
    scroll_x: f32,
    text_area: &TextArea,
    char_width: f32,
) -> Vec<(f32, f32, f32, f32)> {
    let line_height = text_area.line_height;
    let padding     = text_area.text_padding;
    let mut positions = Vec::new();
    if tab.word_wrap() { return positions; }
    let Some(((start_line, start_col), (end_line, end_col))) = tab.selection_range_line_col() else {
        return positions;
    };
    let visible_start = scroll_offset.max(start_line);
    for (line_idx, line_content) in text.lines()
        .enumerate()
        .skip(visible_start)
        .take_while(|(idx, _)| *idx <= end_line)
    {
        let visible_idx = line_idx.saturating_sub(scroll_offset);
        let y = text_area.line_y(visible_idx);
        if y > canvas_height { break; }
        let line_bottom_y = y + line_height;
        let sc = if line_idx == start_line { start_col } else { 0 };
        let ec = if line_idx == end_line {
            end_col.min(line_content.chars().count())
        } else {
            line_content.chars().count()
        };
        let vl = crate::visual_position::VisualLine::new(line_content);
        for col in sc..ec {
            let char_x = vl.char_col_to_visual_center_x(col, padding - scroll_x, char_width);
            if char_x < -char_width || char_x > canvas_width + char_width { continue; }
            positions.push((char_x, y + line_height * 0.5, line_bottom_y, 0.0));
        }
    }
    positions
}

pub fn measure_char_width(canvas: &mut Canvas<OpenGl>, paint: &Paint, scale: f32) -> f32 {
    fonts::measure_char_width(canvas, paint, scale)
}

fn create_burning_paint(
    fonts: &[FontId],
    scale: f32,
    animation_start: std::time::Instant,
    x_offset: f32,
    current_y: f32,
) -> Paint {
    let phase = (x_offset * 0.1 + current_y * 0.07) % std::f32::consts::TAU;
    let time = animation_start.elapsed().as_secs_f32() * 2.5;
    let cycle = (time + phase).sin() * 0.5 + 0.5;
    let r = 0.9 + cycle * 0.1;
    let g = 0.15 + cycle * 0.25;
    let b = cycle * 0.05;
    let mut paint = Paint::color(Color::rgbf(r, g, b));
    paint.set_font(fonts);
    paint.set_font_size(rendering::CONTENT_FONT_SIZE * scale);
    paint
}
