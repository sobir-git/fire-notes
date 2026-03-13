//! Unified Node tree renderer.
//!
//! `NodeRenderer::draw(node)` walks any `Node` tree and emits femtovg draw calls.
//! This is the only rendering path — no per-subsystem renderers, no RenderFrame.

use std::time::Instant;
use femtovg::{Canvas, Color, FontId, Paint, Path, renderer::OpenGl};

use crate::config::rendering;
use crate::layout::Node;
use crate::layout::node::{
    BoxStyle, Color as NColor, CursorNode, EditorNode,
    ScrollbarNode, SelectionNode, TabBarNode, TextStyle, WinButtonKind,
};
use crate::theme::Theme;
use crate::ui::Rect;

use super::flame::FlameSystem;
use super::fonts::{self, snap_to_pixel};

pub struct NodeRenderer<'a> {
    pub canvas:           &'a mut Canvas<OpenGl>,
    pub fonts:            &'a [FontId],
    pub theme:            &'a Theme,
    pub scale:            f32,
    pub animation_start:  Instant,
    pub flame:            &'a mut FlameSystem,
}

impl<'a> NodeRenderer<'a> {
    /// Walk the node tree and emit draw calls — back to front, depth-first.
    pub fn draw(&mut self, node: &Node) {
        match node {
            Node::Layer(children) => {
                for child in children { self.draw(child); }
            }
            Node::Box { rect, style } => self.draw_box(rect, style),
            Node::Text { text, style, clip } => self.draw_text(text, style, clip.as_ref()),
            Node::Cursor(c) => self.draw_cursor_node(c),
            Node::Selection(s) => self.draw_selection(s),
            Node::Scrollbar(s) => self.draw_scrollbar(s),
            Node::TabBar(tb) => self.draw_tab_bar(tb),
            Node::Editor(ed) => self.draw_editor(ed),
        }
    }

    // ── Primitives ────────────────────────────────────────────────────────────

    fn draw_box(&mut self, rect: &Rect, style: &BoxStyle) {
        let mut path = Path::new();
        if style.radius > 0.0 {
            path.rounded_rect(rect.x, rect.y, rect.width, rect.height, style.radius);
        } else {
            path.rect(rect.x, rect.y, rect.width, rect.height);
        }
        self.canvas.fill_path(&path, &Paint::color(nc(style.fill)));
        if style.border_width > 0.0 {
            let mut bp = Path::new();
            if style.radius > 0.0 {
                bp.rounded_rect(rect.x, rect.y, rect.width, rect.height, style.radius);
            } else {
                bp.rect(rect.x, rect.y, rect.width, rect.height);
            }
            self.canvas.stroke_path(&bp, &Paint::color(nc(style.border)).with_line_width(style.border_width));
        }
    }

    fn draw_text(&mut self, text: &str, style: &TextStyle, clip: Option<&Rect>) {
        if text.is_empty() { return; }
        let mut paint = Paint::color(nc(style.color));
        paint.set_font(self.fonts);
        paint.set_font_size(style.font_size);
        let x = style.clip_x - style.scroll_x;
        if let Some(r) = clip {
            self.canvas.save();
            self.canvas.scissor(r.x, r.y, r.width, r.height);
            let _ = self.canvas.fill_text(x, style.baseline_y, text, &paint);
            self.canvas.restore();
        } else {
            let _ = self.canvas.fill_text(x, style.baseline_y, text, &paint);
        }
    }

    fn draw_cursor_node(&mut self, c: &CursorNode) {
        if !c.visible { return; }
        let mut path = Path::new();
        path.rect(c.rect.x, c.rect.y, c.rect.width, c.rect.height);
        self.canvas.fill_path(&path, &Paint::color(nc(c.color)));
    }

    fn draw_selection(&mut self, s: &SelectionNode) {
        let mut path = Path::new();
        path.rect(s.rect.x, s.rect.y, s.rect.width, s.rect.height);
        self.canvas.fill_path(&path, &Paint::color(nc(s.color)));
    }

    fn draw_scrollbar(&mut self, s: &ScrollbarNode) {
        let mut track = Path::new();
        track.rect(s.track.x, s.track.y, s.track.width, s.track.height);
        self.canvas.fill_path(&track, &Paint::color(nc(s.track_color)));
        let r = s.thumb.width / 2.0;
        let mut thumb = Path::new();
        thumb.rounded_rect(s.thumb.x, s.thumb.y, s.thumb.width, s.thumb.height, r);
        self.canvas.fill_path(&thumb, &Paint::color(nc(s.thumb_color)));
    }

