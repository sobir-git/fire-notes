//! Notes picker overlay rendering — pure data, no widget references.

use crate::render_frame::NotesPickerFrameData;
use crate::theme::Theme;
use femtovg::{Canvas, Color, Paint, Path, FontId, renderer::OpenGl};

use super::fonts;

pub struct NotesPickerRenderer<'a> {
    canvas: &'a mut Canvas<OpenGl>,
    fonts: &'a [FontId],
    theme: &'a Theme,
    scale: f32,
}

impl<'a> NotesPickerRenderer<'a> {
    pub fn new(
        canvas: &'a mut Canvas<OpenGl>,
        fonts: &'a [FontId],
        theme: &'a Theme,
        scale: f32,
    ) -> Self {
        Self { canvas, fonts, theme, scale }
    }

    pub fn draw(&mut self, picker: &NotesPickerFrameData) {
        let scale     = picker.scale;
        let font_size = picker.font_size;

        // ── Backdrop ──────────────────────────────────────────────────────
        let b = picker.backdrop_rect;
        let mut path = Path::new();
        path.rect(b.x, b.y, b.width, b.height);
        self.canvas.fill_path(&path, &Paint::color(Color::rgba(0, 0, 0, 120)));

        // ── Overlay panel ─────────────────────────────────────────────────
        let o = picker.overlay_rect;
        let mut bg = Path::new();
        bg.rounded_rect(o.x, o.y, o.width, o.height, 8.0 * scale);
        self.canvas.fill_path(&bg, &Paint::color(Color::rgbf(
            self.theme.tab_inactive.0, self.theme.tab_inactive.1, self.theme.tab_inactive.2,
        )));
        let mut border = Path::new();
        border.rounded_rect(o.x, o.y, o.width, o.height, 8.0 * scale);
        self.canvas.stroke_path(&border,
            &Paint::color(Color::rgbf(
                self.theme.tab_active_border.0,
                self.theme.tab_active_border.1,
                self.theme.tab_active_border.2,
            )).with_line_width(2.0),
        );

        // ── Search input box ──────────────────────────────────────────────
        let ir = picker.input_rect;
        let mut input_bg = Path::new();
        input_bg.rounded_rect(ir.x, ir.y, ir.width, ir.height, 4.0 * scale);
        self.canvas.fill_path(&input_bg, &Paint::color(Color::rgbf(
            self.theme.bg.0, self.theme.bg.1, self.theme.bg.2,
        )));

        let mut text_paint = Paint::color(Color::rgbf(
            self.theme.fg.0, self.theme.fg.1, self.theme.fg.2,
        ));
        text_paint.set_font(self.fonts);
        text_paint.set_font_size(font_size);

        let text_x = picker.input_text_x;
        let text_y = picker.input_text_baseline_y;
        let char_width = self.measure_char_width(&text_paint);

        if picker.query.is_empty() {
            let mut ph = Paint::color(Color::rgba(150, 150, 150, 180));
            ph.set_font(self.fonts);
            ph.set_font_size(font_size);
            let _ = self.canvas.fill_text(text_x, text_y, "Search notes...", &ph);
        } else {
            // ── Selection highlight ──────────────────────────────────────
            if let Some((start, end)) = picker.input_selection {
                let sel_x = text_x - picker.input_scroll_offset + start as f32 * char_width;
                let sel_w = (end - start) as f32 * char_width;
                let mut sel_path = Path::new();
                sel_path.rect(sel_x, ir.y + 2.0, sel_w, ir.height - 4.0);
                self.canvas.fill_path(&sel_path, &Paint::color(Color::rgba(100, 140, 210, 120)));
            }
            let _ = self.canvas.fill_text(
                text_x - picker.input_scroll_offset,
                text_y,
                &picker.query,
                &text_paint,
            );
        }

        // ── Input cursor ──────────────────────────────────────────────────
        if picker.cursor_visible {
            let cur_x = text_x - picker.input_scroll_offset
                + picker.input_cursor as f32 * char_width;
            let mut cursor_path = Path::new();
            cursor_path.rect(cur_x, ir.y + 2.0, 2.0 * scale, ir.height - 4.0);
            self.canvas.fill_path(&cursor_path, &Paint::color(Color::rgbf(
                self.theme.tab_active_border.0,
                self.theme.tab_active_border.1,
                self.theme.tab_active_border.2,
            )));
        }

        // ── List rows ─────────────────────────────────────────────────────
        for row in &picker.rows {
            if row.is_selected {
                let r = row.row_rect;
                let mut hl = Path::new();
                hl.rounded_rect(r.x, r.y, r.width, r.height, 4.0 * scale);
                self.canvas.fill_path(&hl, &Paint::color(Color::rgbf(
                    self.theme.tab_active_border.0 * 0.3,
                    self.theme.tab_active_border.1 * 0.3,
                    self.theme.tab_active_border.2 * 0.3,
                )));
            }

            let title_color = if row.is_selected {
                Color::rgbf(self.theme.fg.0, self.theme.fg.1, self.theme.fg.2)
            } else {
                Color::rgba(200, 200, 200, 220)
            };
            let mut title_paint = Paint::color(title_color);
            title_paint.set_font(self.fonts);
            title_paint.set_font_size(font_size);
            let _ = self.canvas.fill_text(text_x, row.baseline_y, &row.title, &title_paint);

            if row.is_open {
                let mut ind_paint = Paint::color(Color::rgbf(
                    self.theme.tab_active_border.0,
                    self.theme.tab_active_border.1,
                    self.theme.tab_active_border.2,
                ));
                ind_paint.set_font(self.fonts);
                ind_paint.set_font_size(font_size * 0.8);
                let _ = self.canvas.fill_text(picker.indicator_x, row.center_y, "●", &ind_paint);
            }
        }

        // ── Scrollbar ─────────────────────────────────────────────────────
        if let Some((track, thumb)) = picker.scrollbar {
            let mut track_path = Path::new();
            track_path.rect(track.x, track.y, track.width, track.height);
            self.canvas.fill_path(&track_path, &Paint::color(Color::rgba(60, 60, 60, 120)));
            let mut thumb_path = Path::new();
            thumb_path.rounded_rect(thumb.x, thumb.y, thumb.width, thumb.height, thumb.width / 2.0);
            self.canvas.fill_path(&thumb_path, &Paint::color(Color::rgbf(
                self.theme.tab_active_border.0 * 0.6,
                self.theme.tab_active_border.1 * 0.6,
                self.theme.tab_active_border.2 * 0.6,
            )));
        }

        // ── No-results message ────────────────────────────────────────────
        if picker.no_results {
            let mut no_results = Paint::color(Color::rgba(150, 150, 150, 180));
            no_results.set_font(self.fonts);
            no_results.set_font_size(font_size);
            let _ = self.canvas.fill_text(
                text_x, picker.first_row_baseline_y, "No matching notes", &no_results,
            );
        }
    }

    fn measure_char_width(&mut self, paint: &Paint) -> f32 {
        fonts::measure_char_width(self.canvas, paint, self.scale)
    }
}
