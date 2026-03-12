//! Mouse drag handlers — text selection, scrollbar, tab reorder, window.

use std::time::Duration;

use crate::config::{layout, timing};
use crate::ui::{UiDragAction, UiNode};

use super::super::state::AppResult;
use super::super::ui_state::MouseInteraction;
use super::super::App;

impl App {
    pub fn drag_at(&mut self, x: f32, y: f32) -> AppResult {
        match self.logic.ui_state.mouse_interaction {
            MouseInteraction::None => AppResult::Ok,
            MouseInteraction::WindowDrag | MouseInteraction::WindowResize(_) => AppResult::Ok,
            MouseInteraction::TabDrag { tab_index } => {
                if y < layout::TAB_HEIGHT * self.logic.scale {
                    self.reorder_tab_at(x, y, tab_index)
                } else {
                    AppResult::Ok
                }
            }
            MouseInteraction::ScrollbarDrag { drag_offset } => {
                let total_lines = self.logic.tabs[self.logic.active_tab].total_lines();
                let visible_lines = self.visible_lines();
                let scroll_offset = self.logic.tabs[self.logic.active_tab].scroll_offset();
                let tab_info = self.tab_titles();
                let ui_tree = self.logic.build_ui_tree(&tab_info);
                match ui_tree.drag_scrollbar(y, total_lines, visible_lines, scroll_offset, drag_offset) {
                    UiDragAction::ScrollbarDrag { ratio } => self.jump_scrollbar_to_ratio(ratio),
                    UiDragAction::None => AppResult::Ok,
                }
            }
            MouseInteraction::TextSelection => self.handle_text_selection_drag(x, y),
        }
    }

    pub fn end_drag(&mut self) {
        self.logic.ui_state.mouse_interaction = MouseInteraction::None;
    }

    fn handle_text_selection_drag(&mut self, x: f32, y: f32) -> AppResult {
        let content_start_y = self.content_start_y();
        let height = self.visible_lines() as isize;
        let relative_y = y - content_start_y;
        let mut clicked_visual_line = (relative_y / (layout::LINE_HEIGHT * self.logic.scale)).floor() as isize;

        if clicked_visual_line < 0 || clicked_visual_line >= height {
            if self.logic.ui_state.last_drag_scroll.elapsed()
                < Duration::from_millis(timing::DRAG_SCROLL_THROTTLE_MS)
            {
                return AppResult::Ok;
            }
            self.logic.ui_state.last_drag_scroll = std::time::Instant::now();
            clicked_visual_line = if clicked_visual_line < 0 { -1 } else { height };
        }

        let scroll_offset = self.logic.tabs[self.logic.active_tab].scroll_offset();
        let clicked_line = (scroll_offset as isize + clicked_visual_line).max(0) as usize;
        let char_width = self.renderer.get_char_width();
        let scroll_offset_x = self.logic.tabs[self.logic.active_tab].scroll_offset_x();
        let relative_x = (x - layout::PADDING * self.logic.scale + scroll_offset_x).max(0.0);
        let clicked_visual_col = (relative_x / char_width).round() as usize;
        let clicked_col = self.logic.tabs[self.logic.active_tab].visual_col_to_char_col(clicked_line, clicked_visual_col);

        self.logic.tabs[self.logic.active_tab].set_cursor_position(clicked_line, clicked_col, true);
        self.auto_scroll();
        self.logic.ui_state.reset_cursor_blink();
        AppResult::Redraw
    }

    pub(super) fn reorder_tab_at(&mut self, x: f32, y: f32, from_index: usize) -> AppResult {
        if self.logic.focus.is_renaming() { return AppResult::Ok; }

        let tab_info = self.tab_titles();
        let ui_tree = self.logic.build_ui_tree(&tab_info);

        if let UiNode::Tab(to_index) = ui_tree.hit_test(x, y) {
            if to_index != from_index && from_index < self.logic.tabs.len() && to_index < self.logic.tabs.len() {
                let tab = self.logic.tabs.remove(from_index);
                self.logic.tabs.insert(to_index, tab);

                if self.logic.active_tab == from_index {
                    self.logic.active_tab = to_index;
                } else if from_index < self.logic.active_tab && to_index >= self.logic.active_tab {
                    self.logic.active_tab = self.logic.active_tab.saturating_sub(1);
                } else if from_index > self.logic.active_tab && to_index <= self.logic.active_tab {
                    self.logic.active_tab = (self.logic.active_tab + 1).min(self.logic.tabs.len() - 1);
                }

                self.logic.ui_state.mouse_interaction = MouseInteraction::TabDrag { tab_index: to_index };
                return AppResult::Redraw;
            }
        }
        AppResult::Ok
    }
}
