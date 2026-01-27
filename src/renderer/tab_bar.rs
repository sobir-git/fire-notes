//! Tab bar rendering

use crate::theme::Theme;
use crate::ui::TextInput;
use femtovg::{Canvas, Color, FontId, Paint, Path, renderer::OpenGl};

/// Snap a coordinate to the pixel grid to prevent blurry text rendering.
#[inline]
fn snap_to_pixel(coord: f32) -> f32 {
    coord.round()
}

pub struct TabBarRenderer<'a> {
    canvas: &'a mut Canvas<OpenGl>,
    fonts: &'a [FontId],
    theme: &'a Theme,
    width: f32,
    scale: f32,
    tab_scroll_x: f32,
}

impl<'a> TabBarRenderer<'a> {
    pub fn new(
        canvas: &'a mut Canvas<OpenGl>,
        fonts: &'a [FontId],
        theme: &'a Theme,
        width: f32,
        scale: f32,
        tab_scroll_x: f32,
    ) -> Self {
        Self {
            canvas,
            fonts,
            theme,
            width,
            scale,
            tab_scroll_x,
        }
    }

    pub fn draw(
        &mut self,
        tabs: &[(&str, bool)],
        hovered_tab_index: Option<usize>,
        hovered_plus: bool,
        renaming_tab: Option<usize>,
        rename_input: Option<&TextInput>,
        cursor_visible: bool,
        hovered_minimize: bool,
        hovered_maximize: bool,
        hovered_close: bool,
    ) {
        let tab_height = 40.0 * self.scale;
        let tab_padding = 16.0 * self.scale;

        // Save state for clipping
        self.canvas.save();

        // Clip to tab bar area to prevent drawing outside when scrolling
        self.canvas
            .intersect_scissor(0.0, 0.0, self.width, tab_height);

        let mut x = -self.tab_scroll_x;

        for (i, (title, is_active)) in tabs.iter().enumerate() {
            let tab_width =
                (title.len() as f32 * 9.0 * self.scale + tab_padding * 2.0).max(100.0 * self.scale);

            // Optimization: skip drawing off-screen tabs
            if x + tab_width < 0.0 {
                x += tab_width + 1.0;
                continue;
            }

            // Tab background
            let mut path = Path::new();
            path.rect(x, 0.0, tab_width, tab_height);

            let color = if *is_active {
                Color::rgbf(
                    self.theme.tab_active.0,
                    self.theme.tab_active.1,
                    self.theme.tab_active.2,
                )
            } else if Some(i) == hovered_tab_index {
                Color::rgbf(
                    self.theme.tab_hover.0,
                    self.theme.tab_hover.1,
                    self.theme.tab_hover.2,
                )
            } else {
                Color::rgbf(
                    self.theme.tab_inactive.0,
                    self.theme.tab_inactive.1,
                    self.theme.tab_inactive.2,
                )
            };

            self.canvas.fill_path(&path, &Paint::color(color));

            // Active tab indicator (top line)
            if *is_active {
                let mut indicator = Path::new();
                indicator.rect(x, 0.0, tab_width, 2.0 * self.scale);
                self.canvas.fill_path(
                    &indicator,
                    &Paint::color(Color::rgbf(
                        self.theme.tab_active_border.0,
                        self.theme.tab_active_border.1,
                        self.theme.tab_active_border.2,
                    )),
                );
            }

            // Tab title - center it properly within the tab
            let mut text_paint = Paint::color(Color::rgbf(
                self.theme.fg.0,
                self.theme.fg.1,
                self.theme.fg.2,
            ));
            text_paint.set_font(self.fonts);
            text_paint.set_font_size(14.0 * self.scale);

            // Measure text to center it properly
            let text_width = if let Ok(metrics) = self.canvas.measure_text(0.0, 0.0, title, &text_paint) {
                metrics.width()
            } else {
                title.len() as f32 * 9.0 * self.scale // fallback
            };

            let text_x = snap_to_pixel(x + (tab_width - text_width) / 2.0);
            let text_y = snap_to_pixel(tab_height / 2.0 + 5.0 * self.scale);
            let _ = self.canvas.fill_text(text_x, text_y, title, &text_paint);

            // Draw text input cursor and selection if this tab is being renamed
            if Some(i) == renaming_tab {
                if let Some(input) = rename_input {
                    // Draw selection highlight if any
                    if let Some((sel_start, sel_end)) = input.selection_range() {
                        let sel_start_chars = input.text()[..sel_start].chars().count();
                        let sel_end_chars = input.text()[..sel_end].chars().count();
                        
                        // Measure character width
                        let char_width = if let Ok(m) = self.canvas.measure_text(0.0, 0.0, "M", &text_paint) {
                            m.width()
                        } else {
                            9.0 * self.scale
                        };
                        
                        let sel_x = text_x + sel_start_chars as f32 * char_width;
                        let sel_width = (sel_end_chars - sel_start_chars) as f32 * char_width;
                        
                        let mut sel_path = Path::new();
                        sel_path.rect(sel_x, text_y - 14.0 * self.scale, sel_width, 18.0 * self.scale);
                        self.canvas.fill_path(
                            &sel_path,
                            &Paint::color(Color::rgba(100, 150, 255, 100)),
                        );
                    }
                    
                    // Draw cursor if visible
                    if cursor_visible {
                        let cursor_chars = input.text()[..input.cursor()].chars().count();
                        let char_width = if let Ok(m) = self.canvas.measure_text(0.0, 0.0, "M", &text_paint) {
                            m.width()
                        } else {
                            9.0 * self.scale
                        };
                        
                        let cursor_x = snap_to_pixel(text_x + cursor_chars as f32 * char_width);
                        let cursor_y1 = text_y - 14.0 * self.scale;
                        let cursor_y2 = text_y + 4.0 * self.scale;
                        
                        let mut cursor_path = Path::new();
                        cursor_path.move_to(cursor_x, cursor_y1);
                        cursor_path.line_to(cursor_x, cursor_y2);
                        
                        let mut cursor_paint = Paint::color(Color::rgbf(
                            self.theme.fg.0,
                            self.theme.fg.1,
                            self.theme.fg.2,
                        ));
                        cursor_paint.set_line_width(2.0 * self.scale);
                        self.canvas.stroke_path(&cursor_path, &cursor_paint);
                    }
                }
                
                // Draw underline
                let underline_y = text_y + 4.0 * self.scale;
                let mut underline_path = Path::new();
                underline_path.move_to(text_x, underline_y);
                underline_path.line_to(text_x + text_width, underline_y);
                let mut underline_paint = Paint::color(Color::rgbf(
                    self.theme.fg.0,
                    self.theme.fg.1,
                    self.theme.fg.2,
                ));
                underline_paint.set_line_width(2.0 * self.scale);
                self.canvas.stroke_path(&underline_path, &underline_paint);
            }

            x += tab_width + 1.0;
        }

        // New Tab (+) button
        self.draw_new_tab_button(x, tab_height, hovered_plus);

        // Restore state (clear clipping)
        self.canvas.restore();

        // Window control buttons (drawn after restore so they're not clipped)
        self.draw_window_controls(tab_height, hovered_minimize, hovered_maximize, hovered_close);

        // Tab bar bottom line
        self.draw_bottom_line(tab_height);
    }

