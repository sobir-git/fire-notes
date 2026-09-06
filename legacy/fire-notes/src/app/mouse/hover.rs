//! Mouse hover detection.

use super::super::state::AppResult;
use super::super::App;

impl App {
    pub fn handle_mouse_move(&mut self, x: f32, y: f32) -> AppResult {
        self.logic.last_mouse_x = x;
        self.logic.last_mouse_y = y;

        let prev_resize_edge = self.logic.hovered_resize_edge;

        let (changed, cursor_shape, resize_edge) = self.logic.handle_hover(x, y);

        self.logic.cursor_shape = cursor_shape;
        self.logic.hovered_resize_edge = resize_edge;

        if changed || prev_resize_edge != resize_edge {
            AppResult::Redraw
        } else {
            AppResult::Ok
        }
    }
}
