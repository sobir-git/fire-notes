//! Mouse click handlers — single, double, triple, right-click.

use std::time::Duration;

use crate::config::{layout, timing};
use crate::ui::{UiAction, UiNode};

use super::super::state::AppResult;
use super::super::ui_state::MouseInteraction;
use super::super::App;

impl App {
    pub fn click_at(&mut self, x: f32, y: f32, selecting: bool) -> AppResult {
        if self.logic.focus.is_notes_picker() {
            return self.handle_notes_picker_click(x, y);
        }

        let tab_info = self.tab_titles();
        let total_lines = self.logic.tabs[self.logic.active_tab].total_lines();
        let visible_lines = self.visible_lines();
        let scroll_offset = self.logic.tabs[self.logic.active_tab].scroll_offset();
        let ui_tree = self.logic.build_ui_tree(&tab_info);

        match ui_tree.click(x, y, total_lines, visible_lines, scroll_offset, selecting) {
            UiAction::ActivateTab(i) => {
                self.logic.active_tab = i;
                self.auto_scroll();
                self.logic.ui_state.mouse_interaction = MouseInteraction::TabDrag { tab_index: i };
                return AppResult::Redraw;
            }
            UiAction::NewTab => return self.new_tab(),
            UiAction::StartScrollbarDrag { drag_offset } => {
                self.logic.ui_state.mouse_interaction = MouseInteraction::ScrollbarDrag { drag_offset };
                return AppResult::Ok;
            }
            UiAction::ScrollbarJump { ratio } => {
                self.logic.ui_state.mouse_interaction = MouseInteraction::None;
                return self.jump_scrollbar_to_ratio(ratio);
            }
            UiAction::WindowMinimize => return AppResult::WindowMinimize,
            UiAction::WindowMaximize => return AppResult::WindowMaximize,
            UiAction::WindowClose => return AppResult::WindowClose,
            UiAction::WindowDrag => {
                self.logic.ui_state.mouse_interaction = MouseInteraction::WindowDrag;
                return AppResult::WindowDrag;
            }
            UiAction::WindowResize(edge) => {
                self.logic.ui_state.mouse_interaction = MouseInteraction::WindowResize(edge);
                return AppResult::WindowResize(edge);
            }
            UiAction::None => return AppResult::Ok,
            UiAction::TextClick => {
                self.logic.ui_state.mouse_interaction = MouseInteraction::TextSelection;
            }
        }

        let content_start_y = self.content_start_y();
        if !selecting && y < content_start_y { return AppResult::Ok; }

        let height = self.visible_lines() as isize;
        let relative_y = y - content_start_y;
        let mut clicked_visual_line = (relative_y / (layout::LINE_HEIGHT * self.logic.scale)).floor() as isize;

        if selecting && (clicked_visual_line < 0 || clicked_visual_line >= height) {
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

        self.logic.tabs[self.logic.active_tab].set_cursor_position(clicked_line, clicked_col, selecting);
        if selecting { self.auto_scroll(); }
        self.logic.ui_state.reset_cursor_blink();
        AppResult::Redraw
    }

    pub fn handle_double_click(&mut self, x: f32, y: f32) -> AppResult {
        let tab_info = self.tab_titles();
        let total_lines = self.logic.tabs[self.logic.active_tab].total_lines();
        let visible_lines = self.visible_lines();
        let scroll_offset = self.logic.tabs[self.logic.active_tab].scroll_offset();
        let ui_tree = self.logic.build_ui_tree(&tab_info);

        match ui_tree.double_click(x, y, total_lines, visible_lines, scroll_offset) {
            UiAction::ActivateTab(i) => { self.logic.active_tab = i; self.auto_scroll(); AppResult::Redraw }
            UiAction::NewTab => self.new_tab(),
            UiAction::TextClick => {
                let _ = self.click_at(x, y, false);
                self.logic.tabs[self.logic.active_tab].select_word_at_cursor();
                AppResult::Redraw
            }
            _ => AppResult::Ok,
        }
    }

    pub fn handle_triple_click(&mut self, x: f32, y: f32) -> AppResult {
        let tab_info = self.tab_titles();
        let total_lines = self.logic.tabs[self.logic.active_tab].total_lines();
        let visible_lines = self.visible_lines();
        let scroll_offset = self.logic.tabs[self.logic.active_tab].scroll_offset();
        let ui_tree = self.logic.build_ui_tree(&tab_info);

        match ui_tree.triple_click(x, y, total_lines, visible_lines, scroll_offset) {
            UiAction::ActivateTab(i) => { self.logic.active_tab = i; self.auto_scroll(); AppResult::Redraw }
            UiAction::NewTab => self.new_tab(),
            UiAction::TextClick => {
                let _ = self.click_at(x, y, false);
                self.logic.tabs[self.logic.active_tab].select_line_at_cursor();
                AppResult::Redraw
            }
            _ => AppResult::Ok,
        }
    }

    pub fn right_click_at(&mut self, x: f32, y: f32) -> AppResult {
        let tab_info = self.tab_titles();
        let ui_tree = self.logic.build_ui_tree(&tab_info);
        match ui_tree.hit_test(x, y) {
            UiNode::Tab(i) => { self.logic.start_rename(i); AppResult::Redraw }
            _ => AppResult::Ok,
        }
    }

    pub(super) fn jump_scrollbar_to_ratio(&mut self, ratio: f32) -> AppResult {
        let total_lines = self.logic.tabs[self.logic.active_tab].total_lines();
        let visible_lines = self.visible_lines();
        if total_lines <= visible_lines { return AppResult::Ok; }
        let max_scroll = total_lines.saturating_sub(visible_lines);
        let scroll_offset = (ratio.clamp(0.0, 1.0) * max_scroll as f32).round() as usize;
        if self.logic.tabs[self.logic.active_tab].set_scroll_offset(scroll_offset) {
            return AppResult::Redraw;
        }
        AppResult::Ok
    }
}