    fn draw_new_tab_button(&mut self, x: f32, tab_height: f32, hovered: bool) {
        let new_tab_button_size = 28.0 * self.scale;
        let button_x = x + 8.0 * self.scale;
        let button_y = (tab_height - new_tab_button_size) / 2.0;

        let mut btn_path = Path::new();
        btn_path.rounded_rect(
            button_x,
            button_y,
            new_tab_button_size,
            new_tab_button_size,
            4.0 * self.scale,
        );

        let btn_color = if hovered {
            Color::rgbf(
                self.theme.button_hover.0,
                self.theme.button_hover.1,
                self.theme.button_hover.2,
            )
        } else {
            Color::rgbf(
                self.theme.button_bg.0,
                self.theme.button_bg.1,
                self.theme.button_bg.2,
            )
        };
        self.canvas.fill_path(&btn_path, &Paint::color(btn_color));

        // Draw + symbol
        let mut plus_paint = Paint::color(Color::rgbf(
            self.theme.button_fg.0,
            self.theme.button_fg.1,
            self.theme.button_fg.2,
        ));
        plus_paint.set_font(self.fonts);
        plus_paint.set_font_size(20.0 * self.scale);

        // Measure to center perfectly
        let mut plus_width = 0.0;
        if let Ok(metrics) = self.canvas.measure_text(0.0, 0.0, "+", &plus_paint) {
            plus_width = metrics.width();
        }

        let plus_x = snap_to_pixel(button_x + (new_tab_button_size - plus_width) / 2.0);
        let plus_y = snap_to_pixel(button_y + new_tab_button_size / 2.0 + 7.0 * self.scale);
        let _ = self.canvas.fill_text(plus_x, plus_y, "+", &plus_paint);
    }

