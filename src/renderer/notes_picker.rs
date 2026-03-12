//! Notes picker overlay rendering.
//!
//! All geometry comes from `NotesPicker` — this renderer contains no inline pixel math.

use crate::app::NoteEntry;
use crate::theme::Theme;
use crate::ui::{ListWidget, NotesPicker, TextInput, MAX_VISIBLE_ITEMS};
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
        width: f32,
        height: f32,
        scale: f32,
    ) -> Self {
        let _ = (width, height);
        Self { canvas, fonts, theme, scale }
    }

    pub fn draw(
        &mut self,
        input: &TextInput,
        list: &ListWidget<NoteEntry>,
        cursor_visible: bool,
        layout: &NotesPicker,
    ) {
        let scale = layout.scale;
        let font_size = layout.font_size;

        // ── Backdrop ──────────────────────────────────────────────────────
        let mut backdrop = Path::new();
        let b = &layout.backdrop_rect;
        backdrop.rect(b.x, b.y, b.width, b.height);
        self.canvas.fill_path(&backdrop, &Paint::color(Color::rgba(0, 0, 0, 120)));

        // ── Overlay panel ─────────────────────────────────────────────────
        let o = &layout.overlay_rect;
        let mut bg = Path::new();
        bg.rounded_rect(o.x, o.y, o.width, o.height, 8.0 * scale);
        self.canvas.fill_path(
            &bg,
            &Paint::color(Color::rgbf(
                self.theme.tab_inactive.0,
                self.theme.tab_inactive.1,
                self.theme.tab_inactive.2,
            )),
        );
        let mut border = Path::new();
        border.rounded_rect(o.x, o.y, o.width, o.height, 8.0 * scale);
        self.canvas.stroke_path(
            &border,
            &Paint::color(Color::rgbf(
                self.theme.tab_active_border.0,
                self.theme.tab_active_border.1,
                self.theme.tab_active_border.2,
            )).with_line_width(2.0),
        );

        // ── Search input ──────────────────────────────────────────────────
        let ir = &layout.input_rect;
        let mut input_bg = Path::new();
        input_bg.rounded_rect(ir.x, ir.y, ir.width, ir.height, 4.0 * scale);
        self.canvas.fill_path(
            &input_bg,
            &Paint::color(Color::rgbf(self.theme.bg.0, self.theme.bg.1, self.theme.bg.2)),
        );

        let mut text_paint = Paint::color(Color::rgbf(self.theme.fg.0, self.theme.fg.1, self.theme.fg.2));
        text_paint.set_font(self.fonts);
        text_paint.set_font_size(font_size);

        let text_x = layout.input_text_x;
        let text_y = layout.input_text_baseline_y;

        if input.text().is_empty() {
            let mut ph = Paint::color(Color::rgba(150, 150, 150, 180));
            ph.set_font(self.fonts);
            ph.set_font_size(font_size);
            let _ = self.canvas.fill_text(text_x, text_y, "Search notes...", &ph);
        } else {
            let _ = self.canvas.fill_text(text_x, text_y, input.text(), &text_paint);
        }

        // ── Cursor ────────────────────────────────────────────────────────
        if cursor_visible {
            let cursor_char_idx = input.text()[..input.cursor()].chars().count();
            let char_width = self.measure_char_width(&text_paint);
            let cursor_x = text_x + cursor_char_idx as f32 * char_width;
            let mut cursor_path = Path::new();
            cursor_path.rect(cursor_x, ir.y + 4.0 * scale, 2.0, ir.height - 8.0 * scale);
            self.canvas.fill_path(
                &cursor_path,
                &Paint::color(Color::rgbf(
                    self.theme.tab_active_border.0,
                    self.theme.tab_active_border.1,
                    self.theme.tab_active_border.2,
                )),
            );
        }

        // ── List items ────────────────────────────────────────────────────
        let scroll_offset = list.scroll_offset();
        let selected_index = list.selected_index();

        for (display_idx, filtered_idx) in list
            .filtered_indices()
            .iter()
            .skip(scroll_offset)
            .take(MAX_VISIBLE_ITEMS)
            .enumerate()
        {
            let is_selected = scroll_offset + display_idx == selected_index;
            let m = layout.item_metrics(display_idx);

            if is_selected {
                let mut highlight = Path::new();
                highlight.rounded_rect(
                    m.row_rect.x, m.row_rect.y, m.row_rect.width, m.row_rect.height,
                    4.0 * scale,
                );
                self.canvas.fill_path(
                    &highlight,
                    &Paint::color(Color::rgbf(
                        self.theme.tab_active_border.0 * 0.3,
                        self.theme.tab_active_border.1 * 0.3,
                        self.theme.tab_active_border.2 * 0.3,
                    )),
                );
            }

            if let Some(note) = list.items().get(*filtered_idx) {
                let title_color = if is_selected {
                    Color::rgbf(self.theme.fg.0, self.theme.fg.1, self.theme.fg.2)
                } else {
                    Color::rgba(200, 200, 200, 220)
                };
                let mut title_paint = Paint::color(title_color);
                title_paint.set_font(self.fonts);
                title_paint.set_font_size(font_size);
                let _ = self.canvas.fill_text(text_x, m.text_baseline_y, &note.title, &title_paint);

                if note.is_open {
                    let mut ind_paint = Paint::color(Color::rgbf(
                        self.theme.tab_active_border.0,
                        self.theme.tab_active_border.1,
                        self.theme.tab_active_border.2,
                    ));
                    ind_paint.set_font(self.fonts);
                    ind_paint.set_font_size(font_size * 0.8);
                    let _ = self.canvas.fill_text(m.indicator_x, m.text_baseline_y, "●", &ind_paint);
                }
            }
        }

        // ── No-results message ────────────────────────────────────────────
        if list.is_empty() && !input.text().is_empty() {
            let m = layout.item_metrics(0);
            let mut no_results = Paint::color(Color::rgba(150, 150, 150, 180));
            no_results.set_font(self.fonts);
            no_results.set_font_size(font_size);
            let _ = self.canvas.fill_text(text_x, m.text_baseline_y, "No matching notes", &no_results);
        }
    }

    fn measure_char_width(&mut self, paint: &Paint) -> f32 {
        fonts::measure_char_width(self.canvas, paint, self.scale)
    }
}
