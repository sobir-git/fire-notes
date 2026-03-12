//! Tab drawing — individual tabs, rename input, new-tab button, bottom line.

use femtovg::{Canvas, Color, FontId, Paint, Path, renderer::OpenGl};

use crate::config::rendering;
use crate::theme::Theme;
use crate::ui::{TabBar, TextInput};

/// Per-frame interaction state for the tab bar.
/// Groups all hover/rename/cursor fields so `draw()` stays under the arg limit.
pub struct TabBarInteraction<'a> {
    pub hovered_tab_index: Option<usize>,
    pub hovered_plus:      bool,
    pub renaming_tab:      Option<usize>,
    pub rename_input:      Option<&'a TextInput>,
    pub cursor_visible:    bool,
    pub hovered_minimize:  bool,
    pub hovered_maximize:  bool,
    pub hovered_close:     bool,
}

use super::super::fonts::{self, snap_to_pixel};

pub struct TabBarRenderer<'a> {
    pub(super) canvas: &'a mut Canvas<OpenGl>,
    pub(super) fonts: &'a [FontId],
    pub(super) theme: &'a Theme,
    pub(super) scale: f32,
}

impl<'a> TabBarRenderer<'a> {
    pub fn new(
        canvas: &'a mut Canvas<OpenGl>,
        fonts: &'a [FontId],
        theme: &'a Theme,
        scale: f32,
    ) -> Self {
        Self { canvas, fonts, theme, scale }
    }

    /// Draw the entire tab bar using the pre-computed `TabBar` layout widget.
    ///
    /// All coordinates come from the widget — no raw pixel math here.
    pub fn draw(
        &mut self,
        layout: &TabBar,
        tabs: &[(&str, bool)],
        ix: &TabBarInteraction<'_>,
    ) {
        let r = &layout.rect;
        // Clip scrolling tabs to the drag-gap boundary (left of drag zone + controls).
        let tabs_clip_width = layout.tabs_clip_x - r.x;

        self.canvas.save();
        self.canvas.intersect_scissor(r.x, r.y, tabs_clip_width, r.height);

        for tab in &layout.scroll_area.tabs {
            let (title, is_active) = tabs[tab.index];
            let r = &tab.rect;

            let mut path = Path::new();
            path.rect(r.x, r.y, r.width, r.height);
            let color = if is_active {
                Color::rgbf(self.theme.tab_active.0, self.theme.tab_active.1, self.theme.tab_active.2)
            } else if Some(tab.index) == ix.hovered_tab_index {
                Color::rgbf(self.theme.tab_hover.0, self.theme.tab_hover.1, self.theme.tab_hover.2)
            } else {
                Color::rgbf(self.theme.tab_inactive.0, self.theme.tab_inactive.1, self.theme.tab_inactive.2)
            };
            self.canvas.fill_path(&path, &Paint::color(color));

            if is_active {
                let mut indicator = Path::new();
                indicator.rect(r.x, r.y, r.width, 2.0 * self.scale);
                self.canvas.fill_path(
                    &indicator,
                    &Paint::color(Color::rgbf(
                        self.theme.tab_active_border.0,
                        self.theme.tab_active_border.1,
                        self.theme.tab_active_border.2,
                    )),
                );
            }

            let mut text_paint = Paint::color(Color::rgbf(self.theme.fg.0, self.theme.fg.1, self.theme.fg.2));
            text_paint.set_font(self.fonts);
            text_paint.set_font_size(rendering::TAB_FONT_SIZE * self.scale);

            let text_width = if let Ok(metrics) = self.canvas.measure_text(0.0, 0.0, title, &text_paint) {
                metrics.width()
            } else {
                title.len() as f32 * rendering::TAB_CHAR_WIDTH_RATIO * self.scale
            };
            let text_x = snap_to_pixel(r.x + (r.width - text_width) / 2.0);
            let text_y = snap_to_pixel(layout.rect.y + layout.rect.height / 2.0 + 5.0 * self.scale);
            let _ = self.canvas.fill_text(text_x, text_y, title, &text_paint);

            if Some(tab.index) == ix.renaming_tab {
                self.draw_rename_overlay(ix.rename_input, ix.cursor_visible, &text_paint, text_x, text_y);
            }
        }

        self.canvas.restore();
        // Plus button is pinned at a fixed position — always draw it.
        self.draw_new_tab_button(layout, ix.hovered_plus);
        self.draw_window_controls(layout, ix.hovered_minimize, ix.hovered_maximize, ix.hovered_close);
        self.draw_bottom_line(layout);
    }

