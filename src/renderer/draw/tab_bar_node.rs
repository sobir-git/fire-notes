//! Tab bar node rendering — tabs, rename overlay, window buttons, new-tab btn.

use femtovg::{Color, Paint, Path};
use crate::config::rendering;
use crate::layout::node::{RenameOverlayNode, TabBarNode, WinButtonKind};
use super::super::draw_ctx::DrawCtx;
use super::super::fonts::{self, snap_to_pixel};

pub fn draw(tb: &TabBarNode, ctx: &mut DrawCtx<'_>) {
    let r = &tb.rect;
    let tab_font_size = rendering::TAB_FONT_SIZE * ctx.scale;
    let tabs_clip_w = tb.tabs_clip_x - r.x;

    // Background
    let mut bg = Path::new();
    bg.rect(r.x, r.y, r.width, r.height);
    ctx.canvas.fill_path(&bg, &Paint::color(Color::rgbf(
        ctx.theme.tab_inactive.0, ctx.theme.tab_inactive.1, ctx.theme.tab_inactive.2,
    )));

    // Tab strip — scissored to keep tabs inside bounds
    ctx.canvas.save();
    ctx.canvas.intersect_scissor(r.x, r.y, tabs_clip_w, r.height);

    for (slot, tab) in tb.tabs.iter().enumerate() {
        let tr = &tab.rect;
        let mut path = Path::new();
        path.rect(tr.x, tr.y, tr.width, tr.height);
        let color = if tab.is_active {
            Color::rgbf(ctx.theme.tab_active.0, ctx.theme.tab_active.1, ctx.theme.tab_active.2)
        } else if tab.is_hovered {
            Color::rgbf(ctx.theme.tab_hover.0, ctx.theme.tab_hover.1, ctx.theme.tab_hover.2)
        } else {
            Color::rgbf(ctx.theme.tab_inactive.0, ctx.theme.tab_inactive.1, ctx.theme.tab_inactive.2)
        };
        ctx.canvas.fill_path(&path, &Paint::color(color));

        if tab.is_active {
            let mut ind = Path::new();
            ind.rect(tr.x, tr.y, tr.width, 2.0 * ctx.scale);
            ctx.canvas.fill_path(&ind, &Paint::color(Color::rgbf(
                ctx.theme.tab_active_border.0,
                ctx.theme.tab_active_border.1,
                ctx.theme.tab_active_border.2,
            )));
        }

        let mut text_paint = Paint::color(Color::rgbf(
            ctx.theme.fg.0, ctx.theme.fg.1, ctx.theme.fg.2,
        ));
        text_paint.set_font(ctx.fonts);
        text_paint.set_font_size(tab_font_size);

        // Rename overlay replaces the static title for the active rename tab
        if let Some(ren) = &tb.rename {
            let active_slot = tb.tabs.iter().position(|t| t.is_active);
            if ren.tab_index == slot && active_slot == Some(slot) {
                draw_rename(ren, ctx, &text_paint, tr.x, tr.y, tr.width);
                continue;
            }
        }

        let title = &tab.title;
        let text_w = ctx.canvas.measure_text(0.0, 0.0, title, &text_paint)
            .map(|m| m.width())
            .unwrap_or(title.len() as f32 * rendering::TAB_CHAR_WIDTH_RATIO * ctx.scale);
        let text_x = snap_to_pixel(tr.x + (tr.width - text_w) / 2.0);
        let text_y = snap_to_pixel(r.y + r.height / 2.0 + 5.0 * ctx.scale);
        let _ = ctx.canvas.fill_text(text_x, text_y, title, &text_paint);
    }

    ctx.canvas.restore();

    draw_new_tab_button(tb, ctx);

    for btn in &tb.win_buttons {
        draw_win_button(btn, ctx);
    }

    // Bottom separator
    let mut line = Path::new();
    line.rect(r.x, r.y + r.height, r.width, 1.0);
    ctx.canvas.fill_path(&line, &Paint::color(Color::rgbf(
        ctx.theme.border.0, ctx.theme.border.1, ctx.theme.border.2,
    )));
}

// ── Rename overlay ────────────────────────────────────────────────────────────

fn draw_rename(
    ren: &RenameOverlayNode,
    ctx: &mut DrawCtx<'_>,
    text_paint: &Paint,
    tab_x: f32,
    tab_y: f32,
    tab_w: f32,
) {
    let text_w = ctx.canvas.measure_text(0.0, 0.0, &ren.text, text_paint)
        .map(|m| m.width())
        .unwrap_or(ren.text.len() as f32 * rendering::TAB_CHAR_WIDTH_RATIO * ctx.scale);
    let text_x = snap_to_pixel(tab_x + (tab_w - text_w) / 2.0);
    let text_y = ren.text_y;
    let _ = ctx.canvas.fill_text(text_x, text_y, &ren.text, text_paint);

    // Cursor
    if ren.cursor_visible {
        let char_w = fonts::measure_char_width(ctx.canvas, text_paint, ctx.scale);
        let cursor_chars = ren.text[..ren.cursor].chars().count();
        let cx = snap_to_pixel(text_x + cursor_chars as f32 * char_w);
        let cy1 = text_y - 14.0 * ctx.scale;
        let cy2 = text_y + 4.0 * ctx.scale;
        let mut path = Path::new();
        path.move_to(cx, cy1);
        path.line_to(cx, cy2);
        let mut paint = Paint::color(Color::rgbf(
            ctx.theme.fg.0, ctx.theme.fg.1, ctx.theme.fg.2,
        ));
        paint.set_line_width(2.0 * ctx.scale);
        ctx.canvas.stroke_path(&path, &paint);
    }

    // Underline
    let uy = text_y + 4.0 * ctx.scale;
    let mut upath = Path::new();
    upath.move_to(tab_x + (tab_w - text_w) / 2.0, uy);
    upath.line_to(tab_x + (tab_w + text_w) / 2.0, uy);
    let mut upaint = Paint::color(Color::rgbf(
        ctx.theme.fg.0, ctx.theme.fg.1, ctx.theme.fg.2,
    ));
    upaint.set_line_width(2.0 * ctx.scale);
    ctx.canvas.stroke_path(&upath, &upaint);

    let _ = tab_y; // suppress unused warning
}