    // ── Tab bar ───────────────────────────────────────────────────────────────

    fn draw_tab_bar(&mut self, tb: &TabBarNode) {
        let r = &tb.rect;
        let tab_font_size = rendering::TAB_FONT_SIZE * self.scale;
        let tabs_clip_w = tb.tabs_clip_x - r.x;

        // Background
        let mut bg = Path::new();
        bg.rect(r.x, r.y, r.width, r.height);
        self.canvas.fill_path(&bg, &Paint::color(Color::rgbf(
            self.theme.tab_inactive.0, self.theme.tab_inactive.1, self.theme.tab_inactive.2,
        )));

        // Tab strip — scissored
        self.canvas.save();
        self.canvas.intersect_scissor(r.x, r.y, tabs_clip_w, r.height);

        for tab in &tb.tabs {
            let tr = &tab.rect;
            let mut path = Path::new();
            path.rect(tr.x, tr.y, tr.width, tr.height);
            let color = if tab.is_active {
                Color::rgbf(self.theme.tab_active.0, self.theme.tab_active.1, self.theme.tab_active.2)
            } else if tab.is_hovered {
                Color::rgbf(self.theme.tab_hover.0, self.theme.tab_hover.1, self.theme.tab_hover.2)
            } else {
                Color::rgbf(self.theme.tab_inactive.0, self.theme.tab_inactive.1, self.theme.tab_inactive.2)
            };
            self.canvas.fill_path(&path, &Paint::color(color));

            if tab.is_active {
                let mut ind = Path::new();
                ind.rect(tr.x, tr.y, tr.width, 2.0 * self.scale);
                self.canvas.fill_path(&ind, &Paint::color(Color::rgbf(
                    self.theme.tab_active_border.0,
                    self.theme.tab_active_border.1,
                    self.theme.tab_active_border.2,
                )));
            }

            let mut text_paint = Paint::color(Color::rgbf(self.theme.fg.0, self.theme.fg.1, self.theme.fg.2));
            text_paint.set_font(self.fonts);
            text_paint.set_font_size(tab_font_size);

            // If rename is active for this tab (matched by position in the tabs slice),
            // draw the rename overlay instead of the static title.
            let tab_slot = tb.tabs.iter().position(|t| std::ptr::eq(t, tab));
            if let (Some(ren), Some(slot)) = (&tb.rename, tab_slot) {
                let is_rename_tab = tb.tabs.iter().position(|t| t.is_active) == Some(slot)
                    && ren.tab_index == slot;
                if is_rename_tab {
                    let text_w = self.canvas.measure_text(0.0, 0.0, &ren.text, &text_paint)
                        .map(|m| m.width()).unwrap_or(ren.text.len() as f32 * rendering::TAB_CHAR_WIDTH_RATIO * self.scale);
                    let text_x = snap_to_pixel(tr.x + (tr.width - text_w) / 2.0);
                    let text_y = ren.text_y;
                    let _ = self.canvas.fill_text(text_x, text_y, &ren.text, &text_paint);
                    self.draw_rename_cursor(ren, &text_paint, text_x, text_y);
                    continue;
                }
            }

            let title = &tab.title;
            let text_w = self.canvas.measure_text(0.0, 0.0, title, &text_paint)
                .map(|m| m.width()).unwrap_or(title.len() as f32 * rendering::TAB_CHAR_WIDTH_RATIO * self.scale);
            let text_x = snap_to_pixel(tr.x + (tr.width - text_w) / 2.0);
            let text_y = snap_to_pixel(r.y + r.height / 2.0 + 5.0 * self.scale);
            let _ = self.canvas.fill_text(text_x, text_y, title, &text_paint);
        }

        self.canvas.restore();

        // New tab button
        self.draw_new_tab_button(&tb.new_tab_rect, tb.new_tab_hovered);

        // Window control buttons
        for btn in &tb.win_buttons {
            let is_close = btn.kind == WinButtonKind::Close;
            let mut path = Path::new();
            path.rounded_rect(btn.rect.x, btn.rect.y, btn.rect.width, btn.rect.height, 4.0 * self.scale);
            let color = if btn.hovered && is_close {
                Color::rgbf(0.9, 0.2, 0.2)
            } else if btn.hovered {
                Color::rgbf(self.theme.button_hover.0, self.theme.button_hover.1, self.theme.button_hover.2)
            } else {
                Color::rgbf(self.theme.button_bg.0, self.theme.button_bg.1, self.theme.button_bg.2)
            };
            self.canvas.fill_path(&path, &Paint::color(color));
            let icon_size = 10.0 * self.scale;
            match btn.kind {
                WinButtonKind::Close    => self.draw_close_icon(&btn.rect, icon_size),
                WinButtonKind::Maximize => self.draw_maximize_icon(&btn.rect, icon_size),
                WinButtonKind::Minimize => self.draw_minimize_icon(&btn.rect, icon_size),
            }
        }

        // Bottom separator line
        let mut line = Path::new();
        line.rect(r.x, r.y + r.height, r.width, 1.0);
        self.canvas.fill_path(&line, &Paint::color(Color::rgbf(
            self.theme.border.0, self.theme.border.1, self.theme.border.2,
        )));
    }