    fn draw_rename_overlay(
        &mut self,
        rename_input: Option<&TextInput>,
        cursor_visible: bool,
        text_paint: &Paint,
        text_x: f32,
        text_y: f32,
    ) {
        if let Some(input) = rename_input {
            if let Some((sel_start, sel_end)) = input.selection_range() {
                let sel_start_chars = input.text()[..sel_start].chars().count();
                let sel_end_chars = input.text()[..sel_end].chars().count();
                let char_width = fonts::measure_char_width(self.canvas, text_paint, self.scale);
                let sel_x = text_x + sel_start_chars as f32 * char_width;
                let sel_width = (sel_end_chars - sel_start_chars) as f32 * char_width;
                let mut sel_path = Path::new();
                sel_path.rect(sel_x, text_y - 14.0 * self.scale, sel_width, 18.0 * self.scale);
                self.canvas.fill_path(&sel_path, &Paint::color(Color::rgba(100, 150, 255, 100)));
            }

            if cursor_visible {
                let cursor_chars = input.text()[..input.cursor()].chars().count();
                let char_width = fonts::measure_char_width(self.canvas, text_paint, self.scale);
                let cursor_x = snap_to_pixel(text_x + cursor_chars as f32 * char_width);
                let cursor_y1 = text_y - 14.0 * self.scale;
                let cursor_y2 = text_y + 4.0 * self.scale;
                let mut cursor_path = Path::new();
                cursor_path.move_to(cursor_x, cursor_y1);
                cursor_path.line_to(cursor_x, cursor_y2);
                let mut cursor_paint = Paint::color(Color::rgbf(self.theme.fg.0, self.theme.fg.1, self.theme.fg.2));
                cursor_paint.set_line_width(2.0 * self.scale);
                self.canvas.stroke_path(&cursor_path, &cursor_paint);
            }

            let text_width = if let Ok(m) = self.canvas.measure_text(0.0, 0.0, input.text(), text_paint) {
                m.width()
            } else {
                input.text().len() as f32 * rendering::TAB_CHAR_WIDTH_RATIO * self.scale
            };
            let underline_y = text_y + 4.0 * self.scale;
            let mut underline_path = Path::new();
            underline_path.move_to(text_x, underline_y);
            underline_path.line_to(text_x + text_width, underline_y);
            let mut underline_paint = Paint::color(Color::rgbf(self.theme.fg.0, self.theme.fg.1, self.theme.fg.2));
            underline_paint.set_line_width(2.0 * self.scale);
            self.canvas.stroke_path(&underline_path, &underline_paint);
        }
    }

    fn draw_new_tab_button(&mut self, layout: &TabBar, hovered: bool) {
        let r = &layout.new_tab_rect;
        let mut btn_path = Path::new();
        btn_path.rounded_rect(r.x, r.y, r.width, r.height, 4.0 * self.scale);
        let btn_color = if hovered {
            Color::rgbf(self.theme.button_hover.0, self.theme.button_hover.1, self.theme.button_hover.2)
        } else {
            Color::rgbf(self.theme.button_bg.0, self.theme.button_bg.1, self.theme.button_bg.2)
        };
        self.canvas.fill_path(&btn_path, &Paint::color(btn_color));

        let mut plus_paint = Paint::color(Color::rgbf(self.theme.button_fg.0, self.theme.button_fg.1, self.theme.button_fg.2));
        plus_paint.set_font(self.fonts);
        plus_paint.set_font_size(rendering::NEW_TAB_BUTTON_FONT_SIZE * self.scale);
        let plus_width = if let Ok(metrics) = self.canvas.measure_text(0.0, 0.0, "+", &plus_paint) {
            metrics.width()
        } else {
            0.0
        };
        let plus_x = snap_to_pixel(r.x + (r.width - plus_width) / 2.0);
        let plus_y = snap_to_pixel(r.y + r.height / 2.0 + 7.0 * self.scale);
        let _ = self.canvas.fill_text(plus_x, plus_y, "+", &plus_paint);
    }

    pub(super) fn draw_bottom_line(&mut self, layout: &TabBar) {
        let r = &layout.rect;
        let mut line = Path::new();
        line.rect(r.x, r.y + r.height, r.width, 1.0);
        self.canvas.fill_path(
            &line,
            &Paint::color(Color::rgbf(self.theme.border.0, self.theme.border.1, self.theme.border.2)),
        );
    }
}
