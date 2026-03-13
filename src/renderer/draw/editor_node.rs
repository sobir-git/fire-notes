//! Editor node rendering — text lines, cursor, selection, flame effect, scrollbar.

use femtovg::{Color, Paint, Path};
use crate::config::rendering;
use crate::layout::node::EditorNode;
use crate::visual_position;
use super::super::draw_ctx::DrawCtx;
use super::super::fonts::snap_to_pixel;
use super::super::text_content::layout::{FlameHit, build_flame_lookup};
use super::scrollbar_node;

pub fn draw(ed: &EditorNode, ctx: &mut DrawCtx<'_>) {
    let char_positions: Vec<(f32, f32, f32, f32)> = ed.flames.iter()
        .map(|f| (f.cx, f.cy, f.bottom, f.age))
        .collect();

    if !char_positions.is_empty() {
        ctx.flame.update_legacy(&char_positions, ctx.scale);
    } else {
        ctx.flame.clear();
    }

    // Flame layer behind text
    if !char_positions.is_empty() {
        ctx.flame.draw_layer(ctx.canvas, true);
    }

    draw_selection(ed, ctx);
    draw_lines(ed, &char_positions, ctx);
    draw_cursor(ed, ctx);

    // Flame layer in front of text
    if !char_positions.is_empty() {
        ctx.flame.draw_layer(ctx.canvas, false);
    }

    if let Some(sb) = &ed.scrollbar {
        scrollbar_node::draw(sb, ctx);
    }
}

// ── Selection highlight ───────────────────────────────────────────────────────

fn draw_selection(ed: &EditorNode, ctx: &mut DrawCtx<'_>) {
    if let Some(sel) = &ed.selection {
        // Selection blue — matches the overlay accent color in the theme.
        let paint = Paint::color(Color::rgbaf(0.39, 0.55, 0.82, 0.35));
        for rect in &sel.rects {
            let mut path = Path::new();
            path.rect(rect.x, rect.y, rect.width, rect.height);
            ctx.canvas.fill_path(&path, &paint);
        }
    }
}

// ── Text lines ────────────────────────────────────────────────────────────────

fn draw_lines(
    ed: &EditorNode,
    char_positions: &[(f32, f32, f32, f32)],
    ctx: &mut DrawCtx<'_>,
) {
    let mut text_paint = Paint::color(Color::rgbf(
        ctx.theme.fg.0, ctx.theme.fg.1, ctx.theme.fg.2,
    ));
    text_paint.set_font(ctx.fonts);
    text_paint.set_font_size(rendering::CONTENT_FONT_SIZE * ctx.scale);

    let line_height  = ed.line_height;
    let char_width   = ed.char_width;
    let scroll_x     = ed.scroll_x;
    let padding      = ed.text_padding;
    let do_wrap      = ed.word_wrap;
    let bottom       = ed.rect.y + ed.rect.height;
    let right        = ed.rect.x + ed.rect.width;
    let flame_lookup = build_flame_lookup(char_positions, char_width, line_height);
    let cell_w = char_width.max(1.0);
    let cell_h = line_height.max(1.0);

    for line in &ed.lines {
        let mut current_y = line.y;
        let mut x_offset  = if do_wrap { padding } else { padding - scroll_x };

        for ch in line.text.chars() {
            let advance = visual_position::get_char_visual_width(ch);
            let char_w  = char_width * advance as f32;

            if do_wrap && x_offset + char_w > right - padding {
                current_y += line_height;
                x_offset   = padding;
            }
            if current_y > bottom { break; }
            if !ch.is_control() && ch != ' ' {
                let text_x = snap_to_pixel(x_offset);
                let text_y = snap_to_pixel(current_y + line_height * 0.75);
                let grid_x = ((x_offset + char_width * 0.5) / cell_w) as i32;
                let grid_y = ((current_y + line_height * 0.5) / cell_h) as i32;
                let hit = flame_lookup.get(&(grid_x, grid_y)).copied().unwrap_or(FlameHit::None);
                let mut buf = [0u8; 4];
                let s = ch.encode_utf8(&mut buf);
                match hit {
                    FlameHit::Typing(age) => {
                        let g = 0.3 * (1.0 - age) + 1.0 * age;
                        let b = 1.0 * age;
                        let mut p = Paint::color(Color::rgbf(1.0, g, b));
                        p.set_font(ctx.fonts);
                        p.set_font_size(16.0 * ctx.scale);
                        let _ = ctx.canvas.fill_text(text_x, text_y, s, &p);
                    }
                    FlameHit::Selection => {
                        let paint = burning_paint(x_offset, current_y, ctx);
                        let _ = ctx.canvas.fill_text(text_x, text_y, s, &paint);
                    }
                    FlameHit::None => {
                        let _ = ctx.canvas.fill_text(text_x, text_y, s, &text_paint);
                    }
                }
            }
            x_offset += char_w;
        }
    }
}

fn burning_paint(x: f32, y: f32, ctx: &DrawCtx<'_>) -> Paint {
    let phase = (x * 0.1 + y * 0.07) % std::f32::consts::TAU;
    let time  = ctx.animation_start.elapsed().as_secs_f32() * 2.5;
    let cycle = ((time + phase).sin() * 0.5 + 0.5) as f32;
    let mut p = Paint::color(Color::rgbf(
        0.9 + cycle * 0.1, 0.15 + cycle * 0.25, cycle * 0.05,
    ));
    p.set_font(ctx.fonts);
    p.set_font_size(rendering::CONTENT_FONT_SIZE * ctx.scale);
    p
}

// ── Editor cursor ─────────────────────────────────────────────────────────────

fn draw_cursor(ed: &EditorNode, ctx: &mut DrawCtx<'_>) {
    if !ed.cursor.visible { return; }
    let cx = ed.cursor.x;
    let cy = ed.cursor.y;
    let lh = ed.cursor.line_height;
    if cy >= ed.rect.y && cy < ed.rect.y + ed.rect.height {
        let mut path = Path::new();
        path.rect(cx, cy, 2.0 * ctx.scale, lh);
        ctx.canvas.fill_path(&path, &Paint::color(Color::rgbf(
            ctx.theme.cursor.0, ctx.theme.cursor.1, ctx.theme.cursor.2,
        )));
    }
}