// ── New-tab button ────────────────────────────────────────────────────────────

fn draw_new_tab_button(tb: &TabBarNode, ctx: &mut DrawCtx<'_>) {
    let r = &tb.new_tab_rect;
    let mut path = Path::new();
    path.rounded_rect(r.x, r.y, r.width, r.height, 4.0 * ctx.scale);
    let color = if tb.new_tab_hovered {
        Color::rgbf(ctx.theme.button_hover.0, ctx.theme.button_hover.1, ctx.theme.button_hover.2)
    } else {
        Color::rgbf(ctx.theme.button_bg.0, ctx.theme.button_bg.1, ctx.theme.button_bg.2)
    };
    ctx.canvas.fill_path(&path, &Paint::color(color));

    let mut paint = Paint::color(Color::rgbf(
        ctx.theme.button_fg.0, ctx.theme.button_fg.1, ctx.theme.button_fg.2,
    ));
    paint.set_font(ctx.fonts);
    paint.set_font_size(rendering::NEW_TAB_BUTTON_FONT_SIZE * ctx.scale);
    let w = ctx.canvas.measure_text(0.0, 0.0, "+", &paint)
        .map(|m| m.width()).unwrap_or(0.0);
    let tx = snap_to_pixel(r.x + (r.width - w) / 2.0);
    let ty = snap_to_pixel(r.y + r.height / 2.0 + 7.0 * ctx.scale);
    let _ = ctx.canvas.fill_text(tx, ty, "+", &paint);
}

// ── Window control buttons ────────────────────────────────────────────────────

fn draw_win_button(btn: &crate::layout::node::WinButton, ctx: &mut DrawCtx<'_>) {
    let r = &btn.rect;
    let is_close = btn.kind == WinButtonKind::Close;
    let mut path = Path::new();
    path.rounded_rect(r.x, r.y, r.width, r.height, 4.0 * ctx.scale);
    let color = if btn.hovered && is_close {
        Color::rgbf(0.9, 0.2, 0.2)
    } else if btn.hovered {
        Color::rgbf(ctx.theme.button_hover.0, ctx.theme.button_hover.1, ctx.theme.button_hover.2)
    } else {
        Color::rgbf(ctx.theme.button_bg.0, ctx.theme.button_bg.1, ctx.theme.button_bg.2)
    };
    ctx.canvas.fill_path(&path, &Paint::color(color));

    let icon_size = 10.0 * ctx.scale;
    match btn.kind {
        WinButtonKind::Close    => draw_close_icon(r, ctx, icon_size),
        WinButtonKind::Maximize => draw_maximize_icon(r, ctx, icon_size),
        WinButtonKind::Minimize => draw_minimize_icon(r, ctx, icon_size),
    }
}

fn draw_close_icon(r: &crate::ui::Rect, ctx: &mut DrawCtx<'_>, icon_size: f32) {
    let cx = r.x + r.width / 2.0; let cy = r.y + r.height / 2.0; let h = icon_size / 2.0;
    let mut path = Path::new();
    path.move_to(cx - h, cy - h); path.line_to(cx + h, cy + h);
    path.move_to(cx + h, cy - h); path.line_to(cx - h, cy + h);
    let mut p = Paint::color(Color::rgbf(
        ctx.theme.button_fg.0, ctx.theme.button_fg.1, ctx.theme.button_fg.2,
    ));
    p.set_line_width(1.5 * ctx.scale);
    ctx.canvas.stroke_path(&path, &p);
}

fn draw_maximize_icon(r: &crate::ui::Rect, ctx: &mut DrawCtx<'_>, icon_size: f32) {
    let cx = r.x + r.width / 2.0; let cy = r.y + r.height / 2.0; let h = icon_size / 2.0;
    let mut path = Path::new();
    path.rect(cx - h, cy - h, icon_size, icon_size);
    let mut p = Paint::color(Color::rgbf(
        ctx.theme.button_fg.0, ctx.theme.button_fg.1, ctx.theme.button_fg.2,
    ));
    p.set_line_width(1.5 * ctx.scale);
    ctx.canvas.stroke_path(&path, &p);
}

fn draw_minimize_icon(r: &crate::ui::Rect, ctx: &mut DrawCtx<'_>, icon_size: f32) {
    let cx = r.x + r.width / 2.0; let cy = r.y + r.height / 2.0; let h = icon_size / 2.0;
    let mut path = Path::new();
    path.move_to(cx - h, cy); path.line_to(cx + h, cy);
    let mut p = Paint::color(Color::rgbf(
        ctx.theme.button_fg.0, ctx.theme.button_fg.1, ctx.theme.button_fg.2,
    ));
    p.set_line_width(1.5 * ctx.scale);
    ctx.canvas.stroke_path(&path, &p);
}