    fn draw_window_controls(
        &mut self,
        tab_height: f32,
        hovered_minimize: bool,
        hovered_maximize: bool,
        hovered_close: bool,
    ) {
        let button_size = 28.0 * self.scale;
        let button_margin = 8.0 * self.scale;
        let button_y = (tab_height - button_size) / 2.0;
        let icon_size = 10.0 * self.scale;

        // Close button (rightmost)
        let close_x = self.width - button_size - button_margin;
        self.draw_window_button(
            close_x,
            button_y,
            button_size,
            hovered_close,
            true, // is_close
        );
        // Draw X icon
        self.draw_close_icon(close_x, button_y, button_size, icon_size);

        // Maximize button
        let maximize_x = close_x - button_size - 4.0 * self.scale;
        self.draw_window_button(maximize_x, button_y, button_size, hovered_maximize, false);
        // Draw square icon
        self.draw_maximize_icon(maximize_x, button_y, button_size, icon_size);

        // Minimize button
        let minimize_x = maximize_x - button_size - 4.0 * self.scale;
        self.draw_window_button(minimize_x, button_y, button_size, hovered_minimize, false);
        // Draw minus icon
        self.draw_minimize_icon(minimize_x, button_y, button_size, icon_size);
    }

    fn draw_window_button(
        &mut self,
        x: f32,
        y: f32,
        size: f32,
        hovered: bool,
        is_close: bool,
    ) {
        let mut btn_path = Path::new();
        btn_path.rounded_rect(x, y, size, size, 4.0 * self.scale);

        let btn_color = if hovered {
            if is_close {
                Color::rgbf(0.9, 0.2, 0.2) // Red for close button hover
            } else {
                Color::rgbf(
                    self.theme.button_hover.0,
                    self.theme.button_hover.1,
                    self.theme.button_hover.2,
                )
            }
        } else {
            Color::rgbf(
                self.theme.button_bg.0,
                self.theme.button_bg.1,
                self.theme.button_bg.2,
            )
        };
        self.canvas.fill_path(&btn_path, &Paint::color(btn_color));
    }

    fn draw_close_icon(&mut self, btn_x: f32, btn_y: f32, btn_size: f32, icon_size: f32) {
        let center_x = btn_x + btn_size / 2.0;
        let center_y = btn_y + btn_size / 2.0;
        let half = icon_size / 2.0;

        let mut path = Path::new();
        path.move_to(center_x - half, center_y - half);
        path.line_to(center_x + half, center_y + half);
        path.move_to(center_x + half, center_y - half);
        path.line_to(center_x - half, center_y + half);

        let mut paint = Paint::color(Color::rgbf(
            self.theme.button_fg.0,
            self.theme.button_fg.1,
            self.theme.button_fg.2,
        ));
        paint.set_line_width(1.5 * self.scale);
        self.canvas.stroke_path(&path, &paint);
    }

    fn draw_maximize_icon(&mut self, btn_x: f32, btn_y: f32, btn_size: f32, icon_size: f32) {
        let center_x = btn_x + btn_size / 2.0;
        let center_y = btn_y + btn_size / 2.0;
        let half = icon_size / 2.0;

        let mut path = Path::new();
        path.rect(center_x - half, center_y - half, icon_size, icon_size);

        let mut paint = Paint::color(Color::rgbf(
            self.theme.button_fg.0,
            self.theme.button_fg.1,
            self.theme.button_fg.2,
        ));
        paint.set_line_width(1.5 * self.scale);
        self.canvas.stroke_path(&path, &paint);
    }

    fn draw_minimize_icon(&mut self, btn_x: f32, btn_y: f32, btn_size: f32, icon_size: f32) {
        let center_x = btn_x + btn_size / 2.0;
        let center_y = btn_y + btn_size / 2.0;
        let half = icon_size / 2.0;

        let mut path = Path::new();
        path.move_to(center_x - half, center_y);
        path.line_to(center_x + half, center_y);

        let mut paint = Paint::color(Color::rgbf(
            self.theme.button_fg.0,
            self.theme.button_fg.1,
            self.theme.button_fg.2,
        ));
        paint.set_line_width(1.5 * self.scale);
        self.canvas.stroke_path(&path, &paint);
    }

    fn draw_bottom_line(&mut self, tab_height: f32) {
        let mut line = Path::new();
        line.rect(0.0, tab_height, self.width, 1.0);
        self.canvas.fill_path(
            &line,
            &Paint::color(Color::rgbf(
                self.theme.border.0,
                self.theme.border.1,
                self.theme.border.2,
            )),
        );
    }
}
