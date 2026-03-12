//! Mouse hover detection.

use super::super::state::AppResult;
use super::super::App;

impl App {
    pub fn handle_mouse_move(&mut self, x: f32, y: f32) -> AppResult {
        self.logic.ui_state.last_mouse_x = x;
        self.logic.ui_state.last_mouse_y = y;

        let tab_info = self.tab_titles();

        let prev_hovered_tab_index = self.logic.ui_state.hovered_tab_index;
        let prev_hovered_plus = self.logic.ui_state.hovered_plus;
        let prev_hovered_scrollbar = self.logic.ui_state.hovered_scrollbar;
        let prev_hovered_minimize = self.logic.ui_state.hovered_window_minimize;
        let prev_hovered_maximize = self.logic.ui_state.hovered_window_maximize;
        let prev_hovered_close = self.logic.ui_state.hovered_window_close;
        let prev_hovered_resize_edge = self.logic.ui_state.hovered_resize_edge;

        let total_lines = self.logic.tabs[self.logic.active_tab].total_lines();
        let visible_lines = self.visible_lines();
        let scroll_offset = self.logic.tabs[self.logic.active_tab].scroll_offset();
        let ui_tree = self.logic.build_ui_tree(&tab_info);
        let hover = ui_tree.hover(x, y, total_lines, visible_lines, scroll_offset);

        self.logic.ui_state.hovered_tab_index = hover.tab_index;
        self.logic.ui_state.hovered_plus = hover.plus;
        self.logic.ui_state.hovered_scrollbar = hover.scrollbar;
        self.logic.ui_state.hovered_window_minimize = hover.window_minimize;
        self.logic.ui_state.hovered_window_maximize = hover.window_maximize;
        self.logic.ui_state.hovered_window_close = hover.window_close;
        self.logic.ui_state.hovered_resize_edge = hover.resize_edge;
        self.logic.ui_state.cursor_shape = hover.cursor_shape;

        if prev_hovered_tab_index != self.logic.ui_state.hovered_tab_index
            || prev_hovered_plus != self.logic.ui_state.hovered_plus
            || prev_hovered_scrollbar != self.logic.ui_state.hovered_scrollbar
            || prev_hovered_minimize != self.logic.ui_state.hovered_window_minimize
            || prev_hovered_maximize != self.logic.ui_state.hovered_window_maximize
            || prev_hovered_close != self.logic.ui_state.hovered_window_close
            || prev_hovered_resize_edge != self.logic.ui_state.hovered_resize_edge
        {
            AppResult::Redraw
        } else {
            AppResult::Ok
        }
    }
}
