//! Window control buttons — minimize, maximize, close.

use femtovg::{Color, Paint, Path};

use super::draw::TabBarRenderer;

impl<'a> TabBarRenderer<'a> {
    pub(super) fn draw_window_controls(
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

        let close_x = self.width - button_size - button_margin;
        self.draw_window_button(close_x, button_y, button_size, hovered_close, true);
        self.draw_close_icon(close_x, button_y, button_size, icon_size);

        let maximize_x = close_x - button_size - 4.0 * self.scale;
        self.draw_window_button(maximize_x, button_y, button_size, hovered_maximize, false);
        self.draw_maximize_icon(maximize_x, button_y, button_size, icon_size);

        let minimize_x = maximize_x - button_size - 4.0 * self.scale;
        self.draw_window_button(minimize_x, button_y, button_size, hovered_minimize, false);
        self.draw_minimize_icon(minimize_x, button_y, button_size, icon_size);
    }

    fn draw_window_button(&mut self, x: f32, y: f32, size: f32, hovered: bool, is_close: bool) {
        let mut btn_path = Path::new();
        btn_path.rounded_rect(x, y, size, size, 4.0 * self.scale);
        let btn_color = if hovered {
            if is_close {
                Color::rgbf(0.9, 0.2, 0.2)
            } else {
                Color::rgbf(self.theme.button_hover.0, self.theme.button_hover.1, self.theme.button_hover.2)
            }
        } else {
            Color::rgbf(self.theme.button_bg.0, self.theme.button_bg.1, self.theme.button_bg.2)
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
        let mut paint = Paint::color(Color::rgbf(self.theme.button_fg.0, self.theme.button_fg.1, self.theme.button_fg.2));
        paint.set_line_width(1.5 * self.scale);
        self.canvas.stroke_path(&path, &paint);
    }

    fn draw_maximize_icon(&mut self, btn_x: f32, btn_y: f32, btn_size: f32, icon_size: f32) {
        let center_x = btn_x + btn_size / 2.0;
        let center_y = btn_y + btn_size / 2.0;
        let half = icon_size / 2.0;
        let mut path = Path::new();
        path.rect(center_x - half, center_y - half, icon_size, icon_size);
        let mut paint = Paint::color(Color::rgbf(self.theme.button_fg.0, self.theme.button_fg.1, self.theme.button_fg.2));
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
        let mut paint = Paint::color(Color::rgbf(self.theme.button_fg.0, self.theme.button_fg.1, self.theme.button_fg.2));
        paint.set_line_width(1.5 * self.scale);
        self.canvas.stroke_path(&path, &paint);
    }
}