    fn draw_rename_cursor(
        &mut self,
        ren: &crate::layout::node::RenameOverlayNode,
        text_paint: &Paint,
        text_x: f32,
        text_y: f32,
    ) {
        let char_width = fonts::measure_char_width(self.canvas, text_paint, self.scale);
        let cursor_chars = ren.text[..ren.cursor].chars().count();
        if ren.cursor_visible {
            let cx = snap_to_pixel(text_x + cursor_chars as f32 * char_width);
            let cy1 = text_y - 14.0 * self.scale;
            let cy2 = text_y + 4.0 * self.scale;
            let mut path = Path::new();
            path.move_to(cx, cy1);
            path.line_to(cx, cy2);
            let mut paint = Paint::color(Color::rgbf(self.theme.fg.0, self.theme.fg.1, self.theme.fg.2));
            paint.set_line_width(2.0 * self.scale);
            self.canvas.stroke_path(&path, &paint);
        }
        // Underline
        let text_w = self.canvas.measure_text(0.0, 0.0, &ren.text, text_paint)
            .map(|m| m.width()).unwrap_or(ren.text.len() as f32 * 8.0 * self.scale);
        let uy = text_y + 4.0 * self.scale;
        let mut upath = Path::new();
        upath.move_to(text_x, uy);
        upath.line_to(text_x + text_w, uy);
        let mut upaint = Paint::color(Color::rgbf(self.theme.fg.0, self.theme.fg.1, self.theme.fg.2));
        upaint.set_line_width(2.0 * self.scale);
        self.canvas.stroke_path(&upath, &upaint);
    }

    fn draw_new_tab_button(&mut self, r: &Rect, hovered: bool) {
        let mut path = Path::new();
        path.rounded_rect(r.x, r.y, r.width, r.height, 4.0 * self.scale);
        let color = if hovered {
            Color::rgbf(self.theme.button_hover.0, self.theme.button_hover.1, self.theme.button_hover.2)
        } else {
            Color::rgbf(self.theme.button_bg.0, self.theme.button_bg.1, self.theme.button_bg.2)
        };
        self.canvas.fill_path(&path, &Paint::color(color));
        let mut paint = Paint::color(Color::rgbf(self.theme.button_fg.0, self.theme.button_fg.1, self.theme.button_fg.2));
        paint.set_font(self.fonts);
        paint.set_font_size(rendering::NEW_TAB_BUTTON_FONT_SIZE * self.scale);
        let w = self.canvas.measure_text(0.0, 0.0, "+", &paint).map(|m| m.width()).unwrap_or(0.0);
        let tx = snap_to_pixel(r.x + (r.width - w) / 2.0);
        let ty = snap_to_pixel(r.y + r.height / 2.0 + 7.0 * self.scale);
        let _ = self.canvas.fill_text(tx, ty, "+", &paint);
    }

    fn draw_close_icon(&mut self, r: &Rect, icon_size: f32) {
        let cx = r.x + r.width / 2.0; let cy = r.y + r.height / 2.0; let h = icon_size / 2.0;
        let mut path = Path::new();
        path.move_to(cx - h, cy - h); path.line_to(cx + h, cy + h);
        path.move_to(cx + h, cy - h); path.line_to(cx - h, cy + h);
        let mut p = Paint::color(Color::rgbf(self.theme.button_fg.0, self.theme.button_fg.1, self.theme.button_fg.2));
        p.set_line_width(1.5 * self.scale);
        self.canvas.stroke_path(&path, &p);
    }

