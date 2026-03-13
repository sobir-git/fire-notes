//! Text line and cursor drawing.

use femtovg::{Canvas, Color, FontId, Paint, Path, renderer::OpenGl};

use crate::config::rendering;
use crate::render_frame::RenderFrame;
use crate::theme::Theme;

/// All per-frame context needed to draw text content.
/// Passed by reference to every draw function — replaces long decomposed arg lists.
pub struct DrawCtx<'a> {
    pub frame:      &'a RenderFrame,
    pub char_width: f32,
}

use super::layout::{FlameHit, build_flame_lookup};
use super::super::fonts::{self, snap_to_pixel};

fn line_y(ctx: &DrawCtx<'_>, visual_line: usize) -> f32 {
    ctx.frame.content.text_rect.y
        + ctx.frame.content.doc_top_margin
        + visual_line as f32 * ctx.frame.content.line_height
}

pub fn draw_text_lines(
    canvas: &mut Canvas<OpenGl>,
    fonts: &[FontId],
    scale: f32,
    animation_start: std::time::Instant,
    ctx: &DrawCtx<'_>,
    text_paint: &Paint,
    char_positions: &[(f32, f32, f32, f32)],
) {
    let line_height = ctx.frame.content.line_height;
    let padding     = ctx.frame.content.text_padding;
    let scroll_x    = ctx.frame.scroll_offset_x;
    let do_wrap       = ctx.frame.word_wrap;
    let flame_lookup = build_flame_lookup(char_positions, ctx.char_width, line_height);
    let cell_w = ctx.char_width.max(1.0);
    let cell_h = line_height.max(1.0);
    let mut current_y = line_y(ctx, 0);

    let bottom = ctx.frame.content.rect.y + ctx.frame.content.rect.height;
    let right  = ctx.frame.content.rect.x + ctx.frame.content.rect.width;
    for line in ctx.frame.visible_lines.iter().map(|l| l.text.as_str()).chain(
        // If visible_lines doesn't cover all content (shouldn't happen), skip gracefully.
        std::iter::empty()
    ) {
        if current_y > bottom { break; }
        let mut x_offset = if do_wrap { padding } else { padding - scroll_x };

        for ch in line.chars() {
            let advance = crate::visual_position::get_char_visual_width(ch);
            let char_w = ctx.char_width * advance as f32;

            if do_wrap && x_offset + char_w > right - padding {
                current_y += line_height;
                x_offset = padding;
                if current_y > bottom { break; }
            }

            if current_y + line_height > 0.0 && current_y < bottom
                && !ch.is_control() && ch != ' ' {
                    let text_x = snap_to_pixel(x_offset);
                    let text_y = snap_to_pixel(current_y + line_height * 0.75);
                    let cx = x_offset + ctx.char_width * 0.5;
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

pub fn calculate_cursor_position(ctx: &DrawCtx<'_>) -> Option<(f32, f32)> {
    let frame         = ctx.frame;
    let scroll_offset = frame.scroll_offset;
    let scroll_x      = frame.scroll_offset_x;
    let cursor_line   = frame.cursor.line;
    let cursor_col    = frame.cursor.col;
    if cursor_line < scroll_offset { return None; }
    let visual_line = cursor_line - scroll_offset;
    let y = line_y(ctx, visual_line);
    if y > frame.content.rect.y + frame.content.rect.height { return None; }
    let line_content = frame.visible_lines.iter()
        .find(|l| l.line_index == cursor_line)
        .map(|l| l.text.as_str())
        .unwrap_or("");
    let vl = crate::visual_position::VisualLine::new(line_content);
    let x = vl.char_col_to_visual_x(cursor_col, frame.content.text_padding - scroll_x, ctx.char_width);
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

pub fn collect_selection_positions(ctx: &DrawCtx<'_>) -> Vec<(f32, f32, f32, f32)> {
    let frame         = ctx.frame;
    let line_height   = frame.content.line_height;
    let scroll_offset = frame.scroll_offset;
    let scroll_x      = frame.scroll_offset_x;
    let mut positions = Vec::new();
    if frame.word_wrap { return positions; }
    let Some(sel) = &frame.selection else { return positions; };
    let (start_line, start_col) = sel.start;
    let (end_line, end_col)     = sel.end;
    let bottom = frame.content.rect.y + frame.content.rect.height;
    let right  = frame.content.rect.x + frame.content.rect.width;
    for ld in frame.visible_lines.iter() {
        let line_idx = ld.line_index;
        if line_idx < start_line || line_idx > end_line { continue; }
        let visible_idx = line_idx.saturating_sub(scroll_offset);
        let y = line_y(ctx, visible_idx);
        if y > bottom { break; }
        let line_bottom_y = y + line_height;
        let sc = if line_idx == start_line { start_col } else { 0 };
        let ec = if line_idx == end_line {
            end_col.min(ld.text.chars().count())
        } else {
            ld.text.chars().count()
        };
        let vl = crate::visual_position::VisualLine::new(&ld.text);
        for col in sc..ec {
            let char_x = vl.char_col_to_visual_center_x(col, frame.content.text_padding - scroll_x, ctx.char_width);
            if char_x < -ctx.char_width || char_x > right + ctx.char_width { continue; }
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