    fn draw_maximize_icon(&mut self, r: &Rect, icon_size: f32) {
        let cx = r.x + r.width / 2.0; let cy = r.y + r.height / 2.0; let h = icon_size / 2.0;
        let mut path = Path::new(); path.rect(cx - h, cy - h, icon_size, icon_size);
        let mut p = Paint::color(Color::rgbf(self.theme.button_fg.0, self.theme.button_fg.1, self.theme.button_fg.2));
        p.set_line_width(1.5 * self.scale);
        self.canvas.stroke_path(&path, &p);
    }

    fn draw_minimize_icon(&mut self, r: &Rect, icon_size: f32) {
        let cx = r.x + r.width / 2.0; let cy = r.y + r.height / 2.0; let h = icon_size / 2.0;
        let mut path = Path::new(); path.move_to(cx - h, cy); path.line_to(cx + h, cy);
        let mut p = Paint::color(Color::rgbf(self.theme.button_fg.0, self.theme.button_fg.1, self.theme.button_fg.2));
        p.set_line_width(1.5 * self.scale);
        self.canvas.stroke_path(&path, &p);
    }

    // ── Editor ────────────────────────────────────────────────────────────────

    fn draw_editor(&mut self, ed: &EditorNode) {
        let mut text_paint = Paint::color(Color::rgbf(self.theme.fg.0, self.theme.fg.1, self.theme.fg.2));
        text_paint.set_font(self.fonts);
        text_paint.set_font_size(rendering::CONTENT_FONT_SIZE * self.scale);

        // Build flame positions from EditorNode
        let char_positions: Vec<(f32, f32, f32, f32)> = ed.flames.iter()
            .map(|f| (f.cx, f.cy, f.bottom, f.age))
            .collect();

        if !char_positions.is_empty() {
            self.flame.update_legacy(&char_positions, self.scale);
        } else {
            self.flame.clear();
        }

        if !char_positions.is_empty() { self.flame.draw_layer(self.canvas, true); }

        // Draw text lines
        self.draw_editor_lines(ed, &text_paint, &char_positions);

        // Draw cursor
        if ed.cursor.visible {
            let cx = ed.cursor.x;
            let cy = ed.cursor.y;
            let lh = ed.cursor.line_height;
            if cy >= ed.rect.y && cy < ed.rect.y + ed.rect.height {
                let mut path = Path::new();
                path.rect(cx, cy, 2.0 * self.scale, lh);
                self.canvas.fill_path(&path, &Paint::color(Color::rgbf(
                    self.theme.cursor.0, self.theme.cursor.1, self.theme.cursor.2,
                )));
            }
        }

        if !char_positions.is_empty() { self.flame.draw_layer(self.canvas, false); }

        // Draw scrollbar
        if let Some(sb) = &ed.scrollbar {
            self.draw_scrollbar(sb);
        }
    }

    fn draw_editor_lines(
        &mut self,
        ed: &EditorNode,
        text_paint: &Paint,
        char_positions: &[(f32, f32, f32, f32)],
    ) {
        use super::text_content::layout::{FlameHit, build_flame_lookup};

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
                let advance = crate::visual_position::get_char_visual_width(ch);
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
                            p.set_font(self.fonts);
                            p.set_font_size(16.0 * self.scale);
                            let _ = self.canvas.fill_text(text_x, text_y, s, &p);
                        }
                        FlameHit::Selection => {
                            let paint = self.burning_paint(x_offset, current_y);
                            let _ = self.canvas.fill_text(text_x, text_y, s, &paint);
                        }
                        FlameHit::None => {
                            let _ = self.canvas.fill_text(text_x, text_y, s, text_paint);
                        }
                    }
                }
                x_offset += char_w;
            }
        }
    }

    fn burning_paint(&self, x: f32, y: f32) -> Paint {
        let phase = (x * 0.1 + y * 0.07) % std::f32::consts::TAU;
        let time  = self.animation_start.elapsed().as_secs_f32() * 2.5;
        let cycle = ((time + phase).sin() * 0.5 + 0.5) as f32;
        let mut p = Paint::color(Color::rgbf(0.9 + cycle * 0.1, 0.15 + cycle * 0.25, cycle * 0.05));
        p.set_font(self.fonts);
        p.set_font_size(rendering::CONTENT_FONT_SIZE * self.scale);
        p
    }
}

fn nc(c: NColor) -> Color { Color::rgbaf(c.r, c.g, c.b, c.a) }
